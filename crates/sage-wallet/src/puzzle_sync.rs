use crate::prelude::*;
use sage_database::{Database, SqlExecutor, UnsyncedCoin};
use tracing::{debug, warn};

use crate::{
    ChildKind, PeerApi, PuzzleContext, WalletError, database::insert_puzzle, validate_wallet_coin,
};

#[derive(Debug, Clone)]
pub struct SyncedCoin {
    pub coin_state: CoinState,
    pub kind: Option<ChildKind>,
    pub context: PuzzleContext,
}

/// Identifies an unsynced coin by fetching its parent spend (to classify the
/// asset) and, if it has been spent, its children.
pub async fn fetch_puzzles(
    peer: &impl PeerApi,
    genesis_challenge: Bytes32,
    unsynced_coin: UnsyncedCoin,
    is_custody_p2_puzzle_hash: bool,
) -> Result<Vec<SyncedCoin>, WalletError> {
    let coin = unsynced_coin.coin_state.coin;

    let mut synced_coins = Vec::new();

    if unsynced_coin.is_asset_unsynced {
        let parent_spend = if is_custody_p2_puzzle_hash {
            None
        } else {
            peer.fetch_optional_coin_spend(coin.parent_coin_info, genesis_challenge)
                .await?
        };

        if let Some(parent_spend) = parent_spend {
            let kind = ChildKind::from_parent(
                parent_spend.coin,
                &parent_spend.puzzle_reveal,
                &parent_spend.solution,
                coin,
            )?;

            let context = PuzzleContext::fetch(peer, genesis_challenge, &kind).await?;

            synced_coins.push(SyncedCoin {
                coin_state: unsynced_coin.coin_state,
                kind: Some(kind),
                context,
            });
        } else {
            synced_coins.push(SyncedCoin {
                coin_state: unsynced_coin.coin_state,
                kind: None,
                context: PuzzleContext::None,
            });
        }
    }

    if unsynced_coin.is_children_unsynced
        && let Some(spent_height) = unsynced_coin.coin_state.spent_height
    {
        let (puzzle_reveal, solution) = peer
            .fetch_puzzle_solution(coin.coin_id(), spent_height)
            .await?;

        let children = ChildKind::parse_children(coin, &puzzle_reveal, &solution)?;

        let coin_states = peer
            .fetch_coins(
                children.iter().map(|(child, _)| child.coin_id()).collect(),
                genesis_challenge,
            )
            .await?;

        for (child_coin, kind) in children {
            let Some(&coin_state) = coin_states
                .iter()
                .find(|coin_state| coin_state.coin.coin_id() == child_coin.coin_id())
            else {
                continue;
            };

            let context = PuzzleContext::fetch(peer, genesis_challenge, &kind).await?;

            synced_coins.push(SyncedCoin {
                coin_state,
                kind: Some(kind),
                context,
            });
        }
    }

    Ok(synced_coins)
}

/// Applies the result of [`fetch_puzzles`] to the database, inserting the
/// identified assets and relevant child coins. Returns whether anything
/// changed, along with the coin ids that should be subscribed to.
pub async fn apply_synced_coins<E: SqlExecutor>(
    db: &Database<E>,
    root: &UnsyncedCoin,
    synced_coins: Vec<SyncedCoin>,
    subscriptions: &mut Vec<Bytes32>,
) -> Result<bool, WalletError> {
    let mut send_events = false;
    let mut tx = db.tx().await?;

    if root.is_children_unsynced {
        debug!(
            "Children have been synced for coin {}",
            root.coin_state.coin.coin_id()
        );
        tx.set_children_synced(root.coin_state.coin.coin_id())
            .await?;
        send_events = true;
    }

    for item in synced_coins {
        let coin_id = item.coin_state.coin.coin_id();
        let is_root = root.coin_state.coin.coin_id() == coin_id;

        // We want to skip children that we already know about
        if !is_root && tx.is_known_coin(coin_id).await? {
            debug!("Skipping child coin {coin_id} because it is already known");
            continue;
        }

        let is_custody_p2_puzzle_hash = tx.is_custody_p2_puzzle_hash(coin_id).await?;

        let Some(kind) = item.kind.filter(|_| !is_custody_p2_puzzle_hash) else {
            warn!(
                "Retroactively inserting XCH coin that should have already been synced: {coin_id}"
            );

            db.update_coin(
                coin_id,
                Bytes32::default(),
                item.coin_state.coin.puzzle_hash,
            )
            .await?;
            send_events = true;
            continue;
        };

        // We don't want to insert child coins that we don't own.
        let custody_p2_puzzle_hashes = kind.custody_p2_puzzle_hashes();

        let mut is_relevant = false;

        for custody_p2_puzzle_hash in custody_p2_puzzle_hashes {
            if tx.is_custody_p2_puzzle_hash(custody_p2_puzzle_hash).await? {
                is_relevant = true;
                break;
            }
        }

        if !is_relevant {
            if is_root {
                warn!(
                    "Deleting unexpected coin {coin_id} because it is not relevant to this wallet"
                );
                tx.delete_coin(coin_id).await?;
                send_events = true;
            } else {
                debug!("Skipping coin {coin_id} because it is not relevant to this wallet");
            }
            continue;
        }

        if is_root {
            debug!("Synced puzzle for coin {coin_id}");
        } else {
            debug!("Found relevant child coin {coin_id} which wasn't synced");
        }

        send_events = true;

        if let Some(height) = item.coin_state.created_height {
            tx.insert_height(height).await?;
        }

        if let Some(height) = item.coin_state.spent_height {
            tx.insert_height(height).await?;
        }

        tx.insert_coin(item.coin_state).await?;

        let is_inserted = validate_wallet_coin(&mut tx, coin_id, &kind).await?
            && insert_puzzle(
                &mut tx,
                item.coin_state,
                kind.clone(),
                item.context.clone(),
                None,
            )
            .await?;

        if is_inserted {
            if kind.subscribe() {
                subscriptions.push(coin_id);
            }

            if let PuzzleContext::Option(context) = item.context {
                subscriptions.push(context.underlying.coin.coin_id());
            }
        }
    }

    tx.commit().await?;

    Ok(send_events)
}

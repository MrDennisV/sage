use std::{collections::HashSet, sync::Arc, time::Duration};

use crate::prelude::*;
use chia_protocol::CoinStateFilters;
use sage_database::{DatabaseTx, SqlExecutor};
use tracing::info;

use crate::portable::sleep;
use crate::{EventSink, PeerApi, SyncEvent, Wallet, WalletError};

pub async fn sync_wallet<E: SqlExecutor>(
    wallet: Arc<Wallet<E>>,
    peer: &impl PeerApi,
    sink: &impl EventSink,
    delta_sync: bool,
) -> Result<(), WalletError> {
    let p2_puzzle_hashes = wallet.db.custody_p2_puzzle_hashes().await?;

    let (start_height, start_header_hash) = if delta_sync {
        wallet.db.latest_peak().await?
    } else {
        None
    }
    .map_or_else(
        || (None, wallet.genesis_challenge),
        |(peak, header_hash)| (Some(peak), header_hash),
    );

    let coin_ids = wallet.db.subscription_coin_ids().await?;

    sync_coin_ids(
        &wallet,
        peer,
        start_height,
        start_header_hash,
        coin_ids,
        sink,
        false,
    )
    .await?;

    for batch in p2_puzzle_hashes.chunks(1000) {
        sync_puzzle_hashes(&wallet, peer, start_height, start_header_hash, batch, sink).await?;
    }

    loop {
        let mut tx = wallet.db.tx().await?;
        let derivations = auto_insert_unhardened_derivations(&wallet, &mut tx).await?;
        let next_index = tx.derivation_index(false).await?;
        tx.commit().await?;

        if derivations.is_empty() {
            break;
        }

        info!("Inserted {} derivations", derivations.len());

        sink.send_event(SyncEvent::DerivationIndex { next_index })
            .await;

        for batch in derivations.chunks(1000) {
            sync_puzzle_hashes(&wallet, peer, None, wallet.genesis_challenge, batch, sink).await?;
        }
    }

    Ok(())
}

async fn sync_coin_ids<E: SqlExecutor>(
    wallet: &Wallet<E>,
    peer: &impl PeerApi,
    start_height: Option<u32>,
    start_header_hash: Bytes32,
    coin_ids: Vec<Bytes32>,
    sink: &impl EventSink,
    only_send_event_if_spent: bool,
) -> Result<(), WalletError> {
    for (i, coin_ids) in coin_ids.chunks(10000).enumerate() {
        if i != 0 {
            sleep(Duration::from_millis(500)).await;
        }

        info!("Subscribing to {} coins", coin_ids.len());

        let coin_states = peer
            .subscribe_coins(coin_ids.to_vec(), start_height, start_header_hash)
            .await?;

        info!("Received {} coin states", coin_states.len());

        if coin_states
            .iter()
            .any(|cs| cs.spent_height.is_some() || !only_send_event_if_spent)
        {
            incremental_sync(wallet, coin_states, true, sink).await?;
        }
    }

    Ok(())
}

async fn sync_puzzle_hashes<E: SqlExecutor>(
    wallet: &Wallet<E>,
    peer: &impl PeerApi,
    start_height: Option<u32>,
    start_header_hash: Bytes32,
    puzzle_hashes: &[Bytes32],
    sink: &impl EventSink,
) -> Result<(), WalletError> {
    if puzzle_hashes.is_empty() {
        return Ok(());
    }

    let mut prev_height = start_height;
    let mut prev_header_hash = start_header_hash;

    loop {
        info!(
            "Subscribing to {} puzzle hashes at height {:?} and header hash {}",
            puzzle_hashes.len(),
            prev_height,
            prev_header_hash,
        );

        let data = peer
            .subscribe_puzzles(
                puzzle_hashes.to_vec(),
                prev_height,
                prev_header_hash,
                CoinStateFilters::new(true, true, true, 0),
            )
            .await?;

        info!("Received {} coin states", data.coin_states.len());

        if !data.coin_states.is_empty() {
            incremental_sync(wallet, data.coin_states, true, sink).await?;
        }

        prev_height = Some(data.height);
        prev_header_hash = data.header_hash;

        if data.is_finished {
            break;
        }
    }

    Ok(())
}

pub async fn incremental_sync<E: SqlExecutor>(
    wallet: &Wallet<E>,
    coin_states: Vec<CoinState>,
    derive_automatically: bool,
    sink: &impl EventSink,
) -> Result<(), WalletError> {
    let mut tx = wallet.db.tx().await?;
    let mut confirmed_transactions = HashSet::new();

    for &coin_state in &coin_states {
        if let Some(height) = coin_state.created_height {
            tx.insert_height(height).await?;
        }

        if let Some(height) = coin_state.spent_height {
            tx.insert_height(height).await?;
        }

        tx.insert_coin(coin_state).await?;

        if tx
            .is_custody_p2_puzzle_hash(coin_state.coin.puzzle_hash)
            .await?
        {
            tx.update_coin(
                coin_state.coin.coin_id(),
                Bytes32::default(),
                coin_state.coin.puzzle_hash,
            )
            .await?;
        }

        confirmed_transactions.extend(
            tx.mempool_items_for_output(coin_state.coin.coin_id())
                .await?,
        );

        if coin_state.spent_height.is_some() {
            confirmed_transactions.extend(
                tx.mempool_items_for_input(coin_state.coin.coin_id())
                    .await?,
            );
        }
    }

    for mempool_item_id in confirmed_transactions {
        tx.remove_mempool_item(mempool_item_id).await?;
    }

    let mut new_derivations = Vec::new();

    if derive_automatically {
        new_derivations = auto_insert_unhardened_derivations(wallet, &mut tx).await?;
    }

    let next_index = tx.derivation_index(false).await?;

    tx.commit().await?;

    if !coin_states.is_empty() {
        sink.send_event(SyncEvent::CoinsUpdated).await;
    }

    if !new_derivations.is_empty() {
        sink.send_event(SyncEvent::DerivationIndex { next_index })
            .await;

        sink.subscribe_puzzles(new_derivations).await;
    }

    Ok(())
}

async fn auto_insert_unhardened_derivations<E: SqlExecutor>(
    wallet: &Wallet<E>,
    tx: &mut DatabaseTx<'_, E>,
) -> Result<Vec<Bytes32>, WalletError> {
    let mut derivations = Vec::new();
    let mut next_index = tx.derivation_index(false).await?;

    let max_index = tx.unused_derivation_index(false).await?;

    while max_index + 500 >= next_index {
        derivations.extend(
            wallet
                .insert_unhardened_derivations(tx, next_index..next_index + 500)
                .await?,
        );

        next_index += 500;
    }

    Ok(derivations)
}

pub async fn add_new_subscriptions<E: SqlExecutor>(
    wallet: &Wallet<E>,
    peer: &impl PeerApi,
    coin_ids: Vec<Bytes32>,
    puzzle_hashes: Vec<Bytes32>,
    sink: &impl EventSink,
) -> Result<(), WalletError> {
    for batch in coin_ids.chunks(1000) {
        sync_coin_ids(
            wallet,
            peer,
            None,
            wallet.genesis_challenge,
            batch.to_vec(),
            sink,
            true,
        )
        .await?;
    }

    for batch in puzzle_hashes.chunks(1000) {
        sync_puzzle_hashes(wallet, peer, None, wallet.genesis_challenge, batch, sink).await?;
    }

    Ok(())
}

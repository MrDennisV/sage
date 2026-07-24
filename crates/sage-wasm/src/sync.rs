use std::{cell::RefCell, sync::Arc};

use chia_protocol::Bytes32;
use chia_sdk_coinset::CoinsetClient;
use sage_wallet::{
    CoinsetPeer, EventSink, SyncEvent, Wallet, add_new_subscriptions, apply_synced_coins,
    fetch_puzzles, sync_wallet,
};
use tracing::debug;
use wasm_bindgen::prelude::*;

use crate::{
    BrowserExecutor,
    bootstrap::{SAGE, not_initialized},
    js_error,
};

/// Collects sync events so they can be returned to JS after a sync pass.
/// Subscriptions are no-ops since polling re-queries everything.
#[derive(Debug, Default)]
pub struct CollectorSink {
    events: RefCell<Vec<SyncEvent>>,
}

impl CollectorSink {
    fn into_events(self) -> Vec<SyncEvent> {
        self.events.into_inner()
    }
}

impl EventSink for CollectorSink {
    async fn send_event(&self, event: SyncEvent) {
        self.events.borrow_mut().push(event);
    }

    async fn subscribe_puzzles(&self, _puzzle_hashes: Vec<Bytes32>) {}

    async fn subscribe_coins(&self, _coin_ids: Vec<Bytes32>) {}
}

/// Runs one full sync pass against the Coinset API and returns the collected
/// sync events as a JSON array in the `sage_api::SyncEvent` wire shape.
#[wasm_bindgen]
pub async fn sage_sync_once(delta_sync: bool) -> Result<String, JsValue> {
    // Clone the wallet handle and network id out of the RefCell before any
    // await; a borrow must not be held across a suspension point.
    let (wallet, network_id) = SAGE.with(|cell| {
        let guard = cell.borrow();
        let sage = guard.as_ref().ok_or_else(not_initialized)?;
        let wallet = sage.wallet().map_err(js_error)?;
        Ok::<_, JsValue>((wallet, sage.network_id()))
    })?;

    let peer = match network_id.as_str() {
        "mainnet" => CoinsetPeer::mainnet(),
        "testnet11" => CoinsetPeer::testnet11(),
        // Coinset hosts other networks as subdomains, following the
        // testnet11 convention.
        _ => CoinsetPeer::new(CoinsetClient::new(format!(
            "https://{network_id}.api.coinset.org"
        ))),
    };

    let sink = CollectorSink::default();

    sync_wallet(wallet.clone(), &peer, &sink, delta_sync)
        .await
        .map_err(js_error)?;

    // Mirrors the native `sync_wallet_against_peer`: record the peak so the
    // next delta sync resumes from it.
    if delta_sync {
        let (height, header_hash) = peer.get_peak().await.map_err(js_error)?;

        wallet
            .db
            .insert_block(height, header_hash, None, true)
            .await
            .map_err(js_error)?;
    }

    identify_puzzles(&wallet, &peer, &sink).await?;

    let events: Vec<sage_api::SyncEvent> =
        sink.into_events().into_iter().map(to_api_event).collect();

    serde_json::to_string(&events).map_err(js_error)
}

/// Identifies unsynced coins in bounded rounds, mirroring the native
/// `PuzzleQueue`. Coins that fail to fetch are skipped; the round bound
/// keeps repeated failures from spinning the loop forever.
async fn identify_puzzles(
    wallet: &Arc<Wallet<BrowserExecutor>>,
    peer: &CoinsetPeer,
    sink: &CollectorSink,
) -> Result<(), JsValue> {
    let mut subscriptions = Vec::new();

    for _ in 0..20 {
        let rows = wallet.db.unsynced_coins(50).await.map_err(js_error)?;

        if rows.is_empty() {
            break;
        }

        let mut send_events = false;

        for row in rows {
            let is_custody = wallet
                .db
                .is_custody_p2_puzzle_hash(row.coin_state.coin.puzzle_hash)
                .await
                .map_err(js_error)?;

            match fetch_puzzles(peer, wallet.genesis_challenge, row, is_custody).await {
                Ok(synced) => {
                    send_events |= apply_synced_coins(&wallet.db, &row, synced, &mut subscriptions)
                        .await
                        .map_err(js_error)?;
                }
                Err(error) => {
                    debug!(
                        "Failed to sync coin {}: {error}",
                        row.coin_state.coin.coin_id()
                    );
                }
            }
        }

        if send_events {
            sink.send_event(SyncEvent::PuzzleBatchSynced).await;
        }
    }

    // Natively this would be a SubscribeCoins command; here the new
    // subscriptions are synced immediately since polling has no push channel.
    if !subscriptions.is_empty() {
        add_new_subscriptions(wallet, peer, subscriptions, Vec::new(), sink)
            .await
            .map_err(js_error)?;
    }

    Ok(())
}

/// Mirrors the native mapping in `src-tauri/src/app_state.rs` so the frontend
/// adapter can re-broadcast events with the exact same wire shape.
fn to_api_event(event: SyncEvent) -> sage_api::SyncEvent {
    match event {
        SyncEvent::Start(ip) => sage_api::SyncEvent::Start { ip: ip.to_string() },
        SyncEvent::Stop => sage_api::SyncEvent::Stop,
        SyncEvent::Subscribed => sage_api::SyncEvent::Subscribed,
        SyncEvent::DerivationIndex { .. } => sage_api::SyncEvent::Derivation,
        SyncEvent::TransactionFailed {
            transaction_id,
            error,
        } => sage_api::SyncEvent::TransactionFailed {
            transaction_id: transaction_id.to_string(),
            error,
        },
        SyncEvent::CoinsUpdated
        | SyncEvent::TransactionUpdated { .. }
        | SyncEvent::OfferUpdated { .. } => sage_api::SyncEvent::CoinState,
        SyncEvent::PuzzleBatchSynced => sage_api::SyncEvent::PuzzleBatchSynced,
        SyncEvent::CatInfo => sage_api::SyncEvent::CatInfo,
        SyncEvent::DidInfo => sage_api::SyncEvent::DidInfo,
        SyncEvent::NftData => sage_api::SyncEvent::NftData,
    }
}

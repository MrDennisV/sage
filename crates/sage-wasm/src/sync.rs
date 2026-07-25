use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

use chia_protocol::Bytes32;
use futures_util::future::join_all;
use sage_api::NetworkKind;
use sage_wallet::{
    CoinsetPeer, EventSink, SyncEvent, Wallet, add_new_subscriptions, apply_synced_coins,
    fetch_puzzles, refresh_cat_catalog_page, sync_wallet,
};
use tracing::{debug, warn};
use wasm_bindgen::prelude::*;

use crate::{
    BrowserExecutor,
    bootstrap::{not_initialized, sage_cell},
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
    // Clone the wallet handle and peer out of the RefCell before any await, so
    // commands arriving during the sync aren't rejected as busy.
    let (wallet, peer, catalog_network) = {
        let cell = sage_cell();
        let guard = cell.borrow();
        let sage = guard.as_ref().ok_or_else(not_initialized)?;
        let wallet = sage.wallet().map_err(js_error)?;

        // Dexie only lists tokens for the two well known chains, which is also
        // where the native queue restricts itself to.
        let catalog_network = match sage.network_kind() {
            NetworkKind::Mainnet => Some(false),
            NetworkKind::Testnet => Some(true),
            NetworkKind::Unknown => None,
        };

        (wallet, sage.peer(), catalog_network)
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

    if let Some(testnet) = catalog_network {
        refresh_cat_catalog(&wallet, testnet, &sink).await;
    }

    let events: Vec<sage_api::SyncEvent> =
        sink.into_events().into_iter().map(to_api_event).collect();

    serde_json::to_string(&events).map_err(js_error)
}

/// How many coins to claim from the database per round.
const BATCH_SIZE: usize = 50;

/// How many coin lookups to have in flight at once. Browsers cap concurrent
/// connections per host, so a larger number wouldn't go any faster.
const CONCURRENT_REQUESTS: usize = 6;

/// How long one pass may spend identifying coins. The pass runs while holding
/// the wallet, so a request arriving mid-pass waits at most this long; the
/// next pass picks up where this one stopped.
const PUZZLE_SYNC_BUDGET_MS: f64 = 1_500.0;

/// Identifies unsynced coins, mirroring the native `PuzzleQueue`. Coins that
/// fail to fetch are skipped, and the time budget keeps a large wallet (or
/// repeated failures) from holding the wallet for the whole sync.
async fn identify_puzzles(
    wallet: &Arc<Wallet<BrowserExecutor>>,
    peer: &CoinsetPeer,
    sink: &CollectorSink,
) -> Result<(), JsValue> {
    let mut subscriptions = Vec::new();
    let deadline = js_sys::Date::now() + PUZZLE_SYNC_BUDGET_MS;

    while js_sys::Date::now() < deadline {
        let rows = wallet
            .db
            .unsynced_coins(BATCH_SIZE)
            .await
            .map_err(js_error)?;

        if rows.is_empty() {
            break;
        }

        let mut send_events = false;

        for chunk in rows.chunks(CONCURRENT_REQUESTS) {
            let mut requests = Vec::with_capacity(chunk.len());

            for row in chunk {
                let is_custody = wallet
                    .db
                    .is_custody_p2_puzzle_hash(row.coin_state.coin.puzzle_hash)
                    .await
                    .map_err(js_error)?;

                requests.push(fetch_puzzles(
                    peer,
                    wallet.genesis_challenge,
                    *row,
                    is_custody,
                ));
            }

            // The lookups are network bound, so they run together; the results
            // are applied one at a time because they share a database
            // connection.
            let results = join_all(requests).await;

            for (row, result) in chunk.iter().zip(results) {
                match result {
                    Ok(synced) => {
                        send_events |=
                            apply_synced_coins(&wallet.db, row, synced, &mut subscriptions)
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

thread_local! {
    /// The page of the token catalog to ask for next, and when the listing was
    /// last walked all the way through. Both start over when the worker
    /// restarts, which costs nothing but the requests: recording a token that
    /// is already there leaves it as it was.
    static CATALOG_PAGE: Cell<u32> = const { Cell::new(1) };
    static CATALOG_WALKED_AT: Cell<f64> = const { Cell::new(0.0) };
}

/// How long one pass may spend on the token catalog.
const CATALOG_BUDGET_MS: f64 = 1_000.0;

/// How long a finished walk stands before the listing is walked again.
const CATALOG_INTERVAL_MS: f64 = 6.0 * 60.0 * 60.0 * 1_000.0;

/// Records the tokens Dexie lists, which is where the interface gets the
/// assets it offers to pick from. The listing runs to thousands of tokens, so
/// each pass walks as many pages as its budget allows and the next one carries
/// on from there. Natively this is a queue of its own, and a failure there
/// doesn't stop coins from syncing either, so a failed page is logged and
/// retried on the next pass rather than failing the sync.
async fn refresh_cat_catalog(
    wallet: &Arc<Wallet<BrowserExecutor>>,
    testnet: bool,
    sink: &CollectorSink,
) {
    if CATALOG_PAGE.get() == 1
        && js_sys::Date::now() - CATALOG_WALKED_AT.get() < CATALOG_INTERVAL_MS
    {
        return;
    }

    let deadline = js_sys::Date::now() + CATALOG_BUDGET_MS;
    let mut recorded = false;

    while js_sys::Date::now() < deadline {
        let page = CATALOG_PAGE.get();

        match refresh_cat_catalog_page(&wallet.db, testnet, page).await {
            Ok(true) => {
                CATALOG_PAGE.set(page + 1);
                recorded = true;
            }
            Ok(false) => {
                CATALOG_PAGE.set(1);
                CATALOG_WALKED_AT.set(js_sys::Date::now());
                break;
            }
            Err(error) => {
                warn!("Failed to record token catalog page {page}: {error}");
                break;
            }
        }
    }

    if recorded {
        sink.send_event(SyncEvent::CatInfo).await;
    }
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

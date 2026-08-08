use crate::prelude::*;
use sage_database::OfferStatus;
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncEvent {
    Start(IpAddr),
    Stop,
    Subscribed,
    DerivationIndex {
        next_index: u32,
    },
    CoinsUpdated,
    TransactionUpdated {
        transaction_id: Bytes32,
    },
    TransactionFailed {
        transaction_id: Bytes32,
        error: Option<String>,
    },
    OfferUpdated {
        offer_id: Bytes32,
        status: OfferStatus,
    },
    PuzzleBatchSynced,
    CatInfo,
    DidInfo,
    NftData,
    NetworkChanged {
        network_id: String,
    },
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        /// Receives sync progress notifications and subscription requests from
        /// the sync functions. The native implementation forwards to the
        /// `SyncManager` channels; browser implementations collect events and
        /// treat subscriptions as no-ops, since polling re-queries everything.
        pub trait EventSink {
            fn send_event(&self, event: SyncEvent) -> impl Future<Output = ()>;
            fn subscribe_puzzles(&self, puzzle_hashes: Vec<Bytes32>) -> impl Future<Output = ()>;
            fn subscribe_coins(&self, coin_ids: Vec<Bytes32>) -> impl Future<Output = ()>;
        }
    } else {
        /// Receives sync progress notifications and subscription requests from
        /// the sync functions. The native implementation forwards to the
        /// `SyncManager` channels; browser implementations collect events and
        /// treat subscriptions as no-ops, since polling re-queries everything.
        pub trait EventSink: Send + Sync {
            fn send_event(&self, event: SyncEvent) -> impl Future<Output = ()> + Send;
            fn subscribe_puzzles(&self, puzzle_hashes: Vec<Bytes32>) -> impl Future<Output = ()> + Send;
            fn subscribe_coins(&self, coin_ids: Vec<Bytes32>) -> impl Future<Output = ()> + Send;
        }
    }
}

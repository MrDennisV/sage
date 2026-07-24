use chia_protocol::{
    Bytes32, CoinSpend, CoinState, CoinStateFilters, Program, RespondPuzzleState, SpendBundle,
    TransactionAck,
};

use crate::WalletError;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        /// Abstracts the full-node operations the sync and spend logic needs, so
        /// they run against a native peer connection or an HTTP API in the browser.
        pub trait PeerApi {
            /// Fetches coin states by id, optionally subscribing to updates on
            /// implementations that support push notifications.
            fn subscribe_coins(
                &self,
                coin_ids: Vec<Bytes32>,
                previous_height: Option<u32>,
                header_hash: Bytes32,
            ) -> impl Future<Output = Result<Vec<CoinState>, WalletError>>;

            /// Fetches a page of coin states by puzzle hash. The returned response
            /// carries the pagination cursor (`height`, `header_hash`, `is_finished`)
            /// callers must loop on.
            fn subscribe_puzzles(
                &self,
                puzzle_hashes: Vec<Bytes32>,
                previous_height: Option<u32>,
                header_hash: Bytes32,
                filters: CoinStateFilters,
            ) -> impl Future<Output = Result<RespondPuzzleState, WalletError>>;

            fn fetch_coin(
                &self,
                coin_id: Bytes32,
                genesis_challenge: Bytes32,
            ) -> impl Future<Output = Result<CoinState, WalletError>>;

            fn fetch_optional_coin(
                &self,
                coin_id: Bytes32,
                genesis_challenge: Bytes32,
            ) -> impl Future<Output = Result<Option<CoinState>, WalletError>>;

            fn fetch_optional_coin_spend(
                &self,
                coin_id: Bytes32,
                genesis_challenge: Bytes32,
            ) -> impl Future<Output = Result<Option<CoinSpend>, WalletError>>;

            fn fetch_puzzle_solution(
                &self,
                coin_id: Bytes32,
                spent_height: u32,
            ) -> impl Future<Output = Result<(Program, Program), WalletError>>;

            fn fetch_coin_spend(
                &self,
                coin_id: Bytes32,
                genesis_challenge: Bytes32,
            ) -> impl Future<Output = Result<CoinSpend, WalletError>>;

            fn try_fetch_singleton_child(
                &self,
                coin_id: Bytes32,
            ) -> impl Future<Output = Result<Option<CoinState>, WalletError>>;

            fn send_transaction(
                &self,
                spend_bundle: SpendBundle,
            ) -> impl Future<Output = Result<TransactionAck, WalletError>>;

            fn block_timestamp(
                &self,
                height: u32,
            ) -> impl Future<Output = Result<(Bytes32, u64), WalletError>>;
        }
    } else {
        /// Abstracts the full-node operations the sync and spend logic needs, so
        /// they run against a native peer connection or an HTTP API in the browser.
        pub trait PeerApi: Send + Sync {
            /// Fetches coin states by id, optionally subscribing to updates on
            /// implementations that support push notifications.
            fn subscribe_coins(
                &self,
                coin_ids: Vec<Bytes32>,
                previous_height: Option<u32>,
                header_hash: Bytes32,
            ) -> impl Future<Output = Result<Vec<CoinState>, WalletError>> + Send;

            /// Fetches a page of coin states by puzzle hash. The returned response
            /// carries the pagination cursor (`height`, `header_hash`, `is_finished`)
            /// callers must loop on.
            fn subscribe_puzzles(
                &self,
                puzzle_hashes: Vec<Bytes32>,
                previous_height: Option<u32>,
                header_hash: Bytes32,
                filters: CoinStateFilters,
            ) -> impl Future<Output = Result<RespondPuzzleState, WalletError>> + Send;

            fn fetch_coin(
                &self,
                coin_id: Bytes32,
                genesis_challenge: Bytes32,
            ) -> impl Future<Output = Result<CoinState, WalletError>> + Send;

            fn fetch_optional_coin(
                &self,
                coin_id: Bytes32,
                genesis_challenge: Bytes32,
            ) -> impl Future<Output = Result<Option<CoinState>, WalletError>> + Send;

            fn fetch_optional_coin_spend(
                &self,
                coin_id: Bytes32,
                genesis_challenge: Bytes32,
            ) -> impl Future<Output = Result<Option<CoinSpend>, WalletError>> + Send;

            fn fetch_puzzle_solution(
                &self,
                coin_id: Bytes32,
                spent_height: u32,
            ) -> impl Future<Output = Result<(Program, Program), WalletError>> + Send;

            fn fetch_coin_spend(
                &self,
                coin_id: Bytes32,
                genesis_challenge: Bytes32,
            ) -> impl Future<Output = Result<CoinSpend, WalletError>> + Send;

            fn try_fetch_singleton_child(
                &self,
                coin_id: Bytes32,
            ) -> impl Future<Output = Result<Option<CoinState>, WalletError>> + Send;

            fn send_transaction(
                &self,
                spend_bundle: SpendBundle,
            ) -> impl Future<Output = Result<TransactionAck, WalletError>> + Send;

            fn block_timestamp(
                &self,
                height: u32,
            ) -> impl Future<Output = Result<(Bytes32, u64), WalletError>> + Send;
        }
    }
}

use std::collections::HashMap;

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

            fn fetch_coins(
                &self,
                coin_ids: Vec<Bytes32>,
                genesis_challenge: Bytes32,
            ) -> impl Future<Output = Result<Vec<CoinState>, WalletError>>;

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

            fn fetch_coins(
                &self,
                coin_ids: Vec<Bytes32>,
                genesis_challenge: Bytes32,
            ) -> impl Future<Output = Result<Vec<CoinState>, WalletError>> + Send;

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

/// Overlays locally known pending coin states and spends on top of another
/// peer, so lookups for coins that haven't reached the network yet resolve
/// without a round trip.
#[derive(Debug)]
pub struct PendingPeer<'a, P> {
    peer: &'a P,
    coin_states: HashMap<Bytes32, CoinState>,
    coin_spends: HashMap<Bytes32, CoinSpend>,
}

impl<'a, P> PendingPeer<'a, P> {
    pub fn new(
        peer: &'a P,
        coin_states: HashMap<Bytes32, CoinState>,
        coin_spends: HashMap<Bytes32, CoinSpend>,
    ) -> Self {
        Self {
            peer,
            coin_states,
            coin_spends,
        }
    }
}

impl<P: PeerApi> PeerApi for PendingPeer<'_, P> {
    async fn subscribe_coins(
        &self,
        coin_ids: Vec<Bytes32>,
        previous_height: Option<u32>,
        header_hash: Bytes32,
    ) -> Result<Vec<CoinState>, WalletError> {
        self.peer
            .subscribe_coins(coin_ids, previous_height, header_hash)
            .await
    }

    async fn subscribe_puzzles(
        &self,
        puzzle_hashes: Vec<Bytes32>,
        previous_height: Option<u32>,
        header_hash: Bytes32,
        filters: CoinStateFilters,
    ) -> Result<RespondPuzzleState, WalletError> {
        self.peer
            .subscribe_puzzles(puzzle_hashes, previous_height, header_hash, filters)
            .await
    }

    async fn fetch_coin(
        &self,
        coin_id: Bytes32,
        genesis_challenge: Bytes32,
    ) -> Result<CoinState, WalletError> {
        if let Some(coin_state) = self.coin_states.get(&coin_id) {
            return Ok(*coin_state);
        }

        self.peer.fetch_coin(coin_id, genesis_challenge).await
    }

    async fn fetch_coins(
        &self,
        coin_ids: Vec<Bytes32>,
        genesis_challenge: Bytes32,
    ) -> Result<Vec<CoinState>, WalletError> {
        let mut coin_states = Vec::new();
        let mut unknown_coin_ids = Vec::new();

        for coin_id in coin_ids {
            if let Some(coin_state) = self.coin_states.get(&coin_id) {
                coin_states.push(*coin_state);
            } else {
                unknown_coin_ids.push(coin_id);
            }
        }

        if !unknown_coin_ids.is_empty() {
            coin_states.extend(
                self.peer
                    .fetch_coins(unknown_coin_ids, genesis_challenge)
                    .await?,
            );
        }

        Ok(coin_states)
    }

    async fn fetch_optional_coin(
        &self,
        coin_id: Bytes32,
        genesis_challenge: Bytes32,
    ) -> Result<Option<CoinState>, WalletError> {
        if let Some(coin_state) = self.coin_states.get(&coin_id) {
            return Ok(Some(*coin_state));
        }

        self.peer
            .fetch_optional_coin(coin_id, genesis_challenge)
            .await
    }

    async fn fetch_optional_coin_spend(
        &self,
        coin_id: Bytes32,
        genesis_challenge: Bytes32,
    ) -> Result<Option<CoinSpend>, WalletError> {
        if let Some(coin_spend) = self.coin_spends.get(&coin_id) {
            return Ok(Some(coin_spend.clone()));
        }

        let Some(coin_state) = self.fetch_optional_coin(coin_id, genesis_challenge).await? else {
            return Ok(None);
        };

        let spent_height = coin_state.spent_height.ok_or(WalletError::PeerMisbehaved)?;
        let (puzzle_reveal, solution) = self.fetch_puzzle_solution(coin_id, spent_height).await?;

        Ok(Some(CoinSpend::new(
            coin_state.coin,
            puzzle_reveal,
            solution,
        )))
    }

    async fn fetch_puzzle_solution(
        &self,
        coin_id: Bytes32,
        spent_height: u32,
    ) -> Result<(Program, Program), WalletError> {
        if let Some(coin_spend) = self.coin_spends.get(&coin_id) {
            return Ok((
                coin_spend.puzzle_reveal.clone(),
                coin_spend.solution.clone(),
            ));
        }

        self.peer.fetch_puzzle_solution(coin_id, spent_height).await
    }

    async fn fetch_coin_spend(
        &self,
        coin_id: Bytes32,
        genesis_challenge: Bytes32,
    ) -> Result<CoinSpend, WalletError> {
        if let Some(coin_spend) = self.coin_spends.get(&coin_id) {
            return Ok(coin_spend.clone());
        }

        let coin_state = self.fetch_coin(coin_id, genesis_challenge).await?;
        let spent_height = coin_state.spent_height.ok_or(WalletError::PeerMisbehaved)?;
        let (puzzle_reveal, solution) = self.fetch_puzzle_solution(coin_id, spent_height).await?;

        Ok(CoinSpend::new(coin_state.coin, puzzle_reveal, solution))
    }

    async fn try_fetch_singleton_child(
        &self,
        coin_id: Bytes32,
    ) -> Result<Option<CoinState>, WalletError> {
        if let Some(child) = self
            .coin_states
            .values()
            .find(|state| state.coin.parent_coin_info == coin_id && state.coin.amount % 2 == 1)
        {
            return Ok(Some(*child));
        }

        self.peer.try_fetch_singleton_child(coin_id).await
    }

    async fn send_transaction(
        &self,
        spend_bundle: SpendBundle,
    ) -> Result<TransactionAck, WalletError> {
        self.peer.send_transaction(spend_bundle).await
    }

    async fn block_timestamp(&self, height: u32) -> Result<(Bytes32, u64), WalletError> {
        self.peer.block_timestamp(height).await
    }
}

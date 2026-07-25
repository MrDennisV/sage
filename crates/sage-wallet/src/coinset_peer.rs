use crate::prelude::*;
use chia_protocol::{CoinStateFilters, RespondPuzzleState, TransactionAck};
use chia_sdk_coinset::{ChiaRpcClient, CoinRecord, CoinsetClient};

use crate::{PeerApi, WalletError};


/// A [`PeerApi`] implementation backed by the public Coinset HTTP API, used
/// where the native peer protocol isn't available (such as browsers). There
/// are no push notifications, so callers poll; `subscribe_*` methods fetch
/// current state and subscriptions are implicit no-ops.
#[derive(Debug, Clone)]
pub struct CoinsetPeer {
    client: CoinsetClient,
}

impl CoinsetPeer {
    pub fn new(client: CoinsetClient) -> Self {
        Self { client }
    }

    pub fn mainnet() -> Self {
        Self::new(CoinsetClient::mainnet())
    }

    pub fn testnet11() -> Self {
        Self::new(CoinsetClient::testnet11())
    }

    /// Builds a client for an API base URL.
    pub fn for_api_url(api_url: String) -> Self {
        Self::new(CoinsetClient::new(api_url))
    }

    /// Returns the current peak height and header hash.
    pub async fn get_peak(&self) -> Result<(u32, Bytes32), WalletError> {
        let response = self
            .client
            .get_blockchain_state()
            .await
            .map_err(coinset_error("get_blockchain_state"))?;

        let state = response
            .blockchain_state
            .ok_or(WalletError::PeerMisbehaved)?;

        Ok((state.peak.height, state.peak.header_hash))
    }
}

/// Names the request in the error, since the underlying client reports
/// transport failures without saying which call produced them.
fn coinset_error<E: std::error::Error>(request: &str) -> impl Fn(E) -> WalletError + '_ {
    move |error| {
        // The client reports transport failures without naming the request or
        // the underlying cause, so include both.
        use std::fmt::Write;

        let mut message = format!("{request}: {error}");
        let mut source = error.source();

        while let Some(cause) = source {
            let _ = write!(message, " -> {cause}");
            source = cause.source();
        }

        WalletError::Coinset(message)
    }
}

fn to_coin_state(record: &CoinRecord) -> CoinState {
    CoinState::new(
        record.coin,
        (record.spent_block_index > 0).then_some(record.spent_block_index),
        (record.confirmed_block_index > 0).then_some(record.confirmed_block_index),
    )
}

impl PeerApi for CoinsetPeer {
    async fn subscribe_coins(
        &self,
        coin_ids: Vec<Bytes32>,
        previous_height: Option<u32>,
        _header_hash: Bytes32,
    ) -> Result<Vec<CoinState>, WalletError> {
        if coin_ids.is_empty() {
            return Ok(Vec::new());
        }

        let response = self
            .client
            .get_coin_records_by_names(coin_ids, previous_height, None, Some(true))
            .await
            .map_err(coinset_error("get_coin_records_by_names"))?;

        Ok(response
            .coin_records
            .unwrap_or_default()
            .iter()
            .map(to_coin_state)
            .collect())
    }

    async fn subscribe_puzzles(
        &self,
        puzzle_hashes: Vec<Bytes32>,
        previous_height: Option<u32>,
        header_hash: Bytes32,
        filters: CoinStateFilters,
    ) -> Result<RespondPuzzleState, WalletError> {
        let response = self
            .client
            .get_coin_records_by_puzzle_hashes(
                puzzle_hashes.clone(),
                previous_height,
                None,
                Some(filters.include_spent),
            )
            .await
            .map_err(coinset_error("get_coin_records_by_puzzle_hashes"))?;

        let coin_states = response
            .coin_records
            .unwrap_or_default()
            .iter()
            .map(to_coin_state)
            .collect();

        // The response is not paginated, so report the cursor as finished at
        // the current peak. When the peak isn't available, fall back to the
        // caller's own cursor so delta sync stays monotonic.
        let (height, header_hash) = match self.get_peak().await {
            Ok(peak) => peak,
            Err(_) => (previous_height.unwrap_or(0), header_hash),
        };

        Ok(RespondPuzzleState::new(
            puzzle_hashes,
            height,
            header_hash,
            true,
            coin_states,
        ))
    }

    async fn fetch_coin(
        &self,
        coin_id: Bytes32,
        genesis_challenge: Bytes32,
    ) -> Result<CoinState, WalletError> {
        self.fetch_optional_coin(coin_id, genesis_challenge)
            .await?
            .ok_or(WalletError::MissingCoin(coin_id))
    }

    async fn fetch_coins(
        &self,
        coin_ids: Vec<Bytes32>,
        _genesis_challenge: Bytes32,
    ) -> Result<Vec<CoinState>, WalletError> {
        if coin_ids.is_empty() {
            return Ok(Vec::new());
        }

        let response = self
            .client
            .get_coin_records_by_names(coin_ids, None, None, Some(true))
            .await
            .map_err(coinset_error("get_coin_records_by_names"))?;

        Ok(response
            .coin_records
            .unwrap_or_default()
            .iter()
            .map(to_coin_state)
            .collect())
    }

    async fn fetch_optional_coin(
        &self,
        coin_id: Bytes32,
        _genesis_challenge: Bytes32,
    ) -> Result<Option<CoinState>, WalletError> {
        let response = self
            .client
            .get_coin_record_by_name(coin_id)
            .await
            .map_err(coinset_error("get_coin_record_by_name"))?;

        Ok(response.coin_record.as_ref().map(to_coin_state))
    }

    async fn fetch_optional_coin_spend(
        &self,
        coin_id: Bytes32,
        genesis_challenge: Bytes32,
    ) -> Result<Option<CoinSpend>, WalletError> {
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
        let response = self
            .client
            .get_puzzle_and_solution(coin_id, Some(spent_height))
            .await
            .map_err(coinset_error("get_puzzle_and_solution"))?;

        let coin_spend = response
            .coin_solution
            .ok_or(WalletError::MissingSpend(coin_id))?;

        Ok((coin_spend.puzzle_reveal, coin_spend.solution))
    }

    async fn fetch_coin_spend(
        &self,
        coin_id: Bytes32,
        genesis_challenge: Bytes32,
    ) -> Result<CoinSpend, WalletError> {
        let coin_state = self.fetch_coin(coin_id, genesis_challenge).await?;
        let spent_height = coin_state.spent_height.ok_or(WalletError::PeerMisbehaved)?;
        let (puzzle_reveal, solution) = self.fetch_puzzle_solution(coin_id, spent_height).await?;
        Ok(CoinSpend::new(coin_state.coin, puzzle_reveal, solution))
    }

    async fn try_fetch_singleton_child(
        &self,
        coin_id: Bytes32,
    ) -> Result<Option<CoinState>, WalletError> {
        let response = self
            .client
            .get_coin_records_by_parent_ids(vec![coin_id], None, None, Some(true))
            .await
            .map_err(coinset_error("get_coin_records_by_parent_ids"))?;

        Ok(response
            .coin_records
            .unwrap_or_default()
            .iter()
            .map(to_coin_state)
            .find(|child| child.coin.amount % 2 == 1))
    }

    async fn send_transaction(
        &self,
        spend_bundle: SpendBundle,
    ) -> Result<TransactionAck, WalletError> {
        let transaction_id = spend_bundle.name();

        let response = self
            .client
            .push_tx(spend_bundle)
            .await
            .map_err(coinset_error("push_tx"))?;

        let status = if response.success { 1 } else { 3 };

        Ok(TransactionAck::new(
            transaction_id,
            status,
            response.error,
        ))
    }

    async fn block_timestamp(&self, height: u32) -> Result<(Bytes32, u64), WalletError> {
        let response = self
            .client
            .get_block_record_by_height(height)
            .await
            .map_err(coinset_error("get_block_record_by_height"))?;

        let block = response.block_record.ok_or(WalletError::PeerMisbehaved)?;

        Ok((
            block.header_hash,
            block.timestamp.ok_or(WalletError::PeerMisbehaved)?,
        ))
    }
}

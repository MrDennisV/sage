#![cfg(feature = "coinset")]

use chia_protocol::{Bytes32, CoinStateFilters};
use hex_literal::hex;
use sage_wallet::{CoinsetPeer, PeerApi};

/// Every offer settles through this puzzle, so it always has coins that were
/// created long before they were spent — which is the shape this test needs.
const SETTLEMENT_PAYMENT_HASH: Bytes32 = Bytes32::new(hex!(
    "cfbfdeed5c4ca2de3d0bf520b9cb4bb7743a359bd2e6a188d19ce7dffc21d3e7"
));

/// `previous_height` means "state changes since this height" — the wallet sync
/// relies on it to learn that a coin it already knows about has been spent. The
/// REST API has no such parameter; its `start_height` filters on the height a
/// coin was *created*, so translating one to the other silently drops the spend
/// of every coin older than the cursor and the wallet keeps counting it.
#[tokio::test]
#[ignore = "hits the live mainnet Coinset API"]
async fn test_coinset_delta_reports_spends_of_older_coins() -> anyhow::Result<()> {
    let peer = CoinsetPeer::mainnet();
    let (_height, header_hash) = peer.get_peak().await?;
    let filters = CoinStateFilters::new(true, true, true, 0);

    let all = peer
        .subscribe_puzzles(
            vec![SETTLEMENT_PAYMENT_HASH],
            None,
            header_hash,
            filters.clone(),
        )
        .await?;

    // Pick the coin with the widest gap between creation and spend, so the
    // cursor below lands well inside it.
    let target = all
        .coin_states
        .iter()
        .filter_map(|state| Some((state, state.created_height?, state.spent_height?)))
        .max_by_key(|(_, created, spent)| spent.saturating_sub(*created))
        .map(|(state, created, spent)| (*state, created, spent))
        .expect("no spent coin found to test against");

    let (coin_state, created, spent) = target;
    let cutoff = created + (spent - created) / 2;

    let delta = peer
        .subscribe_puzzles(
            vec![SETTLEMENT_PAYMENT_HASH],
            Some(cutoff),
            header_hash,
            filters,
        )
        .await?;

    let coin_id = coin_state.coin.coin_id();

    assert!(
        delta
            .coin_states
            .iter()
            .any(|state| state.coin.coin_id() == coin_id && state.spent_height.is_some()),
        "a coin created at {created} and spent at {spent} was not reported \
         by a sync resuming from height {cutoff}, so its spend would be lost"
    );

    Ok(())
}

#[tokio::test]
#[ignore = "hits the live mainnet Coinset API"]
async fn test_coinset_puzzle_state_batch() -> anyhow::Result<()> {
    let peer = CoinsetPeer::mainnet();
    let (_height, header_hash) = peer.get_peak().await?;

    // The same shape the wallet sync sends: a full batch of puzzle hashes.
    let puzzle_hashes: Vec<Bytes32> = (0..1000u32)
        .map(|i| {
            let mut bytes = [0u8; 32];
            bytes[28..].copy_from_slice(&i.to_be_bytes());
            Bytes32::new(bytes)
        })
        .collect();

    let response = peer
        .subscribe_puzzles(
            puzzle_hashes,
            None,
            header_hash,
            CoinStateFilters::new(true, true, true, 0),
        )
        .await?;

    println!("decoded {} coin states", response.coin_states.len());

    Ok(())
}

#[tokio::test]
#[ignore = "hits the live testnet11 CoinSet API"]
async fn test_coinset_peak_and_puzzle_state() -> anyhow::Result<()> {
    let peer = CoinsetPeer::testnet11();

    let (height, header_hash) = peer.get_peak().await?;
    assert!(height > 0);
    assert_ne!(header_hash, Bytes32::default());

    let response = peer
        .subscribe_puzzles(
            vec![Bytes32::default()],
            None,
            header_hash,
            CoinStateFilters::new(true, true, true, 0),
        )
        .await?;

    assert!(response.is_finished);
    assert!(response.height > 0);

    Ok(())
}

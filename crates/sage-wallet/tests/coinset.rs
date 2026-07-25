#![cfg(feature = "coinset")]

use chia_protocol::{Bytes32, CoinStateFilters};
use sage_wallet::{CoinsetPeer, PeerApi};

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

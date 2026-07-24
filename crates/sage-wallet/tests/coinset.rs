#![cfg(feature = "coinset")]

use chia_protocol::{Bytes32, CoinStateFilters};
use sage_wallet::{CoinsetPeer, PeerApi};

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

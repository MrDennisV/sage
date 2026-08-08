use std::time::Duration;

use chia_wallet_sdk::types::TESTNET11_CONSTANTS;
use sage_config::Network;
use sage_database::Database;
use tokio::{sync::mpsc, time::sleep};

use crate::{SyncEvent, WalletError, download_nft_uris};

/// How many URIs to download per round.
const BATCH_SIZE: u32 = 25;

#[derive(Debug)]
pub struct NftUriQueue {
    db: Database,
    sync_sender: mpsc::Sender<SyncEvent>,
    network: Network,
}

impl NftUriQueue {
    pub fn new(db: Database, sync_sender: mpsc::Sender<SyncEvent>, network: Network) -> Self {
        Self {
            db,
            sync_sender,
            network,
        }
    }

    pub async fn start(self, delay: Duration) -> Result<(), WalletError> {
        loop {
            self.process_batch().await?;
            sleep(delay).await;
        }
    }

    async fn process_batch(&self) -> Result<(), WalletError> {
        let testnet = self.network.genesis_challenge == TESTNET11_CONSTANTS.genesis_challenge;

        let downloaded = download_nft_uris(&self.db, testnet, BATCH_SIZE).await?;

        if downloaded {
            self.sync_sender.send(SyncEvent::NftData).await.ok();
        }

        Ok(())
    }
}

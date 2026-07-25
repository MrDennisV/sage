use std::time::Duration;

use sage_database::Database;
use tokio::{
    sync::mpsc,
    time::{sleep, timeout},
};

use crate::{SyncEvent, WalletError, refresh_cat_catalog};

#[derive(Debug)]
pub struct CatQueue {
    db: Database,
    testnet: bool,
    sync_sender: mpsc::Sender<SyncEvent>,
}

impl CatQueue {
    pub fn new(db: Database, testnet: bool, sync_sender: mpsc::Sender<SyncEvent>) -> Self {
        Self {
            db,
            testnet,
            sync_sender,
        }
    }

    pub async fn start(self, delay: Duration) -> Result<(), WalletError> {
        loop {
            self.process_batch().await?;
            sleep(delay).await;
        }
    }

    async fn process_batch(&self) -> Result<(), WalletError> {
        let refreshed = timeout(
            Duration::from_secs(120),
            refresh_cat_catalog(&self.db, self.testnet),
        )
        .await??;

        if refreshed {
            self.sync_sender.send(SyncEvent::CatInfo).await.ok();
        }

        Ok(())
    }
}

use std::{sync::Arc, time::Duration};

use chia_wallet_sdk::prelude::*;
use futures_util::{StreamExt, stream::FuturesUnordered};
use sage_database::Database;
use tokio::{
    sync::{Mutex, mpsc},
    time::sleep,
};
use tracing::{debug, info};

use crate::{PeerState, SyncCommand, SyncEvent, WalletError, apply_synced_coins, fetch_puzzles};

#[derive(Debug)]
pub struct PuzzleQueue {
    db: Database,
    genesis_challenge: Bytes32,
    batch_size_per_peer: usize,
    state: Arc<Mutex<PeerState>>,
    sync_sender: mpsc::Sender<SyncEvent>,
    command_sender: mpsc::Sender<SyncCommand>,
}

impl PuzzleQueue {
    pub fn new(
        db: Database,
        genesis_challenge: Bytes32,
        batch_size_per_peer: usize,
        state: Arc<Mutex<PeerState>>,
        sync_sender: mpsc::Sender<SyncEvent>,
        command_sender: mpsc::Sender<SyncCommand>,
    ) -> Self {
        Self {
            db,
            genesis_challenge,
            batch_size_per_peer,
            state,
            sync_sender,
            command_sender,
        }
    }

    pub async fn start(mut self, delay: Duration) -> Result<(), WalletError> {
        loop {
            self.process_batch().await?;
            sleep(delay).await;
        }
    }

    async fn process_batch(&mut self) -> Result<(), WalletError> {
        let peers = self.state.lock().await.peers();

        if peers.is_empty() {
            return Ok(());
        }

        let limit = peers.len() * self.batch_size_per_peer;

        let coin_states = self.db.unsynced_coins(limit).await?;

        if coin_states.is_empty() {
            return Ok(());
        }

        info!(
            "Syncing a batch of {} coins from {} peers",
            coin_states.len(),
            peers.len()
        );

        let mut futures = FuturesUnordered::new();
        let mut remaining = coin_states.into_iter();

        for peer in peers {
            for _ in 0..self.batch_size_per_peer {
                let Some(row) = remaining.next() else {
                    break;
                };

                let peer = peer.clone();
                let genesis_challenge = self.genesis_challenge;
                let is_custody_p2_puzzle_hash = self
                    .db
                    .is_custody_p2_puzzle_hash(row.coin_state.coin.puzzle_hash)
                    .await?;

                futures.push(async move {
                    let result =
                        fetch_puzzles(&peer, genesis_challenge, row, is_custody_p2_puzzle_hash)
                            .await;
                    (peer.socket_addr(), row, result)
                });
            }
        }

        let mut subscriptions = Vec::new();
        let mut send_events = false;

        while let Some((addr, root, synced_coins)) = futures.next().await {
            match synced_coins {
                Ok(synced_coins) => {
                    send_events |=
                        apply_synced_coins(&self.db, &root, synced_coins, &mut subscriptions)
                            .await?;
                }
                Err(error) => {
                    debug!(
                        "Failed to sync {} from peer {}: {}",
                        root.coin_state.coin.coin_id(),
                        addr,
                        error
                    );

                    if matches!(
                        error,
                        WalletError::Elapsed(..)
                            | WalletError::PeerMisbehaved
                            | WalletError::Client(..)
                    ) {
                        self.state.lock().await.ban(
                            addr.ip(),
                            Duration::from_secs(300),
                            "failed puzzle lookup",
                        );
                    }
                }
            }
        }

        if send_events {
            self.command_sender
                .send(SyncCommand::SubscribeCoins {
                    coin_ids: subscriptions,
                })
                .await
                .ok();

            self.sync_sender
                .send(SyncEvent::PuzzleBatchSynced)
                .await
                .ok();
        }
        Ok(())
    }
}

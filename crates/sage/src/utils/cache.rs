use chia_puzzle_types::nft::NftMetadata;
use sage_database::{Asset, AssetKind, SqlExecutor};
use sage_wallet::prelude::*;

use crate::{ConfirmationInfo, Error, ExtractedNftData, Result, Sage, extract_nft_data};

impl<E: SqlExecutor> Sage<E> {
    pub async fn cache_cat(
        &self,
        asset_id: Bytes32,
        hidden_puzzle_hash: Option<Bytes32>,
    ) -> Result<Asset> {
        let wallet = self.wallet()?;

        if let Some(asset) = wallet.db.asset(asset_id).await? {
            return Ok(asset);
        }

        #[cfg(feature = "native")]
        let asset = self.fetch_cat(asset_id, hidden_puzzle_hash).await;

        // There is no metadata API to query without a native HTTP client, so
        // the token is recorded as unknown until a later sync fills it in.
        #[cfg(not(feature = "native"))]
        let asset = unknown_cat(asset_id, hidden_puzzle_hash);

        wallet.db.insert_asset(asset.clone()).await?;

        let mut tx = wallet.db.tx().await?;

        if tx.existing_hidden_puzzle_hash(asset_id).await?.is_none() {
            tx.update_hidden_puzzle_hash(asset_id, asset.hidden_puzzle_hash.or(hidden_puzzle_hash))
                .await?;
        }

        tx.commit().await?;

        Ok(asset)
    }

    pub async fn cache_nft(
        &self,
        allocator: &Allocator,
        launcher_id: Bytes32,
        nft_metadata: NodePtr,
        confirmation_info: &mut ConfirmationInfo,
    ) -> Result<Asset> {
        let wallet = self.wallet()?;

        if let Some(asset) = wallet.db.asset(launcher_id).await? {
            return Ok(asset);
        }

        let info = if let Ok(metadata) = NftMetadata::from_clvm(allocator, nft_metadata) {
            // Off-chain data can only be downloaded natively; elsewhere the
            // extraction below falls back to whatever the database has.
            #[cfg(feature = "native")]
            self.fetch_nft_data(&metadata, confirmation_info).await;

            extract_nft_data(Some(&wallet.db), Some(metadata), confirmation_info).await?
        } else {
            ExtractedNftData::default()
        };

        let asset = Asset {
            hash: launcher_id,
            name: info.name,
            ticker: None,
            precision: 1,
            icon_url: info.icon_url,
            description: info.description,
            is_sensitive_content: info.is_sensitive_content,
            is_visible: true,
            hidden_puzzle_hash: None,
            kind: AssetKind::Nft,
        };

        wallet.db.insert_asset(asset.clone()).await?;

        Ok(asset)
    }

    pub async fn cache_option(&self, launcher_id: Bytes32) -> Result<Asset> {
        let wallet = self.wallet()?;

        let peer = self.acquire_peer().await;

        wallet
            .fetch_offer_option_info(peer.as_ref(), launcher_id)
            .await?;

        let Some(asset) = wallet.db.asset(launcher_id).await? else {
            return Err(Error::MissingOption(launcher_id));
        };

        Ok(asset)
    }
}

/// The placeholder record for a token whose metadata isn't known yet.
fn unknown_cat(asset_id: Bytes32, hidden_puzzle_hash: Option<Bytes32>) -> Asset {
    Asset {
        hash: asset_id,
        name: None,
        ticker: None,
        precision: 3,
        icon_url: None,
        description: None,
        is_sensitive_content: false,
        is_visible: true,
        hidden_puzzle_hash,
        kind: AssetKind::Token,
    }
}

#[cfg(feature = "native")]
mod native {
    use std::{collections::hash_map::Entry, time::Duration};

    use chia_puzzle_types::nft::NftMetadata;
    use sage_assets::{DexieCat, fetch_uris_with_hash};
    use sage_database::{Asset, AssetKind, SqlExecutor};
    use sage_wallet::prelude::*;
    use tokio::time::timeout;

    use crate::{ConfirmationInfo, Sage};

    use super::unknown_cat;

    impl<E: SqlExecutor> Sage<E> {
        /// Looks up token metadata on Dexie, falling back to an unknown token
        /// when the lookup fails or times out.
        pub(super) async fn fetch_cat(
            &self,
            asset_id: Bytes32,
            hidden_puzzle_hash: Option<Bytes32>,
        ) -> Asset {
            let testnet = self.network().genesis_challenge == TESTNET11_CONSTANTS.genesis_challenge;

            if let Ok(Ok(asset)) =
                timeout(Duration::from_secs(5), DexieCat::fetch(asset_id, testnet)).await
            {
                Asset {
                    hash: asset_id,
                    name: asset.name,
                    ticker: asset.ticker,
                    precision: 3,
                    icon_url: asset.icon_url,
                    description: asset.description,
                    is_sensitive_content: false,
                    is_visible: true,
                    hidden_puzzle_hash: asset.hidden_puzzle_hash.or(hidden_puzzle_hash),
                    kind: AssetKind::Token,
                }
            } else {
                unknown_cat(asset_id, hidden_puzzle_hash)
            }
        }

        /// Downloads the off-chain data and metadata blobs referenced by an
        /// NFT, caching them for the summary that follows.
        pub(crate) async fn fetch_nft_data(
            &self,
            metadata: &NftMetadata,
            confirmation_info: &mut ConfirmationInfo,
        ) {
            let testnet = self.network().genesis_challenge == TESTNET11_CONSTANTS.genesis_challenge;

            if let Some(hash) = metadata.data_hash
                && let Entry::Vacant(entry) = confirmation_info.nft_data.entry(hash)
                && let Ok(Some(data)) = timeout(
                    Duration::from_secs(10),
                    fetch_uris_with_hash(metadata.data_uris.clone(), hash, testnet),
                )
                .await
            {
                entry.insert(data);
            }

            if let Some(hash) = metadata.metadata_hash
                && let Entry::Vacant(entry) = confirmation_info.nft_data.entry(hash)
                && let Ok(Some(data)) = timeout(
                    Duration::from_secs(10),
                    fetch_uris_with_hash(metadata.metadata_uris.clone(), hash, testnet),
                )
                .await
            {
                entry.insert(data);
            }
        }
    }
}

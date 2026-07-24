use chia_protocol::{Bytes32, Program};
#[cfg(feature = "sqlite")]
use chia_wallet_sdk::prelude::*;
#[cfg(feature = "sqlite")]
use sqlx::query;

#[cfg(feature = "sqlite")]
use crate::{AssetKind, CoinKind, Convert, Database};
use crate::{Asset, CoinRow, DatabaseTx, Result, SqlAccess, SqlExecutor, sql_file};

#[derive(Debug, Clone)]
pub struct DidCoinInfo {
    pub metadata: Program,
    pub recovery_list_hash: Option<Bytes32>,
    pub num_verifications_required: u64,
}

#[derive(Debug, Clone)]
pub struct DidRow {
    pub asset: Asset,
    pub did_info: DidCoinInfo,
    pub coin_row: CoinRow,
}

#[cfg(feature = "sqlite")]
impl Database {
    pub async fn owned_dids(&self) -> Result<Vec<DidRow>> {
        query!(
            "
            SELECT
                asset_hash, asset_name, asset_ticker, asset_precision, asset_icon_url,
                asset_description, asset_is_visible, asset_is_sensitive_content,
                asset_hidden_puzzle_hash, owned_coins.created_height, spent_height,
                parent_coin_hash, puzzle_hash, amount, p2_puzzle_hash,
                metadata, recovery_list_hash, num_verifications_required,
                offer_hash, created_timestamp, spent_timestamp,
                clawback_expiration_seconds AS clawback_timestamp
            FROM owned_coins
            INNER JOIN dids ON dids.asset_id = owned_coins.asset_id
            ORDER BY asset_name ASC
            "
        )
        .fetch_all(self.pool())
        .await?
        .into_iter()
        .map(|row| {
            Ok(DidRow {
                asset: Asset {
                    hash: row.asset_hash.convert()?,
                    name: row.asset_name,
                    ticker: row.asset_ticker,
                    precision: row.asset_precision.convert()?,
                    icon_url: row.asset_icon_url,
                    description: row.asset_description,
                    is_visible: row.asset_is_visible,
                    is_sensitive_content: row.asset_is_sensitive_content,
                    hidden_puzzle_hash: row.asset_hidden_puzzle_hash.convert()?,
                    kind: AssetKind::Did,
                },
                did_info: DidCoinInfo {
                    metadata: row.metadata.into(),
                    recovery_list_hash: row.recovery_list_hash.convert()?,
                    num_verifications_required: row.num_verifications_required.convert()?,
                },
                coin_row: CoinRow {
                    coin: Coin::new(
                        row.parent_coin_hash.convert()?,
                        row.puzzle_hash.convert()?,
                        row.amount.convert()?,
                    ),
                    p2_puzzle_hash: row.p2_puzzle_hash.convert()?,
                    kind: CoinKind::Did,
                    mempool_item_hash: None,
                    offer_hash: row.offer_hash.convert()?,
                    clawback_timestamp: row.clawback_timestamp.convert()?,
                    created_height: row.created_height.convert()?,
                    spent_height: row.spent_height.convert()?,
                    created_timestamp: row.created_timestamp.convert()?,
                    spent_timestamp: row.spent_timestamp.convert()?,
                },
            })
        })
        .collect()
    }
}

impl<E: SqlExecutor> DatabaseTx<'_, E> {
    pub async fn insert_did(&mut self, hash: Bytes32, coin_info: &DidCoinInfo) -> Result<()> {
        insert_did(&mut self.tx, hash, coin_info).await
    }

    pub async fn update_did(&mut self, hash: Bytes32, coin_info: &DidCoinInfo) -> Result<()> {
        update_did(&mut self.tx, hash, coin_info).await
    }
}

async fn insert_did(mut conn: impl SqlAccess, hash: Bytes32, coin_info: &DidCoinInfo) -> Result<()> {
    let num_verifications_required: i64 = coin_info.num_verifications_required.try_into()?;

    conn.execute(
        sql_file!("dids/insert_did.sql"),
        vec![
            hash.into(),
            coin_info.metadata.as_slice().into(),
            coin_info.recovery_list_hash.into(),
            num_verifications_required.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn update_did(mut conn: impl SqlAccess, hash: Bytes32, coin_info: &DidCoinInfo) -> Result<()> {
    let num_verifications_required: i64 = coin_info.num_verifications_required.try_into()?;

    conn.execute(
        sql_file!("dids/update_did.sql"),
        vec![
            coin_info.metadata.as_slice().into(),
            coin_info.recovery_list_hash.into(),
            num_verifications_required.into(),
            hash.into(),
        ],
    )
    .await?;

    Ok(())
}

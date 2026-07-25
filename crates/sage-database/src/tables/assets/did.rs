use chia_protocol::{Bytes32, Coin, Program};

use crate::{
    Asset, AssetKind, CoinKind, CoinRow, Convert, Database, DatabaseTx, Result, SqlAccess,
    SqlExecutor, sql_file,
};

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

impl<E: SqlExecutor> Database<E> {
    pub async fn owned_dids(&self) -> Result<Vec<DidRow>> {
        owned_dids(&self.executor).await
    }
}

async fn owned_dids(mut conn: impl SqlAccess) -> Result<Vec<DidRow>> {
    conn.fetch_all(sql_file!("dids/owned_dids.sql"), vec![])
        .await?
        .iter()
        .map(|row| {
            Ok(DidRow {
                asset: Asset {
                    hash: row.converted("asset_hash")?,
                    name: row.opt_text("asset_name")?,
                    ticker: row.opt_text("asset_ticker")?,
                    precision: row.i64("asset_precision")?.convert()?,
                    icon_url: row.opt_text("asset_icon_url")?,
                    description: row.opt_text("asset_description")?,
                    is_visible: row.i64("asset_is_visible")? != 0,
                    is_sensitive_content: row.i64("asset_is_sensitive_content")? != 0,
                    hidden_puzzle_hash: row.opt_converted("asset_hidden_puzzle_hash")?,
                    kind: AssetKind::Did,
                },
                did_info: DidCoinInfo {
                    metadata: row.blob("metadata")?.into(),
                    recovery_list_hash: row.opt_converted("recovery_list_hash")?,
                    num_verifications_required: row.i64("num_verifications_required")?.convert()?,
                },
                coin_row: CoinRow {
                    coin: Coin::new(
                        row.converted("parent_coin_hash")?,
                        row.converted("puzzle_hash")?,
                        row.converted("amount")?,
                    ),
                    p2_puzzle_hash: row.converted("p2_puzzle_hash")?,
                    kind: CoinKind::Did,
                    mempool_item_hash: None,
                    offer_hash: row.opt_converted("offer_hash")?,
                    clawback_timestamp: row.opt_i64("clawback_timestamp")?.convert()?,
                    created_height: row.opt_i64("created_height")?.convert()?,
                    spent_height: row.opt_i64("spent_height")?.convert()?,
                    created_timestamp: row.opt_i64("created_timestamp")?.convert()?,
                    spent_timestamp: row.opt_i64("spent_timestamp")?.convert()?,
                },
            })
        })
        .collect()
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

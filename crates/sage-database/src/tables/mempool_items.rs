use chia_bls::Signature;
use chia_protocol::{Bytes32, Coin, CoinSpend};

use crate::{Database, DatabaseTx, Result, SqlAccess, SqlExecutor, SqlRow, sql_file};

#[derive(Debug, Clone)]
pub struct MempoolItem {
    pub hash: Bytes32,
    pub aggregated_signature: Signature,
    pub fee: u64,
    pub submitted_timestamp: Option<u64>,
}

impl<E: SqlExecutor> Database<E> {
    pub async fn mempool_items_to_submit(
        &self,
        check_every_seconds: i64,
        limit: i64,
    ) -> Result<Vec<MempoolItem>> {
        mempool_items_to_submit(&self.executor, check_every_seconds, limit).await
    }

    pub async fn mempool_coin_spends(&self, mempool_item_id: Bytes32) -> Result<Vec<CoinSpend>> {
        mempool_coin_spends(&self.executor, mempool_item_id).await
    }

    pub async fn update_mempool_item_time(&self, mempool_item_id: Bytes32) -> Result<()> {
        update_mempool_item_time(&self.executor, mempool_item_id).await
    }

    pub async fn mempool_items(&self) -> Result<Vec<MempoolItem>> {
        mempool_items(&self.executor).await
    }
}

impl<E: SqlExecutor> DatabaseTx<'_, E> {
    pub async fn insert_mempool_item(
        &mut self,
        hash: Bytes32,
        aggregated_signature: Signature,
        fee: u64,
    ) -> Result<()> {
        insert_mempool_item(&mut self.tx, hash, aggregated_signature, fee).await
    }

    pub async fn insert_mempool_coin(
        &mut self,
        mempool_item_id: Bytes32,
        coin_id: Bytes32,
        is_input: bool,
        is_output: bool,
    ) -> Result<()> {
        insert_mempool_coin(&mut self.tx, mempool_item_id, coin_id, is_input, is_output).await
    }

    pub async fn insert_mempool_spend(
        &mut self,
        mempool_item_id: Bytes32,
        coin_spend: CoinSpend,
        seq: usize,
    ) -> Result<()> {
        insert_mempool_spend(&mut self.tx, mempool_item_id, coin_spend, seq).await
    }

    pub async fn mempool_items_for_input(&mut self, coin_id: Bytes32) -> Result<Vec<Bytes32>> {
        mempool_items_for_input(&mut self.tx, coin_id).await
    }

    pub async fn mempool_items_for_output(&mut self, coin_id: Bytes32) -> Result<Vec<Bytes32>> {
        mempool_items_for_output(&mut self.tx, coin_id).await
    }

    pub async fn remove_mempool_item(&mut self, mempool_item_id: Bytes32) -> Result<()> {
        remove_mempool_item(&mut self.tx, mempool_item_id).await
    }
}

async fn insert_mempool_item(
    mut conn: impl SqlAccess,
    hash: Bytes32,
    aggregated_signature: Signature,
    fee: u64,
) -> Result<()> {
    conn.execute(
        sql_file!("mempool_items/insert_mempool_item.sql"),
        vec![
            hash.into(),
            aggregated_signature.into(),
            fee.to_be_bytes().to_vec().into(),
        ],
    )
    .await?;

    Ok(())
}

async fn insert_mempool_coin(
    mut conn: impl SqlAccess,
    mempool_item_id: Bytes32,
    coin_id: Bytes32,
    is_input: bool,
    is_output: bool,
) -> Result<()> {
    conn.execute(
        sql_file!("mempool_items/insert_mempool_coin.sql"),
        vec![
            mempool_item_id.into(),
            coin_id.into(),
            is_input.into(),
            is_output.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn insert_mempool_spend(
    mut conn: impl SqlAccess,
    mempool_item_id: Bytes32,
    coin_spend: CoinSpend,
    seq: usize,
) -> Result<()> {
    let coin_id = coin_spend.coin.coin_id();
    let seq: i64 = seq.try_into()?;

    conn.execute(
        sql_file!("mempool_items/insert_mempool_spend.sql"),
        vec![
            mempool_item_id.into(),
            coin_id.into(),
            coin_spend.coin.parent_coin_info.into(),
            coin_spend.coin.puzzle_hash.into(),
            coin_spend.coin.amount.to_be_bytes().to_vec().into(),
            coin_spend.puzzle_reveal.into_bytes().into(),
            coin_spend.solution.into_bytes().into(),
            seq.into(),
        ],
    )
    .await?;

    Ok(())
}

/// Decodes the columns shared by the `mempool_items_to_submit` and
/// `mempool_items` queries.
fn mempool_item_from_row(row: &SqlRow) -> Result<MempoolItem> {
    Ok(MempoolItem {
        hash: row.converted("hash")?,
        aggregated_signature: row.converted("aggregated_signature")?,
        fee: row.converted("fee")?,
        submitted_timestamp: row.opt_i64("submitted_timestamp")?.map(|ts| ts as u64),
    })
}

async fn mempool_items_to_submit(
    mut conn: impl SqlAccess,
    check_every_seconds: i64,
    limit: i64,
) -> Result<Vec<MempoolItem>> {
    conn.fetch_all(
        sql_file!("mempool_items/mempool_items_to_submit.sql"),
        vec![check_every_seconds.into(), limit.into()],
    )
    .await?
    .iter()
    .map(mempool_item_from_row)
    .collect()
}

async fn mempool_coin_spends(
    mut conn: impl SqlAccess,
    mempool_item_id: Bytes32,
) -> Result<Vec<CoinSpend>> {
    conn.fetch_all(
        sql_file!("mempool_items/mempool_coin_spends.sql"),
        vec![mempool_item_id.into()],
    )
    .await?
    .iter()
    .map(|row| {
        Ok(CoinSpend::new(
            Coin::new(
                row.converted("parent_coin_hash")?,
                row.converted("puzzle_hash")?,
                row.converted("amount")?,
            ),
            row.blob("puzzle_reveal")?.into(),
            row.blob("solution")?.into(),
        ))
    })
    .collect()
}

async fn mempool_items_for_input(
    mut conn: impl SqlAccess,
    coin_id: Bytes32,
) -> Result<Vec<Bytes32>> {
    conn.fetch_all(
        sql_file!("mempool_items/mempool_items_for_input.sql"),
        vec![coin_id.into()],
    )
    .await?
    .iter()
    .map(|row| row.converted("mempool_item_hash"))
    .collect()
}

async fn mempool_items_for_output(
    mut conn: impl SqlAccess,
    coin_id: Bytes32,
) -> Result<Vec<Bytes32>> {
    conn.fetch_all(
        sql_file!("mempool_items/mempool_items_for_output.sql"),
        vec![coin_id.into()],
    )
    .await?
    .iter()
    .map(|row| row.converted("mempool_item_hash"))
    .collect()
}

async fn remove_mempool_item(mut conn: impl SqlAccess, mempool_item_id: Bytes32) -> Result<()> {
    conn.execute(
        sql_file!("mempool_items/remove_mempool_coins.sql"),
        vec![mempool_item_id.into()],
    )
    .await?;

    conn.execute(
        sql_file!("mempool_items/remove_mempool_item.sql"),
        vec![mempool_item_id.into()],
    )
    .await?;

    Ok(())
}

async fn update_mempool_item_time(
    mut conn: impl SqlAccess,
    mempool_item_id: Bytes32,
) -> Result<()> {
    conn.execute(
        sql_file!("mempool_items/update_mempool_item_time.sql"),
        vec![mempool_item_id.into()],
    )
    .await?;

    Ok(())
}

async fn mempool_items(mut conn: impl SqlAccess) -> Result<Vec<MempoolItem>> {
    conn.fetch_all(sql_file!("mempool_items/mempool_items.sql"), vec![])
        .await?
        .iter()
        .map(mempool_item_from_row)
        .collect()
}

use chia_bls::Signature;
#[cfg(feature = "sqlite")]
use chia_protocol::Coin;
use chia_protocol::{Bytes32, CoinSpend};
#[cfg(feature = "sqlite")]
use sqlx::{SqliteExecutor, query};

#[cfg(feature = "sqlite")]
use crate::{Convert, Database};
use crate::{DatabaseTx, Result, SqlAccess, SqlExecutor, sql_file};

#[derive(Debug, Clone)]
pub struct MempoolItem {
    pub hash: Bytes32,
    pub aggregated_signature: Signature,
    pub fee: u64,
    pub submitted_timestamp: Option<u64>,
}

#[cfg(feature = "sqlite")]
impl Database {
    pub async fn mempool_items_to_submit(
        &self,
        check_every_seconds: i64,
        limit: i64,
    ) -> Result<Vec<MempoolItem>> {
        mempool_items_to_submit(self.pool(), check_every_seconds, limit).await
    }

    pub async fn mempool_coin_spends(&self, mempool_item_id: Bytes32) -> Result<Vec<CoinSpend>> {
        mempool_coin_spends(self.pool(), mempool_item_id).await
    }

    pub async fn update_mempool_item_time(&self, mempool_item_id: Bytes32) -> Result<()> {
        update_mempool_item_time(self.pool(), mempool_item_id).await
    }

    pub async fn mempool_items(&self) -> Result<Vec<MempoolItem>> {
        mempool_items(self.pool()).await
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

#[cfg(feature = "sqlite")]
async fn mempool_items_to_submit(
    conn: impl SqliteExecutor<'_>,
    check_every_seconds: i64,
    limit: i64,
) -> Result<Vec<MempoolItem>> {
    query!(
        "
        SELECT hash, aggregated_signature, fee, submitted_timestamp
        FROM mempool_items
        WHERE submitted_timestamp IS NULL OR unixepoch() - submitted_timestamp >= ?
        LIMIT ?
        ",
        check_every_seconds,
        limit
    )
    .fetch_all(conn)
    .await?
    .into_iter()
    .map(|row| {
        Ok(MempoolItem {
            hash: row.hash.convert()?,
            aggregated_signature: row.aggregated_signature.convert()?,
            fee: row.fee.convert()?,
            submitted_timestamp: row.submitted_timestamp.map(|ts| ts as u64),
        })
    })
    .collect()
}

#[cfg(feature = "sqlite")]
async fn mempool_coin_spends(
    conn: impl SqliteExecutor<'_>,
    mempool_item_id: Bytes32,
) -> Result<Vec<CoinSpend>> {
    let mempool_item_id = mempool_item_id.as_ref();

    query!(
        "
        SELECT parent_coin_hash, puzzle_hash, amount, puzzle_reveal, solution
        FROM mempool_spends
        INNER JOIN mempool_items ON mempool_items.id = mempool_spends.mempool_item_id
        WHERE mempool_items.hash = ?
        ORDER BY seq ASC
        ",
        mempool_item_id
    )
    .fetch_all(conn)
    .await?
    .into_iter()
    .map(|row| {
        Ok(CoinSpend::new(
            Coin::new(
                row.parent_coin_hash.convert()?,
                row.puzzle_hash.convert()?,
                row.amount.convert()?,
            ),
            row.puzzle_reveal.into(),
            row.solution.into(),
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

#[cfg(feature = "sqlite")]
async fn update_mempool_item_time(
    conn: impl SqliteExecutor<'_>,
    mempool_item_id: Bytes32,
) -> Result<()> {
    let mempool_item_id = mempool_item_id.as_ref();

    query!(
        "UPDATE mempool_items SET submitted_timestamp = unixepoch() WHERE hash = ?",
        mempool_item_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

#[cfg(feature = "sqlite")]
async fn mempool_items(conn: impl SqliteExecutor<'_>) -> Result<Vec<MempoolItem>> {
    query!(
        "
        SELECT hash, aggregated_signature, fee, submitted_timestamp
        FROM mempool_items
        ORDER BY submitted_timestamp DESC, hash ASC
        ",
    )
    .fetch_all(conn)
    .await?
    .into_iter()
    .map(|row| {
        Ok(MempoolItem {
            hash: row.hash.convert()?,
            aggregated_signature: row.aggregated_signature.convert()?,
            fee: row.fee.convert()?,
            submitted_timestamp: row.submitted_timestamp.map(|ts| ts as u64),
        })
    })
    .collect()
}

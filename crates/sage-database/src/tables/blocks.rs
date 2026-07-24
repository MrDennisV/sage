use chia_protocol::Bytes32;
#[cfg(feature = "sqlite")]
use sqlx::SqliteExecutor;

use crate::{
    Convert, Database, DatabaseTx, Result, SqlAccess, SqlExecutor, sql_file,
};

#[cfg(feature = "sqlite")]
impl Database {
    pub async fn unsynced_blocks(&self, limit: u32) -> Result<Vec<u32>> {
        unsynced_blocks(self.pool(), limit).await
    }
}

impl<E: SqlExecutor> Database<E> {
    pub async fn insert_block(
        &self,
        height: u32,
        header_hash: Bytes32,
        timestamp: Option<i64>,
        is_peak: bool,
    ) -> Result<()> {
        insert_block(&self.executor, height, header_hash, timestamp, is_peak).await
    }

    pub async fn latest_peak(&self) -> Result<Option<(u32, Bytes32)>> {
        latest_peak(&self.executor).await
    }
}

impl<E: SqlExecutor> DatabaseTx<'_, E> {
    pub async fn insert_height(&mut self, height: u32) -> Result<()> {
        insert_height(&mut self.tx, height).await
    }
}

async fn insert_height(mut conn: impl SqlAccess, height: u32) -> Result<()> {
    conn.execute(sql_file!("blocks/insert_height.sql"), vec![height.into()])
        .await?;

    Ok(())
}

#[cfg(feature = "sqlite")]
async fn unsynced_blocks(conn: impl SqliteExecutor<'_>, limit: u32) -> Result<Vec<u32>> {
    let row = sqlx::query!(
        "
        SELECT created_height AS height FROM coins
        INNER JOIN blocks ON blocks.height = coins.created_height
        WHERE blocks.timestamp IS NULL
        UNION
        SELECT spent_height AS height FROM coins
        INNER JOIN blocks ON blocks.height = coins.spent_height
        WHERE blocks.timestamp IS NULL
        ORDER BY height DESC
        LIMIT ?
        ",
        limit
    )
    .fetch_all(conn)
    .await?;

    row.into_iter()
        .filter_map(|r| r.height.convert().transpose())
        .collect()
}

async fn insert_block(
    mut conn: impl SqlAccess,
    height: u32,
    header_hash: Bytes32,
    unix_timestamp: Option<i64>,
    is_peak: bool,
) -> Result<()> {
    conn.execute(
        sql_file!("blocks/insert_block.sql"),
        vec![
            height.into(),
            unix_timestamp.into(),
            header_hash.into(),
            is_peak.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn latest_peak(mut conn: impl SqlAccess) -> Result<Option<(u32, Bytes32)>> {
    let rows = conn.fetch_all(sql_file!("blocks/latest_peak.sql"), vec![]).await?;

    let Some(row) = rows.first() else {
        return Ok(None);
    };

    let Some(header_hash) = row.opt_blob("header_hash")? else {
        return Ok(None);
    };

    Ok(Some((row.i64("height")?.convert()?, header_hash.convert()?)))
}

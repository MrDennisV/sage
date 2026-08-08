use crate::{Database, Result, SqlAccess, SqlExecutor, sql_file};

/// A group of records a resync can clear before the wallet syncs again. The
/// caller decides which groups to drop; the mempool and the peak marker always
/// go, since a resync replays them from the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResyncRecords {
    Coins,
    Assets,
    Files,
    Offers,
    Addresses,
    Blocks,
}

impl<E: SqlExecutor> Database<E> {
    /// Clears the requested record groups so the next sync refills them.
    /// Reclaiming the space they occupied is left to the caller, because it is
    /// a file-level operation that only the native build has.
    pub async fn resync(&self, records: &[ResyncRecords]) -> Result<()> {
        delete_mempool_items(&self.executor).await?;
        clear_peak(&self.executor).await?;

        for record in records {
            match record {
                ResyncRecords::Coins => delete_coins(&self.executor).await?,
                ResyncRecords::Assets => {
                    // The XCH asset and the placeholder collection are row zero
                    // in their tables and the schema depends on them existing.
                    delete_assets(&self.executor).await?;
                    delete_collections(&self.executor).await?;
                }
                ResyncRecords::Files => delete_files(&self.executor).await?,
                ResyncRecords::Offers => delete_offers(&self.executor).await?,
                ResyncRecords::Addresses => delete_p2_puzzles(&self.executor).await?,
                ResyncRecords::Blocks => delete_blocks(&self.executor).await?,
            }
        }

        Ok(())
    }
}

async fn delete_mempool_items(mut conn: impl SqlAccess) -> Result<()> {
    conn.execute(sql_file!("mempool_items/delete_mempool_items.sql"), vec![])
        .await?;

    Ok(())
}

async fn clear_peak(mut conn: impl SqlAccess) -> Result<()> {
    conn.execute(sql_file!("blocks/clear_peak.sql"), vec![])
        .await?;

    Ok(())
}

async fn delete_coins(mut conn: impl SqlAccess) -> Result<()> {
    conn.execute(sql_file!("coins/delete_coins.sql"), vec![])
        .await?;

    Ok(())
}

async fn delete_assets(mut conn: impl SqlAccess) -> Result<()> {
    conn.execute(sql_file!("assets/delete_assets.sql"), vec![])
        .await?;

    Ok(())
}

async fn delete_collections(mut conn: impl SqlAccess) -> Result<()> {
    conn.execute(sql_file!("collections/delete_collections.sql"), vec![])
        .await?;

    Ok(())
}

async fn delete_files(mut conn: impl SqlAccess) -> Result<()> {
    conn.execute(sql_file!("files/delete_files.sql"), vec![])
        .await?;

    Ok(())
}

async fn delete_offers(mut conn: impl SqlAccess) -> Result<()> {
    conn.execute(sql_file!("offers/delete_offers.sql"), vec![])
        .await?;

    Ok(())
}

async fn delete_p2_puzzles(mut conn: impl SqlAccess) -> Result<()> {
    conn.execute(sql_file!("p2_puzzles/delete_p2_puzzles.sql"), vec![])
        .await?;

    Ok(())
}

async fn delete_blocks(mut conn: impl SqlAccess) -> Result<()> {
    conn.execute(sql_file!("blocks/delete_blocks.sql"), vec![])
        .await?;

    Ok(())
}

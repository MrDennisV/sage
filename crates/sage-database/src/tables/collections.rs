use crate::{Convert, Database, DatabaseTx, Result, SqlAccess, SqlExecutor, SqlRow, sql_file};
use chia_protocol::Bytes32;

#[derive(Debug, Clone)]
pub struct CollectionRow {
    pub hash: Bytes32,
    pub uuid: String,
    pub minter_hash: Bytes32,
    pub name: Option<String>,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub description: Option<String>,
    pub is_visible: bool,
}

impl<E: SqlExecutor> Database<E> {
    pub async fn collections(
        &self,
        limit: u32,
        offset: u32,
        include_hidden: bool,
    ) -> Result<(Vec<CollectionRow>, u32)> {
        collections(&self.executor, limit, offset, include_hidden).await
    }

    pub async fn collection(&self, hash: Bytes32) -> Result<Option<CollectionRow>> {
        collection(&self.executor, hash).await
    }

    pub async fn set_collection_visible(&self, hash: Bytes32, visible: bool) -> Result<()> {
        set_collection_visible(&self.executor, hash, visible).await
    }
}

impl<E: SqlExecutor> DatabaseTx<'_, E> {
    pub async fn set_collection_visible(&mut self, hash: Bytes32, visible: bool) -> Result<()> {
        set_collection_visible(&mut self.tx, hash, visible).await
    }

    pub async fn insert_collection(&mut self, row: CollectionRow) -> Result<()> {
        insert_collection(&mut self.tx, row).await
    }
}

/// Decodes the columns shared by the `collection` and `collections` queries.
fn collection_row_from_row(row: &SqlRow) -> Result<CollectionRow> {
    Ok(CollectionRow {
        hash: row.converted("hash")?,
        uuid: row.text("uuid")?,
        minter_hash: row.converted("minter_hash")?,
        name: row.opt_text("name")?,
        icon_url: row.opt_text("icon_url")?,
        banner_url: row.opt_text("banner_url")?,
        description: row.opt_text("description")?,
        is_visible: row.i64("is_visible")? != 0,
    })
}

async fn collection(mut conn: impl SqlAccess, hash: Bytes32) -> Result<Option<CollectionRow>> {
    conn.fetch_all(sql_file!("collections/collection.sql"), vec![hash.into()])
        .await?
        .first()
        .map(collection_row_from_row)
        .transpose()
}

async fn collections(
    mut conn: impl SqlAccess,
    limit: u32,
    offset: u32,
    include_hidden: bool,
) -> Result<(Vec<CollectionRow>, u32)> {
    // we only return collections that have nfts
    let rows = conn
        .fetch_all(
            sql_file!("collections/collections.sql"),
            vec![include_hidden.into(), limit.into(), offset.into()],
        )
        .await?;

    let total_count = rows
        .first()
        .map_or(Ok(0), |row| row.i64("total_count")?.convert())?;

    let collections = rows
        .iter()
        .map(collection_row_from_row)
        .collect::<Result<Vec<_>>>()?;

    Ok((collections, total_count))
}

async fn insert_collection(mut conn: impl SqlAccess, row: CollectionRow) -> Result<()> {
    conn.execute(
        sql_file!("collections/insert_collection.sql"),
        vec![
            row.hash.into(),
            row.uuid.into(),
            row.minter_hash.into(),
            row.name.into(),
            row.icon_url.into(),
            row.banner_url.into(),
            row.description.into(),
            row.is_visible.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn set_collection_visible(
    mut conn: impl SqlAccess,
    hash: Bytes32,
    visible: bool,
) -> Result<()> {
    conn.execute(
        sql_file!("collections/set_collection_visible.sql"),
        vec![visible.into(), hash.into()],
    )
    .await?;

    Ok(())
}

#[cfg(feature = "sqlite")]
use crate::{Convert, Database};
use crate::{DatabaseTx, Result, SqlAccess, SqlExecutor, sql_file};
use chia_protocol::Bytes32;
#[cfg(feature = "sqlite")]
use sqlx::{SqliteExecutor, query};

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

#[cfg(feature = "sqlite")]
impl Database {
    pub async fn collections(
        &self,
        limit: u32,
        offset: u32,
        include_hidden: bool,
    ) -> Result<(Vec<CollectionRow>, u32)> {
        collections(self.pool(), limit, offset, include_hidden).await
    }

    pub async fn collection(&self, hash: Bytes32) -> Result<Option<CollectionRow>> {
        collection(self.pool(), hash).await
    }

    pub async fn set_collection_visible(&self, hash: Bytes32, visible: bool) -> Result<()> {
        set_collection_visible(self.pool(), hash, visible).await
    }
}

#[cfg(feature = "sqlite")]
impl DatabaseTx<'_> {
    pub async fn set_collection_visible(&mut self, hash: Bytes32, visible: bool) -> Result<()> {
        set_collection_visible(&mut *self.tx, hash, visible).await
    }
}

impl<E: SqlExecutor> DatabaseTx<'_, E> {
    pub async fn insert_collection(&mut self, row: CollectionRow) -> Result<()> {
        insert_collection(&mut self.tx, row).await
    }
}

#[cfg(feature = "sqlite")]
async fn collection(conn: impl SqliteExecutor<'_>, hash: Bytes32) -> Result<Option<CollectionRow>> {
    let hash_ref = hash.as_ref();
    let row = query!(
        "SELECT id, hash, uuid, minter_hash, name, icon_url, banner_url, description, is_visible 
        FROM collections
        WHERE hash = ?",
        hash_ref
    )
    .fetch_optional(conn)
    .await?;

    row.map(|row| {
        Ok(CollectionRow {
            hash: row.hash.convert()?,
            uuid: row.uuid,
            minter_hash: row.minter_hash.convert()?,
            name: row.name,
            icon_url: row.icon_url,
            banner_url: row.banner_url,
            description: row.description,
            is_visible: row.is_visible,
        })
    })
    .transpose()
}

#[cfg(feature = "sqlite")]
async fn collections(
    conn: impl SqliteExecutor<'_>,
    limit: u32,
    offset: u32,
    include_hidden: bool,
) -> Result<(Vec<CollectionRow>, u32)> {
    // we only return collections that have nfts
    let rows = query!(
        "SELECT collections.hash, uuid, collections.minter_hash, collections.name, collections.icon_url, 
        collections.banner_url, collections.description, collections.is_visible, COUNT(*) OVER() as total_count
        FROM collections
        WHERE 1=1
        AND EXISTS (SELECT 1 FROM owned_nfts WHERE owned_nfts.collection_id = collections.id)
        AND (? OR is_visible = 1)
        ORDER BY CASE WHEN collections.id = 0 THEN 1 ELSE 0 END, name ASC
        LIMIT ?
        OFFSET ?",
        include_hidden,
        limit,
        offset
    )
    .fetch_all(conn)
    .await?;

    let total_count = rows
        .first()
        .map_or(Ok(0), |row| row.total_count.try_into())?;

    let collections = rows
        .into_iter()
        .map(|row| {
            Ok(CollectionRow {
                hash: row.hash.convert()?,
                uuid: row.uuid,
                minter_hash: row.minter_hash.convert()?,
                name: row.name,
                icon_url: row.icon_url,
                banner_url: row.banner_url,
                description: row.description,
                is_visible: row.is_visible,
            })
        })
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

#[cfg(feature = "sqlite")]
async fn set_collection_visible(
    conn: impl SqliteExecutor<'_>,
    hash: Bytes32,
    visible: bool,
) -> Result<()> {
    let hash_ref = hash.as_ref();
    query!(
        "UPDATE collections SET is_visible = ? WHERE hash = ?",
        visible,
        hash_ref
    )
    .execute(conn)
    .await?;

    Ok(())
}

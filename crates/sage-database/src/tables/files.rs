use chia_protocol::Bytes32;

use crate::{
    Convert, Database, DatabaseError, DatabaseTx, Result, SqlAccess, SqlExecutor, sql_file,
};

#[derive(Debug, Clone)]
pub struct FileUri {
    pub hash: Bytes32,
    pub uri: String,
    pub last_checked_timestamp: Option<u64>,
    pub failed_attempts: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct UpdateableNft {
    pub hash: Bytes32,
    pub minter_hash: Option<Bytes32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResizedImageKind {
    Icon,
    Thumbnail,
}

#[derive(Debug, Clone)]
pub struct FileData {
    pub hash: Bytes32,
    pub data: Vec<u8>,
    pub mime_type: String,
    pub is_hash_match: bool,
}

#[derive(Debug, Clone)]
pub struct ResizedImage {
    pub data: Vec<u8>,
    pub mime_type: Option<String>,
}

impl<E: SqlExecutor> Database<E> {
    pub async fn candidates_for_download(
        &self,
        check_every_seconds: i64,
        max_failed_attempts: u32,
        limit: u32,
    ) -> Result<Vec<FileUri>> {
        candidates_for_download(
            &self.executor,
            check_every_seconds,
            max_failed_attempts,
            limit,
        )
        .await
    }

    pub async fn full_file_data(&self, hash: Bytes32) -> Result<Option<FileData>> {
        full_file_data(&self.executor, hash).await
    }

    pub async fn checked_files(&self) -> Result<u64> {
        checked_files(&self.executor).await
    }

    pub async fn total_files(&self) -> Result<u64> {
        total_files(&self.executor).await
    }

    pub async fn thumbnail(&self, hash: Bytes32) -> Result<Option<ResizedImage>> {
        resized_image(&self.executor, hash, ResizedImageKind::Thumbnail).await
    }

    pub async fn icon(&self, hash: Bytes32) -> Result<Option<ResizedImage>> {
        resized_image(&self.executor, hash, ResizedImageKind::Icon).await
    }
}

impl<E: SqlExecutor> DatabaseTx<'_, E> {
    pub async fn insert_file(&mut self, hash: Bytes32) -> Result<()> {
        insert_file(&mut self.tx, hash).await
    }

    pub async fn insert_file_uri(&mut self, hash: Bytes32, uri: String) -> Result<()> {
        insert_file_uri(&mut self.tx, hash, uri).await
    }

    pub async fn file_data(&mut self, hash: Bytes32) -> Result<Option<Vec<u8>>> {
        file_data(&mut self.tx, hash).await
    }

    pub async fn icon(&mut self, hash: Bytes32) -> Result<Option<ResizedImage>> {
        resized_image(&mut self.tx, hash, ResizedImageKind::Icon).await
    }

    pub async fn update_checked_uri(&mut self, hash: Bytes32, uri: String) -> Result<()> {
        update_checked_uri(&mut self.tx, hash, uri).await
    }

    pub async fn update_failed_uri(&mut self, hash: Bytes32, uri: String) -> Result<()> {
        update_failed_uri(&mut self.tx, hash, uri).await
    }

    pub async fn update_file(
        &mut self,
        hash: Bytes32,
        data: Vec<u8>,
        mime_type: String,
        is_hash_match: bool,
    ) -> Result<()> {
        update_file(&mut self.tx, hash, data, mime_type, is_hash_match).await
    }

    pub async fn insert_resized_image(
        &mut self,
        file_hash: Bytes32,
        kind: ResizedImageKind,
        data: Vec<u8>,
    ) -> Result<()> {
        insert_resized_image(&mut self.tx, file_hash, kind, data).await
    }

    pub async fn nfts_with_metadata_hash(&mut self, hash: Bytes32) -> Result<Vec<UpdateableNft>> {
        nfts_with_metadata_hash(&mut self.tx, hash).await
    }

    pub async fn delete_file_data(&mut self, hash: Bytes32) -> Result<()> {
        delete_file_data(&mut self.tx, hash).await
    }

    pub async fn set_uri_unchecked(&mut self, uri: String) -> Result<()> {
        set_uri_unchecked(&mut self.tx, uri).await
    }
}

async fn insert_file(mut conn: impl SqlAccess, hash: Bytes32) -> Result<()> {
    conn.execute(sql_file!("files/insert_file.sql"), vec![hash.into()])
        .await?;

    Ok(())
}

async fn insert_file_uri(mut conn: impl SqlAccess, hash: Bytes32, uri: String) -> Result<()> {
    conn.execute(
        sql_file!("files/insert_file_uri.sql"),
        vec![hash.into(), uri.into()],
    )
    .await?;

    Ok(())
}

async fn file_data(mut conn: impl SqlAccess, hash: Bytes32) -> Result<Option<Vec<u8>>> {
    Ok(conn
        .fetch_all(sql_file!("files/file_data.sql"), vec![hash.into()])
        .await?
        .first()
        .map(|row| row.opt_blob("data"))
        .transpose()?
        .flatten())
}

async fn full_file_data(mut conn: impl SqlAccess, hash: Bytes32) -> Result<Option<FileData>> {
    conn.fetch_all(sql_file!("files/full_file_data.sql"), vec![hash.into()])
        .await?
        .first()
        .map(|row| {
            Ok(FileData {
                hash: row.converted("hash")?,
                data: row.opt_blob("data")?.unwrap_or_default(),
                mime_type: row.opt_text("mime_type")?.unwrap_or_default(),
                is_hash_match: row.opt_i64("is_hash_match")?.unwrap_or_default() != 0,
            })
        })
        .transpose()
}

async fn candidates_for_download(
    mut conn: impl SqlAccess,
    check_every_seconds: i64,
    max_failed_attempts: u32,
    limit: u32,
) -> Result<Vec<FileUri>> {
    conn.fetch_all(
        sql_file!("files/candidates_for_download.sql"),
        vec![
            check_every_seconds.into(),
            max_failed_attempts.into(),
            limit.into(),
        ],
    )
    .await?
    .iter()
    .map(|row| {
        Ok(FileUri {
            hash: row.converted("hash")?,
            uri: row.text("uri")?,
            last_checked_timestamp: row.opt_i64("last_checked_timestamp")?.convert()?,
            failed_attempts: row.i64("failed_attempts")?.convert()?,
        })
    })
    .collect()
}

async fn update_failed_uri(mut conn: impl SqlAccess, hash: Bytes32, uri: String) -> Result<()> {
    conn.execute(
        sql_file!("files/update_failed_uri.sql"),
        vec![hash.into(), uri.into()],
    )
    .await?;

    Ok(())
}

async fn update_checked_uri(mut conn: impl SqlAccess, hash: Bytes32, uri: String) -> Result<()> {
    conn.execute(
        sql_file!("files/update_checked_uri.sql"),
        vec![hash.into(), uri.into()],
    )
    .await?;

    Ok(())
}

async fn update_file(
    mut conn: impl SqlAccess,
    hash: Bytes32,
    data: Vec<u8>,
    mime_type: String,
    is_hash_match: bool,
) -> Result<()> {
    conn.execute(
        sql_file!("files/update_file.sql"),
        vec![
            data.into(),
            mime_type.into(),
            is_hash_match.into(),
            hash.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn nfts_with_metadata_hash(
    mut conn: impl SqlAccess,
    hash: Bytes32,
) -> Result<Vec<UpdateableNft>> {
    conn.fetch_all(
        sql_file!("files/nfts_with_metadata_hash.sql"),
        vec![hash.into()],
    )
    .await?
    .iter()
    .map(|row| {
        Ok(UpdateableNft {
            hash: row.converted("hash")?,
            minter_hash: row.opt_converted("minter_hash")?,
        })
    })
    .collect()
}

async fn insert_resized_image(
    mut conn: impl SqlAccess,
    file_hash: Bytes32,
    kind: ResizedImageKind,
    data: Vec<u8>,
) -> Result<()> {
    conn.execute(
        sql_file!("files/insert_resized_image.sql"),
        vec![file_hash.into(), (kind as i64).into(), data.into()],
    )
    .await?;

    Ok(())
}

async fn resized_image(
    mut conn: impl SqlAccess,
    hash: Bytes32,
    kind: ResizedImageKind,
) -> Result<Option<ResizedImage>> {
    conn.fetch_all(
        sql_file!("files/resized_image.sql"),
        vec![hash.into(), (kind as i64).into()],
    )
    .await?
    .first()
    .map(|row| {
        Ok(ResizedImage {
            data: row.blob("data")?,
            mime_type: row.opt_text("mime_type")?,
        })
    })
    .transpose()
}

async fn delete_file_data(mut conn: impl SqlAccess, hash: Bytes32) -> Result<()> {
    conn.execute(sql_file!("files/delete_file_data.sql"), vec![hash.into()])
        .await?;

    Ok(())
}

async fn set_uri_unchecked(mut conn: impl SqlAccess, uri: String) -> Result<()> {
    conn.execute(sql_file!("files/set_uri_unchecked.sql"), vec![uri.into()])
        .await?;

    Ok(())
}

async fn checked_files(mut conn: impl SqlAccess) -> Result<u64> {
    conn.fetch_all(sql_file!("files/checked_files.sql"), vec![])
        .await?
        .first()
        .ok_or(DatabaseError::RowNotFound)?
        .i64("count")?
        .try_into()
        .map_err(DatabaseError::PrecisionLost)
}

async fn total_files(mut conn: impl SqlAccess) -> Result<u64> {
    conn.fetch_all(sql_file!("files/total_files.sql"), vec![])
        .await?
        .first()
        .ok_or(DatabaseError::RowNotFound)?
        .i64("count")?
        .try_into()
        .map_err(DatabaseError::PrecisionLost)
}

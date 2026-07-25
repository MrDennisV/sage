use chia_protocol::Bytes32;

use crate::{
    Convert, Database, DatabaseError, DatabaseTx, Result, SqlAccess, SqlExecutor, sql_file,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetKind {
    Token,
    Nft,
    Did,
    Option,
}

impl Convert<AssetKind> for i64 {
    fn convert(self) -> Result<AssetKind> {
        Ok(match self {
            0 => AssetKind::Token,
            1 => AssetKind::Nft,
            2 => AssetKind::Did,
            3 => AssetKind::Option,
            _ => return Err(DatabaseError::InvalidEnumVariant),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Asset {
    pub hash: Bytes32,
    pub name: Option<String>,
    pub ticker: Option<String>,
    pub precision: u8,
    pub icon_url: Option<String>,
    pub description: Option<String>,
    pub is_sensitive_content: bool,
    pub is_visible: bool,
    pub hidden_puzzle_hash: Option<Bytes32>,
    pub kind: AssetKind,
}

impl<E: SqlExecutor> Database<E> {
    pub async fn is_asset_owned(&self, hash: Bytes32) -> Result<bool> {
        is_asset_owned(&self.executor, hash).await
    }

    pub async fn update_asset(&self, asset: Asset) -> Result<()> {
        update_asset(&self.executor, asset).await
    }

    pub async fn asset_kind(&self, hash: Bytes32) -> Result<Option<AssetKind>> {
        asset_kind(&self.executor, hash).await
    }

    pub async fn asset(&self, hash: Bytes32) -> Result<Option<Asset>> {
        asset(&self.executor, hash).await
    }

    pub async fn insert_asset(&self, asset: Asset) -> Result<()> {
        insert_asset(&self.executor, asset).await
    }

    pub async fn existing_hidden_puzzle_hash(
        &self,
        asset_hash: Bytes32,
    ) -> Result<Option<Option<Bytes32>>> {
        existing_hidden_puzzle_hash(&self.executor, asset_hash).await
    }
}

impl<E: SqlExecutor> DatabaseTx<'_, E> {
    pub async fn asset(&mut self, hash: Bytes32) -> Result<Option<Asset>> {
        asset(&mut self.tx, hash).await
    }

    pub async fn insert_asset(&mut self, asset: Asset) -> Result<()> {
        insert_asset(&mut self.tx, asset).await
    }

    pub async fn update_hidden_puzzle_hash(
        &mut self,
        asset_hash: Bytes32,
        hidden_puzzle_hash: Option<Bytes32>,
    ) -> Result<()> {
        update_hidden_puzzle_hash(&mut self.tx, asset_hash, hidden_puzzle_hash).await
    }

    pub async fn existing_hidden_puzzle_hash(
        &mut self,
        asset_hash: Bytes32,
    ) -> Result<Option<Option<Bytes32>>> {
        existing_hidden_puzzle_hash(&mut self.tx, asset_hash).await
    }

    pub async fn delete_asset_coins(&mut self, asset_hash: Bytes32) -> Result<()> {
        delete_asset_coins(&mut self.tx, asset_hash).await
    }
}

async fn is_asset_owned(mut conn: impl SqlAccess, hash: Bytes32) -> Result<bool> {
    let count = conn
        .fetch_all(sql_file!("assets/is_asset_owned.sql"), vec![hash.into()])
        .await?
        .first()
        .ok_or(DatabaseError::RowNotFound)?
        .i64("count")?;

    Ok(count > 0)
}

async fn update_asset(mut conn: impl SqlAccess, asset: Asset) -> Result<()> {
    conn.execute(
        sql_file!("assets/update_asset.sql"),
        vec![
            (asset.kind as i64).into(),
            asset.name.into(),
            asset.ticker.into(),
            asset.precision.into(),
            asset.icon_url.into(),
            asset.description.into(),
            asset.is_sensitive_content.into(),
            asset.is_visible.into(),
            asset.hash.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn insert_asset(mut conn: impl SqlAccess, asset: Asset) -> Result<()> {
    conn.execute(
        sql_file!("assets/insert_asset.sql"),
        vec![
            asset.hash.into(),
            (asset.kind as i64).into(),
            asset.name.into(),
            asset.ticker.into(),
            asset.precision.into(),
            asset.icon_url.into(),
            asset.description.into(),
            asset.is_sensitive_content.into(),
            asset.is_visible.into(),
            asset.hidden_puzzle_hash.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn update_hidden_puzzle_hash(
    mut conn: impl SqlAccess,
    asset_hash: Bytes32,
    hidden_puzzle_hash: Option<Bytes32>,
) -> Result<()> {
    conn.execute(
        sql_file!("assets/update_hidden_puzzle_hash.sql"),
        vec![hidden_puzzle_hash.into(), asset_hash.into()],
    )
    .await?;

    Ok(())
}

async fn delete_asset_coins(mut conn: impl SqlAccess, asset_hash: Bytes32) -> Result<()> {
    conn.execute(
        sql_file!("assets/delete_asset_coins.sql"),
        vec![asset_hash.into()],
    )
    .await?;

    Ok(())
}

async fn existing_hidden_puzzle_hash(
    mut conn: impl SqlAccess,
    asset_hash: Bytes32,
) -> Result<Option<Option<Bytes32>>> {
    conn.fetch_all(
        sql_file!("assets/existing_hidden_puzzle_hash.sql"),
        vec![asset_hash.into()],
    )
    .await?
    .first()
    .map(|row| row.opt_converted("hidden_puzzle_hash"))
    .transpose()
}

async fn asset_kind(mut conn: impl SqlAccess, hash: Bytes32) -> Result<Option<AssetKind>> {
    conn.fetch_all(sql_file!("assets/asset_kind.sql"), vec![hash.into()])
        .await?
        .first()
        .map(|row| row.i64("kind")?.convert())
        .transpose()
}

async fn asset(mut conn: impl SqlAccess, hash: Bytes32) -> Result<Option<Asset>> {
    conn.fetch_all(sql_file!("assets/asset.sql"), vec![hash.into()])
        .await?
        .first()
        .map(|row| {
            Ok(Asset {
                hash: row.converted("hash")?,
                kind: row.i64("kind")?.convert()?,
                name: row.opt_text("name")?,
                ticker: row.opt_text("ticker")?,
                precision: row.i64("precision")?.convert()?,
                icon_url: row.opt_text("icon_url")?,
                description: row.opt_text("description")?,
                is_sensitive_content: row.i64("is_sensitive_content")? != 0,
                is_visible: row.i64("is_visible")? != 0,
                hidden_puzzle_hash: row.opt_converted("hidden_puzzle_hash")?,
            })
        })
        .transpose()
}

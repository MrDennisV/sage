use crate::{
    Asset, AssetKind, Convert, Database, Result, SqlAccess, SqlExecutor, SqlRow, sql_file,
};

impl<E: SqlExecutor> Database<E> {
    pub async fn all_cats(&self) -> Result<Vec<Asset>> {
        all_cats(&self.executor).await
    }

    pub async fn owned_cats(&self) -> Result<Vec<Asset>> {
        owned_cats(&self.executor).await
    }
}

/// Decodes the columns shared by the `all_cats` and `owned_cats` queries.
fn cat_from_row(row: &SqlRow) -> Result<Asset> {
    Ok(Asset {
        hash: row.converted("hash")?,
        name: row.opt_text("name")?,
        ticker: row.opt_text("ticker")?,
        precision: row.i64("precision")?.convert()?,
        icon_url: row.opt_text("icon_url")?,
        description: row.opt_text("description")?,
        is_visible: row.i64("is_visible")? != 0,
        is_sensitive_content: row.i64("is_sensitive_content")? != 0,
        hidden_puzzle_hash: row.opt_converted("hidden_puzzle_hash")?,
        kind: AssetKind::Token,
    })
}

async fn all_cats(mut conn: impl SqlAccess) -> Result<Vec<Asset>> {
    conn.fetch_all(sql_file!("assets/all_cats.sql"), vec![])
        .await?
        .iter()
        .map(cat_from_row)
        .collect()
}

async fn owned_cats(mut conn: impl SqlAccess) -> Result<Vec<Asset>> {
    conn.fetch_all(sql_file!("assets/owned_cats.sql"), vec![])
        .await?
        .iter()
        .map(cat_from_row)
        .collect()
}

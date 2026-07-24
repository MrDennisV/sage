use sqlx::{
    Column, Row, Sqlite, SqlitePool, Transaction as SqliteTransaction, TypeInfo, ValueRef,
    query::Query,
    sqlite::{SqliteArguments, SqliteRow},
};

use crate::{Result, SqlExecutor, SqlRow, SqlTx, SqlValue};

/// The native [`SqlExecutor`] implementation, backed by a sqlx connection pool.
#[derive(Debug, Clone)]
pub struct SqlxExecutor {
    pub(crate) pool: SqlitePool,
}

impl SqlxExecutor {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn bind_params(sql: &str, params: Vec<SqlValue>) -> Query<'_, Sqlite, SqliteArguments<'_>> {
    let mut query = sqlx::query(sql);

    for param in params {
        query = match param {
            SqlValue::Null => query.bind(None::<Vec<u8>>),
            SqlValue::Int(value) => query.bind(value),
            SqlValue::Real(value) => query.bind(value),
            SqlValue::Text(value) => query.bind(value),
            SqlValue::Blob(value) => query.bind(value),
        };
    }

    query
}

fn decode_row(row: &SqliteRow) -> Result<SqlRow> {
    let columns = row
        .columns()
        .iter()
        .map(|column| column.name().to_string())
        .collect();

    let mut values = Vec::with_capacity(row.len());

    for index in 0..row.len() {
        let raw = row.try_get_raw(index)?;

        let value = if raw.is_null() {
            SqlValue::Null
        } else {
            match raw.type_info().name() {
                "INTEGER" | "BOOLEAN" => SqlValue::Int(row.try_get(index)?),
                "REAL" => SqlValue::Real(row.try_get(index)?),
                "TEXT" => SqlValue::Text(row.try_get(index)?),
                _ => SqlValue::Blob(row.try_get(index)?),
            }
        };

        values.push(value);
    }

    Ok(SqlRow::new(columns, values))
}

impl SqlExecutor for SqlxExecutor {
    type Tx<'a> = SqliteTransaction<'a, Sqlite>;

    async fn fetch_all(&self, sql: &str, params: Vec<SqlValue>) -> Result<Vec<SqlRow>> {
        let rows = bind_params(sql, params).fetch_all(&self.pool).await?;
        rows.iter().map(decode_row).collect()
    }

    async fn execute(&self, sql: &str, params: Vec<SqlValue>) -> Result<u64> {
        Ok(bind_params(sql, params)
            .execute(&self.pool)
            .await?
            .rows_affected())
    }

    async fn execute_batch(&self, sql: &str) -> Result<()> {
        sqlx::raw_sql(sql).execute(&self.pool).await?;
        Ok(())
    }

    async fn begin(&self) -> Result<Self::Tx<'_>> {
        Ok(self.pool.begin().await?)
    }
}

impl SqlTx for SqliteTransaction<'_, Sqlite> {
    async fn fetch_all(&mut self, sql: &str, params: Vec<SqlValue>) -> Result<Vec<SqlRow>> {
        let rows = bind_params(sql, params).fetch_all(&mut **self).await?;
        rows.iter().map(decode_row).collect()
    }

    async fn execute(&mut self, sql: &str, params: Vec<SqlValue>) -> Result<u64> {
        Ok(bind_params(sql, params)
            .execute(&mut **self)
            .await?
            .rows_affected())
    }

    async fn commit(self) -> Result<()> {
        Ok(SqliteTransaction::commit(self).await?)
    }

    async fn rollback(self) -> Result<()> {
        Ok(SqliteTransaction::rollback(self).await?)
    }
}

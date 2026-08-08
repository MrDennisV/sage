use sage_database::{DatabaseError, Result, SqlExecutor, SqlRow, SqlTx, SqlValue};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// The SQL bridge implemented by the service worker over sql.js. Parameters and
// rows cross the boundary as JSON strings (see JsSqlValue). Multi-statement
// SQL must be supported by splitting statements and distributing the bound
// parameters across them in order, matching sqlx's SQLite driver semantics.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = dbQuery, catch)]
    fn db_query(sql: &str, params: &str) -> std::result::Result<String, JsValue>;

    #[wasm_bindgen(js_name = dbExecute, catch)]
    fn db_execute(sql: &str, params: &str) -> std::result::Result<f64, JsValue>;

    #[wasm_bindgen(js_name = dbExecuteBatch, catch)]
    fn db_execute_batch(sql: &str) -> std::result::Result<(), JsValue>;
}

/// The JSON representation of a [`SqlValue`] crossing the JS bridge.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
enum JsSqlValue {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl From<SqlValue> for JsSqlValue {
    fn from(value: SqlValue) -> Self {
        match value {
            SqlValue::Null => Self::Null,
            SqlValue::Int(value) => Self::Int(value),
            SqlValue::Real(value) => Self::Real(value),
            SqlValue::Text(value) => Self::Text(value),
            SqlValue::Blob(value) => Self::Blob(value),
        }
    }
}

impl From<JsSqlValue> for SqlValue {
    fn from(value: JsSqlValue) -> Self {
        match value {
            JsSqlValue::Null => Self::Null,
            JsSqlValue::Int(value) => Self::Int(value),
            JsSqlValue::Real(value) => Self::Real(value),
            JsSqlValue::Text(value) => Self::Text(value),
            JsSqlValue::Blob(value) => Self::Blob(value),
        }
    }
}

#[derive(Debug, Deserialize)]
struct JsRows {
    columns: Vec<String>,
    rows: Vec<Vec<JsSqlValue>>,
}

fn js_error(error: &JsValue) -> DatabaseError {
    DatabaseError::JsError(error.as_string().unwrap_or_else(|| format!("{error:?}")))
}

fn encode_params(params: Vec<SqlValue>) -> Result<String> {
    serde_json::to_string(
        &params
            .into_iter()
            .map(JsSqlValue::from)
            .collect::<Vec<JsSqlValue>>(),
    )
    .map_err(|error| DatabaseError::JsError(error.to_string()))
}

fn fetch_all_sync(sql: &str, params: Vec<SqlValue>) -> Result<Vec<SqlRow>> {
    let rows = db_query(sql, &encode_params(params)?).map_err(|error| js_error(&error))?;

    let rows: JsRows =
        serde_json::from_str(&rows).map_err(|error| DatabaseError::JsError(error.to_string()))?;

    Ok(rows
        .rows
        .into_iter()
        .map(|values| {
            SqlRow::new(
                rows.columns.clone(),
                values.into_iter().map(SqlValue::from).collect(),
            )
        })
        .collect())
}

fn execute_sync(sql: &str, params: Vec<SqlValue>) -> Result<u64> {
    let changes = db_execute(sql, &encode_params(params)?).map_err(|error| js_error(&error))?;
    Ok(changes as u64)
}

/// A [`SqlExecutor`] over the service worker's sql.js database. The JS engine
/// is synchronous and single-threaded, so transactions are plain
/// BEGIN/COMMIT statements on the one shared connection.
#[derive(Debug, Clone, Copy)]
pub struct BrowserExecutor;

impl SqlExecutor for BrowserExecutor {
    type Tx<'a> = BrowserTx;

    async fn fetch_all(&self, sql: &str, params: Vec<SqlValue>) -> Result<Vec<SqlRow>> {
        fetch_all_sync(sql, params)
    }

    async fn execute(&self, sql: &str, params: Vec<SqlValue>) -> Result<u64> {
        execute_sync(sql, params)
    }

    async fn execute_batch(&self, sql: &str) -> Result<()> {
        db_execute_batch(sql).map_err(|error| js_error(&error))
    }

    async fn begin(&self) -> Result<Self::Tx<'_>> {
        execute_sync("BEGIN", Vec::new())?;
        Ok(BrowserTx { finished: false })
    }
}

/// A transaction on the shared sql.js connection. Dropping it without calling
/// commit rolls the transaction back.
#[derive(Debug)]
pub struct BrowserTx {
    finished: bool,
}

impl SqlTx for BrowserTx {
    async fn fetch_all(&mut self, sql: &str, params: Vec<SqlValue>) -> Result<Vec<SqlRow>> {
        fetch_all_sync(sql, params)
    }

    async fn execute(&mut self, sql: &str, params: Vec<SqlValue>) -> Result<u64> {
        execute_sync(sql, params)
    }

    async fn commit(mut self) -> Result<()> {
        self.finished = true;
        execute_sync("COMMIT", Vec::new())?;
        Ok(())
    }

    async fn rollback(mut self) -> Result<()> {
        self.finished = true;
        execute_sync("ROLLBACK", Vec::new())?;
        Ok(())
    }
}

impl Drop for BrowserTx {
    fn drop(&mut self) {
        if !self.finished {
            execute_sync("ROLLBACK", Vec::new()).ok();
        }
    }
}

use crate::{Convert, DatabaseError, Result};

/// Includes a query from the `queries/` directory as a `&'static str`.
///
/// Queries stored as standalone `.sql` files run through [`SqlExecutor`] on
/// every target, while the `query_check` module references the same files with
/// `sqlx::query_file!` so they stay verified against the schema at compile time.
#[macro_export]
macro_rules! sql_file {
    ($path:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/queries/", $path))
    };
}

/// A dynamically typed `SQLite` value, used to bind parameters and decode rows
/// through the [`SqlExecutor`] abstraction.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl From<i64> for SqlValue {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<bool> for SqlValue {
    fn from(value: bool) -> Self {
        Self::Int(i64::from(value))
    }
}

impl From<String> for SqlValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for SqlValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<Vec<u8>> for SqlValue {
    fn from(value: Vec<u8>) -> Self {
        Self::Blob(value)
    }
}

impl From<&[u8]> for SqlValue {
    fn from(value: &[u8]) -> Self {
        Self::Blob(value.to_vec())
    }
}

impl<T> From<Option<T>> for SqlValue
where
    T: Into<SqlValue>,
{
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Into::into)
    }
}

/// A single decoded row returned by a [`SqlExecutor`] query.
#[derive(Debug, Clone)]
pub struct SqlRow {
    columns: Vec<String>,
    values: Vec<SqlValue>,
}

impl SqlRow {
    pub fn new(columns: Vec<String>, values: Vec<SqlValue>) -> Self {
        Self { columns, values }
    }

    fn value(&self, column: &str) -> Result<&SqlValue> {
        let index = self
            .columns
            .iter()
            .position(|name| name == column)
            .ok_or_else(|| DatabaseError::ColumnNotFound(column.to_string()))?;

        Ok(&self.values[index])
    }

    pub fn i64(&self, column: &str) -> Result<i64> {
        match self.value(column)? {
            SqlValue::Int(value) => Ok(*value),
            _ => Err(DatabaseError::UnexpectedColumnType(column.to_string())),
        }
    }

    pub fn opt_i64(&self, column: &str) -> Result<Option<i64>> {
        match self.value(column)? {
            SqlValue::Null => Ok(None),
            SqlValue::Int(value) => Ok(Some(*value)),
            _ => Err(DatabaseError::UnexpectedColumnType(column.to_string())),
        }
    }

    pub fn f64(&self, column: &str) -> Result<f64> {
        match self.value(column)? {
            SqlValue::Real(value) => Ok(*value),
            _ => Err(DatabaseError::UnexpectedColumnType(column.to_string())),
        }
    }

    pub fn text(&self, column: &str) -> Result<String> {
        match self.value(column)? {
            SqlValue::Text(value) => Ok(value.clone()),
            _ => Err(DatabaseError::UnexpectedColumnType(column.to_string())),
        }
    }

    pub fn opt_text(&self, column: &str) -> Result<Option<String>> {
        match self.value(column)? {
            SqlValue::Null => Ok(None),
            SqlValue::Text(value) => Ok(Some(value.clone())),
            _ => Err(DatabaseError::UnexpectedColumnType(column.to_string())),
        }
    }

    pub fn blob(&self, column: &str) -> Result<Vec<u8>> {
        match self.value(column)? {
            SqlValue::Blob(value) => Ok(value.clone()),
            _ => Err(DatabaseError::UnexpectedColumnType(column.to_string())),
        }
    }

    pub fn opt_blob(&self, column: &str) -> Result<Option<Vec<u8>>> {
        match self.value(column)? {
            SqlValue::Null => Ok(None),
            SqlValue::Blob(value) => Ok(Some(value.clone())),
            _ => Err(DatabaseError::UnexpectedColumnType(column.to_string())),
        }
    }

    /// Decodes a BLOB column into a typed value via the [`Convert`] trait,
    /// matching how the sqlx queries decode their columns.
    pub fn converted<T>(&self, column: &str) -> Result<T>
    where
        Vec<u8>: Convert<T>,
    {
        self.blob(column)?.convert()
    }

    pub fn opt_converted<T>(&self, column: &str) -> Result<Option<T>>
    where
        Vec<u8>: Convert<T>,
    {
        self.opt_blob(column)?.map(Convert::convert).transpose()
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        /// Executes SQL against a `SQLite` database. Implementations exist for
        /// sqlx on native targets and for a JS-bridged engine in the browser.
        pub trait SqlExecutor {
            type Tx<'a>: SqlTx
            where
                Self: 'a;

            fn fetch_all(
                &self,
                sql: &str,
                params: Vec<SqlValue>,
            ) -> impl Future<Output = Result<Vec<SqlRow>>>;

            fn execute(
                &self,
                sql: &str,
                params: Vec<SqlValue>,
            ) -> impl Future<Output = Result<u64>>;

            /// Executes multiple semicolon-separated statements, used to apply migrations.
            fn execute_batch(&self, sql: &str) -> impl Future<Output = Result<()>>;

            fn begin(&self) -> impl Future<Output = Result<Self::Tx<'_>>>;
        }

        /// A transaction started by a [`SqlExecutor`].
        pub trait SqlTx {
            fn fetch_all(
                &mut self,
                sql: &str,
                params: Vec<SqlValue>,
            ) -> impl Future<Output = Result<Vec<SqlRow>>>;

            fn execute(
                &mut self,
                sql: &str,
                params: Vec<SqlValue>,
            ) -> impl Future<Output = Result<u64>>;

            fn commit(self) -> impl Future<Output = Result<()>>
            where
                Self: Sized;

            fn rollback(self) -> impl Future<Output = Result<()>>
            where
                Self: Sized;
        }

        /// Unifies executors and transactions so each query is written once and
        /// runs against either. Implemented for `&E` and `&mut Tx`.
        pub trait SqlAccess {
            fn fetch_all(
                &mut self,
                sql: &str,
                params: Vec<SqlValue>,
            ) -> impl Future<Output = Result<Vec<SqlRow>>>;

            fn execute(
                &mut self,
                sql: &str,
                params: Vec<SqlValue>,
            ) -> impl Future<Output = Result<u64>>;
        }
    } else {
        /// Executes SQL against a `SQLite` database. Implementations exist for
        /// sqlx on native targets and for a JS-bridged engine in the browser.
        pub trait SqlExecutor: Send + Sync {
            type Tx<'a>: SqlTx + Send
            where
                Self: 'a;

            fn fetch_all(
                &self,
                sql: &str,
                params: Vec<SqlValue>,
            ) -> impl Future<Output = Result<Vec<SqlRow>>> + Send;

            fn execute(
                &self,
                sql: &str,
                params: Vec<SqlValue>,
            ) -> impl Future<Output = Result<u64>> + Send;

            /// Executes multiple semicolon-separated statements, used to apply migrations.
            fn execute_batch(&self, sql: &str) -> impl Future<Output = Result<()>> + Send;

            fn begin(&self) -> impl Future<Output = Result<Self::Tx<'_>>> + Send;
        }

        /// A transaction started by a [`SqlExecutor`].
        pub trait SqlTx: Send {
            fn fetch_all(
                &mut self,
                sql: &str,
                params: Vec<SqlValue>,
            ) -> impl Future<Output = Result<Vec<SqlRow>>> + Send;

            fn execute(
                &mut self,
                sql: &str,
                params: Vec<SqlValue>,
            ) -> impl Future<Output = Result<u64>> + Send;

            fn commit(self) -> impl Future<Output = Result<()>> + Send
            where
                Self: Sized;

            fn rollback(self) -> impl Future<Output = Result<()>> + Send
            where
                Self: Sized;
        }

        /// Unifies executors and transactions so each query is written once and
        /// runs against either. Implemented for `&E` and `&mut Tx`.
        pub trait SqlAccess: Send {
            fn fetch_all(
                &mut self,
                sql: &str,
                params: Vec<SqlValue>,
            ) -> impl Future<Output = Result<Vec<SqlRow>>> + Send;

            fn execute(
                &mut self,
                sql: &str,
                params: Vec<SqlValue>,
            ) -> impl Future<Output = Result<u64>> + Send;
        }
    }
}

impl<E: SqlExecutor> SqlAccess for &E {
    async fn fetch_all(&mut self, sql: &str, params: Vec<SqlValue>) -> Result<Vec<SqlRow>> {
        SqlExecutor::fetch_all(*self, sql, params).await
    }

    async fn execute(&mut self, sql: &str, params: Vec<SqlValue>) -> Result<u64> {
        SqlExecutor::execute(*self, sql, params).await
    }
}

impl<T: SqlTx> SqlAccess for &mut T {
    async fn fetch_all(&mut self, sql: &str, params: Vec<SqlValue>) -> Result<Vec<SqlRow>> {
        SqlTx::fetch_all(*self, sql, params).await
    }

    async fn execute(&mut self, sql: &str, params: Vec<SqlValue>) -> Result<u64> {
        SqlTx::execute(*self, sql, params).await
    }
}

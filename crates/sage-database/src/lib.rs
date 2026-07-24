mod executor;
#[cfg(feature = "sqlite")]
mod executor_sqlx;
#[cfg(feature = "sqlite")]
mod maintenance;
mod serialized_primitives;
mod migrations;
#[cfg(feature = "sqlite")]
mod query_check;
mod tables;
mod utils;

pub use executor::*;
#[cfg(feature = "sqlite")]
pub use executor_sqlx::*;
#[cfg(feature = "sqlite")]
pub use maintenance::*;
pub use serialized_primitives::*;
pub use migrations::*;
pub use tables::*;

pub(crate) use utils::*;

use std::num::TryFromIntError;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;
use thiserror::Error;
#[cfg(feature = "sqlite")]
use tracing::info;

cfg_if::cfg_if! {
    if #[cfg(feature = "sqlite")] {
        #[derive(Debug, Clone)]
        pub struct Database<E: SqlExecutor = SqlxExecutor> {
            pub(crate) executor: E,
        }

        #[derive(Debug)]
        pub struct DatabaseTx<'a, E: SqlExecutor + 'a = SqlxExecutor> {
            pub(crate) tx: E::Tx<'a>,
        }
    } else {
        #[derive(Debug, Clone)]
        pub struct Database<E: SqlExecutor> {
            pub(crate) executor: E,
        }

        #[derive(Debug)]
        pub struct DatabaseTx<'a, E: SqlExecutor + 'a> {
            pub(crate) tx: E::Tx<'a>,
        }
    }
}

impl<E: SqlExecutor> Database<E> {
    pub fn from_executor(executor: E) -> Self {
        Self { executor }
    }

    pub async fn tx(&self) -> Result<DatabaseTx<'_, E>> {
        let tx = self.executor.begin().await?;
        Ok(DatabaseTx::new(tx))
    }
}

impl<'a, E: SqlExecutor + 'a> DatabaseTx<'a, E> {
    pub fn new(tx: E::Tx<'a>) -> Self {
        Self { tx }
    }

    pub async fn commit(self) -> Result<()> {
        self.tx.commit().await
    }

    pub async fn rollback(self) -> Result<()> {
        self.tx.rollback().await
    }
}

#[cfg(feature = "sqlite")]
impl Database {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            executor: SqlxExecutor::new(pool),
        }
    }

    pub(crate) fn pool(&self) -> &SqlitePool {
        &self.executor.pool
    }

    pub async fn run_rust_migrations(&self, ticker: String) -> Result<()> {
        let mut tx = self.tx().await?;

        let version = tx.rust_migration_version().await?;

        info!("The current Sage migration version is {version}");

        if version < 1 {
            let ticker_upper = ticker.to_uppercase();
            info!("Migrating to version 1 - setting chia token ticker to {ticker_upper}");
            sqlx::query!("UPDATE assets SET ticker = ? WHERE id = 0", ticker_upper)
                .execute(&mut *tx.tx)
                .await?;
            tx.set_rust_migration_version(1).await?;
        }

        tx.commit().await?;

        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl DatabaseTx<'_> {
    pub async fn rust_migration_version(&mut self) -> Result<i64> {
        let row = sqlx::query_scalar!("SELECT version FROM rust_migrations LIMIT 1")
            .fetch_one(&mut *self.tx)
            .await?;

        Ok(row)
    }

    pub async fn set_rust_migration_version(&mut self, version: i64) -> Result<()> {
        sqlx::query!("UPDATE rust_migrations SET version = ?", version)
            .execute(&mut *self.tx)
            .await?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[cfg(feature = "sqlite")]
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Precision lost during cast")]
    PrecisionLost(#[from] TryFromIntError),

    #[error("Invalid length {0}, expected {1}")]
    InvalidLength(usize, usize),

    #[error("BLS error: {0}")]
    Bls(#[from] chia_bls::Error),

    #[error("Invalid enum variant")]
    InvalidEnumVariant,

    #[error("Invalid address")]
    InvalidAddress,

    #[error("Option underlying not found")]
    OptionUnderlyingNotFound,

    #[error("Public key not found for puzzle hash")]
    PublicKeyNotFound,

    #[error("Query returned no rows")]
    RowNotFound,

    #[error("Column {0} not found in row")]
    ColumnNotFound(String),

    #[error("Unexpected type for column {0}")]
    UnexpectedColumnType(String),
}

pub type Result<T> = std::result::Result<T, DatabaseError>;

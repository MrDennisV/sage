use crate::{Result, SqlExecutor, SqlValue};

/// The schema migrations, in order. These are the same files sqlx applies on
/// native targets via `sqlx::migrate!`; browser targets apply them through a
/// [`SqlExecutor`] with [`run_migrations`].
pub const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_setup",
        include_str!("../../../migrations/0001_setup.sql"),
    ),
    (
        "0002_options",
        include_str!("../../../migrations/0002_options.sql"),
    ),
    (
        "0003_unowned_assets",
        include_str!("../../../migrations/0003_unowned_assets.sql"),
    ),
    (
        "0004_arbor",
        include_str!("../../../migrations/0004_arbor.sql"),
    ),
    (
        "0005_clawback_coin_fix",
        include_str!("../../../migrations/0005_clawback_coin_fix.sql"),
    ),
];

/// Drops the whole schema, migration bookkeeping included, so that
/// [`run_migrations`] builds an empty one in its place. This is how a mounted
/// database is emptied where there is no file to delete.
pub async fn drop_schema<E: SqlExecutor>(executor: &E) -> Result<()> {
    // Dropping a table with foreign keys on runs an implicit delete first,
    // which the parent tables would reject halfway through the sweep. They are
    // turned back on afterwards, since every connection runs with them on.
    executor.execute_batch("PRAGMA foreign_keys = OFF").await?;

    // Views go first, because dropping the tables underneath one leaves the
    // view itself in place. Indexes and triggers belong to whatever they were
    // declared on and are dropped along with it.
    for (kind, statement) in [("view", "DROP VIEW"), ("table", "DROP TABLE")] {
        let names = executor
            .fetch_all(
                "SELECT name FROM sqlite_master WHERE type = ? AND name NOT LIKE 'sqlite_%'",
                vec![SqlValue::from(kind)],
            )
            .await?
            .into_iter()
            .map(|row| row.text("name"))
            .collect::<Result<Vec<String>>>()?;

        for name in names {
            // An identifier can't be bound as a parameter. The name comes from
            // the schema itself, and quoting keeps it valid whatever it holds.
            executor
                .execute_batch(&format!("{statement} IF EXISTS \"{name}\""))
                .await?;
        }
    }

    executor.execute_batch("PRAGMA foreign_keys = ON").await?;

    Ok(())
}

/// Applies any pending schema migrations, tracking progress in a
/// `_migrations` table.
pub async fn run_migrations<E: SqlExecutor>(executor: &E) -> Result<()> {
    executor
        .execute_batch("CREATE TABLE IF NOT EXISTS _migrations (name TEXT NOT NULL PRIMARY KEY)")
        .await?;

    let applied = executor
        .fetch_all("SELECT name FROM _migrations", Vec::new())
        .await?
        .into_iter()
        .map(|row| row.text("name"))
        .collect::<Result<Vec<String>>>()?;

    for (name, sql) in MIGRATIONS {
        if applied.iter().any(|applied| applied == name) {
            continue;
        }

        executor.execute_batch(sql).await?;

        executor
            .execute(
                "INSERT INTO _migrations (name) VALUES (?)",
                vec![SqlValue::from(*name)],
            )
            .await?;
    }

    Ok(())
}

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

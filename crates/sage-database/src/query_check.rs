//! Compile-time verification for the queries stored in `queries/*.sql`.
//!
//! Each function references one query file with `sqlx::query_file!`, which
//! checks it against the actual schema (via the offline cache in `.sqlx/` or a
//! live `DATABASE_URL`) exactly like the inline `sqlx::query!` macros. Nothing
//! in this module is ever executed or shipped; if a query drifts from the
//! schema, compilation fails.
//!
//! When converting a query to a `.sql` file, add a matching entry here and
//! regenerate the offline cache with `cargo sqlx prepare`.
#![allow(dead_code)]

fn derivation_index() {
    let _ = sqlx::query_file!("queries/p2_puzzles/derivation_index.sql", true);
}

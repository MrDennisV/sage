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

fn hash() -> Vec<u8> {
    Vec::new()
}

fn derivation_index() {
    let _ = sqlx::query_file!("queries/p2_puzzles/derivation_index.sql", true);
}

fn unused_derivation_index() {
    let _ = sqlx::query_file!("queries/p2_puzzles/unused_derivation_index.sql", true);
}

fn custody_p2_puzzle_hash() {
    let _ = sqlx::query_file!("queries/p2_puzzles/custody_p2_puzzle_hash.sql", 0i64, true);
}

fn is_custody_p2_puzzle_hash() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/p2_puzzles/is_custody_p2_puzzle_hash.sql", hash);
}

fn derivation() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/p2_puzzles/derivation.sql", hash);
}

fn public_key() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/p2_puzzles/public_key.sql", hash);
}

fn p2_puzzle_kind() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/p2_puzzles/p2_puzzle_kind.sql", hash);
}

fn clawback() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/p2_puzzles/clawback.sql", hash);
}

fn underlying_launcher_id() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/p2_puzzles/underlying_launcher_id.sql", hash);
}

fn arbor_key() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/p2_puzzles/arbor_key.sql", hash);
}

fn selectable_xch_coins() {
    let _ = sqlx::query_file!("queries/coins/selectable_xch_coins.sql");
}

fn selectable_cat_coins() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/selectable_cat_coins.sql", hash);
}

fn coin_kind() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/coin_kind.sql", hash);
}

fn underlying_coin_kind() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/underlying_coin_kind.sql", hash);
}

fn xch_coin() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/xch_coin.sql", hash);
}

fn cat_coin() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/cat_coin.sql", hash);
}

fn did_coin() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/did_coin.sql", hash);
}

fn nft_coin() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/nft_coin.sql", hash);
}

fn option_coin() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/option_coin.sql", hash);
}

fn did() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/did.sql", hash);
}

fn nft() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/nft.sql", hash);
}

fn option() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/option.sql", hash);
}

fn asset_kind() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/assets/asset_kind.sql", hash);
}

fn asset() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/assets/asset.sql", hash);
}

fn offer_nft_info() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/nfts/offer_nft_info.sql", hash);
}

fn option_underlying() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/options/option_underlying.sql", hash);
}

fn offer_option_info() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/options/offer_option_info.sql", hash);
}

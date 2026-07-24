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

fn unsynced_coins() {
    let _ = sqlx::query_file!("queries/coins/unsynced_coins.sql", 0i64);
}

fn subscription_coin_ids() {
    let _ = sqlx::query_file!("queries/coins/subscription_coin_ids.sql");
}

fn insert_coin() {
    let hash = hash();
    let parent_coin_hash = hash.clone();
    let puzzle_hash = hash.clone();
    let amount = hash.clone();
    let _ = sqlx::query_file!(
        "queries/coins/insert_coin.sql",
        hash,
        parent_coin_hash,
        puzzle_hash,
        amount,
        0i64,
        0i64
    );
}

fn is_known_coin() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/is_known_coin.sql", hash);
}

fn update_coin() {
    let asset_hash = hash();
    let p2_puzzle_hash = hash();
    let coin_hash = hash();
    let _ = sqlx::query_file!(
        "queries/coins/update_coin.sql",
        asset_hash,
        p2_puzzle_hash,
        coin_hash
    );
}

fn set_children_synced() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/set_children_synced.sql", hash);
}

fn delete_coin() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/coins/delete_coin.sql", hash);
}

fn insert_lineage_proof() {
    let coin_hash = hash();
    let parent_parent_coin_hash = hash();
    let parent_inner_puzzle_hash = hash();
    let parent_amount = hash();
    let _ = sqlx::query_file!(
        "queries/coins/insert_lineage_proof.sql",
        coin_hash,
        parent_parent_coin_hash,
        parent_inner_puzzle_hash,
        parent_amount
    );
}

fn insert_height() {
    let _ = sqlx::query_file!("queries/blocks/insert_height.sql", 0i64);
}

fn insert_block() {
    let header_hash = hash();
    let _ = sqlx::query_file!("queries/blocks/insert_block.sql", 0i64, 0i64, header_hash, true);
}

fn latest_peak() {
    let _ = sqlx::query_file!("queries/blocks/latest_peak.sql");
}

fn insert_mempool_item() {
    let hash = hash();
    let aggregated_signature = hash.clone();
    let fee = hash.clone();
    let _ = sqlx::query_file!(
        "queries/mempool_items/insert_mempool_item.sql",
        hash,
        aggregated_signature,
        fee
    );
}

fn insert_mempool_coin() {
    let mempool_item_hash = hash();
    let coin_hash = hash();
    let _ = sqlx::query_file!(
        "queries/mempool_items/insert_mempool_coin.sql",
        mempool_item_hash,
        coin_hash,
        true,
        false
    );
}

fn insert_mempool_spend() {
    let mempool_item_hash = hash();
    let coin_hash = hash();
    let parent_coin_hash = hash();
    let puzzle_hash = hash();
    let amount = hash();
    let puzzle_reveal = hash();
    let solution = hash();
    let _ = sqlx::query_file!(
        "queries/mempool_items/insert_mempool_spend.sql",
        mempool_item_hash,
        coin_hash,
        parent_coin_hash,
        puzzle_hash,
        amount,
        puzzle_reveal,
        solution,
        0i64
    );
}

fn mempool_items_for_input() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/mempool_items/mempool_items_for_input.sql", hash);
}

fn mempool_items_for_output() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/mempool_items/mempool_items_for_output.sql", hash);
}

fn remove_mempool_coins() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/mempool_items/remove_mempool_coins.sql", hash);
}

fn remove_mempool_item() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/mempool_items/remove_mempool_item.sql", hash);
}

fn custody_p2_puzzle_hashes() {
    let _ = sqlx::query_file!("queries/p2_puzzles/custody_p2_puzzle_hashes.sql");
}

fn is_p2_puzzle_hash() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/p2_puzzles/is_p2_puzzle_hash.sql", hash);
}

fn insert_custody_p2_puzzle() {
    let hash = hash();
    let hash_again = hash.clone();
    let key = hash.clone();
    let _ = sqlx::query_file!(
        "queries/p2_puzzles/insert_custody_p2_puzzle.sql",
        hash,
        hash_again,
        true,
        0i64,
        key
    );
}

fn insert_asset() {
    let hash = hash();
    let hidden_puzzle_hash = hash.clone();
    let _ = sqlx::query_file!(
        "queries/assets/insert_asset.sql",
        hash,
        0i64,
        "",
        "",
        0i64,
        "",
        "",
        true,
        true,
        hidden_puzzle_hash
    );
}

fn update_hidden_puzzle_hash() {
    let hidden_puzzle_hash = hash();
    let asset_hash = hash();
    let _ = sqlx::query_file!(
        "queries/assets/update_hidden_puzzle_hash.sql",
        hidden_puzzle_hash,
        asset_hash
    );
}

fn existing_hidden_puzzle_hash() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/assets/existing_hidden_puzzle_hash.sql", hash);
}

fn delete_asset_coins() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/assets/delete_asset_coins.sql", hash);
}

fn update_xch_ticker() {
    let _ = sqlx::query_file!("queries/assets/update_xch_ticker.sql", "");
}

fn insert_did() {
    let hash = hash();
    let metadata = hash.clone();
    let recovery_list_hash = hash.clone();
    let _ = sqlx::query_file!(
        "queries/dids/insert_did.sql",
        hash,
        metadata,
        recovery_list_hash,
        0i64
    );
}

fn update_did() {
    let metadata = hash();
    let recovery_list_hash = hash();
    let asset_hash = hash();
    let _ = sqlx::query_file!(
        "queries/dids/update_did.sql",
        metadata,
        recovery_list_hash,
        0i64,
        asset_hash
    );
}

fn insert_nft() {
    let asset_hash = hash();
    let collection_hash = hash();
    let minter_hash = hash();
    let owner_hash = hash();
    let metadata = hash();
    let metadata_updater_puzzle_hash = hash();
    let royalty_puzzle_hash = hash();
    let data_hash = hash();
    let metadata_hash = hash();
    let license_hash = hash();
    let _ = sqlx::query_file!(
        "queries/nfts/insert_nft.sql",
        asset_hash,
        collection_hash,
        minter_hash,
        owner_hash,
        metadata,
        metadata_updater_puzzle_hash,
        royalty_puzzle_hash,
        0i64,
        data_hash,
        metadata_hash,
        license_hash,
        0i64,
        0i64
    );
}

fn update_nft() {
    let collection_hash = hash();
    let minter_hash = hash();
    let owner_hash = hash();
    let metadata = hash();
    let metadata_updater_puzzle_hash = hash();
    let royalty_puzzle_hash = hash();
    let data_hash = hash();
    let metadata_hash = hash();
    let license_hash = hash();
    let asset_hash = hash();
    let _ = sqlx::query_file!(
        "queries/nfts/update_nft.sql",
        collection_hash,
        minter_hash,
        owner_hash,
        metadata,
        metadata_updater_puzzle_hash,
        royalty_puzzle_hash,
        0i64,
        data_hash,
        metadata_hash,
        license_hash,
        0i64,
        0i64,
        asset_hash
    );
}

fn insert_option() {
    let asset_hash = hash();
    let underlying_coin_hash = hash();
    let underlying_delegated_puzzle_hash = hash();
    let strike_asset_hash = hash();
    let strike_amount = hash();
    let _ = sqlx::query_file!(
        "queries/options/insert_option.sql",
        asset_hash,
        underlying_coin_hash,
        underlying_delegated_puzzle_hash,
        strike_asset_hash,
        strike_amount
    );
}

fn insert_collection() {
    let hash = hash();
    let minter_hash = hash.clone();
    let _ = sqlx::query_file!(
        "queries/collections/insert_collection.sql",
        hash,
        "",
        minter_hash,
        "",
        "",
        "",
        "",
        true
    );
}

fn insert_file() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/files/insert_file.sql", hash);
}

fn insert_file_uri() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/files/insert_file_uri.sql", hash, "");
}

fn file_data() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/files/file_data.sql", hash);
}

fn resized_image() {
    let hash = hash();
    let _ = sqlx::query_file!("queries/files/resized_image.sql", hash, 0i64);
}

fn rust_migration_version() {
    let _ = sqlx::query_file!("queries/rust_migrations/rust_migration_version.sql");
}

fn set_rust_migration_version() {
    let _ = sqlx::query_file!("queries/rust_migrations/set_rust_migration_version.sql", 0i64);
}

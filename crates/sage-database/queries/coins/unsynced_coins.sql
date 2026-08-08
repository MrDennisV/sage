SELECT
    parent_coin_hash, puzzle_hash, amount, created_height, spent_height,
    (asset_id IS NULL) AS is_asset_unsynced,
    (spent_height IS NOT NULL AND is_children_synced = FALSE) AS is_children_unsynced
FROM coins
WHERE asset_id IS NULL OR (spent_height IS NOT NULL AND is_children_synced = FALSE)
LIMIT ?

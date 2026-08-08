SELECT COUNT(*) AS count FROM coins
WHERE asset_id IS NOT NULL
AND (spent_height IS NULL OR is_children_synced = TRUE)

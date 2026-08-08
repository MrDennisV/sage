INSERT INTO coins
    (hash, parent_coin_hash, puzzle_hash, amount, created_height, spent_height)
VALUES
    (?, ?, ?, ?, ?, ?)
ON CONFLICT(hash) DO UPDATE SET
    created_height = excluded.created_height,
    spent_height = excluded.spent_height

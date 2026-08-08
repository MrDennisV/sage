INSERT OR IGNORE INTO p2_puzzles (hash, kind) VALUES (?, 2);

INSERT OR IGNORE INTO p2_options (p2_puzzle_id, option_asset_id, creator_puzzle_hash, expiration_seconds)
VALUES (
    (SELECT id FROM p2_puzzles WHERE hash = ?),
    (SELECT id FROM assets WHERE hash = ?),
    ?,
    ?
);

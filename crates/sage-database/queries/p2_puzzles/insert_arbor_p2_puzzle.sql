INSERT OR IGNORE INTO p2_puzzles (hash, kind) VALUES (?, 3);

INSERT OR IGNORE INTO p2_arbor (p2_puzzle_id, key)
VALUES ((SELECT id FROM p2_puzzles WHERE hash = ?), ?);

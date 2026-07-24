INSERT OR IGNORE INTO p2_puzzles (hash, kind) VALUES (?, 1);

INSERT OR IGNORE INTO clawbacks (p2_puzzle_id, sender_puzzle_hash, receiver_puzzle_hash, expiration_seconds)
VALUES ((SELECT id FROM p2_puzzles WHERE hash = ?), ?, ?, ?);

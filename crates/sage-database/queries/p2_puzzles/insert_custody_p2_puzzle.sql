INSERT OR IGNORE INTO p2_puzzles (hash, kind) VALUES (?, 0);

INSERT OR IGNORE INTO public_keys (p2_puzzle_id, is_hardened, derivation_index, key)
VALUES ((SELECT id FROM p2_puzzles WHERE hash = ?), ?, ?, ?);

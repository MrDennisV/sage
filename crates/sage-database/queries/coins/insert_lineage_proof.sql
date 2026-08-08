INSERT OR IGNORE INTO lineage_proofs
    (coin_id, parent_parent_coin_hash, parent_inner_puzzle_hash, parent_amount)
VALUES
    ((SELECT id FROM coins WHERE hash = ?), ?, ?, ?)

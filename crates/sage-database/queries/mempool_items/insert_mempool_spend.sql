INSERT OR IGNORE INTO mempool_spends (mempool_item_id, coin_hash, parent_coin_hash, puzzle_hash, amount, puzzle_reveal, solution, seq)
VALUES ((SELECT id FROM mempool_items WHERE hash = ?), ?, ?, ?, ?, ?, ?, ?)

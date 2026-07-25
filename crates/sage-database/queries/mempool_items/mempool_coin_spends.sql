SELECT parent_coin_hash, puzzle_hash, amount, puzzle_reveal, solution
FROM mempool_spends
INNER JOIN mempool_items ON mempool_items.id = mempool_spends.mempool_item_id
WHERE mempool_items.hash = ?
ORDER BY seq ASC

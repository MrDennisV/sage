SELECT mempool_items.hash AS mempool_item_hash
FROM mempool_items
INNER JOIN mempool_coins ON mempool_coins.mempool_item_id = mempool_items.id
INNER JOIN coins ON coins.hash = ?
WHERE mempool_coins.is_output = TRUE

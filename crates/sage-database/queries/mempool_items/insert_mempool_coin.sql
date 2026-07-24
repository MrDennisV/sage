INSERT OR IGNORE INTO mempool_coins (mempool_item_id, coin_id, is_input, is_output)
VALUES ((SELECT id FROM mempool_items WHERE hash = ?), (SELECT id FROM coins WHERE hash = ?), ?, ?)

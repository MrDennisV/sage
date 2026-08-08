UPDATE coins SET is_children_synced = FALSE WHERE id IN (
    SELECT coin_id FROM mempool_coins
    INNER JOIN mempool_items ON mempool_items.id = mempool_coins.mempool_item_id
    WHERE mempool_items.hash = ? AND is_input = TRUE
)

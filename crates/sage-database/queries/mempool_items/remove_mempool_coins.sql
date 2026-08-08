DELETE FROM coins WHERE created_height IS NULL AND id IN (
    SELECT coin_id FROM mempool_coins
    INNER JOIN mempool_items ON mempool_items.id = mempool_coins.mempool_item_id
    WHERE hash = ? AND is_output = TRUE
)

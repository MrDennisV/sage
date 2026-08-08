INSERT OR IGNORE INTO offer_assets (offer_id, asset_id, amount, royalty, is_requested) 
VALUES (
    (SELECT id FROM offers WHERE hash = ?), 
    (SELECT id FROM assets WHERE hash = ?), 
    ?, ?, ?
)

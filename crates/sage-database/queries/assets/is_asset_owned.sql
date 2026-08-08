SELECT COUNT(*) AS count FROM owned_coins 
INNER JOIN assets ON assets.id = owned_coins.asset_id
WHERE assets.hash = ?

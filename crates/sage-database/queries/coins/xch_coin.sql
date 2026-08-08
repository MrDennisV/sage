SELECT parent_coin_hash, puzzle_hash, amount
FROM wallet_coins
WHERE coin_hash = ? AND asset_id = 0

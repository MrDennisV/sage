SELECT coin_hash FROM wallet_coins
WHERE spent_height IS NULL
AND (asset_id != 0 OR p2_puzzle_kind != 0)

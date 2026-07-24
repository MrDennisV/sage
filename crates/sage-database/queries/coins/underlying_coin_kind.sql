SELECT
    underlying_assets.kind, underlying_assets.id
FROM coins
INNER JOIN p2_options ON p2_options.p2_puzzle_id = coins.p2_puzzle_id
INNER JOIN assets AS option_assets ON option_assets.id = p2_options.option_asset_id
INNER JOIN assets AS underlying_assets ON underlying_assets.id = coins.asset_id
WHERE option_assets.hash = ?

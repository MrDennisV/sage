SELECT
    creator_puzzle_hash, expiration_seconds,
    (
        SELECT amount FROM coins
        WHERE coins.p2_puzzle_id = p2_options.p2_puzzle_id LIMIT 1
    ) AS underlying_amount,
    (SELECT hash FROM assets WHERE id = strike_asset_id) AS strike_asset_hash,
    strike_amount, strike_assets.hidden_puzzle_hash AS strike_hidden_puzzle_hash
FROM p2_options
INNER JOIN options ON options.asset_id = p2_options.option_asset_id
INNER JOIN assets AS strike_assets ON strike_assets.id = options.strike_asset_id
WHERE option_asset_id = (SELECT id FROM assets WHERE hash = ?)

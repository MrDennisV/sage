SELECT
    (SELECT hash FROM coins WHERE coins.id = underlying_coin_id) AS underlying_coin_hash,
    underlying_delegated_puzzle_hash
FROM options
INNER JOIN assets ON assets.id = options.asset_id
WHERE hash = ?

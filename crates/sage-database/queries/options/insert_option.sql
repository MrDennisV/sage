INSERT OR IGNORE INTO options (
    asset_id, underlying_coin_id, underlying_delegated_puzzle_hash, strike_asset_id, strike_amount
)
VALUES (
    (SELECT id FROM assets WHERE hash = ?),
    (SELECT id FROM coins WHERE hash = ?),
    ?,
    (SELECT id FROM assets WHERE hash = ?),
    ?
)

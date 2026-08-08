SELECT
    parent_coin_hash, puzzle_hash, amount, p2_puzzle_hash,
    parent_parent_coin_hash, parent_inner_puzzle_hash, parent_amount,
    asset_hash AS launcher_id,
    (SELECT hash FROM coins WHERE id = underlying_coin_id) AS underlying_coin_hash,
    underlying_delegated_puzzle_hash
FROM wallet_coins
INNER JOIN options ON options.asset_id = wallet_coins.asset_id
INNER JOIN lineage_proofs ON lineage_proofs.coin_id = wallet_coins.coin_id
WHERE asset_hash = ? AND spent_height IS NULL

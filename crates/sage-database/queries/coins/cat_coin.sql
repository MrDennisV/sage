SELECT
    parent_coin_hash, puzzle_hash, amount, asset_hidden_puzzle_hash,
    p2_puzzle_hash, parent_parent_coin_hash, parent_inner_puzzle_hash,
    parent_amount, asset_hash AS asset_id
FROM wallet_coins
INNER JOIN lineage_proofs ON lineage_proofs.coin_id = wallet_coins.coin_id
WHERE coin_hash = ?

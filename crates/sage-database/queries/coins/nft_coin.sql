SELECT
    parent_coin_hash, puzzle_hash, amount, p2_puzzle_hash,
    parent_parent_coin_hash, parent_inner_puzzle_hash, parent_amount,
    asset_hash AS launcher_id, metadata, metadata_updater_puzzle_hash,
    owner_hash, royalty_puzzle_hash, royalty_basis_points
FROM wallet_coins
INNER JOIN nfts ON nfts.asset_id = wallet_coins.asset_id
INNER JOIN lineage_proofs ON lineage_proofs.coin_id = wallet_coins.coin_id
WHERE coin_hash = ?

SELECT
    metadata, metadata_updater_puzzle_hash, royalty_puzzle_hash, royalty_basis_points
FROM nfts
INNER JOIN assets ON assets.id = nfts.asset_id
WHERE hash = ?

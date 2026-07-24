INSERT OR IGNORE INTO nfts (
    asset_id, collection_id, minter_hash, owner_hash, metadata, metadata_updater_puzzle_hash,
    royalty_puzzle_hash, royalty_basis_points, data_hash, metadata_hash, license_hash,
    edition_number, edition_total
)
VALUES ((SELECT id FROM assets WHERE hash = ?), (SELECT id FROM collections WHERE hash = ?), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)

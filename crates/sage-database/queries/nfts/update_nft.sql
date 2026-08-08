UPDATE nfts
SET
    collection_id = (SELECT id FROM collections WHERE hash = ?),
    minter_hash = ?,
    owner_hash = ?,
    metadata = ?,
    metadata_updater_puzzle_hash = ?,
    royalty_puzzle_hash = ?,
    royalty_basis_points = ?,
    data_hash = ?,
    metadata_hash = ?,
    license_hash = ?,
    edition_number = ?,
    edition_total = ?
WHERE asset_id = (SELECT id FROM assets WHERE hash = ?)

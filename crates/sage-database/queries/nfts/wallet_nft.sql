SELECT        
    asset_hash, asset_name, asset_ticker, asset_precision, asset_icon_url,
    asset_description, asset_is_sensitive_content, asset_is_visible,
    collections.hash AS 'collection_hash?', collections.name AS collection_name, 
    wallet_nfts.minter_hash, owner_hash, metadata, metadata_updater_puzzle_hash,
    royalty_puzzle_hash, royalty_basis_points, data_hash, metadata_hash, license_hash,
    edition_number, edition_total,
    parent_coin_hash, puzzle_hash, amount, p2_puzzle_hash, created_height, spent_height,
    offer_hash AS 'offer_hash?', created_timestamp, spent_timestamp, clawback_expiration_seconds AS 'clawback_timestamp?',
    asset_hidden_puzzle_hash
FROM wallet_nfts
LEFT JOIN collections ON collections.id = wallet_nfts.collection_id
WHERE wallet_nfts.asset_hash = ?

SELECT hash, minter_hash FROM nfts INNER JOIN assets ON assets.id = asset_id WHERE metadata_hash = ?

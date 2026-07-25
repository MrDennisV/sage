UPDATE nfts SET collection_id = (SELECT id FROM collections WHERE hash = ?)
WHERE asset_id = (SELECT id FROM assets WHERE hash = ?)

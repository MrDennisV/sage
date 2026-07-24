DELETE FROM coins WHERE asset_id = (SELECT id FROM assets WHERE hash = ?)

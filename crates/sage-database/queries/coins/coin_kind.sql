SELECT asset_id, kind FROM coins INNER JOIN assets ON assets.id = asset_id WHERE coins.hash = ?

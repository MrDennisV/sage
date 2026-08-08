SELECT hidden_puzzle_hash FROM assets WHERE hash = ?
AND EXISTS (SELECT 1 FROM coins WHERE coins.asset_id = assets.id)

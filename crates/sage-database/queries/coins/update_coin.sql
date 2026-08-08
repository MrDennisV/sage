UPDATE coins SET
    asset_id = (SELECT id FROM assets WHERE hash = ?),
    p2_puzzle_id = (SELECT id FROM p2_puzzles WHERE hash = ?)
WHERE hash = ?

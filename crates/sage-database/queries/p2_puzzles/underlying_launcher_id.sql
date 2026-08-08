SELECT assets.hash AS launcher_id
FROM p2_puzzles
INNER JOIN p2_options ON p2_options.p2_puzzle_id = p2_puzzles.id
INNER JOIN options ON options.asset_id = p2_options.option_asset_id
INNER JOIN assets ON assets.id = options.asset_id
WHERE p2_puzzles.hash = ?

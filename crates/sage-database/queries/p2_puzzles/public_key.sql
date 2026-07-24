SELECT key
FROM p2_puzzles
INNER JOIN public_keys ON public_keys.p2_puzzle_id = p2_puzzles.id
WHERE p2_puzzles.hash = ?

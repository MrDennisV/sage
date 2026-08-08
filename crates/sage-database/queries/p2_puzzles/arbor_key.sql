SELECT key
FROM p2_puzzles
INNER JOIN p2_arbor ON p2_arbor.p2_puzzle_id = p2_puzzles.id
WHERE p2_puzzles.hash = ?

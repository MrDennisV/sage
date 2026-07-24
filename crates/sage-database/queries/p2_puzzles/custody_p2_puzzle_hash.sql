SELECT hash FROM p2_puzzles
INNER JOIN public_keys ON public_keys.p2_puzzle_id = p2_puzzles.id
WHERE public_keys.derivation_index = ? AND public_keys.is_hardened = ?

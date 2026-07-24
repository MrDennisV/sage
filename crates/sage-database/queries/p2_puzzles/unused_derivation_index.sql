SELECT COALESCE(MAX(derivation_index) + 1, 0) AS derivation_index
FROM public_keys
INNER JOIN coins ON coins.p2_puzzle_id = public_keys.p2_puzzle_id
WHERE is_hardened = ?

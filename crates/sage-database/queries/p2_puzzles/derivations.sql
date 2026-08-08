SELECT
    p2_puzzles.hash AS p2_puzzle_hash,
    public_keys.derivation_index,
    public_keys.is_hardened,
    public_keys.key AS synthetic_key,
    COUNT(*) OVER() AS total
FROM p2_puzzles
INNER JOIN public_keys ON public_keys.p2_puzzle_id = p2_puzzles.id
WHERE public_keys.is_hardened = ?
ORDER BY public_keys.derivation_index ASC
LIMIT ? OFFSET ?

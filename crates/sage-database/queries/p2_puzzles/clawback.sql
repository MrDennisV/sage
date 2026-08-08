SELECT key, sender_puzzle_hash, receiver_puzzle_hash, expiration_seconds
FROM p2_puzzles
INNER JOIN clawbacks ON clawbacks.p2_puzzle_id = p2_puzzles.id
LEFT JOIN public_keys ON public_keys.p2_puzzle_id IN (
    SELECT id FROM p2_puzzles
    WHERE (hash = sender_puzzle_hash AND unixepoch() < expiration_seconds)
    OR (hash = receiver_puzzle_hash AND unixepoch() >= expiration_seconds)
    LIMIT 1
)
WHERE p2_puzzles.hash = ?

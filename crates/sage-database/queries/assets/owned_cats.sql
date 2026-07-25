SELECT
    hash, name, icon_url, description, ticker, precision,
    is_visible, is_sensitive_content, hidden_puzzle_hash
FROM assets
WHERE assets.kind = 0 AND assets.id != 0
AND EXISTS (
    SELECT 1 FROM coins
    INNER JOIN p2_puzzles ON p2_puzzles.id = coins.p2_puzzle_id
    WHERE coins.asset_id = assets.id
)
ORDER BY name ASC

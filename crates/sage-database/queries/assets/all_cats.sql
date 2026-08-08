SELECT
    hash, name, icon_url, description, ticker, precision,
    is_visible, is_sensitive_content, hidden_puzzle_hash
FROM assets
WHERE assets.kind = 0 AND assets.id != 0
ORDER BY name ASC

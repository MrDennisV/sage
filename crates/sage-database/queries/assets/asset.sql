SELECT
    hash, kind, name, ticker, precision, icon_url, description,
    is_sensitive_content, is_visible, hidden_puzzle_hash
FROM assets
WHERE hash = ?

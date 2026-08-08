INSERT INTO assets (
    hash, kind, name, ticker, precision, icon_url, description,
    is_sensitive_content, is_visible, hidden_puzzle_hash
)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
ON CONFLICT(hash) DO UPDATE SET
    name = COALESCE(excluded.name, name),
    ticker = COALESCE(excluded.ticker, ticker),
    icon_url = COALESCE(excluded.icon_url, icon_url),
    description = COALESCE(excluded.description, description),
    is_sensitive_content = is_sensitive_content OR excluded.is_sensitive_content

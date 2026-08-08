INSERT OR IGNORE INTO collections (
    hash, uuid, minter_hash, name, icon_url,
    banner_url, description, is_visible
)
VALUES (?, ?, ?, ?, ?, ?, ?, ?)

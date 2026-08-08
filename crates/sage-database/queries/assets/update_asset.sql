UPDATE assets SET
    kind = ?,
    name = ?,
    ticker = ?,
    precision = ?,
    icon_url = ?,
    description = ?,
    is_sensitive_content = ?,
    is_visible = ?
WHERE hash = ?

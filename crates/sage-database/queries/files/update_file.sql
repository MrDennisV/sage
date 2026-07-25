UPDATE files SET data = ?, mime_type = ?, is_hash_match = ?
WHERE hash = ?
AND (data IS NULL OR NOT is_hash_match)

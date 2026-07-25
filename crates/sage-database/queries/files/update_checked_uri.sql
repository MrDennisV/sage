UPDATE file_uris
SET last_checked_timestamp = unixepoch()
WHERE file_id = (SELECT id FROM files WHERE hash = ?) AND uri = ?

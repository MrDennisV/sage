UPDATE file_uris
SET failed_attempts = failed_attempts + 1, last_checked_timestamp = unixepoch()
WHERE file_id = (SELECT id FROM files WHERE hash = ?) AND uri = ?

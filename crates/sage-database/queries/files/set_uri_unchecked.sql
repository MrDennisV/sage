UPDATE file_uris 
SET failed_attempts = 0, last_checked_timestamp = NULL
FROM files
WHERE file_uris.file_id = files.id AND file_uris.uri = ?

SELECT hash, uri, last_checked_timestamp, failed_attempts
FROM file_uris
INNER JOIN files ON files.id = file_uris.file_id
WHERE data IS NULL
AND (last_checked_timestamp IS NULL OR unixepoch() - last_checked_timestamp >= ?)
AND failed_attempts < ?
LIMIT ?

SELECT COUNT(*) AS count FROM files
WHERE EXISTS (
    SELECT 1 FROM file_uris
    WHERE file_uris.file_id = files.id
)

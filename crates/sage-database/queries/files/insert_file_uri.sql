INSERT OR IGNORE INTO file_uris (file_id, uri) VALUES ((SELECT id FROM files WHERE hash = ?), ?)

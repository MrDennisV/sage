INSERT INTO resized_images (file_id, kind, data) VALUES ((SELECT id FROM files WHERE hash = ?), ?, ?)

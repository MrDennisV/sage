SELECT resized_images.data, mime_type
FROM resized_images
INNER JOIN files ON files.id = resized_images.file_id
WHERE files.hash = ? AND kind = ?

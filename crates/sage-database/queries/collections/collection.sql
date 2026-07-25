SELECT id, hash, uuid, minter_hash, name, icon_url, banner_url, description, is_visible 
FROM collections
WHERE hash = ?

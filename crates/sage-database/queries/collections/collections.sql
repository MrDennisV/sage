SELECT collections.hash, uuid, collections.minter_hash, collections.name, collections.icon_url, 
collections.banner_url, collections.description, collections.is_visible, COUNT(*) OVER() as total_count
FROM collections
WHERE 1=1
AND EXISTS (SELECT 1 FROM owned_nfts WHERE owned_nfts.collection_id = collections.id)
AND (? OR is_visible = 1)
ORDER BY CASE WHEN collections.id = 0 THEN 1 ELSE 0 END, name ASC
LIMIT ?
OFFSET ?

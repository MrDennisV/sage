SELECT DISTINCT minter_hash 
FROM owned_nfts 
WHERE minter_hash IS NOT NULL
ORDER BY minter_hash ASC    
LIMIT ? OFFSET ?

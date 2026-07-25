SELECT COUNT(DISTINCT minter_hash) AS total_count 
FROM owned_nfts 
WHERE minter_hash IS NOT NULL

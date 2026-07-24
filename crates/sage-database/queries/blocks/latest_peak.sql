SELECT height, header_hash
FROM blocks
WHERE header_hash IS NOT NULL AND is_peak = TRUE
ORDER BY height DESC
LIMIT 1

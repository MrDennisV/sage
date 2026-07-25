SELECT hash, aggregated_signature, fee, submitted_timestamp
FROM mempool_items
WHERE submitted_timestamp IS NULL OR unixepoch() - submitted_timestamp >= ?
LIMIT ?

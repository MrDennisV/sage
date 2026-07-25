SELECT hash, aggregated_signature, fee, submitted_timestamp
FROM mempool_items
ORDER BY submitted_timestamp DESC, hash ASC

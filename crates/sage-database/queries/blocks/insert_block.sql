INSERT INTO blocks (height, timestamp, header_hash, is_peak) VALUES (?, ?, ?, ?)
ON CONFLICT (height) DO UPDATE SET
    timestamp = COALESCE(excluded.timestamp, timestamp),
    header_hash = excluded.header_hash,
    is_peak = (excluded.is_peak OR is_peak)

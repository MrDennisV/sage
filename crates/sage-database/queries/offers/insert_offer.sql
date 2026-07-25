INSERT OR IGNORE INTO offers (
    hash, encoded_offer, fee, status,
    expiration_height, expiration_timestamp, inserted_timestamp
)
VALUES (?, ?, ?, ?, ?, ?, ?)

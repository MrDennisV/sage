SELECT
    hash as offer_id,
    encoded_offer,
    fee,
    status,
    expiration_height,
    expiration_timestamp,
    inserted_timestamp
FROM offers 
WHERE status = ? OR ? IS NULL
ORDER BY inserted_timestamp DESC

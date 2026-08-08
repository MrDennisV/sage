SELECT
    offers.hash as offer_id,
    encoded_offer,
    fee,
    status,
    expiration_height,
    expiration_timestamp,
    inserted_timestamp
FROM offers
INNER JOIN offer_assets ON offers.id = offer_assets.offer_id
INNER JOIN assets ON offer_assets.asset_id = assets.id
WHERE assets.hash = ? AND offers.status = ? OR ? IS NULL
ORDER BY inserted_timestamp DESC

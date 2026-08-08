SELECT
    offers.hash as offer_id, assets.hash as asset_id,
    amount, royalty, is_requested, 
    assets.description, assets.is_sensitive_content,
    assets.is_visible, assets.icon_url, assets.name,
    assets.ticker, assets.precision, assets.kind,
    assets.hidden_puzzle_hash
FROM offer_assets 
INNER JOIN assets ON offer_assets.asset_id = assets.id
INNER JOIN offers ON offer_assets.offer_id = offers.id
WHERE offers.hash = ?

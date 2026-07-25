UPDATE assets SET icon_url = ?
WHERE assets.id IN (
    SELECT asset_id FROM nfts
    WHERE data_hash = ?
)

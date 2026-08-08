SELECT
    asset_hash, asset_name, asset_ticker, asset_precision, asset_icon_url,
    asset_description, asset_is_visible, asset_is_sensitive_content,
    asset_hidden_puzzle_hash, owned_coins.created_height, spent_height,
    parent_coin_hash, puzzle_hash, amount, p2_puzzle_hash,
    metadata, recovery_list_hash, num_verifications_required,
    offer_hash, created_timestamp, spent_timestamp,
    clawback_expiration_seconds AS clawback_timestamp
FROM owned_coins
INNER JOIN dids ON dids.asset_id = owned_coins.asset_id
ORDER BY asset_name ASC

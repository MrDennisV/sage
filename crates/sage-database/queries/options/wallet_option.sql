SELECT
    asset_hash, asset_name, asset_ticker, asset_precision, asset_icon_url,
    asset_description, asset_is_visible, asset_is_sensitive_content,
    asset_hidden_puzzle_hash, wallet_coins.created_height, wallet_coins.spent_height,
    wallet_coins.parent_coin_hash, wallet_coins.puzzle_hash, wallet_coins.amount, wallet_coins.p2_puzzle_hash,
    offer_hash AS 'offer_hash?', created_timestamp, spent_timestamp,
    clawback_expiration_seconds AS 'clawback_timestamp?',
    p2_options.expiration_seconds AS option_expiration_seconds,

    strike_asset.hash AS strike_asset_hash, strike_asset.name AS strike_asset_name,
    strike_asset.ticker AS strike_asset_ticker, strike_asset.precision AS strike_asset_precision,
    strike_asset.icon_url AS strike_asset_icon_url, strike_asset.description AS strike_asset_description,
    strike_asset.is_visible AS strike_asset_is_visible, strike_asset.is_sensitive_content AS strike_asset_is_sensitive_content,
    strike_asset.hidden_puzzle_hash AS strike_asset_hidden_puzzle_hash, strike_asset.kind AS strike_asset_kind,

    underlying_asset.hash AS underlying_asset_hash, underlying_asset.name AS underlying_asset_name,
    underlying_asset.ticker AS underlying_asset_ticker, underlying_asset.precision AS underlying_asset_precision,
    underlying_asset.icon_url AS underlying_asset_icon_url, underlying_asset.description AS underlying_asset_description,
    underlying_asset.is_visible AS underlying_asset_is_visible, underlying_asset.is_sensitive_content AS underlying_asset_is_sensitive_content,
    underlying_asset.hidden_puzzle_hash AS underlying_asset_hidden_puzzle_hash, underlying_asset.kind AS underlying_asset_kind,
    
    strike_amount, 
    underlying_coin.amount AS underlying_amount,
    underlying_coin.hash AS underlying_coin_id
FROM wallet_coins
INNER JOIN options ON options.asset_id = wallet_coins.asset_id
INNER JOIN p2_options ON p2_options.option_asset_id = options.asset_id
INNER JOIN coins AS underlying_coin ON underlying_coin.id = options.underlying_coin_id
INNER JOIN assets AS strike_asset ON strike_asset.id = options.strike_asset_id
INNER JOIN assets AS underlying_asset ON underlying_asset.id = underlying_coin.asset_id
WHERE asset_hash = ?

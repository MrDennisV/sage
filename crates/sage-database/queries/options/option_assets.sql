SELECT
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

    p2_options.expiration_seconds AS expiration_seconds,
    strike_amount, 
    underlying_coin.amount AS underlying_amount
FROM options
INNER JOIN assets AS option_asset ON option_asset.id = options.asset_id
INNER JOIN coins AS underlying_coin ON underlying_coin.id = options.underlying_coin_id
INNER JOIN p2_options ON p2_options.option_asset_id = options.asset_id
INNER JOIN assets AS strike_asset ON strike_asset.id = options.strike_asset_id
INNER JOIN assets AS underlying_asset ON underlying_asset.id = underlying_coin.asset_id
WHERE option_asset.hash = ?

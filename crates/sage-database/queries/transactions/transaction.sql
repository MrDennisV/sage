SELECT 	
    height, timestamp, coin_id, puzzle_hash, parent_coin_hash, amount,
    is_created_in_block, is_spent_in_block, asset_hash, asset_description,
    asset_is_visible, asset_is_sensitive_content, asset_name, asset_icon_url,
    asset_kind, p2_puzzle_hash, asset_ticker, asset_precision, asset_hidden_puzzle_hash
FROM transaction_coins 
WHERE height = ?

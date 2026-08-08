INSERT OR IGNORE INTO offer_coins (offer_id, coin_id) 
VALUES ((SELECT id FROM offers WHERE hash = ?), (SELECT id FROM coins WHERE hash = ?))

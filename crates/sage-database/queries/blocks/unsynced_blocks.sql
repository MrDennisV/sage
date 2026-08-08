SELECT created_height AS height FROM coins
INNER JOIN blocks ON blocks.height = coins.created_height
WHERE blocks.timestamp IS NULL
UNION
SELECT spent_height AS height FROM coins
INNER JOIN blocks ON blocks.height = coins.spent_height
WHERE blocks.timestamp IS NULL
ORDER BY height DESC
LIMIT ?

SELECT COALESCE(MAX(derivation_index) + 1, 0) AS derivation_index
FROM public_keys
WHERE is_hardened = ?

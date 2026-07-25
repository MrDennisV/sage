SELECT MAX(derivation_index) AS derivation_index
FROM public_keys
WHERE is_hardened = ?

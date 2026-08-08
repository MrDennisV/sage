UPDATE dids
SET
    metadata = ?,
    recovery_list_hash = ?,
    num_verifications_required = ?
WHERE asset_id = (SELECT id FROM assets WHERE hash = ?)

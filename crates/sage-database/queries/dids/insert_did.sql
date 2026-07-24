INSERT OR IGNORE INTO dids (
    asset_id, metadata, recovery_list_hash, num_verifications_required
)
VALUES ((SELECT id FROM assets WHERE hash = ?), ?, ?, ?)

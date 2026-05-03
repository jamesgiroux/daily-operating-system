CREATE TABLE IF NOT EXISTS source_asof_backfill_quarantine (
    id TEXT PRIMARY KEY,
    claim_source TEXT NOT NULL,
    legacy_entity_id TEXT NOT NULL,
    legacy_field_path TEXT NOT NULL,
    legacy_item_hash TEXT,
    raw_sourced_at TEXT,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    remediation_status TEXT NOT NULL DEFAULT 'pending'
                                  CHECK (remediation_status IN ('pending', 'resolved', 'discarded'))
);

CREATE INDEX IF NOT EXISTS idx_source_asof_quarantine_status
    ON source_asof_backfill_quarantine(remediation_status, reason);

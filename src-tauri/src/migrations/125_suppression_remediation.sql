-- DOS-308: is_suppressed lookup support + tombstone remediation quarantine.

-- Covering index for the precedence query used by ActionDb::is_suppressed.
CREATE INDEX IF NOT EXISTS idx_tombstones_lookup
  ON suppression_tombstones(entity_id, field_key, dismissed_at DESC);

-- Quarantine table for malformed or superseded suppression tombstones.
CREATE TABLE IF NOT EXISTS suppression_tombstones_quarantine (
    id INTEGER PRIMARY KEY,
    entity_id TEXT NOT NULL,
    field_key TEXT NOT NULL,
    item_key TEXT,
    item_hash TEXT,
    dismissed_at TEXT,
    source_scope TEXT,
    expires_at TEXT,
    superseded_by_evidence_after TEXT,
    quarantined_at TEXT NOT NULL DEFAULT (datetime('now')),
    quarantine_reason TEXT NOT NULL
);

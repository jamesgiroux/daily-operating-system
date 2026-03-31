-- I645: Feedback events + suppression tombstones
CREATE TABLE IF NOT EXISTS entity_feedback_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL DEFAULT 'account',
    field_key TEXT NOT NULL,
    item_key TEXT,
    feedback_type TEXT NOT NULL,
    source_system TEXT,
    source_kind TEXT,
    previous_value TEXT,
    corrected_value TEXT,
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_feedback_entity ON entity_feedback_events(entity_id, field_key);

CREATE TABLE IF NOT EXISTS suppression_tombstones (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id TEXT NOT NULL,
    field_key TEXT NOT NULL,
    item_key TEXT,
    item_hash TEXT,
    dismissed_at TEXT NOT NULL DEFAULT (datetime('now')),
    source_scope TEXT,
    expires_at TEXT,
    superseded_by_evidence_after TEXT
);
CREATE INDEX IF NOT EXISTS idx_tombstones_entity ON suppression_tombstones(entity_id, field_key);

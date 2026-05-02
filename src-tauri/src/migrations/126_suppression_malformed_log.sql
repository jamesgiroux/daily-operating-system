CREATE TABLE IF NOT EXISTS suppression_malformed_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    record_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    field_key TEXT NOT NULL,
    encountered_at TEXT NOT NULL DEFAULT (datetime('now')),
    caller_context TEXT
);

CREATE INDEX IF NOT EXISTS idx_suppression_malformed_log_lookup
    ON suppression_malformed_log(entity_id, field_key, encountered_at DESC);

CREATE TABLE IF NOT EXISTS pipeline_failures (
    id TEXT PRIMARY KEY,
    pipeline TEXT NOT NULL,
    entity_id TEXT,
    entity_type TEXT,
    error_type TEXT NOT NULL,
    error_message TEXT,
    attempt INTEGER NOT NULL DEFAULT 1,
    resolved INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_pipeline_failures_unresolved
ON pipeline_failures(pipeline, resolved, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_pipeline_failures_entity
ON pipeline_failures(entity_type, entity_id, resolved);

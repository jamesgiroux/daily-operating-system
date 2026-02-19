-- I353 Phase 2: Signal → Hygiene feedback loop action log

CREATE TABLE IF NOT EXISTS hygiene_actions_log (
    id TEXT PRIMARY KEY,
    source_signal_id TEXT,
    action_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL DEFAULT '',
    confidence REAL NOT NULL DEFAULT 0.0,
    result TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_hygiene_actions_log_entity
    ON hygiene_actions_log(entity_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_hygiene_actions_log_source
    ON hygiene_actions_log(source_signal_id);

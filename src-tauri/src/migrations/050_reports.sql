CREATE TABLE IF NOT EXISTS reports (
    id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL DEFAULT 'account',
    report_type TEXT NOT NULL,
    content_json TEXT NOT NULL,
    generated_at DATETIME NOT NULL,
    intel_hash TEXT NOT NULL,
    is_stale INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_reports_entity_type ON reports(entity_id, entity_type, report_type);
CREATE INDEX IF NOT EXISTS idx_reports_stale ON reports(is_stale);

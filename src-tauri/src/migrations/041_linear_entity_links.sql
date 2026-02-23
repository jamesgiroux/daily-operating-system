-- Migration 041: Linear entity links for signal routing
CREATE TABLE IF NOT EXISTS linear_entity_links (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    linear_project_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('account', 'project', 'person')),
    confirmed INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(linear_project_id, entity_id, entity_type)
);

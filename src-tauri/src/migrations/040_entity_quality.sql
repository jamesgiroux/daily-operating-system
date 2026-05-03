-- Migration 040: Entity quality scoring for self-healing intelligence

CREATE TABLE IF NOT EXISTS entity_quality (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    quality_alpha REAL NOT NULL DEFAULT 1.0,
    quality_beta REAL NOT NULL DEFAULT 1.0,
    quality_score REAL NOT NULL DEFAULT 0.5,
    last_enrichment_at TEXT,
    correction_count INTEGER NOT NULL DEFAULT 0,
    coherence_retry_count INTEGER NOT NULL DEFAULT 0,
    coherence_window_start TEXT,
    coherence_blocked INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_entity_quality_score ON entity_quality(quality_score);
CREATE INDEX IF NOT EXISTS idx_entity_quality_blocked ON entity_quality(coherence_blocked);

ALTER TABLE entity_intelligence ADD COLUMN coherence_score REAL;
ALTER TABLE entity_intelligence ADD COLUMN coherence_flagged INTEGER DEFAULT 0;

-- Migration 021: Proactive surfacing tables (I260 — ADR-0080 Phase 5)
--
-- Tracks proactive scan state per detector and deduplicates insights
-- so the same pattern isn't surfaced repeatedly.

CREATE TABLE IF NOT EXISTS proactive_scan_state (
    detector_name TEXT PRIMARY KEY,
    last_run_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_insight_count INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS proactive_insights (
    id TEXT PRIMARY KEY,
    detector_name TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    signal_id TEXT,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    headline TEXT NOT NULL,
    detail TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_proactive_insights_fingerprint
    ON proactive_insights(fingerprint);

CREATE INDEX IF NOT EXISTS idx_proactive_insights_detector
    ON proactive_insights(detector_name, created_at DESC);

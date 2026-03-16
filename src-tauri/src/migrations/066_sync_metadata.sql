-- I428: Sync metadata for connectivity/freshness tracking
CREATE TABLE IF NOT EXISTS sync_metadata (
    source TEXT PRIMARY KEY,
    last_success_at TEXT,
    last_attempt_at TEXT,
    last_error TEXT,
    consecutive_failures INTEGER NOT NULL DEFAULT 0
);

-- Seed with sources that have discrete sync paths.
-- Glean is inline during enrichment (no separate poller) — not tracked here.
INSERT OR IGNORE INTO sync_metadata (source) VALUES ('google_calendar');
INSERT OR IGNORE INTO sync_metadata (source) VALUES ('gmail');
INSERT OR IGNORE INTO sync_metadata (source) VALUES ('claude_code');

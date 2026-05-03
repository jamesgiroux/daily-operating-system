-- Email retry pipeline + sync meta.
--
-- Adds a single-row metadata table that tracks the last *successful* Gmail
-- fetch completion (as opposed to `last_seen_at` on each email row, which only
-- records the last time any individual email was upserted). Keeping these
-- separate lets the UI distinguish "inbox fetch itself is healthy" from
-- "nothing has been enriched in a while because enrichment failed".
--
-- The table uses a fixed `id = 1` singleton row pattern so `UPDATE` is always
-- safe without worrying about multiple rows; `INSERT OR IGNORE` seeds it once.

CREATE TABLE IF NOT EXISTS email_sync_meta (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    last_successful_fetch_at TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO email_sync_meta (id, last_successful_fetch_at, updated_at)
VALUES (1, NULL, datetime('now'));

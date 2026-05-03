-- Persist source attribution for email_signals feedback routing.
-- Use rebuild pattern for SQLite idempotency and to tolerate partial legacy schemas.
PRAGMA foreign_keys = OFF;
BEGIN IMMEDIATE;

-- Backstop for partially-corrupt legacy states: create base table if missing.
CREATE TABLE IF NOT EXISTS email_signals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email_id TEXT NOT NULL,
    sender_email TEXT,
    person_id TEXT,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    signal_type TEXT NOT NULL,
    signal_text TEXT NOT NULL,
    confidence REAL,
    sentiment TEXT,
    urgency TEXT,
    detected_at TEXT NOT NULL DEFAULT (datetime('now')),
    deactivated_at TEXT
);

DROP TABLE IF EXISTS email_signals_new;
CREATE TABLE email_signals_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email_id TEXT NOT NULL,
    sender_email TEXT,
    person_id TEXT,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    signal_type TEXT NOT NULL,
    signal_text TEXT NOT NULL,
    confidence REAL,
    sentiment TEXT,
    urgency TEXT,
    detected_at TEXT NOT NULL DEFAULT (datetime('now')),
    deactivated_at TEXT,
    source TEXT NOT NULL DEFAULT 'email_enrichment'
);

INSERT OR IGNORE INTO email_signals_new (
    id,
    email_id,
    sender_email,
    person_id,
    entity_id,
    entity_type,
    signal_type,
    signal_text,
    confidence,
    sentiment,
    urgency,
    detected_at,
    deactivated_at,
    source
)
SELECT
    id,
    email_id,
    sender_email,
    person_id,
    entity_id,
    entity_type,
    signal_type,
    signal_text,
    confidence,
    sentiment,
    urgency,
    detected_at,
    deactivated_at,
    'email_enrichment'
FROM email_signals;

DROP TABLE IF EXISTS email_signals;
ALTER TABLE email_signals_new RENAME TO email_signals;
CREATE INDEX IF NOT EXISTS idx_email_signals_entity_detected
    ON email_signals(entity_id, detected_at DESC);
CREATE INDEX IF NOT EXISTS idx_email_signals_email_id
    ON email_signals(email_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_email_signals_dedupe
    ON email_signals(email_id, entity_id, signal_type);

PRAGMA foreign_keys = ON;
COMMIT;

-- Sprint 22: email intelligence signals linked to entities

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
    detected_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_email_signals_entity_detected
    ON email_signals(entity_id, detected_at DESC);

CREATE INDEX IF NOT EXISTS idx_email_signals_email_id
    ON email_signals(email_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_email_signals_dedupe
    ON email_signals(email_id, entity_id, signal_type);

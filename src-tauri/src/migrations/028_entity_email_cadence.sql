-- I319: Entity-level email cadence monitoring

CREATE TABLE IF NOT EXISTS entity_email_cadence (
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    period TEXT NOT NULL,
    message_count INTEGER NOT NULL DEFAULT 0,
    rolling_avg REAL NOT NULL DEFAULT 0.0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (entity_id, entity_type, period)
);

CREATE INDEX IF NOT EXISTS idx_entity_email_cadence_updated
    ON entity_email_cadence(entity_id, entity_type, updated_at DESC);

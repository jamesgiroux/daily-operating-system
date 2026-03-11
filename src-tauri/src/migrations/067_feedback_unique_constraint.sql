-- I529: Add UNIQUE constraint on (entity_id, entity_type, field) for intelligence_feedback.
-- Ensures one vote per field per entity — changing vote replaces previous (AC16).
-- Approach: recreate table with constraint since SQLite doesn't support ADD CONSTRAINT.

CREATE TABLE IF NOT EXISTS intelligence_feedback_new (
    id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    field TEXT NOT NULL,
    feedback_type TEXT NOT NULL CHECK(feedback_type IN ('positive', 'negative', 'replaced')),
    previous_value TEXT,
    context TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(entity_id, entity_type, field),
    FOREIGN KEY (entity_id) REFERENCES entities(id)
);

-- Migrate existing data (keep most recent per entity+field combo)
INSERT OR IGNORE INTO intelligence_feedback_new
    (id, entity_id, entity_type, field, feedback_type, previous_value, context, created_at)
SELECT id, entity_id, entity_type, field, feedback_type, previous_value, context, created_at
FROM intelligence_feedback
ORDER BY created_at DESC;

DROP TABLE intelligence_feedback;
ALTER TABLE intelligence_feedback_new RENAME TO intelligence_feedback;

CREATE INDEX IF NOT EXISTS idx_intelligence_feedback_entity ON intelligence_feedback(entity_id);

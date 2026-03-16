-- I529/I536: Intelligence feedback table for tracking user reactions to AI-generated content.
--
-- Records positive/negative/replaced feedback on individual intelligence fields,
-- enabling the system to learn which assessments resonate and which don't.

CREATE TABLE IF NOT EXISTS intelligence_feedback (
    id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    field TEXT NOT NULL,
    feedback_type TEXT NOT NULL CHECK(feedback_type IN ('positive', 'negative', 'replaced')),
    previous_value TEXT,
    context TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (entity_id) REFERENCES entities(id)
);

CREATE INDEX idx_intelligence_feedback_entity ON intelligence_feedback(entity_id);

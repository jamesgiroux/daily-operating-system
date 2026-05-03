CREATE TABLE claim_feedback_new (
    id              TEXT PRIMARY KEY,
    claim_id        TEXT NOT NULL REFERENCES intelligence_claims(id),
    feedback_type   TEXT NOT NULL
                              CHECK (feedback_type IN (
                                  'confirm_current',
                                  'mark_outdated',
                                  'mark_false',
                                  'wrong_subject',
                                  'wrong_source',
                                  'cannot_verify',
                                  'needs_nuance',
                                  'surface_inappropriate',
                                  'not_relevant_here'
                              )),
    actor           TEXT NOT NULL,
    actor_id        TEXT,
    payload_json    TEXT,
    submitted_at    TEXT NOT NULL DEFAULT (datetime('now')),
    applied_at      TEXT NULL
);

INSERT INTO claim_feedback_new (
    id,
    claim_id,
    feedback_type,
    actor,
    actor_id,
    payload_json,
    submitted_at,
    applied_at
)
SELECT
    id,
    claim_id,
    CASE feedback_type
        WHEN 'confirm' THEN 'confirm_current'
        WHEN 'correct' THEN 'needs_nuance'
        WHEN 'reject' THEN 'mark_false'
        ELSE feedback_type
    END,
    actor,
    actor_id,
    payload_json,
    submitted_at,
    NULL
FROM claim_feedback;

DROP TABLE claim_feedback;

ALTER TABLE claim_feedback_new RENAME TO claim_feedback;

CREATE INDEX IF NOT EXISTS idx_feedback_claim
    ON claim_feedback(claim_id);

CREATE INDEX IF NOT EXISTS idx_feedback_type
    ON claim_feedback(feedback_type, submitted_at);

ALTER TABLE intelligence_claims ADD COLUMN verification_state TEXT NOT NULL DEFAULT 'active'
    CHECK (verification_state IN ('active', 'contested', 'needs_user_decision'));

ALTER TABLE intelligence_claims ADD COLUMN verification_reason TEXT;

ALTER TABLE intelligence_claims ADD COLUMN needs_user_decision_at TEXT;

-- ADR-0123 V1.1 amendment (v1.4.4 W2 §5.3 / DOS-484):
-- extend the `claim_feedback.feedback_type` CHECK constraint to include
-- the 10th typed feedback variant, `merge_intent`. MergeIntent persists a
-- typed proposal row (merge_target + optional supporting_evidence) that
-- the downstream Person Detail merge picker consumes. It does NOT mutate
-- claim verification or lifecycle state on the source claim; that flow
-- is handled by the merge execution service.
--
-- SQLite CHECK constraints can only be modified by recreating the table.
-- We rebuild claim_feedback with the widened CHECK, preserve every row
-- (existing 9-variant rows remain valid; the constraint is a superset),
-- and re-create the existing indexes (`idx_feedback_claim`,
-- `idx_feedback_type`) so reads stay covered.
--
-- Safe across multi-process readers: SQLite serializes table-rebuilds at
-- the writer lock. No data shape changes; this is purely a CHECK
-- widening.

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
                                  'not_relevant_here',
                                  'merge_intent'
                              )),
    actor           TEXT NOT NULL,
    actor_id        TEXT,
    payload_json    TEXT,
    submitted_at    TEXT NOT NULL DEFAULT (datetime('now')),
    applied_at      TEXT NULL
);

INSERT INTO claim_feedback_new (
    id, claim_id, feedback_type, actor, actor_id, payload_json,
    submitted_at, applied_at
)
SELECT
    id, claim_id, feedback_type, actor, actor_id, payload_json,
    submitted_at, applied_at
FROM claim_feedback;

DROP TABLE claim_feedback;

ALTER TABLE claim_feedback_new RENAME TO claim_feedback;

CREATE INDEX IF NOT EXISTS idx_feedback_claim
    ON claim_feedback(claim_id);

CREATE INDEX IF NOT EXISTS idx_feedback_type
    ON claim_feedback(feedback_type, submitted_at);

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
-- Atomicity (W1W2 L2 cycle-2 CRITICAL fix; same class as v244 / L3 cycle-2
-- F3): the migration runner at `migrations.rs` calls
-- `conn.execute_batch(sql)` which does NOT wrap the batch in a single
-- transaction unless the SQL contains explicit `BEGIN; ... COMMIT;`.
-- Without that, multi-process readers (additional processes opening the
-- encrypted DB during the migration window) could observe the moment
-- between `DROP TABLE claim_feedback` and the `ALTER TABLE ... RENAME`,
-- failing reads against the missing `claim_feedback` table.
--
-- Strategy: wrap the rebuild in `BEGIN IMMEDIATE; ... COMMIT;` so the
-- write lock is held for the entire CREATE/INSERT/DROP/RENAME/INDEX
-- sequence. SQLite guarantees other connections cannot observe schema
-- state between those statements while the IMMEDIATE transaction holds
-- the write lock. The class-wide CI gate
-- `src-tauri/scripts/check_migrations_transactional.sh` enforces this
-- pattern for every destructive migration going forward.

BEGIN IMMEDIATE;

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

COMMIT;

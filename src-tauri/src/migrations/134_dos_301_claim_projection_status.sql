-- Per-claim projection-status ledger.
--
-- intelligence_claims is the durable source of truth for claim-shaped
-- state. Legacy consumers (entity_assessment + entity_quality tables,
-- success_plans, account AI columns, intelligence.json on disk) keep
-- working through derived projections. When commit_claim runs, each
-- target gets one row here recording whether the projection succeeded,
-- failed, or was later repaired.
--
-- One row per (claim_id, projection_target). Failed rows are the
-- repair worklist; the per-rule SAVEPOINT pattern in the projection
-- rules ensures one rule's failure does not abort the others.

CREATE TABLE IF NOT EXISTS claim_projection_status (
    claim_id            TEXT NOT NULL REFERENCES intelligence_claims(id),
    projection_target   TEXT NOT NULL
                                CHECK (projection_target IN (
                                    'entity_intelligence',
                                    'success_plans',
                                    'accounts_columns',
                                    'intelligence_json'
                                )),
    status              TEXT NOT NULL
                                CHECK (status IN ('committed', 'failed', 'repaired')),
    error_message       TEXT,
    attempted_at        TEXT NOT NULL,
    succeeded_at        TEXT,
    PRIMARY KEY (claim_id, projection_target)
);

-- Repair worklist index: scan failed projections by target and status.
CREATE INDEX IF NOT EXISTS idx_claim_projection_status_failed
    ON claim_projection_status(projection_target, status)
    WHERE status = 'failed';

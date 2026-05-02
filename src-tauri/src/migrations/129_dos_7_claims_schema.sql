-- DOS-7 D1: Claims commit substrate. The intelligence_claims table is the
-- queryable commit log + tombstone gate + trust/feedback target + per-claim
-- identity layer for v1.4.0. 5 sibling tables hold corroboration evidence,
-- contradiction reconciliation, agent trust ledger, typed feedback, and the
-- repair-job skeleton.
--
-- This migration creates schema only. commit_claim algorithm + 9-mechanism
-- backfill + caller refactors land in DOS-7 D2-D5.

CREATE TABLE IF NOT EXISTS intelligence_claims (
    -- Immutable assertion columns (UPDATE-forbidden outside services/claims.rs).
    id              TEXT PRIMARY KEY,                  -- UUID v4 string
    subject_ref     TEXT NOT NULL,                     -- JSON: SubjectRef from W3-B
    claim_type      TEXT NOT NULL,                     -- ADR-0125 registry-validated
    field_path      TEXT,                              -- ADR-0113 structural key
    topic_key       TEXT,                              -- DOS-280 canonicalization (nullable)
    text            TEXT NOT NULL,                     -- canonicalized claim content
    dedup_key       TEXT NOT NULL,                     -- ADR-0113 section 8 + DOS-308 item_hash
    item_hash       TEXT,                              -- shared canonicalization::item_hash
    actor           TEXT NOT NULL,                     -- typed Actor enum string
    data_source     TEXT NOT NULL,                     -- ADR-0107 DataSource variant
    source_ref      TEXT,                              -- JSON: opaque per ADR-0107
    source_asof     TEXT,                              -- ADR-0105 amendment, must-be-populated-when-knowable
    observed_at     TEXT NOT NULL,                     -- when claim was observed
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    provenance_json TEXT NOT NULL,                     -- W3-B Provenance envelope
    metadata_json   TEXT,                              -- per-mechanism extra fields

    -- Lifecycle columns (UPDATE-allowed via services/claims.rs allowlist).
    claim_state         TEXT NOT NULL DEFAULT 'active'
                                  CHECK (claim_state IN ('active', 'dormant', 'tombstoned', 'withdrawn')),
    surfacing_state     TEXT NOT NULL DEFAULT 'active'
                                  CHECK (surfacing_state IN ('active', 'dormant')),
    demotion_reason     TEXT,
    reactivated_at      TEXT,
    retraction_reason   TEXT,                          -- e.g., 'user_removal'
    expires_at          TEXT,
    superseded_by       TEXT,                          -- FK soft-ref to intelligence_claims.id

    -- Trust columns (owned by W4 / DOS-5; D1 just stores them).
    trust_score         REAL,
    trust_computed_at   TEXT,
    trust_version       INTEGER,

    -- Threading + claim anatomy (ADR-0124 + ADR-0125).
    thread_id           TEXT,                          -- ADR-0124, nullable
    temporal_scope      TEXT NOT NULL DEFAULT 'state'
                                  CHECK (temporal_scope IN ('state', 'point_in_time', 'trend')),
    sensitivity         TEXT NOT NULL DEFAULT 'internal'
                                  CHECK (sensitivity IN ('public', 'internal', 'confidential', 'user_only'))
);

-- Indexes for default reads + suppression lookup.
CREATE INDEX IF NOT EXISTS idx_claims_default_read
    ON intelligence_claims(subject_ref, claim_state, surfacing_state, claim_type);

CREATE INDEX IF NOT EXISTS idx_claims_suppression_lookup
    ON intelligence_claims(subject_ref, claim_type, field_path, claim_state, dedup_key);

CREATE INDEX IF NOT EXISTS idx_claims_dedup_key
    ON intelligence_claims(dedup_key)
    WHERE claim_state = 'active';

CREATE INDEX IF NOT EXISTS idx_claims_thread_id
    ON intelligence_claims(thread_id)
    WHERE thread_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_claims_superseded_by
    ON intelligence_claims(superseded_by)
    WHERE superseded_by IS NOT NULL;

-- ---------------------------------------------------------------------------
-- Amendment A: claim_corroborations with strength/reinforcement fields.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS claim_corroborations (
    id                    TEXT PRIMARY KEY,            -- UUID v4
    claim_id              TEXT NOT NULL REFERENCES intelligence_claims(id),
    data_source           TEXT NOT NULL,               -- DataSource variant per DOS-212
    source_asof           TEXT,                        -- ADR-0105
    source_mechanism      TEXT,                        -- which legacy mechanism (backfill audit)
    strength              REAL NOT NULL DEFAULT 0.5
                                    CHECK (strength >= 0.0 AND strength <= 1.0),
    reinforcement_count   INTEGER NOT NULL DEFAULT 1,
    last_reinforced_at    TEXT NOT NULL DEFAULT (datetime('now')),
    created_at            TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_corroborations_claim
    ON claim_corroborations(claim_id);

CREATE INDEX IF NOT EXISTS idx_corroborations_source
    ON claim_corroborations(claim_id, data_source);

-- ---------------------------------------------------------------------------
-- Amendment C: claim_contradictions with branch/reconciliation fields.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS claim_contradictions (
    id                     TEXT PRIMARY KEY,             -- UUID v4
    primary_claim_id       TEXT NOT NULL REFERENCES intelligence_claims(id),
    contradicting_claim_id TEXT NOT NULL REFERENCES intelligence_claims(id),
    branch_kind            TEXT NOT NULL
                                  CHECK (branch_kind IN ('contradiction', 'clarification', 'supersession')),
    detected_at            TEXT NOT NULL DEFAULT (datetime('now')),
    reconciliation_kind    TEXT
                                  CHECK (reconciliation_kind IS NULL OR reconciliation_kind IN
                                         ('user_picked_winner', 'evidence_converged', 'merged_as_qualified', 'both_dormant')),
    reconciliation_note    TEXT,
    reconciled_at          TEXT,
    winner_claim_id        TEXT REFERENCES intelligence_claims(id),
    merged_claim_id        TEXT REFERENCES intelligence_claims(id)
);

CREATE INDEX IF NOT EXISTS idx_contradictions_primary
    ON claim_contradictions(primary_claim_id);

CREATE INDEX IF NOT EXISTS idx_contradictions_unreconciled
    ON claim_contradictions(reconciled_at)
    WHERE reconciled_at IS NULL;

-- ---------------------------------------------------------------------------
-- agent_trust_ledger: per-agent trust accumulator consumed by W4 Trust Compiler.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS agent_trust_ledger (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_kind         TEXT NOT NULL,                  -- 'pty' | 'glean' | 'human' etc.
    agent_id           TEXT NOT NULL,
    claim_type         TEXT,                           -- per-claim-type accumulation
    correct_count      INTEGER NOT NULL DEFAULT 0,
    incorrect_count    INTEGER NOT NULL DEFAULT 0,
    total_count        INTEGER NOT NULL DEFAULT 0,
    last_updated_at    TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (agent_kind, agent_id, claim_type)
);

CREATE INDEX IF NOT EXISTS idx_agent_trust_lookup
    ON agent_trust_ledger(agent_kind, agent_id);

-- ---------------------------------------------------------------------------
-- claim_feedback: ADR-0123 typed feedback target. Per ADR-0123 lines 91-106.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS claim_feedback (
    id              TEXT PRIMARY KEY,                  -- UUID v4
    claim_id        TEXT NOT NULL REFERENCES intelligence_claims(id),
    feedback_type   TEXT NOT NULL
                              CHECK (feedback_type IN ('confirm', 'correct', 'reject', 'wrong_subject', 'cannot_verify')),
    actor           TEXT NOT NULL,
    actor_id        TEXT,
    payload_json    TEXT,                              -- typed feedback content (correction, etc.)
    submitted_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_feedback_claim
    ON claim_feedback(claim_id);

CREATE INDEX IF NOT EXISTS idx_feedback_type
    ON claim_feedback(feedback_type, submitted_at);

-- ---------------------------------------------------------------------------
-- claim_repair_job: skeleton table for CannotVerify budgets per ADR-0123 lines 112-118.
-- D1 creates the shape; D2+ owns the dispatch/processing logic.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS claim_repair_job (
    id                   TEXT PRIMARY KEY,             -- UUID v4
    claim_id             TEXT NOT NULL REFERENCES intelligence_claims(id),
    feedback_id          TEXT REFERENCES claim_feedback(id),
    state                TEXT NOT NULL DEFAULT 'pending'
                                  CHECK (state IN ('pending', 'in_progress', 'completed', 'failed', 'budget_exhausted')),
    attempts             INTEGER NOT NULL DEFAULT 0,
    max_attempts         INTEGER NOT NULL DEFAULT 3,
    last_attempt_at      TEXT,
    completed_at         TEXT,
    error_message        TEXT,
    created_at           TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_repair_pending
    ON claim_repair_job(state, created_at)
    WHERE state IN ('pending', 'in_progress');

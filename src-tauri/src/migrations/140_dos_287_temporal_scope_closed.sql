-- Relax the intelligence_claims temporal_scope CHECK so existing databases
-- accept the Closed enum variant added to the trust substrate.

PRAGMA foreign_keys = OFF;

CREATE TABLE intelligence_claims_new (
    id              TEXT PRIMARY KEY,
    subject_ref     TEXT NOT NULL,
    claim_type      TEXT NOT NULL,
    field_path      TEXT,
    topic_key       TEXT,
    text            TEXT NOT NULL,
    dedup_key       TEXT NOT NULL,
    item_hash       TEXT,
    actor           TEXT NOT NULL,
    data_source     TEXT NOT NULL,
    source_ref      TEXT,
    source_asof     TEXT,
    observed_at     TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    provenance_json TEXT NOT NULL,
    metadata_json   TEXT,

    claim_state         TEXT NOT NULL DEFAULT 'active'
                                  CHECK (claim_state IN ('active', 'dormant', 'tombstoned', 'withdrawn')),
    surfacing_state     TEXT NOT NULL DEFAULT 'active'
                                  CHECK (surfacing_state IN ('active', 'dormant')),
    demotion_reason     TEXT,
    reactivated_at      TEXT,
    retraction_reason   TEXT,
    expires_at          TEXT,
    superseded_by       TEXT,

    trust_score         REAL,
    trust_computed_at   TEXT,
    trust_version       INTEGER,

    thread_id           TEXT,
    temporal_scope      TEXT NOT NULL DEFAULT 'state'
                                  CHECK (temporal_scope IN ('state', 'point_in_time', 'trend', 'closed')),
    sensitivity         TEXT NOT NULL DEFAULT 'internal'
                                  CHECK (sensitivity IN ('public', 'internal', 'confidential', 'user_only')),
    verification_state  TEXT NOT NULL DEFAULT 'active'
                                  CHECK (verification_state IN ('active', 'contested', 'needs_user_decision')),
    verification_reason TEXT,
    needs_user_decision_at TEXT
);

INSERT INTO intelligence_claims_new (
    id, subject_ref, claim_type, field_path, topic_key, text, dedup_key,
    item_hash, actor, data_source, source_ref, source_asof, observed_at,
    created_at, provenance_json, metadata_json, claim_state, surfacing_state,
    demotion_reason, reactivated_at, retraction_reason, expires_at,
    superseded_by, trust_score, trust_computed_at, trust_version, thread_id,
    temporal_scope, sensitivity, verification_state, verification_reason,
    needs_user_decision_at
)
SELECT
    id, subject_ref, claim_type, field_path, topic_key, text, dedup_key,
    item_hash, actor, data_source, source_ref, source_asof, observed_at,
    created_at, provenance_json, metadata_json, claim_state, surfacing_state,
    demotion_reason, reactivated_at, retraction_reason, expires_at,
    superseded_by, trust_score, trust_computed_at, trust_version, thread_id,
    temporal_scope, sensitivity, verification_state, verification_reason,
    needs_user_decision_at
FROM intelligence_claims;

DROP TABLE intelligence_claims;

ALTER TABLE intelligence_claims_new RENAME TO intelligence_claims;

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

PRAGMA foreign_keys = ON;

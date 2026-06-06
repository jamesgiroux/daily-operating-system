CREATE TABLE IF NOT EXISTS source_claim_type_reliability (
    source_key_version INTEGER NOT NULL,
    source_key_epoch_hash TEXT NOT NULL,
    source_key_hash TEXT NOT NULL,
    data_source TEXT NOT NULL,
    source_key_kind TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    signal_type TEXT NOT NULL,
    alpha REAL NOT NULL DEFAULT 1.0 CHECK (alpha >= 0.0),
    beta REAL NOT NULL DEFAULT 1.0 CHECK (beta >= 0.0),
    update_count INTEGER NOT NULL DEFAULT 0 CHECK (update_count >= 0),
    excluded_at TEXT,
    stale_key_version INTEGER,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (source_key_version, source_key_epoch_hash, source_key_hash, claim_type, signal_type)
);

CREATE INDEX IF NOT EXISTS idx_source_claim_type_reliability_lookup
    ON source_claim_type_reliability(source_key_hash, claim_type, signal_type, excluded_at);

CREATE TABLE IF NOT EXISTS source_reliability_feedback_deltas (
    id TEXT PRIMARY KEY,
    feedback_id TEXT NOT NULL REFERENCES claim_feedback(id),
    source_key_version INTEGER NOT NULL,
    source_key_epoch_hash TEXT NOT NULL,
    source_key_hash TEXT NOT NULL,
    data_source TEXT NOT NULL,
    source_key_kind TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    signal_type TEXT NOT NULL,
    effect_kind TEXT NOT NULL,
    alpha_delta REAL NOT NULL DEFAULT 0.0,
    beta_delta REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL DEFAULT 'applied'
        CHECK (status IN ('applied', 'redacted', 'source_removed', 'stale_key_version')),
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (feedback_id, signal_type, source_key_version, source_key_epoch_hash, source_key_hash, claim_type, effect_kind)
);

CREATE INDEX IF NOT EXISTS idx_source_reliability_feedback_deltas_key
    ON source_reliability_feedback_deltas(source_key_hash, claim_type, signal_type, status);

CREATE TABLE IF NOT EXISTS subject_inference_reliability (
    subject_ref_hash TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    signal_type TEXT NOT NULL,
    alpha REAL NOT NULL DEFAULT 1.0 CHECK (alpha >= 0.0),
    beta REAL NOT NULL DEFAULT 1.0 CHECK (beta >= 0.0),
    update_count INTEGER NOT NULL DEFAULT 0 CHECK (update_count >= 0),
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'redacted', 'subject_deleted', 'subject_orphaned', 'subject_rebound')),
    excluded_at TEXT,
    stale_key_version INTEGER,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (subject_ref_hash, claim_type, signal_type)
);

CREATE TABLE IF NOT EXISTS subject_inference_reliability_deltas (
    id TEXT PRIMARY KEY,
    feedback_id TEXT NOT NULL REFERENCES claim_feedback(id),
    subject_ref_hash TEXT NOT NULL,
    corrected_subject_ref_hash TEXT,
    claim_type TEXT NOT NULL,
    signal_type TEXT NOT NULL,
    alpha_delta REAL NOT NULL DEFAULT 0.0,
    beta_delta REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL DEFAULT 'applied'
        CHECK (status IN ('applied', 'redacted', 'subject_deleted', 'subject_orphaned')),
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (feedback_id, signal_type, subject_ref_hash, claim_type)
);

CREATE INDEX IF NOT EXISTS idx_subject_inference_deltas_subject
    ON subject_inference_reliability_deltas(subject_ref_hash, claim_type, status);

CREATE TABLE IF NOT EXISTS source_reliability_backfill_runs (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'aborted', 'stale_key_version')),
    source_key_version INTEGER NOT NULL,
    source_key_epoch_hash TEXT NOT NULL,
    cursor_json TEXT,
    high_water_feedback_id TEXT,
    source_cursor TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
    max_attempts INTEGER NOT NULL DEFAULT 5 CHECK (max_attempts > 0),
    failure_reason_code TEXT,
    stale_key_version_at TEXT,
    rekey_run_id TEXT,
    started_at TEXT,
    completed_at TEXT,
    terminalized_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_source_reliability_backfill_status
    ON source_reliability_backfill_runs(status, updated_at);

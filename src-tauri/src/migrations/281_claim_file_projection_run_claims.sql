CREATE TABLE IF NOT EXISTS claim_file_projection_run_claims (
    run_id TEXT NOT NULL REFERENCES claim_file_projection_runs(id) ON DELETE CASCADE,
    claim_id TEXT NOT NULL REFERENCES intelligence_claims(id) ON DELETE CASCADE,
    claim_version INTEGER NOT NULL,
    semantic_identity_json TEXT NOT NULL,
    trust_band TEXT NOT NULL,
    sensitivity TEXT NOT NULL,
    PRIMARY KEY (run_id, claim_id)
);

CREATE INDEX IF NOT EXISTS idx_claim_file_projection_run_claims_claim
    ON claim_file_projection_run_claims(claim_id, claim_version);

CREATE INDEX IF NOT EXISTS idx_claim_file_projection_run_claims_run_trust
    ON claim_file_projection_run_claims(run_id, trust_band, sensitivity);

CREATE TABLE IF NOT EXISTS claim_file_projection_path_bindings (
    markdown_rel_path TEXT PRIMARY KEY,
    sidecar_rel_path TEXT NOT NULL UNIQUE,
    entity_subject_compact TEXT NOT NULL UNIQUE,
    projection_root TEXT NOT NULL DEFAULT '_dailyos_claims'
        CHECK (projection_root = '_dailyos_claims'),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_claim_file_projection_path_bindings_subject
    ON claim_file_projection_path_bindings(entity_subject_compact);

CREATE TABLE IF NOT EXISTS claim_file_correction_apply_events (
    idempotency_key TEXT PRIMARY KEY,
    sidecar_checksum TEXT NOT NULL,
    claim_id TEXT NOT NULL REFERENCES intelligence_claims(id) ON DELETE CASCADE,
    projected_claim_version INTEGER NOT NULL,
    projected_identity_hash TEXT NOT NULL,
    feedback_action TEXT NOT NULL,
    payload_hash TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('claimed', 'applied', 'failed')),
    feedback_id TEXT REFERENCES claim_feedback(id) ON DELETE SET NULL,
    error_detail_hash TEXT,
    claimed_at TEXT NOT NULL,
    applied_at TEXT,
    failed_at TEXT,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_claim_file_correction_apply_claim_status
    ON claim_file_correction_apply_events(claim_id, status, updated_at);

CREATE INDEX IF NOT EXISTS idx_claim_file_correction_apply_sidecar
    ON claim_file_correction_apply_events(sidecar_checksum, status, updated_at);

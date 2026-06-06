CREATE TABLE IF NOT EXISTS claim_feedback_correction_envelopes (
    feedback_id TEXT PRIMARY KEY REFERENCES claim_feedback(id),
    claim_id TEXT NOT NULL REFERENCES intelligence_claims(id),
    action TEXT NOT NULL,
    actor TEXT NOT NULL,
    actor_id TEXT,
    surface TEXT NOT NULL DEFAULT 'unknown',
    target_receipt_json TEXT,
    asserted_subject_ref_json TEXT NOT NULL,
    asserted_subject_kind TEXT,
    asserted_subject_id TEXT,
    corrected_subject_ref_json TEXT,
    field_path TEXT,
    data_source TEXT NOT NULL,
    source_ref TEXT,
    source_ref_hash TEXT,
    source_asof TEXT,
    source_key_version INTEGER NOT NULL,
    source_key_epoch_hash TEXT NOT NULL,
    source_key_hash TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    sensitivity TEXT NOT NULL,
    idempotency_key TEXT,
    replay_key TEXT NOT NULL,
    action_metadata_json TEXT NOT NULL DEFAULT '{}',
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'redacted', 'source_removed', 'subject_deleted', 'subject_orphaned', 'meeting_removed', 'workspace_reset', 'proof_purged')),
    parent_lifecycle_state TEXT,
    lifecycle_reason_code TEXT,
    redacted_at TEXT,
    source_removed_at TEXT,
    subject_orphaned_at TEXT,
    meeting_removed_at TEXT,
    proof_purged_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_correction_envelopes_claim
    ON claim_feedback_correction_envelopes(claim_id, created_at);

CREATE INDEX IF NOT EXISTS idx_correction_envelopes_source_key
    ON claim_feedback_correction_envelopes(source_key_hash, claim_type, action);

CREATE INDEX IF NOT EXISTS idx_correction_envelopes_subject
    ON claim_feedback_correction_envelopes(asserted_subject_kind, asserted_subject_id, lifecycle_state);

CREATE TABLE IF NOT EXISTS claim_feedback_propagation_jobs (
    id TEXT PRIMARY KEY,
    feedback_id TEXT NOT NULL REFERENCES claim_feedback(id),
    action TEXT NOT NULL,
    target_kind TEXT NOT NULL,
    operation TEXT NOT NULL,
    sync_class TEXT NOT NULL CHECK (sync_class IN ('bounded_sync', 'async')),
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'stale', 'dead_lettered', 'coalesced')),
    coalescing_key TEXT NOT NULL,
    scope_json TEXT NOT NULL DEFAULT '{}',
    cursor_json TEXT,
    enqueue_run_id TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
    max_attempts INTEGER NOT NULL DEFAULT 5 CHECK (max_attempts > 0),
    next_run_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    stale_reason TEXT,
    failure_reason_code TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TEXT,
    dead_lettered_at TEXT,
    UNIQUE (feedback_id, target_kind, operation, coalescing_key, sync_class)
);

CREATE INDEX IF NOT EXISTS idx_feedback_propagation_jobs_feedback
    ON claim_feedback_propagation_jobs(feedback_id, target_kind, status);

CREATE INDEX IF NOT EXISTS idx_feedback_propagation_jobs_status
    ON claim_feedback_propagation_jobs(status, next_run_at, target_kind, updated_at);

CREATE INDEX IF NOT EXISTS idx_feedback_propagation_jobs_coalescing
    ON claim_feedback_propagation_jobs(coalescing_key, status);

CREATE TABLE IF NOT EXISTS claim_feedback_propagation_outcomes (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES claim_feedback_propagation_jobs(id),
    feedback_id TEXT NOT NULL REFERENCES claim_feedback(id),
    target_kind TEXT NOT NULL,
    operation TEXT NOT NULL,
    sync_class TEXT NOT NULL CHECK (sync_class IN ('bounded_sync', 'async')),
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'stale', 'dead_lettered', 'coalesced')),
    reason_code TEXT,
    detail_hash TEXT,
    observed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_feedback_propagation_outcomes_feedback
    ON claim_feedback_propagation_outcomes(feedback_id, target_kind, observed_at);

CREATE TABLE IF NOT EXISTS claim_feedback_review_queue_events (
    id TEXT PRIMARY KEY,
    feedback_id TEXT NOT NULL REFERENCES claim_feedback(id),
    claim_id TEXT NOT NULL REFERENCES intelligence_claims(id),
    operation TEXT NOT NULL,
    resolved_deferrals INTEGER NOT NULL DEFAULT 0 CHECK (resolved_deferrals >= 0),
    reason_code TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_claim_feedback_review_queue_events_feedback
    ON claim_feedback_review_queue_events(feedback_id, operation, created_at);

CREATE TABLE IF NOT EXISTS claim_feedback_subject_graph_invalidations (
    id TEXT PRIMARY KEY,
    feedback_id TEXT NOT NULL REFERENCES claim_feedback(id),
    claim_id TEXT NOT NULL REFERENCES intelligence_claims(id),
    subject_json TEXT NOT NULL,
    operation TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    entity_graph_version_after INTEGER,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_claim_feedback_subject_graph_invalidations_feedback
    ON claim_feedback_subject_graph_invalidations(feedback_id, operation, created_at);

CREATE TABLE IF NOT EXISTS correction_artifact_lifecycle_events (
    id TEXT PRIMARY KEY,
    artifact_kind TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK (event_type IN (
        'redacted',
        'source_removed',
        'subject_deleted',
        'subject_merged',
        'subject_rebound',
        'meeting_removed',
        'workspace_reset',
        'proof_purged',
        'declassification_revoked'
    )),
    actor TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    pii_safe_detail_hash TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_correction_lifecycle_artifact
    ON correction_artifact_lifecycle_events(artifact_kind, artifact_id, created_at);

CREATE TABLE IF NOT EXISTS correction_artifact_declassification_decisions (
    id TEXT PRIMARY KEY,
    artifact_kind TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    source_artifact_hash TEXT NOT NULL,
    source_artifact_version TEXT NOT NULL,
    derived_field TEXT NOT NULL,
    destination_surface TEXT NOT NULL,
    decision_version INTEGER NOT NULL DEFAULT 1,
    source_sensitivity TEXT NOT NULL CHECK (source_sensitivity IN ('public', 'internal', 'confidential', 'user_only')),
    requester_actor TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'revoked', 'expired')),
    reason_code TEXT NOT NULL,
    revoked_reason_code TEXT,
    parent_lifecycle_state TEXT,
    parent_version_hash TEXT NOT NULL,
    decided_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT,
    revoked_at TEXT,
    UNIQUE (
        artifact_kind,
        source_artifact_hash,
        source_artifact_version,
        derived_field,
        destination_surface,
        decision_version
    )
);

CREATE INDEX IF NOT EXISTS idx_declassification_decisions_active
    ON correction_artifact_declassification_decisions(destination_surface, derived_field, status, parent_version_hash);

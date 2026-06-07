CREATE TABLE IF NOT EXISTS dos338_stickiness_runs (
    id TEXT PRIMARY KEY,
    fixture_id_hash TEXT NOT NULL,
    entry_point TEXT NOT NULL CHECK (entry_point IN ('app', 'file_projection', 'mcp')),
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'purged')),
    report_hash TEXT,
    purge_state TEXT NOT NULL DEFAULT 'active' CHECK (purge_state IN ('active', 'purged')),
    reason_code TEXT,
    started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TEXT,
    purged_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_dos338_stickiness_runs_status
    ON dos338_stickiness_runs(status, entry_point, updated_at);

CREATE TABLE IF NOT EXISTS dos338_stickiness_observations (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES dos338_stickiness_runs(id),
    feedback_id TEXT REFERENCES claim_feedback(id) ON DELETE SET NULL,
    entry_point TEXT NOT NULL CHECK (entry_point IN ('app', 'file_projection', 'mcp')),
    action TEXT NOT NULL,
    subject_kind TEXT NOT NULL,
    subject_ref_hash TEXT NOT NULL,
    direct_surface TEXT NOT NULL,
    indirect_surface TEXT NOT NULL,
    direct_surface_before_hash TEXT,
    direct_surface_after_hash TEXT,
    indirect_surface_before_hash TEXT,
    indirect_surface_after_hash TEXT,
    pre_reenrichment_state_hash TEXT,
    post_reenrichment_state_hash TEXT,
    post_rebuild_state_hash TEXT,
    trust_band_before TEXT,
    trust_band_after TEXT,
    recompute_job_id TEXT,
    repair_job_id TEXT,
    dead_letter_reason TEXT,
    sensitivity_gate_result TEXT NOT NULL,
    result TEXT NOT NULL CHECK (result IN ('passed', 'failed', 'blocked_by_w5', 'blocked_by_privacy_gate')),
    reason_code TEXT,
    observed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_dos338_stickiness_observations_run
    ON dos338_stickiness_observations(run_id, result, observed_at);

CREATE INDEX IF NOT EXISTS idx_dos338_stickiness_observations_feedback
    ON dos338_stickiness_observations(feedback_id, observed_at);

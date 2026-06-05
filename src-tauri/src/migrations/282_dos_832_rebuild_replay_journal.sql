ALTER TABLE claim_feedback
    ADD COLUMN replay_event_id TEXT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_claim_feedback_replay_event
    ON claim_feedback(replay_event_id)
    WHERE replay_event_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS rebuild_correction_replay_events (
    sidecar_event_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    sidecar_schema_version INTEGER NOT NULL,
    source_runtime_claim_id TEXT NOT NULL,
    resolved_claim_id TEXT,
    action TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN (
        'claimed',
        'applied',
        'already_applied',
        'orphan_missing',
        'orphan_ambiguous',
        'failed'
    )),
    reason_code TEXT,
    reason_detail_hash TEXT,
    semantic_identity_hash TEXT NOT NULL,
    feedback_content_hash TEXT NOT NULL,
    applied_feedback_id TEXT REFERENCES claim_feedback(id),
    attempt_count INTEGER NOT NULL DEFAULT 1,
    claimed_at TEXT NOT NULL,
    applied_at TEXT,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rebuild_replay_events_run_status
    ON rebuild_correction_replay_events(run_id, status, updated_at);

CREATE INDEX IF NOT EXISTS idx_rebuild_replay_events_resolved_claim
    ON rebuild_correction_replay_events(resolved_claim_id, status);

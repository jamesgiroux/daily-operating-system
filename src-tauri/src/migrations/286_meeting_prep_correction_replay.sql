ALTER TABLE meeting_prep ADD COLUMN user_preparation_text TEXT;
ALTER TABLE meeting_prep ADD COLUMN user_hidden_attendees_json TEXT
    CHECK (user_hidden_attendees_json IS NULL OR json_valid(user_hidden_attendees_json) = 1);
ALTER TABLE meeting_prep ADD COLUMN user_decisions_json TEXT
    CHECK (user_decisions_json IS NULL OR json_valid(user_decisions_json) = 1);

CREATE TABLE IF NOT EXISTS meeting_prep_correction_journal (
    id TEXT PRIMARY KEY,
    feedback_id TEXT REFERENCES claim_feedback(id) ON DELETE SET NULL,
    meeting_stable_key TEXT NOT NULL,
    meeting_id TEXT,
    field_path TEXT NOT NULL,
    actor TEXT NOT NULL,
    surface TEXT NOT NULL,
    source_asof TEXT,
    sensitivity TEXT NOT NULL DEFAULT 'user_only' CHECK (sensitivity IN ('confidential', 'user_only')),
    replay_key TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    payload_hash TEXT NOT NULL,
    lifecycle_version INTEGER NOT NULL DEFAULT 1 CHECK (lifecycle_version > 0),
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'redacted', 'orphaned', 'meeting_removed', 'replayed')),
    replay_attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (replay_attempt_count >= 0),
    rebuild_replay_id TEXT,
    orphan_reason TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    replayed_at TEXT,
    orphaned_at TEXT,
    redacted_at TEXT,
    redaction_reason_code TEXT,
    meeting_removed_reason_code TEXT,
    UNIQUE (replay_key, field_path)
);

CREATE INDEX IF NOT EXISTS idx_meeting_prep_correction_journal_meeting
    ON meeting_prep_correction_journal(meeting_stable_key, field_path, lifecycle_state);

CREATE INDEX IF NOT EXISTS idx_meeting_prep_correction_journal_feedback
    ON meeting_prep_correction_journal(feedback_id);

CREATE TABLE IF NOT EXISTS meeting_prep_regeneration_jobs (
    id TEXT PRIMARY KEY,
    journal_id TEXT REFERENCES meeting_prep_correction_journal(id) ON DELETE SET NULL,
    meeting_stable_key TEXT NOT NULL,
    field_path TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'stale', 'dead_lettered', 'coalesced')),
    coalescing_key TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
    max_attempts INTEGER NOT NULL DEFAULT 5 CHECK (max_attempts > 0),
    next_run_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    stale_reason TEXT,
    failure_reason_code TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TEXT,
    dead_lettered_at TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_meeting_prep_regeneration_pending
    ON meeting_prep_regeneration_jobs(coalescing_key)
    WHERE status IN ('pending', 'running');

CREATE INDEX IF NOT EXISTS idx_meeting_prep_regeneration_status
    ON meeting_prep_regeneration_jobs(status, next_run_at, updated_at);

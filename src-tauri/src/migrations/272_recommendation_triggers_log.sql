CREATE TABLE IF NOT EXISTS triggers_log (
    run_id TEXT PRIMARY KEY,
    policy_version TEXT NOT NULL,
    trigger_class TEXT NOT NULL CHECK (
        trigger_class IN (
            'scheduled_freshness',
            'event_invalidation',
            'manual_refresh',
            'entity_change',
            'claim_change',
            'source_change',
            'open_loop_change',
            'meeting_window',
            'decision_window',
            'feedback_echo'
        )
    ),
    trigger_kind TEXT NOT NULL CHECK (
        trigger_kind IN ('signal_arrival', 'entity_change', 'scheduled_scan', 'feedback_echo')
    ),
    status TEXT NOT NULL CHECK (
        status IN ('started', 'completed', 'failed_retryable', 'failed_terminal')
    ),
    trigger_disposition TEXT NOT NULL CHECK (
        trigger_disposition IN (
            'silent_prepare',
            'primary_candidate',
            'review_candidate',
            'quiet_candidate'
        )
    ),
    result_kind TEXT NOT NULL DEFAULT 'prepared_silently' CHECK (
        result_kind IN (
            'prepared_silently',
            'render_decision_recorded',
            'held_for_review',
            'stayed_quiet',
            'failed'
        )
    ),
    downstream_policy TEXT NOT NULL DEFAULT 'recommendation_surfacing_policy' CHECK (
        downstream_policy = 'recommendation_surfacing_policy'
    ),
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    dedupe_key TEXT NOT NULL,
    suppression_key TEXT NOT NULL,
    trust_floor REAL NOT NULL CHECK (trust_floor >= 0.0 AND trust_floor <= 1.0),
    freshness_window_secs INTEGER NOT NULL CHECK (freshness_window_secs >= 0),
    source_signal_id TEXT,
    source_signal_type TEXT,
    source_asof TEXT,
    evidence_signature TEXT,
    subject_version INTEGER,
    signal_id TEXT,
    signal_coalesced INTEGER NOT NULL DEFAULT 0 CHECK (signal_coalesced IN (0, 1)),
    derived_signal_ids_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(derived_signal_ids_json) = 1),
    candidate_claim_ids_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(candidate_claim_ids_json) = 1),
    salience_evaluation_ids_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(salience_evaluation_ids_json) = 1),
    surfacing_decision_ids_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(surfacing_decision_ids_json) = 1),
    error_code TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
    next_retry_at TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_triggers_log_dedupe_policy
    ON triggers_log(policy_version, dedupe_key, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_triggers_log_subject
    ON triggers_log(subject_kind, subject_id, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_triggers_log_retry
    ON triggers_log(status, next_retry_at)
    WHERE status = 'failed_retryable';

CREATE INDEX IF NOT EXISTS idx_triggers_log_signal
    ON triggers_log(source_signal_id)
    WHERE source_signal_id IS NOT NULL;

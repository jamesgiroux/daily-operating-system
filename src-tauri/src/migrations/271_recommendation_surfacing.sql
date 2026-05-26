CREATE TABLE IF NOT EXISTS recommendation_surfacing_policy (
    policy_version TEXT NOT NULL,
    claim_type TEXT NOT NULL CHECK (claim_type = 'recommendation'),
    critical_threshold REAL NOT NULL CHECK (critical_threshold >= 0.0 AND critical_threshold <= 1.0),
    urgent_factor_threshold REAL NOT NULL CHECK (urgent_factor_threshold >= 0.0 AND urgent_factor_threshold <= 1.0),
    critical_primary_daily_budget INTEGER NOT NULL CHECK (critical_primary_daily_budget >= 0),
    notable_threshold REAL NOT NULL CHECK (notable_threshold >= 0.0 AND notable_threshold <= 1.0),
    notable_primary_daily_budget INTEGER NOT NULL CHECK (notable_primary_daily_budget >= 0),
    background_threshold REAL NOT NULL CHECK (background_threshold >= 0.0 AND background_threshold <= 1.0),
    background_daily_budget INTEGER NOT NULL CHECK (background_daily_budget >= 0),
    claim_cooldown_days INTEGER NOT NULL CHECK (claim_cooldown_days >= 0),
    subject_action_cooldown_days INTEGER NOT NULL CHECK (subject_action_cooldown_days >= 0),
    feedback_suppression_days INTEGER NOT NULL CHECK (feedback_suppression_days >= 0),
    policy_source TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (policy_version, claim_type)
);

INSERT INTO recommendation_surfacing_policy (
    policy_version,
    claim_type,
    critical_threshold,
    urgent_factor_threshold,
    critical_primary_daily_budget,
    notable_threshold,
    notable_primary_daily_budget,
    background_threshold,
    background_daily_budget,
    claim_cooldown_days,
    subject_action_cooldown_days,
    feedback_suppression_days,
    policy_source
) VALUES (
    'recommendation_surfacing_v1',
    'recommendation',
    0.85,
    0.90,
    1,
    0.68,
    3,
    0.45,
    10,
    7,
    3,
    14,
    'claim_type:recommendation'
)
ON CONFLICT(policy_version, claim_type) DO UPDATE SET
    critical_threshold = excluded.critical_threshold,
    urgent_factor_threshold = excluded.urgent_factor_threshold,
    critical_primary_daily_budget = excluded.critical_primary_daily_budget,
    notable_threshold = excluded.notable_threshold,
    notable_primary_daily_budget = excluded.notable_primary_daily_budget,
    background_threshold = excluded.background_threshold,
    background_daily_budget = excluded.background_daily_budget,
    claim_cooldown_days = excluded.claim_cooldown_days,
    subject_action_cooldown_days = excluded.subject_action_cooldown_days,
    feedback_suppression_days = excluded.feedback_suppression_days,
    policy_source = excluded.policy_source,
    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now');

CREATE TABLE IF NOT EXISTS surfacing_decisions (
    id TEXT PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    policy_version TEXT NOT NULL,
    claim_id TEXT NOT NULL REFERENCES intelligence_claims(id) ON DELETE CASCADE,
    decision_kind TEXT NOT NULL CHECK (decision_kind IN ('render', 'defer', 'suppress')),
    surfacing_tier TEXT CHECK (
        surfacing_tier IS NULL
        OR surfacing_tier IN ('critical', 'notable', 'background', 'quiet')
    ),
    defer_reason TEXT CHECK (
        defer_reason IS NULL
        OR defer_reason IN (
            'cooldown_active',
            'budget_exhausted',
            'awaiting_corroboration',
            'pending_trigger'
        )
    ),
    defer_until TEXT,
    suppress_reason TEXT CHECK (
        suppress_reason IS NULL
        OR suppress_reason IN (
            'below_threshold',
            'user_muted_subject',
            'dismissed_recently',
            'contradicted_with_stronger_evidence'
        )
    ),
    budget_key TEXT NOT NULL,
    actor_kind TEXT NOT NULL,
    local_day TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    sensitivity TEXT NOT NULL,
    surface_class TEXT NOT NULL CHECK (surface_class IN ('primary', 'background', 'quiet', 'review')),
    render_surface TEXT NOT NULL,
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    action_signature TEXT NOT NULL,
    salience_total REAL NOT NULL CHECK (salience_total >= 0.0 AND salience_total <= 1.0),
    salience_evaluation_id TEXT NOT NULL,
    why_this_now_json TEXT CHECK (why_this_now_json IS NULL OR json_valid(why_this_now_json) = 1),
    trigger_refs_json TEXT NOT NULL CHECK (json_valid(trigger_refs_json) = 1),
    evidence_signature TEXT,
    source_asof TEXT,
    source_signal_id TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_surfacing_decisions_claim_created
    ON surfacing_decisions(claim_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_surfacing_decisions_budget
    ON surfacing_decisions(policy_version, budget_key, decision_kind, surfacing_tier);

CREATE INDEX IF NOT EXISTS idx_surfacing_decisions_subject_action
    ON surfacing_decisions(
        policy_version,
        subject_kind,
        subject_id,
        action_signature,
        surface_class,
        created_at DESC
    );

CREATE INDEX IF NOT EXISTS idx_surfacing_decisions_salience
    ON surfacing_decisions(salience_evaluation_id);

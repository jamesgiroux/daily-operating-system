--: Success plan data model + template support
-- Rebuild entity_assessment to add success_plan_signals_json column.

BEGIN IMMEDIATE;

DROP TABLE IF EXISTS entity_assessment_new;
CREATE TABLE entity_assessment_new (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL DEFAULT 'account',
    enriched_at TEXT,
    source_file_count INTEGER DEFAULT 0,
    executive_assessment TEXT,
    risks_json TEXT,
    recent_wins_json TEXT,
    current_state_json TEXT,
    stakeholder_insights_json TEXT,
    next_meeting_readiness_json TEXT,
    company_context_json TEXT,
    value_delivered TEXT,
    success_metrics TEXT,
    open_commitments TEXT,
    relationship_depth TEXT,
    health_json TEXT,
    org_health_json TEXT,
    user_relevance_weight REAL DEFAULT 1.0,
    consistency_status TEXT,
    consistency_findings_json TEXT,
    consistency_checked_at TEXT,
    portfolio_json TEXT,
    network_json TEXT,
    user_edits_json TEXT,
    source_manifest_json TEXT,
    dimensions_json TEXT,
    success_plan_signals_json TEXT
);

INSERT OR REPLACE INTO entity_assessment_new (
    entity_id,
    entity_type,
    enriched_at,
    source_file_count,
    executive_assessment,
    risks_json,
    recent_wins_json,
    current_state_json,
    stakeholder_insights_json,
    next_meeting_readiness_json,
    company_context_json,
    value_delivered,
    success_metrics,
    open_commitments,
    relationship_depth,
    health_json,
    org_health_json,
    user_relevance_weight,
    consistency_status,
    consistency_findings_json,
    consistency_checked_at,
    portfolio_json,
    network_json,
    user_edits_json,
    source_manifest_json,
    dimensions_json,
    success_plan_signals_json
)
SELECT
    entity_id,
    entity_type,
    enriched_at,
    source_file_count,
    executive_assessment,
    risks_json,
    recent_wins_json,
    current_state_json,
    stakeholder_insights_json,
    next_meeting_readiness_json,
    company_context_json,
    value_delivered,
    success_metrics,
    open_commitments,
    relationship_depth,
    health_json,
    org_health_json,
    user_relevance_weight,
    consistency_status,
    consistency_findings_json,
    consistency_checked_at,
    portfolio_json,
    network_json,
    user_edits_json,
    source_manifest_json,
    dimensions_json,
    NULL
FROM entity_assessment;

DROP TABLE entity_assessment;
ALTER TABLE entity_assessment_new RENAME TO entity_assessment;
CREATE INDEX IF NOT EXISTS idx_entity_assessment_type ON entity_assessment(entity_type);

COMMIT;

CREATE TABLE IF NOT EXISTS account_objectives (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    target_date TEXT,
    completed_at TEXT,
    source TEXT NOT NULL DEFAULT 'user',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_account_objectives_account
    ON account_objectives(account_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_account_objectives_status
    ON account_objectives(status);

CREATE TABLE IF NOT EXISTS account_milestones (
    id TEXT PRIMARY KEY,
    objective_id TEXT NOT NULL REFERENCES account_objectives(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    target_date TEXT,
    completed_at TEXT,
    auto_detect_signal TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_account_milestones_objective
    ON account_milestones(objective_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_account_milestones_account
    ON account_milestones(account_id);

CREATE TABLE IF NOT EXISTS action_objective_links (
    action_id TEXT NOT NULL REFERENCES actions(id) ON DELETE CASCADE,
    objective_id TEXT NOT NULL REFERENCES account_objectives(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (action_id, objective_id)
);

CREATE INDEX IF NOT EXISTS idx_action_objective_links_objective
    ON action_objective_links(objective_id);

CREATE TABLE IF NOT EXISTS captured_commitments (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    meeting_id TEXT REFERENCES meetings(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    owner TEXT,
    target_date TEXT,
    confidence TEXT NOT NULL DEFAULT 'medium',
    source TEXT,
    consumed INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_captured_commitments_account
    ON captured_commitments(account_id, consumed, created_at DESC);

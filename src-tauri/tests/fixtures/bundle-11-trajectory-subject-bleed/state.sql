CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE intelligence_claims (
    id TEXT PRIMARY KEY,
    subject_ref TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    field_path TEXT,
    topic_key TEXT,
    text TEXT NOT NULL,
    dedup_key TEXT NOT NULL,
    item_hash TEXT,
    actor TEXT NOT NULL,
    data_source TEXT NOT NULL,
    source_ref TEXT,
    source_asof TEXT,
    observed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    provenance_json TEXT NOT NULL,
    metadata_json TEXT,
    claim_state TEXT NOT NULL DEFAULT 'active',
    surfacing_state TEXT NOT NULL DEFAULT 'active',
    demotion_reason TEXT,
    reactivated_at TEXT,
    retraction_reason TEXT,
    expires_at TEXT,
    superseded_by TEXT,
    trust_score REAL,
    trust_computed_at TEXT,
    trust_version INTEGER,
    thread_id TEXT,
    temporal_scope TEXT NOT NULL DEFAULT 'state',
    sensitivity TEXT NOT NULL DEFAULT 'internal',
    verification_state TEXT NOT NULL DEFAULT 'active',
    verification_reason TEXT,
    needs_user_decision_at TEXT
);

CREATE TABLE entity_engagement_curve (
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    week_start TEXT NOT NULL,
    meetings_count INTEGER NOT NULL,
    emails_count INTEGER NOT NULL,
    bidirectional_ratio REAL NOT NULL,
    source_refs_json TEXT NOT NULL,
    PRIMARY KEY (entity_type, entity_id, week_start)
);

INSERT INTO accounts (id, name) VALUES
    ('acct-risk-target', 'Target Account'),
    ('acct-risk-adjacent', 'Adjacent Account');

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, dedup_key, item_hash, actor,
    data_source, source_ref, source_asof, observed_at, created_at, provenance_json, metadata_json,
    claim_state, surfacing_state, trust_score, temporal_scope, sensitivity, verification_state
) VALUES
    ('claim-risk-target-engagement', '{"kind":"account","id":"acct-risk-target"}', 'account_risk_signal', 'engagement', NULL, 'Target Account has lower recent engagement.', 'dedup-risk-target-engagement', 'hash-risk-target-engagement', 'agent:fixture', 'user', 'src-risk-target-engagement', '2026-05-12T09:00:00Z', '2026-05-12T09:00:00Z', '2026-05-12T09:00:00Z', '{}', NULL, 'active', 'active', 0.91, 'state', 'internal', 'active'),
    ('claim-risk-adjacent-engagement', '{"kind":"account","id":"acct-risk-adjacent"}', 'account_risk_signal', 'engagement', NULL, 'Adjacent Account has unrelated engagement decline.', 'dedup-risk-adjacent-engagement', 'hash-risk-adjacent-engagement', 'agent:fixture', 'user', 'src-risk-adjacent-engagement', '2026-05-12T10:00:00Z', '2026-05-12T10:00:00Z', '2026-05-12T10:00:00Z', '{}', NULL, 'active', 'active', 0.91, 'state', 'internal', 'active');

INSERT INTO entity_engagement_curve (
    entity_type, entity_id, week_start, meetings_count, emails_count, bidirectional_ratio, source_refs_json
) VALUES
    ('account', 'acct-risk-target', '2026-04-14T12:00:00Z', 6, 4, 0.60, '["src-risk-target-engagement"]'),
    ('account', 'acct-risk-target', '2026-05-14T12:00:00Z', 2, 3, 0.50, '["src-risk-target-engagement"]'),
    ('account', 'acct-risk-adjacent', '2026-04-14T12:00:00Z', 4, 4, 0.70, '["src-risk-adjacent-engagement"]'),
    ('account', 'acct-risk-adjacent', '2026-05-14T12:00:00Z', 1, 2, 0.40, '["src-risk-adjacent-engagement"]');

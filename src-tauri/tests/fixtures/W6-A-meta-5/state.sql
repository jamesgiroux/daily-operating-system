-- W6-A-meta-5: detect_risk_shift / happy / degrading account.
-- Minimum substrate: one current risk claim for the requested account.
CREATE TABLE IF NOT EXISTS entities (
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    tracker_path TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (id, entity_type)
);

CREATE TABLE IF NOT EXISTS intelligence_claims (
    id TEXT PRIMARY KEY,
    subject_ref TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    field_path TEXT,
    topic_key TEXT,
    text TEXT NOT NULL,
    actor TEXT NOT NULL,
    data_source TEXT NOT NULL,
    source_ref TEXT,
    source_asof TEXT,
    observed_at TEXT NOT NULL,
    provenance_json TEXT NOT NULL,
    claim_state TEXT NOT NULL DEFAULT 'active',
    surfacing_state TEXT NOT NULL DEFAULT 'active',
    trust_score REAL,
    temporal_scope TEXT NOT NULL DEFAULT 'state',
    sensitivity TEXT NOT NULL DEFAULT 'internal',
    verification_state TEXT NOT NULL DEFAULT 'active'
);

INSERT INTO entities (id, name, entity_type, tracker_path, updated_at)
VALUES ('account-meta-5-example', 'Account Meta 5 Example', 'account', NULL, '2026-05-15T10:00:00Z');

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, actor, data_source,
    source_ref, source_asof, observed_at, provenance_json, trust_score,
    temporal_scope, sensitivity, verification_state
)
VALUES (
    'claim-meta-5-degrading',
    '{"kind":"account","id":"account-meta-5-example"}',
    'risk_signal',
    'risk.adoption',
    'account-meta-5-example:risk:adoption',
    'Recent account notes show adoption slipping and implementation concerns escalating.',
    'agent:fixture',
    'support',
    '{"source_id":"source-meta-5-support"}',
    '2026-05-14T18:00:00Z',
    '2026-05-14T18:05:00Z',
    '{"source_id":"source-meta-5-support","risk_direction":"increasing"}',
    0.78,
    'state',
    'internal',
    'active'
);

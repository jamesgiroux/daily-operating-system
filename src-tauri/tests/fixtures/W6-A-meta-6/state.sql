-- W6-A-meta-6: detect_risk_shift / stale / stale champion silence.
-- Minimum substrate: one stale champion-silence claim that cannot drive current risk as fresh evidence.
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
    verification_state TEXT NOT NULL DEFAULT 'active',
    verification_reason TEXT
);

INSERT INTO entities (id, name, entity_type, tracker_path, updated_at)
VALUES ('account-meta-6-example', 'Account Meta 6 Example', 'account', NULL, '2026-05-15T10:00:00Z');

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, actor, data_source,
    source_ref, source_asof, observed_at, provenance_json, claim_state,
    surfacing_state, trust_score, temporal_scope, sensitivity, verification_state,
    verification_reason
)
VALUES (
    'claim-meta-6-champion-silence',
    '{"kind":"account","id":"account-meta-6-example"}',
    'risk_signal',
    'champion.engagement',
    'account-meta-6-example:risk:champion-silence',
    'The champion has not replied since early March.',
    'agent:fixture',
    'email',
    '{"source_id":"source-meta-6-email"}',
    '2026-03-04T15:00:00Z',
    '2026-03-04T15:05:00Z',
    '{"source_id":"source-meta-6-email","stale_asof_days":72}',
    'active',
    'active',
    0.42,
    'state',
    'internal',
    'stale',
    'source_asof_outside_current_window'
);

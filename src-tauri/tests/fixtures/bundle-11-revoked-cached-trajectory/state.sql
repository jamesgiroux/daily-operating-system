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

CREATE TABLE glean_source_lifecycle (
    source_ref TEXT PRIMARY KEY,
    lifecycle TEXT NOT NULL,
    revoked_at TEXT
);

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, dedup_key, item_hash, actor,
    data_source, source_ref, source_asof, observed_at, created_at, provenance_json, metadata_json,
    claim_state, surfacing_state, trust_score, temporal_scope, sensitivity, verification_state
) VALUES
    ('claim-risk-active-cache', '{"kind":"account","id":"acct-risk-revoked-cache"}', 'account_risk_signal', 'engagement', NULL, 'Active evidence shows improved engagement.', 'dedup-risk-active-cache', 'hash-risk-active-cache', 'agent:fixture', 'user', 'src-risk-active-cache', '2026-05-13T09:00:00Z', '2026-05-13T09:00:00Z', '2026-05-13T09:00:00Z', '{}', NULL, 'active', 'active', 0.90, 'state', 'internal', 'active'),
    ('claim-risk-revoked-cache', '{"kind":"account","id":"acct-risk-revoked-cache"}', 'account_risk_signal', 'engagement', NULL, 'Revoked Glean evidence must not support risk output.', 'dedup-risk-revoked-cache', 'hash-risk-revoked-cache', 'agent:fixture', 'glean', 'src-risk-revoked-cache', '2026-04-10T09:00:00Z', '2026-04-10T09:00:00Z', '2026-04-10T09:00:00Z', '{}', '{ "lifecycle": "revoked" }', 'active', 'active', 0.30, 'state', 'internal', 'active');

INSERT INTO glean_source_lifecycle (source_ref, lifecycle, revoked_at)
VALUES ('src-risk-revoked-cache', 'revoked', '2026-05-13T18:00:00Z');

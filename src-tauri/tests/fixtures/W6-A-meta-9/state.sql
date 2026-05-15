-- W6-A-meta-9: list_open_loops_extract_commitments / stale / commitments past TTL.
-- Minimum substrate: one expired commitment that should not render as an active open loop.
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
    metadata_json TEXT,
    claim_state TEXT NOT NULL DEFAULT 'active',
    surfacing_state TEXT NOT NULL DEFAULT 'active',
    expires_at TEXT,
    trust_score REAL,
    temporal_scope TEXT NOT NULL DEFAULT 'state',
    sensitivity TEXT NOT NULL DEFAULT 'internal',
    verification_state TEXT NOT NULL DEFAULT 'active',
    verification_reason TEXT
);

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, actor, data_source,
    source_ref, source_asof, observed_at, provenance_json, metadata_json,
    claim_state, surfacing_state, expires_at, trust_score, temporal_scope,
    sensitivity, verification_state, verification_reason
)
VALUES (
    'claim-meta-9-expired-commitment',
    '{"kind":"account","id":"account-meta-9-example"}',
    'open_loop',
    'commitment.follow_up',
    'account-meta-9-example:commitment:follow-up',
    'Send a follow-up summary after the April review.',
    'agent:fixture',
    'meeting_transcript',
    '{"source_id":"source-meta-9-transcript"}',
    '2026-04-01T16:00:00Z',
    '2026-04-01T16:05:00Z',
    '{"source_id":"source-meta-9-transcript","ttl_days":30}',
    '{"ttl_expired_at":"2026-05-01T00:00:00Z"}',
    'active',
    'active',
    '2026-05-01T00:00:00Z',
    0.51,
    'state',
    'internal',
    'stale',
    'commitment_ttl_expired'
);

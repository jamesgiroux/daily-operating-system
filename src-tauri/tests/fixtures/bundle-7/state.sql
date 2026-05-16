PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    lifecycle TEXT,
    arr REAL,
    health TEXT CHECK(health IN ('green', 'yellow', 'red')),
    contract_start TEXT,
    contract_end TEXT,
    csm TEXT,
    champion TEXT,
    nps INTEGER,
    tracker_path TEXT,
    parent_id TEXT,
    updated_at TEXT NOT NULL,
    archived INTEGER DEFAULT 0,
    is_internal INTEGER NOT NULL DEFAULT 0,
    account_type TEXT NOT NULL DEFAULT 'customer',
    keywords TEXT,
    keywords_extracted_at TEXT,
    metadata TEXT DEFAULT '{}',
    commercial_stage TEXT,
    claim_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS account_domains (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    domain TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'enrichment',
    PRIMARY KEY (account_id, domain)
);

CREATE TABLE IF NOT EXISTS account_source_refs (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    field TEXT NOT NULL,
    source_system TEXT NOT NULL,
    source_kind TEXT NOT NULL DEFAULT 'inference',
    source_value TEXT,
    observed_at TEXT NOT NULL,
    source_record_ref TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS signal_weights (
    source TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    signal_type TEXT NOT NULL,
    alpha REAL DEFAULT 1.0,
    beta REAL DEFAULT 1.0,
    update_count INTEGER DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (source, entity_type, signal_type)
);

CREATE TABLE IF NOT EXISTS intelligence_claims (
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
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    provenance_json TEXT NOT NULL,
    metadata_json TEXT,
    claim_state TEXT NOT NULL DEFAULT 'active'
        CHECK (claim_state IN ('active', 'dormant', 'tombstoned', 'withdrawn')),
    surfacing_state TEXT NOT NULL DEFAULT 'active'
        CHECK (surfacing_state IN ('active', 'dormant')),
    demotion_reason TEXT,
    reactivated_at TEXT,
    retraction_reason TEXT,
    expires_at TEXT,
    superseded_by TEXT,
    trust_score REAL,
    trust_computed_at TEXT,
    trust_version INTEGER,
    thread_id TEXT,
    temporal_scope TEXT NOT NULL DEFAULT 'state'
        CHECK (temporal_scope IN ('state', 'point_in_time', 'trend', 'closed')),
    sensitivity TEXT NOT NULL DEFAULT 'internal'
        CHECK (sensitivity IN ('public', 'internal', 'confidential', 'user_only')),
    verification_state TEXT NOT NULL DEFAULT 'active'
        CHECK (verification_state IN ('active', 'contested', 'needs_user_decision')),
    verification_reason TEXT,
    needs_user_decision_at TEXT,
    claim_version INTEGER NOT NULL DEFAULT 1,
    canonical_status TEXT NOT NULL DEFAULT 'live',
    non_semantic_mergeable BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS claim_corroborations (
    id TEXT PRIMARY KEY,
    claim_id TEXT NOT NULL REFERENCES intelligence_claims(id),
    data_source TEXT NOT NULL,
    source_asof TEXT,
    source_mechanism TEXT,
    strength REAL NOT NULL DEFAULT 0.5 CHECK (strength >= 0.0 AND strength <= 1.0),
    reinforcement_count INTEGER NOT NULL DEFAULT 1,
    last_reinforced_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_source_refs_account_field
    ON account_source_refs(account_id, field);
CREATE INDEX IF NOT EXISTS idx_claims_default_read
    ON intelligence_claims(subject_ref, claim_state, surfacing_state, claim_type);
CREATE INDEX IF NOT EXISTS idx_claims_suppression_lookup
    ON intelligence_claims(subject_ref, claim_type, field_path, claim_state, dedup_key);
CREATE INDEX IF NOT EXISTS idx_claims_dedup_key
    ON intelligence_claims(dedup_key)
    WHERE claim_state = 'active';
CREATE INDEX IF NOT EXISTS idx_corroborations_claim
    ON claim_corroborations(claim_id);

INSERT INTO accounts (
    id, name, lifecycle, health, tracker_path, updated_at, archived,
    is_internal, account_type, claim_version
) VALUES (
    'acct-test-1', 'acme.example.com', 'active', 'green',
    'Accounts/acme.example.com', '2026-05-01T12:00:00Z', 0,
    0, 'customer', 0
);

INSERT INTO account_domains (account_id, domain, source)
VALUES ('acct-test-1', 'acme.example.com', 'enrichment');

INSERT INTO account_source_refs (
    id, account_id, field, source_system, source_kind, source_value,
    observed_at, source_record_ref, created_at
) VALUES
(
    'src-test-source-original',
    'acct-test-1',
    'executiveAssessment',
    'account_source_ref',
    'closed_scope_fact',
    'As of 2025-11-02T12:00:00Z, acme.example.com had completed the renewal checklist.',
    '2025-11-02T12:00:00Z',
    '{"source_id":"src-test-source-original","source_asof":"2025-11-02T12:00:00Z","lifecycle_state":"active","temporal_scope":"closed","observed_at_window_end":"2025-11-02T12:00:00Z","evidence_weight":0.9}',
    '2025-11-02T12:00:00Z'
),
(
    'src-test-source-stale-postclosure',
    'acct-test-1',
    'executiveAssessment',
    'provider_completion',
    'post_closure_fact',
    'Post-closure observation repeats the closed-window renewal checklist completion.',
    '2026-04-01T12:00:00Z',
    '{"source_id":"src-test-source-stale-postclosure","source_asof":"2026-04-01T12:00:00Z","lifecycle_state":"active","target_claim_id":"claim-test-closed-renewal-checklist","target_temporal_scope":"closed","target_observed_at_window_end":"2025-11-02T12:00:00Z","days_after_window_end":150,"evidence_weight":0.7}',
    '2026-04-01T12:00:00Z'
);

INSERT INTO signal_weights (
    source, entity_type, signal_type, alpha, beta, update_count, updated_at
) VALUES
    ('account_source_ref', 'account', 'source_reliability', 0.90, 0.10, 1, '2026-05-01T12:00:00Z'),
    ('provider_completion', 'account', 'enrichment_quality', 0.70, 0.30, 1, '2026-05-01T12:00:00Z');

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text,
    dedup_key, item_hash, actor, data_source, source_ref, source_asof,
    observed_at, created_at, provenance_json, metadata_json,
    claim_state, surfacing_state, trust_score, trust_computed_at,
    trust_version, thread_id, temporal_scope, sensitivity, verification_state
) VALUES (
    'claim-test-closed-renewal-checklist',
    '{"kind":"account","id":"acct-test-1"}',
    'entity_summary',
    'executiveAssessment',
    'acct-test-1:renewal-checklist:closed-window',
    'As of 2025-11-02T12:00:00Z, acme.example.com had completed the renewal checklist.',
    'acct-test-1|entity_summary|executiveAssessment|renewal-checklist|closed-window',
    'sha256:closed-renewal-checklist',
    'system:fixture',
    'account_source_ref',
    '{"source_id":"src-test-source-original","source_system":"account_source_ref","lifecycle_state":"active","temporal_scope":"closed","observed_at_window_end":"2025-11-02T12:00:00Z","evidence_weight":0.9}',
    '2025-11-02T12:00:00Z',
    '2025-11-02T12:00:00Z',
    '2025-11-02T12:00:00Z',
    '{"sources":["src-test-source-original"],"source_asof":"2025-11-02T12:00:00Z","lifecycle_state":"active","temporal_scope":"closed","observed_at_window_end":"2025-11-02T12:00:00Z"}',
    '{"fixture_bundle":7,"closed_window":true,"original_trust_score":0.76,"observed_at_window_end":"2025-11-02T12:00:00Z","source_lifecycle_state":"active"}',
    'active',
    'active',
    0.76,
    '2025-11-02T12:00:00Z',
    1,
    'thread-test-closed-renewal-checklist',
    'closed',
    'internal',
    'active'
);

INSERT INTO claim_corroborations (
    id, claim_id, data_source, source_asof, source_mechanism,
    strength, reinforcement_count, last_reinforced_at, created_at
) VALUES (
    'corroboration-test-source-original',
    'claim-test-closed-renewal-checklist',
    'account_source_ref',
    '2025-11-02T12:00:00Z',
    'src-test-source-original',
    0.76,
    1,
    '2025-11-02T12:00:00Z',
    '2025-11-02T12:00:00Z'
);

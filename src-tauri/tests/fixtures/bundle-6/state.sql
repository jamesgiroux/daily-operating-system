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

CREATE TABLE IF NOT EXISTS people (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    organization TEXT,
    role TEXT,
    relationship TEXT CHECK(relationship IN ('internal', 'external', 'unknown')) DEFAULT 'unknown',
    notes TEXT,
    tracker_path TEXT,
    last_seen TEXT,
    first_seen TEXT,
    meeting_count INTEGER DEFAULT 0,
    updated_at TEXT NOT NULL,
    archived INTEGER DEFAULT 0,
    claim_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS account_stakeholders (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'associated',
    relationship_type TEXT DEFAULT 'associated',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (account_id, person_id)
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
        CHECK (temporal_scope IN ('state', 'point_in_time', 'trend')),
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

CREATE TABLE IF NOT EXISTS claim_contradictions (
    id TEXT PRIMARY KEY,
    primary_claim_id TEXT NOT NULL REFERENCES intelligence_claims(id),
    contradicting_claim_id TEXT NOT NULL REFERENCES intelligence_claims(id),
    branch_kind TEXT NOT NULL CHECK (branch_kind IN ('contradiction', 'clarification', 'supersession')),
    detected_at TEXT NOT NULL DEFAULT (datetime('now')),
    reconciliation_kind TEXT CHECK (
        reconciliation_kind IS NULL OR reconciliation_kind IN
        ('user_picked_winner', 'evidence_converged', 'merged_as_qualified', 'both_dormant')
    ),
    reconciliation_note TEXT,
    reconciled_at TEXT,
    winner_claim_id TEXT REFERENCES intelligence_claims(id),
    merged_claim_id TEXT REFERENCES intelligence_claims(id)
);

CREATE TABLE IF NOT EXISTS claim_feedback (
    id TEXT PRIMARY KEY,
    claim_id TEXT NOT NULL REFERENCES intelligence_claims(id),
    feedback_type TEXT NOT NULL CHECK (feedback_type IN (
        'confirm_current',
        'mark_outdated',
        'mark_false',
        'wrong_subject',
        'wrong_source',
        'cannot_verify',
        'needs_nuance',
        'surface_inappropriate',
        'not_relevant_here'
    )),
    actor TEXT NOT NULL,
    actor_id TEXT,
    payload_json TEXT,
    submitted_at TEXT NOT NULL DEFAULT (datetime('now')),
    applied_at TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_claims_default_read
    ON intelligence_claims(subject_ref, claim_state, surfacing_state, claim_type);
CREATE INDEX IF NOT EXISTS idx_claims_suppression_lookup
    ON intelligence_claims(subject_ref, claim_type, field_path, claim_state, dedup_key);
CREATE INDEX IF NOT EXISTS idx_claims_dedup_key
    ON intelligence_claims(dedup_key)
    WHERE claim_state = 'active';
CREATE INDEX IF NOT EXISTS idx_corroborations_claim
    ON claim_corroborations(claim_id);
CREATE INDEX IF NOT EXISTS idx_contradictions_primary
    ON claim_contradictions(primary_claim_id);
CREATE INDEX IF NOT EXISTS idx_feedback_claim
    ON claim_feedback(claim_id);

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

INSERT INTO people (
    id, email, name, organization, role, relationship,
    first_seen, last_seen, updated_at, archived, claim_version
) VALUES (
    'person-test-jane-doe', 'jane.doe@example.com', 'Jane Doe',
    'acme.example.com', 'Primary contact', 'external',
    '2026-04-29T12:00:00Z', '2026-05-01T12:00:00Z',
    '2026-05-01T12:00:00Z', 0, 0
);

INSERT INTO account_stakeholders (
    account_id, person_id, role, relationship_type, created_at
) VALUES (
    'acct-test-1', 'person-test-jane-doe', 'primary_contact', 'stakeholder',
    '2026-04-29T12:00:00Z'
);

INSERT INTO account_source_refs (
    id, account_id, field, source_system, source_kind, source_value,
    observed_at, source_record_ref, created_at
) VALUES
(
    'src-test-strong',
    'acct-test-1',
    'primary_contact',
    'user_authored',
    'fact',
    'acme.example.com primary contact is jane.doe@example.com.',
    '2026-04-29T12:00:00Z',
    '{"source_id":"src-test-strong","source_asof":"2026-04-29T12:00:00Z","lifecycle_state":"active","evidence_weight":1.0,"scoring_class":"authoritative_user_fact"}',
    '2026-04-29T12:00:00Z'
),
(
    'src-test-weak-1',
    'acct-test-1',
    'primary_contact',
    'third_party_scrape',
    'scraped_profile',
    'Scraped record 1 lists the primary contact as unknown.',
    '2026-04-30T08:00:00Z',
    '{"source_id":"src-test-weak-1","source_asof":"2026-04-30T08:00:00Z","lifecycle_state":"active","evidence_weight":0.2,"scoring_class":"weak_scraped_reference"}',
    '2026-04-30T08:00:00Z'
),
(
    'src-test-weak-2',
    'acct-test-1',
    'primary_contact',
    'third_party_scrape',
    'scraped_profile',
    'Scraped record 2 lists the primary contact as unknown.',
    '2026-04-30T08:15:00Z',
    '{"source_id":"src-test-weak-2","source_asof":"2026-04-30T08:15:00Z","lifecycle_state":"active","evidence_weight":0.2,"scoring_class":"weak_scraped_reference"}',
    '2026-04-30T08:15:00Z'
),
(
    'src-test-weak-3',
    'acct-test-1',
    'primary_contact',
    'third_party_scrape',
    'scraped_profile',
    'Scraped record 3 lists the primary contact as unknown.',
    '2026-04-30T08:30:00Z',
    '{"source_id":"src-test-weak-3","source_asof":"2026-04-30T08:30:00Z","lifecycle_state":"active","evidence_weight":0.2,"scoring_class":"weak_scraped_reference"}',
    '2026-04-30T08:30:00Z'
),
(
    'src-test-weak-4',
    'acct-test-1',
    'primary_contact',
    'third_party_scrape',
    'scraped_profile',
    'Scraped record 4 lists the primary contact as unknown.',
    '2026-04-30T08:45:00Z',
    '{"source_id":"src-test-weak-4","source_asof":"2026-04-30T08:45:00Z","lifecycle_state":"active","evidence_weight":0.2,"scoring_class":"weak_scraped_reference"}',
    '2026-04-30T08:45:00Z'
),
(
    'src-test-weak-5',
    'acct-test-1',
    'primary_contact',
    'third_party_scrape',
    'scraped_profile',
    'Scraped record 5 lists the primary contact as unknown.',
    '2026-04-30T09:00:00Z',
    '{"source_id":"src-test-weak-5","source_asof":"2026-04-30T09:00:00Z","lifecycle_state":"active","evidence_weight":0.2,"scoring_class":"weak_scraped_reference"}',
    '2026-04-30T09:00:00Z'
);

INSERT INTO signal_weights (
    source, entity_type, signal_type, alpha, beta, update_count, updated_at
) VALUES
    ('user_authored', 'account', 'source_reliability', 1.0, 0.0, 1, '2026-05-01T12:00:00Z'),
    ('third_party_scrape', 'account', 'source_reliability', 0.2, 0.8, 5, '2026-05-01T12:00:00Z'),
    ('provider_completion', 'account', 'enrichment_quality', 0.2, 0.8, 1, '2026-05-01T12:00:00Z');

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text,
    dedup_key, item_hash, actor, data_source, source_ref, source_asof,
    observed_at, created_at, provenance_json, metadata_json,
    claim_state, surfacing_state, trust_score, trust_computed_at,
    trust_version, temporal_scope, sensitivity, verification_state
) VALUES
(
    'claim-test-strong-primary-contact-jane',
    '{"kind":"account","id":"acct-test-1"}',
    'entity_summary',
    'primaryContact',
    'acct-test-1:primary_contact',
    'acme.example.com primary contact is jane.doe@example.com.',
    'acct-test-1|entity_summary|primaryContact|jane.doe@example.com|src-test-strong',
    'sha256:strong-primary-contact-jane-doe',
    'system:fixture',
    'user_authored',
    '{"source_id":"src-test-strong","source_system":"user_authored","lifecycle_state":"active","evidence_weight":1.0}',
    '2026-04-29T12:00:00Z',
    '2026-04-29T12:00:00Z',
    '2026-04-29T12:00:00Z',
    '{"sources":["src-test-strong"],"source_asof":"2026-04-29T12:00:00Z","lifecycle_state":"active","evidence_weight":1.0}',
    '{"fixture_bundle":6,"ground_truth":true,"source_lifecycle_state":"active","evidence_weight":1.0}',
    'active',
    'active',
    0.88,
    '2026-05-01T12:00:00Z',
    1,
    'state',
    'internal',
    'active'
),
(
    'claim-test-weak-primary-contact-unknown-1',
    '{"kind":"account","id":"acct-test-1"}',
    'entity_summary',
    'primaryContact',
    'acct-test-1:primary_contact',
    'acme.example.com primary contact is unknown.',
    'acct-test-1|entity_summary|primaryContact|unknown|src-test-weak-1',
    'sha256:weak-primary-contact-unknown-1',
    'system:fixture',
    'third_party_scrape',
    '{"source_id":"src-test-weak-1","source_system":"third_party_scrape","lifecycle_state":"active","evidence_weight":0.2}',
    '2026-04-30T08:00:00Z',
    '2026-04-30T08:00:00Z',
    '2026-04-30T08:00:00Z',
    '{"sources":["src-test-weak-1"],"source_asof":"2026-04-30T08:00:00Z","lifecycle_state":"active","evidence_weight":0.2}',
    '{"fixture_bundle":6,"adversarial_corroborator":true,"source_lifecycle_state":"active","evidence_weight":0.2}',
    'active',
    'active',
    0.24,
    '2026-05-01T12:00:00Z',
    1,
    'state',
    'internal',
    'contested'
),
(
    'claim-test-weak-primary-contact-unknown-2',
    '{"kind":"account","id":"acct-test-1"}',
    'entity_summary',
    'primaryContact',
    'acct-test-1:primary_contact',
    'acme.example.com primary contact is unknown.',
    'acct-test-1|entity_summary|primaryContact|unknown|src-test-weak-2',
    'sha256:weak-primary-contact-unknown-2',
    'system:fixture',
    'third_party_scrape',
    '{"source_id":"src-test-weak-2","source_system":"third_party_scrape","lifecycle_state":"active","evidence_weight":0.2}',
    '2026-04-30T08:15:00Z',
    '2026-04-30T08:15:00Z',
    '2026-04-30T08:15:00Z',
    '{"sources":["src-test-weak-2"],"source_asof":"2026-04-30T08:15:00Z","lifecycle_state":"active","evidence_weight":0.2}',
    '{"fixture_bundle":6,"adversarial_corroborator":true,"source_lifecycle_state":"active","evidence_weight":0.2}',
    'active',
    'active',
    0.24,
    '2026-05-01T12:00:00Z',
    1,
    'state',
    'internal',
    'contested'
),
(
    'claim-test-weak-primary-contact-unknown-3',
    '{"kind":"account","id":"acct-test-1"}',
    'entity_summary',
    'primaryContact',
    'acct-test-1:primary_contact',
    'acme.example.com primary contact is unknown.',
    'acct-test-1|entity_summary|primaryContact|unknown|src-test-weak-3',
    'sha256:weak-primary-contact-unknown-3',
    'system:fixture',
    'third_party_scrape',
    '{"source_id":"src-test-weak-3","source_system":"third_party_scrape","lifecycle_state":"active","evidence_weight":0.2}',
    '2026-04-30T08:30:00Z',
    '2026-04-30T08:30:00Z',
    '2026-04-30T08:30:00Z',
    '{"sources":["src-test-weak-3"],"source_asof":"2026-04-30T08:30:00Z","lifecycle_state":"active","evidence_weight":0.2}',
    '{"fixture_bundle":6,"adversarial_corroborator":true,"source_lifecycle_state":"active","evidence_weight":0.2}',
    'active',
    'active',
    0.24,
    '2026-05-01T12:00:00Z',
    1,
    'state',
    'internal',
    'contested'
),
(
    'claim-test-weak-primary-contact-unknown-4',
    '{"kind":"account","id":"acct-test-1"}',
    'entity_summary',
    'primaryContact',
    'acct-test-1:primary_contact',
    'acme.example.com primary contact is unknown.',
    'acct-test-1|entity_summary|primaryContact|unknown|src-test-weak-4',
    'sha256:weak-primary-contact-unknown-4',
    'system:fixture',
    'third_party_scrape',
    '{"source_id":"src-test-weak-4","source_system":"third_party_scrape","lifecycle_state":"active","evidence_weight":0.2}',
    '2026-04-30T08:45:00Z',
    '2026-04-30T08:45:00Z',
    '2026-04-30T08:45:00Z',
    '{"sources":["src-test-weak-4"],"source_asof":"2026-04-30T08:45:00Z","lifecycle_state":"active","evidence_weight":0.2}',
    '{"fixture_bundle":6,"adversarial_corroborator":true,"source_lifecycle_state":"active","evidence_weight":0.2}',
    'active',
    'active',
    0.24,
    '2026-05-01T12:00:00Z',
    1,
    'state',
    'internal',
    'contested'
),
(
    'claim-test-weak-primary-contact-unknown-5',
    '{"kind":"account","id":"acct-test-1"}',
    'entity_summary',
    'primaryContact',
    'acct-test-1:primary_contact',
    'acme.example.com primary contact is unknown.',
    'acct-test-1|entity_summary|primaryContact|unknown|src-test-weak-5',
    'sha256:weak-primary-contact-unknown-5',
    'system:fixture',
    'third_party_scrape',
    '{"source_id":"src-test-weak-5","source_system":"third_party_scrape","lifecycle_state":"active","evidence_weight":0.2}',
    '2026-04-30T09:00:00Z',
    '2026-04-30T09:00:00Z',
    '2026-04-30T09:00:00Z',
    '{"sources":["src-test-weak-5"],"source_asof":"2026-04-30T09:00:00Z","lifecycle_state":"active","evidence_weight":0.2}',
    '{"fixture_bundle":6,"adversarial_corroborator":true,"source_lifecycle_state":"active","evidence_weight":0.2}',
    'active',
    'active',
    0.24,
    '2026-05-01T12:00:00Z',
    1,
    'state',
    'internal',
    'contested'
);

INSERT INTO claim_contradictions (
    id, primary_claim_id, contradicting_claim_id, branch_kind,
    detected_at, reconciliation_note
) VALUES
    ('contradiction-test-weak-1-vs-strong', 'claim-test-weak-primary-contact-unknown-1', 'claim-test-strong-primary-contact-jane', 'contradiction', '2026-05-01T12:00:00Z', 'Weak scraped primary-contact unknown claim contradicts user-authored primary contact.'),
    ('contradiction-test-weak-2-vs-strong', 'claim-test-weak-primary-contact-unknown-2', 'claim-test-strong-primary-contact-jane', 'contradiction', '2026-05-01T12:00:00Z', 'Weak scraped primary-contact unknown claim contradicts user-authored primary contact.'),
    ('contradiction-test-weak-3-vs-strong', 'claim-test-weak-primary-contact-unknown-3', 'claim-test-strong-primary-contact-jane', 'contradiction', '2026-05-01T12:00:00Z', 'Weak scraped primary-contact unknown claim contradicts user-authored primary contact.'),
    ('contradiction-test-weak-4-vs-strong', 'claim-test-weak-primary-contact-unknown-4', 'claim-test-strong-primary-contact-jane', 'contradiction', '2026-05-01T12:00:00Z', 'Weak scraped primary-contact unknown claim contradicts user-authored primary contact.'),
    ('contradiction-test-weak-5-vs-strong', 'claim-test-weak-primary-contact-unknown-5', 'claim-test-strong-primary-contact-jane', 'contradiction', '2026-05-01T12:00:00Z', 'Weak scraped primary-contact unknown claim contradicts user-authored primary contact.');

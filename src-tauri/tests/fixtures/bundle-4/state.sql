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

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT DEFAULT 'active',
    milestone TEXT,
    owner TEXT,
    target_date TEXT,
    tracker_path TEXT,
    parent_id TEXT,
    updated_at TEXT NOT NULL,
    archived INTEGER DEFAULT 0,
    keywords TEXT,
    keywords_extracted_at TEXT,
    metadata TEXT DEFAULT '{}',
    description TEXT,
    milestones TEXT,
    notes TEXT,
    claim_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS meetings (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    meeting_type TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT,
    attendees TEXT,
    notes_path TEXT,
    description TEXT,
    created_at TEXT NOT NULL,
    calendar_event_id TEXT
);

CREATE TABLE IF NOT EXISTS meeting_transcripts (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    summary TEXT,
    transcript_path TEXT,
    transcript_processed_at TEXT,
    intelligence_state TEXT NOT NULL DEFAULT 'detected',
    intelligence_quality TEXT NOT NULL DEFAULT 'sparse',
    last_enriched_at TEXT,
    signal_count INTEGER NOT NULL DEFAULT 0,
    has_new_signals INTEGER NOT NULL DEFAULT 0,
    last_viewed_at TEXT,
    record_path TEXT
);

CREATE TABLE IF NOT EXISTS meeting_entities (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL DEFAULT 'account',
    confidence REAL NOT NULL DEFAULT 0.95,
    is_primary INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (meeting_id, entity_id)
);

CREATE TABLE IF NOT EXISTS meeting_attendees (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    PRIMARY KEY (meeting_id, person_id)
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
CREATE INDEX IF NOT EXISTS idx_meeting_entities_entity
    ON meeting_entities(entity_id);
CREATE INDEX IF NOT EXISTS idx_attendees_person
    ON meeting_attendees(person_id);

INSERT INTO accounts (
    id, name, lifecycle, health, tracker_path, parent_id, updated_at,
    archived, is_internal, account_type, metadata, claim_version
) VALUES
(
    'acct-test-1', 'acme.example.com', 'active', 'green',
    'Accounts/acme.example.com', 'acct-test-shared-parent',
    '2026-05-01T12:00:00Z', 0, 0, 'customer',
    '{"fixture_account_label":"Account A","aliases":["Account A"]}', 0
),
(
    'acct-test-2', 'subsidiary.example.com', 'active', 'green',
    'Accounts/subsidiary.example.com', 'acct-test-shared-parent',
    '2026-05-01T12:00:00Z', 0, 0, 'customer',
    '{"fixture_account_label":"Account B","aliases":["proj-test-b-1","proj-test-b-2"]}', 0
);

INSERT INTO account_domains (account_id, domain, source)
VALUES
    ('acct-test-1', 'acme.example.com', 'enrichment'),
    ('acct-test-2', 'subsidiary.example.com', 'enrichment');

INSERT INTO people (
    id, email, name, organization, role, relationship,
    first_seen, last_seen, meeting_count, updated_at, archived, claim_version
) VALUES (
    'person-test-shared', 'jane.doe@example.com', 'Jane Doe',
    'example.com', 'Shared stakeholder', 'external',
    '2026-04-20T12:00:00Z', '2026-05-01T12:00:00Z', 2,
    '2026-05-01T12:00:00Z', 0, 0
);

INSERT INTO account_stakeholders (
    account_id, person_id, role, relationship_type, created_at
) VALUES
    ('acct-test-1', 'person-test-shared', 'champion', 'stakeholder', '2026-04-24T12:00:00Z'),
    ('acct-test-2', 'person-test-shared', 'sponsor', 'stakeholder', '2026-04-25T12:00:00Z');

INSERT INTO projects (
    id, name, status, tracker_path, updated_at, archived,
    keywords, metadata, claim_version
) VALUES
    (
        'proj-test-a-1', 'proj-test-a-1', 'active',
        'Projects/proj-test-a-1', '2026-04-24T12:00:00Z', 0,
        '["acct-test-1","acme.example.com"]',
        '{"account_id":"acct-test-1","fixture_account_label":"Account A"}', 0
    ),
    (
        'proj-test-a-2', 'proj-test-a-2', 'active',
        'Projects/proj-test-a-2', '2026-04-24T12:00:00Z', 0,
        '["acct-test-1","acme.example.com"]',
        '{"account_id":"acct-test-1","fixture_account_label":"Account A"}', 0
    ),
    (
        'proj-test-b-1', 'proj-test-b-1', 'active',
        'Projects/proj-test-b-1', '2026-04-25T12:00:00Z', 0,
        '["acct-test-2","subsidiary.example.com"]',
        '{"account_id":"acct-test-2","fixture_account_label":"Account B"}', 0
    ),
    (
        'proj-test-b-2', 'proj-test-b-2', 'active',
        'Projects/proj-test-b-2', '2026-04-25T12:00:00Z', 0,
        '["acct-test-2","subsidiary.example.com"]',
        '{"account_id":"acct-test-2","fixture_account_label":"Account B"}', 0
    );

INSERT INTO meetings (
    id, title, meeting_type, start_time, end_time,
    attendees, notes_path, description, created_at, calendar_event_id
) VALUES
    (
        'mtg-test-b-1',
        'subsidiary.example.com project planning',
        'customer',
        '2026-04-25T15:00:00Z',
        '2026-04-25T15:30:00Z',
        'jane.doe@example.com',
        'Accounts/subsidiary.example.com/Call-Transcripts/mtg-test-b-1.md',
        'Account B meeting where Jane Doe discussed proj-test-b-1.',
        '2026-04-25T16:00:00Z',
        NULL
    ),
    (
        'mtg-test-b-2',
        'subsidiary.example.com launch sequencing',
        'customer',
        '2026-04-26T15:00:00Z',
        '2026-04-26T15:30:00Z',
        'jane.doe@example.com',
        'Accounts/subsidiary.example.com/Call-Transcripts/mtg-test-b-2.md',
        'Account B meeting where Jane Doe discussed proj-test-b-2.',
        '2026-04-26T16:00:00Z',
        NULL
    );

INSERT INTO meeting_transcripts (
    meeting_id, summary, transcript_path, transcript_processed_at,
    intelligence_state, intelligence_quality, last_enriched_at,
    signal_count, has_new_signals, last_viewed_at, record_path
) VALUES
    (
        'mtg-test-b-1',
        'Jane Doe said proj-test-b-1 is the Account B migration priority for subsidiary.example.com.',
        'Accounts/subsidiary.example.com/Call-Transcripts/mtg-test-b-1.md',
        '2026-04-25T16:00:00Z',
        'processed',
        'rich',
        '2026-04-25T16:00:00Z',
        1,
        0,
        NULL,
        'Accounts/subsidiary.example.com/Call-Transcripts/mtg-test-b-1.md'
    ),
    (
        'mtg-test-b-2',
        'Jane Doe said proj-test-b-2 is the Account B launch sequencing priority for subsidiary.example.com.',
        'Accounts/subsidiary.example.com/Call-Transcripts/mtg-test-b-2.md',
        '2026-04-26T16:00:00Z',
        'processed',
        'rich',
        '2026-04-26T16:00:00Z',
        1,
        0,
        NULL,
        'Accounts/subsidiary.example.com/Call-Transcripts/mtg-test-b-2.md'
    );

INSERT INTO meeting_entities (
    meeting_id, entity_id, entity_type, confidence, is_primary
) VALUES
    ('mtg-test-b-1', 'acct-test-2', 'account', 0.98, 1),
    ('mtg-test-b-1', 'proj-test-b-1', 'project', 0.95, 0),
    ('mtg-test-b-2', 'acct-test-2', 'account', 0.98, 1),
    ('mtg-test-b-2', 'proj-test-b-2', 'project', 0.95, 0);

INSERT INTO meeting_attendees (meeting_id, person_id)
VALUES
    ('mtg-test-b-1', 'person-test-shared'),
    ('mtg-test-b-2', 'person-test-shared');

INSERT INTO account_source_refs (
    id, account_id, field, source_system, source_kind, source_value,
    observed_at, source_record_ref, created_at
) VALUES
    (
        'seeded-account-a',
        'acct-test-1',
        'project_context',
        'fixture_seed',
        'fact',
        'Account A project context contains proj-test-a-1 and proj-test-a-2.',
        '2026-04-24T12:00:00Z',
        '{"source_id":"seeded-account-a","source_asof":"2026-04-24T12:00:00Z","lifecycle_state":"active"}',
        '2026-04-24T12:00:00Z'
    ),
    (
        'seeded-account-b',
        'acct-test-2',
        'project_context',
        'fixture_seed',
        'fact',
        'Account B project context contains proj-test-b-1 and proj-test-b-2.',
        '2026-04-25T12:00:00Z',
        '{"source_id":"seeded-account-b","source_asof":"2026-04-25T12:00:00Z","lifecycle_state":"active"}',
        '2026-04-25T12:00:00Z'
    ),
    (
        'seeded-person-shared',
        'acct-test-2',
        'person_context',
        'fixture_seed',
        'meeting_transcript',
        'jane.doe@example.com discussed proj-test-b-1 and proj-test-b-2 in Account B meetings.',
        '2026-04-26T12:00:00Z',
        '{"source_id":"seeded-person-shared","person_id":"person-test-shared","source_asof":"2026-04-26T12:00:00Z","lifecycle_state":"active","origin_account_id":"acct-test-2"}',
        '2026-04-26T12:00:00Z'
    );

INSERT INTO signal_weights (
    source, entity_type, signal_type, alpha, beta, update_count, updated_at
) VALUES (
    'provider_completion', 'account', 'enrichment_quality',
    0.20, 0.80, 1, '2026-05-01T12:00:00Z'
);

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text,
    dedup_key, item_hash, actor, data_source, source_ref, source_asof,
    observed_at, created_at, provenance_json, metadata_json,
    claim_state, surfacing_state, trust_score, trust_computed_at,
    trust_version, temporal_scope, sensitivity, verification_state
) VALUES
    (
        'claim-test-a-proj-1',
        '{"kind":"project","id":"proj-test-a-1"}',
        'project_status',
        'projectContext[0].status',
        'acct-test-1:project:proj-test-a-1',
        'proj-test-a-1 tracks Account A onboarding readiness for acme.example.com.',
        'acct-test-1|project_status|proj-test-a-1|onboarding-readiness',
        'sha256:bundle-4-a-proj-1',
        'system:fixture',
        'fixture_seed',
        '{"source_id":"seeded-account-a","source_system":"fixture_seed","lifecycle_state":"active"}',
        '2026-04-24T12:00:00Z',
        '2026-04-24T12:00:00Z',
        '2026-04-24T12:00:00Z',
        '{"sources":["seeded-account-a"],"source_asof":"2026-04-24T12:00:00Z","lifecycle_state":"active"}',
        '{"fixture_bundle":4,"account_id":"acct-test-1","project_id":"proj-test-a-1","ground_truth":true}',
        'active',
        'active',
        0.91,
        '2026-04-24T12:00:00Z',
        1,
        'state',
        'internal',
        'active'
    ),
    (
        'claim-test-a-proj-2',
        '{"kind":"project","id":"proj-test-a-2"}',
        'project_status',
        'projectContext[1].status',
        'acct-test-1:project:proj-test-a-2',
        'proj-test-a-2 tracks Account A stakeholder enablement for acme.example.com.',
        'acct-test-1|project_status|proj-test-a-2|stakeholder-enablement',
        'sha256:bundle-4-a-proj-2',
        'system:fixture',
        'fixture_seed',
        '{"source_id":"seeded-account-a","source_system":"fixture_seed","lifecycle_state":"active"}',
        '2026-04-24T12:00:00Z',
        '2026-04-24T12:00:00Z',
        '2026-04-24T12:00:00Z',
        '{"sources":["seeded-account-a"],"source_asof":"2026-04-24T12:00:00Z","lifecycle_state":"active"}',
        '{"fixture_bundle":4,"account_id":"acct-test-1","project_id":"proj-test-a-2","ground_truth":true}',
        'active',
        'active',
        0.90,
        '2026-04-24T12:00:00Z',
        1,
        'state',
        'internal',
        'active'
    ),
    (
        'claim-test-b-proj-1',
        '{"kind":"project","id":"proj-test-b-1"}',
        'project_status',
        'projectContext[0].status',
        'acct-test-2:project:proj-test-b-1',
        'proj-test-b-1 tracks Account B data migration readiness for subsidiary.example.com.',
        'acct-test-2|project_status|proj-test-b-1|data-migration-readiness',
        'sha256:bundle-4-b-proj-1',
        'system:fixture',
        'fixture_seed',
        '{"source_id":"seeded-account-b","source_system":"fixture_seed","lifecycle_state":"active"}',
        '2026-04-25T12:00:00Z',
        '2026-04-25T12:00:00Z',
        '2026-04-25T12:00:00Z',
        '{"sources":["seeded-account-b"],"source_asof":"2026-04-25T12:00:00Z","lifecycle_state":"active"}',
        '{"fixture_bundle":4,"account_id":"acct-test-2","project_id":"proj-test-b-1","ground_truth":true}',
        'active',
        'active',
        0.92,
        '2026-04-25T12:00:00Z',
        1,
        'state',
        'internal',
        'active'
    ),
    (
        'claim-test-b-proj-2',
        '{"kind":"project","id":"proj-test-b-2"}',
        'project_status',
        'projectContext[1].status',
        'acct-test-2:project:proj-test-b-2',
        'proj-test-b-2 tracks Account B launch sequencing for subsidiary.example.com.',
        'acct-test-2|project_status|proj-test-b-2|launch-sequencing',
        'sha256:bundle-4-b-proj-2',
        'system:fixture',
        'fixture_seed',
        '{"source_id":"seeded-account-b","source_system":"fixture_seed","lifecycle_state":"active"}',
        '2026-04-25T12:00:00Z',
        '2026-04-25T12:00:00Z',
        '2026-04-25T12:00:00Z',
        '{"sources":["seeded-account-b"],"source_asof":"2026-04-25T12:00:00Z","lifecycle_state":"active"}',
        '{"fixture_bundle":4,"account_id":"acct-test-2","project_id":"proj-test-b-2","ground_truth":true}',
        'active',
        'active',
        0.91,
        '2026-04-25T12:00:00Z',
        1,
        'state',
        'internal',
        'active'
    );


CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    lifecycle TEXT,
    arr REAL,
    health TEXT,
    contract_start TEXT,
    contract_end TEXT,
    nps INTEGER,
    tracker_path TEXT,
    parent_id TEXT,
    is_internal INTEGER NOT NULL DEFAULT 0,
    account_type TEXT NOT NULL DEFAULT 'customer',
    updated_at TEXT NOT NULL,
    archived INTEGER NOT NULL DEFAULT 0,
    keywords TEXT,
    keywords_extracted_at TEXT,
    metadata TEXT,
    commercial_stage TEXT,
    arr_range_low REAL,
    arr_range_high REAL,
    renewal_likelihood REAL,
    renewal_likelihood_source TEXT,
    renewal_likelihood_updated_at TEXT,
    renewal_model TEXT,
    renewal_pricing_method TEXT,
    support_tier TEXT,
    support_tier_source TEXT,
    support_tier_updated_at TEXT,
    active_subscription_count INTEGER,
    growth_potential_score REAL,
    growth_potential_score_source TEXT,
    icp_fit_score REAL,
    icp_fit_score_source TEXT,
    primary_product TEXT,
    customer_status TEXT,
    customer_status_source TEXT,
    customer_status_updated_at TEXT,
    company_overview TEXT,
    strategic_programs TEXT,
    notes TEXT,
    user_health_sentiment TEXT,
    sentiment_set_at TEXT,
    claim_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE account_domains (
    account_id TEXT NOT NULL,
    domain TEXT NOT NULL,
    source TEXT,
    PRIMARY KEY (account_id, domain)
);

CREATE TABLE people (
    id TEXT PRIMARY KEY,
    email TEXT,
    name TEXT NOT NULL,
    organization TEXT,
    role TEXT,
    relationship TEXT NOT NULL DEFAULT 'customer',
    notes TEXT,
    tracker_path TEXT,
    last_seen TEXT,
    first_seen TEXT,
    meeting_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    archived INTEGER NOT NULL DEFAULT 0,
    linkedin_url TEXT,
    twitter_handle TEXT,
    phone TEXT,
    photo_url TEXT,
    bio TEXT,
    title_history TEXT,
    company_industry TEXT,
    company_size TEXT,
    company_hq TEXT,
    last_enriched_at TEXT,
    enrichment_sources TEXT,
    claim_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE account_stakeholders (
    account_id TEXT NOT NULL,
    person_id TEXT NOT NULL,
    engagement TEXT,
    data_source_engagement TEXT,
    assessment TEXT,
    data_source_assessment TEXT,
    data_source TEXT DEFAULT 'user',
    status TEXT NOT NULL DEFAULT 'active',
    confidence REAL,
    last_seen_in_glean TEXT,
    created_at TEXT,
    PRIMARY KEY (account_id, person_id)
);

CREATE TABLE entity_context_entries (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    embedding BLOB,
    created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    updated_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
);

CREATE INDEX idx_entity_context_entity
    ON entity_context_entries (entity_type, entity_id);

CREATE TABLE meetings (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    meeting_type TEXT NOT NULL DEFAULT 'external',
    start_time TEXT NOT NULL,
    end_time TEXT,
    attendees TEXT,
    created_at TEXT NOT NULL,
    calendar_event_id TEXT,
    description TEXT
);

CREATE TABLE meeting_attendees (
    meeting_id TEXT NOT NULL,
    person_id TEXT NOT NULL,
    PRIMARY KEY (meeting_id, person_id)
);

CREATE TABLE entities (
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    tracker_path TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (id, entity_type)
);

CREATE TABLE meeting_entities (
    meeting_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    confidence REAL DEFAULT 0.95,
    is_primary INTEGER DEFAULT 1,
    PRIMARY KEY (meeting_id, entity_id, entity_type)
);

CREATE TABLE intelligence_claims (
    id TEXT PRIMARY KEY, subject_ref TEXT NOT NULL, claim_type TEXT NOT NULL, field_path TEXT,
    topic_key TEXT, text TEXT NOT NULL, dedup_key TEXT NOT NULL, item_hash TEXT, actor TEXT NOT NULL,
    data_source TEXT NOT NULL, source_ref TEXT, source_asof TEXT, observed_at TEXT NOT NULL,
    created_at TEXT NOT NULL, provenance_json TEXT NOT NULL, metadata_json TEXT,
    claim_state TEXT NOT NULL DEFAULT 'active', surfacing_state TEXT NOT NULL DEFAULT 'active',
    demotion_reason TEXT, reactivated_at TEXT, retraction_reason TEXT, expires_at TEXT,
    superseded_by TEXT, trust_score REAL, trust_computed_at TEXT, trust_version INTEGER,
    thread_id TEXT, temporal_scope TEXT NOT NULL DEFAULT 'state', sensitivity TEXT NOT NULL DEFAULT 'internal',
    verification_state TEXT NOT NULL DEFAULT 'active', verification_reason TEXT, needs_user_decision_at TEXT
);

CREATE TABLE claim_corroborations (
    id TEXT PRIMARY KEY, claim_id TEXT NOT NULL, data_source TEXT NOT NULL, source_asof TEXT,
    source_mechanism TEXT, strength REAL NOT NULL DEFAULT 0.5, reinforcement_count INTEGER NOT NULL DEFAULT 1,
    last_reinforced_at TEXT NOT NULL, created_at TEXT NOT NULL
);

CREATE TABLE claim_feedback (
    id TEXT PRIMARY KEY,
    claim_id TEXT NOT NULL,
    feedback_type TEXT NOT NULL,
    actor TEXT NOT NULL,
    actor_id TEXT,
    payload_json TEXT,
    submitted_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    applied_at TEXT
);

INSERT INTO accounts (id, name, account_type, updated_at, archived)
VALUES ('dos287-example-parent', 'Example Portfolio', 'customer', '2026-05-04T12:00:00Z', 0);

INSERT INTO accounts (id, name, parent_id, account_type, updated_at, archived)
VALUES ('dos287-target-example', 'Target Example', 'dos287-example-parent', 'customer', '2026-05-04T12:00:00Z', 0);

INSERT INTO accounts (id, name, parent_id, account_type, updated_at, archived)
VALUES ('dos287-adjacent-example', 'Adjacent Example', 'dos287-example-parent', 'customer', '2026-05-04T12:00:00Z', 0);

INSERT INTO accounts (id, name, parent_id, account_type, updated_at, archived)
VALUES ('dos287-sibling-example', 'Sibling Example', 'dos287-example-parent', 'customer', '2026-05-04T12:00:00Z', 0);

INSERT INTO account_domains (account_id, domain, source)
VALUES ('dos287-target-example', 'target.example.com', 'test');

INSERT INTO account_domains (account_id, domain, source)
VALUES ('dos287-target-example', 'subsidiary.example.com', 'test');

INSERT INTO account_domains (account_id, domain, source)
VALUES ('dos287-adjacent-example', 'adjacent.example.com', 'test');

INSERT INTO account_domains (account_id, domain, source)
VALUES ('dos287-adjacent-example', 'cluster-1.example.com', 'test');

INSERT INTO account_domains (account_id, domain, source)
VALUES ('dos287-sibling-example', 'subsidiary.example.com', 'test');

INSERT INTO account_domains (account_id, domain, source)
VALUES ('dos287-example-parent', 'parent.example.com', 'test');

INSERT INTO people (id, email, name, relationship, updated_at, archived)
VALUES ('person-dos287-alice', 'alice@target.example.com', 'Alice Adams', 'external', '2026-05-04T12:00:00Z', 0);

INSERT INTO people (id, email, name, relationship, updated_at, archived)
VALUES ('person-dos287-blake', 'blake@adjacent.example.com', 'Blake Branch', 'external', '2026-05-04T12:00:00Z', 0);

INSERT INTO people (id, email, name, relationship, updated_at, archived)
VALUES ('person-dos287-parent-alex', 'alex@parent.example.com', 'Alex Avery', 'external', '2026-05-04T12:00:00Z', 0);

INSERT INTO people (id, email, name, relationship, updated_at, archived)
VALUES ('person-dos287-sibling-alex', 'alex@subsidiary.example.com', 'Alex Stone', 'external', '2026-05-04T12:00:00Z', 0);

INSERT INTO account_stakeholders
    (account_id, person_id, engagement, data_source_engagement, assessment,
     data_source_assessment, data_source, status, created_at)
VALUES
    ('dos287-target-example', 'person-dos287-alice', 'trusted champion', 'user',
     'Alice owns the Target Example rollout.', 'user', 'user', 'active',
     '2026-05-04T12:00:00Z');

INSERT INTO account_stakeholders
    (account_id, person_id, engagement, data_source_engagement, assessment,
     data_source_assessment, data_source, status, created_at)
VALUES
    ('dos287-adjacent-example', 'person-dos287-blake', 'blocked', 'user',
     'Blake owns the Adjacent Example cluster migration.', 'user', 'user',
     'active', '2026-05-04T12:00:00Z');

INSERT INTO account_stakeholders
    (account_id, person_id, engagement, data_source_engagement, assessment,
     data_source_assessment, data_source, status, created_at)
VALUES
    ('dos287-sibling-example', 'person-dos287-sibling-alex', 'supporting sponsor', 'user',
     'Alex Stone owns the Sibling Example rollout checkpoint.', 'user', 'user',
     'active', '2026-05-04T12:00:00Z');

INSERT INTO entity_context_entries
    (id, entity_type, entity_id, title, content, created_at, updated_at)
VALUES
    ('ctx-target-newer', 'account', 'dos287-target-example', 'Renewal owner',
     'Alice Adams owns the Target Example rollout and renewal plan.',
     '2026-05-05T09:00:00Z', '2026-05-05T10:00:00Z');

INSERT INTO entity_context_entries
    (id, entity_type, entity_id, title, content, created_at, updated_at)
VALUES
    ('ctx-target-older', 'account', 'dos287-target-example', 'Boundary note',
     'Do not merge Adjacent Example infrastructure risks into Target Example.',
     '2026-05-04T09:00:00Z', '2026-05-04T10:00:00Z');

INSERT INTO entity_context_entries
    (id, entity_type, entity_id, title, content, created_at, updated_at)
VALUES
    ('ctx-foreign', 'account', 'dos287-adjacent-example', 'Foreign risk',
     'Blake Branch owns cluster-1.example.com migration risk for Adjacent Example.',
     '2026-05-06T09:00:00Z', '2026-05-06T10:00:00Z');

INSERT INTO entities (id, name, entity_type, tracker_path, updated_at)
VALUES
    ('dos287-example-parent', 'Example Portfolio', 'account', NULL, '2026-05-04T12:00:00Z'),
    ('dos287-target-example', 'Target Example', 'account', NULL, '2026-05-04T12:00:00Z'),
    ('dos287-sibling-example', 'Sibling Example', 'account', NULL, '2026-05-04T12:00:00Z'),
    ('dos287-adjacent-example', 'Adjacent Example', 'account', NULL, '2026-05-04T12:00:00Z');

INSERT INTO meetings (id, title, meeting_type, start_time, end_time, attendees, created_at, description)
VALUES (
    'meeting-b1-parent-renewal',
    'Example Portfolio renewal sync',
    'external',
    '2026-05-07T16:00:00Z',
    '2026-05-07T16:30:00Z',
    '["alex@parent.example.com","alice@target.example.com","alex@subsidiary.example.com"]',
    '2026-05-05T16:00:00Z',
    'Parent-account renewal planning call that legitimately includes Target Example and its same-domain sibling.'
);

INSERT INTO meeting_attendees (meeting_id, person_id)
VALUES
    ('meeting-b1-parent-renewal', 'person-dos287-parent-alex'),
    ('meeting-b1-parent-renewal', 'person-dos287-alice'),
    ('meeting-b1-parent-renewal', 'person-dos287-sibling-alex');

INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
VALUES
    ('meeting-b1-parent-renewal', 'dos287-example-parent', 'account', 0.99, 1),
    ('meeting-b1-parent-renewal', 'dos287-target-example', 'account', 0.88, 0),
    ('meeting-b1-parent-renewal', 'dos287-sibling-example', 'account', 0.84, 0);

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, dedup_key, item_hash, actor,
    data_source, source_ref, source_asof, observed_at, created_at, provenance_json, metadata_json,
    claim_state, surfacing_state, demotion_reason, reactivated_at, retraction_reason, expires_at,
    superseded_by, trust_score, trust_computed_at, trust_version, thread_id, temporal_scope, sensitivity,
    verification_state, verification_reason, needs_user_decision_at
)
VALUES
    ('claim-b1-renewal-owner-canonical', '{"id":"dos287-target-example","kind":"account"}', 'entity_current_state', 'renewal.owner', 'target-renewal-owner', 'Alice Adams owns the Target Example renewal plan.', 'dedup-b1-target-renewal-owner', 'hash-b1-target-renewal-owner', 'agent:fixture', 'user', 'ctx-target-newer', '2026-05-05T10:00:00Z', '2026-05-05T10:00:00Z', '2026-05-05T10:00:00Z', '{"sources":["ctx-target-newer"],"fixture_bundle":1}', '{"scenario":"six_paraphrases_collapsed","expected_trust_band":"likely_current","render_policy":"dos411_pending_legacy_read"}', 'active', 'dormant', 'dos411_pending_legacy_read', NULL, NULL, NULL, NULL, 0.91, '2026-05-06T12:00:00Z', 1, 'thread-b1-renewal-owner', 'state', 'internal', 'active', NULL, NULL),
    ('claim-b1-use-caution-support-stale', '{"id":"dos287-target-example","kind":"account"}', 'entity_risk', 'support.risk', 'target-support-stale', 'Support risk for Target Example is based on an older transcript and needs review before quoting.', 'dedup-b1-target-support-stale', 'hash-b1-target-support-stale', 'agent:fixture', 'transcript', '{"meeting_id":"meeting-b1-parent-renewal","source_id":"transcript-b1-support-risk"}', '2026-04-20T17:00:00Z', '2026-04-20T17:00:00Z', '2026-04-20T17:00:00Z', '{"sources":["transcript-b1-support-risk"],"fixture_bundle":1}', '{"scenario":"trust_band_diversity","expected_trust_band":"use_with_caution","render_policy":"dos411_pending_legacy_read"}', 'active', 'dormant', 'dos411_pending_legacy_read', NULL, NULL, NULL, NULL, 0.62, '2026-05-06T12:00:00Z', 1, 'thread-b1-trust-bands', 'state', 'internal', 'active', NULL, NULL),
    ('claim-b1-needs-verification-opaque', '{"id":"dos287-target-example","kind":"account"}', 'entity_risk', 'renewal.risk', 'target-opaque-renewal-risk', 'An opaque imported note claims Target Example renewal risk increased, but the source timestamp is incomplete.', 'dedup-b1-target-opaque-risk', 'hash-b1-target-opaque-risk', 'agent:fixture', 'glean', '{"source_id":"glean-b1-opaque-renewal","lifecycle_state":"active"}', NULL, '2026-05-02T11:00:00Z', '2026-05-02T11:00:00Z', '{"sources":["glean-b1-opaque-renewal"],"fixture_bundle":1,"source_timestamp":"unknown"}', '{"scenario":"trust_band_diversity","expected_trust_band":"needs_verification","render_policy":"dos411_pending_legacy_read"}', 'active', 'dormant', 'dos411_pending_legacy_read', NULL, NULL, NULL, NULL, 0.31, '2026-05-06T12:00:00Z', 1, 'thread-b1-trust-bands', 'state', 'internal', 'needs_user_decision', 'source_timestamp_unknown', '2026-05-06T12:00:00Z'),
    ('claim-b1-wrong-parent-owner', '{"id":"dos287-example-parent","kind":"account"}', 'entity_current_state', 'renewal.owner', 'wrong-parent-renewal-owner', 'Alice Adams owns the Target Example renewal plan.', 'dedup-b1-wrong-parent-owner', 'hash-b1-target-renewal-owner', 'agent:fixture', 'email', '{"email_id":"email-b1-wrong-parent-owner","meeting_id":"meeting-b1-parent-renewal"}', '2026-05-05T16:30:00Z', '2026-05-05T16:30:00Z', '2026-05-05T16:30:00Z', '{"sources":["email-b1-wrong-parent-owner"],"fixture_bundle":1}', '{"scenario":"wrong_subject_tombstone","corrected_to":{"kind":"account","id":"dos287-target-example"}}', 'tombstoned', 'dormant', 'wrong_subject_feedback', NULL, 'wrong_subject', NULL, NULL, 0.18, '2026-05-06T12:00:00Z', 1, 'thread-b1-renewal-owner', 'state', 'internal', 'contested', 'wrong_subject', NULL);

INSERT INTO claim_corroborations (
    id, claim_id, data_source, source_asof, source_mechanism,
    strength, reinforcement_count, last_reinforced_at, created_at
)
VALUES
    ('corro-b1-renewal-owner-p1', 'claim-b1-renewal-owner-canonical', 'email', '2026-05-05T09:30:00Z', 'paraphrase_1_target_owner', 0.82, 1, '2026-05-05T09:30:00Z', '2026-05-05T09:30:00Z'),
    ('corro-b1-renewal-owner-p2', 'claim-b1-renewal-owner-canonical', 'transcript', '2026-05-05T09:45:00Z', 'paraphrase_2_target_owner', 0.78, 1, '2026-05-05T09:45:00Z', '2026-05-05T09:45:00Z'),
    ('corro-b1-renewal-owner-p3', 'claim-b1-renewal-owner-canonical', 'glean', '2026-05-05T10:00:00Z', 'paraphrase_3_target_owner', 0.74, 1, '2026-05-05T10:00:00Z', '2026-05-05T10:00:00Z'),
    ('corro-b1-renewal-owner-p4', 'claim-b1-renewal-owner-canonical', 'user', '2026-05-05T10:15:00Z', 'paraphrase_4_target_owner', 0.86, 1, '2026-05-05T10:15:00Z', '2026-05-05T10:15:00Z'),
    ('corro-b1-renewal-owner-p5', 'claim-b1-renewal-owner-canonical', 'email', '2026-05-05T10:30:00Z', 'paraphrase_5_target_owner', 0.80, 1, '2026-05-05T10:30:00Z', '2026-05-05T10:30:00Z'),
    ('corro-b1-renewal-owner-p6', 'claim-b1-renewal-owner-canonical', 'transcript', '2026-05-05T10:45:00Z', 'paraphrase_6_target_owner', 0.79, 1, '2026-05-05T10:45:00Z', '2026-05-05T10:45:00Z');

INSERT INTO claim_feedback (
    id, claim_id, feedback_type, actor, actor_id, payload_json, submitted_at, applied_at
)
VALUES (
    'feedback-b1-wrong-subject-parent',
    'claim-b1-wrong-parent-owner',
    'wrong_subject',
    'user',
    NULL,
    '{"reason":"claim_belongs_to_target_not_parent","corrected_subject":{"kind":"account","id":"dos287-target-example"}}',
    '2026-05-06T09:00:00Z',
    '2026-05-06T09:00:00Z'
);

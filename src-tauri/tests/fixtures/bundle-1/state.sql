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

INSERT INTO accounts (id, name, account_type, updated_at, archived)
VALUES ('dos287-example-parent', 'Example Portfolio', 'customer', '2026-05-04T12:00:00Z', 0);

INSERT INTO accounts (id, name, parent_id, account_type, updated_at, archived)
VALUES ('dos287-target-example', 'Target Example', 'dos287-example-parent', 'customer', '2026-05-04T12:00:00Z', 0);

INSERT INTO accounts (id, name, parent_id, account_type, updated_at, archived)
VALUES ('dos287-adjacent-example', 'Adjacent Example', 'dos287-example-parent', 'customer', '2026-05-04T12:00:00Z', 0);

INSERT INTO account_domains (account_id, domain, source)
VALUES ('dos287-target-example', 'target.example.com', 'test');

INSERT INTO account_domains (account_id, domain, source)
VALUES ('dos287-adjacent-example', 'adjacent.example.com', 'test');

INSERT INTO account_domains (account_id, domain, source)
VALUES ('dos287-adjacent-example', 'cluster-1.example.com', 'test');

INSERT INTO people (id, email, name, relationship, updated_at, archived)
VALUES ('person-dos287-alice', 'alice@target.example.com', 'Alice Adams', 'external', '2026-05-04T12:00:00Z', 0);

INSERT INTO people (id, email, name, relationship, updated_at, archived)
VALUES ('person-dos287-blake', 'blake@adjacent.example.com', 'Blake Branch', 'external', '2026-05-04T12:00:00Z', 0);

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

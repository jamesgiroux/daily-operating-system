-- W6-A-meta-3: get_daily_readiness / happy / typical day.
-- Minimum substrate: one upcoming meeting, one linked account, and one current claim.
CREATE TABLE IF NOT EXISTS meetings (
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

CREATE TABLE IF NOT EXISTS entities (
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    tracker_path TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (id, entity_type)
);

CREATE TABLE IF NOT EXISTS meeting_entities (
    meeting_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    confidence REAL DEFAULT 0.95,
    is_primary INTEGER DEFAULT 1,
    PRIMARY KEY (meeting_id, entity_id, entity_type)
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

INSERT INTO meetings (id, title, meeting_type, start_time, end_time, attendees, created_at, description)
VALUES (
    'meeting-meta-3-example',
    'Weekly Account Review',
    'external',
    '2026-05-15T15:00:00Z',
    '2026-05-15T15:30:00Z',
    '["contact3@example.com"]',
    '2026-05-15T08:00:00Z',
    'Review renewal readiness.'
);

INSERT INTO entities (id, name, entity_type, tracker_path, updated_at)
VALUES ('account-meta-3-example', 'Account Meta 3 Example', 'account', NULL, '2026-05-15T09:00:00Z');

INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
VALUES ('meeting-meta-3-example', 'account-meta-3-example', 'account', 0.95, 1);

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, actor, data_source,
    source_ref, source_asof, observed_at, provenance_json, trust_score,
    temporal_scope, sensitivity, verification_state
)
VALUES (
    'claim-meta-3-renewal-ready',
    '{"kind":"account","id":"account-meta-3-example"}',
    'current_state',
    'renewal.readiness',
    'account-meta-3-example:renewal',
    'The account is ready for a renewal check-in today.',
    'agent:fixture',
    'calendar',
    '{"source_id":"source-meta-3-calendar"}',
    '2026-05-15T09:00:00Z',
    '2026-05-15T09:05:00Z',
    '{"source_id":"source-meta-3-calendar","field":"renewal.readiness"}',
    0.86,
    'state',
    'internal',
    'active'
);

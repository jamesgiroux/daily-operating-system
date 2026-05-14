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

CREATE TABLE people (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    name TEXT NOT NULL
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

CREATE TABLE subject_ownerships (
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    PRIMARY KEY (subject_kind, subject_id)
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

INSERT INTO meetings (id, title, meeting_type, start_time, end_time, attendees, created_at)
VALUES
    ('meeting-b10-cross-tenant', 'Subsidiary Account Follow-up Review', 'external', '2026-05-07T15:00:00Z', '2026-05-07T15:30:00Z', '["casey@subsidiary.example.com","devon@parent.example.com"]', '2026-05-05T12:00:00Z');

INSERT INTO people (id, email, name)
VALUES
    ('person-b10-cross-casey', 'casey@subsidiary.example.com', 'Casey Chen'),
    ('person-b10-cross-devon', 'devon@parent.example.com', 'Devon Diaz');

INSERT INTO meeting_attendees (meeting_id, person_id)
VALUES
    ('meeting-b10-cross-tenant', 'person-b10-cross-casey'),
    ('meeting-b10-cross-tenant', 'person-b10-cross-devon');

INSERT INTO entities (id, name, entity_type, tracker_path, updated_at)
VALUES
    ('acct-b10-current-subsidiary', 'Current Workspace Subsidiary Example', 'account', NULL, '2026-05-05T12:00:00Z'),
    ('acct-b10-other-workspace', 'Other Workspace Subsidiary Example', 'account', NULL, '2026-05-05T12:00:00Z');

INSERT INTO subject_ownerships (subject_kind, subject_id, workspace_id)
VALUES
    ('account', 'acct-b10-current-subsidiary', 'workspace-current'),
    ('account', 'acct-b10-other-workspace', 'workspace-other');

INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
VALUES
    ('meeting-b10-cross-tenant', 'acct-b10-other-workspace', 'account', 0.95, 1);

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, dedup_key, item_hash, actor,
    data_source, source_ref, source_asof, observed_at, created_at, provenance_json, metadata_json,
    claim_state, surfacing_state, demotion_reason, reactivated_at, retraction_reason, expires_at,
    superseded_by, trust_score, trust_computed_at, trust_version, thread_id, temporal_scope, sensitivity,
    verification_state, verification_reason, needs_user_decision_at
)
VALUES
    ('src-b10-cross-tenant-open-loop', '{"id":"acct-b10-other-workspace","kind":"account"}', 'open_loop', 'summary', NULL, 'Other Workspace Subsidiary Example still needs a written rollout owner before the next review.', 'dedup-src-b10-cross-tenant-open-loop', 'hash-src-b10-cross-tenant-open-loop', 'agent:fixture', 'glean', 'doc-b10-other-workspace-loop', '2026-05-05T12:15:00Z', '2026-05-05T12:15:00Z', '2026-05-05T12:15:00Z', '{}', '{"loop_kind":"follow_up","owner":"Casey Chen","status":"active"}', 'active', 'active', NULL, NULL, NULL, NULL, NULL, 0.91, NULL, NULL, NULL, 'state', 'internal', 'active', NULL, NULL);

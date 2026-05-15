CREATE TABLE source_lifecycle_states (
    source_id TEXT PRIMARY KEY,
    source_class TEXT NOT NULL,
    lifecycle_state TEXT NOT NULL CHECK (
        lifecycle_state IN ('active', 'unavailable', 'revoked', 'expired', 'restricted', 'stale')
    ),
    claim_id TEXT NOT NULL,
    source_asof TEXT,
    observed_at TEXT NOT NULL,
    wrapper_fetch_at TEXT,
    downstream_source_asof TEXT,
    actor_visibility_json TEXT NOT NULL,
    render_policy TEXT NOT NULL,
    redaction_reason TEXT,
    invalidation_signal TEXT
);

CREATE TABLE bundle17_channel_policy (
    channel TEXT PRIMARY KEY,
    revoked_source_rejected INTEGER NOT NULL CHECK (revoked_source_rejected IN (0, 1)),
    restricted_source_rejected INTEGER NOT NULL CHECK (restricted_source_rejected IN (0, 1)),
    internal_only_rejected INTEGER NOT NULL CHECK (internal_only_rejected IN (0, 1)),
    safe_summary_retained INTEGER NOT NULL CHECK (safe_summary_retained IN (0, 1)),
    low_detail_only INTEGER NOT NULL CHECK (low_detail_only IN (0, 1))
);

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
    needs_user_decision_at TEXT,
    claim_version INTEGER NOT NULL DEFAULT 1,
    canonical_status TEXT NOT NULL DEFAULT 'live',
    non_semantic_mergeable BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE claim_corroborations (
    id TEXT PRIMARY KEY,
    claim_id TEXT NOT NULL,
    data_source TEXT NOT NULL,
    source_asof TEXT,
    source_mechanism TEXT,
    strength REAL NOT NULL DEFAULT 0.5,
    reinforcement_count INTEGER NOT NULL DEFAULT 1,
    last_reinforced_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE claim_feedback (
    id TEXT PRIMARY KEY,
    claim_id TEXT NOT NULL,
    feedback_type TEXT NOT NULL,
    actor TEXT NOT NULL,
    actor_id TEXT,
    payload_json TEXT,
    submitted_at TEXT NOT NULL,
    applied_at TEXT
);

INSERT INTO meetings (id, title, meeting_type, start_time, end_time, attendees, created_at)
VALUES (
    'meeting-b17-example',
    'Source Lifecycle Validation',
    'external',
    '2026-05-15T16:00:00Z',
    '2026-05-15T16:30:00Z',
    '["user@example.com"]',
    '2026-05-15T08:00:00Z'
);

INSERT INTO people (id, email, name)
VALUES ('person-b17-example', 'user@example.com', 'Example Contact');

INSERT INTO meeting_attendees (meeting_id, person_id)
VALUES ('meeting-b17-example', 'person-b17-example');

INSERT INTO entities (id, name, entity_type, tracker_path, updated_at)
VALUES ('account-b17-example', 'Account B17 Example', 'account', NULL, '2026-05-15T08:00:00Z');

INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
VALUES ('meeting-b17-example', 'account-b17-example', 'account', 0.95, 1);

INSERT INTO source_lifecycle_states (
    source_id, source_class, lifecycle_state, claim_id, source_asof, observed_at,
    wrapper_fetch_at, downstream_source_asof, actor_visibility_json, render_policy,
    redaction_reason, invalidation_signal
)
VALUES
    (
        'source-b17-active-baseline',
        'google',
        'active',
        'claim-b17-active-baseline',
        '2026-05-15T09:00:00Z',
        '2026-05-15T09:05:00Z',
        NULL,
        NULL,
        '{"tauri_user":"detail_allowed","mcp_agent":"summary_allowed"}',
        'show',
        NULL,
        NULL
    ),
    (
        'source-b17-google-disconnected',
        'google',
        'unavailable',
        'claim-b17-google-disconnected',
        '2026-05-01T13:00:00Z',
        '2026-05-01T13:05:00Z',
        NULL,
        NULL,
        '{"tauri_user":"safe_summary","mcp_agent":"safe_summary"}',
        'degraded_safe_summary',
        'source_unavailable',
        'source_lifecycle_changed'
    ),
    (
        'source-b17-slack-revoked',
        'slack',
        'revoked',
        'claim-b17-slack-revoked',
        '2026-04-20T10:00:00Z',
        '2026-04-20T10:05:00Z',
        NULL,
        NULL,
        '{"tauri_user":"masked","mcp_agent":"masked"}',
        'masked',
        'source_revoked',
        'source_lifecycle_changed'
    ),
    (
        'source-b17-gong-expired',
        'gong',
        'expired',
        'claim-b17-gong-expired',
        '2026-03-15T15:00:00Z',
        '2026-03-15T15:05:00Z',
        NULL,
        NULL,
        '{"tauri_user":"masked","mcp_agent":"masked"}',
        'masked',
        'retention_expired',
        'source_lifecycle_changed'
    ),
    (
        'source-b17-zendesk-revoked',
        'zendesk',
        'revoked',
        'claim-b17-zendesk-revoked',
        '2026-04-25T11:00:00Z',
        '2026-04-25T11:05:00Z',
        NULL,
        NULL,
        '{"tauri_user":"masked","mcp_agent":"masked"}',
        'masked',
        'source_revoked',
        'source_lifecycle_changed'
    ),
    (
        'source-b17-glean-restricted',
        'glean',
        'restricted',
        'claim-b17-glean-restricted',
        '2026-05-10T10:00:00Z',
        '2026-05-15T10:00:00Z',
        '2026-05-15T10:00:00Z',
        '2026-05-10T10:00:00Z',
        '{"tauri_user":"detail_allowed","mcp_agent":"summary_only","blocked":["object-b17-restricted-downstream","internal-graph-b17-restricted","raw-attribution-b17-restricted"]}',
        'actor_filtered_summary',
        'actor_not_authorized',
        'source_actor_visibility_changed'
    ),
    (
        'source-b17-stale-downstream',
        'glean',
        'stale',
        'claim-b17-stale-downstream',
        '2025-10-01T09:00:00Z',
        '2026-05-15T11:45:00Z',
        '2026-05-15T11:45:00Z',
        '2025-10-01T09:00:00Z',
        '{"tauri_user":"stale_qualified","mcp_agent":"stale_qualified_summary"}',
        'stale_qualified',
        'downstream_source_stale',
        'source_freshness_invalidated'
    ),
    (
        'source-b17-internal-note',
        'user_note',
        'active',
        'claim-b17-internal-only-public-risk',
        '2026-05-14T16:00:00Z',
        '2026-05-14T16:05:00Z',
        NULL,
        NULL,
        '{"tauri_user":"detail_allowed","mcp_agent":"summary_only","customer_facing":"blocked"}',
        'block_customer_facing',
        'sensitivity_internal_only',
        NULL
    );

INSERT INTO bundle17_channel_policy (
    channel, revoked_source_rejected, restricted_source_rejected, internal_only_rejected,
    safe_summary_retained, low_detail_only
)
VALUES
    ('callouts', 1, 1, 1, 1, 0),
    ('prep_outputs', 1, 1, 1, 1, 0),
    ('mcp_responses', 1, 1, 1, 1, 0),
    ('tauri_renders', 1, 1, 1, 1, 0),
    ('signal_payloads', 1, 1, 1, 1, 1),
    ('telemetry', 1, 1, 1, 1, 1),
    ('eval_fixtures', 1, 1, 1, 1, 1),
    ('replay', 1, 1, 1, 1, 1),
    ('error_logs', 1, 1, 1, 1, 1);

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, dedup_key, actor,
    data_source, source_ref, source_asof, observed_at, created_at, provenance_json,
    metadata_json, claim_state, surfacing_state, trust_score, trust_computed_at,
    trust_version, temporal_scope, sensitivity, verification_state, verification_reason
)
VALUES
    (
        'claim-b17-active-baseline',
        '{"kind":"account","id":"account-b17-example"}',
        'current_state',
        'account.status',
        'account-b17-example:status:active',
        'Active source indicates the implementation status is ready for safe discussion.',
        'dedup-b17-active-baseline',
        'agent:fixture',
        'google',
        '{"source_id":"source-b17-active-baseline","provider":"google","lifecycle_state":"active"}',
        '2026-05-15T09:00:00Z',
        '2026-05-15T09:05:00Z',
        '2026-05-15T09:05:00Z',
        '{}',
        '{"source_lifecycle_state":"active","render_policy":"show"}',
        'active',
        'active',
        0.84,
        '2026-05-15T12:00:00Z',
        1,
        'state',
        'public',
        'active',
        NULL
    ),
    (
        'claim-b17-google-disconnected',
        '{"kind":"account","id":"account-b17-example"}',
        'current_state',
        'calendar.commitment',
        'account-b17-example:calendar:disconnected',
        'A disconnected Google source previously supplied a follow-up commitment.',
        'dedup-b17-google-disconnected',
        'agent:fixture',
        'google',
        '{"source_id":"source-b17-google-disconnected","provider":"google","lifecycle_state":"unavailable"}',
        '2026-05-01T13:00:00Z',
        '2026-05-01T13:05:00Z',
        '2026-05-01T13:05:00Z',
        '{}',
        '{"source_lifecycle_state":"unavailable","render_policy":"degraded_safe_summary","invalidation_signal":"source_lifecycle_changed","redaction_reason":"source_unavailable"}',
        'active',
        'dormant',
        0.31,
        '2026-05-15T12:00:00Z',
        1,
        'state',
        'internal',
        'needs_user_decision',
        'source_unavailable'
    ),
    (
        'claim-b17-slack-revoked',
        '{"kind":"account","id":"account-b17-example"}',
        'current_state',
        'slack.thread',
        'account-b17-example:slack:revoked',
        'A revoked Slack thread had internal detail that must not drive output.',
        'dedup-b17-slack-revoked',
        'agent:fixture',
        'slack',
        '{"source_id":"source-b17-slack-revoked","provider":"slack","lifecycle_state":"revoked"}',
        '2026-04-20T10:00:00Z',
        '2026-04-20T10:05:00Z',
        '2026-04-20T10:05:00Z',
        '{}',
        '{"source_lifecycle_state":"revoked","render_policy":"masked","mask_reason":"source_revoked"}',
        'dormant',
        'dormant',
        0.18,
        '2026-05-15T12:00:00Z',
        1,
        'state',
        'confidential',
        'needs_user_decision',
        'source_revoked'
    ),
    (
        'claim-b17-gong-expired',
        '{"kind":"account","id":"account-b17-example"}',
        'current_state',
        'call.recording',
        'account-b17-example:gong:expired',
        'An expired Gong recording had call detail that must be masked.',
        'dedup-b17-gong-expired',
        'agent:fixture',
        'gong',
        '{"source_id":"source-b17-gong-expired","provider":"gong","lifecycle_state":"expired"}',
        '2026-03-15T15:00:00Z',
        '2026-03-15T15:05:00Z',
        '2026-03-15T15:05:00Z',
        '{}',
        '{"source_lifecycle_state":"expired","render_policy":"masked","mask_reason":"retention_expired"}',
        'dormant',
        'dormant',
        0.17,
        '2026-05-15T12:00:00Z',
        1,
        'state',
        'confidential',
        'needs_user_decision',
        'retention_expired'
    ),
    (
        'claim-b17-zendesk-revoked',
        '{"kind":"account","id":"account-b17-example"}',
        'current_state',
        'support.case',
        'account-b17-example:zendesk:revoked',
        'A revoked Zendesk case detail must not render as a current prep fact.',
        'dedup-b17-zendesk-revoked',
        'agent:fixture',
        'zendesk',
        '{"source_id":"source-b17-zendesk-revoked","provider":"zendesk","lifecycle_state":"revoked"}',
        '2026-04-25T11:00:00Z',
        '2026-04-25T11:05:00Z',
        '2026-04-25T11:05:00Z',
        '{}',
        '{"source_lifecycle_state":"revoked","render_policy":"masked","mask_reason":"source_revoked"}',
        'dormant',
        'dormant',
        0.16,
        '2026-05-15T12:00:00Z',
        1,
        'state',
        'confidential',
        'needs_user_decision',
        'source_revoked'
    ),
    (
        'claim-b17-glean-restricted',
        '{"kind":"account","id":"account-b17-example"}',
        'current_state',
        'support.restricted',
        'account-b17-example:glean:restricted',
        'Restricted downstream source says a support workflow changed.',
        'dedup-b17-glean-restricted',
        'agent:fixture',
        'glean',
        '{"source_id":"source-b17-glean-restricted","provider":"glean","downstream_provider":"zendesk","downstream_object_id":"object-b17-restricted-downstream","internal_graph_id":"internal-graph-b17-restricted","lifecycle_state":"restricted"}',
        '2026-05-10T10:00:00Z',
        '2026-05-15T10:00:00Z',
        '2026-05-15T10:00:00Z',
        '{}',
        '{"source_lifecycle_state":"restricted","render_policy":"actor_filtered_summary","tauri_user_visibility":"detail_allowed","mcp_agent_visibility":"summary_only","redaction_reason":"actor_not_authorized"}',
        'active',
        'active',
        0.55,
        '2026-05-15T12:00:00Z',
        1,
        'state',
        'internal',
        'contested',
        'actor_restricted'
    ),
    (
        'claim-b17-stale-downstream',
        '{"kind":"account","id":"account-b17-example"}',
        'current_state',
        'support.stale',
        'account-b17-example:stale-downstream',
        'A fresh Glean wrapper fetch points to an old downstream Zendesk object.',
        'dedup-b17-stale-downstream',
        'agent:fixture',
        'glean',
        '{"source_id":"source-b17-stale-downstream","provider":"glean","wrapper_fetch_at":"2026-05-15T11:45:00Z","downstream_provider":"zendesk","downstream_object_id":"object-b17-zendesk-stale","downstream_source_asof":"2025-10-01T09:00:00Z","lifecycle_state":"stale"}',
        '2025-10-01T09:00:00Z',
        '2026-05-15T11:45:00Z',
        '2026-05-15T11:45:00Z',
        '{}',
        '{"source_lifecycle_state":"stale","wrapper_fetch_at":"2026-05-15T11:45:00Z","downstream_source_asof":"2025-10-01T09:00:00Z","trust_path_input":"downstream_source_asof","render_policy":"stale_qualified"}',
        'active',
        'active',
        0.34,
        '2026-05-15T12:00:00Z',
        1,
        'state',
        'internal',
        'contested',
        'downstream_source_stale'
    ),
    (
        'claim-b17-internal-only-public-risk',
        '{"kind":"account","id":"account-b17-example"}',
        'current_state',
        'internal.note',
        'account-b17-example:internal-note-public-risk',
        'Internal-only note suggests wording that must never become a customer-facing suggestion.',
        'dedup-b17-internal-only-public-risk',
        'agent:fixture',
        'user_note',
        '{"source_id":"source-b17-internal-note","provider":"user_note","lifecycle_state":"active"}',
        '2026-05-14T16:00:00Z',
        '2026-05-14T16:05:00Z',
        '2026-05-14T16:05:00Z',
        '{}',
        '{"source_lifecycle_state":"active","render_policy":"block_customer_facing","public_render_allowed":false}',
        'active',
        'active',
        0.57,
        '2026-05-15T12:00:00Z',
        1,
        'state',
        'internal',
        'active',
        NULL
    );

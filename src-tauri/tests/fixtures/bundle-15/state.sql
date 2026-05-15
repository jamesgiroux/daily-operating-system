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

CREATE TABLE account_domains (
    account_id TEXT NOT NULL,
    domain TEXT NOT NULL,
    PRIMARY KEY (account_id, domain)
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

CREATE TABLE activity_log (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    subject_ref TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE surface_snapshots (
    id TEXT PRIMARY KEY,
    surface TEXT NOT NULL,
    subject_ref TEXT NOT NULL,
    rendered_claim_id TEXT NOT NULL,
    generated_at TEXT NOT NULL,
    payload_json TEXT NOT NULL
);

-- Scenario 1: daily page and readiness share the same four eligible meetings.
INSERT INTO meetings (id, title, meeting_type, start_time, end_time, attendees, created_at)
VALUES
    ('meeting-b15-example', 'Example Customer Review', 'customer', '2026-05-15T16:00:00Z', '2026-05-15T16:30:00Z', '["person@example.com"]', '2026-05-15T08:00:00Z'),
    ('meeting-b15-secondary-1', 'Example Follow Up 1', 'customer', '2026-05-15T17:00:00Z', '2026-05-15T17:30:00Z', '["person@example.com"]', '2026-05-15T08:00:00Z'),
    ('meeting-b15-secondary-2', 'Example Follow Up 2', 'customer', '2026-05-15T18:00:00Z', '2026-05-15T18:30:00Z', '["person@example.com"]', '2026-05-15T08:00:00Z'),
    ('meeting-b15-secondary-3', 'Example Follow Up 3', 'customer', '2026-05-15T19:00:00Z', '2026-05-15T19:30:00Z', '["person@example.com"]', '2026-05-15T08:00:00Z'),
    ('meeting-b15-personal', 'Personal Hold', 'personal', '2026-05-15T20:00:00Z', '2026-05-15T20:30:00Z', '["user@example.com"]', '2026-05-15T08:00:00Z');

INSERT INTO people (id, email, name)
VALUES ('person-b15-example', 'person@example.com', 'Example Person');

INSERT INTO meeting_attendees (meeting_id, person_id)
VALUES
    ('meeting-b15-example', 'person-b15-example'),
    ('meeting-b15-secondary-1', 'person-b15-example'),
    ('meeting-b15-secondary-2', 'person-b15-example'),
    ('meeting-b15-secondary-3', 'person-b15-example');

INSERT INTO entities (id, name, entity_type, tracker_path, updated_at)
VALUES
    ('account-b15-example', 'Example Account', 'account', NULL, '2026-05-15T11:05:00Z'),
    ('project-b15-example', 'Example Project', 'project', NULL, '2026-05-15T11:05:00Z'),
    ('person-b15-example', 'Example Person', 'person', NULL, '2026-05-15T11:05:00Z');

INSERT INTO account_domains (account_id, domain)
VALUES ('account-b15-example', 'example.com');

INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
VALUES
    ('meeting-b15-example', 'account-b15-example', 'account', 0.99, 1),
    ('meeting-b15-example', 'project-b15-example', 'project', 0.95, 0),
    ('meeting-b15-example', 'person-b15-example', 'person', 0.95, 0),
    ('meeting-b15-secondary-1', 'account-b15-example', 'account', 0.95, 1),
    ('meeting-b15-secondary-2', 'account-b15-example', 'account', 0.95, 1),
    ('meeting-b15-secondary-3', 'account-b15-example', 'account', 0.95, 1);

-- Scenarios 2-5 and 8: primary entity, health/risk, project status, person action, and MCP/Tauri parity.
INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, dedup_key, item_hash,
    actor, data_source, source_ref, source_asof, observed_at, created_at, provenance_json,
    metadata_json, claim_state, surfacing_state, demotion_reason, reactivated_at,
    retraction_reason, expires_at, superseded_by, trust_score, trust_computed_at,
    trust_version, thread_id, temporal_scope, sensitivity, verification_state,
    verification_reason, needs_user_decision_at, claim_version, canonical_status,
    non_semantic_mergeable
)
VALUES
    (
        'claim-b15-account-health-current',
        '{"kind":"account","id":"account-b15-example"}',
        'current_state',
        'account.health_risk',
        'account-b15-example:health-risk',
        'Example Account is at risk because expansion alignment is blocked.',
        'dedup-b15-account-health-current',
        'hash-b15-account-health-current',
        'agent:fixture',
        'support',
        '{"source_id":"source-b15-account-health-current","provider":"support","meeting_id":"meeting-b15-example"}',
        '2026-05-15T11:00:00Z',
        '2026-05-15T11:02:00Z',
        '2026-05-15T11:02:00Z',
        '{}',
        '{"health":"at_risk","risk":"expansion_blocked","project_id":"project-b15-example"}',
        'active',
        'active',
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        0.92,
        '2026-05-15T11:04:00Z',
        1,
        NULL,
        'state',
        'internal',
        'active',
        NULL,
        NULL,
        1,
        'live',
        FALSE
    ),
    (
        'claim-b15-project-status-current',
        '{"kind":"project","id":"project-b15-example"}',
        'current_state',
        'project.status',
        'project-b15-example:status',
        'Example Project is blocked by a customer dependency.',
        'dedup-b15-project-status-current',
        'hash-b15-project-status-current',
        'agent:fixture',
        'project',
        '{"source_id":"source-b15-project-status-current","provider":"project","meeting_id":"meeting-b15-example"}',
        '2026-05-15T11:00:00Z',
        '2026-05-15T11:02:00Z',
        '2026-05-15T11:02:00Z',
        '{}',
        '{"status":"blocked_by_dependency","account_id":"account-b15-example"}',
        'active',
        'active',
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        0.88,
        '2026-05-15T11:04:00Z',
        1,
        NULL,
        'state',
        'internal',
        'active',
        NULL,
        NULL,
        1,
        'live',
        FALSE
    ),
    (
        'claim-b15-person-action-current',
        '{"kind":"person","id":"person-b15-example"}',
        'open_loop',
        'actions.owner_follow_up',
        'person-b15-example:owner-follow-up',
        'Example Person owns the follow-up action for expansion alignment before the customer review.',
        'dedup-b15-person-action-current',
        'hash-b15-person-action-current',
        'agent:fixture',
        'calendar',
        '{"source_id":"source-b15-person-action-current","provider":"calendar","meeting_id":"meeting-b15-example"}',
        '2026-05-15T11:00:00Z',
        '2026-05-15T11:02:00Z',
        '2026-05-15T11:02:00Z',
        '{}',
        '{"open_loop_id":"loop-b15-owner-action","meeting_id":"meeting-b15-example","account_id":"account-b15-example"}',
        'active',
        'active',
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        0.86,
        '2026-05-15T11:04:00Z',
        1,
        NULL,
        'state',
        'internal',
        'active',
        NULL,
        NULL,
        1,
        'live',
        FALSE
    ),
    (
        'claim-b15-post-refresh-risk',
        '{"kind":"account","id":"account-b15-example"}',
        'current_state',
        'account.refresh_state',
        'account-b15-example:refresh-state',
        'After refresh completed, Example Account renders the post-refresh at-risk state.',
        'dedup-b15-post-refresh-risk',
        'hash-b15-post-refresh-risk',
        'agent:fixture',
        'support',
        '{"source_id":"source-b15-post-refresh-risk","provider":"support","activity_id":"activity-b15-refresh-completed"}',
        '2026-05-15T11:05:00Z',
        '2026-05-15T11:06:00Z',
        '2026-05-15T11:06:00Z',
        '{}',
        '{"post_refresh_state":"at_risk","supersedes":"claim-b15-pre-refresh-healthy"}',
        'active',
        'active',
        NULL,
        NULL,
        NULL,
        NULL,
        'claim-b15-pre-refresh-healthy',
        0.91,
        '2026-05-15T11:07:00Z',
        1,
        NULL,
        'state',
        'internal',
        'active',
        NULL,
        NULL,
        1,
        'live',
        FALSE
    ),
    (
        'claim-b15-pre-refresh-healthy',
        '{"kind":"account","id":"account-b15-example"}',
        'current_state',
        'account.refresh_state',
        'account-b15-example:refresh-state',
        'Before the refresh, Example Account appeared healthy.',
        'dedup-b15-pre-refresh-healthy',
        'hash-b15-pre-refresh-healthy',
        'agent:fixture',
        'support',
        '{"source_id":"source-b15-pre-refresh-healthy","provider":"support"}',
        '2026-05-15T09:00:00Z',
        '2026-05-15T09:01:00Z',
        '2026-05-15T09:01:00Z',
        '{}',
        '{"pre_refresh_state":"healthy"}',
        'dormant',
        'dormant',
        'refresh_superseded',
        NULL,
        NULL,
        NULL,
        'claim-b15-post-refresh-risk',
        0.2,
        '2026-05-15T11:07:00Z',
        1,
        NULL,
        'state',
        'internal',
        'contested',
        'superseded_by_refresh_completed',
        NULL,
        1,
        'live',
        FALSE
    ),
    (
        'claim-b15-lint-blocked-bleed',
        '{"kind":"account","id":"account-b15-example"}',
        'current_state',
        'account.private_bleed',
        'account-b15-example:private-bleed',
        'A lint-blocked adjacent-account detail must not render as confident current state.',
        'dedup-b15-lint-blocked-bleed',
        'hash-b15-lint-blocked-bleed',
        'agent:fixture',
        'lint',
        '{"source_id":"source-b15-lint-blocked-bleed","provider":"lint","blocked_reason":"subject_bleed"}',
        '2026-05-15T10:30:00Z',
        '2026-05-15T10:31:00Z',
        '2026-05-15T10:31:00Z',
        '{}',
        '{"render_policy":"blocked","lint_rule":"subject_bleed"}',
        'dormant',
        'dormant',
        'lint_blocked_subject_bleed',
        NULL,
        NULL,
        NULL,
        NULL,
        0.1,
        '2026-05-15T11:04:00Z',
        1,
        NULL,
        'state',
        'internal',
        'contested',
        'lint_blocked_subject_bleed',
        NULL,
        1,
        'live',
        FALSE
    );

-- Scenario 6: lint-blocked claim cannot render confidently on another surface.
INSERT INTO claim_feedback (id, claim_id, feedback_type, actor, actor_id, payload_json, submitted_at, applied_at)
VALUES (
    'feedback-b15-lint-blocked-bleed',
    'claim-b15-lint-blocked-bleed',
    'mark_outdated',
    'system',
    NULL,
    '{"render_policy":"blocked","reason":"subject_bleed"}',
    '2026-05-15T10:31:00Z',
    '2026-05-15T10:31:00Z'
);

-- Scenario 7: refresh_completed means subsequent reads use post-refresh state.
INSERT INTO activity_log (id, event_type, subject_ref, payload_json, created_at)
VALUES (
    'activity-b15-refresh-completed',
    'refresh_completed',
    '{"kind":"account","id":"account-b15-example"}',
    '{"pre_refresh_claim_id":"claim-b15-pre-refresh-healthy","post_refresh_claim_id":"claim-b15-post-refresh-risk"}',
    '2026-05-15T11:05:00Z'
);

INSERT INTO surface_snapshots (id, surface, subject_ref, rendered_claim_id, generated_at, payload_json)
VALUES
    ('surface-b15-entity-post-refresh', 'get_entity_context', '{"kind":"account","id":"account-b15-example"}', 'claim-b15-post-refresh-risk', '2026-05-15T12:15:00Z', '{"state":"at_risk"}'),
    ('surface-b15-meeting-post-refresh', 'prepare_meeting', '{"kind":"meeting","id":"meeting-b15-example"}', 'claim-b15-post-refresh-risk', '2026-05-15T12:15:00Z', '{"state":"at_risk"}'),
    ('surface-b15-dashboard-post-refresh', 'dashboard', '{"kind":"workspace","id":"workspace-b15-example"}', 'claim-b15-post-refresh-risk', '2026-05-15T12:15:00Z', '{"state":"at_risk"}'),
    ('surface-b15-mcp-post-refresh', 'mcp', '{"kind":"account","id":"account-b15-example"}', 'claim-b15-post-refresh-risk', '2026-05-15T12:15:00Z', '{"state":"at_risk"}');

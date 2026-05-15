CREATE TABLE bundle16_scenarios (
    scenario_id TEXT PRIMARY KEY,
    owner_type TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    substrate_table TEXT NOT NULL,
    expected_state TEXT NOT NULL,
    notes TEXT NOT NULL
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

CREATE TABLE linked_entities_raw (
    owner_type TEXT NOT NULL CHECK (owner_type IN ('meeting', 'email', 'email_thread')),
    owner_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('primary', 'related', 'auto_suggested')),
    source TEXT NOT NULL,
    rule_id TEXT,
    confidence REAL,
    evidence_json TEXT,
    graph_version INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (owner_type, owner_id, entity_id, entity_type)
);

CREATE UNIQUE INDEX idx_one_primary
    ON linked_entities_raw (owner_type, owner_id)
    WHERE role = 'primary';

CREATE VIEW linked_entities AS
    SELECT * FROM linked_entities_raw
    WHERE source != 'user_dismissed';

CREATE TABLE entity_linking_evaluations (
    id INTEGER PRIMARY KEY,
    owner_type TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    link_trigger TEXT NOT NULL,
    rule_id TEXT,
    entity_id TEXT,
    entity_type TEXT,
    role TEXT,
    graph_version INTEGER NOT NULL,
    evidence_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
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

CREATE TABLE linear_issues (
    id TEXT PRIMARY KEY,
    identifier TEXT NOT NULL,
    title TEXT NOT NULL,
    state_name TEXT,
    state_type TEXT,
    priority INTEGER,
    priority_label TEXT,
    project_id TEXT,
    project_name TEXT,
    due_date TEXT,
    url TEXT,
    synced_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE linear_projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    state TEXT,
    url TEXT,
    synced_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE linear_entity_links (
    id TEXT PRIMARY KEY,
    linear_project_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('account', 'project', 'person')),
    confirmed INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(linear_project_id, entity_id, entity_type)
);

INSERT INTO bundle16_scenarios (
    scenario_id, owner_type, owner_id, substrate_table, expected_state, notes
)
VALUES
    ('same-domain-twins', 'meeting', 'meeting-b16-same-domain-twins', 'linked_entities_raw', 'ambiguous', 'Two active account candidates share one attendee domain.'),
    ('parent-child-account', 'meeting', 'meeting-b16-parent-child-account', 'linked_entities_raw', 'primary', 'Direct child attendee evidence outranks inherited parent domain evidence.'),
    ('similar-project-names', 'meeting', 'meeting-b16-similar-project-names', 'linked_entities_raw', 'ambiguous', 'Two projects under one account share the Phoenix prefix.'),
    ('same-name-people', 'meeting', 'meeting-b16-same-name-people', 'linked_entities_raw', 'ambiguous', 'Two people share first name and similar title.'),
    ('recurring-series-subject-change', 'meeting', 'meeting-b16-recurring-current', 'linked_entities_raw', 'primary', 'Current attendee evidence outranks historical recurring-series inheritance.'),
    ('email-thread-two-customers', 'email_thread', 'thread-b16-two-customers', 'linked_entities_raw', 'ambiguous', 'One email thread mentions two customer accounts.'),
    ('linear-title-internal-work', 'linear_issue', 'linear-b16-internal-work', 'entity_linking_evaluations', 'unconfirmed', 'Linear title matches an account but project metadata is internal platform work.'),
    ('user-confirmed-override-attempt', 'meeting', 'meeting-b16-user-confirmed-override-attempt', 'linked_entities_raw', 'primary', 'User-confirmed primary row survives rejected classifier override attempt.');

INSERT INTO meetings (
    id, title, meeting_type, start_time, end_time, attendees, created_at,
    calendar_event_id, description
)
VALUES
    ('meeting-b16-same-domain-twins', 'Shared Domain Account Review', 'external', '2026-05-15T16:00:00Z', '2026-05-15T16:30:00Z', '["casey@shared.example.com"]', '2026-05-15T08:00:00Z', 'cal-b16-same-domain', 'One attendee domain maps to two active accounts.'),
    ('meeting-b16-parent-child-account', 'Child Account Implementation', 'external', '2026-05-15T17:00:00Z', '2026-05-15T17:30:00Z', '["devon@child.example.com"]', '2026-05-15T08:05:00Z', 'cal-b16-parent-child', 'Direct child account attendee plus inherited parent account.'),
    ('meeting-b16-similar-project-names', 'Phoenix Project Planning', 'external', '2026-05-15T18:00:00Z', '2026-05-15T18:30:00Z', '["project@example.com"]', '2026-05-15T08:10:00Z', 'cal-b16-similar-projects', 'Two same-prefix project candidates.'),
    ('meeting-b16-same-name-people', 'Alex Follow Up', 'external', '2026-05-15T19:00:00Z', '2026-05-15T19:30:00Z', '["alex.a@example.com","alex.b@example.com"]', '2026-05-15T08:15:00Z', 'cal-b16-same-name-people', 'Two similar people candidates.'),
    ('meeting-b16-recurring-history', 'Recurring Account A Historical', 'external', '2026-05-01T15:00:00Z', '2026-05-01T15:30:00Z', '["old@account-a.example.com"]', '2026-05-01T08:00:00Z', 'series-b16-subject-shift-001', 'Historical instance was about Account A.'),
    ('meeting-b16-recurring-current', 'Recurring Account B Current', 'external', '2026-05-15T20:00:00Z', '2026-05-15T20:30:00Z', '["new@account-b.example.com"]', '2026-05-15T08:20:00Z', 'series-b16-subject-shift-002', 'Current instance is about Account B.'),
    ('meeting-b16-user-confirmed-override-attempt', 'User Confirmed Subject Review', 'external', '2026-05-15T21:00:00Z', '2026-05-15T21:30:00Z', '["confirmed@user-choice.example.com"]', '2026-05-15T08:25:00Z', 'cal-b16-user-confirmed', 'User selected Account Confirmed Example as primary.');

INSERT INTO people (id, email, name)
VALUES
    ('person-b16-casey-shared', 'casey@shared.example.com', 'Casey Example'),
    ('person-b16-devon-child', 'devon@child.example.com', 'Devon Example'),
    ('person-b16-project-contact', 'project@example.com', 'Project Example'),
    ('person-b16-alex-a', 'alex.a@example.com', 'Alex Example A'),
    ('person-b16-alex-b', 'alex.b@example.com', 'Alex Example B'),
    ('person-b16-old-account-a', 'old@account-a.example.com', 'Old Account Example'),
    ('person-b16-new-account-b', 'new@account-b.example.com', 'New Account Example'),
    ('person-b16-confirmed', 'confirmed@user-choice.example.com', 'Confirmed Example');

INSERT INTO meeting_attendees (meeting_id, person_id)
VALUES
    ('meeting-b16-same-domain-twins', 'person-b16-casey-shared'),
    ('meeting-b16-parent-child-account', 'person-b16-devon-child'),
    ('meeting-b16-similar-project-names', 'person-b16-project-contact'),
    ('meeting-b16-same-name-people', 'person-b16-alex-a'),
    ('meeting-b16-same-name-people', 'person-b16-alex-b'),
    ('meeting-b16-recurring-history', 'person-b16-old-account-a'),
    ('meeting-b16-recurring-current', 'person-b16-new-account-b'),
    ('meeting-b16-user-confirmed-override-attempt', 'person-b16-confirmed');

INSERT INTO entities (id, name, entity_type, tracker_path, updated_at)
VALUES
    ('account-b16-domain-a', 'Account Domain A Example', 'account', NULL, '2026-05-15T08:00:00Z'),
    ('account-b16-domain-b', 'Account Domain B Example', 'account', NULL, '2026-05-15T08:00:00Z'),
    ('account-b16-parent-example', 'Parent Account Example', 'account', NULL, '2026-05-15T08:00:00Z'),
    ('account-b16-child-example', 'Child Account Example', 'account', NULL, '2026-05-15T08:00:00Z'),
    ('account-b16-project-parent', 'Project Parent Account Example', 'account', NULL, '2026-05-15T08:00:00Z'),
    ('project-b16-phoenix-alpha', 'Phoenix Alpha', 'project', NULL, '2026-05-15T08:00:00Z'),
    ('project-b16-phoenix-beta', 'Phoenix Beta', 'project', NULL, '2026-05-15T08:00:00Z'),
    ('person-b16-alex-a', 'Alex Example A', 'person', NULL, '2026-05-15T08:00:00Z'),
    ('person-b16-alex-b', 'Alex Example B', 'person', NULL, '2026-05-15T08:00:00Z'),
    ('account-b16-recurring-a', 'Account Recurring A Example', 'account', NULL, '2026-05-15T08:00:00Z'),
    ('account-b16-recurring-b', 'Account Recurring B Example', 'account', NULL, '2026-05-15T08:00:00Z'),
    ('account-b16-thread-a', 'Account Thread A Example', 'account', NULL, '2026-05-15T08:00:00Z'),
    ('account-b16-thread-b', 'Account Thread B Example', 'account', NULL, '2026-05-15T08:00:00Z'),
    ('account-b16-linear-title-match', 'Account Linear Example', 'account', NULL, '2026-05-15T08:00:00Z'),
    ('account-b16-user-confirmed', 'Account Confirmed Example', 'account', NULL, '2026-05-15T08:00:00Z'),
    ('account-b16-classifier-attempt', 'Account Classifier Attempt Example', 'account', NULL, '2026-05-15T08:00:00Z');

INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
VALUES
    ('meeting-b16-parent-child-account', 'account-b16-child-example', 'account', 0.92, 1),
    ('meeting-b16-parent-child-account', 'account-b16-parent-example', 'account', 0.70, 0),
    ('meeting-b16-recurring-history', 'account-b16-recurring-a', 'account', 0.95, 1),
    ('meeting-b16-recurring-current', 'account-b16-recurring-b', 'account', 0.91, 1),
    ('meeting-b16-recurring-current', 'account-b16-recurring-a', 'account', 0.68, 0),
    ('meeting-b16-user-confirmed-override-attempt', 'account-b16-user-confirmed', 'account', 1.00, 1);

INSERT INTO linked_entities_raw (
    owner_type, owner_id, entity_id, entity_type, role, source, rule_id,
    confidence, evidence_json, graph_version, created_at
)
VALUES
    ('meeting', 'meeting-b16-same-domain-twins', 'account-b16-domain-a', 'account', 'related', 'rule:P9', 'P9', 0.81, '{"scenario_id":"same-domain-twins","selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary","confidence_margin":0.01,"evidence":{"domain":"shared.example.com","source_asof":"2026-05-15T09:00:00Z"}}', 16, '2026-05-15T09:05:00Z'),
    ('meeting', 'meeting-b16-same-domain-twins', 'account-b16-domain-b', 'account', 'related', 'rule:P9', 'P9', 0.80, '{"scenario_id":"same-domain-twins","selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary","confidence_margin":0.01,"evidence":{"domain":"shared.example.com","source_asof":"2026-05-15T09:00:00Z"}}', 16, '2026-05-15T09:05:00Z'),
    ('meeting', 'meeting-b16-parent-child-account', 'account-b16-child-example', 'account', 'primary', 'rule:P4c', 'P4c', 0.92, '{"scenario_id":"parent-child-account","selection_state":"primary","decision_rule":"direct_over_inherited","evidence":{"direct_attendee":{"score":0.92,"source_asof":"2026-05-15T09:10:00Z"},"inherited_parent":{"score":0.70,"inheritance_reason":"parent_account_domain_match","source_asof":"2026-05-15T09:10:00Z"}}}', 16, '2026-05-15T09:11:00Z'),
    ('meeting', 'meeting-b16-parent-child-account', 'account-b16-parent-example', 'account', 'related', 'inherited_from_parent', 'P4c-parent', 0.70, '{"scenario_id":"parent-child-account","selection_state":"secondary","inheritance_reason":"parent_account_domain_match","decision_rule":"direct_over_inherited"}', 16, '2026-05-15T09:11:00Z'),
    ('meeting', 'meeting-b16-similar-project-names', 'project-b16-phoenix-alpha', 'project', 'related', 'rule:P5', 'P5', 0.78, '{"scenario_id":"similar-project-names","selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary","confidence_margin":0.01,"evidence":{"title_prefix":"Phoenix","source_asof":"2026-05-15T09:20:00Z"}}', 16, '2026-05-15T09:21:00Z'),
    ('meeting', 'meeting-b16-similar-project-names', 'project-b16-phoenix-beta', 'project', 'related', 'rule:P5', 'P5', 0.77, '{"scenario_id":"similar-project-names","selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary","confidence_margin":0.01,"evidence":{"title_prefix":"Phoenix","source_asof":"2026-05-15T09:20:00Z"}}', 16, '2026-05-15T09:21:00Z'),
    ('meeting', 'meeting-b16-same-name-people', 'person-b16-alex-a', 'person', 'related', 'rule:P6', 'P6', 0.74, '{"scenario_id":"same-name-people","selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary","confidence_margin":0.01,"evidence":{"name":"Alex Example","source_asof":"2026-05-15T09:30:00Z"}}', 16, '2026-05-15T09:31:00Z'),
    ('meeting', 'meeting-b16-same-name-people', 'person-b16-alex-b', 'person', 'related', 'rule:P6', 'P6', 0.73, '{"scenario_id":"same-name-people","selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary","confidence_margin":0.01,"evidence":{"name":"Alex Example","source_asof":"2026-05-15T09:30:00Z"}}', 16, '2026-05-15T09:31:00Z'),
    ('meeting', 'meeting-b16-recurring-history', 'account-b16-recurring-a', 'account', 'primary', 'user', 'P1', 1.00, '{"scenario_id":"recurring-series-subject-change","selection_state":"historical_primary","source_asof":"2026-05-01T09:40:00Z"}', 15, '2026-05-01T09:40:00Z'),
    ('meeting', 'meeting-b16-recurring-current', 'account-b16-recurring-b', 'account', 'primary', 'rule:P4c', 'P4c', 0.91, '{"scenario_id":"recurring-series-subject-change","selection_state":"primary","decision_rule":"current_direct_over_historical_inheritance","evidence":{"current_direct":{"score":0.91,"source_asof":"2026-05-15T09:40:00Z"},"historical_inherited":{"score":0.68,"inheritance_reason":"historical_series_subject","source_asof":"2026-05-01T09:40:00Z"}}}', 16, '2026-05-15T09:41:00Z'),
    ('meeting', 'meeting-b16-recurring-current', 'account-b16-recurring-a', 'account', 'related', 'inherited_from_series', 'P3', 0.68, '{"scenario_id":"recurring-series-subject-change","selection_state":"secondary","inheritance_reason":"historical_series_subject","decision_rule":"current_direct_over_historical_inheritance"}', 16, '2026-05-15T09:41:00Z'),
    ('email_thread', 'thread-b16-two-customers', 'account-b16-thread-a', 'account', 'related', 'rule:P2', 'P2', 0.83, '{"scenario_id":"email-thread-two-customers","selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary","confidence_margin":0.01,"evidence":{"thread_id":"thread-b16-two-customers","source_asof":"2026-05-15T09:50:00Z"}}', 16, '2026-05-15T09:51:00Z'),
    ('email_thread', 'thread-b16-two-customers', 'account-b16-thread-b', 'account', 'related', 'rule:P2', 'P2', 0.82, '{"scenario_id":"email-thread-two-customers","selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary","confidence_margin":0.01,"evidence":{"thread_id":"thread-b16-two-customers","source_asof":"2026-05-15T09:50:00Z"}}', 16, '2026-05-15T09:51:00Z'),
    ('meeting', 'meeting-b16-user-confirmed-override-attempt', 'account-b16-user-confirmed', 'account', 'primary', 'user', 'P1', 1.00, '{"scenario_id":"user-confirmed-override-attempt","selection_state":"primary","decision_rule":"user_confirmed_wins","row_id":"linked:meeting:meeting-b16-user-confirmed-override-attempt:account-b16-user-confirmed","source_asof":"2026-05-15T10:10:00Z"}', 16, '2026-05-15T10:10:00Z'),
    ('meeting', 'meeting-b16-user-confirmed-override-attempt', 'account-b16-classifier-attempt', 'account', 'related', 'rule:P4b', 'P4b', 0.93, '{"scenario_id":"user-confirmed-override-attempt","selection_state":"rejected_override_attempt","decision_rule":"user_confirmed_wins","rejected":true,"attempted_role":"primary","source_asof":"2026-05-15T10:11:00Z"}', 16, '2026-05-15T10:11:00Z');

INSERT INTO entity_linking_evaluations (
    id, owner_type, owner_id, link_trigger, rule_id, entity_id, entity_type,
    role, graph_version, evidence_json, created_at
)
VALUES
    (1601, 'meeting', 'meeting-b16-same-domain-twins', 'CalendarPoll', 'P9', NULL, NULL, NULL, 16, '{"scenario_id":"same-domain-twins","selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary","confidence_margin":0.01,"candidates":["account-b16-domain-a","account-b16-domain-b"],"rejected_provider_confident_primary":true,"source_asof":"2026-05-15T09:00:00Z"}', '2026-05-15T09:05:00Z'),
    (1602, 'meeting', 'meeting-b16-parent-child-account', 'CalendarPoll', 'P4c', 'account-b16-child-example', 'account', 'primary', 16, '{"scenario_id":"parent-child-account","selection_state":"primary","decision_rule":"direct_over_inherited","chosen_subject_ref":{"kind":"account","id":"account-b16-child-example"},"alternatives":["account-b16-parent-example"],"evidence":{"direct_attendee":{"score":0.92,"source_asof":"2026-05-15T09:10:00Z"},"inherited_parent":{"score":0.70,"inheritance_reason":"parent_account_domain_match","source_asof":"2026-05-15T09:10:00Z"}}}', '2026-05-15T09:11:00Z'),
    (1603, 'meeting', 'meeting-b16-similar-project-names', 'LinearSync', 'P5', NULL, NULL, NULL, 16, '{"scenario_id":"similar-project-names","selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary","confidence_margin":0.01,"candidates":["project-b16-phoenix-alpha","project-b16-phoenix-beta"],"rejected_provider_confident_primary":true,"source_asof":"2026-05-15T09:20:00Z"}', '2026-05-15T09:21:00Z'),
    (1604, 'meeting', 'meeting-b16-same-name-people', 'CalendarPoll', 'P6', NULL, NULL, NULL, 16, '{"scenario_id":"same-name-people","selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary","confidence_margin":0.01,"candidates":["person-b16-alex-a","person-b16-alex-b"],"rejected_provider_confident_primary":true,"source_asof":"2026-05-15T09:30:00Z"}', '2026-05-15T09:31:00Z'),
    (1605, 'meeting', 'meeting-b16-recurring-current', 'CalendarPoll', 'P4c', 'account-b16-recurring-b', 'account', 'primary', 16, '{"scenario_id":"recurring-series-subject-change","selection_state":"primary","decision_rule":"current_direct_over_historical_inheritance","chosen_subject_ref":{"kind":"account","id":"account-b16-recurring-b"},"alternatives":["account-b16-recurring-a"],"evidence":{"current_direct":{"score":0.91,"source_asof":"2026-05-15T09:40:00Z"},"historical_inherited":{"score":0.68,"inheritance_reason":"historical_series_subject","source_asof":"2026-05-01T09:40:00Z"}}}', '2026-05-15T09:41:00Z'),
    (1606, 'email_thread', 'thread-b16-two-customers', 'EmailFetch', 'P2', NULL, NULL, NULL, 16, '{"scenario_id":"email-thread-two-customers","selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary","confidence_margin":0.01,"candidates":["account-b16-thread-a","account-b16-thread-b"],"rejected_provider_confident_primary":true,"source_asof":"2026-05-15T09:50:00Z"}', '2026-05-15T09:51:00Z'),
    (1607, 'linear_issue', 'linear-b16-internal-work', 'LinearSync', NULL, NULL, NULL, NULL, 16, '{"scenario_id":"linear-title-internal-work","selection_state":"unconfirmed","decision_rule":"title_match_does_not_select_customer_for_internal_work","attempted_subject_ref":{"kind":"account","id":"account-b16-linear-title-match"},"rejected":true,"rejection_reason":"internal_platform_work","source_asof":"2026-05-15T10:00:00Z"}', '2026-05-15T10:01:00Z'),
    (1608, 'meeting', 'meeting-b16-user-confirmed-override-attempt', 'CalendarPoll', 'P4b', 'account-b16-classifier-attempt', 'account', 'rejected', 16, '{"attempt_id":"classifier-override-b16-user-confirmed","scenario_id":"user-confirmed-override-attempt","attempted_subject_ref":{"kind":"account","id":"account-b16-classifier-attempt"},"preserved_subject_ref":{"kind":"account","id":"account-b16-user-confirmed"},"rejected":true,"rejection_reason":"user_confirmed_wins","source_asof":"2026-05-15T10:11:00Z"}', '2026-05-15T10:11:00Z');

INSERT INTO linear_projects (id, name, state, url, synced_at)
VALUES ('linear-project-b16-internal', 'Internal Platform', 'started', 'https://linear.example.com/project/internal', '2026-05-15T10:00:00Z');

INSERT INTO linear_issues (
    id, identifier, title, state_name, state_type, priority, priority_label,
    project_id, project_name, due_date, url, synced_at
)
VALUES (
    'linear-b16-internal-work',
    'DOS-291',
    'Account Linear Example search ranking cleanup',
    'In Progress',
    'started',
    3,
    'Medium',
    'linear-project-b16-internal',
    'Internal Platform',
    NULL,
    'https://linear.example.com/issue/DOS-291',
    '2026-05-15T10:00:00Z'
);

INSERT INTO linear_entity_links (
    id, linear_project_id, entity_id, entity_type, confirmed, created_at
)
VALUES (
    'linear-link-b16-rejected-title-match',
    'linear-project-b16-internal',
    'account-b16-linear-title-match',
    'account',
    0,
    '2026-05-15T10:01:00Z'
);

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, dedup_key,
    item_hash, actor, data_source, source_ref, source_asof, observed_at,
    created_at, provenance_json, metadata_json, claim_state, surfacing_state,
    demotion_reason, reactivated_at, retraction_reason, expires_at,
    superseded_by, trust_score, trust_computed_at, trust_version, thread_id,
    temporal_scope, sensitivity, verification_state, verification_reason,
    needs_user_decision_at
)
VALUES
    ('claim-b16-same-domain-ambiguous', '{"kind":"meeting","id":"meeting-b16-same-domain-twins"}', 'subject_selection', 'primary_subject', 'same-domain-twins', 'Shared domain evidence is tied across two account candidates.', 'dedup-b16-same-domain', 'hash-b16-same-domain', 'agent:fixture', 'crm', '{"source_id":"source-b16-same-domain-twins"}', '2026-05-15T09:00:00Z', '2026-05-15T09:05:00Z', '2026-05-15T09:05:00Z', '{"scenario_id":"same-domain-twins"}', '{"selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary"}', 'active', 'active', NULL, NULL, NULL, NULL, NULL, 0.50, '2026-05-15T12:00:00Z', 1, NULL, 'state', 'internal', 'needs_user_decision', 'ambiguous_subject_selection', '2026-05-15T09:05:00Z'),
    ('claim-b16-parent-child-primary', '{"kind":"account","id":"account-b16-child-example"}', 'subject_selection', 'primary_subject', 'parent-child-account', 'Direct child attendee evidence outranks inherited parent account evidence.', 'dedup-b16-parent-child', 'hash-b16-parent-child', 'agent:fixture', 'calendar', '{"source_id":"source-b16-parent-child-account"}', '2026-05-15T09:10:00Z', '2026-05-15T09:11:00Z', '2026-05-15T09:11:00Z', '{"scenario_id":"parent-child-account"}', '{"selection_state":"primary","decision_rule":"direct_over_inherited","direct_score":0.92,"inherited_score":0.70}', 'active', 'active', NULL, NULL, NULL, NULL, NULL, 0.92, '2026-05-15T12:00:00Z', 1, NULL, 'state', 'internal', 'active', NULL, NULL),
    ('claim-b16-similar-projects-ambiguous', '{"kind":"meeting","id":"meeting-b16-similar-project-names"}', 'subject_selection', 'primary_subject', 'similar-project-names', 'Phoenix project title evidence is tied across two project candidates.', 'dedup-b16-similar-projects', 'hash-b16-similar-projects', 'agent:fixture', 'linear', '{"source_id":"source-b16-similar-project-names"}', '2026-05-15T09:20:00Z', '2026-05-15T09:21:00Z', '2026-05-15T09:21:00Z', '{"scenario_id":"similar-project-names"}', '{"selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary"}', 'active', 'active', NULL, NULL, NULL, NULL, NULL, 0.50, '2026-05-15T12:00:00Z', 1, NULL, 'state', 'internal', 'needs_user_decision', 'ambiguous_subject_selection', '2026-05-15T09:21:00Z'),
    ('claim-b16-same-name-people-ambiguous', '{"kind":"meeting","id":"meeting-b16-same-name-people"}', 'subject_selection', 'primary_subject', 'same-name-people', 'Same-name people evidence is tied and requires confirmation.', 'dedup-b16-same-name-people', 'hash-b16-same-name-people', 'agent:fixture', 'calendar', '{"source_id":"source-b16-same-name-people"}', '2026-05-15T09:30:00Z', '2026-05-15T09:31:00Z', '2026-05-15T09:31:00Z', '{"scenario_id":"same-name-people"}', '{"selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary"}', 'active', 'active', NULL, NULL, NULL, NULL, NULL, 0.50, '2026-05-15T12:00:00Z', 1, NULL, 'state', 'internal', 'needs_user_decision', 'ambiguous_subject_selection', '2026-05-15T09:31:00Z'),
    ('claim-b16-recurring-current-primary', '{"kind":"account","id":"account-b16-recurring-b"}', 'subject_selection', 'primary_subject', 'recurring-series-subject-change', 'Current attendee evidence selects Account Recurring B over historical series Account Recurring A.', 'dedup-b16-recurring-current', 'hash-b16-recurring-current', 'agent:fixture', 'calendar', '{"source_id":"source-b16-recurring-series-subject-change"}', '2026-05-15T09:40:00Z', '2026-05-15T09:41:00Z', '2026-05-15T09:41:00Z', '{"scenario_id":"recurring-series-subject-change"}', '{"selection_state":"primary","decision_rule":"current_direct_over_historical_inheritance","current_direct_score":0.91,"historical_inherited_score":0.68}', 'active', 'active', NULL, NULL, NULL, NULL, NULL, 0.91, '2026-05-15T12:00:00Z', 1, NULL, 'state', 'internal', 'active', NULL, NULL),
    ('claim-b16-email-thread-ambiguous', '{"kind":"email_thread","id":"thread-b16-two-customers"}', 'subject_selection', 'primary_subject', 'email-thread-two-customers', 'Email thread evidence names two account candidates and remains ambiguous.', 'dedup-b16-email-thread', 'hash-b16-email-thread', 'agent:fixture', 'email', '{"source_id":"source-b16-email-thread-two-customers"}', '2026-05-15T09:50:00Z', '2026-05-15T09:51:00Z', '2026-05-15T09:51:00Z', '{"scenario_id":"email-thread-two-customers"}', '{"selection_state":"ambiguous","decision_rule":"ambiguous_blocks_confident_primary"}', 'active', 'active', NULL, NULL, NULL, NULL, NULL, 0.50, '2026-05-15T12:00:00Z', 1, 'thread-b16-two-customers', 'state', 'internal', 'needs_user_decision', 'ambiguous_subject_selection', '2026-05-15T09:51:00Z'),
    ('claim-b16-linear-internal-unconfirmed', '{"kind":"linear_issue","id":"linear-b16-internal-work"}', 'subject_selection', 'primary_subject', 'linear-title-internal-work', 'Linear title evidence is internal platform work and does not select the title-matched account.', 'dedup-b16-linear-internal', 'hash-b16-linear-internal', 'agent:fixture', 'linear', '{"source_id":"source-b16-linear-title-internal-work"}', '2026-05-15T10:00:00Z', '2026-05-15T10:01:00Z', '2026-05-15T10:01:00Z', '{"scenario_id":"linear-title-internal-work"}', '{"selection_state":"unconfirmed","decision_rule":"title_match_does_not_select_customer_for_internal_work"}', 'active', 'active', NULL, NULL, NULL, NULL, NULL, 0.52, '2026-05-15T12:00:00Z', 1, NULL, 'state', 'internal', 'active', NULL, NULL),
    ('claim-b16-user-confirmed-primary', '{"kind":"account","id":"account-b16-user-confirmed"}', 'subject_selection', 'primary_subject', 'user-confirmed-override-attempt', 'User-confirmed primary subject survives a classifier attempt to promote another account.', 'dedup-b16-user-confirmed', 'hash-b16-user-confirmed', 'user:fixture', 'user', '{"source_id":"source-b16-user-confirmed-override-attempt"}', '2026-05-15T10:10:00Z', '2026-05-15T10:10:00Z', '2026-05-15T10:10:00Z', '{"scenario_id":"user-confirmed-override-attempt"}', '{"selection_state":"primary","decision_rule":"user_confirmed_wins","row_id":"linked:meeting:meeting-b16-user-confirmed-override-attempt:account-b16-user-confirmed"}', 'active', 'active', NULL, NULL, NULL, NULL, NULL, 1.00, '2026-05-15T12:00:00Z', 1, NULL, 'state', 'internal', 'active', NULL, NULL);

INSERT INTO claim_feedback (
    id, claim_id, feedback_type, actor, actor_id, payload_json,
    submitted_at, applied_at
)
VALUES (
    'feedback-b16-user-confirmed-primary',
    'claim-b16-user-confirmed-primary',
    'confirm_current',
    'user',
    NULL,
    '{"reason":"manual_primary_subject_selection","preserved_linked_entities_raw_row":"linked:meeting:meeting-b16-user-confirmed-override-attempt:account-b16-user-confirmed"}',
    '2026-05-15T10:10:00Z',
    '2026-05-15T10:10:00Z'
);

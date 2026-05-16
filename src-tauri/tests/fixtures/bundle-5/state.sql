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
    id TEXT PRIMARY KEY, subject_ref TEXT NOT NULL, claim_type TEXT NOT NULL, field_path TEXT,
    topic_key TEXT, text TEXT NOT NULL, dedup_key TEXT NOT NULL, item_hash TEXT, actor TEXT NOT NULL,
    data_source TEXT NOT NULL, source_ref TEXT, source_asof TEXT, observed_at TEXT NOT NULL,
    created_at TEXT NOT NULL, provenance_json TEXT NOT NULL, metadata_json TEXT,
    claim_state TEXT NOT NULL DEFAULT 'active', surfacing_state TEXT NOT NULL DEFAULT 'active',
    demotion_reason TEXT, reactivated_at TEXT, retraction_reason TEXT, expires_at TEXT,
    superseded_by TEXT, trust_score REAL, trust_computed_at TEXT, trust_version INTEGER,
    thread_id TEXT, temporal_scope TEXT NOT NULL DEFAULT 'state', sensitivity TEXT NOT NULL DEFAULT 'internal',
    verification_state TEXT NOT NULL DEFAULT 'active', verification_reason TEXT, needs_user_decision_at TEXT,
    claim_version INTEGER NOT NULL DEFAULT 1,
    canonical_status TEXT NOT NULL DEFAULT 'live',
    non_semantic_mergeable BOOLEAN NOT NULL DEFAULT FALSE
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

INSERT INTO meetings (id, title, meeting_type, start_time, end_time, attendees, created_at)
VALUES
    ('meeting-b5-first-person', 'Intro with Riley Rivera', 'external', '2026-05-06T15:00:00Z', '2026-05-06T15:30:00Z', '["riley@first-person.example.com"]', '2026-05-05T15:00:00Z');

INSERT INTO people (id, email, name)
VALUES
    ('person-b5-riley', 'riley@first-person.example.com', 'Riley Rivera');

INSERT INTO meeting_attendees (meeting_id, person_id)
VALUES
    ('meeting-b5-first-person', 'person-b5-riley');

INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, topic_key, text, dedup_key, item_hash, actor,
    data_source, source_ref, source_asof, observed_at, created_at, provenance_json, metadata_json,
    claim_state, surfacing_state, demotion_reason, reactivated_at, retraction_reason, expires_at,
    superseded_by, trust_score, trust_computed_at, trust_version, thread_id, temporal_scope, sensitivity,
    verification_state, verification_reason, needs_user_decision_at
)
VALUES
    ('src-b5-intro-note', '{"id":"person-b5-riley","kind":"person"}', 'attendee_context', 'summary', NULL, 'Riley Rivera is new to the workspace and asked for a concise first-call agenda.', 'dedup-src-b5-intro-note', 'hash-src-b5-intro-note', 'agent:fixture', 'user', 'meeting-b5-first-person', '2026-05-05T15:30:00Z', '2026-05-05T15:30:00Z', '2026-05-05T15:30:00Z', '{}', '{"scenario":"first_person_parity"}', 'active', 'active', NULL, NULL, NULL, NULL, NULL, 0.92, NULL, NULL, NULL, 'state', 'internal', 'active', NULL, NULL),
    ('src-b5-wrong-attendee-original', '{"id":"person-b5-riley","kind":"person"}', 'attendee_context', 'summary', NULL, 'Riley Rivera owns the renewal approval for Example Portfolio.', 'dedup-src-b5-wrong-attendee-original', 'hash-src-b5-wrong-attendee-original', 'agent:fixture', 'email', 'meeting-b5-first-person', '2026-05-05T14:00:00Z', '2026-05-05T14:00:00Z', '2026-05-05T14:00:00Z', '{}', '{"scenario":"wrong_subject_tombstone","corrected_to":{"kind":"account","id":"acct-b5-example-portfolio"}}', 'tombstoned', 'dormant', 'wrong_subject_feedback', NULL, 'wrong_subject', NULL, NULL, 0.21, NULL, NULL, NULL, 'state', 'internal', 'contested', 'wrong_subject', NULL),
    ('src-b5-original-preference', '{"id":"person-b5-riley","kind":"person"}', 'attendee_context', 'summary', NULL, 'Riley prefers a broad discovery agenda.', 'dedup-src-b5-preference', 'hash-src-b5-original-preference', 'agent:fixture', 'email', 'meeting-b5-first-person', '2026-05-05T14:15:00Z', '2026-05-05T14:15:00Z', '2026-05-05T14:15:00Z', '{}', '{"scenario":"user_edited_supersession","superseded_by":"src-b5-user-edited-preference"}', 'dormant', 'dormant', 'user_edited_superseded', NULL, NULL, NULL, 'src-b5-user-edited-preference', 0.48, NULL, NULL, NULL, 'state', 'internal', 'active', NULL, NULL),
    ('src-b5-user-edited-preference', '{"id":"person-b5-riley","kind":"person"}', 'attendee_context', 'summary', NULL, 'Riley Rivera asked to start with a written agenda and confirm next ownership.', 'dedup-src-b5-user-edited-preference', 'hash-src-b5-user-edited-preference', 'user:fixture', 'user', 'meeting-b5-first-person', '2026-05-05T16:00:00Z', '2026-05-05T16:00:00Z', '2026-05-05T16:00:00Z', '{}', '{"scenario":"user_edited_supersession","supersedes":"src-b5-original-preference"}', 'active', 'active', NULL, NULL, NULL, NULL, NULL, 0.96, NULL, NULL, NULL, 'state', 'internal', 'active', NULL, NULL),
    ('src-b5-agenda-dup-canonical', '{"id":"person-b5-riley","kind":"person"}', 'attendee_context', 'summary', NULL, 'Riley wants onboarding steps summarized before follow-up.', 'dedup-src-b5-onboarding-summary', 'hash-src-b5-onboarding-summary', 'agent:fixture', 'transcript', 'meeting-b5-first-person', '2026-05-05T16:20:00Z', '2026-05-05T16:20:00Z', '2026-05-05T16:20:00Z', '{}', '{"scenario":"duplicate_paraphrase_collapsed","collapsed_pair":"corro-b5-agenda-paraphrase"}', 'active', 'active', NULL, NULL, NULL, NULL, NULL, 0.82, NULL, NULL, NULL, 'state', 'internal', 'active', NULL, NULL),
    ('src-b5-expired-dormant-open-loop', '{"id":"meeting-b5-first-person","kind":"meeting"}', 'open_loop', 'follow_up', NULL, 'Send Riley the old onboarding checklist before the call.', 'dedup-src-b5-expired-dormant-open-loop', 'hash-src-b5-expired-dormant-open-loop', 'agent:fixture', 'email', 'meeting-b5-first-person', '2026-04-20T12:00:00Z', '2026-04-20T12:00:00Z', '2026-04-20T12:00:00Z', '{}', '{"scenario":"expired_dormant_no_resurrection","double_refresh_expected":"stays_dormant_after_two_prepare_meeting_runs"}', 'dormant', 'dormant', 'expired_before_meeting_refresh', NULL, 'expired', '2026-05-02T00:00:00Z', NULL, 0.35, NULL, NULL, NULL, 'state', 'internal', 'active', NULL, NULL);

INSERT INTO claim_corroborations (
    id, claim_id, data_source, source_asof, source_mechanism,
    strength, reinforcement_count, last_reinforced_at, created_at
)
VALUES (
    'corro-b5-agenda-paraphrase',
    'src-b5-agenda-dup-canonical',
    'email',
    '2026-05-05T17:00:00Z',
    'paraphrase_duplicate',
    0.73,
    1,
    '2026-05-05T17:00:00Z',
    '2026-05-05T17:00:00Z'
);

INSERT INTO claim_feedback (
    id, claim_id, feedback_type, actor, actor_id, payload_json,
    submitted_at, applied_at
)
VALUES
    ('feedback-b5-wrong-subject-riley', 'src-b5-wrong-attendee-original', 'wrong_subject', 'user', NULL, '{"reason":"claim_belongs_to_account_not_attendee","corrected_subject":{"kind":"account","id":"acct-b5-example-portfolio"}}', '2026-05-05T14:30:00Z', '2026-05-05T14:30:00Z'),
    ('feedback-b5-user-edit-preference', 'src-b5-original-preference', 'needs_nuance', 'user', NULL, '{"corrected_claim_id":"src-b5-user-edited-preference","reason":"user_edited_claim_takes_precedence"}', '2026-05-05T16:00:00Z', '2026-05-05T16:00:00Z'),
    ('feedback-b5-expired-dormant-refresh-guard', 'src-b5-expired-dormant-open-loop', 'mark_outdated', 'user', NULL, '{"reason":"expired_before_meeting_refresh","double_refresh_expected":"do_not_reactivate"}', '2026-05-05T18:00:00Z', '2026-05-05T18:00:00Z');

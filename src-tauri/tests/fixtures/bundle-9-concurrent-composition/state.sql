CREATE TABLE daily_readiness_fixture_invocations (
    invocation_kind TEXT NOT NULL,
    workspace_scope TEXT NOT NULL,
    user_id TEXT NOT NULL,
    requested_at TEXT NOT NULL
);

CREATE TABLE daily_readiness_fixture_children (
    workspace_scope TEXT NOT NULL,
    child_ability TEXT NOT NULL,
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    cache_dedupe_key TEXT NOT NULL
);

INSERT INTO daily_readiness_fixture_invocations VALUES
    ('refresh', 'ws-alpha', 'user-alpha', '2026-05-14T12:00:00Z'),
    ('retry', 'ws-alpha', 'user-alpha', '2026-05-14T12:00:00Z'),
    ('scheduled', 'ws-alpha', 'user-alpha', '2026-05-14T12:00:00Z'),
    ('scheduled', 'ws-beta', 'user-alpha', '2026-05-14T12:01:00Z');

INSERT INTO daily_readiness_fixture_children VALUES
    ('ws-alpha', 'prepare_meeting', 'meeting', 'meeting-alpha-readiness', 'prepare_meeting:ws-alpha:meeting-alpha-readiness'),
    ('ws-alpha', 'get_entity_context', 'account', 'acct-shared', 'get_entity_context:ws-alpha:account:acct-shared'),
    ('ws-beta', 'prepare_meeting', 'meeting', 'meeting-beta-readiness', 'prepare_meeting:ws-beta:meeting-beta-readiness'),
    ('ws-beta', 'get_entity_context', 'account', 'acct-shared', 'get_entity_context:ws-beta:account:acct-shared');

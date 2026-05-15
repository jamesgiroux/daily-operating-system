CREATE TABLE daily_readiness_fixture_subjects (
    workspace_scope TEXT NOT NULL,
    kind TEXT NOT NULL,
    id TEXT NOT NULL,
    display_name TEXT NOT NULL
);

CREATE TABLE daily_readiness_fixture_meetings (
    workspace_scope TEXT NOT NULL,
    id TEXT NOT NULL,
    title TEXT NOT NULL,
    starts_at TEXT NOT NULL,
    ends_at TEXT NOT NULL
);

CREATE TABLE daily_readiness_fixture_risks (
    workspace_scope TEXT NOT NULL,
    id TEXT NOT NULL,
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    direction TEXT NOT NULL,
    evidence_summary TEXT NOT NULL,
    source_ref TEXT NOT NULL
);

INSERT INTO daily_readiness_fixture_subjects VALUES
    ('ws-alpha', 'account', 'acct-alpha', 'Account Alpha'),
    ('ws-beta', 'account', 'acct-beta', 'Account Beta');

INSERT INTO daily_readiness_fixture_meetings VALUES
    ('ws-alpha', 'meeting-alpha-readiness', 'Account Alpha readiness review', '2026-05-14T14:00:00Z', '2026-05-14T14:30:00Z'),
    ('ws-beta', 'meeting-beta-expansion', 'Account Beta expansion review', '2026-05-14T15:00:00Z', '2026-05-14T15:30:00Z');

INSERT INTO daily_readiness_fixture_risks VALUES
    ('ws-alpha', 'risk-alpha-renewal', 'account', 'acct-alpha', 'up', 'Account Alpha renewal risk increased after the sponsor missed yesterday''s security review.', 'src-alpha-risk'),
    ('ws-beta', 'risk-beta-expansion', 'account', 'acct-beta', 'up', 'Workspace Beta expansion risk should stay isolated from Account Alpha.', 'src-beta-risk');

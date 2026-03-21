-- I555: Captures metadata enrichment + interaction dynamics persistence
-- Adds urgency/sub_type/impact/evidence_quote/speaker columns to captures.
-- Creates per-meeting tables for interaction dynamics, champion health, and role changes.
-- Note: captured_commitments and success_plan_signals_json already exist from migration 068.

-- Step 1: Ensure captures exists for legacy bootstrap paths.
CREATE TABLE IF NOT EXISTS captures (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    meeting_title TEXT NOT NULL,
    account_id TEXT,
    project_id TEXT,
    capture_type TEXT CHECK(capture_type IN ('win', 'risk', 'action', 'decision')) NOT NULL,
    content TEXT NOT NULL,
    owner TEXT,
    due_date TEXT,
    captured_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Step 2: Per-meeting interaction dynamics
CREATE TABLE IF NOT EXISTS meeting_interaction_dynamics (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    talk_balance_customer_pct INTEGER,
    talk_balance_internal_pct INTEGER,
    speaker_sentiments_json TEXT,
    question_density TEXT,
    decision_maker_active TEXT,
    forward_looking TEXT,
    monologue_risk INTEGER DEFAULT 0,
    competitor_mentions_json TEXT,
    escalation_language_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Step 3: Per-meeting champion health assessment
CREATE TABLE IF NOT EXISTS meeting_champion_health (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    champion_name TEXT,
    champion_status TEXT NOT NULL CHECK(champion_status IN ('strong', 'weak', 'lost', 'none')),
    champion_evidence TEXT,
    champion_risk TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Step 4: Per-meeting role changes
CREATE TABLE IF NOT EXISTS meeting_role_changes (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    person_name TEXT NOT NULL,
    old_status TEXT,
    new_status TEXT,
    evidence_quote TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_meeting_role_changes_meeting
    ON meeting_role_changes(meeting_id);

-- Step 5: Add commitment capture_type to CHECK constraint
-- The captures table uses CHECK(capture_type IN ('win', 'risk', 'action', 'decision'))
-- SQLite requires table rebuild to add 'commitment' to the CHECK.
-- However, the existing CHECK may not be enforced in all code paths,
-- and adding a commitment via INSERT may work without the CHECK change.
-- To be safe, rebuild the table:

CREATE TABLE captures_new (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    meeting_title TEXT NOT NULL,
    account_id TEXT,
    project_id TEXT,
    capture_type TEXT CHECK(capture_type IN ('win', 'risk', 'action', 'decision', 'commitment')),
    content TEXT NOT NULL,
    owner TEXT,
    due_date TEXT,
    captured_at TEXT NOT NULL DEFAULT (datetime('now')),
    sub_type TEXT,
    urgency TEXT,
    impact TEXT,
    evidence_quote TEXT,
    speaker TEXT
);

INSERT INTO captures_new (id, meeting_id, meeting_title, account_id, project_id, capture_type, content, owner, due_date, captured_at)
    SELECT id, meeting_id, meeting_title, account_id, project_id, capture_type, content, owner, due_date, captured_at
    FROM captures;

DROP TABLE captures;
ALTER TABLE captures_new RENAME TO captures;

CREATE INDEX IF NOT EXISTS idx_captures_meeting ON captures(meeting_id);
CREATE INDEX IF NOT EXISTS idx_captures_account ON captures(account_id);
CREATE INDEX IF NOT EXISTS idx_captures_type ON captures(capture_type, urgency);

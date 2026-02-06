-- DailyOS Actions State Management
-- Location: ~/.dailyos/actions.db
-- This file is embedded via include_str! and executed on DB open.

CREATE TABLE IF NOT EXISTS actions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    priority TEXT CHECK(priority IN ('P1', 'P2', 'P3')) DEFAULT 'P2',
    status TEXT CHECK(status IN ('pending', 'completed', 'waiting', 'cancelled')) DEFAULT 'pending',
    created_at TEXT NOT NULL,
    due_date TEXT,
    completed_at TEXT,
    account_id TEXT,
    project_id TEXT,
    source_type TEXT,
    source_id TEXT,
    source_label TEXT,
    context TEXT,
    waiting_on TEXT,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_actions_status ON actions(status);
CREATE INDEX IF NOT EXISTS idx_actions_due_date ON actions(due_date);
CREATE INDEX IF NOT EXISTS idx_actions_account ON actions(account_id);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    ring INTEGER CHECK(ring BETWEEN 1 AND 4),
    arr REAL,
    health TEXT CHECK(health IN ('green', 'yellow', 'red')),
    contract_start TEXT,
    contract_end TEXT,
    csm TEXT,
    champion TEXT,
    tracker_path TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS meetings_history (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    meeting_type TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT,
    account_id TEXT,
    attendees TEXT,
    notes_path TEXT,
    summary TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_meetings_account ON meetings_history(account_id);
CREATE INDEX IF NOT EXISTS idx_meetings_start ON meetings_history(start_time);

CREATE TABLE IF NOT EXISTS processing_log (
    id TEXT PRIMARY KEY,
    filename TEXT NOT NULL,
    source_path TEXT NOT NULL,
    destination_path TEXT,
    classification TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    processed_at TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_processing_status ON processing_log(status);
CREATE INDEX IF NOT EXISTS idx_processing_created ON processing_log(created_at);

-- Post-meeting captures (wins, risks from capture prompts)
CREATE TABLE IF NOT EXISTS captures (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    meeting_title TEXT NOT NULL,
    account_id TEXT,
    capture_type TEXT CHECK(capture_type IN ('win', 'risk', 'action')) NOT NULL,
    content TEXT NOT NULL,
    owner TEXT,
    due_date TEXT,
    captured_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_captures_meeting ON captures(meeting_id);
CREATE INDEX IF NOT EXISTS idx_captures_account ON captures(account_id);
CREATE INDEX IF NOT EXISTS idx_captures_type ON captures(capture_type);

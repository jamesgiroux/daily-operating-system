-- DailyOS Schema Baseline (v1)
-- Consolidates schema.sql + all inline ALTER TABLE migrations from db.rs.
-- This migration creates the complete schema for a fresh database.
-- For existing databases, the bootstrap function marks v1 as applied (this SQL never runs).

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
    updated_at TEXT NOT NULL,
    person_id TEXT,
    needs_decision INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_actions_status ON actions(status);
CREATE INDEX IF NOT EXISTS idx_actions_due_date ON actions(due_date);
CREATE INDEX IF NOT EXISTS idx_actions_account ON actions(account_id);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    lifecycle TEXT,
    arr REAL,
    health TEXT CHECK(health IN ('green', 'yellow', 'red')),
    contract_start TEXT,
    contract_end TEXT,
    csm TEXT,
    champion TEXT,
    nps INTEGER,
    tracker_path TEXT,
    parent_id TEXT,
    updated_at TEXT NOT NULL,
    archived INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_accounts_parent ON accounts(parent_id);
CREATE INDEX IF NOT EXISTS idx_accounts_archived ON accounts(archived);

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT DEFAULT 'active',
    milestone TEXT,
    owner TEXT,
    target_date TEXT,
    tracker_path TEXT,
    updated_at TEXT NOT NULL,
    archived INTEGER DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);
CREATE INDEX IF NOT EXISTS idx_projects_archived ON projects(archived);

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
    created_at TEXT NOT NULL,
    calendar_event_id TEXT,
    prep_context_json TEXT,
    description TEXT,
    user_agenda_json TEXT,
    user_notes TEXT,
    prep_frozen_json TEXT,
    prep_frozen_at TEXT,
    prep_snapshot_path TEXT,
    prep_snapshot_hash TEXT,
    transcript_path TEXT,
    transcript_processed_at TEXT
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

CREATE INDEX IF NOT EXISTS idx_captures_meeting ON captures(meeting_id);
CREATE INDEX IF NOT EXISTS idx_captures_account ON captures(account_id);
CREATE INDEX IF NOT EXISTS idx_captures_type ON captures(capture_type);

CREATE TABLE IF NOT EXISTS entities (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    entity_type TEXT NOT NULL DEFAULT 'account',
    tracker_path TEXT,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);

CREATE TABLE IF NOT EXISTS meeting_prep_state (
    prep_file TEXT PRIMARY KEY,
    calendar_event_id TEXT,
    reviewed_at TEXT NOT NULL,
    title TEXT
);
CREATE INDEX IF NOT EXISTS idx_prep_state_event ON meeting_prep_state(calendar_event_id);

CREATE TABLE IF NOT EXISTS people (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    organization TEXT,
    role TEXT,
    relationship TEXT CHECK(relationship IN ('internal', 'external', 'unknown'))
        DEFAULT 'unknown',
    notes TEXT,
    tracker_path TEXT,
    last_seen TEXT,
    first_seen TEXT,
    meeting_count INTEGER DEFAULT 0,
    updated_at TEXT NOT NULL,
    archived INTEGER DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_people_email ON people(email);
CREATE INDEX IF NOT EXISTS idx_people_relationship ON people(relationship);
CREATE INDEX IF NOT EXISTS idx_people_archived ON people(archived);

CREATE TABLE IF NOT EXISTS meeting_attendees (
    meeting_id TEXT NOT NULL,
    person_id TEXT NOT NULL,
    PRIMARY KEY (meeting_id, person_id)
);
CREATE INDEX IF NOT EXISTS idx_attendees_person ON meeting_attendees(person_id);

CREATE TABLE IF NOT EXISTS entity_people (
    entity_id TEXT NOT NULL,
    person_id TEXT NOT NULL,
    relationship_type TEXT DEFAULT 'associated',
    PRIMARY KEY (entity_id, person_id)
);
CREATE INDEX IF NOT EXISTS idx_entity_people_person ON entity_people(person_id);

CREATE TABLE IF NOT EXISTS meeting_entities (
    meeting_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL DEFAULT 'account',
    PRIMARY KEY (meeting_id, entity_id)
);
CREATE INDEX IF NOT EXISTS idx_meeting_entities_entity ON meeting_entities(entity_id);

CREATE TABLE IF NOT EXISTS content_index (
    id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL DEFAULT 'account',
    filename TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    absolute_path TEXT NOT NULL,
    format TEXT NOT NULL,
    file_size INTEGER NOT NULL DEFAULT 0,
    modified_at TEXT NOT NULL,
    indexed_at TEXT NOT NULL,
    extracted_at TEXT,
    summary TEXT,
    content_type TEXT NOT NULL DEFAULT 'general',
    priority INTEGER NOT NULL DEFAULT 5
);
CREATE INDEX IF NOT EXISTS idx_content_entity ON content_index(entity_id);
CREATE INDEX IF NOT EXISTS idx_content_modified ON content_index(modified_at);

CREATE TABLE IF NOT EXISTS entity_intelligence (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL DEFAULT 'account',
    enriched_at TEXT,
    source_file_count INTEGER DEFAULT 0,
    executive_assessment TEXT,
    risks_json TEXT,
    recent_wins_json TEXT,
    current_state_json TEXT,
    stakeholder_insights_json TEXT,
    next_meeting_readiness_json TEXT,
    company_context_json TEXT
);

CREATE TABLE IF NOT EXISTS account_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK(event_type IN ('renewal', 'expansion', 'churn', 'downgrade')),
    event_date TEXT NOT NULL,
    arr_impact REAL,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_account_events_account ON account_events(account_id);
CREATE INDEX IF NOT EXISTS idx_account_events_date ON account_events(event_date);

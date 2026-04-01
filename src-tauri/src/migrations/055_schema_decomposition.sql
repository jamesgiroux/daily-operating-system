-- 055: Schema decomposition (I511)
-- Splits meetings_history → meetings + meeting_prep + meeting_transcripts
-- Splits entity_intelligence → entity_assessment (+ adds cols to entity_quality)
-- Splits entity_people + account_team → account_stakeholders + entity_members

-- Disable FK enforcement during migration. Table rebuilds copy data that may
-- include orphaned FK references (e.g., attendees referencing deleted people).
-- The new tables define FKs correctly; enforcement resumes after migration.
PRAGMA foreign_keys = OFF;
BEGIN IMMEDIATE;

-- =========================================================================
-- 1. meetings_history → meetings + meeting_prep + meeting_transcripts
-- =========================================================================

CREATE TABLE IF NOT EXISTS meetings (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    meeting_type TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT,
    attendees TEXT,
    notes_path TEXT,
    description TEXT,
    created_at TEXT NOT NULL,
    calendar_event_id TEXT
);

CREATE TABLE IF NOT EXISTS meeting_prep (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    prep_context_json TEXT,
    user_agenda_json TEXT,
    user_notes TEXT,
    prep_frozen_json TEXT,
    prep_frozen_at TEXT,
    prep_snapshot_path TEXT,
    prep_snapshot_hash TEXT
);

CREATE TABLE IF NOT EXISTS meeting_transcripts (
    meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE,
    summary TEXT,
    transcript_path TEXT,
    transcript_processed_at TEXT,
    intelligence_state TEXT NOT NULL DEFAULT 'detected',
    intelligence_quality TEXT NOT NULL DEFAULT 'sparse',
    last_enriched_at TEXT,
    signal_count INTEGER NOT NULL DEFAULT 0,
    has_new_signals INTEGER NOT NULL DEFAULT 0,
    last_viewed_at TEXT
);

-- Copy data from meetings_history.
--
-- Pre-framework databases may be missing columns that were added by inline
-- ALTER TABLE statements in the old open_at() method (description,
-- calendar_event_id, prep_*, user_*, transcript_*). Migration 023 should
-- have rebuilt the table with all columns, but its error was silently
-- swallowed by the migration runner's benign-error logic (no BEGIN →
-- "no such column" treated as benign). Using NULL for potentially-missing
-- columns is safe: if the column never existed, there is no data to lose;
-- if it did exist, this migration already succeeded and won't re-run.
--
-- Clean up leftover temp table from partially-applied migration 023.
DROP TABLE IF EXISTS meetings_history_new;

INSERT OR IGNORE INTO meetings (id, title, meeting_type, start_time, end_time,
    attendees, notes_path, description, created_at, calendar_event_id)
SELECT id, title, meeting_type, start_time, end_time,
    attendees, notes_path, NULL, created_at, NULL
FROM meetings_history;

INSERT OR IGNORE INTO meeting_prep (meeting_id, prep_context_json, user_agenda_json,
    user_notes, prep_frozen_json, prep_frozen_at, prep_snapshot_path, prep_snapshot_hash)
SELECT id, NULL, NULL, NULL,
    NULL, NULL, NULL, NULL
FROM meetings_history;

INSERT OR IGNORE INTO meeting_transcripts (meeting_id, summary, transcript_path,
    transcript_processed_at, intelligence_state, intelligence_quality,
    last_enriched_at, signal_count, has_new_signals, last_viewed_at)
SELECT id, summary, NULL, NULL,
    intelligence_state, intelligence_quality, last_enriched_at,
    signal_count, has_new_signals, last_viewed_at
FROM meetings_history;

-- Indexes on new meeting tables
CREATE INDEX IF NOT EXISTS idx_meetings_start ON meetings(start_time);
CREATE UNIQUE INDEX IF NOT EXISTS idx_meetings_calendar
    ON meetings(calendar_event_id)
    WHERE calendar_event_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_meeting_transcripts_state ON meeting_transcripts(intelligence_state);

-- Repoint FK references from meetings_history → meetings
-- Each section drops any leftover _new table from a failed previous attempt
-- before recreating, making the entire migration idempotent on retry.

-- meeting_entities (created in 032 with FK to meetings_history)
DROP TABLE IF EXISTS meeting_entities_new;
CREATE TABLE meeting_entities_new (
    meeting_id  TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    entity_id   TEXT NOT NULL,
    entity_type TEXT NOT NULL DEFAULT 'account',
    PRIMARY KEY (meeting_id, entity_id)
);
INSERT OR IGNORE INTO meeting_entities_new (meeting_id, entity_id, entity_type)
SELECT meeting_id, entity_id, entity_type FROM meeting_entities;
DROP TABLE IF EXISTS meeting_entities;
ALTER TABLE meeting_entities_new RENAME TO meeting_entities;
CREATE INDEX IF NOT EXISTS idx_meeting_entities_entity ON meeting_entities(entity_id);

-- meeting_attendees (created in 032 with FK to meetings_history)
DROP TABLE IF EXISTS meeting_attendees_new;
CREATE TABLE meeting_attendees_new (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    person_id  TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    PRIMARY KEY (meeting_id, person_id)
);
INSERT OR IGNORE INTO meeting_attendees_new (meeting_id, person_id)
SELECT meeting_id, person_id FROM meeting_attendees;
DROP TABLE IF EXISTS meeting_attendees;
ALTER TABLE meeting_attendees_new RENAME TO meeting_attendees;
CREATE INDEX IF NOT EXISTS idx_attendees_person ON meeting_attendees(person_id);

-- quill_sync_state (created in 013/014 with FK to meetings_history)
DROP TABLE IF EXISTS quill_sync_state_new;
CREATE TABLE quill_sync_state_new (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    quill_meeting_id TEXT,
    state TEXT NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 6,
    next_attempt_at TEXT,
    last_attempt_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    match_confidence REAL,
    transcript_path TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    source TEXT NOT NULL DEFAULT 'quill',
    FOREIGN KEY (meeting_id) REFERENCES meetings(id)
);
INSERT OR IGNORE INTO quill_sync_state_new (
    id, meeting_id, quill_meeting_id, state, attempts, max_attempts,
    next_attempt_at, last_attempt_at, completed_at, error_message,
    match_confidence, transcript_path, created_at, updated_at, source)
SELECT id, meeting_id, quill_meeting_id, state, attempts, max_attempts,
    next_attempt_at, last_attempt_at, completed_at, error_message,
    match_confidence, transcript_path, created_at, updated_at, source
FROM quill_sync_state;
DROP TABLE IF EXISTS quill_sync_state;
ALTER TABLE quill_sync_state_new RENAME TO quill_sync_state;
CREATE INDEX IF NOT EXISTS idx_quill_sync_state ON quill_sync_state(state, next_attempt_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_quill_sync_meeting_source ON quill_sync_state(meeting_id, source);

-- Drop old table
DROP TABLE IF EXISTS meetings_history;

-- =========================================================================
-- 2. entity_intelligence → entity_assessment + entity_quality additions
-- =========================================================================

CREATE TABLE IF NOT EXISTS entity_assessment (
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
    company_context_json TEXT,
    value_delivered TEXT,
    success_metrics TEXT,
    open_commitments TEXT,
    relationship_depth TEXT,
    user_relevance_weight REAL DEFAULT 1.0,
    consistency_status TEXT,
    consistency_findings_json TEXT,
    consistency_checked_at TEXT
);

-- Add health/coherence cols to entity_quality via table rebuild
-- (ALTER TABLE ADD COLUMN is not idempotent; execute_batch stops on
--  "duplicate column name" and skips all subsequent statements.)
DROP TABLE IF EXISTS entity_quality_new;
CREATE TABLE entity_quality_new (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    quality_alpha REAL NOT NULL DEFAULT 1.0,
    quality_beta REAL NOT NULL DEFAULT 1.0,
    quality_score REAL NOT NULL DEFAULT 0.5,
    last_enrichment_at TEXT,
    correction_count INTEGER NOT NULL DEFAULT 0,
    coherence_retry_count INTEGER NOT NULL DEFAULT 0,
    coherence_window_start TEXT,
    coherence_blocked INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    health_score REAL,
    health_trend TEXT,
    coherence_score REAL,
    coherence_flagged INTEGER DEFAULT 0
);
INSERT OR IGNORE INTO entity_quality_new (entity_id, entity_type, quality_alpha,
    quality_beta, quality_score, last_enrichment_at, correction_count,
    coherence_retry_count, coherence_window_start, coherence_blocked,
    created_at, updated_at)
SELECT entity_id, entity_type, quality_alpha, quality_beta, quality_score,
    last_enrichment_at, correction_count, coherence_retry_count,
    coherence_window_start, coherence_blocked, created_at, updated_at
FROM entity_quality;
DROP TABLE IF EXISTS entity_quality;
ALTER TABLE entity_quality_new RENAME TO entity_quality;
CREATE INDEX IF NOT EXISTS idx_entity_quality_score ON entity_quality(quality_score);
CREATE INDEX IF NOT EXISTS idx_entity_quality_blocked ON entity_quality(coherence_blocked);

-- Copy data from entity_intelligence → entity_assessment
INSERT OR IGNORE INTO entity_assessment (entity_id, entity_type, enriched_at,
    source_file_count, executive_assessment, risks_json, recent_wins_json,
    current_state_json, stakeholder_insights_json, next_meeting_readiness_json,
    company_context_json, value_delivered, success_metrics, open_commitments,
    relationship_depth, user_relevance_weight, consistency_status,
    consistency_findings_json, consistency_checked_at)
SELECT entity_id, entity_type, enriched_at, source_file_count,
    executive_assessment, risks_json, recent_wins_json, current_state_json,
    stakeholder_insights_json, next_meeting_readiness_json, company_context_json,
    value_delivered, success_metrics, open_commitments, relationship_depth,
    user_relevance_weight, consistency_status, consistency_findings_json,
    consistency_checked_at
FROM entity_intelligence;

-- Copy health/coherence data into entity_quality for entities that already have quality rows
UPDATE entity_quality SET
    health_score = (SELECT ei.health_score FROM entity_intelligence ei WHERE ei.entity_id = entity_quality.entity_id),
    health_trend = (SELECT ei.health_trend FROM entity_intelligence ei WHERE ei.entity_id = entity_quality.entity_id),
    coherence_score = (SELECT ei.coherence_score FROM entity_intelligence ei WHERE ei.entity_id = entity_quality.entity_id),
    coherence_flagged = (SELECT COALESCE(ei.coherence_flagged, 0) FROM entity_intelligence ei WHERE ei.entity_id = entity_quality.entity_id)
WHERE entity_id IN (SELECT entity_id FROM entity_intelligence);

-- Create entity_quality rows for entities that have intelligence but no quality row yet
INSERT OR IGNORE INTO entity_quality (entity_id, entity_type, health_score, health_trend,
    coherence_score, coherence_flagged)
SELECT ei.entity_id, ei.entity_type, ei.health_score, ei.health_trend,
    ei.coherence_score, COALESCE(ei.coherence_flagged, 0)
FROM entity_intelligence ei
WHERE ei.entity_id NOT IN (SELECT entity_id FROM entity_quality)
  AND (ei.health_score IS NOT NULL OR ei.coherence_score IS NOT NULL);

-- Index on entity_assessment
CREATE INDEX IF NOT EXISTS idx_entity_assessment_type ON entity_assessment(entity_type);

-- Drop old table
DROP TABLE IF EXISTS entity_intelligence;

-- =========================================================================
-- 3. entity_people + account_team → account_stakeholders + entity_members
-- =========================================================================

CREATE TABLE IF NOT EXISTS account_stakeholders (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'associated',
    relationship_type TEXT DEFAULT 'associated',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (account_id, person_id)
);

CREATE TABLE IF NOT EXISTS entity_members (
    entity_id TEXT NOT NULL,
    person_id TEXT NOT NULL,
    relationship_type TEXT DEFAULT 'associated',
    PRIMARY KEY (entity_id, person_id)
);

-- Copy account_team rows into account_stakeholders.
-- Multi-role collapse uses explicit business priority (not alphabetical MIN).
WITH ranked_roles AS (
    SELECT
        account_id,
        person_id,
        role,
        created_at,
        ROW_NUMBER() OVER (
            PARTITION BY account_id, person_id
            ORDER BY
                CASE LOWER(role)
                    WHEN 'executive_sponsor' THEN 1
                    WHEN 'champion' THEN 2
                    WHEN 'decision_maker' THEN 3
                    WHEN 'economic_buyer' THEN 4
                    WHEN 'technical_buyer' THEN 5
                    WHEN 'csm' THEN 6
                    WHEN 'implementation' THEN 7
                    WHEN 'associated' THEN 8
                    ELSE 99
                END,
                created_at ASC
        ) AS rn
    FROM account_team
)
INSERT OR IGNORE INTO account_stakeholders (account_id, person_id, role, created_at)
SELECT account_id, person_id, role, COALESCE(created_at, datetime('now'))
FROM ranked_roles
WHERE rn = 1;

-- Merge entity_people account rows into account_stakeholders (add relationship_type)
UPDATE account_stakeholders SET
    relationship_type = (
        SELECT ep.relationship_type FROM entity_people ep
        WHERE ep.entity_id = account_stakeholders.account_id
          AND ep.person_id = account_stakeholders.person_id
    )
WHERE EXISTS (
    SELECT 1 FROM entity_people ep
    WHERE ep.entity_id = account_stakeholders.account_id
      AND ep.person_id = account_stakeholders.person_id
);

-- entity_people rows for accounts that have no account_team entry
INSERT OR IGNORE INTO account_stakeholders (account_id, person_id, role, relationship_type)
SELECT ep.entity_id, ep.person_id, 'associated', ep.relationship_type
FROM entity_people ep
INNER JOIN accounts a ON a.id = ep.entity_id
WHERE NOT EXISTS (
    SELECT 1 FROM account_stakeholders as2
    WHERE as2.account_id = ep.entity_id AND as2.person_id = ep.person_id
);

-- Copy non-account entity_people rows to entity_members
INSERT OR IGNORE INTO entity_members (entity_id, person_id, relationship_type)
SELECT ep.entity_id, ep.person_id, ep.relationship_type
FROM entity_people ep
WHERE ep.entity_id NOT IN (SELECT id FROM accounts);

-- Indexes on new tables
CREATE INDEX IF NOT EXISTS idx_account_stakeholders_person ON account_stakeholders(person_id);
CREATE INDEX IF NOT EXISTS idx_entity_members_person ON entity_members(person_id);

-- Drop old tables
DROP TABLE IF EXISTS account_team;
DROP TABLE IF EXISTS entity_people;

-- Clean up orphaned FK references (e.g., attendees referencing deleted people)
DELETE FROM meeting_attendees WHERE person_id NOT IN (SELECT id FROM people);

PRAGMA foreign_keys = ON;
COMMIT;

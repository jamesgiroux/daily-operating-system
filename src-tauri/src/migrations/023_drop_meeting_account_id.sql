-- Migration 023: Drop legacy account_id column from meetings_history.
--
-- The account_id column is replaced by the meeting_entities junction table (I52).
-- Before dropping, backfill any orphaned account_id values into meeting_entities.

-- Step 1: Backfill meeting_entities from existing account_id values.
-- Join accounts table to find the matching entity ID for the account name stored
-- in account_id (which may be either a slug ID or a display name).
INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type)
SELECT mh.id, a.id, 'account'
FROM meetings_history mh
JOIN accounts a ON a.id = mh.account_id OR LOWER(a.name) = LOWER(mh.account_id)
WHERE mh.account_id IS NOT NULL AND mh.account_id != ''
  AND NOT EXISTS (
    SELECT 1 FROM meeting_entities me
    WHERE me.meeting_id = mh.id AND me.entity_id = a.id
  );

-- Step 2: Recreate meetings_history without account_id column.
CREATE TABLE meetings_history_new (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    meeting_type TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT,
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

-- Step 3: Copy data (excluding account_id).
INSERT INTO meetings_history_new (
    id, title, meeting_type, start_time, end_time,
    attendees, notes_path, summary, created_at, calendar_event_id,
    prep_context_json, description, user_agenda_json, user_notes,
    prep_frozen_json, prep_frozen_at, prep_snapshot_path, prep_snapshot_hash,
    transcript_path, transcript_processed_at
)
SELECT
    id, title, meeting_type, start_time, end_time,
    attendees, notes_path, summary, created_at, calendar_event_id,
    prep_context_json, description, user_agenda_json, user_notes,
    prep_frozen_json, prep_frozen_at, prep_snapshot_path, prep_snapshot_hash,
    transcript_path, transcript_processed_at
FROM meetings_history;

-- Step 4: Drop old table and rename.
DROP TABLE meetings_history;
ALTER TABLE meetings_history_new RENAME TO meetings_history;

-- Step 5: Recreate indexes (without the old idx_meetings_account).
CREATE INDEX IF NOT EXISTS idx_meetings_start ON meetings_history(start_time);

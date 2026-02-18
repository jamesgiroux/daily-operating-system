-- Migration 014: Add source column to quill_sync_state for multi-provider transcript sync.
-- Granola integration (I226) reuses the same sync state table with source='granola'.
-- Must recreate the table to remove the inline UNIQUE constraint on meeting_id,
-- replacing it with a composite unique index on (meeting_id, source).

-- Step 1: Rename old table
ALTER TABLE quill_sync_state RENAME TO quill_sync_state_old;

-- Step 2: Create new table without UNIQUE on meeting_id, with source column
CREATE TABLE quill_sync_state (
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
    FOREIGN KEY (meeting_id) REFERENCES meetings_history(id)
);

-- Step 3: Copy existing data
INSERT INTO quill_sync_state (
    id, meeting_id, quill_meeting_id, state, attempts, max_attempts,
    next_attempt_at, last_attempt_at, completed_at, error_message,
    match_confidence, transcript_path, created_at, updated_at, source
)
SELECT
    id, meeting_id, quill_meeting_id, state, attempts, max_attempts,
    next_attempt_at, last_attempt_at, completed_at, error_message,
    match_confidence, transcript_path, created_at, updated_at, 'quill'
FROM quill_sync_state_old;

-- Step 4: Drop old table
DROP TABLE quill_sync_state_old;

-- Step 5: Recreate indexes
CREATE INDEX IF NOT EXISTS idx_quill_sync_state ON quill_sync_state(state, next_attempt_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_quill_sync_meeting_source ON quill_sync_state(meeting_id, source);

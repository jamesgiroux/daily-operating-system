CREATE TABLE IF NOT EXISTS quill_sync_state (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL UNIQUE,
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
    FOREIGN KEY (meeting_id) REFERENCES meetings_history(id)
);
CREATE INDEX IF NOT EXISTS idx_quill_sync_state ON quill_sync_state(state, next_attempt_at);

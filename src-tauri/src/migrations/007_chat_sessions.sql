-- Sprint 26: chat session persistence

CREATE TABLE IF NOT EXISTS chat_sessions (
    id TEXT PRIMARY KEY,
    entity_id TEXT,
    entity_type TEXT,
    session_start TEXT NOT NULL,
    session_end TEXT,
    turn_count INTEGER DEFAULT 0,
    last_message TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS chat_turns (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    turn_index INTEGER NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sessions_entity ON chat_sessions(entity_id);
CREATE INDEX IF NOT EXISTS idx_turns_session ON chat_turns(session_id);

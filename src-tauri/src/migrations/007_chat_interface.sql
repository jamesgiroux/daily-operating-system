-- Migration 007: Chat interface storage (ADR-0075, Sprint 26)
--
-- Adds chat_sessions and chat_turns tables for conversational interface.
-- Phase 1: External via MCP (Claude Desktop). Phase 2: In-app if validated.
-- Sessions can be entity-scoped (account/project) or general.

CREATE TABLE chat_sessions (
    id TEXT PRIMARY KEY,
    entity_id TEXT,              -- nullable (general chat not tied to entity)
    entity_type TEXT,            -- 'account' | 'project' | NULL
    session_start TEXT NOT NULL,
    session_end TEXT,            -- NULL if active
    turn_count INTEGER DEFAULT 0,
    last_message TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE chat_turns (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    turn_index INTEGER NOT NULL,
    role TEXT NOT NULL,           -- 'user' | 'assistant'
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_sessions_entity ON chat_sessions(entity_id);
CREATE INDEX idx_turns_session ON chat_turns(session_id);

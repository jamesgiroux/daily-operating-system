-- Migration 048: Google Drive sync metadata table
--
-- Stores watched Drive sources and their sync state.
-- Files are downloaded and placed in entity Documents/ folders.

CREATE TABLE IF NOT EXISTS drive_watched_sources (
    id TEXT PRIMARY KEY,
    google_id TEXT NOT NULL,
    name TEXT NOT NULL,
    file_type TEXT NOT NULL DEFAULT 'document',
    google_doc_url TEXT,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    last_synced_at TEXT,
    changes_token TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (entity_id) REFERENCES entity_intel(entity_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_drive_watched_sources_entity
    ON drive_watched_sources(entity_id, entity_type);

CREATE INDEX IF NOT EXISTS idx_drive_watched_sources_google_id
    ON drive_watched_sources(google_id);

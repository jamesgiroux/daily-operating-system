-- Migration 006: Content embeddings for vector search (ADR-0074, Sprint 26)
--
-- Adds content_embeddings table to store chunk-level vector embeddings for
-- semantic search over entity content. One file produces multiple chunks (1:N).
-- Uses nomic-embed-text-v1.5 model (768 dimensions) via fastembed.

CREATE TABLE IF NOT EXISTS content_embeddings (
    id TEXT PRIMARY KEY,
    content_file_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    chunk_text TEXT NOT NULL,
    embedding BLOB NOT NULL,        -- f32 vector, 768 dimensions
    created_at TEXT NOT NULL,
    FOREIGN KEY (content_file_id) REFERENCES content_index(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_embeddings_file ON content_embeddings(content_file_id);

ALTER TABLE content_index ADD COLUMN embeddings_generated_at TEXT;

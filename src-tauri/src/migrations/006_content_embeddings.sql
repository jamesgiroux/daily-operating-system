-- Migration 006: Content embeddings for vector search (ADR-0074, Sprint 26)
--
-- Adds content_embeddings table to store chunk-level vector embeddings for
-- semantic search over entity content. One file produces multiple chunks (1:N).
-- Uses snowflake-arctic-embed-s model (384 dimensions, ~1536 bytes per vector).

CREATE TABLE content_embeddings (
    id TEXT PRIMARY KEY,
    content_file_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    chunk_text TEXT NOT NULL,
    embedding BLOB NOT NULL,        -- f32 vector, 384 dimensions (1536 bytes)
    created_at TEXT NOT NULL,
    FOREIGN KEY (content_file_id) REFERENCES content_index(id) ON DELETE CASCADE
);

CREATE INDEX idx_embeddings_file ON content_embeddings(content_file_id);

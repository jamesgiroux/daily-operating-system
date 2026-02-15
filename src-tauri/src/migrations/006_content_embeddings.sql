-- Sprint 26: semantic content embeddings

CREATE TABLE IF NOT EXISTS content_embeddings (
    id TEXT PRIMARY KEY,
    content_file_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    chunk_text TEXT NOT NULL,
    embedding BLOB NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (content_file_id) REFERENCES content_index(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_embeddings_file ON content_embeddings(content_file_id);

ALTER TABLE content_index ADD COLUMN embeddings_generated_at TEXT;

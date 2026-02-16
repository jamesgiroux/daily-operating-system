-- Migration 009: Ensure embeddings_generated_at column on content_index.
--
-- Fixes databases where migration 006's ALTER TABLE didn't take effect because
-- the DB was created from the baseline schema (which already included content_index
-- but without this column). The migration runner handles the "duplicate column"
-- error gracefully for databases where the column already exists.

ALTER TABLE content_index ADD COLUMN embeddings_generated_at TEXT;

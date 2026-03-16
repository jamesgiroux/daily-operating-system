-- Migration 009: Ensure embeddings_generated_at column on content_index.
--
-- Fixes databases where migration 006's ALTER TABLE didn't take effect because
-- the DB was created from the baseline schema (which already included content_index
-- but without this column).
--
-- This is a single ALTER TABLE ADD COLUMN statement. The migration runner
-- tolerates "duplicate column name" errors specifically for ADD COLUMN
-- migrations — this is the only safe exception to the fail-hard rule,
-- because SQLite lacks ALTER TABLE ADD COLUMN IF NOT EXISTS.

ALTER TABLE content_index ADD COLUMN embeddings_generated_at TEXT;

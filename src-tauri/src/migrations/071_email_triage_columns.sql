-- Migration 071: Email triage and extraction columns
-- Adds pinned_at for triage sort boost, commitments/questions for AI extraction display.

ALTER TABLE emails ADD COLUMN pinned_at TEXT;
ALTER TABLE emails ADD COLUMN commitments TEXT;  -- JSON array of extracted commitments
ALTER TABLE emails ADD COLUMN questions TEXT;    -- JSON array of extracted questions

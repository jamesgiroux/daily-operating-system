-- I636: Add record_path column for structured meeting record markdown files
ALTER TABLE meeting_transcripts ADD COLUMN record_path TEXT;

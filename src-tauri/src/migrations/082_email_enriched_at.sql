-- Email enrichment filtering & workflow optimization.
-- Add enriched_at column to track when emails were last enriched.
-- Gate 0 (primary deduplication) uses this to skip already-enriched emails.

ALTER TABLE emails ADD COLUMN enriched_at DATETIME NULL DEFAULT NULL;

-- Index for fresh-email queries (Gate 0 deduplication + recency filtering)
CREATE INDEX idx_emails_enriched_at ON emails(enriched_at DESC);

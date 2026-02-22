-- I395: Email relevance scoring columns
ALTER TABLE emails ADD COLUMN relevance_score REAL;
ALTER TABLE emails ADD COLUMN score_reason TEXT;

CREATE INDEX IF NOT EXISTS idx_emails_relevance ON emails(relevance_score);

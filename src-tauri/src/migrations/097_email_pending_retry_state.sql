-- DOS-226: Introduce `pending_retry` as a transitional enrichment state to make
-- manual retries rollback-safe.
--
-- Before this change, `retry_failed_emails` reset rows `failed -> pending` with
-- `enrichment_attempts = 0` *before* the Gmail refresh ran. If the refresh
-- itself then failed (e.g. Gmail auth error), the reset was already committed:
-- the UI's "some emails couldn't be processed" notice vanished silently and
-- the rows appeared healthy while enrichment had in fact never re-run.
--
-- The new state vocabulary is:
--   failed         -> terminal: 3 enrichment attempts exhausted, surfaced to user
--   pending_retry  -> user asked to retry; refresh in flight. Still counts as
--                     "failed" for UI purposes so the Retry notice stays visible
--                     until we know the outcome.
--   pending        -> refresh confirmed success; enrichment may re-run.
--
-- On refresh failure we transition `pending_retry` back to `failed` so the
-- user can retry again. On refresh success we transition to `pending` and
-- zero out `enrichment_attempts` so the enrichment pipeline will pick them up.
--
-- SQLite can't ALTER a CHECK constraint in place, so we rebuild the table.
-- The column list matches the accumulated ALTERs from migrations 034, 035,
-- 071, and 082. Defaults and indexes are restored at the end.

-- Disable FK enforcement during the table swap so dependent rows survive.
PRAGMA foreign_keys = OFF;

CREATE TABLE emails_new (
    email_id TEXT PRIMARY KEY,
    thread_id TEXT,
    sender_email TEXT,
    sender_name TEXT,
    subject TEXT,
    snippet TEXT,
    priority TEXT,
    is_unread INTEGER DEFAULT 1,
    received_at TEXT,
    enrichment_state TEXT DEFAULT 'pending' CHECK(enrichment_state IN ('pending', 'pending_retry', 'enriching', 'enriched', 'failed')),
    enrichment_attempts INTEGER DEFAULT 0,
    last_enrichment_at TEXT,
    enriched_at DATETIME,
    last_seen_at TEXT,
    resolved_at TEXT,
    entity_id TEXT,
    entity_type TEXT,
    contextual_summary TEXT,
    sentiment TEXT,
    urgency TEXT,
    user_is_last_sender INTEGER DEFAULT 0,
    last_sender_email TEXT,
    message_count INTEGER DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    relevance_score REAL,
    score_reason TEXT,
    pinned_at TEXT,
    commitments TEXT,
    questions TEXT
);

INSERT INTO emails_new (
    email_id, thread_id, sender_email, sender_name, subject, snippet,
    priority, is_unread, received_at, enrichment_state, enrichment_attempts,
    last_enrichment_at, enriched_at, last_seen_at, resolved_at, entity_id, entity_type,
    contextual_summary, sentiment, urgency, user_is_last_sender, last_sender_email,
    message_count, created_at, updated_at, relevance_score, score_reason,
    pinned_at, commitments, questions
)
SELECT
    email_id, thread_id, sender_email, sender_name, subject, snippet,
    priority, is_unread, received_at, enrichment_state, enrichment_attempts,
    last_enrichment_at, enriched_at, last_seen_at, resolved_at, entity_id, entity_type,
    contextual_summary, sentiment, urgency, user_is_last_sender, last_sender_email,
    message_count, created_at, updated_at, relevance_score, score_reason,
    pinned_at, commitments, questions
FROM emails;

DROP TABLE emails;
ALTER TABLE emails_new RENAME TO emails;

CREATE INDEX IF NOT EXISTS idx_emails_thread_id ON emails(thread_id);
CREATE INDEX IF NOT EXISTS idx_emails_enrichment ON emails(enrichment_state, enrichment_attempts);
CREATE INDEX IF NOT EXISTS idx_emails_entity ON emails(entity_id, entity_type);
CREATE INDEX IF NOT EXISTS idx_emails_priority_resolved ON emails(priority, resolved_at);
CREATE INDEX IF NOT EXISTS idx_emails_last_seen ON emails(last_seen_at);
CREATE INDEX IF NOT EXISTS idx_emails_resolved ON emails(resolved_at);

PRAGMA foreign_keys = ON;

-- Migration 034: Email metadata persistence (I368)
-- Stores email metadata from Gmail inbox for enrichment, reconciliation, and entity linking.

CREATE TABLE IF NOT EXISTS emails (
    email_id TEXT PRIMARY KEY,
    thread_id TEXT,
    sender_email TEXT,
    sender_name TEXT,
    subject TEXT,
    snippet TEXT,
    priority TEXT,
    is_unread INTEGER DEFAULT 1,
    received_at TEXT,
    enrichment_state TEXT DEFAULT 'pending' CHECK(enrichment_state IN ('pending', 'enriching', 'enriched', 'failed')),
    enrichment_attempts INTEGER DEFAULT 0,
    last_enrichment_at TEXT,
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
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_emails_thread_id ON emails(thread_id);
CREATE INDEX IF NOT EXISTS idx_emails_enrichment ON emails(enrichment_state, enrichment_attempts);
CREATE INDEX IF NOT EXISTS idx_emails_entity ON emails(entity_id, entity_type);
CREATE INDEX IF NOT EXISTS idx_emails_priority_resolved ON emails(priority, resolved_at);
CREATE INDEX IF NOT EXISTS idx_emails_last_seen ON emails(last_seen_at);
CREATE INDEX IF NOT EXISTS idx_emails_resolved ON emails(resolved_at);

-- Add deactivated_at column to email_signals for lifecycle tracking
ALTER TABLE email_signals ADD COLUMN deactivated_at TEXT;

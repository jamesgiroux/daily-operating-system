-- Migration 030: Email dismissals for relevance learning.
--
-- When a user dismisses a commitment, question, or reply-needed item from
-- The Correspondent, record the dismissal with enough context for 0.13.0
-- to build a relevance model (sender domains, email types, entity linkage).

CREATE TABLE email_dismissals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- What was dismissed: 'commitment', 'question', 'reply_needed'
    item_type TEXT NOT NULL,
    -- The email this item came from
    email_id TEXT NOT NULL,
    -- Context for relevance learning
    sender_domain TEXT,
    email_type TEXT,
    entity_id TEXT,
    -- The dismissed text (for dedup on re-enrichment)
    item_text TEXT NOT NULL,
    dismissed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_email_dismissals_email ON email_dismissals(email_id);
CREATE INDEX idx_email_dismissals_domain ON email_dismissals(sender_domain);
CREATE INDEX idx_email_dismissals_type ON email_dismissals(item_type, dismissed_at DESC);

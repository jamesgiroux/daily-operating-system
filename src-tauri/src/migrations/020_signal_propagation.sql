-- Migration 020: Signal propagation tables (I308 — ADR-0080 Phase 4)
--
-- signal_derivations: tracks which signals were derived from which source signals
-- post_meeting_emails: correlates emails received after meetings with meeting context
-- briefing_callouts: surfaced intelligence items for the daily briefing

CREATE TABLE signal_derivations (
    id TEXT PRIMARY KEY,
    source_signal_id TEXT NOT NULL,
    derived_signal_id TEXT NOT NULL,
    rule_name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_signal_derivations_source ON signal_derivations(source_signal_id);
CREATE INDEX idx_signal_derivations_derived ON signal_derivations(derived_signal_id);

CREATE TABLE post_meeting_emails (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    email_signal_id TEXT NOT NULL,
    thread_id TEXT,
    correlated_at TEXT NOT NULL DEFAULT (datetime('now')),
    actions_extracted TEXT
);
CREATE INDEX idx_post_meeting_emails_meeting ON post_meeting_emails(meeting_id);

CREATE TABLE briefing_callouts (
    id TEXT PRIMARY KEY,
    signal_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_name TEXT,
    severity TEXT NOT NULL DEFAULT 'info',
    headline TEXT NOT NULL,
    detail TEXT,
    context_json TEXT,
    surfaced_at TEXT,
    dismissed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_briefing_callouts_unsurfaced ON briefing_callouts(surfaced_at, dismissed_at);

-- Add signal_type index for efficient type-based queries (used by callout generation)
CREATE INDEX IF NOT EXISTS idx_signal_events_type ON signal_events(signal_type, created_at DESC);

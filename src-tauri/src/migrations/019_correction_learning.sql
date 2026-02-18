-- Migration 019: Correction Learning (I307 / ADR-0080 Phase 3)
--
-- Feedback table for user corrections, attendee group patterns,
-- and context tagging on signal events.

-- Feedback table: records every user correction
CREATE TABLE entity_resolution_feedback (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    old_entity_id TEXT,
    old_entity_type TEXT,
    new_entity_id TEXT,
    new_entity_type TEXT,
    signal_source TEXT,
    corrected_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_feedback_meeting ON entity_resolution_feedback(meeting_id);
CREATE INDEX idx_feedback_source ON entity_resolution_feedback(signal_source, corrected_at DESC);

-- Attendee group patterns: learned co-occurrence → entity mappings
CREATE TABLE attendee_group_patterns (
    group_hash TEXT PRIMARY KEY,
    attendee_emails TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    occurrence_count INTEGER DEFAULT 1,
    last_seen_at TEXT NOT NULL DEFAULT (datetime('now')),
    confidence REAL DEFAULT 0.0
);
CREATE INDEX idx_group_patterns_entity ON attendee_group_patterns(entity_id, entity_type);

-- Context tagging on signals (internal vs external)
ALTER TABLE signal_events ADD COLUMN source_context TEXT;

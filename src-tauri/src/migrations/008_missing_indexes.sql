-- Migration 008: Add missing indexes identified in beta hardening audit (I283)
--
-- meeting_entities(meeting_id): eliminates full table scan on entity detail page loads
-- meetings_history(calendar_event_id): prevents duplicate calendar imports
-- actions(status, due_date): composite index for filtered + sorted action queries

CREATE INDEX IF NOT EXISTS idx_meeting_entities_meeting_id
    ON meeting_entities(meeting_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_meetings_calendar_event_id
    ON meetings_history(calendar_event_id)
    WHERE calendar_event_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_actions_status_due_date
    ON actions(status, due_date);

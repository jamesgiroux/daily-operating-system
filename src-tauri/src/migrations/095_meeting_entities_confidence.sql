-- Add confidence + is_primary columns to meeting_entities.
--
-- Prior behavior: when two accounts (e.g. parent + subsidiary BU) shared a
-- domain, the entity resolver auto-linked BOTH as equally-resolved entities.
-- Meeting briefings then displayed every domain-matched account as an equal
-- primary chip, cluttering the UI with duplicates.
--
-- This migration adds per-junction confidence and an is_primary flag so:
--   - The highest-confidence link per meeting is marked primary.
--   - Lower-confidence links (<0.60) can render as muted "suggestions"
--     rather than co-equal primaries.
--
-- Defaults: existing rows get confidence = 0.95 (same as manual junction
-- matches) and is_primary = 1 (treat prior auto-links as primary) so the
-- migration is backward-compatible for already-linked meetings.

ALTER TABLE meeting_entities ADD COLUMN confidence REAL NOT NULL DEFAULT 0.95;
ALTER TABLE meeting_entities ADD COLUMN is_primary INTEGER NOT NULL DEFAULT 1;

CREATE INDEX IF NOT EXISTS idx_meeting_entities_primary
    ON meeting_entities(meeting_id, is_primary);

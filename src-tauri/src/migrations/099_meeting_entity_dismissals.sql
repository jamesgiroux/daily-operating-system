-- Persistent meeting-entity dismissal dictionary.
--
-- Prior behavior: when a user unlinked an auto-resolved entity from a
-- meeting (e.g., the resolver wrongly linked "Acme Corp" to an internal
-- planning meeting), the next calendar-sync / resolver pass would happily
-- re-link it. Users experienced this as "dismissed entities keep coming
-- back every sync."
--
-- This migration records every dismissal so both the calendar-sync
-- persistence path (`persist_classification_entities*`) and the background
-- scored resolver (`persist_and_invalidate_entity_links_sync_scored`) can
-- short-circuit before re-inserting a link the user already rejected.
--
-- The table is keyed on (meeting_id, entity_id, entity_type) so the same
-- entity can be dismissed independently from multiple meetings. Undo is
-- modeled as deletion of the dismissal row via `restore_meeting_entity`.

CREATE TABLE IF NOT EXISTS meeting_entity_dismissals (
    meeting_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    dismissed_at TEXT NOT NULL,
    dismissed_by TEXT NULL,
    PRIMARY KEY (meeting_id, entity_id, entity_type)
);

CREATE INDEX IF NOT EXISTS idx_meeting_entity_dismissals_meeting
    ON meeting_entity_dismissals(meeting_id);

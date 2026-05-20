-- DOS-335 (v1.4.4 W1): Meeting prep status indexed view.
--
-- Materializes the join across `meetings` + `meeting_entities` +
-- `meeting_prep` so `services::meeting_prep_status::read::compute_status`
-- can compute status without N+1 queries. Reuses existing substrate; no
-- new authoritative tables. Status is computed at query time from this
-- view's columns plus dismissal table (v242) and claim invalidation
-- queue state observed through existing `services::invalidation_jobs`.
--
-- View columns are intentionally non-derived; they expose the raw
-- substrate joined together. The Rust read path applies status logic
-- (BlockedNoEntity, PrepNeeded, Ready, Stale, etc.) on these inputs.
--
-- ADR-0102 §3 (Read-ability call graph): view is read-only. No
-- triggers, no writes. Indexed for fast lookup by meeting_id (PK on
-- meetings) and by start_time for upcoming-window scans.

CREATE VIEW IF NOT EXISTS meeting_prep_status_view AS
SELECT
    m.id                       AS meeting_id,
    m.calendar_event_id        AS event_id,
    m.title                    AS meeting_title,
    m.start_time               AS start_time,
    m.end_time                 AS end_time,
    me.entity_id               AS linked_entity_id,
    me.entity_type             AS linked_entity_type,
    mp.prep_context_json       AS prep_context_json,
    mp.user_agenda_json        AS user_agenda_json,
    mp.user_notes              AS user_notes,
    mp.prep_frozen_at          AS last_prepared_at,
    mp.prep_snapshot_hash      AS prep_snapshot_hash
FROM meetings m
LEFT JOIN meeting_entities me ON me.meeting_id = m.id
LEFT JOIN meeting_prep mp    ON mp.meeting_id = m.id;

-- The underlying tables already carry the relevant indexes
-- (meetings.start_time, meeting_entities.meeting_id, meeting_prep PK on
-- meeting_id). No additional indexes are required for the view itself.

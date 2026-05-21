-- Deterministic meeting prep status view.
--
-- The v241 view emits one row per `(meeting, linked_entity)` join. When
-- `meeting_entities` carries multiple links for the same meeting (e.g. a
-- meeting linked to both an Account and a Project), `compute_status` reads
-- `LIMIT 1` without an `ORDER BY`. SQLite is free to return any of the rows,
-- so status derivation is non-deterministic for multi-linked meetings.
--
-- Fix: aggregate `meeting_entities` to exactly ONE row per meeting before the
-- join, picking the lexicographically smallest `(entity_id, entity_type)`
-- pair via per-meeting MIN aggregation so the choice is stable across reads.
-- Future per-meeting status expansion (e.g. multi-binding status) can drop
-- this aggregation; for now, a single deterministic row matches the W1 read
-- contract.

DROP VIEW IF EXISTS meeting_prep_status_view;

CREATE VIEW meeting_prep_status_view AS
WITH primary_entity AS (
    -- One row per meeting: the lexicographically smallest entity_id, and
    -- among rows sharing that entity_id, the lexicographically smallest
    -- entity_type. SQLite's MIN() over a tuple of columns isn't supported,
    -- so we compose two grouped aggregates.
    SELECT
        meeting_id,
        MIN(entity_id) AS entity_id,
        MIN(entity_type) AS entity_type
    FROM (
        SELECT
            me.meeting_id,
            me.entity_id,
            me.entity_type
        FROM meeting_entities me
        INNER JOIN (
            SELECT meeting_id, MIN(entity_id) AS min_entity_id
            FROM meeting_entities
            GROUP BY meeting_id
        ) pick
            ON pick.meeting_id = me.meeting_id
           AND pick.min_entity_id = me.entity_id
    )
    GROUP BY meeting_id
)
SELECT
    m.id                       AS meeting_id,
    m.calendar_event_id        AS event_id,
    m.title                    AS meeting_title,
    m.start_time               AS start_time,
    m.end_time                 AS end_time,
    pe.entity_id               AS linked_entity_id,
    pe.entity_type             AS linked_entity_type,
    mp.prep_context_json       AS prep_context_json,
    mp.user_agenda_json        AS user_agenda_json,
    mp.user_notes              AS user_notes,
    mp.prep_frozen_at          AS last_prepared_at,
    mp.prep_snapshot_hash      AS prep_snapshot_hash
FROM meetings m
LEFT JOIN primary_entity pe ON pe.meeting_id = m.id
LEFT JOIN meeting_prep mp    ON mp.meeting_id = m.id;

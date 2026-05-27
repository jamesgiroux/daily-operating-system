-- Current-graph compatible meeting/entity read view.
--
-- `meeting_entities` is the legacy junction table. The current entity-linking
-- system writes `linked_entities_raw` and exposes non-dismissed rows via the
-- `linked_entities` view. Runtime and surface reads need one stable relation
-- that means:
--   1. if current graph state exists for a meeting, read that graph;
--   2. if current graph state exists only as dismissed rows, return no rows;
--   3. only fall back to `meeting_entities` when the meeting has no current
--      graph state at all.
--
-- This lets older read queries move mechanically from `meeting_entities` to
-- `effective_meeting_entities` without resurrecting dismissed links or missing
-- current graph-only links.

BEGIN IMMEDIATE;

DROP VIEW IF EXISTS effective_meeting_entities;

CREATE VIEW effective_meeting_entities AS
SELECT
    le.owner_id AS meeting_id,
    le.entity_id,
    le.entity_type,
    COALESCE(le.confidence, 0.95) AS confidence,
    CASE WHEN le.role = 'primary' THEN 1 ELSE 0 END AS is_primary,
    le.role,
    le.source,
    le.rule_id
FROM linked_entities le
WHERE le.owner_type = 'meeting'

UNION ALL

SELECT
    me.meeting_id,
    me.entity_id,
    me.entity_type,
    COALESCE(me.confidence, 0.95) AS confidence,
    COALESCE(me.is_primary, 1) AS is_primary,
    CASE WHEN COALESCE(me.is_primary, 1) = 1 THEN 'primary' ELSE 'related' END AS role,
    'legacy:meeting_entities' AS source,
    NULL AS rule_id
FROM meeting_entities me
WHERE NOT EXISTS (
    SELECT 1
    FROM linked_entities_raw ler
    WHERE ler.owner_type = 'meeting'
      AND ler.owner_id = me.meeting_id
);

DROP VIEW IF EXISTS meeting_prep_status_view;

CREATE VIEW meeting_prep_status_view AS
WITH primary_entity AS (
    SELECT
        meeting_id,
        MIN(entity_id) AS entity_id,
        MIN(entity_type) AS entity_type
    FROM (
        SELECT
            me.meeting_id,
            me.entity_id,
            me.entity_type
        FROM effective_meeting_entities me
        INNER JOIN (
            SELECT meeting_id, MIN(entity_id) AS min_entity_id
            FROM effective_meeting_entities
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

COMMIT;

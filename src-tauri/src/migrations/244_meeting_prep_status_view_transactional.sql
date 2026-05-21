-- Atomic meeting_prep_status_view recreation.
--
-- v243 rebuilt `meeting_prep_status_view` with two separate statements
-- (`DROP VIEW`; `CREATE VIEW`). The migration runner at
-- `migrations.rs:3573` calls `conn.execute_batch(sql)` which does NOT
-- wrap the batch in a single transaction unless the SQL contains
-- explicit `BEGIN; ... COMMIT;`. Multi-process readers (additional
-- processes opening the encrypted DB during the migration window) could
-- observe the moment between DROP and CREATE and fail on
-- "no such view: meeting_prep_status_view" at `read.rs:106` prepare.
--
-- In-process readers (built by `DbService::open_at` at
-- `db_service.rs:338` AFTER the writer's migrations complete) were
-- safe; this migration closes the multi-process gap.
--
-- Strategy: wrap the rebuild in `BEGIN IMMEDIATE; ... COMMIT;` so the
-- write lock is held for the entire DROP/CREATE sequence. SQLite
-- guarantees other connections cannot observe schema state between
-- those two statements while the IMMEDIATE transaction holds the
-- write lock.
--
-- The view body is identical to v243; the only change is atomicity.
-- Once any reader (or this writer) wakes up after the COMMIT, it sees
-- the new view definition exclusively.

BEGIN IMMEDIATE;

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

COMMIT;

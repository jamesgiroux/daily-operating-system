-- Collapse duplicate commitment-typed actions and rewire bridge rows.
--
-- Background: the AI emits the same commitment text under different
-- commitment_id values (one per source — Gong call, meeting transcript,
-- Glean enrichment, CRM update, etc.). The bridge dedupes on
-- commitment_id, so each source produced a fresh action row. Real
-- production data on Globex showed 109 commitment rows collapsing
-- to 52 unique titles (52% duplication, growing every refresh).
--
-- This migration runs once at startup. It is safe to run on any DB:
-- if no duplicates exist, all statements are no-ops.
--
-- Strategy:
--   1. Pick a canonical action per (entity, normalized_title) group:
--      prefer user-accepted rows (status='unstarted' or 'started')
--      over backlog rows; tie-break by oldest created_at.
--   2. Rewire any ai_commitment_bridge rows pointing at a non-canonical
--      duplicate to the canonical action_id.
--   3. Delete the non-canonical action rows.
--
-- Forward-going dedup is enforced at the application layer in
-- `services::commitment_bridge::sync_ai_commitments` via
-- `find_existing_open_commitment_by_title`.

BEGIN;

-- Build the canonical-id table for each duplicate group. The CTE picks
-- the row with the highest "user-state weight" first (started > unstarted >
-- backlog) so accepted commitments are never replaced by backlog dupes.
CREATE TEMP TABLE _dos321_canonical AS
SELECT
    canonical_id,
    coalesce(account_id, project_id) AS entity_id,
    norm_title
FROM (
    SELECT
        a.id AS canonical_id,
        a.account_id,
        a.project_id,
        lower(trim(a.title)) AS norm_title,
        ROW_NUMBER() OVER (
            PARTITION BY
                coalesce(a.account_id, a.project_id),
                lower(trim(a.title))
            ORDER BY
                CASE a.status
                    WHEN 'started'   THEN 0
                    WHEN 'unstarted' THEN 1
                    WHEN 'backlog'   THEN 2
                    ELSE 3
                END ASC,
                a.created_at ASC
        ) AS rn
    FROM actions a
    WHERE a.action_kind = 'commitment'
      AND a.status NOT IN ('completed', 'cancelled', 'rejected', 'archived')
      AND coalesce(a.account_id, a.project_id) IS NOT NULL
)
WHERE rn = 1;

-- Index the temp table so the rewire/delete joins are fast.
CREATE INDEX _dos321_idx ON _dos321_canonical (entity_id, norm_title);

-- Rewire bridge rows: any bridge row whose action_id is a non-canonical
-- duplicate gets redirected to the canonical action_id. Bridge rows on
-- the canonical action are unchanged.
UPDATE ai_commitment_bridge
SET action_id = (
    SELECT c.canonical_id
    FROM actions a
    JOIN _dos321_canonical c
      ON c.entity_id = coalesce(a.account_id, a.project_id)
     AND c.norm_title = lower(trim(a.title))
    WHERE a.id = ai_commitment_bridge.action_id
    LIMIT 1
)
WHERE action_id IN (
    SELECT a.id
    FROM actions a
    JOIN _dos321_canonical c
      ON c.entity_id = coalesce(a.account_id, a.project_id)
     AND c.norm_title = lower(trim(a.title))
    WHERE a.action_kind = 'commitment'
      AND a.id != c.canonical_id
);

-- Delete duplicate action rows (everything that isn't a canonical row in
-- its (entity, normalized_title) group). The bridge rows that pointed at
-- these are already redirected by the UPDATE above.
DELETE FROM actions
WHERE id IN (
    SELECT a.id
    FROM actions a
    JOIN _dos321_canonical c
      ON c.entity_id = coalesce(a.account_id, a.project_id)
     AND c.norm_title = lower(trim(a.title))
    WHERE a.action_kind = 'commitment'
      AND a.status NOT IN ('completed', 'cancelled', 'rejected', 'archived')
      AND a.id != c.canonical_id
);

DROP INDEX _dos321_idx;
DROP TABLE _dos321_canonical;

COMMIT;

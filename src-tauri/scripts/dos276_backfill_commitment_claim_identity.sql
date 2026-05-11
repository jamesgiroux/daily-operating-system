-- One-shot CommitmentClaim identity backfill/verification script.
--
-- Intended for databases that already have migration 155 applied. The runtime
-- derives new typed commitment ids in Rust; this script handles the SQL-only
-- cleanup that can be run safely as a one-shot maintenance step:
--   1. move legacy "owner:" prose into structural owner columns
--   2. seed source-sighting rows from ai_commitment_bridge
--   3. collapse exact identity-tuple backlog duplicate commitments per account
--   4. leave verification queries at the bottom for the release log

BEGIN;

UPDATE actions
SET owner_raw = trim(substr(context, length('owner:') + 1)),
    owner_confidence = 0.0,
    owner_source = 'legacy_context_ambiguous',
    context = NULL
WHERE action_kind = 'commitment'
  AND context IS NOT NULL
  AND lower(trim(context)) LIKE 'owner:%'
  AND owner_raw IS NULL;

INSERT OR IGNORE INTO action_commitment_sources (
    id,
    commitment_id,
    action_id,
    source_key,
    source_type,
    source_id,
    source_label,
    observed_at,
    trust_score,
    trust_band,
    owner_raw
)
SELECT
    'dos276-backfill:' || b.commitment_id,
    COALESCE(a.commitment_id, b.commitment_id),
    b.action_id,
    lower(trim(COALESCE(NULLIF(a.source_type, ''), 'commitment')))
        || ':' ||
        lower(trim(COALESCE(NULLIF(a.source_id, ''), NULLIF(a.source_label, ''), a.id))),
    a.source_type,
    a.source_id,
    a.source_label,
    COALESCE(b.last_seen_at, a.updated_at, datetime('now')),
    a.trust_score,
    COALESCE(a.trust_band, 'unscored'),
    a.owner_raw
FROM ai_commitment_bridge b
JOIN actions a ON a.id = b.action_id
WHERE a.action_kind = 'commitment';

CREATE TEMP TABLE _dos276_backlog_canonical AS
SELECT canonical_id, account_id, title_key, due_date_key, owner_key
FROM (
    SELECT
        id AS canonical_id,
        account_id,
        lower(trim(title)) AS title_key,
        COALESCE(due_date, '') AS due_date_key,
        COALESCE(owner_raw, '') AS owner_key,
        ROW_NUMBER() OVER (
            PARTITION BY
                account_id,
                lower(trim(title)),
                COALESCE(due_date, ''),
                COALESCE(owner_raw, '')
            ORDER BY created_at ASC, id ASC
        ) AS rn
    FROM actions
    WHERE action_kind = 'commitment'
      AND status = 'backlog'
      AND account_id IS NOT NULL
)
WHERE rn = 1;

CREATE INDEX _dos276_backlog_canonical_idx
    ON _dos276_backlog_canonical(account_id, title_key, due_date_key, owner_key);

UPDATE ai_commitment_bridge
SET action_id = (
    SELECT c.canonical_id
    FROM actions a
    JOIN _dos276_backlog_canonical c
      ON c.account_id = a.account_id
     AND c.title_key = lower(trim(a.title))
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.owner_raw, '')
    WHERE a.id = ai_commitment_bridge.action_id
    LIMIT 1
)
WHERE action_id IN (
    SELECT a.id
    FROM actions a
    JOIN _dos276_backlog_canonical c
      ON c.account_id = a.account_id
     AND c.title_key = lower(trim(a.title))
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.owner_raw, '')
    WHERE a.action_kind = 'commitment'
      AND a.status = 'backlog'
      AND a.id != c.canonical_id
);

UPDATE action_commitment_sources
SET action_id = (
    SELECT c.canonical_id
    FROM actions a
    JOIN _dos276_backlog_canonical c
      ON c.account_id = a.account_id
     AND c.title_key = lower(trim(a.title))
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.owner_raw, '')
    WHERE a.id = action_commitment_sources.action_id
    LIMIT 1
)
WHERE action_id IN (
    SELECT a.id
    FROM actions a
    JOIN _dos276_backlog_canonical c
      ON c.account_id = a.account_id
     AND c.title_key = lower(trim(a.title))
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.owner_raw, '')
    WHERE a.action_kind = 'commitment'
      AND a.status = 'backlog'
      AND a.id != c.canonical_id
);

DELETE FROM actions
WHERE id IN (
    SELECT a.id
    FROM actions a
    JOIN _dos276_backlog_canonical c
      ON c.account_id = a.account_id
     AND c.title_key = lower(trim(a.title))
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.owner_raw, '')
    WHERE a.action_kind = 'commitment'
      AND a.status = 'backlog'
      AND a.id != c.canonical_id
);

DROP INDEX _dos276_backlog_canonical_idx;
DROP TABLE _dos276_backlog_canonical;

COMMIT;

-- Verification: must return zero.
SELECT COUNT(*) AS owner_only_prose_rows
FROM actions
WHERE action_kind = 'commitment'
  AND context IS NOT NULL
  AND lower(trim(context)) LIKE 'owner:%'
  AND owner_raw IS NULL;

-- Verification: must return zero rows. This is the identity-tuple duplicate
-- GROUP BY/HAVING shape used for the production backlog duplicate audit.
SELECT title, account_id, due_date, owner_raw, COUNT(*) AS duplicate_count
FROM actions
WHERE action_kind = 'commitment'
  AND status = 'backlog'
  AND account_id IS NOT NULL
GROUP BY lower(trim(title)), account_id, COALESCE(due_date, ''), COALESCE(owner_raw, '')
HAVING COUNT(*) > 1;

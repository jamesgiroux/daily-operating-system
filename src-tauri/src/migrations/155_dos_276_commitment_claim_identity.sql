-- DOS-276 W4-A: typed CommitmentClaim identity and structural owner fields.
--
-- Existing Work-tab commitment rows used ai_commitment_bridge.commitment_id
-- plus actions.context = 'owner: ...'. This migration moves commitment
-- identity onto actions, records owner resolution state structurally, preserves
-- old bridge rows for tombstone compatibility, and installs the identity-tuple
-- backlog duplicate guard used by the production audit query.

BEGIN;

ALTER TABLE actions ADD COLUMN commitment_id TEXT;
ALTER TABLE actions ADD COLUMN owner_raw TEXT;
ALTER TABLE actions ADD COLUMN owner_entity_id TEXT REFERENCES people(id) ON DELETE SET NULL;
ALTER TABLE actions ADD COLUMN owner_confidence REAL;
ALTER TABLE actions ADD COLUMN owner_source TEXT;
ALTER TABLE actions ADD COLUMN trust_score REAL;
ALTER TABLE actions ADD COLUMN trust_band TEXT;
ALTER TABLE actions ADD COLUMN normalized_title TEXT;
ALTER TABLE actions ADD COLUMN normalized_owner TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_actions_commitment_id_unique
    ON actions(commitment_id)
    WHERE commitment_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS action_commitment_sources (
    id                TEXT PRIMARY KEY,
    commitment_id     TEXT NOT NULL,
    action_id         TEXT NOT NULL REFERENCES actions(id) ON DELETE CASCADE,
    -- Normalized source_type:source_id; LLM ordinals are not authoritative.
    source_key        TEXT,
    source_type       TEXT,
    source_id         TEXT,
    source_label      TEXT,
    observed_at       TEXT NOT NULL,
    source_confidence REAL,
    trust_score       REAL,
    trust_band        TEXT,
    owner_raw         TEXT,
    owner_ref_json    TEXT
);

CREATE INDEX IF NOT EXISTS idx_action_commitment_sources_commitment
    ON action_commitment_sources(commitment_id, observed_at);
CREATE INDEX IF NOT EXISTS idx_action_commitment_sources_action
    ON action_commitment_sources(action_id, observed_at);
CREATE INDEX IF NOT EXISTS idx_action_commitment_sources_commitment_key
    ON action_commitment_sources(commitment_id, source_key);

-- Seed commitment_id from the legacy bridge where possible. The new runtime
-- will replace legacy ids with derived CommitmentClaim ids on first sighting,
-- but this prevents rows from being identity-less during the migration window.
UPDATE actions
SET commitment_id = (
    SELECT b.commitment_id
    FROM ai_commitment_bridge b
    WHERE b.action_id = actions.id
    ORDER BY b.first_seen_at ASC
    LIMIT 1
)
WHERE action_kind = 'commitment'
  AND commitment_id IS NULL
  AND EXISTS (
      SELECT 1 FROM ai_commitment_bridge b WHERE b.action_id = actions.id
  );

UPDATE actions
SET commitment_id = source_id
WHERE action_kind = 'commitment'
  AND commitment_id IS NULL
  AND source_type = 'commitment'
  AND source_id IS NOT NULL
  AND (
      SELECT COUNT(*)
      FROM actions other
      WHERE other.action_kind = 'commitment'
        AND other.source_id = actions.source_id
  ) = 1
  AND NOT EXISTS (
      SELECT 1
      FROM actions other
      WHERE other.id != actions.id
        AND other.commitment_id = actions.source_id
  );

-- Migrate owner-only prose prefixes into structural columns. Rows that cannot
-- be resolved in SQL are explicitly flagged as ambiguous instead of remaining
-- owner-only prose.
UPDATE actions
SET owner_raw = trim(substr(context, length('owner:') + 1)),
    owner_confidence = 0.0,
    owner_source = 'legacy_context_ambiguous',
    context = NULL
WHERE action_kind = 'commitment'
  AND context IS NOT NULL
  AND lower(trim(context)) LIKE 'owner:%'
  AND owner_raw IS NULL;

-- Resolve exact unique person-name matches globally.
UPDATE actions
SET owner_entity_id = (
        SELECT p.id
        FROM people p
        WHERE lower(trim(p.name)) = lower(trim(actions.owner_raw))
        LIMIT 1
    ),
    owner_confidence = 0.95,
    owner_source = 'migration_exact_person_name'
WHERE action_kind = 'commitment'
  AND owner_raw IS NOT NULL
  AND owner_entity_id IS NULL
  AND (
      SELECT COUNT(*)
      FROM people p
      WHERE lower(trim(p.name)) = lower(trim(actions.owner_raw))
  ) = 1;

-- Backfill normalized identity fields using the runtime commitment identity
-- contract: lowercase, trim, collapse internal whitespace, and strip ASCII
-- punctuation before duplicate collapse and partial unique indexes run.
UPDATE actions
SET normalized_title = COALESCE(title, ''),
    normalized_owner = COALESCE(owner_raw, '')
WHERE action_kind = 'commitment';

UPDATE actions SET normalized_title = replace(normalized_title, char(9), ' '), normalized_owner = replace(normalized_owner, char(9), ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, char(10), ' '), normalized_owner = replace(normalized_owner, char(10), ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, char(11), ' '), normalized_owner = replace(normalized_owner, char(11), ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, char(12), ' '), normalized_owner = replace(normalized_owner, char(12), ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, char(13), ' '), normalized_owner = replace(normalized_owner, char(13), ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '!', ' '), normalized_owner = replace(normalized_owner, '!', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '"', ' '), normalized_owner = replace(normalized_owner, '"', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '#', ' '), normalized_owner = replace(normalized_owner, '#', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '$', ' '), normalized_owner = replace(normalized_owner, '$', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '%', ' '), normalized_owner = replace(normalized_owner, '%', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '&', ' '), normalized_owner = replace(normalized_owner, '&', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '''', ' '), normalized_owner = replace(normalized_owner, '''', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '(', ' '), normalized_owner = replace(normalized_owner, '(', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, ')', ' '), normalized_owner = replace(normalized_owner, ')', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '*', ' '), normalized_owner = replace(normalized_owner, '*', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '+', ' '), normalized_owner = replace(normalized_owner, '+', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '.', ' '), normalized_owner = replace(normalized_owner, '.', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, ',', ' '), normalized_owner = replace(normalized_owner, ',', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, ':', ' '), normalized_owner = replace(normalized_owner, ':', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, ';', ' '), normalized_owner = replace(normalized_owner, ';', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '<', ' '), normalized_owner = replace(normalized_owner, '<', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '=', ' '), normalized_owner = replace(normalized_owner, '=', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '>', ' '), normalized_owner = replace(normalized_owner, '>', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '?', ' '), normalized_owner = replace(normalized_owner, '?', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '@', ' '), normalized_owner = replace(normalized_owner, '@', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '-', ' '), normalized_owner = replace(normalized_owner, '-', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '_', ' '), normalized_owner = replace(normalized_owner, '_', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '/', ' '), normalized_owner = replace(normalized_owner, '/', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, char(92), ' '), normalized_owner = replace(normalized_owner, char(92), ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '[', ' '), normalized_owner = replace(normalized_owner, '[', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, ']', ' '), normalized_owner = replace(normalized_owner, ']', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '^', ' '), normalized_owner = replace(normalized_owner, '^', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '{', ' '), normalized_owner = replace(normalized_owner, '{', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '}', ' '), normalized_owner = replace(normalized_owner, '}', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '`', ' '), normalized_owner = replace(normalized_owner, '`', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '|', ' '), normalized_owner = replace(normalized_owner, '|', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '~', ' '), normalized_owner = replace(normalized_owner, '~', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '  ', ' '), normalized_owner = replace(normalized_owner, '  ', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '  ', ' '), normalized_owner = replace(normalized_owner, '  ', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '  ', ' '), normalized_owner = replace(normalized_owner, '  ', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '  ', ' '), normalized_owner = replace(normalized_owner, '  ', ' ') WHERE action_kind = 'commitment';
UPDATE actions SET normalized_title = replace(normalized_title, '  ', ' '), normalized_owner = replace(normalized_owner, '  ', ' ') WHERE action_kind = 'commitment';
UPDATE actions
SET normalized_title = lower(trim(normalized_title)),
    normalized_owner = NULLIF(lower(trim(normalized_owner)), '')
WHERE action_kind = 'commitment';

-- Seed one source row per legacy bridge mapping so existing commitment cards
-- can show non-zero source counts immediately after migration.
INSERT OR IGNORE INTO action_commitment_sources (
    id,
    commitment_id,
    action_id,
    source_key,
    source_type,
    source_id,
    source_label,
    observed_at,
    trust_band,
    owner_raw
)
SELECT
    'migration:' || b.commitment_id,
    COALESCE(a.commitment_id, b.commitment_id),
    b.action_id,
    lower(trim(COALESCE(NULLIF(a.source_type, ''), 'commitment')))
        || ':' ||
        lower(trim(COALESCE(NULLIF(a.source_id, ''), NULLIF(a.source_label, ''), a.id))),
    a.source_type,
    a.source_id,
    a.source_label,
    b.last_seen_at,
    COALESCE(a.trust_band, 'unscored'),
    a.owner_raw
FROM ai_commitment_bridge b
JOIN actions a ON a.id = b.action_id
WHERE a.action_kind = 'commitment';

-- Collapse exact identity duplicate backlog commitments within account_id before
-- adding the partial unique index that enforces the audit query going forward.
CREATE TEMP TABLE _dos276_backlog_canonical AS
SELECT
    canonical_id,
    account_id,
    title_key,
    due_date_key,
    owner_key
FROM (
    SELECT
        id AS canonical_id,
        account_id,
        normalized_title AS title_key,
        COALESCE(due_date, '') AS due_date_key,
        COALESCE(normalized_owner, '') AS owner_key,
        ROW_NUMBER() OVER (
            PARTITION BY
                account_id,
                normalized_title,
                COALESCE(due_date, ''),
                COALESCE(normalized_owner, '')
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
     AND c.title_key = a.normalized_title
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.normalized_owner, '')
    WHERE a.id = ai_commitment_bridge.action_id
    LIMIT 1
)
WHERE action_id IN (
    SELECT a.id
    FROM actions a
    JOIN _dos276_backlog_canonical c
      ON c.account_id = a.account_id
     AND c.title_key = a.normalized_title
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.normalized_owner, '')
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
     AND c.title_key = a.normalized_title
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.normalized_owner, '')
    WHERE a.id = action_commitment_sources.action_id
    LIMIT 1
)
WHERE action_id IN (
    SELECT a.id
    FROM actions a
    JOIN _dos276_backlog_canonical c
      ON c.account_id = a.account_id
     AND c.title_key = a.normalized_title
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.normalized_owner, '')
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
     AND c.title_key = a.normalized_title
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.normalized_owner, '')
    WHERE a.action_kind = 'commitment'
      AND a.status = 'backlog'
      AND a.id != c.canonical_id
);

DROP INDEX _dos276_backlog_canonical_idx;
DROP TABLE _dos276_backlog_canonical;

-- Repeat the same duplicate-collapse pass for project-scoped backlog
-- commitments before adding the project partial unique index.
CREATE TEMP TABLE _dos276_backlog_project_canonical AS
SELECT
    canonical_id,
    project_id,
    title_key,
    due_date_key,
    owner_key
FROM (
    SELECT
        id AS canonical_id,
        project_id,
        normalized_title AS title_key,
        COALESCE(due_date, '') AS due_date_key,
        COALESCE(normalized_owner, '') AS owner_key,
        ROW_NUMBER() OVER (
            PARTITION BY
                project_id,
                normalized_title,
                COALESCE(due_date, ''),
                COALESCE(normalized_owner, '')
            ORDER BY created_at ASC, id ASC
        ) AS rn
    FROM actions
    WHERE action_kind = 'commitment'
      AND status = 'backlog'
      AND project_id IS NOT NULL
)
WHERE rn = 1;

CREATE INDEX _dos276_backlog_project_canonical_idx
    ON _dos276_backlog_project_canonical(project_id, title_key, due_date_key, owner_key);

UPDATE ai_commitment_bridge
SET action_id = (
    SELECT c.canonical_id
    FROM actions a
    JOIN _dos276_backlog_project_canonical c
      ON c.project_id = a.project_id
     AND c.title_key = a.normalized_title
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.normalized_owner, '')
    WHERE a.id = ai_commitment_bridge.action_id
    LIMIT 1
)
WHERE action_id IN (
    SELECT a.id
    FROM actions a
    JOIN _dos276_backlog_project_canonical c
      ON c.project_id = a.project_id
     AND c.title_key = a.normalized_title
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.normalized_owner, '')
    WHERE a.action_kind = 'commitment'
      AND a.status = 'backlog'
      AND a.id != c.canonical_id
);

UPDATE action_commitment_sources
SET action_id = (
    SELECT c.canonical_id
    FROM actions a
    JOIN _dos276_backlog_project_canonical c
      ON c.project_id = a.project_id
     AND c.title_key = a.normalized_title
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.normalized_owner, '')
    WHERE a.id = action_commitment_sources.action_id
    LIMIT 1
)
WHERE action_id IN (
    SELECT a.id
    FROM actions a
    JOIN _dos276_backlog_project_canonical c
      ON c.project_id = a.project_id
     AND c.title_key = a.normalized_title
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.normalized_owner, '')
    WHERE a.action_kind = 'commitment'
      AND a.status = 'backlog'
      AND a.id != c.canonical_id
);

DELETE FROM actions
WHERE id IN (
    SELECT a.id
    FROM actions a
    JOIN _dos276_backlog_project_canonical c
      ON c.project_id = a.project_id
     AND c.title_key = a.normalized_title
     AND c.due_date_key = COALESCE(a.due_date, '')
     AND c.owner_key = COALESCE(a.normalized_owner, '')
    WHERE a.action_kind = 'commitment'
      AND a.status = 'backlog'
      AND a.id != c.canonical_id
);

DROP INDEX _dos276_backlog_project_canonical_idx;
DROP TABLE _dos276_backlog_project_canonical;

CREATE UNIQUE INDEX IF NOT EXISTS idx_actions_backlog_commitment_identity_account_unique
    ON actions(account_id, normalized_title, COALESCE(due_date, ''), COALESCE(normalized_owner, ''))
    WHERE action_kind = 'commitment'
      AND status = 'backlog'
      AND account_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_actions_backlog_commitment_identity_project_unique
    ON actions(project_id, normalized_title, COALESCE(due_date, ''), COALESCE(normalized_owner, ''))
    WHERE action_kind = 'commitment'
      AND status = 'backlog'
      AND project_id IS NOT NULL;

COMMIT;

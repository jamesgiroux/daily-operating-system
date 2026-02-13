-- Sprint 21: Account team model (I207)
-- - Introduce account_team + account_team_import_notes
-- - Backfill legacy accounts.csm/champion to account_team where exact person match exists
-- - Drop legacy csm/champion columns by rebuilding accounts table

CREATE TABLE IF NOT EXISTS account_team (
    account_id TEXT NOT NULL,
    person_id TEXT NOT NULL,
    role TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (account_id, person_id, role)
);
CREATE INDEX IF NOT EXISTS idx_account_team_account ON account_team(account_id);
CREATE INDEX IF NOT EXISTS idx_account_team_person ON account_team(person_id);

CREATE TABLE IF NOT EXISTS account_team_import_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    legacy_field TEXT NOT NULL,
    legacy_value TEXT NOT NULL,
    note TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_account_team_notes_account ON account_team_import_notes(account_id);

-- Backfill exact csm matches
INSERT OR IGNORE INTO account_team (account_id, person_id, role, created_at)
SELECT
    a.id,
    p.id,
    'csm',
    datetime('now')
FROM accounts a
JOIN people p
  ON LOWER(TRIM(p.name)) = LOWER(TRIM(a.csm))
WHERE a.csm IS NOT NULL
  AND TRIM(a.csm) <> ''
  AND (
    SELECT COUNT(*)
    FROM people p2
    WHERE LOWER(TRIM(p2.name)) = LOWER(TRIM(a.csm))
  ) = 1;

-- Backfill exact champion matches
INSERT OR IGNORE INTO account_team (account_id, person_id, role, created_at)
SELECT
    a.id,
    p.id,
    'champion',
    datetime('now')
FROM accounts a
JOIN people p
  ON LOWER(TRIM(p.name)) = LOWER(TRIM(a.champion))
WHERE a.champion IS NOT NULL
  AND TRIM(a.champion) <> ''
  AND (
    SELECT COUNT(*)
    FROM people p2
    WHERE LOWER(TRIM(p2.name)) = LOWER(TRIM(a.champion))
  ) = 1;

-- Record unmatched/ambiguous legacy values for user follow-up
INSERT INTO account_team_import_notes (account_id, legacy_field, legacy_value, note, created_at)
SELECT
    a.id,
    'csm',
    a.csm,
    'No exact unique person match found during migration',
    datetime('now')
FROM accounts a
WHERE a.csm IS NOT NULL
  AND TRIM(a.csm) <> ''
  AND (
    SELECT COUNT(*)
    FROM people p2
    WHERE LOWER(TRIM(p2.name)) = LOWER(TRIM(a.csm))
  ) != 1;

INSERT INTO account_team_import_notes (account_id, legacy_field, legacy_value, note, created_at)
SELECT
    a.id,
    'champion',
    a.champion,
    'No exact unique person match found during migration',
    datetime('now')
FROM accounts a
WHERE a.champion IS NOT NULL
  AND TRIM(a.champion) <> ''
  AND (
    SELECT COUNT(*)
    FROM people p2
    WHERE LOWER(TRIM(p2.name)) = LOWER(TRIM(a.champion))
  ) != 1;

-- Ensure account_team links also exist in generic entity_people for compatibility
INSERT OR IGNORE INTO entity_people (entity_id, person_id, relationship_type)
SELECT account_id, person_id, 'associated'
FROM account_team;

-- Rebuild accounts table without legacy csm/champion columns
PRAGMA foreign_keys = OFF;

CREATE TABLE accounts_new (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    lifecycle TEXT,
    arr REAL,
    health TEXT CHECK(health IN ('green', 'yellow', 'red')),
    contract_start TEXT,
    contract_end TEXT,
    nps INTEGER,
    tracker_path TEXT,
    parent_id TEXT,
    is_internal INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    archived INTEGER DEFAULT 0
);

INSERT INTO accounts_new (
    id, name, lifecycle, arr, health, contract_start, contract_end,
    nps, tracker_path, parent_id, is_internal, updated_at, archived
)
SELECT
    id, name, lifecycle, arr, health, contract_start, contract_end,
    nps, tracker_path, parent_id, is_internal, updated_at, archived
FROM accounts;

DROP TABLE accounts;
ALTER TABLE accounts_new RENAME TO accounts;

CREATE INDEX IF NOT EXISTS idx_accounts_parent ON accounts(parent_id);
CREATE INDEX IF NOT EXISTS idx_accounts_archived ON accounts(archived);
CREATE INDEX IF NOT EXISTS idx_accounts_internal ON accounts(is_internal);

PRAGMA foreign_keys = ON;

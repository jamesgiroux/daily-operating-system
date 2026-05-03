-- Person-first stakeholder architecture.
-- Promotes account_stakeholders to sole source of truth for stakeholder data.
-- Adds: FK to people(id), engagement/assessment columns, multi-role table, suggestions table.
--
-- Rebuilds account_stakeholders to add the FK constraint (SQLite requires table rebuild).
-- Captures old roles into account_stakeholder_roles before dropping the role column.

PRAGMA foreign_keys = OFF;
BEGIN IMMEDIATE;

-- =============================================================================
-- Step 1: Capture existing roles before we rebuild account_stakeholders
-- =============================================================================
CREATE TEMP TABLE _old_stakeholder_data AS
SELECT account_id, person_id, role, data_source, last_seen_in_glean, created_at
FROM account_stakeholders;

-- =============================================================================
-- Step 2: Rebuild account_stakeholders with FK to people + new columns
--         The `role` column is removed — roles now live in account_stakeholder_roles.
-- =============================================================================
DROP TABLE IF EXISTS account_stakeholders;
CREATE TABLE account_stakeholders (
    account_id             TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    person_id              TEXT NOT NULL REFERENCES people(id)   ON DELETE CASCADE,
    engagement             TEXT,          -- strong_advocate | engaged | neutral | disengaged | unknown
    data_source_engagement TEXT NOT NULL DEFAULT 'ai',
    assessment             TEXT,          -- free-text assessment of the person's stance
    data_source_assessment TEXT NOT NULL DEFAULT 'ai',
    data_source            TEXT NOT NULL DEFAULT 'user',       -- row-level provenance (preserved)
    last_seen_in_glean     TEXT,                                -- staleness tracking (preserved)
    created_at             TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (account_id, person_id)
);

-- Migrate only rows whose person_id actually exists in people (FK enforcement)
INSERT OR IGNORE INTO account_stakeholders (
    account_id, person_id, data_source, last_seen_in_glean, created_at
)
SELECT account_id, person_id, data_source, last_seen_in_glean, created_at
FROM _old_stakeholder_data
WHERE person_id IN (SELECT id FROM people);

CREATE INDEX IF NOT EXISTS idx_account_stakeholders_person
    ON account_stakeholders(person_id);

-- =============================================================================
-- Step 3: Multi-role table (one row per role per person per account)
-- =============================================================================
CREATE TABLE account_stakeholder_roles (
    account_id  TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    person_id   TEXT NOT NULL REFERENCES people(id)   ON DELETE CASCADE,
    role        TEXT NOT NULL,
    data_source TEXT NOT NULL DEFAULT 'ai',
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (account_id, person_id, role)
);

CREATE INDEX IF NOT EXISTS idx_account_stakeholder_roles_person
    ON account_stakeholder_roles(person_id);

-- Seed roles from captured old data
INSERT OR IGNORE INTO account_stakeholder_roles (account_id, person_id, role, data_source)
SELECT account_id, person_id, role, data_source
FROM _old_stakeholder_data
WHERE person_id IN (SELECT id FROM people);

DROP TABLE IF EXISTS _old_stakeholder_data;

-- =============================================================================
-- Step 4: Suggestions table (AI-proposed stakeholders pending user accept/dismiss)
-- =============================================================================
CREATE TABLE stakeholder_suggestions (
    id                   INTEGER PRIMARY KEY,
    account_id           TEXT NOT NULL,
    person_id            TEXT,              -- NULL if person not yet created
    suggested_name       TEXT,
    suggested_email      TEXT,
    suggested_role       TEXT,
    suggested_engagement TEXT,
    source               TEXT NOT NULL,     -- 'glean' | 'pty' | 'google'
    status               TEXT NOT NULL DEFAULT 'pending',  -- pending | accepted | dismissed
    raw_suggestion       TEXT,              -- original JSON from AI (debugging)
    created_at           TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at          TEXT
);

CREATE INDEX IF NOT EXISTS idx_stakeholder_suggestions_account
    ON stakeholder_suggestions(account_id);
CREATE INDEX IF NOT EXISTS idx_stakeholder_suggestions_status
    ON stakeholder_suggestions(account_id, status);

COMMIT;
PRAGMA foreign_keys = ON;

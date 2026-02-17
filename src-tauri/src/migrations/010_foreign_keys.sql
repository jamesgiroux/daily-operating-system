-- Migration 010: Add foreign key constraints to existing tables (I285)
--
-- SQLite does not support ALTER TABLE ADD CONSTRAINT, so we use the
-- copy → drop → create-with-FKs → re-insert → recreate-indexes pattern.
--
-- Tables modified:
--   actions        — account_id→accounts, project_id→projects, person_id→people (ON DELETE SET NULL)
--   account_team   — account_id→accounts (CASCADE)
--   account_domains— account_id→accounts (CASCADE)
--
-- Skipped tables:
--   captures       — meeting_id is polymorphic (references meetings OR inbox files), not a pure FK
--   email_signals  — uses polymorphic entity_id/entity_type pattern, not a single FK to accounts

PRAGMA foreign_keys = OFF;

BEGIN;

------------------------------------------------------------
-- 1. actions
------------------------------------------------------------
CREATE TABLE actions_new (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    priority TEXT CHECK(priority IN ('P1', 'P2', 'P3')) DEFAULT 'P2',
    status TEXT CHECK(status IN ('pending', 'completed', 'waiting', 'cancelled')) DEFAULT 'pending',
    created_at TEXT NOT NULL,
    due_date TEXT,
    completed_at TEXT,
    account_id TEXT REFERENCES accounts(id) ON DELETE SET NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    source_type TEXT,
    source_id TEXT,
    source_label TEXT,
    context TEXT,
    waiting_on TEXT,
    updated_at TEXT NOT NULL,
    person_id TEXT REFERENCES people(id) ON DELETE SET NULL,
    needs_decision INTEGER DEFAULT 0
);

INSERT INTO actions_new (id, title, priority, status, created_at, due_date,
    completed_at, account_id, project_id, source_type, source_id, source_label,
    context, waiting_on, updated_at, person_id, needs_decision)
SELECT id, title, priority, status, created_at, due_date,
    completed_at, account_id, project_id, source_type, source_id, source_label,
    context, waiting_on, updated_at, person_id, needs_decision
FROM actions;
DROP TABLE actions;
ALTER TABLE actions_new RENAME TO actions;

CREATE INDEX IF NOT EXISTS idx_actions_status ON actions(status);
CREATE INDEX IF NOT EXISTS idx_actions_due_date ON actions(due_date);
CREATE INDEX IF NOT EXISTS idx_actions_account ON actions(account_id);
CREATE INDEX IF NOT EXISTS idx_actions_status_due_date ON actions(status, due_date);

------------------------------------------------------------
-- 2. account_team
------------------------------------------------------------
CREATE TABLE account_team_new (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL,
    role TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (account_id, person_id, role)
);

INSERT INTO account_team_new SELECT * FROM account_team;
DROP TABLE account_team;
ALTER TABLE account_team_new RENAME TO account_team;

CREATE INDEX IF NOT EXISTS idx_account_team_account ON account_team(account_id);
CREATE INDEX IF NOT EXISTS idx_account_team_person ON account_team(person_id);
CREATE INDEX IF NOT EXISTS idx_account_team_account_role ON account_team(account_id, role);

------------------------------------------------------------
-- 4. account_domains
------------------------------------------------------------
CREATE TABLE account_domains_new (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    domain TEXT NOT NULL,
    PRIMARY KEY (account_id, domain)
);

INSERT INTO account_domains_new SELECT * FROM account_domains;
DROP TABLE account_domains;
ALTER TABLE account_domains_new RENAME TO account_domains;

CREATE INDEX IF NOT EXISTS idx_account_domains_domain ON account_domains(domain);

COMMIT;

-- Note: PRAGMA foreign_keys = ON is set in db.rs open_at() at the connection
-- level, not here. Migrations handle schema only; FK enforcement is a runtime
-- per-connection setting.

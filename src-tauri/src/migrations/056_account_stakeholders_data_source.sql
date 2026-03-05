-- 056: Add data_source provenance to account_stakeholders (I511 follow-up hardening)
-- Tracks stakeholder link origin for purge-on-revocation workflows (ADR-0098).

PRAGMA foreign_keys = OFF;
BEGIN IMMEDIATE;

DROP TABLE IF EXISTS account_stakeholders_new;
CREATE TABLE account_stakeholders_new (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'associated',
    relationship_type TEXT DEFAULT 'associated',
    data_source TEXT NOT NULL DEFAULT 'user',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (account_id, person_id)
);

INSERT OR IGNORE INTO account_stakeholders_new (
    account_id,
    person_id,
    role,
    relationship_type,
    data_source,
    created_at
)
SELECT
    account_id,
    person_id,
    role,
    relationship_type,
    'user',
    created_at
FROM account_stakeholders;

DROP TABLE IF EXISTS account_stakeholders;
ALTER TABLE account_stakeholders_new RENAME TO account_stakeholders;
CREATE INDEX IF NOT EXISTS idx_account_stakeholders_person ON account_stakeholders(person_id);

PRAGMA foreign_keys = ON;
COMMIT;

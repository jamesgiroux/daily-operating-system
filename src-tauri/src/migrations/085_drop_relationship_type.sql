-- 085: Drop dead `relationship_type` column from account_stakeholders.
-- Column was only ever written with default 'associated' and never read by any query.
-- Stakeholder roles are managed via `account_stakeholder_roles` table (future migration)
-- and `entity_members.relationship_type` (separate table, unaffected).

PRAGMA foreign_keys = OFF;
BEGIN IMMEDIATE;

CREATE TABLE account_stakeholders_new (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'associated',
    data_source TEXT NOT NULL DEFAULT 'user',
    last_seen_in_glean TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (account_id, person_id)
);

INSERT INTO account_stakeholders_new (account_id, person_id, role, data_source, last_seen_in_glean, created_at)
SELECT account_id, person_id, role, data_source, last_seen_in_glean, created_at
FROM account_stakeholders;

DROP TABLE account_stakeholders;
ALTER TABLE account_stakeholders_new RENAME TO account_stakeholders;
CREATE INDEX IF NOT EXISTS idx_account_stakeholders_person ON account_stakeholders(person_id);

COMMIT;
-- Note: PRAGMA foreign_keys is left at OFF for migration safety.
-- The app connection initializer sets it to ON at startup.

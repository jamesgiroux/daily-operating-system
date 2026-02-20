-- Migration 032: FK constraints on junction tables + expression indexes
--
-- 1. Add FK constraints to junction tables:
--    - meeting_entities.meeting_id → meetings_history(id) ON DELETE CASCADE
--    - meeting_attendees.meeting_id → meetings_history(id) ON DELETE CASCADE
--    - meeting_attendees.person_id → people(id) ON DELETE SET NULL
--
-- 2. Expression indexes for case-insensitive lookups:
--    - idx_accounts_name_lower ON accounts(LOWER(name))
--    - idx_actions_title_lower ON actions(LOWER(TRIM(title)))
--
-- Skipped FK constraints (polymorphic columns):
--    - meeting_entities.entity_id — polymorphic, points to accounts OR projects via entity_type
--    - entity_people.entity_id — polymorphic, points to accounts OR projects
--
-- Note: chat_turns.session_id already has FK from migration 007.

PRAGMA foreign_keys = OFF;

BEGIN;

------------------------------------------------------------
-- 1. meeting_entities: add FK on meeting_id
------------------------------------------------------------
CREATE TABLE meeting_entities_new (
    meeting_id  TEXT NOT NULL REFERENCES meetings_history(id) ON DELETE CASCADE,
    entity_id   TEXT NOT NULL,
    -- entity_id is polymorphic (accounts, projects) — no FK constraint
    entity_type TEXT NOT NULL DEFAULT 'account',
    PRIMARY KEY (meeting_id, entity_id)
);

INSERT INTO meeting_entities_new (meeting_id, entity_id, entity_type)
SELECT meeting_id, entity_id, entity_type FROM meeting_entities;

DROP TABLE meeting_entities;
ALTER TABLE meeting_entities_new RENAME TO meeting_entities;

CREATE INDEX IF NOT EXISTS idx_meeting_entities_entity ON meeting_entities(entity_id);

------------------------------------------------------------
-- 2. meeting_attendees: add FK on meeting_id and person_id
------------------------------------------------------------
-- person_id uses SET NULL so we need to allow NULLs for the FK behavior,
-- but the original schema has person_id as NOT NULL in the composite PK.
-- We keep NOT NULL since orphaned attendee rows should cascade-delete
-- with the meeting, and person deletions should also cascade.
CREATE TABLE meeting_attendees_new (
    meeting_id TEXT NOT NULL REFERENCES meetings_history(id) ON DELETE CASCADE,
    person_id  TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    PRIMARY KEY (meeting_id, person_id)
);

INSERT INTO meeting_attendees_new (meeting_id, person_id)
SELECT meeting_id, person_id FROM meeting_attendees;

DROP TABLE meeting_attendees;
ALTER TABLE meeting_attendees_new RENAME TO meeting_attendees;

CREATE INDEX IF NOT EXISTS idx_attendees_person ON meeting_attendees(person_id);

COMMIT;

------------------------------------------------------------
-- 3. Expression indexes for case-insensitive lookups
------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_accounts_name_lower ON accounts(LOWER(name));
CREATE INDEX IF NOT EXISTS idx_actions_title_lower ON actions(LOWER(TRIM(title)));

-- DOS-379: entity_members.entity_id must reference a real profile-agnostic
-- entity. Rebuild is required because SQLite cannot add an FK in place.

PRAGMA foreign_keys = OFF;
BEGIN;

CREATE TABLE entity_members_new (
    entity_id TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL,
    relationship_type TEXT DEFAULT 'associated',
    PRIMARY KEY (entity_id, person_id)
);

INSERT OR IGNORE INTO entity_members_new (entity_id, person_id, relationship_type)
SELECT em.entity_id, em.person_id, em.relationship_type
FROM entity_members em
WHERE EXISTS (
    SELECT 1
    FROM entities e
    WHERE e.id = em.entity_id
);

DROP TABLE entity_members;
ALTER TABLE entity_members_new RENAME TO entity_members;

CREATE INDEX IF NOT EXISTS idx_entity_members_person ON entity_members(person_id);

COMMIT;
PRAGMA foreign_keys = ON;

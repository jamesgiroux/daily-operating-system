-- entity_members.entity_id must reference a real profile-agnostic
-- entity. Rebuild is required because SQLite cannot add an FK in place.

PRAGMA foreign_keys = OFF;
BEGIN;

-- Legacy project rows may predate the project -> entities mirror. Restore every
-- missing mirror before the FK rebuild so project entities remain canonical
-- even when the legacy project currently has no members.
INSERT OR IGNORE INTO entities (id, name, entity_type, tracker_path, updated_at)
SELECT p.id, p.name, 'project', p.tracker_path, COALESCE(p.updated_at, datetime('now'))
FROM projects p
WHERE NOT EXISTS (
    SELECT 1
    FROM entities e
    WHERE e.id = p.id
);

-- Preserve any membership that still cannot satisfy the FK after the project
-- mirror backfill. These rows cannot stay in entity_members once the FK is
-- active, but they must remain queryable for manual repair instead of being
-- silently discarded during upgrade.
CREATE TABLE IF NOT EXISTS entity_members_migration_145_orphans (
    entity_id TEXT NOT NULL,
    person_id TEXT NOT NULL,
    relationship_type TEXT DEFAULT 'associated',
    reason TEXT NOT NULL,
    surfaced_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (entity_id, person_id)
);

INSERT OR IGNORE INTO entity_members_migration_145_orphans (
    entity_id,
    person_id,
    relationship_type,
    reason
)
SELECT
    em.entity_id,
    em.person_id,
    em.relationship_type,
    'missing_entity_after_project_mirror_backfill'
FROM entity_members em
WHERE NOT EXISTS (
    SELECT 1
    FROM entities e
    WHERE e.id = em.entity_id
);

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
CREATE INDEX IF NOT EXISTS idx_entity_members_migration_145_orphans_person
ON entity_members_migration_145_orphans(person_id);

COMMIT;
PRAGMA foreign_keys = ON;

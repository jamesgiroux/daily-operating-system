CREATE TABLE IF NOT EXISTS legacy_user_note_migration_audit (
    legacy_entry_id TEXT PRIMARY KEY,
    claim_id TEXT NOT NULL UNIQUE,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    legacy_created_at TEXT NOT NULL,
    legacy_updated_at TEXT,
    migrated_at TEXT NOT NULL DEFAULT (datetime('now')),
    status TEXT NOT NULL DEFAULT 'migrated'
);

CREATE INDEX IF NOT EXISTS idx_legacy_user_note_migration_audit_entity
    ON legacy_user_note_migration_audit(entity_type, entity_id);

CREATE INDEX IF NOT EXISTS idx_claims_user_note_entity_read
    ON intelligence_claims(
        lower(json_extract(subject_ref, '$.kind')),
        json_extract(subject_ref, '$.id'),
        created_at DESC
    )
    WHERE claim_type = 'user_note'
      AND claim_state = 'active'
      AND surfacing_state = 'active'
      AND json_valid(subject_ref) = 1;

DROP TABLE IF EXISTS entity_context_entries_frozen_dos411;

CREATE TABLE entity_context_entries_frozen_dos411 (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    embedding BLOB,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT entity_context_entries_write_frozen_dos411
        CHECK (created_at < '0001-01-01')
);

PRAGMA ignore_check_constraints = ON;

INSERT INTO entity_context_entries_frozen_dos411 (
    id, entity_type, entity_id, title, content, embedding, created_at, updated_at
)
SELECT id, entity_type, entity_id, title, content, embedding, created_at, updated_at
FROM entity_context_entries;

PRAGMA ignore_check_constraints = OFF;

DROP TABLE entity_context_entries;

ALTER TABLE entity_context_entries_frozen_dos411 RENAME TO entity_context_entries;

CREATE INDEX IF NOT EXISTS idx_entity_context_entity
    ON entity_context_entries (entity_type, entity_id);

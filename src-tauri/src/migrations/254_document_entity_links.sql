-- v1.4.5 W1-C — document/entity link table.
-- Per L0 packet V1.3 §6. Relational table (not a claim per L0 question 10 cycle-2 resolution).
-- Tombstone semantics: partial UNIQUE on rejected=0 + service-layer BEGIN IMMEDIATE guard.

CREATE TABLE IF NOT EXISTS document_entity_links (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    link_id                 TEXT NOT NULL UNIQUE,
    file_id                 TEXT NOT NULL,
    entity_type             TEXT NOT NULL,                  -- snake_case per crate::entity::EntityType
    entity_id               TEXT NOT NULL,                  -- canonical v1.4.0 entity-slug
    attribution_source      TEXT NOT NULL,                  -- LinkAttributionSource serde tag
    confidence              REAL NOT NULL DEFAULT 0.5,
    rationale               TEXT,
    actor                   TEXT NOT NULL,
    user_override_actor     TEXT,                           -- nullable; populated by override_link
    user_override_at        TEXT,                           -- nullable; populated by override_link
    rejected                INTEGER NOT NULL DEFAULT 0,
    rejected_at             TEXT,
    rejected_reason         TEXT,
    created_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (file_id) REFERENCES workspace_file_lifecycle(file_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_del_file_id ON document_entity_links (file_id);
CREATE INDEX IF NOT EXISTS idx_del_entity ON document_entity_links (entity_type, entity_id);
-- Partial UNIQUE on rejected=0 prevents duplicate active links at SQL layer. Use:
--   INSERT INTO document_entity_links (...) VALUES (...)
--   ON CONFLICT (file_id, entity_type, entity_id) WHERE rejected = 0
--   DO NOTHING RETURNING link_id;
CREATE UNIQUE INDEX IF NOT EXISTS idx_del_active_unique
    ON document_entity_links (file_id, entity_type, entity_id)
    WHERE rejected = 0;
-- Lookup index for service-layer tombstone guard.
CREATE INDEX IF NOT EXISTS idx_del_rejected_lookup
    ON document_entity_links (file_id, entity_type, entity_id)
    WHERE rejected = 1;

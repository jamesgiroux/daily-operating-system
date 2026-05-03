-- Person-to-person relationship graph (ADR-0088).
CREATE TABLE IF NOT EXISTS person_relationships (
    id TEXT PRIMARY KEY,
    from_person_id TEXT NOT NULL,
    to_person_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL CHECK(relationship_type IN (
        'peer','manager','mentor',
        'collaborator','ally','partner','introduced_by'
    )),
    direction TEXT NOT NULL DEFAULT 'directed' CHECK(direction IN ('directed','symmetric')),
    confidence REAL NOT NULL DEFAULT 0.5,
    context_entity_id TEXT,
    context_entity_type TEXT,
    source TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_reinforced_at TEXT,
    FOREIGN KEY (from_person_id) REFERENCES people(id) ON DELETE CASCADE,
    FOREIGN KEY (to_person_id) REFERENCES people(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_person_relationships_from ON person_relationships(from_person_id);
CREATE INDEX IF NOT EXISTS idx_person_relationships_to ON person_relationships(to_person_id);
CREATE INDEX IF NOT EXISTS idx_person_relationships_context ON person_relationships(context_entity_id) WHERE context_entity_id IS NOT NULL;

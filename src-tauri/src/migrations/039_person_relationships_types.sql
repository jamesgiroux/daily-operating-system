-- Replace account-context relationship types with person-to-person types.
-- SQLite doesn't support ALTER CHECK, so recreate the table.

CREATE TABLE person_relationships_new (
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

-- Migrate any existing rows, mapping old types to new equivalents
INSERT INTO person_relationships_new
    SELECT id, from_person_id, to_person_id,
        CASE relationship_type
            WHEN 'champion' THEN 'ally'
            WHEN 'executive_sponsor' THEN 'manager'
            WHEN 'decision_maker' THEN 'manager'
            WHEN 'technical_evaluator' THEN 'collaborator'
            WHEN 'blocker' THEN 'peer'
            WHEN 'detractor' THEN 'peer'
            WHEN 'dependency' THEN 'collaborator'
            WHEN 'reports_to' THEN 'manager'
            ELSE relationship_type
        END,
        direction, confidence, context_entity_id, context_entity_type,
        source, created_at, updated_at, last_reinforced_at
    FROM person_relationships;

DROP TABLE person_relationships;
ALTER TABLE person_relationships_new RENAME TO person_relationships;

CREATE INDEX IF NOT EXISTS idx_person_relationships_from ON person_relationships(from_person_id);
CREATE INDEX IF NOT EXISTS idx_person_relationships_to ON person_relationships(to_person_id);
CREATE INDEX IF NOT EXISTS idx_person_relationships_context ON person_relationships(context_entity_id) WHERE context_entity_id IS NOT NULL;

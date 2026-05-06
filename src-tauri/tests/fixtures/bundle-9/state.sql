CREATE TABLE entity_context_entries (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT INTO entity_context_entries
    (id, entity_type, entity_id, title, content, created_at, updated_at)
VALUES
    ('ctx-b9-one-on-one', 'person', 'person-b9-morgan', 'Recurring one-on-one context', 'Morgan Malik wants the recurring one-on-one to track commitments before new planning topics.', '2026-05-04T14:00:00Z', '2026-05-04T14:15:00Z');

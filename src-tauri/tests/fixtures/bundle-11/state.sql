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
    ('ctx-b11-stale-account', 'account', 'acct-b11-stale', 'Old account context', 'Stale Account Example had an implementation risk recorded last year.', '2025-03-01T10:00:00Z', '2025-03-01T10:00:00Z');

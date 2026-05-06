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
    ('ctx-b12-revoked-account', 'account', 'acct-b12-revoked', 'Revoked source context', 'Revoked Account Example has context from a source that must not be rendered as fact.', '2026-05-04T09:00:00Z', '2026-05-04T09:30:00Z');

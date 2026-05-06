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
    ('ctx-b10-account', 'account', 'acct-b10-known', 'Known account context', 'Known Account Example is evaluating an integration expansion and asked for a decision-oriented meeting.', '2026-05-05T11:00:00Z', '2026-05-05T11:20:00Z');

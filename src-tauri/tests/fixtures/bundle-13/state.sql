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
    ('ctx-b13-target', 'account', 'dos287-target-example', 'Target context', 'Target Example uses the stable rollout plan.', '2026-05-05T09:00:00Z', '2026-05-05T09:30:00Z'),
    ('ctx-b13-adjacent', 'account', 'dos287-adjacent-example', 'Adjacent context', 'Adjacent Example has an unrelated infrastructure escalation.', '2026-05-05T10:00:00Z', '2026-05-05T10:30:00Z');

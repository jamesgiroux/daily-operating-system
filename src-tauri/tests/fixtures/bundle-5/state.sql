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
    ('ctx-b5-person-first', 'person', 'person-b5-riley', 'First meeting context', 'Riley Rivera is meeting DailyOS for the first time and asked to keep the agenda focused on onboarding risks.', '2026-05-05T15:00:00Z', '2026-05-05T15:30:00Z');

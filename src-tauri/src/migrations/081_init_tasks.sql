-- Track one-time initialization tasks (backfills, data migrations, etc.)
-- Separate from schema_version because these are data operations, not schema changes.

CREATE TABLE IF NOT EXISTS init_tasks (
    task_name TEXT PRIMARY KEY,
    completed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_init_tasks_completed_at ON init_tasks(completed_at);

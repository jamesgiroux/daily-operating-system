-- Migration 024: Linear issue/project sync tables (I346)
CREATE TABLE IF NOT EXISTS linear_issues (
    id TEXT PRIMARY KEY,
    identifier TEXT NOT NULL,
    title TEXT NOT NULL,
    state_name TEXT,
    state_type TEXT,
    priority INTEGER,
    priority_label TEXT,
    project_id TEXT,
    project_name TEXT,
    due_date TEXT,
    url TEXT,
    synced_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS linear_projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    state TEXT,
    url TEXT,
    synced_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_linear_issues_state ON linear_issues(state_type);
CREATE INDEX IF NOT EXISTS idx_linear_issues_project ON linear_issues(project_id);

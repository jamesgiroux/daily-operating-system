-- Migration 085: Action-to-Linear-issue link table for push-to-Linear (DOS-50).
CREATE TABLE IF NOT EXISTS action_linear_links (
    id TEXT PRIMARY KEY,
    action_id TEXT NOT NULL REFERENCES actions(id) ON DELETE CASCADE,
    linear_issue_id TEXT NOT NULL,
    linear_identifier TEXT NOT NULL,
    linear_url TEXT NOT NULL,
    pushed_at TEXT NOT NULL,
    UNIQUE(action_id)
);
CREATE INDEX IF NOT EXISTS idx_action_linear_links_action ON action_linear_links(action_id);

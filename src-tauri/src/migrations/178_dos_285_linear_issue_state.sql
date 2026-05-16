-- Linear issue state provenance for entity chapters, signal emission, and
-- meeting callouts: retain the upstream update timestamp + assignee identity
-- so claim freshness and trust factors can read source-of-truth state.
ALTER TABLE linear_issues ADD COLUMN linear_updated_at TEXT;
ALTER TABLE linear_issues ADD COLUMN assignee_id TEXT;
ALTER TABLE linear_issues ADD COLUMN assignee_name TEXT;

CREATE INDEX IF NOT EXISTS idx_linear_issues_updated
    ON linear_issues(linear_updated_at DESC);

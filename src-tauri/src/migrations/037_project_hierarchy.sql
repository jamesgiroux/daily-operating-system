-- Project hierarchy — parent_id for project nesting
ALTER TABLE projects ADD COLUMN parent_id TEXT REFERENCES projects(id);
CREATE INDEX IF NOT EXISTS idx_projects_parent_id ON projects(parent_id);

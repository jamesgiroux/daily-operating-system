-- I256: Add 'proposed' and 'archived' status values to the actions table.
--
-- SQLite cannot ALTER CHECK constraints, so we recreate the table using
-- the copy-drop-create-reinsert pattern. The new CHECK constraint adds
-- 'proposed' and 'archived' to the existing status values.

-- 1. Copy existing data into a temp table
CREATE TABLE actions_backup AS SELECT * FROM actions;

-- 2. Drop the old table (and its indexes)
DROP TABLE actions;

-- 3. Recreate with expanded CHECK constraint (preserving FK REFERENCES from migration 010)
CREATE TABLE actions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    priority TEXT CHECK(priority IN ('P1', 'P2', 'P3')) DEFAULT 'P2',
    status TEXT CHECK(status IN ('pending', 'completed', 'waiting', 'cancelled', 'proposed', 'archived')) DEFAULT 'pending',
    created_at TEXT NOT NULL,
    due_date TEXT,
    completed_at TEXT,
    account_id TEXT REFERENCES accounts(id) ON DELETE SET NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    source_type TEXT,
    source_id TEXT,
    source_label TEXT,
    context TEXT,
    waiting_on TEXT,
    updated_at TEXT NOT NULL,
    person_id TEXT REFERENCES people(id) ON DELETE SET NULL,
    needs_decision INTEGER DEFAULT 0
);

CREATE INDEX idx_actions_status ON actions(status);
CREATE INDEX idx_actions_due_date ON actions(due_date);
CREATE INDEX idx_actions_account ON actions(account_id);
CREATE INDEX idx_actions_status_due_date ON actions(status, due_date);

-- 4. Reinsert data
INSERT INTO actions SELECT * FROM actions_backup;

-- 5. Drop backup
DROP TABLE actions_backup;

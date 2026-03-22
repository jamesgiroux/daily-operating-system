-- Reconcile action status vocabulary.
--
-- Target statuses: suggested, pending, completed, archived
-- Removes: proposed (→ suggested), waiting (→ pending), cancelled (unused)
--
-- The CHECK constraint on the production DB is stale — it only allows
-- (pending, completed, waiting, cancelled) because migration 011's table
-- recreation was superseded. This migration recreates the table with the
-- correct constraint and renames statuses in one pass.

-- 0. Drop stale backup from prior migration if it exists
DROP TABLE IF EXISTS actions_backup;

-- 1. Copy existing data into a temp table
CREATE TABLE actions_backup AS SELECT * FROM actions;

-- 2. Rename statuses in the backup
UPDATE actions_backup SET status = 'suggested' WHERE status = 'proposed';
UPDATE actions_backup SET status = 'pending' WHERE status = 'waiting';
UPDATE actions_backup SET status = 'archived' WHERE status = 'cancelled';

-- 3. Drop the old table (and its indexes)
DROP TABLE actions;

-- 4. Recreate with corrected CHECK constraint
CREATE TABLE actions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    priority TEXT CHECK(priority IN ('P1', 'P2', 'P3')) DEFAULT 'P2',
    status TEXT CHECK(status IN ('suggested', 'pending', 'completed', 'archived')) DEFAULT 'pending',
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
    needs_decision INTEGER DEFAULT 0,
    rejected_at TEXT,
    rejection_source TEXT,
    is_demo INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_actions_status ON actions(status);
CREATE INDEX IF NOT EXISTS idx_actions_due_date ON actions(due_date);
CREATE INDEX IF NOT EXISTS idx_actions_account ON actions(account_id);
CREATE INDEX IF NOT EXISTS idx_actions_status_due_date ON actions(status, due_date);

-- 5. Reinsert data
INSERT INTO actions SELECT * FROM actions_backup;

-- 6. Drop backup
DROP TABLE actions_backup;

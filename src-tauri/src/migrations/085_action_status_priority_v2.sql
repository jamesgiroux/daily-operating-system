-- Migrate action status to Linear-compatible 6-status vocabulary
-- and priority from P1/P2/P3 strings to integers 0-4.
--
-- Status mapping:
--   suggested → backlog
--   pending   → unstarted
--   completed → completed (unchanged)
--   archived  → archived  (unchanged)
--   (new)     → started, cancelled
--
-- Priority mapping:
--   P1 → 1 (Urgent)
--   P2 → 3 (Medium, default)
--   P3 → 4 (Low)
--   (new) → 0 (None), 2 (High)

-- 0. Drop stale backup from prior migration if it exists
DROP TABLE IF EXISTS actions_backup;

-- 1. Copy existing data into a temp table
CREATE TABLE actions_backup AS SELECT * FROM actions;

-- 2. Remap statuses
UPDATE actions_backup SET status = 'backlog' WHERE status = 'suggested';
UPDATE actions_backup SET status = 'unstarted' WHERE status = 'pending';
-- 'completed' and 'archived' stay as-is

-- 3. Remap priorities (string → integer stored as text temporarily)
UPDATE actions_backup SET priority = '1' WHERE priority = 'P1';
UPDATE actions_backup SET priority = '3' WHERE priority = 'P2';
UPDATE actions_backup SET priority = '4' WHERE priority = 'P3';

-- 4. Drop old table and indexes
DROP TABLE actions;

-- 5. Recreate with new CHECK constraints
CREATE TABLE actions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    priority INTEGER CHECK(priority BETWEEN 0 AND 4) DEFAULT 3,
    status TEXT CHECK(status IN ('backlog', 'unstarted', 'started', 'completed', 'cancelled', 'archived')) DEFAULT 'unstarted',
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

-- 6. Reinsert data (CAST priority text to integer)
INSERT INTO actions (
    id, title, priority, status, created_at, due_date, completed_at,
    account_id, project_id, source_type, source_id, source_label,
    context, waiting_on, updated_at, person_id,
    needs_decision, rejected_at, rejection_source, is_demo
)
SELECT
    id, title, CAST(priority AS INTEGER), status, created_at, due_date, completed_at,
    account_id, project_id, source_type, source_id, source_label,
    context, waiting_on, updated_at, person_id,
    needs_decision, rejected_at, rejection_source, is_demo
FROM actions_backup;

-- 7. Drop backup
DROP TABLE actions_backup;

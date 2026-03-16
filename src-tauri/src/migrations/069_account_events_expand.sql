-- Expand account_events CHECK constraint from 4 to 16 event types.
-- SQLite requires a full table rebuild to modify CHECK constraints.

CREATE TABLE IF NOT EXISTS account_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK(event_type IN ('renewal', 'expansion', 'churn', 'downgrade')),
    event_date TEXT NOT NULL,
    arr_impact REAL,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

-- Step 1: Rebuild account_events with expanded CHECK
CREATE TABLE account_events_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK(event_type IN (
        'renewal', 'expansion', 'churn', 'downgrade',
        'go_live', 'onboarding_complete', 'kickoff',
        'ebr_completed', 'qbr_completed',
        'escalation', 'escalation_resolved',
        'champion_change', 'executive_sponsor_change',
        'contract_signed', 'pilot_start',
        'health_review'
    )),
    event_date TEXT NOT NULL,
    arr_impact REAL,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

INSERT INTO account_events_new (id, account_id, event_type, event_date, arr_impact, notes, created_at)
    SELECT id, account_id, event_type, event_date, arr_impact, notes, created_at
    FROM account_events;

DROP TABLE account_events;
ALTER TABLE account_events_new RENAME TO account_events;

-- Step 2: Add CHECK constraints to account_objectives.status
CREATE TABLE account_objectives_new (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('draft', 'active', 'completed', 'abandoned')),
    target_date TEXT,
    completed_at TEXT,
    source TEXT NOT NULL DEFAULT 'user',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO account_objectives_new SELECT * FROM account_objectives;
DROP TABLE account_objectives;
ALTER TABLE account_objectives_new RENAME TO account_objectives;

CREATE INDEX IF NOT EXISTS idx_account_objectives_account
    ON account_objectives(account_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_account_objectives_status
    ON account_objectives(status);

-- Step 3: Add CHECK constraint to account_milestones.status
CREATE TABLE account_milestones_new (
    id TEXT PRIMARY KEY,
    objective_id TEXT NOT NULL REFERENCES account_objectives(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'completed', 'skipped')),
    target_date TEXT,
    completed_at TEXT,
    auto_detect_signal TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO account_milestones_new SELECT * FROM account_milestones;
DROP TABLE account_milestones;
ALTER TABLE account_milestones_new RENAME TO account_milestones;

CREATE INDEX IF NOT EXISTS idx_account_milestones_objective
    ON account_milestones(objective_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_account_milestones_account
    ON account_milestones(account_id);

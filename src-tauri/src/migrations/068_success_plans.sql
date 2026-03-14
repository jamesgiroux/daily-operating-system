-- I551/I553: Success plan data model + template support

ALTER TABLE entity_assessment ADD COLUMN success_plan_signals_json TEXT;

CREATE TABLE IF NOT EXISTS account_objectives (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    target_date TEXT,
    completed_at TEXT,
    source TEXT NOT NULL DEFAULT 'user',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_account_objectives_account
    ON account_objectives(account_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_account_objectives_status
    ON account_objectives(status);

CREATE TABLE IF NOT EXISTS account_milestones (
    id TEXT PRIMARY KEY,
    objective_id TEXT NOT NULL REFERENCES account_objectives(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    target_date TEXT,
    completed_at TEXT,
    auto_detect_signal TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_account_milestones_objective
    ON account_milestones(objective_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_account_milestones_account
    ON account_milestones(account_id);

CREATE TABLE IF NOT EXISTS action_objective_links (
    action_id TEXT NOT NULL REFERENCES actions(id) ON DELETE CASCADE,
    objective_id TEXT NOT NULL REFERENCES account_objectives(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (action_id, objective_id)
);

CREATE INDEX IF NOT EXISTS idx_action_objective_links_objective
    ON action_objective_links(objective_id);

CREATE TABLE IF NOT EXISTS captured_commitments (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    meeting_id TEXT REFERENCES meetings(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    owner TEXT,
    target_date TEXT,
    confidence TEXT NOT NULL DEFAULT 'medium',
    source TEXT,
    consumed INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_captured_commitments_account
    ON captured_commitments(account_id, consumed, created_at DESC);

-- v1.1.0 foundations: lifecycle changes, renewal stage, products, provenance, and AI usage support.

ALTER TABLE accounts ADD COLUMN renewal_stage TEXT;

ALTER TABLE accounts ADD COLUMN arr_source TEXT;
ALTER TABLE accounts ADD COLUMN arr_updated_at TEXT;
ALTER TABLE accounts ADD COLUMN lifecycle_source TEXT;
ALTER TABLE accounts ADD COLUMN lifecycle_updated_at TEXT;
ALTER TABLE accounts ADD COLUMN contract_end_source TEXT;
ALTER TABLE accounts ADD COLUMN contract_end_updated_at TEXT;
ALTER TABLE accounts ADD COLUMN nps_source TEXT;
ALTER TABLE accounts ADD COLUMN nps_updated_at TEXT;

CREATE TABLE IF NOT EXISTS lifecycle_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    previous_lifecycle TEXT,
    new_lifecycle TEXT NOT NULL,
    previous_stage TEXT,
    new_stage TEXT,
    previous_contract_end TEXT,
    new_contract_end TEXT,
    source TEXT NOT NULL,
    confidence REAL NOT NULL,
    evidence TEXT,
    health_score_before REAL,
    health_score_after REAL,
    user_response TEXT NOT NULL DEFAULT 'pending',
    response_notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    reviewed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_lifecycle_changes_account_created
    ON lifecycle_changes(account_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_lifecycle_changes_pending
    ON lifecycle_changes(user_response, created_at DESC);

CREATE TABLE IF NOT EXISTS account_products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    name TEXT NOT NULL,
    category TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    arr_portion REAL,
    source TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.7,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_account_products_account
    ON account_products(account_id);
CREATE INDEX IF NOT EXISTS idx_account_products_name
    ON account_products(account_id, lower(name));

ALTER TABLE account_milestones ADD COLUMN completed_by TEXT;
ALTER TABLE account_milestones ADD COLUMN completion_trigger TEXT;

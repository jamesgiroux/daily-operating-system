-- Health score history for trend computation.
-- Records each recomputed health score so we can derive real trends
-- instead of the hardcoded "stable" placeholder.

CREATE TABLE IF NOT EXISTS health_score_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    score REAL NOT NULL,
    band TEXT NOT NULL,
    confidence REAL NOT NULL,
    computed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_health_history_account_time
    ON health_score_history(account_id, computed_at DESC);

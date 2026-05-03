-- Rejection learning for proposed actions.
-- Tracks patterns from rejected actions to suppress future proposals.

CREATE TABLE IF NOT EXISTS rejected_action_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT,
    pattern_type TEXT NOT NULL CHECK(pattern_type IN ('exact_title', 'keyword', 'source_fatigue')),
    pattern_value TEXT NOT NULL,
    rejection_count INTEGER NOT NULL DEFAULT 1,
    first_rejected_at TEXT NOT NULL,
    last_rejected_at TEXT NOT NULL,
    suppressed INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_rejected_patterns_account
    ON rejected_action_patterns(account_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_rejected_patterns_lookup
    ON rejected_action_patterns(account_id, pattern_type, pattern_value);

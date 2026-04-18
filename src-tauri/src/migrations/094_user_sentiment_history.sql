-- DOS-27: Sentiment journal entries.
-- Each entry is a journal record (value + optional note + timestamp).
-- Computed health band at set-time is stored for divergence analysis
-- and sparkline overlay.

CREATE TABLE IF NOT EXISTS user_sentiment_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    sentiment TEXT NOT NULL
        CHECK(sentiment IN ('strong','on_track','concerning','at_risk','critical')),
    note TEXT,
    computed_band TEXT,
    computed_score REAL,
    set_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_sentiment_history_account_time
    ON user_sentiment_history(account_id, set_at DESC);

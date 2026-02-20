-- I318: Thread position tracking ("ball in your court")

CREATE TABLE IF NOT EXISTS email_threads (
    thread_id TEXT PRIMARY KEY,
    subject TEXT NOT NULL DEFAULT '',
    last_sender_email TEXT NOT NULL DEFAULT '',
    last_message_date TEXT NOT NULL DEFAULT '',
    message_count INTEGER NOT NULL DEFAULT 1,
    user_is_last_sender INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_email_threads_position
    ON email_threads(user_is_last_sender, updated_at DESC);

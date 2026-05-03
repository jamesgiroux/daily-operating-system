-- Migration 053: App state singleton + demo data markers
--
-- app_state is a single-row table holding onboarding and demo mode state.
-- is_demo columns on accounts/actions/people allow selective cleanup.

CREATE TABLE IF NOT EXISTS app_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    demo_mode_active INTEGER NOT NULL DEFAULT 0,
    has_completed_tour INTEGER NOT NULL DEFAULT 0,
    wizard_completed_at TEXT,
    wizard_last_step TEXT
);

INSERT OR IGNORE INTO app_state (id) VALUES (1);

ALTER TABLE accounts ADD COLUMN is_demo INTEGER NOT NULL DEFAULT 0;
ALTER TABLE actions ADD COLUMN is_demo INTEGER NOT NULL DEFAULT 0;
ALTER TABLE people ADD COLUMN is_demo INTEGER NOT NULL DEFAULT 0;

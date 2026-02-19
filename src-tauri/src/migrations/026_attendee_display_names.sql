-- Migration 026: Attendee display names for hygiene name resolution (I342)
--
-- Stores display names from Google Calendar attendees so the hygiene scanner
-- can resolve unnamed people (e.g., "jgiroux" → "James Giroux").

CREATE TABLE IF NOT EXISTS attendee_display_names (
    email        TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    last_seen    TEXT NOT NULL DEFAULT (datetime('now'))
);

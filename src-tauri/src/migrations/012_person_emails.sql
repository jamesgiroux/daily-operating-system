-- Migration 012: person_emails table for email aliasing / deduplication.
--
-- Tracks all known email addresses per person. The primary email lives in
-- people.email; additional aliases (same local part across sibling domains)
-- are recorded here so that future calendar events resolve to the existing
-- person instead of creating a duplicate.

CREATE TABLE IF NOT EXISTS person_emails (
    person_id TEXT NOT NULL,
    email TEXT NOT NULL COLLATE NOCASE,
    is_primary INTEGER NOT NULL DEFAULT 0,
    added_at TEXT NOT NULL,
    PRIMARY KEY (person_id, email),
    FOREIGN KEY (person_id) REFERENCES people(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_person_emails_email
    ON person_emails(email COLLATE NOCASE);

-- Backfill from existing people.email so alias lookups work immediately.
INSERT OR IGNORE INTO person_emails (person_id, email, is_primary, added_at)
    SELECT id, email, 1, datetime('now') FROM people;

-- DOS-308 cycle-3: distinguish unresolved quarantine entries (still need
-- operator attention) from resolved entries (audit trail of completed
-- remediation). NULL means the row was created but not yet processed by
-- the remediation script; a timestamp means the row is the remediation
-- audit record.
--
-- Note: ALTER TABLE ADD COLUMN with DEFAULT NULL is the only safe form
-- under SQLite; rows pre-existing this migration will have NULL, which
-- the gate treats as unresolved (preserves cycle-2 semantics for any
-- operator who created quarantine rows manually before this column
-- existed).

ALTER TABLE suppression_tombstones_quarantine ADD COLUMN resolved_at TEXT;
CREATE INDEX IF NOT EXISTS idx_quarantine_unresolved
    ON suppression_tombstones_quarantine(resolved_at)
    WHERE resolved_at IS NULL;

-- DOS-308 cycle-3: distinguish unresolved quarantine entries (still need
-- operator attention) from resolved entries (audit trail of completed
-- remediation). NULL means the row was created but not yet processed by
-- the remediation script; a timestamp means the row is the remediation
-- audit record.
--
-- DOS-308 cycle-4: this migration intentionally adds ONLY the column.
-- The partial index `idx_quarantine_unresolved` lives in migration 128
-- so a partial-failure retry between ALTER TABLE and CREATE INDEX cannot
-- record the version complete with the index missing (the runner's
-- duplicate-column swallow on retry would otherwise skip the index
-- creation in a single-batch migration).

ALTER TABLE suppression_tombstones_quarantine ADD COLUMN resolved_at TEXT;

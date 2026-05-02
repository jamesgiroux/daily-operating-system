-- DOS-308 cycle-4: partial index for the quarantine gate's
-- `WHERE resolved_at IS NULL` count query. Split out from migration 127
-- so a partial-failure retry cannot record v127 complete with the column
-- added but the index missing. CREATE INDEX IF NOT EXISTS makes this
-- migration safely re-runnable.

CREATE INDEX IF NOT EXISTS idx_quarantine_unresolved
    ON suppression_tombstones_quarantine(resolved_at)
    WHERE resolved_at IS NULL;

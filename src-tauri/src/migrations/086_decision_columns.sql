-- DOS-17: Add decision_owner and decision_stakes columns to actions table.
-- The needs_decision column already exists from the baseline schema.
ALTER TABLE actions ADD COLUMN decision_owner TEXT;
ALTER TABLE actions ADD COLUMN decision_stakes TEXT;

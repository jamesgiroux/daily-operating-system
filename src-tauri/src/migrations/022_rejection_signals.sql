-- I334: Capture rejection signals for correction learning (I307).
--
-- When a user rejects a proposed action, store when and where so the
-- system can learn which AI suggestions to suppress in future.

ALTER TABLE actions ADD COLUMN rejected_at TEXT;
ALTER TABLE actions ADD COLUMN rejection_source TEXT;

CREATE INDEX IF NOT EXISTS idx_actions_rejected ON actions(rejected_at)
    WHERE rejected_at IS NOT NULL;

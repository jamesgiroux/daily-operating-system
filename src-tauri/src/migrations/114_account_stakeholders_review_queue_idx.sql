-- DOS-258 Lane A: account_stakeholders status + confidence columns + queue index.
--
-- The C2 cascade writes auto-suggested stakeholders from meeting attendance
-- and email correspondence with status='pending_review'. Users confirm or
-- dismiss from the account detail page review queue.
--
-- Existing rows (confirmed stakeholders added before Lane A) default to
-- status='active' — they are not placed back into the queue. The confidence
-- column is nullable; it is NULL for rows added by users directly, and
-- populated (0.0–1.0) for rows written by the auto-suggest engine.
--
-- The (account_id, status) index supports the review-queue read:
--   SELECT ... FROM account_stakeholders WHERE account_id = ? AND status = 'pending_review'
-- Without this index that query would be a full scan of the stakeholders table,
-- which grows linearly with account count.

ALTER TABLE account_stakeholders ADD COLUMN status TEXT NOT NULL DEFAULT 'active';
ALTER TABLE account_stakeholders ADD COLUMN confidence REAL;

CREATE INDEX IF NOT EXISTS idx_account_stakeholders_review_queue
    ON account_stakeholders (account_id, status);

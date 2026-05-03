--  scope retry transitions to a single batch so
-- concurrent refreshes and crash-recovery can tell "our rows" apart from
-- "someone else's rows stuck in pending_retry".
--
-- Before this change:
--   - `mark_failed_for_retry` flipped every `failed` row to `pending_retry`
--     without a correlation key. A second refresh starting while the first
--     was in flight would reuse the same rows.
--   - `rollback_pending_retry` was best-effort log-only in
--     `services::emails::refresh_emails`, so a crash between
--     `mark_failed_for_retry` and the finalize/rollback left rows stranded
--     in `pending_retry` forever. Stats counted them as failed (UI showed
--     the Retry notice), but `retry_failed_emails` counted only `failed`
--     rows and returned 0 — orphaning the stuck rows.
--
-- `retry_batch_id` identifies the refresh that owns a transitioning row.
-- `retry_started_at` bounds staleness so refresh startup can recover rows
-- left by a crashed predecessor without needing a separate lockfile or
-- process registry.

ALTER TABLE emails ADD COLUMN retry_batch_id TEXT;
ALTER TABLE emails ADD COLUMN retry_started_at TEXT;

-- Lookups scoped to a batch are rare but must be fast (one write on refresh
-- start, one on refresh end, one recovery scan). Partial index keeps cost
-- bounded — we only index rows mid-retry.
CREATE INDEX IF NOT EXISTS idx_emails_retry_batch
    ON emails(retry_batch_id)
    WHERE retry_batch_id IS NOT NULL;

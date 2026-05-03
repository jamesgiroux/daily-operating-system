-- two-part schema update bundled into one migration to
-- minimize collision risk with parallel work in flight.
--
-- Part A — `risk_briefing_jobs.attempt_id`:
--   Codex flagged that the prior lifecycle used three spawned tasks keyed
--   only by `account_id`. Two rapid `retry_risk_briefing` calls could
--   overwrite each other's terminal state (last-write-wins), and there was
--   no way to tell the "I'm handling the outer attempt" writer apart from
--   a stale writer. The `attempt_id` column lets us do compare-and-set:
--   a lifecycle runner stamps its UUID at enqueue-time and will only
--   advance a row whose attempt_id still matches its own. A superseding
--   retry overwrites attempt_id first, so the old runner's updates fail
--   the CAS and log+exit instead of corrupting the new attempt.
--
-- Part B — `health_recompute_pending`:
--   The previous in-memory debouncer
--   (services::health_debouncer::HealthRecomputeDebouncer) lost pending
--   recomputes on app crash or exit inside the 2s window. A committed
--   field edit could silently fail to recompute health. This table
--   persists the "pending recompute" marker across restarts; startup
--   drains the table and runs recomputes for any rows that survived a
--   crash. Scheduling writes the marker, a successful recompute clears
--   it — so the worst case is one redundant recompute on clean restart,
--   not a lost one.
--
-- Intelligence Loop 5Q check:
-- 1. Signals: no — these are observational (job lifecycle) and operational
--    (pending queue) columns, not propagating signals.
-- 2. Health dimensions: no — they govern WHEN health runs, not inputs.
-- 3. Intel/prep context: no — not user-facing data surfaces.
-- 4. Briefing callouts: no — unchanged CALLOUT_SIGNAL_TYPES.
-- 5. Bayesian weights: no — these rows are not feedback signals.

-- Part A
ALTER TABLE risk_briefing_jobs ADD COLUMN attempt_id TEXT;

-- Part B
CREATE TABLE IF NOT EXISTS health_recompute_pending (
    account_id    TEXT PRIMARY KEY,
    requested_at  TEXT NOT NULL
);

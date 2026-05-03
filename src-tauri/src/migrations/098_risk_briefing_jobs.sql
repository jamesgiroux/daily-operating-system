-- Regression guard: Persist risk-briefing job lifecycle so the UI can surface
-- failures instead of silently dropping them into the log stream.
--
-- Previous behavior: `set_user_health_sentiment` spawned a fire-and-forget
-- task that logged Ok/Err. If the PTY pipeline failed (timeout, missing CLI,
-- malformed JSON) the user saw nothing — no retry, no error, no "in progress"
-- indicator. This table records every enqueued briefing and its terminal
-- state so `get_account_detail` can return it and a new `retry_risk_briefing`
-- command can re-run a failed job on demand.
--
-- Intelligence Loop 5Q check:
-- 1. Signals: yes — `services::accounts::enqueue_risk_briefing` already emits
--    `field_updated`; this table is observational only, no new propagation rules.
-- 2. Health dimensions: no — job status does not feed scoring.
-- 3. Intel/prep context: no — briefing CONTENT is in the reports table; this
--    table tracks the async generation pipeline only.
-- 4. Briefing callouts: no — CALLOUT_SIGNAL_TYPES is unchanged.
-- 5. Bayesian weights: no — not a source of truth signal.

CREATE TABLE IF NOT EXISTS risk_briefing_jobs (
    account_id        TEXT PRIMARY KEY,
    status            TEXT NOT NULL CHECK (status IN ('enqueued', 'running', 'complete', 'failed')),
    enqueued_at       TEXT NOT NULL,
    completed_at      TEXT,
    error_message     TEXT
);

-- Lookup: list recent failures across accounts for diagnostics.
CREATE INDEX IF NOT EXISTS idx_risk_briefing_jobs_status
    ON risk_briefing_jobs (status, enqueued_at DESC);

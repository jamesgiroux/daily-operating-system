CREATE TABLE IF NOT EXISTS invalidation_jobs (
    id                            TEXT PRIMARY KEY,
    job_kind                      TEXT NOT NULL
                                      CHECK (job_kind IN (
                                          'signal_invalidation',
                                          'claim_recompute',
                                          'transform',
                                          'maintenance_apply',
                                          'outbox_replay'
                                      )),
    operation                     TEXT NOT NULL,
    status                        TEXT NOT NULL
                                      CHECK (status IN (
                                          'pending',
                                          'running',
                                          'completed',
                                          'failed',
                                          'dead_lettered',
                                          'cycle_detected'
                                      )),
    priority                      INTEGER NOT NULL DEFAULT 0,
    chain_id                      TEXT NOT NULL,
    parent_job_id                 TEXT REFERENCES invalidation_jobs(id),
    successor_of_job_id           TEXT REFERENCES invalidation_jobs(id),
    origin_signal_id              TEXT REFERENCES signal_events(id),
    depth                         INTEGER NOT NULL DEFAULT 0,
    chain_ancestry_json           TEXT NOT NULL DEFAULT '[]',

    idempotency_key               TEXT NOT NULL,
    coalescing_key                TEXT,
    subject_type                  TEXT NOT NULL,
    subject_id                    TEXT NOT NULL,
    ability_id                    TEXT NOT NULL,
    ability_version               TEXT NOT NULL,
    source_claim_version          INTEGER NOT NULL DEFAULT 0,
    latest_source_claim_version   INTEGER NOT NULL DEFAULT 0,
    source_asof                   TEXT,
    input_snapshot_hash           TEXT,
    provider_fingerprint          TEXT,
    prompt_fingerprint            TEXT,
    payload_json                  TEXT NOT NULL DEFAULT '{}',

    first_signal_id               TEXT,
    latest_signal_id              TEXT,
    raw_signal_count              INTEGER NOT NULL DEFAULT 1,
    covered_since_at              TEXT NOT NULL,
    covered_until_at              TEXT NOT NULL,

    attempts                      INTEGER NOT NULL DEFAULT 0,
    max_attempts                  INTEGER NOT NULL DEFAULT 5,
    next_run_at                   TEXT NOT NULL,
    lease_owner                   TEXT,
    lease_expires_at              TEXT,
    claimed_at                    TEXT,
    completed_at                  TEXT,
    dead_lettered_at              TEXT,
    last_error                    TEXT,
    stale_marker_json             TEXT,
    created_at                    TEXT NOT NULL,
    updated_at                    TEXT NOT NULL,

    CHECK (attempts >= 0),
    CHECK (max_attempts > 0),
    CHECK (depth >= 0),
    CHECK (raw_signal_count > 0),
    CHECK (latest_source_claim_version >= source_claim_version)
);

CREATE INDEX IF NOT EXISTS idx_invalidation_jobs_status_run
    ON invalidation_jobs(status, next_run_at, created_at);

CREATE INDEX IF NOT EXISTS idx_invalidation_jobs_chain
    ON invalidation_jobs(chain_id, depth, created_at);

CREATE INDEX IF NOT EXISTS idx_invalidation_jobs_origin_signal
    ON invalidation_jobs(origin_signal_id);

CREATE INDEX IF NOT EXISTS idx_invalidation_jobs_dead_letter
    ON invalidation_jobs(dead_lettered_at, updated_at)
    WHERE status = 'dead_lettered';

CREATE UNIQUE INDEX IF NOT EXISTS ux_invalidation_jobs_pending_coalescing
    ON invalidation_jobs(coalescing_key)
    WHERE status = 'pending'
      AND coalescing_key IS NOT NULL
      AND successor_of_job_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS ux_invalidation_jobs_pending_successor
    ON invalidation_jobs(coalescing_key, successor_of_job_id)
    WHERE status = 'pending'
      AND coalescing_key IS NOT NULL
      AND successor_of_job_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS ux_invalidation_jobs_active_idempotency
    ON invalidation_jobs(idempotency_key)
    WHERE status IN ('pending', 'running')
      AND successor_of_job_id IS NULL;

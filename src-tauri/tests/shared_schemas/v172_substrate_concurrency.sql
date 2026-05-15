-- Test-fixture mirror of v172 substrate concurrency contract (claim_version
-- watermark on intelligence_claims + mutation_attempts chokepoint +
-- composition_versions watermark + version_events outbox). The production
-- migration lives at src/migrations/v172_dos_567_w4b_versions_and_outbox.rs;
-- this SQL file is its schema-only twin for tests that hand-roll an inline
-- schema rather than calling run_migrations(). commit_claim and
-- commit_composition now write to mutation_attempts + version_events via the
-- MutationGuard protocol, so any test exercising those services must have
-- these surfaces present.
--
-- Apply this AFTER the intelligence_claims table has been created (the
-- ALTER TABLE below depends on it).

ALTER TABLE intelligence_claims
    ADD COLUMN claim_version INTEGER NOT NULL DEFAULT 0
    CHECK (claim_version BETWEEN 0 AND 9223372036854775807);

CREATE TABLE IF NOT EXISTS mutation_attempts (
    mutation_id TEXT PRIMARY KEY,
    claim_id TEXT,
    composition_id TEXT,
    cursor TEXT NOT NULL UNIQUE,
    started_at TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('in_flight', 'committed', 'aborted')),
    finalized_at TEXT,
    CHECK (
        (status = 'in_flight' AND finalized_at IS NULL)
        OR (status != 'in_flight' AND finalized_at IS NOT NULL)
    ),
    CHECK ((claim_id IS NOT NULL) != (composition_id IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS idx_mutation_attempts_in_flight
    ON mutation_attempts (started_at)
    WHERE status = 'in_flight';

CREATE TABLE IF NOT EXISTS composition_versions (
    composition_id TEXT PRIMARY KEY,
    composition_version INTEGER NOT NULL,
    generated_at TEXT NOT NULL,
    generated_by_invocation_id TEXT NOT NULL,
    generated_by_actor_kind TEXT NOT NULL,
    CHECK (composition_version BETWEEN 1 AND 9223372036854775807)
);

CREATE TABLE IF NOT EXISTS version_events (
    event_seq INTEGER PRIMARY KEY AUTOINCREMENT,
    cursor TEXT NOT NULL UNIQUE CHECK (length(cursor) = 36 AND cursor GLOB '*-*-*-*-*'),
    event_kind TEXT NOT NULL CHECK (event_kind IN (
        'claim.updated',
        'claim.corrected',
        'claim.superseded',
        'claim.tombstoned',
        'claim.write_rejected',
        'claim.conflict_detected',
        'composition.updated',
        'composition.write_rejected',
        'mutation_aborted'
    )),
    claim_id TEXT,
    composition_id TEXT,
    previous_version INTEGER,
    current_version INTEGER NOT NULL,
    reason TEXT,
    scope_redacted INTEGER NOT NULL CHECK (scope_redacted IN (0, 1)),
    correction_event_log_id TEXT,
    mutation_id TEXT,
    created_at TEXT NOT NULL,
    actor_kind TEXT NOT NULL CHECK (actor_kind IN ('user', 'agent', 'admin', 'system', 'surface_client')),
    CHECK ((claim_id IS NOT NULL) != (composition_id IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS idx_version_events_claim
    ON version_events (claim_id, current_version);

CREATE INDEX IF NOT EXISTS idx_version_events_composition
    ON version_events (composition_id, current_version);

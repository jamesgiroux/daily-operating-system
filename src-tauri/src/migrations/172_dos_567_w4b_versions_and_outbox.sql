ALTER TABLE intelligence_claims
    ADD COLUMN claim_version INTEGER NOT NULL DEFAULT 0
        CHECK (claim_version BETWEEN 0 AND 9223372036854775807);

CREATE TABLE mutation_attempts (
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

CREATE INDEX idx_mutation_attempts_in_flight
    ON mutation_attempts (started_at)
    WHERE status = 'in_flight';

CREATE TABLE composition_versions (
    composition_id TEXT PRIMARY KEY,
    composition_version INTEGER NOT NULL,
    generated_at TEXT NOT NULL,
    generated_by_invocation_id TEXT NOT NULL,
    generated_by_actor_kind TEXT NOT NULL,
    CHECK (composition_version BETWEEN 1 AND 9223372036854775807)
);

CREATE TABLE version_events (
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

CREATE INDEX idx_version_events_claim
    ON version_events (claim_id, current_version);

CREATE INDEX idx_version_events_composition
    ON version_events (composition_id, current_version);

UPDATE intelligence_claims
SET claim_version = 1
WHERE claim_version = 0;

INSERT INTO mutation_attempts (
    mutation_id,
    claim_id,
    composition_id,
    cursor,
    started_at,
    status,
    finalized_at
)
SELECT
    'migration-172-' || id,
    id,
    NULL,
    lower(
        hex(randomblob(4)) || '-' ||
        hex(randomblob(2)) || '-' ||
        '4' || substr(hex(randomblob(2)), 2) || '-' ||
        substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' ||
        hex(randomblob(6))
    ),
    datetime('now'),
    'committed',
    datetime('now')
FROM intelligence_claims
WHERE claim_version = 1;

INSERT INTO version_events (
    cursor,
    event_kind,
    claim_id,
    previous_version,
    current_version,
    reason,
    scope_redacted,
    mutation_id,
    created_at,
    actor_kind
)
SELECT
    cursor,
    'claim.updated',
    claim_id,
    0,
    1,
    'claim_version_backfill',
    0,
    mutation_id,
    finalized_at,
    'system'
FROM mutation_attempts
WHERE mutation_id LIKE 'migration-172-%';

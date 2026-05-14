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

-- Per packet §11 + L2 cycle-2 H5: backfill MUST NOT emit a per-row
-- mutation_attempts + version_events pair (2N rows + N synthetic
-- claim.updated events would wake DOS-589 subscribers with a false
-- invalidation storm on first migration). Backfill is a one-shot
-- substrate operation, not an observable claim mutation.
--
-- Instead we emit a single summary `claim_version_backfill` audit row.
-- We use a synthetic mutation_attempts + version_events pair anchored at
-- a sentinel claim_id ('__migration_172_backfill__'). The single row's
-- `reason` carries `claim_version_backfill:row_count=<N>:migration_version=172`
-- per ac §34 audit-detail shape. Subscribers filter on event_kind =
-- 'claim.updated' AND claim_id = '__migration_172_backfill__' as the
-- single-row backfill marker — they do NOT see N spurious events.
--
-- The doctor outbox-integrity check treats `claim_version = 1` as the
-- post-migration baseline that doesn't require a per-row outbox entry.
INSERT INTO mutation_attempts (
    mutation_id,
    claim_id,
    composition_id,
    cursor,
    started_at,
    status,
    finalized_at
)
VALUES (
    'migration-172-backfill-summary',
    '__migration_172_backfill__',
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
);

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
    '__migration_172_backfill__',
    0,
    1,
    'claim_version_backfill:row_count=' || (SELECT COUNT(*) FROM intelligence_claims WHERE claim_version = 1) || ':migration_version=172',
    0,
    mutation_id,
    finalized_at,
    'system'
FROM mutation_attempts
WHERE mutation_id = 'migration-172-backfill-summary';

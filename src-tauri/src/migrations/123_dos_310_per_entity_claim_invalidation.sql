-- DOS-310: per-entity claim invalidation primitive (replaces singleton-counter
-- entity_graph_version trigger extension that DOS-7 originally proposed).
--
-- Codex round 1 finding 4 + round 2 finding 6: entity_graph_version is a
-- singleton counter; bumping it on every claim write thrashes unrelated
-- entity-linking evaluations. This migration introduces per-entity
-- claim_version columns + a shared migration_state(global_claim_epoch) row.
--
-- Architecture (Option A, picked per Codex round 2 finding 6):
--   - Per-entity claim_version: sync transactional, matches single-source-of-truth
--     posture. Readers check entity.claim_version off the row they already loaded.
--   - global_claim_epoch (in migration_state): for SubjectRef::Global claims.
--     Spine restriction: no claim_type registers canonical_subject_types
--     containing Global; the row is structurally available for v1.4.1+.
--   - SubjectRef::Multi: deterministic lock ordering (Account < Meeting < Person
--     < Project) prevents deadlocks under concurrent commits.
--
-- The Rust helpers in src-tauri/src/db/claim_invalidation.rs are the SOLE
-- writers of these counters. CI lint enforces.

ALTER TABLE accounts         ADD COLUMN claim_version INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects         ADD COLUMN claim_version INTEGER NOT NULL DEFAULT 0;
ALTER TABLE people           ADD COLUMN claim_version INTEGER NOT NULL DEFAULT 0;
ALTER TABLE meetings ADD COLUMN claim_version INTEGER NOT NULL DEFAULT 0;

-- migration_state is shared with DOS-311 (which also writes a 'schema_epoch'
-- row). CREATE IF NOT EXISTS keeps both migrations independent.
CREATE TABLE IF NOT EXISTS migration_state (
    key   TEXT PRIMARY KEY,
    value INTEGER NOT NULL
);

INSERT OR IGNORE INTO migration_state (key, value) VALUES ('global_claim_epoch', 0);

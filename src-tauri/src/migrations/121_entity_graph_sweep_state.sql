-- Tier 4: record the last time the startup rescan swept stale
-- weak-primary entity links. Consumed by the one-shot post-upgrade sweep
-- that self-corrects existing production data after the evidence-hierarchy
-- fix shipped.
--
-- ALTER TABLE ADD COLUMN is idempotent when guarded by a schema inspection,
-- but sqlite itself will error on duplicate-column-name. The migration
-- framework only runs this once (tracked in schema_version), so a plain
-- ALTER is safe.

ALTER TABLE entity_graph_version ADD COLUMN last_migration_sweep_at TEXT;

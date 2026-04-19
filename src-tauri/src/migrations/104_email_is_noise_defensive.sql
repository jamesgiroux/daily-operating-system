-- DOS-247 follow-up: defensive ALTER for the `is_noise` column.
--
-- Some users hit "no such column: is_noise" when migration v105 (the
-- recovery UPDATE) ran — schema_version said v103 was applied but the
-- ALTER never materialized on their DB. This single-statement ALTER is
-- tolerated by the migration framework as "duplicate column name" if
-- the column already exists, so it's a no-op for normal upgrade paths
-- and a real fix for the broken-history case.
--
-- Must remain a single ALTER TABLE statement to qualify for the
-- framework's idempotence tolerance (see run_migrations in
-- src-tauri/src/migrations.rs around line 825).

ALTER TABLE emails ADD COLUMN is_noise INTEGER NOT NULL DEFAULT 0;

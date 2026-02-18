# ADR-0071: Schema Migration Framework

**Status:** Accepted
**Date:** 2026-02-13
**Deciders:** James, Claude

## Context

The DailyOS SQLite database had ~22 inline `ALTER TABLE` migrations scattered throughout `db.rs::open_at()`, plus a separate `schema.sql` file for the initial table creation. Every startup ran all migrations unconditionally. There was no version tracking, no way to know which migrations had been applied, and no forward-compatibility guard to prevent an older app version from opening a database created by a newer version.

This was acceptable for a single-user alpha (4 testers), but becomes untenable for beta (20-50 users) where:
- Schema changes must be reliable and traceable
- Users on different versions must not silently corrupt each other's databases
- The auto-updater (ADR-0072) means databases will transition between versions without manual intervention

## Decision

1. **Numbered SQL migration files** in `src-tauri/src/migrations/`, embedded at compile time via `include_str!`. Each migration has a sequential integer version number and a `.sql` file.

2. **`schema_version` table** tracks which migrations have been applied. Created automatically on first run.

3. **Migration 001 is the baseline** — consolidates `schema.sql` + all 22 inline ALTER TABLE migrations into one complete `CREATE TABLE IF NOT EXISTS` schema. For new databases, migration 001 creates everything. For existing databases, it never runs (see bootstrap).

4. **Bootstrap detection** — if the `actions` table exists but `schema_version` doesn't, the database predates the migration framework. The bootstrap function marks migration 001 as applied without executing it. Existing data is untouched.

5. **Forward-compatibility guard** — if `schema_version` is higher than the highest known migration, `run_migrations()` returns an error telling the user to update DailyOS. Prevents older binaries from running against newer schemas.

6. **Pre-migration backup** — before applying any pending migrations, a hot copy is created at `<db_path>.pre-migration.bak` using `rusqlite::backup::Backup`. Skipped for in-memory databases (tests).

7. **`db.rs::open_at()` simplified** — the ~160 lines of inline migrations and `schema.sql` execution are replaced with a single `migrations::run_migrations(&conn)` call. Three idempotent Rust backfill functions remain as post-migration startup tasks (to be removed once all alpha users are past v0.7.3).

8. **`schema.sql` deleted** — absorbed into `migrations/001_baseline.sql`.

## How to Add a Migration

1. Create `src-tauri/src/migrations/NNN_description.sql`
2. Add to `MIGRATIONS` array in `migrations.rs`
3. Document in `MIGRATIONS.md`
4. Test with both fresh and existing databases

## Consequences

- Schema evolution is now safe, versioned, and auditable
- Adding a new migration is a two-step process (SQL file + array entry) instead of scattering ALTER TABLE calls in Rust code
- Forward-compat guard prevents silent database corruption across versions
- Pre-migration backup provides rollback safety for alpha/beta users
- Bootstrap detection means zero disruption for existing databases — no data loss, no re-creation
- The three Rust backfill functions (`normalize_reviewed_prep_keys`, `backfill_meeting_identity`, `backfill_meeting_user_layer`) are tech debt that should be removed once all users are past v0.7.3
- Future migrations must be additive (SQLite doesn't support `DROP COLUMN` before 3.35.0, and `ALTER TABLE` is limited) — destructive schema changes require create-new-table + copy pattern

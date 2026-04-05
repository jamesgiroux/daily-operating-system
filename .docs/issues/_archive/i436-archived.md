# I436 — Workspace File Deprecation — DB as Sole Source of Truth

**Status:** Open
**Priority:** P2
**Version:** Post-1.0 (unscheduled)
**Area:** Backend / Architecture

## Summary

DailyOS maintains a parallel file system alongside the SQLite DB: `Accounts/*/dashboard.md`, `People/*/person.json`, `Projects/*/context.json`, and the `_today/data/*.json` daily pipeline files. ADR-0086 established the DB as the source of truth for entity intelligence, but the workspace files are still written and read in several code paths. This creates the "stale data on disk" class of bugs documented in MEMORY.md — code changes to the pipeline do not retroactively fix already-generated files, and the DB and disk can diverge silently.

This issue removes the dual-write, eliminates workspace sync on startup, and makes the DB the only storage layer. The workspace directory becomes optional. The `_today/data/` files are eliminated in favor of direct DB reads.

This is a large blast-radius change. It is post-1.0 and unscheduled deliberately — it should follow a stability period during which the DB as source of truth (established in ADR-0086) is battle-tested.

## Acceptance Criteria

1. `sync_people_from_workspace()` and `sync_accounts_from_workspace()` are removed from the startup flow in `state.rs`. On startup, entity data is read from the DB only. No file system walk of the workspace directory occurs during initialization.

2. `write_person_json()`, `write_person_markdown()`, and `write_account_markdown()` are removed from the entity update path (or relocated behind an explicit "Export to workspace" Tauri command only, not called automatically). All entity updates — from signals, from user edits, from enrichment — write to the DB only.

3. The `_today/data/` pipeline files are eliminated: `schedule.json`, `actions.json`, and `emails.json` are no longer written by the daily executor or read by the frontend. The dashboard data service reads directly from the DB via existing query infrastructure. The Tauri commands `load_schedule_json`, `load_actions_json`, and `load_emails_json` are removed (or converted to DB-backed queries if the frontend calls them by name).

4. The app runs correctly with no workspace path configured. `workspace_path` in the app config is deprecated: existing values are respected for the migration window but the field is marked deprecated in the config schema. A DailyOS-managed data directory at `~/.dailyos/` is the sole required path.

5. On first launch after upgrading, a one-time migration runs:
   - Reads entity data from existing workspace markdown/JSON files.
   - Imports any data not already present in the DB.
   - Moves the original workspace files to `~/.dailyos/workspace-archive/` (not deleted — user can inspect or recover).
   - Records migration completion in the settings DB. Does not re-run on subsequent launches.

6. `cargo test` passes. The running app shows identical data in the daily briefing, meeting cards, account detail, and person detail pages before and after the migration.

## Dependencies

No hard technical blockers. However:
- This is post-1.0 and should not be scheduled until the DB-as-source-of-truth pattern from ADR-0086 has been stable for at least one release cycle.
- I432/I433/I434 (intelligence provider abstraction) are independent and should land first — they share no code paths with workspace file handling.
- The I376 audit's documentation of `workspace_path` dependencies is the reference for identifying all file-reading code paths that need migration.

## Coordination

- **I456 (In-App Markdown Reader)** reads entity documents from the workspace filesystem. If I436 removes workspace files, I456's `read_entity_document` backend command needs to read from the archive or the DB instead. I436 should land first (or concurrently with I456 adapting its read path). If I456 ships before I436, the reader reads from workspace. If I436 ships first, I456 must read from wherever documents migrated to.

## Notes

The "stale data on disk" bug class is the primary motivation. When the pipeline writes `_today/data/schedule.json` and that file persists across code changes, users see outdated data after an update until they manually trigger a re-run. Eliminating the file layer eliminates this entire class: the DB reflects the current pipeline output by definition, and there is no stale artifact to confuse the system.

The migration in AC#5 uses an archive-not-delete strategy because workspace files may contain user-authored content (meeting templates, custom directives) that is not captured in the DB schema. The archive preserves that content while removing the files from the active read path.

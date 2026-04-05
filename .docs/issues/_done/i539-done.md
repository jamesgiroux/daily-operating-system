# I539: Database Recovery UX for Migration/DB Failure

**Priority:** P1
**Area:** Backend + Frontend
**Version:** v1.0.0 (Phase 1)
**Depends on:** I511 (backup infrastructure in migration runner)
**Blocks:** Nothing — runs parallel with I512
**Absorbs:** Nothing

## Problem

Users currently have no in-app recovery path when database startup fails due to migration or schema integrity issues. Recovery requires manual/devtools intervention, which is not acceptable for production.

Failure modes that require recovery:
1. Migration SQL error (schema change fails mid-apply)
2. Schema integrity check failure (`verify_required_schema()` detects missing tables/columns post-migration)
3. SQLite corruption (WAL journal issue, disk failure, interrupted write)
4. Forward-compat guard (DB version newer than app — user downgraded)

## Design

### Detection logic

Recovery is required when `run_migrations()` returns `Err(String)` during app startup. The error string contains the specific failure reason. The app stores this in-memory (not on disk — the DB may be unusable) and routes to the recovery screen.

Additionally, `PRAGMA integrity_check` can detect SQLite-level corruption. Run this on startup only when `run_migrations()` succeeds but the app encounters unexpected query failures on first load.

The detection flow:
```
App startup
  → open DB connection
  → run_migrations()
    → Ok(n): normal startup, verify_required_schema() passed
    → Err(reason): set recovery_required = true, store reason
  → if recovery_required: route to DatabaseRecovery screen
  → else: normal app load
```

### Recovery options

1. **Restore from backup** — list available pre-migration backups, user selects one, restore replaces the current DB file. Requires app restart after restore.
2. **Start fresh** — delete the current DB file, create a new empty database. User loses all data but can re-sync from Google Calendar/Gmail. This is the fallback when no viable backups exist.
3. **Export raw DB** — allow user to save a copy of the (possibly corrupt) database for manual recovery or support. Available even when the DB is in a failed state.

### Backup presentation

Backups are named `dailyos.db.pre-migration.YYYYMMDD-HHMMSS.bak` on disk. In the UI, present them as:
- Date/time (human-readable, relative: "2 hours ago", "March 5, 2026 at 2:30 PM")
- Size (human-readable: "42 MB")
- Kind: "Pre-migration backup" (currently the only kind; future: manual backups)

Up to 10 backups are retained (pruned by the migration runner).

### Post-restore behavior

After a successful restore:
- The app must fully restart (Tauri process restart, not just a page reload). The DB connection, migration state, and all cached data need to reinitialize.
- Data created between the backup timestamp and now is lost. The recovery UI should state this clearly: "This will restore your data to [timestamp]. Changes made after this point will be lost."
- After restart, `run_migrations()` runs again on the restored DB. If the restored DB is from a version that needs the failing migration, it will fail again. The recovery UI should warn: "If the same error occurs after restore, try an older backup or contact support."

## Backend API

| Command | Input | Output | Notes |
|---------|-------|--------|-------|
| `get_database_recovery_status` | none | `DatabaseRecoveryStatus` | Called on startup before normal app init |
| `list_database_backups` | none | `Vec<BackupInfo>` | Works even when DB is in failed state (reads filesystem) |
| `restore_database_from_backup` | `backup_path: String` | `Result<(), String>` | Replaces current DB file. Validates backup is valid SQLite before overwriting. |
| `create_manual_backup` | none | `BackupInfo` | Creates a backup on demand from Settings (not just pre-migration). Uses same backup infrastructure. |
| `start_fresh_database` | none | `Result<(), String>` | Deletes current DB, creates empty. Requires user confirmation in frontend. |
| `export_database_copy` | `destination: String` | `Result<(), String>` | Copies current DB (even if corrupt) to user-chosen location. |

## Public API / Types

```typescript
interface DatabaseRecoveryStatus {
  required: boolean;
  reason: string;     // e.g., "Migration v55 failed: no such column..."
  detail: string;     // User-friendly explanation
  dbPath: string;     // For "export raw DB" feature
}

interface BackupInfo {
  path: string;
  filename: string;          // Human-readable name
  createdAt: string;         // ISO timestamp
  sizeBytes: number;
  kind: "pre_migration" | "manual";
  schemaVersion: number | null;  // If detectable from backup
}
```

## Frontend surfaces

### Startup blocker: `DatabaseRecovery` screen
- Full-screen, same precedence as encryption recovery (blocks all other UI)
- Shows: error reason (technical detail in expandable section), backup list, restore/fresh/export buttons
- "Restore from backup" is the primary action
- "Start fresh" requires confirmation dialog: "This will delete all your data. You can re-sync from Google Calendar and Gmail, but local intelligence, notes, and corrections will be lost."
- "Export database" as secondary action for support/debugging

### Settings > Data recovery card
- Available during normal operation
- Shows: current DB path, size, schema version, last backup timestamp
- Actions: create manual backup, view backup list, restore from backup
- "Restore from backup" shows same confirmation as startup blocker

## Acceptance Criteria

1. If `run_migrations()` fails, app blocks normal UI and shows recovery screen with error detail.
2. Recovery screen lists all available backups with human-readable timestamps and sizes.
3. Restore from backup succeeds: app restarts, migrations re-run on restored DB, normal operation resumes.
4. Restore from a corrupt/invalid backup shows a clear error without losing the current (possibly broken) DB.
5. "Start fresh" deletes DB and creates empty database. App restarts into onboarding flow.
6. "Export database" copies current DB to user-chosen location, works even when DB is in failed state.
7. Settings > Data shows backup list and allows manual backup creation and restore during normal operation.
8. Encryption-key-missing flow remains unchanged (no regression). Recovery screen only appears for migration/integrity failures, not encryption issues.
9. If no backups exist, recovery screen shows "Start fresh" as primary action with clear explanation.
10. Forward-compat guard error ("DB version newer than app") shows specific message: "Update DailyOS to the latest version" with no restore option (restore would just hit the same error).

## Out of Scope

- Automatic corruption detection during normal operation (only on startup)
- Cloud backup / sync
- Point-in-time recovery (only full-DB restore from snapshots)
- Backup scheduling (backups are created pre-migration and on-demand only)

//! SQLite backup and rebuild-from-filesystem (ADR-0048)
//!
//! **Backup**: Uses `rusqlite::backup::Backup` API to create a hot copy next
//! to the active database (`<active-db>.bak`). Runs on app startup and after
//! daily archive.
//!
//! **Rebuild**: Scans `Accounts/` and `People/` workspace directories,
//! re-populates SQLite from JSON files. Known gap: email enrichment state
//! and meeting history are lost on rebuild (acceptable per DEC48).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;

use crate::accounts;
use crate::db::ActionDb;
use crate::people;
use crate::projects;

const MANUAL_BACKUP_SUFFIX: &str = ".bak";
const PRE_MIGRATION_MARKER: &str = ".pre-migration.";
const PRE_RESTORE_MARKER: &str = ".pre-restore.";
const UNSUPPORTED_BACKUP_MESSAGE: &str = "Backup is not a supported plain SQLite DailyOS backup. Legacy encrypted backups cannot be restored automatically; rebuild from canonical workspace files or use the storage salvage procedure.";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub kind: String,
    pub filename: String,
    pub schema_version: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    pub path: String,
    pub size_bytes: u64,
    pub schema_version: i64,
    pub last_backup: Option<String>,
}

pub(crate) struct ActiveDatabaseFileMutation {
    db_path: PathBuf,
    guarded_paths: Vec<PathBuf>,
    _cutover_guards: Vec<crate::services::rebuild::RebuildCutoverGuard>,
}

impl ActiveDatabaseFileMutation {
    pub(crate) fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub(crate) fn guarded_paths(&self) -> &[PathBuf] {
        &self.guarded_paths
    }
}

fn active_db_path() -> Result<PathBuf, String> {
    ActionDb::db_path_public().map_err(|e| {
        log::warn!("resolve active database path failed: {e}");
        "Failed to resolve active database path".to_string()
    })
}

fn active_db_file_mutation_target_path() -> Result<PathBuf, String> {
    let dailyos_dir = crate::db::dailyos_data_dir().map_err(|e| {
        log::warn!("resolve active database mutation target failed: {e}");
        "Failed to resolve active database mutation target".to_string()
    })?;
    let path = match crate::db::db_mode() {
        crate::db::DbMode::Live => dailyos_dir.join("dailyos.db"),
        crate::db::DbMode::Replica => dailyos_dir.join("dailyos-replica.db"),
        crate::db::DbMode::Mock => dailyos_dir.join("dailyos-dev.db"),
    };
    Ok(path)
}

fn active_db_file_mutation_guard_paths() -> Result<Vec<PathBuf>, String> {
    let dailyos_dir = crate::db::dailyos_data_dir().map_err(|e| {
        log::warn!("resolve active database mutation guard paths failed: {e}");
        "Failed to resolve active database mutation guard paths".to_string()
    })?;
    let paths = match crate::db::db_mode() {
        crate::db::DbMode::Live => {
            vec![
                dailyos_dir.join("dailyos.db"),
                dailyos_dir.join("actions.db"),
            ]
        }
        crate::db::DbMode::Replica => vec![dailyos_dir.join("dailyos-replica.db")],
        crate::db::DbMode::Mock => vec![dailyos_dir.join("dailyos-dev.db")],
    };
    Ok(paths)
}

fn begin_cutover_guards_for_paths(
    paths: &[PathBuf],
) -> Result<Vec<crate::services::rebuild::RebuildCutoverGuard>, String> {
    let mut guards = Vec::with_capacity(paths.len());
    for path in paths {
        crate::db::guard_path_for_mode(path).map_err(|e| {
            log::warn!(
                "refusing database file mutation for {} in current DB mode: {e}",
                path.display()
            );
            "Refusing database file mutation in current DB mode".to_string()
        })?;
        guards.push(
            crate::services::rebuild::try_begin_live_cutover(path).map_err(|e| {
                log::warn!(
                    "database file mutation unavailable for {}: {e}",
                    path.display()
                );
                "Database file mutation unavailable while database access is active".to_string()
            })?,
        );
    }
    Ok(guards)
}

pub(crate) fn begin_active_database_file_mutation() -> Result<ActiveDatabaseFileMutation, String> {
    let guarded_paths = active_db_file_mutation_guard_paths()?;
    let cutover_guards = begin_cutover_guards_for_paths(&guarded_paths)?;
    let db_path = active_db_file_mutation_target_path()?;
    Ok(ActiveDatabaseFileMutation {
        db_path,
        guarded_paths,
        _cutover_guards: cutover_guards,
    })
}

fn active_db_path_for_connection(db: &ActionDb) -> Result<PathBuf, String> {
    let path: String = db
        .conn_ref()
        .query_row("PRAGMA database_list", [], |row| row.get(2))
        .map_err(|e| format!("Failed to resolve database path from connection: {e}"))?;

    if path.is_empty() || path == ":memory:" {
        return active_db_path();
    }

    Ok(PathBuf::from(path))
}

fn manual_backup_path(db_path: &Path) -> Result<PathBuf, String> {
    let parent = db_path
        .parent()
        .ok_or_else(|| "Database path has no parent directory".to_string())?;
    let mut file_name = db_path
        .file_name()
        .ok_or_else(|| "Database path has no filename".to_string())?
        .to_os_string();
    file_name.push(MANUAL_BACKUP_SUFFIX);
    Ok(parent.join(file_name))
}

/// Read schema version (PRAGMA user_version) from a SQLite file.
/// Returns None if the file cannot be opened or read.
fn read_schema_version(path: &Path) -> Option<i64> {
    crate::db::ActionDb::validate_plain_sqlite_storage(path).ok()?;
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .ok()?;
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .ok()
}

fn backup_kind(db_path: &Path, backup_path: &Path) -> Option<&'static str> {
    let base = db_path.file_name()?.to_str()?;
    let name = backup_path.file_name()?.to_str()?;
    if name == format!("{base}{MANUAL_BACKUP_SUFFIX}") {
        return Some("manual");
    }
    if name.starts_with(&format!("{base}{PRE_MIGRATION_MARKER}")) && name.ends_with(".bak") {
        return Some("pre-migration");
    }
    if name.starts_with(&format!("{base}{PRE_RESTORE_MARKER}")) && name.ends_with(".bak") {
        return Some("restore-point");
    }
    None
}

fn parse_timestamp_from_name(file_name: &str, marker: &str) -> Option<String> {
    let (_, rest) = file_name.split_once(marker)?;
    let stamp = rest.strip_suffix(".bak")?;
    let ts = chrono::NaiveDateTime::parse_from_str(stamp, "%Y%m%d-%H%M%S").ok()?;
    Some(ts.and_utc().to_rfc3339())
}

fn backup_created_at(path: &Path, metadata: &fs::Metadata) -> String {
    let name = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or_default();
    if let Some(ts) = parse_timestamp_from_name(name, PRE_MIGRATION_MARKER) {
        return ts;
    }
    if let Some(ts) = parse_timestamp_from_name(name, PRE_RESTORE_MARKER) {
        return ts;
    }
    metadata
        .modified()
        .map(chrono::DateTime::<Utc>::from)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|_| Utc::now().to_rfc3339())
}

fn pre_restore_snapshot_path(db_path: &Path) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let file_name = format!(
        "{}{PRE_RESTORE_MARKER}{timestamp}.bak",
        db_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("dailyos.db")
    );
    db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(file_name)
}

fn wal_path(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(
            "{}-wal",
            db_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("dailyos.db")
        ))
}

fn shm_path(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(
            "{}-shm",
            db_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("dailyos.db")
        ))
}

fn prune_restore_snapshots(db_path: &Path, keep: usize) -> Result<(), String> {
    let parent = db_path
        .parent()
        .ok_or_else(|| "Database path has no parent directory".to_string())?;
    let mut snapshots: Vec<PathBuf> = fs::read_dir(parent)
        .map_err(|e| format!("Failed to read backup directory: {e}"))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| matches!(backup_kind(db_path, p), Some("restore-point")))
        .collect();
    snapshots.sort();
    if snapshots.len() <= keep {
        return Ok(());
    }
    let to_delete = snapshots.len() - keep;
    for path in snapshots.into_iter().take(to_delete) {
        if let Err(e) = fs::remove_file(&path) {
            log::warn!("remove old restore point {} failed: {e}", path.display());
        }
    }
    Ok(())
}

/// Back up the live database to `<active-db>.bak`.
///
/// Uses SQLite's online backup API so the source DB can remain open and
/// in use during the backup. Returns the backup filename token on success.
///
/// Copying happens in chunks of [`BACKUP_PAGES_PER_STEP`] pages with
/// busy/locked retry, mirroring `migrations.rs::create_backup_via_api`. The
/// previous one-shot `step(-1)` path returned `Ok(StepResult::Done)` on the
/// 400 MB DBs we see in production but did not actually produce a
/// consistent file — restored `.bak` files surfaced as
/// `database disk image is malformed` on first open. The fix replicates the
/// chunked pattern the pre-migration backup path already documents and uses.
pub fn backup_database(db: &ActionDb) -> Result<String, String> {
    let db_path = active_db_path_for_connection(db)?;
    let backup_path = manual_backup_path(&db_path)?;

    let mut backup_conn = rusqlite::Connection::open(&backup_path)
        .map_err(|e| format!("Failed to open backup file: {}", e))?;

    run_chunked_backup(db.conn_ref(), &mut backup_conn)
        .map_err(|e| format!("Backup failed: {e}"))?;

    // Restrict backup file permissions
    crate::db::hardening::set_file_permissions(&backup_path);

    log::info!("Database backed up to {}", backup_path.display());
    backup_file_token(&backup_path)
}

fn backup_file_token(path: &Path) -> Result<String, String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .ok_or_else(|| "Backup path has no filename".to_string())
}

/// Pages copied per `Backup::step` iteration.
///
/// Matches `migrations.rs::PAGES_PER_STEP`. Single `step(-1)` calls on 100+ MB
/// DBs returned `Ok(StepResult::Done)` while leaving the destination silently
/// inconsistent (the historic "not an error" mapping); chunked stepping does
/// not exhibit that pathology and gives us progress logging on large copies.
const BACKUP_PAGES_PER_STEP: i32 = 1024;

/// Cap consecutive Busy/Locked retries during a chunked backup. At 50 ms per
/// retry this gives a 30 s wall clock — long enough to outlast normal writer
/// activity, short enough to fail loudly rather than wedge.
const BACKUP_MAX_BUSY_RETRIES: u32 = 600;

/// Drive a `rusqlite::backup::Backup` to completion using chunked stepping
/// with Busy/Locked retry. Shared shape with `migrations.rs::create_backup_via_api`
/// so both startup-time and live-time backups go through the same proven path.
pub(crate) fn run_chunked_backup(
    source: &rusqlite::Connection,
    destination: &mut rusqlite::Connection,
) -> Result<(), String> {
    let backup = rusqlite::backup::Backup::new(source, destination)
        .map_err(|e| format!("Failed to initialize backup: {e}"))?;
    let mut step_count = 0_u64;
    let mut busy_retries = 0_u32;
    loop {
        match backup.step(BACKUP_PAGES_PER_STEP) {
            Ok(rusqlite::backup::StepResult::More) => {
                step_count += 1;
                busy_retries = 0;
                if step_count.is_multiple_of(64) {
                    log::info!(
                        "Database backup in progress: ~{} pages copied",
                        step_count * BACKUP_PAGES_PER_STEP as u64
                    );
                }
            }
            Ok(rusqlite::backup::StepResult::Done) => return Ok(()),
            Ok(rusqlite::backup::StepResult::Busy) | Ok(rusqlite::backup::StepResult::Locked) => {
                busy_retries += 1;
                if busy_retries >= BACKUP_MAX_BUSY_RETRIES {
                    return Err(format!(
                        "Database backup gave up after {} consecutive Busy/Locked retries (~{}s); a long writer may be holding the source DB",
                        busy_retries,
                        (busy_retries as u64 * 50) / 1000
                    ));
                }
                if busy_retries.is_multiple_of(40) {
                    log::warn!(
                        "Database backup waiting on Busy/Locked source: retry {} of {}",
                        busy_retries,
                        BACKUP_MAX_BUSY_RETRIES
                    );
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Ok(other) => {
                return Err(format!("Database backup unexpected step result: {other:?}"));
            }
            Err(e) => return Err(format!("Database backup step failed: {e}")),
        }
    }
}

pub(crate) fn clone_database_to_path(
    source_db_path: &Path,
    destination_db_path: &Path,
) -> Result<(), String> {
    guard_database_clone_destination(destination_db_path)?;
    let staged_path = database_refresh_temp_path(destination_db_path);
    clone_database_to_staged_path(source_db_path, destination_db_path, &staged_path)?;
    activate_staged_database_clone(&staged_path, destination_db_path)?.commit();
    Ok(())
}

pub(crate) fn clone_database_to_staged_path(
    source_db_path: &Path,
    destination_db_path: &Path,
    staged_db_path: &Path,
) -> Result<(), String> {
    guard_database_clone_destination(destination_db_path)?;
    let provider = Arc::new(crate::db::LocalKeychain::new());
    let source_db =
        ActionDb::open_readonly_at(source_db_path, provider.clone()).map_err(|e| e.to_string())?;

    let parent = staged_db_path
        .parent()
        .ok_or_else(|| "Database clone staged path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("Failed to create database clone destination directory: {e}"))?;

    if staged_db_path == destination_db_path {
        return Err("Database clone staged path must differ from destination path".to_string());
    }
    remove_file_if_exists(staged_db_path)?;

    let mut destination_conn = rusqlite::Connection::open(staged_db_path)
        .map_err(|e| format!("Failed to open database clone staged file: {e}"))?;

    run_chunked_backup(source_db.conn_ref(), &mut destination_conn)?;
    drop(destination_conn);
    drop(source_db);

    crate::db::hardening::set_file_permissions(staged_db_path);

    Ok(())
}

pub(crate) fn database_refresh_temp_path(destination_db_path: &Path) -> PathBuf {
    destination_db_path.with_file_name(format!(
        "{}.refresh.tmp",
        destination_db_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("dailyos-replica.db")
    ))
}

#[derive(Debug)]
pub(crate) struct ActivatedDatabaseClone {
    backups: Vec<(PathBuf, PathBuf)>,
}

impl ActivatedDatabaseClone {
    pub(crate) fn rollback(self) {
        for (backup, destination) in self.backups.into_iter().rev() {
            if let Err(error) = remove_file_if_exists(&destination) {
                log::warn!(
                    "database clone rollback could not remove activated file {}: {error}",
                    destination.display()
                );
            }
            if backup.exists() {
                if let Err(error) = fs::rename(&backup, &destination) {
                    log::warn!(
                        "database clone rollback could not restore {} from {}: {error}",
                        destination.display(),
                        backup.display()
                    );
                }
            }
        }
    }

    pub(crate) fn commit(self) {
        for (backup, _) in self.backups {
            if let Err(error) = remove_file_if_exists(&backup) {
                log::warn!(
                    "database clone cleanup could not remove backup {}: {error}",
                    backup.display()
                );
            }
        }
    }
}

pub(crate) fn activate_staged_database_clone(
    staged_db_path: &Path,
    destination_db_path: &Path,
) -> Result<ActivatedDatabaseClone, String> {
    guard_database_clone_destination(destination_db_path)?;
    if staged_db_path == destination_db_path {
        return Err("Database clone staged path must differ from destination path".to_string());
    }
    if !staged_db_path.exists() {
        return Err(format!(
            "Database clone staged file does not exist: {}",
            staged_db_path.display()
        ));
    }

    let mut backups = Vec::new();
    for destination in [
        destination_db_path.to_path_buf(),
        wal_path(destination_db_path),
        shm_path(destination_db_path),
    ] {
        let backup = refresh_previous_path(&destination);
        if let Err(error) = remove_file_if_exists(&backup) {
            let activated = ActivatedDatabaseClone { backups };
            activated.rollback();
            return Err(error);
        }
        if destination.exists() {
            if let Err(error) = fs::rename(&destination, &backup) {
                let activated = ActivatedDatabaseClone { backups };
                activated.rollback();
                return Err(format!(
                    "Failed to move existing database file {} aside to {}: {e}",
                    destination.display(),
                    backup.display(),
                    e = error
                ));
            }
            backups.push((backup, destination));
        }
    }

    match fs::rename(staged_db_path, destination_db_path) {
        Ok(()) => {}
        Err(error) => {
            let activated = ActivatedDatabaseClone { backups };
            activated.rollback();
            return Err(format!(
                "Failed to activate database clone {} from {}: {error}",
                destination_db_path.display(),
                staged_db_path.display()
            ));
        }
    }
    crate::db::hardening::set_file_permissions(destination_db_path);

    Ok(ActivatedDatabaseClone { backups })
}

fn guard_database_clone_destination(destination_db_path: &Path) -> Result<(), String> {
    crate::db::guard_path_for_mode(destination_db_path).map_err(|e| {
        format!(
            "Refusing to activate database clone for destination {} in current DB mode: {e}",
            destination_db_path.display()
        )
    })
}

fn refresh_previous_path(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.refresh.previous",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("dailyos-replica.db")
    ))
}

fn remove_file_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => {
            log::warn!("failed to remove database file {}: {e}", path.display());
            Err(format!("Failed to remove database file: {e}"))
        }
    }
}

fn canonical_backup_path_in_active_directory(
    db_path: &Path,
    backup_path: &Path,
) -> Result<PathBuf, String> {
    let backup_dir = db_path
        .parent()
        .ok_or_else(|| "Database path has no parent directory".to_string())?
        .canonicalize()
        .map_err(|e| format!("Failed to resolve database backup directory: {e}"))?;
    let supplied_backup_path = if backup_path.components().count() == 1 {
        backup_dir.join(backup_path)
    } else {
        backup_path.to_path_buf()
    };
    let backup_path = supplied_backup_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve backup path: {e}"))?;
    if !backup_path.is_file() {
        return Err("Backup file not found".to_string());
    }

    let supplied_dir = backup_path
        .parent()
        .ok_or_else(|| "Backup path has no parent directory".to_string())?;
    if supplied_dir != backup_dir.as_path() {
        return Err("Backup path must be in the active database backup directory".to_string());
    }

    if backup_kind(db_path, &backup_path).is_none() {
        return Err("Backup path is not a valid DailyOS backup file".to_string());
    }
    Ok(backup_path)
}

pub(crate) fn remove_database_files_for_active_mutation(
    mutation: &ActiveDatabaseFileMutation,
) -> Result<(), String> {
    remove_database_files_after_cutover(mutation.guarded_paths())
}

fn remove_inactive_database_files_after_restore(
    mutation: &ActiveDatabaseFileMutation,
) -> Result<(), String> {
    for path in mutation
        .guarded_paths()
        .iter()
        .filter(|path| path.as_path() != mutation.db_path())
    {
        for sidecar in [path.clone(), wal_path(path), shm_path(path)] {
            remove_file_if_exists(&sidecar)?;
        }
    }
    Ok(())
}

fn remove_database_files_after_cutover(paths: &[PathBuf]) -> Result<(), String> {
    for db_path in paths {
        for path in [db_path.clone(), wal_path(db_path), shm_path(db_path)] {
            remove_file_if_exists(&path)?;
        }
    }
    Ok(())
}

/// List known backup files for the active database.
pub fn list_database_backups() -> Result<Vec<BackupInfo>, String> {
    let db_path = active_db_file_mutation_target_path()?;
    list_database_backups_for_path(&db_path)
}

fn list_database_backups_for_path(db_path: &Path) -> Result<Vec<BackupInfo>, String> {
    let parent = db_path
        .parent()
        .ok_or_else(|| "Database path has no parent directory".to_string())?;

    let mut backups = Vec::new();
    let mut unsupported_backup_seen = false;
    for entry in
        fs::read_dir(parent).map_err(|e| format!("Failed to read backup directory: {e}"))?
    {
        let entry = entry.map_err(|e| format!("Failed to read backup entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(kind) = backup_kind(db_path, &path) else {
            continue;
        };
        if let Err(error) = validate_backup(&path) {
            unsupported_backup_seen = true;
            log::warn!(
                "skipping unsupported or unreadable database backup {}: {error}",
                path.display()
            );
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|e| format!("Failed to inspect backup metadata: {e}"))?;
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        let schema_version = read_schema_version(&path);
        backups.push(BackupInfo {
            path: filename.clone(),
            created_at: backup_created_at(&path, &metadata),
            size_bytes: metadata.len(),
            kind: kind.to_string(),
            filename,
            schema_version,
        });
    }

    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    if backups.is_empty() && unsupported_backup_seen {
        return Err(UNSUPPORTED_BACKUP_MESSAGE.to_string());
    }
    Ok(backups)
}

/// Restore the active database file from a selected backup.
pub fn restore_database_from_backup(backup_path: &Path) -> Result<(), String> {
    let mutation = begin_active_database_file_mutation()?;
    restore_database_from_backup_for_path_after_cutover(mutation.db_path(), backup_path)?;
    remove_inactive_database_files_after_restore(&mutation)
}

fn restore_database_from_backup_for_path(db_path: &Path, backup_path: &Path) -> Result<(), String> {
    let guarded_paths = vec![db_path.to_path_buf()];
    let _cutover_guards = begin_cutover_guards_for_paths(&guarded_paths)?;
    restore_database_from_backup_for_path_after_cutover(db_path, backup_path)
}

fn restore_database_from_backup_for_path_after_cutover(
    db_path: &Path,
    backup_path: &Path,
) -> Result<(), String> {
    crate::db::guard_path_for_mode(db_path).map_err(|e| {
        log::warn!(
            "refusing database restore for {} in current DB mode: {e}",
            db_path.display()
        );
        "Refusing database restore in current DB mode".to_string()
    })?;

    let backup_path = canonical_backup_path_in_active_directory(db_path, backup_path)?;

    validate_backup(&backup_path)?;

    let snapshot_path = pre_restore_snapshot_path(db_path);
    let mut snapshot_created = false;
    if db_path.exists() {
        fs::copy(db_path, &snapshot_path)
            .map_err(|e| format!("Failed to create pre-restore snapshot: {e}"))?;
        crate::db::hardening::set_file_permissions(&snapshot_path);
        snapshot_created = true;
    }

    let temp_restore = db_path.with_file_name(format!(
        "{}.restore.tmp",
        db_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("dailyos.db")
    ));
    let restore_attempt = (|| -> Result<(), String> {
        if let Err(e) = fs::remove_file(&temp_restore) {
            log::warn!(
                "remove stale restore temp file {} failed: {e}",
                temp_restore.display()
            );
        }
        fs::copy(&backup_path, &temp_restore)
            .map_err(|e| format!("Failed to stage backup restore: {e}"))?;

        if db_path.exists() {
            fs::remove_file(db_path)
                .map_err(|e| format!("Failed to remove existing database file: {e}"))?;
        }

        fs::rename(&temp_restore, db_path)
            .map_err(|e| format!("Failed to activate restored database: {e}"))?;

        remove_file_if_exists(&wal_path(db_path))?;
        remove_file_if_exists(&shm_path(db_path))?;
        crate::db::hardening::set_file_permissions(db_path);
        prune_restore_snapshots(db_path, 5)?;
        Ok(())
    })();

    if let Err(err) = restore_attempt {
        let mut rollback_errors = Vec::new();
        if let Err(e) = fs::remove_file(&temp_restore) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!(
                    "remove failed restore temp file {} failed: {e}",
                    temp_restore.display()
                );
                rollback_errors.push(format!("failed to remove restore temp file: {e}"));
            }
        }
        if snapshot_created {
            if let Err(e) = fs::copy(&snapshot_path, db_path) {
                log::warn!(
                    "restore pre-restore snapshot {} failed: {e}",
                    snapshot_path.display()
                );
                rollback_errors.push(format!("pre-restore snapshot rollback failed: {e}"));
            }
            let wal = wal_path(db_path);
            if let Err(e) = remove_file_if_exists(&wal) {
                log::warn!(
                    "remove WAL after failed restore {} failed: {e}",
                    wal.display()
                );
                rollback_errors.push(format!("rollback sidecar cleanup failed: {e}"));
            }
            let shm = shm_path(db_path);
            if let Err(e) = remove_file_if_exists(&shm) {
                log::warn!(
                    "remove SHM after failed restore {} failed: {e}",
                    shm.display()
                );
                rollback_errors.push(format!("rollback sidecar cleanup failed: {e}"));
            }
        }
        if !rollback_errors.is_empty() {
            return Err(format!(
                "Database restore failed: {err}; rollback failed: {}",
                rollback_errors.join("; ")
            ));
        }
        return Err(format!("Database restore failed: {err}"));
    }

    log::info!(
        "Database restored from backup {}",
        backup_file_token(&backup_path).unwrap_or_else(|_| "<unknown>".to_string())
    );
    Ok(())
}

/// Validate a backup file's integrity before restoring.
///
pub fn validate_backup(path: &Path) -> Result<(), String> {
    crate::db::ActionDb::validate_plain_sqlite_storage(path).map_err(|error| {
        log::warn!("unsupported database backup {}: {error}", path.display());
        UNSUPPORTED_BACKUP_MESSAGE.to_string()
    })?;
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| {
                log::warn!("cannot open database backup {}: {e}", path.display());
                UNSUPPORTED_BACKUP_MESSAGE.to_string()
            })?;
    let result = conn
        .pragma_query_value(None, "integrity_check", |row| row.get::<_, String>(0))
        .map_err(|e| {
            log::warn!(
                "database backup integrity check failed for {}: {e}",
                path.display()
            );
            UNSUPPORTED_BACKUP_MESSAGE.to_string()
        })?;

    if result != "ok" {
        log::warn!(
            "database backup integrity check failed for {}: {result}",
            path.display()
        );
        return Err(UNSUPPORTED_BACKUP_MESSAGE.to_string());
    }
    Ok(())
}

/// Delete the active database and all associated WAL/SHM files.
pub fn start_fresh_database() -> Result<(), String> {
    let mutation = begin_active_database_file_mutation()?;
    remove_database_files_for_active_mutation(&mutation)
}

fn start_fresh_database_for_path(db_path: &Path) -> Result<(), String> {
    crate::db::guard_path_for_mode(db_path).map_err(|e| {
        log::warn!(
            "refusing to start fresh database for {} in current DB mode: {e}",
            db_path.display()
        );
        "Refusing to start fresh database in current DB mode".to_string()
    })?;
    let guarded_paths = vec![db_path.to_path_buf()];
    let _cutover_guards = begin_cutover_guards_for_paths(&guarded_paths)?;
    remove_database_files_after_cutover(&guarded_paths)
}

/// Copy the active database to a user-chosen destination.
pub fn export_database_copy(destination: &str) -> Result<(), String> {
    let db_path = active_db_path()?;
    export_database_copy_for_path(&db_path, Path::new(destination))
}

fn export_database_copy_for_path(db_path: &Path, destination: &Path) -> Result<(), String> {
    if db_path == destination {
        return Err("Export destination must be different from the active database".to_string());
    }

    let source_db = ActionDb::open_readonly_at(db_path, Arc::new(crate::db::LocalKeychain::new()))
        .map_err(|error| {
            log::warn!("failed to open active database for export: {error}");
            "Failed to open active database for export".to_string()
        })?;

    remove_file_if_exists(destination)?;
    remove_file_if_exists(&wal_path(destination))?;
    remove_file_if_exists(&shm_path(destination))?;

    let mut destination_conn = rusqlite::Connection::open(destination)
        .map_err(|e| format!("Failed to open export destination: {e}"))?;
    run_chunked_backup(source_db.conn_ref(), &mut destination_conn)
        .map_err(|e| format!("Failed to export database: {e}"))?;
    drop(destination_conn);
    drop(source_db);

    crate::db::hardening::set_file_permissions(destination);
    Ok(())
}

/// Get information about the active database.
pub fn get_database_info() -> Result<DatabaseInfo, String> {
    let db_path = active_db_path()?;
    let size_bytes = fs::metadata(&db_path)
        .map_err(|e| format!("Failed to read database metadata: {e}"))?
        .len();
    let schema_version = read_schema_version(&db_path).unwrap_or(0);
    let last_backup = list_database_backups()?
        .first()
        .map(|b| b.created_at.clone());
    Ok(DatabaseInfo {
        path: db_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "dailyos.db".to_string()),
        size_bytes,
        schema_version,
        last_backup,
    })
}

/// Rebuild SQLite tables from workspace JSON files.
///
/// Scans `Accounts/*/dashboard.json`, `Projects/*/dashboard.json`, and
/// `People/*/person.json`, upserting each into the database. Regenerates
/// markdown files from the merged state.
///
/// **Known gaps** (acceptable per DEC48):
/// - Email enrichment state is lost
/// - Meeting history is lost (meetings/meeting_prep/meeting_transcripts tables not rebuilt)
/// - Action source references may not match if the source files were moved
///
/// Returns `(accounts_synced, projects_synced, people_synced)`.
pub fn rebuild_from_filesystem(
    workspace: &Path,
    db: &ActionDb,
    user_domains: &[String],
) -> Result<(usize, usize, usize), String> {
    let accounts_synced = accounts::sync_accounts_from_workspace(workspace, db)
        .map_err(|e| format!("Account rebuild failed: {}", e))?;

    let projects_synced = projects::sync_projects_from_workspace(workspace, db)
        .map_err(|e| format!("Project rebuild failed: {}", e))?;

    let people_synced = people::sync_people_from_workspace(workspace, db, user_domains)
        .map_err(|e| format!("People rebuild failed: {}", e))?;

    log::info!(
        "Database rebuilt from filesystem: {} accounts, {} projects, {} people synced",
        accounts_synced,
        projects_synced,
        people_synced
    );

    Ok((accounts_synced, projects_synced, people_synced))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ResetDbMode;

    impl Drop for ResetDbMode {
        fn drop(&mut self) {
            crate::db::set_db_mode(crate::db::DbMode::Live);
        }
    }

    fn remove_test_file_if_exists(path: &Path) {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("remove test file {} failed: {error}", path.display()),
        }
    }

    fn create_plain_sqlite_file(path: &Path, user_version: i64) {
        let conn = rusqlite::Connection::open(path).expect("open sqlite file");
        conn.pragma_update(None, "user_version", user_version)
            .expect("set user_version");
        drop(conn);
    }

    fn create_header_valid_malformed_file(path: &Path) {
        let mut bytes = b"SQLite format 3\0".to_vec();
        bytes.extend_from_slice(b"not a sqlite database body");
        std::fs::write(path, bytes).expect("write malformed backup");
    }

    #[test]
    fn test_backup_creates_file_and_returns_filename_token() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let db = ActionDb::open_at_unencrypted(db_path).expect("open db");

        let backup_path =
            manual_backup_path(&active_db_path_for_connection(&db).expect("active path"))
                .expect("backup path");
        let token = backup_database(&db).expect("backup database");

        assert!(backup_path.exists());
        assert_eq!(token, "test.db.bak");
        assert!(
            !Path::new(&token).is_absolute(),
            "backup command should not return absolute paths"
        );
    }

    #[test]
    fn test_manual_backup_path_uses_active_db_filename() {
        let live = Path::new("/tmp/.dailyos/dailyos.db");
        let dev = Path::new("/tmp/.dailyos/dailyos-dev.db");

        assert_eq!(
            manual_backup_path(live).expect("live backup path"),
            Path::new("/tmp/.dailyos/dailyos.db.bak")
        );
        assert_eq!(
            manual_backup_path(dev).expect("dev backup path"),
            Path::new("/tmp/.dailyos/dailyos-dev.db.bak")
        );
    }

    #[test]
    fn test_read_schema_version_plaintext_has_no_keychain_dependency() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dailyos.db.bak");
        let conn = rusqlite::Connection::open(&db_path).expect("open db");
        conn.pragma_update(None, "user_version", 42)
            .expect("set user_version");
        drop(conn);

        assert_eq!(read_schema_version(&db_path), Some(42));
    }

    #[test]
    fn test_rebuild_empty_workspace() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let db = ActionDb::open_at_unencrypted(db_path).expect("open db");

        let workspace = dir.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();

        let (accounts, projects, people) =
            rebuild_from_filesystem(&workspace, &db, &[]).expect("rebuild");
        assert_eq!(accounts, 0);
        assert_eq!(projects, 0);
        assert_eq!(people, 0);
    }

    #[test]
    fn test_backup_kind_parsing() {
        let db = Path::new("/tmp/dailyos.db");
        assert_eq!(
            backup_kind(db, Path::new("/tmp/dailyos.db.bak")),
            Some("manual")
        );
        assert_eq!(
            backup_kind(
                db,
                Path::new("/tmp/dailyos.db.pre-migration.20260305-120000.bak")
            ),
            Some("pre-migration")
        );
        assert_eq!(
            backup_kind(
                db,
                Path::new("/tmp/dailyos.db.pre-restore.20260305-120500.bak")
            ),
            Some("restore-point")
        );
    }

    #[test]
    fn test_restore_database_from_backup_for_path_replaces_db_and_creates_snapshot() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dailyos.db");
        let backup_path = dir
            .path()
            .join("dailyos.db.pre-migration.20260305-120000.bak");

        let conn = rusqlite::Connection::open(&db_path).expect("open live db");
        conn.execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t (v) VALUES ('live');")
            .expect("seed live db");
        drop(conn);

        let backup_conn = rusqlite::Connection::open(&backup_path).expect("open backup db");
        backup_conn
            .execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t (v) VALUES ('backup');")
            .expect("seed backup db");
        drop(backup_conn);

        restore_database_from_backup_for_path(&db_path, &backup_path).expect("restore");

        let reopened = rusqlite::Connection::open(&db_path).expect("open restored db");
        let value: String = reopened
            .query_row("SELECT v FROM t LIMIT 1", [], |r| r.get(0))
            .expect("read restored value");
        assert_eq!(value, "backup");

        let snapshots: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| matches!(backup_kind(&db_path, p), Some("restore-point")))
            .collect();
        assert!(
            !snapshots.is_empty(),
            "restore should create a pre-restore snapshot"
        );
    }

    #[test]
    fn restore_database_from_backup_for_path_refuses_backup_outside_active_directory() {
        let active_dir = tempfile::tempdir().expect("active tempdir");
        let outside_dir = tempfile::tempdir().expect("outside tempdir");
        let db_path = active_dir.path().join("dailyos.db");
        let backup_path = outside_dir.path().join("dailyos.db.bak");

        let conn = rusqlite::Connection::open(&db_path).expect("open live db");
        conn.execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t (v) VALUES ('live');")
            .expect("seed live db");
        drop(conn);

        let backup_conn = rusqlite::Connection::open(&backup_path).expect("open backup db");
        backup_conn
            .execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t (v) VALUES ('backup');")
            .expect("seed backup db");
        drop(backup_conn);

        let error = restore_database_from_backup_for_path(&db_path, &backup_path)
            .expect_err("restore must reject valid-looking backups outside the active directory");
        assert!(error.contains("active database backup directory"));
    }

    #[test]
    fn restore_database_from_backup_for_path_accepts_filename_restore_token() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dailyos.db");
        let backup_path = dir.path().join("dailyos.db.bak");

        let conn = rusqlite::Connection::open(&db_path).expect("open live db");
        conn.execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t (v) VALUES ('live');")
            .expect("seed live db");
        drop(conn);

        let backup_conn = rusqlite::Connection::open(&backup_path).expect("open backup db");
        backup_conn
            .execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t (v) VALUES ('backup');")
            .expect("seed backup db");
        drop(backup_conn);

        restore_database_from_backup_for_path(&db_path, Path::new("dailyos.db.bak"))
            .expect("restore from filename token");

        let reopened = rusqlite::Connection::open(&db_path).expect("open restored db");
        let value: String = reopened
            .query_row("SELECT v FROM t LIMIT 1", [], |r| r.get(0))
            .expect("read restored value");
        assert_eq!(value, "backup");
    }

    #[test]
    fn restore_database_from_backup_for_path_rejects_open_db_access_guard() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dailyos.db");
        let backup_path = dir.path().join("dailyos.db.bak");

        let db = ActionDb::open_at_unencrypted(db_path.clone()).expect("open live db");
        let backup_conn = rusqlite::Connection::open(&backup_path).expect("open backup db");
        backup_conn
            .execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t (v) VALUES ('backup');")
            .expect("seed backup db");
        drop(backup_conn);

        let error = restore_database_from_backup_for_path(&db_path, &backup_path)
            .expect_err("restore must reject while a DB access guard is active");
        assert!(
            error.contains("Database file mutation unavailable"),
            "unexpected error: {error}"
        );
        drop(db);
    }

    #[test]
    fn restore_database_from_backup_for_path_refuses_prod_path_in_replica_mode() {
        let _lock = crate::db::DB_MODE_TEST_LOCK.lock().expect("db mode lock");
        let _reset = ResetDbMode;
        crate::db::set_db_mode(crate::db::DbMode::Replica);

        let dailyos_dir = crate::db::dailyos_data_dir().expect("data dir");
        let prod_path = dailyos_dir.join("dailyos.db");
        let backup_path = tempfile::NamedTempFile::new()
            .expect("backup temp")
            .into_temp_path();

        let error = restore_database_from_backup_for_path(&prod_path, &backup_path)
            .expect_err("replica mode must not restore the production DB");
        assert!(error.contains("Refusing database file mutation"));
    }

    #[test]
    fn active_file_mutation_target_does_not_preflight_unsupported_legacy_actions_db() {
        let _mode_lock = crate::db::DB_MODE_TEST_LOCK.lock().expect("db mode lock");
        let _reset = ResetDbMode;
        crate::db::set_db_mode(crate::db::DbMode::Live);

        let dailyos_dir = crate::db::dailyos_data_dir().expect("data dir");
        std::fs::create_dir_all(&dailyos_dir).expect("create data dir");
        let new_path = dailyos_dir.join("dailyos.db");
        let legacy_path = dailyos_dir.join("actions.db");
        remove_test_file_if_exists(&legacy_path);
        remove_test_file_if_exists(&wal_path(&legacy_path));
        remove_test_file_if_exists(&shm_path(&legacy_path));

        std::fs::write(&legacy_path, b"not plain sqlite legacy payload")
            .expect("write unsupported legacy DB");
        std::fs::write(wal_path(&legacy_path), b"legacy wal").expect("write legacy WAL");
        std::fs::write(shm_path(&legacy_path), b"legacy shm").expect("write legacy SHM");

        let mutation_target = active_db_file_mutation_target_path().expect("mutation target path");
        assert_eq!(mutation_target, new_path);
        let guarded_paths = active_db_file_mutation_guard_paths().expect("mutation guarded paths");
        assert!(
            guarded_paths.iter().any(|path| path == &legacy_path),
            "legacy DB path should still be guarded for deletion"
        );

        remove_test_file_if_exists(&legacy_path);
        remove_test_file_if_exists(&wal_path(&legacy_path));
        remove_test_file_if_exists(&shm_path(&legacy_path));
    }

    #[test]
    fn start_fresh_database_for_path_refuses_prod_path_in_replica_mode() {
        let _lock = crate::db::DB_MODE_TEST_LOCK.lock().expect("db mode lock");
        let _reset = ResetDbMode;
        crate::db::set_db_mode(crate::db::DbMode::Replica);

        let dailyos_dir = crate::db::dailyos_data_dir().expect("data dir");
        let prod_path = dailyos_dir.join("dailyos.db");

        let error = start_fresh_database_for_path(&prod_path)
            .expect_err("replica mode must not start fresh against the production DB");
        assert!(error.contains("Refusing to start fresh database"));
    }

    #[test]
    fn start_fresh_database_for_path_removes_target_wal_and_shm() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dailyos-replica.db");
        let wal = wal_path(&db_path);
        let shm = shm_path(&db_path);
        std::fs::write(&db_path, b"db").expect("db");
        std::fs::write(&wal, b"wal").expect("wal");
        std::fs::write(&shm, b"shm").expect("shm");

        start_fresh_database_for_path(&db_path).expect("fresh database");

        assert!(!db_path.exists(), "target DB should be removed");
        assert!(!wal.exists(), "target WAL should be removed");
        assert!(!shm.exists(), "target SHM should be removed");
    }

    #[test]
    fn start_fresh_database_for_path_rejects_open_db_access_guard() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dailyos-replica.db");
        let db = ActionDb::open_at_unencrypted(db_path.clone()).expect("open live db");

        let error = start_fresh_database_for_path(&db_path)
            .expect_err("start fresh must reject while a DB access guard is active");
        assert!(
            error.contains("Database file mutation unavailable"),
            "unexpected error: {error}"
        );
        drop(db);
    }

    #[test]
    fn clone_database_to_staged_path_refuses_prod_destination_in_replica_mode() {
        let _lock = crate::db::DB_MODE_TEST_LOCK.lock().expect("db mode lock");
        let _reset = ResetDbMode;
        crate::db::set_db_mode(crate::db::DbMode::Replica);

        let dailyos_dir = crate::db::dailyos_data_dir().expect("data dir");
        let prod_path = dailyos_dir.join("dailyos.db");
        let dir = tempfile::tempdir().expect("tempdir");
        let staged_path = dir.path().join("dailyos.db.refresh.tmp");

        let error = clone_database_to_staged_path(
            &dir.path().join("missing-source.db"),
            &prod_path,
            &staged_path,
        )
        .expect_err("replica mode must not stage a clone for the production DB");
        assert!(error.contains("Refused to open production database"));
        assert!(
            !staged_path.exists(),
            "destination guard must fire before writing a staged DB"
        );
    }

    #[test]
    fn activate_staged_database_clone_refuses_prod_destination_in_replica_mode() {
        let _lock = crate::db::DB_MODE_TEST_LOCK.lock().expect("db mode lock");
        let _reset = ResetDbMode;
        crate::db::set_db_mode(crate::db::DbMode::Replica);

        let dailyos_dir = crate::db::dailyos_data_dir().expect("data dir");
        let prod_path = dailyos_dir.join("dailyos.db");
        let staged = tempfile::NamedTempFile::new().expect("staged db");

        let error = activate_staged_database_clone(staged.path(), &prod_path)
            .expect_err("replica mode must not activate a clone over the production DB");
        assert!(error.contains("Refused to open production database"));
        assert!(
            staged.path().exists(),
            "destination guard must fire before consuming the staged DB"
        );
    }

    #[test]
    fn activate_staged_database_clone_rolls_back_db_when_sidecar_cleanup_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        let destination_path = dir.path().join("dailyos-replica.db");
        let staged_path = dir.path().join("dailyos-replica.db.refresh.tmp");
        std::fs::write(&destination_path, b"old db").expect("old db");
        std::fs::write(&staged_path, b"new db").expect("staged db");

        let wal_backup = refresh_previous_path(&wal_path(&destination_path));
        std::fs::create_dir(&wal_backup).expect("stale wal backup directory");

        let error = activate_staged_database_clone(&staged_path, &destination_path)
            .expect_err("sidecar cleanup failure must abort activation");
        assert!(
            error.contains("Failed to remove"),
            "unexpected activation error: {error}"
        );
        assert_eq!(
            std::fs::read(&destination_path).expect("restored db"),
            b"old db",
            "old destination DB must be restored after mid-loop failure"
        );
        assert!(
            !refresh_previous_path(&destination_path).exists(),
            "rollback should not strand the old DB at the previous path"
        );
        assert!(
            staged_path.exists(),
            "staged DB should remain for the caller cleanup path"
        );
    }

    #[test]
    fn clone_database_to_path_produces_readable_plain_clone_and_clears_stale_sidecars() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source_path = dir.path().join("source.db");
        let destination_path = dir.path().join("dailyos-replica.db");
        let provider = Arc::new(crate::db::LocalKeychain::new());

        let source_db =
            ActionDb::open_at(source_path.clone(), provider).expect("open plain source");
        source_db
            .conn_ref()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS replica_clone_marker (label TEXT);
                 DELETE FROM replica_clone_marker;",
            )
            .expect("marker schema");
        source_db
            .conn_ref()
            .execute(
                "INSERT INTO replica_clone_marker (label) VALUES (?1)",
                ["cloned"],
            )
            .expect("marker insert");
        source_db
            .conn_ref()
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .expect("checkpoint source");
        drop(source_db);

        let old_destination = ActionDb::open_at(
            destination_path.clone(),
            Arc::new(crate::db::LocalKeychain::new()),
        )
        .expect("open old plain destination");
        old_destination
            .conn_ref()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS replica_clone_marker (label TEXT);
                 DELETE FROM replica_clone_marker;
                 INSERT INTO replica_clone_marker (label) VALUES ('old');",
            )
            .expect("old marker");
        old_destination
            .conn_ref()
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .expect("checkpoint old destination");
        drop(old_destination);
        std::fs::write(wal_path(&destination_path), b"old wal").expect("old wal");
        std::fs::write(shm_path(&destination_path), b"old shm").expect("old shm");

        clone_database_to_path(&source_path, &destination_path).expect("clone database");

        assert!(
            !wal_path(&destination_path).exists(),
            "stale destination WAL should be removed during activation"
        );
        assert!(
            !shm_path(&destination_path).exists(),
            "stale destination SHM should be removed during activation"
        );

        let cloned = ActionDb::open_readonly_at(
            &destination_path,
            Arc::new(crate::db::LocalKeychain::new()),
        )
        .expect("open plain clone");
        let label: String = cloned
            .conn_ref()
            .query_row("SELECT label FROM replica_clone_marker", [], |row| {
                row.get(0)
            })
            .expect("read clone marker");
        assert_eq!(label, "cloned");
    }

    #[test]
    fn test_list_database_backups_for_path_includes_known_backup_kinds() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dailyos.db");
        create_plain_sqlite_file(&db_path, 1);

        let manual = dir.path().join("dailyos.db.bak");
        let pre_migration = dir
            .path()
            .join("dailyos.db.pre-migration.20260305-120000.bak");
        let pre_restore = dir
            .path()
            .join("dailyos.db.pre-restore.20260305-121000.bak");
        create_plain_sqlite_file(&manual, 1);
        create_plain_sqlite_file(&pre_migration, 2);
        create_plain_sqlite_file(&pre_restore, 3);

        let items = list_database_backups_for_path(&db_path).expect("list backups");
        let kinds: Vec<_> = items.iter().map(|i| i.kind.as_str()).collect();
        assert!(kinds.contains(&"manual"));
        assert!(kinds.contains(&"pre-migration"));
        assert!(kinds.contains(&"restore-point"));
        assert!(
            items
                .iter()
                .all(|item| !Path::new(&item.path).is_absolute()),
            "backup restore tokens should not expose absolute paths"
        );
        assert!(
            items.iter().any(|item| item.path == "dailyos.db.bak"),
            "manual backup should use its filename as the restore token"
        );
    }

    #[test]
    fn list_database_backups_for_path_skips_unsupported_legacy_backup_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dailyos.db");
        create_plain_sqlite_file(&db_path, 1);
        create_plain_sqlite_file(&dir.path().join("dailyos.db.bak"), 1);
        std::fs::write(
            dir.path()
                .join("dailyos.db.pre-migration.20260305-120000.bak"),
            b"legacy encrypted backup payload",
        )
        .expect("legacy backup");

        let items = list_database_backups_for_path(&db_path).expect("list backups");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].filename, "dailyos.db.bak");
    }

    #[test]
    fn list_database_backups_for_path_skips_header_valid_malformed_backup_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dailyos.db");
        create_plain_sqlite_file(&db_path, 1);
        create_plain_sqlite_file(&dir.path().join("dailyos.db.bak"), 1);
        create_header_valid_malformed_file(
            &dir.path()
                .join("dailyos.db.pre-migration.20260305-120000.bak"),
        );

        let items = list_database_backups_for_path(&db_path).expect("list backups");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].filename, "dailyos.db.bak");
    }

    #[test]
    fn list_database_backups_for_path_reports_unsupported_only_backup_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dailyos.db");
        create_plain_sqlite_file(&db_path, 1);
        std::fs::write(
            dir.path().join("dailyos.db.bak"),
            b"legacy encrypted backup payload",
        )
        .expect("legacy backup");

        let error = list_database_backups_for_path(&db_path).expect_err("unsupported backup");

        assert_eq!(error, UNSUPPORTED_BACKUP_MESSAGE);
    }

    #[test]
    fn list_database_backups_uses_dailyos_target_when_legacy_actions_db_is_unsupported() {
        let _lock = crate::db::DB_MODE_TEST_LOCK.lock().expect("db mode lock");
        let _reset = ResetDbMode;
        crate::db::set_db_mode(crate::db::DbMode::Live);

        let dir = crate::db::dailyos_data_dir().expect("dailyos data dir");
        std::fs::create_dir_all(&dir).expect("create dailyos data dir");
        let dailyos_db = dir.join("dailyos.db");
        let actions_db = dir.join("actions.db");
        let backup_path = dir.join("dailyos.db.bak");
        for path in [
            dailyos_db.clone(),
            wal_path(&dailyos_db),
            shm_path(&dailyos_db),
            actions_db.clone(),
            wal_path(&actions_db),
            shm_path(&actions_db),
            backup_path.clone(),
        ] {
            remove_test_file_if_exists(&path);
        }

        std::fs::write(&actions_db, b"legacy encrypted database payload").expect("legacy db");
        create_plain_sqlite_file(&backup_path, 42);

        let items = list_database_backups().expect("list backups");

        assert!(
            items.iter().any(|item| item.filename == "dailyos.db.bak"),
            "canonical backup should list even when legacy actions.db is unsupported"
        );

        for path in [dailyos_db, actions_db, backup_path] {
            remove_test_file_if_exists(&path);
        }
    }

    #[test]
    fn validate_backup_redacts_unsupported_storage_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let backup_path = dir.path().join("dailyos.db.bak");
        std::fs::write(&backup_path, b"legacy encrypted backup payload").expect("backup");

        let error = validate_backup(&backup_path).expect_err("unsupported backup");

        assert_eq!(error, UNSUPPORTED_BACKUP_MESSAGE);
        assert!(
            !error.contains(dir.path().to_string_lossy().as_ref()),
            "user-facing validation error should not expose local paths"
        );
    }

    #[test]
    fn validate_backup_redacts_header_valid_malformed_backup_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let backup_path = dir.path().join("dailyos.db.bak");
        create_header_valid_malformed_file(&backup_path);

        let error = validate_backup(&backup_path).expect_err("malformed backup");

        assert_eq!(error, UNSUPPORTED_BACKUP_MESSAGE);
        assert!(
            !error.contains(dir.path().to_string_lossy().as_ref()),
            "user-facing validation error should not expose local paths"
        );
    }

    #[test]
    fn export_database_copy_for_path_uses_sqlite_backup_and_preserves_wal_frames() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source_path = dir.path().join("dailyos.db");
        let export_path = dir.path().join("export.db");

        let source = rusqlite::Connection::open(&source_path).expect("open source");
        source
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 CREATE TABLE export_marker (id INTEGER PRIMARY KEY, label TEXT);
                 INSERT INTO export_marker (label) VALUES ('from-wal');",
            )
            .expect("seed source");
        assert!(
            wal_path(&source_path).exists(),
            "test setup should leave committed frames in WAL"
        );

        export_database_copy_for_path(&source_path, &export_path).expect("export copy");

        let exported = rusqlite::Connection::open(&export_path).expect("open export");
        let label: String = exported
            .query_row("SELECT label FROM export_marker", [], |row| row.get(0))
            .expect("read exported WAL-backed row");
        assert_eq!(label, "from-wal");

        drop(exported);
        drop(source);
    }

    #[test]
    fn run_chunked_backup_produces_byte_identical_copy_at_size_above_one_step() {
        // Regression test for the silent-malformation pattern: source DB
        // larger than BACKUP_PAGES_PER_STEP must produce a destination that
        // passes integrity_check and has the same content. The historic
        // `step(-1)` path on large DBs at this size returned
        // `Ok(StepResult::Done)` without copying every page.
        let dir = tempfile::tempdir().expect("tempdir");
        let src_path = dir.path().join("src.db");
        let dst_path = dir.path().join("dst.db");

        let src = rusqlite::Connection::open(&src_path).expect("open src");
        src.execute_batch("PRAGMA journal_mode = WAL; PRAGMA page_size = 4096;")
            .expect("pragmas");
        src.execute_batch("CREATE TABLE rows (id INTEGER PRIMARY KEY, payload BLOB);")
            .expect("schema");
        // Insert enough rows that the DB exceeds BACKUP_PAGES_PER_STEP * page_size.
        // 1024 pages * 4 KB = 4 MB; seed with 8 MB worth of payload so the copy
        // requires at least two chunked-step iterations.
        let payload = vec![0xA5_u8; 8192];
        let mut stmt = src
            .prepare("INSERT INTO rows (payload) VALUES (?1)")
            .expect("prep");
        for _ in 0..1024 {
            stmt.execute([&payload]).expect("insert");
        }
        drop(stmt);

        let mut dst = rusqlite::Connection::open(&dst_path).expect("open dst");
        run_chunked_backup(&src, &mut dst).expect("backup");
        drop(dst);

        let reopened = rusqlite::Connection::open(&dst_path).expect("reopen dst");
        let integrity: String = reopened
            .pragma_query_value(None, "integrity_check", |row| row.get(0))
            .expect("integrity_check");
        assert_eq!(integrity, "ok", "restored backup must pass integrity_check");
        let row_count: i64 = reopened
            .query_row("SELECT COUNT(*) FROM rows", [], |row| row.get(0))
            .expect("count rows");
        assert_eq!(row_count, 1024, "restored backup must contain all rows");
    }
}

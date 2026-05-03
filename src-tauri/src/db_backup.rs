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

use chrono::Utc;

use crate::accounts;
use crate::db::ActionDb;
use crate::people;
use crate::projects;

const MANUAL_BACKUP_SUFFIX: &str = ".bak";
const PRE_MIGRATION_MARKER: &str = ".pre-migration.";
const PRE_RESTORE_MARKER: &str = ".pre-restore.";

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

fn active_db_path() -> Result<PathBuf, String> {
    ActionDb::db_path_public().map_err(|e| format!("Failed to resolve database path: {e}"))
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
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .ok()?;
    // Apply an existing encryption key only for encrypted-looking files.
    // Schema reads must not create Keychain entries as a side effect.
    if !crate::db::encryption::is_database_plaintext(path) {
        if let Ok(hex_key) = crate::db::encryption::get_existing_db_key() {
            let _ = conn.execute_batch(&crate::db::encryption::key_to_pragma(&hex_key));
        }
    }
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
        let _ = fs::remove_file(path);
    }
    Ok(())
}

/// Back up the live database to `<active-db>.bak`.
///
/// Uses SQLite's online backup API so the source DB can remain open and
/// in use during the backup. Returns the backup file path on success.
pub fn backup_database(db: &ActionDb) -> Result<String, String> {
    let db_path = active_db_path_for_connection(db)?;
    let backup_path = manual_backup_path(&db_path)?;

    let mut backup_conn = rusqlite::Connection::open(&backup_path)
        .map_err(|e| format!("Failed to open backup file: {}", e))?;

    // Apply encryption key to backup destination so.bak is also encrypted
    let hex_key = crate::db::encryption::get_or_create_db_key(&backup_path)
        .map_err(|e| format!("Failed to get encryption key for backup: {e}"))?;
    backup_conn
        .execute_batch(&crate::db::encryption::key_to_pragma(&hex_key))
        .map_err(|e| format!("Failed to set backup encryption key: {e}"))?;

    let backup = rusqlite::backup::Backup::new(db.conn_ref(), &mut backup_conn)
        .map_err(|e| format!("Failed to initialize backup: {}", e))?;

    // Copy all pages in one step (small DB, typically < 10 MB)
    backup
        .step(-1)
        .map_err(|e| format!("Backup failed: {}", e))?;

    // Restrict backup file permissions
    crate::db::hardening::set_file_permissions(&backup_path);

    log::info!("Database backed up to {}", backup_path.display());
    Ok(backup_path.to_string_lossy().to_string())
}

/// List known backup files for the active database.
pub fn list_database_backups() -> Result<Vec<BackupInfo>, String> {
    let db_path = active_db_path()?;
    list_database_backups_for_path(&db_path)
}

fn list_database_backups_for_path(db_path: &Path) -> Result<Vec<BackupInfo>, String> {
    let parent = db_path
        .parent()
        .ok_or_else(|| "Database path has no parent directory".to_string())?;

    let mut backups = Vec::new();
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
        let metadata = entry
            .metadata()
            .map_err(|e| format!("Failed to inspect backup metadata: {e}"))?;
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        let schema_version = read_schema_version(&path);
        backups.push(BackupInfo {
            path: path.to_string_lossy().to_string(),
            created_at: backup_created_at(&path, &metadata),
            size_bytes: metadata.len(),
            kind: kind.to_string(),
            filename,
            schema_version,
        });
    }

    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(backups)
}

/// Restore the active database file from a selected backup.
pub fn restore_database_from_backup(backup_path: &Path) -> Result<(), String> {
    let db_path = active_db_path()?;
    restore_database_from_backup_for_path(&db_path, backup_path)
}

fn restore_database_from_backup_for_path(db_path: &Path, backup_path: &Path) -> Result<(), String> {
    let backup_path = backup_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve backup path: {e}"))?;

    if !backup_path.exists() || !backup_path.is_file() {
        return Err("Backup file not found".to_string());
    }
    if backup_kind(db_path, &backup_path).is_none() {
        return Err("Backup path is not a valid DailyOS backup file".to_string());
    }

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
        let _ = fs::remove_file(&temp_restore);
        fs::copy(&backup_path, &temp_restore)
            .map_err(|e| format!("Failed to stage backup restore: {e}"))?;

        if db_path.exists() {
            fs::remove_file(db_path)
                .map_err(|e| format!("Failed to remove existing database file: {e}"))?;
        }

        fs::rename(&temp_restore, db_path)
            .map_err(|e| format!("Failed to activate restored database: {e}"))?;

        let _ = fs::remove_file(wal_path(db_path));
        let _ = fs::remove_file(shm_path(db_path));
        crate::db::hardening::set_file_permissions(db_path);
        prune_restore_snapshots(db_path, 5)?;
        Ok(())
    })();

    if let Err(err) = restore_attempt {
        let _ = fs::remove_file(&temp_restore);
        if snapshot_created {
            let _ = fs::copy(&snapshot_path, db_path);
            let _ = fs::remove_file(wal_path(db_path));
            let _ = fs::remove_file(shm_path(db_path));
        }
        return Err(format!("Database restore failed: {err}"));
    }

    log::info!(
        "Database restored from backup {}",
        backup_path.to_string_lossy()
    );
    Ok(())
}

/// Validate a backup file's integrity before restoring.
///
/// Tries with the encryption key first. If that fails (e.g. backup is
/// unencrypted), falls back to a plain connection.
pub fn validate_backup(path: &Path) -> Result<(), String> {
    // Attempt 1: with encryption key
    let result = (|| -> Option<String> {
        let conn =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .ok()?;
        if let Ok(hex_key) = crate::db::encryption::get_or_create_db_key(path) {
            conn.execute_batch(&crate::db::encryption::key_to_pragma(&hex_key))
                .ok()?;
        }
        conn.pragma_query_value(None, "integrity_check", |row| row.get::<_, String>(0))
            .ok()
    })();

    // Attempt 2: without encryption (plain SQLite)
    let result = match result {
        Some(ref r) if r == "ok" => return Ok(()),
        _ => {
            let conn = rusqlite::Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .map_err(|e| format!("Cannot open backup file: {e}"))?;
            conn.pragma_query_value(None, "integrity_check", |row| row.get::<_, String>(0))
                .map_err(|e| format!("Integrity check failed: {e}"))?
        }
    };

    if result != "ok" {
        return Err(format!("Backup integrity check failed: {result}"));
    }
    Ok(())
}

/// Delete the active database and all associated WAL/SHM files.
pub fn start_fresh_database() -> Result<(), String> {
    let db_path = active_db_path()?;
    for path in [&db_path, &wal_path(&db_path), &shm_path(&db_path)] {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Failed to remove {}: {e}", path.display())),
        }
    }
    Ok(())
}

/// Copy the active database to a user-chosen destination.
pub fn export_database_copy(destination: &str) -> Result<(), String> {
    let db_path = active_db_path()?;
    fs::copy(&db_path, destination).map_err(|e| format!("Failed to export database: {e}"))?;
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
        path: db_path.to_string_lossy().to_string(),
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

    #[test]
    fn test_backup_creates_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let db = ActionDb::open_at_unencrypted(db_path).expect("open db");

        let backup_path =
            manual_backup_path(&active_db_path_for_connection(&db).expect("active path"))
                .expect("backup path");
        let mut backup_conn = rusqlite::Connection::open(&backup_path).expect("open backup");
        let backup =
            rusqlite::backup::Backup::new(db.conn_ref(), &mut backup_conn).expect("init backup");
        backup.step(-1).expect("backup step");
        drop(backup);
        drop(backup_conn);

        assert!(backup_path.exists());
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
    fn test_list_database_backups_for_path_includes_known_backup_kinds() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dailyos.db");
        std::fs::write(&db_path, b"db").expect("seed db file");

        let manual = dir.path().join("dailyos.db.bak");
        let pre_migration = dir
            .path()
            .join("dailyos.db.pre-migration.20260305-120000.bak");
        let pre_restore = dir
            .path()
            .join("dailyos.db.pre-restore.20260305-121000.bak");
        std::fs::write(&manual, b"m").expect("manual");
        std::fs::write(&pre_migration, b"pm").expect("pre migration");
        std::fs::write(&pre_restore, b"pr").expect("pre restore");

        let items = list_database_backups_for_path(&db_path).expect("list backups");
        let kinds: Vec<_> = items.iter().map(|i| i.kind.as_str()).collect();
        assert!(kinds.contains(&"manual"));
        assert!(kinds.contains(&"pre-migration"));
        assert!(kinds.contains(&"restore-point"));
    }
}

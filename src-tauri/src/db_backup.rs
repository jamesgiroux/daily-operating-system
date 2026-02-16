//! SQLite backup and rebuild-from-filesystem (I76 / ADR-0048)
//!
//! **Backup**: Uses `rusqlite::backup::Backup` API to create a hot copy at
//! `~/.dailyos/dailyos.db.bak`. Runs on app startup and after daily archive.
//!
//! **Rebuild**: Scans `Accounts/` and `People/` workspace directories,
//! re-populates SQLite from JSON files. Known gap: email enrichment state
//! and meeting history are lost on rebuild (acceptable per DEC48).

use std::path::Path;

use crate::accounts;
use crate::db::ActionDb;
use crate::people;
use crate::projects;

/// Back up the live database to `~/.dailyos/dailyos.db.bak`.
///
/// Uses SQLite's online backup API so the source DB can remain open and
/// in use during the backup. Returns the backup file path on success.
pub fn backup_database(db: &ActionDb) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Home directory not found")?;
    let backup_path = home.join(".dailyos").join("dailyos.db.bak");

    let mut backup_conn = rusqlite::Connection::open(&backup_path)
        .map_err(|e| format!("Failed to open backup file: {}", e))?;

    let backup = rusqlite::backup::Backup::new(db.conn_ref(), &mut backup_conn)
        .map_err(|e| format!("Failed to initialize backup: {}", e))?;

    // Copy all pages in one step (small DB, typically < 10 MB)
    backup
        .step(-1)
        .map_err(|e| format!("Backup failed: {}", e))?;

    log::info!("Database backed up to {}", backup_path.display());
    Ok(backup_path.to_string_lossy().to_string())
}

/// Rebuild SQLite tables from workspace JSON files.
///
/// Scans `Accounts/*/dashboard.json`, `Projects/*/dashboard.json`, and
/// `People/*/person.json`, upserting each into the database. Regenerates
/// markdown files from the merged state.
///
/// **Known gaps** (acceptable per DEC48):
/// - Email enrichment state is lost
/// - Meeting history is lost (meetings_history table not rebuilt)
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
        let db = ActionDb::open_at(db_path).expect("open db");

        // The backup function uses a hardcoded path (~/.dailyos/dailyos.db.bak),
        // so we test the Backup API directly with a custom path.
        let backup_path = dir.path().join("test.db.bak");
        let mut backup_conn = rusqlite::Connection::open(&backup_path).expect("open backup");
        let backup =
            rusqlite::backup::Backup::new(db.conn_ref(), &mut backup_conn).expect("init backup");
        backup.step(-1).expect("backup step");
        drop(backup);
        drop(backup_conn);

        assert!(backup_path.exists());
    }

    #[test]
    fn test_rebuild_empty_workspace() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let db = ActionDb::open_at(db_path).expect("open db");

        let workspace = dir.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();

        let (accounts, projects, people) =
            rebuild_from_filesystem(&workspace, &db, &[]).expect("rebuild");
        assert_eq!(accounts, 0);
        assert_eq!(projects, 0);
        assert_eq!(people, 0);
    }
}

//! Shared entity I/O helpers (I290).
//!
//! Generic file operations common to accounts, projects, and other entities
//! that follow the workspace directory pattern:
//!   {Workspace}/{DirName}/{EntityName}/dashboard.json
//!   {Workspace}/{DirName}/{EntityName}/dashboard.md

use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::db::ActionDb;

// =============================================================================
// Directory
// =============================================================================

/// Construct the canonical directory for an entity inside the workspace.
///
/// Returns `workspace/{dir_name}/{sanitized_entity_id}`.
/// `dir_name` is typically `"Accounts"` or `"Projects"`.
pub fn entity_dir(workspace: &Path, dir_name: &str, entity_name: &str) -> PathBuf {
    workspace
        .join(dir_name)
        .join(crate::util::sanitize_for_filesystem(entity_name))
}

// =============================================================================
// JSON Write
// =============================================================================

/// Serialize `data` as pretty-printed JSON and write it atomically to
/// `{entity_directory}/{filename}`.
///
/// Creates the entity directory (and parents) if it doesn't exist.
pub fn write_entity_json<T: Serialize>(
    entity_directory: &Path,
    filename: &str,
    data: &T,
) -> Result<(), String> {
    std::fs::create_dir_all(entity_directory)
        .map_err(|e| format!("Failed to create {}: {}", entity_directory.display(), e))?;

    let path = entity_directory.join(filename);
    let content =
        serde_json::to_string_pretty(data).map_err(|e| format!("Serialize error: {}", e))?;
    crate::util::atomic_write_str(&path, &content).map_err(|e| format!("Write error: {}", e))?;

    Ok(())
}

// =============================================================================
// JSON Read
// =============================================================================

/// Read and deserialize a JSON file from `{entity_directory}/{filename}`.
pub fn read_entity_json<T: DeserializeOwned>(
    entity_directory: &Path,
    filename: &str,
) -> Result<T, String> {
    let path = entity_directory.join(filename);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Read error ({}): {}", path.display(), e))?;
    serde_json::from_str(&content).map_err(|e| format!("Parse error ({}): {}", path.display(), e))
}

/// Get the file mtime as an RFC 3339 timestamp string, falling back to now.
pub fn file_updated_at(path: &Path) -> String {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.to_rfc3339()
        })
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
}

// =============================================================================
// Content Index Sync
// =============================================================================

/// Sync the content index for a single entity directory.
///
/// Thin wrapper around `entity_intel::sync_content_index_for_entity()` that
/// resolves the entity directory from the standard workspace layout.
///
/// Returns `(added, updated, removed)` counts.
pub fn sync_content_index_for_entity(
    db: &ActionDb,
    workspace: &Path,
    entity_id: &str,
    entity_type: &str,
    entity_directory: &Path,
) -> Result<(usize, usize, usize), String> {
    crate::intelligence::sync_content_index_for_entity(
        entity_directory,
        entity_id,
        entity_type,
        workspace,
        db,
    )
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_dir() {
        let workspace = Path::new("/workspace");
        let dir = entity_dir(workspace, "Accounts", "Acme Corp");
        assert_eq!(dir, PathBuf::from("/workspace/Accounts/Acme Corp"));
    }

    #[test]
    fn test_write_and_read_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let entity_dir = dir.path().join("TestEntity");

        #[derive(Serialize, serde::Deserialize, Debug, PartialEq)]
        struct TestData {
            name: String,
            value: i32,
        }

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        write_entity_json(&entity_dir, "data.json", &data).unwrap();

        let read_back: TestData = read_entity_json(&entity_dir, "data.json").unwrap();
        assert_eq!(read_back, data);
    }

    #[test]
    fn test_file_updated_at_returns_valid_rfc3339() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "content").unwrap();

        let ts = file_updated_at(&path);
        // Should parse as valid RFC 3339
        assert!(chrono::DateTime::parse_from_rfc3339(&ts).is_ok());
    }

    #[test]
    fn test_file_updated_at_nonexistent_returns_now() {
        let ts = file_updated_at(Path::new("/nonexistent/file.txt"));
        assert!(chrono::DateTime::parse_from_rfc3339(&ts).is_ok());
    }

    #[test]
    fn test_read_entity_json_missing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result: Result<serde_json::Value, _> = read_entity_json(dir.path(), "missing.json");
        assert!(result.is_err());
    }
}

//! Audit trail for AI-generated data (I297).
//!
//! Persists raw Claude output to disk so bad AI data can be investigated
//! after the fact. Files are written to `{workspace}/_audit/` and pruned
//! during hygiene scans.

use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::util::atomic_write_str;

/// How many days to keep audit files before pruning.
pub const AUDIT_RETENTION_DAYS: u32 = 30;

/// Sanitize an entity ID for safe use in filenames.
/// Keeps alphanumeric and hyphens; replaces everything else with underscore.
fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Write a raw AI output to the audit trail.
///
/// Creates `{workspace}/_audit/{timestamp}_{entity_type}_{entity_id}.txt`.
/// Uses atomic writes for crash safety. Returns the path of the written file.
pub fn write_audit_entry(
    workspace: &Path,
    entity_type: &str,
    entity_id: &str,
    raw_output: &str,
) -> Result<PathBuf, String> {
    let audit_dir = workspace.join("_audit");
    if !audit_dir.exists() {
        std::fs::create_dir_all(&audit_dir)
            .map_err(|e| format!("Failed to create _audit dir: {}", e))?;
    }

    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let safe_type = sanitize_id(entity_type);
    let safe_id = sanitize_id(entity_id);
    let filename = format!("{}_{}_{}.txt", timestamp, safe_type, safe_id);
    let file_path = audit_dir.join(&filename);

    atomic_write_str(&file_path, raw_output)
        .map_err(|e| format!("Audit write failed: {}", e))?;

    Ok(file_path)
}

/// Delete audit files older than the retention period.
///
/// Returns the number of files pruned.
pub fn prune_audit_files(workspace: &Path) -> usize {
    let audit_dir = workspace.join("_audit");
    if !audit_dir.exists() {
        return 0;
    }

    let cutoff = Utc::now()
        - chrono::Duration::days(AUDIT_RETENTION_DAYS as i64);
    let cutoff_ts = cutoff.timestamp();

    let entries = match std::fs::read_dir(&audit_dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    let mut pruned = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        // Use file modification time to determine age
        let mtime = match path.metadata().and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let mtime_secs = mtime
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        if mtime_secs < cutoff_ts && std::fs::remove_file(&path).is_ok() {
            pruned += 1;
        }
    }
    pruned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_audit_entry_creates_file() {
        let dir = tempfile::tempdir().expect("tempdir");

        let path = write_audit_entry(dir.path(), "account", "acme-corp", "raw AI output here")
            .expect("write should succeed");

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "raw AI output here");
        assert!(path.to_str().unwrap().contains("_audit"));
        assert!(path.to_str().unwrap().contains("account"));
        assert!(path.to_str().unwrap().contains("acme-corp"));
    }

    #[test]
    fn test_write_audit_entry_sanitizes_id() {
        let dir = tempfile::tempdir().expect("tempdir");

        let path = write_audit_entry(dir.path(), "account", "foo/bar baz\\qux", "data")
            .expect("write should succeed");

        let filename = path.file_name().unwrap().to_str().unwrap();
        assert!(!filename.contains('/'));
        assert!(!filename.contains(' '));
        assert!(!filename.contains('\\'));
        assert!(filename.contains("foo_bar_baz_qux"));
    }

    #[test]
    fn test_prune_removes_old_keeps_recent() {
        let dir = tempfile::tempdir().expect("tempdir");

        // Write two entries
        let recent = write_audit_entry(dir.path(), "account", "recent", "new data")
            .expect("write");
        let old = write_audit_entry(dir.path(), "account", "old", "old data")
            .expect("write");

        // Backdate the old file's mtime to 60 days ago
        let old_time = std::time::SystemTime::now()
            - std::time::Duration::from_secs(60 * 24 * 3600);
        filetime::set_file_mtime(
            &old,
            filetime::FileTime::from_system_time(old_time),
        )
        .expect("set mtime");

        let pruned = prune_audit_files(dir.path());

        assert_eq!(pruned, 1);
        assert!(!old.exists(), "old file should be deleted");
        assert!(recent.exists(), "recent file should be kept");
    }
}

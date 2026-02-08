//! File routing to PARA workspace locations.
//!
//! After classification, files are moved to the appropriate location
//! in the user's workspace based on the PARA structure.

use std::path::{Path, PathBuf};

use chrono::Utc;

use super::classifier::Classification;

/// Result of routing a file to a destination.
#[derive(Debug, Clone)]
pub struct RouteResult {
    /// Where the file was moved to.
    pub destination: PathBuf,
    /// Human-readable description of the routing.
    pub description: String,
}

/// Determine the destination path for a classified file.
///
/// Returns None if the file should stay in _inbox/ (e.g., needs AI enrichment).
pub fn resolve_destination(
    classification: &Classification,
    workspace: &Path,
    filename: &str,
) -> Option<PathBuf> {
    match classification {
        Classification::MeetingNotes { .. } => {
            // Route to archive/YYYY-MM-DD/
            let date = Utc::now().format("%Y-%m-%d").to_string();
            Some(workspace.join("_archive").join(&date).join(filename))
        }

        Classification::AccountUpdate { account } => {
            // Route to Accounts/<name>/01-Customer-Information/
            let account_dir = sanitize_dir_name(account);
            Some(
                workspace
                    .join("Accounts")
                    .join(&account_dir)
                    .join("01-Customer-Information")
                    .join(filename),
            )
        }

        Classification::ActionItems { .. } => {
            // Actions get extracted to SQLite, original goes to archive
            let date = Utc::now().format("%Y-%m-%d").to_string();
            Some(workspace.join("_archive").join(&date).join(filename))
        }

        Classification::MeetingContext { .. } => {
            // Route to _today/ for upcoming meeting prep, or archive
            let today_dir = workspace.join("_today");
            if today_dir.exists() {
                Some(today_dir.join(filename))
            } else {
                let date = Utc::now().format("%Y-%m-%d").to_string();
                Some(workspace.join("_archive").join(&date).join(filename))
            }
        }

        Classification::Unknown => {
            // Stay in inbox — needs AI enrichment
            None
        }
    }
}

/// Resolve a unique destination path by appending `-1`, `-2` etc. when the
/// target already exists (I69: prevent silent overwrites).
fn unique_destination(dest: &Path) -> PathBuf {
    if !dest.exists() {
        return dest.to_path_buf();
    }
    let stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = dest.extension().and_then(|e| e.to_str()).unwrap_or("");
    let parent = dest.parent().unwrap_or(Path::new("."));

    for i in 1..1000 {
        let new_name = if ext.is_empty() {
            format!("{}-{}", stem, i)
        } else {
            format!("{}-{}.{}", stem, i, ext)
        };
        let candidate = parent.join(&new_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    // Fallback (extremely unlikely)
    dest.to_path_buf()
}

/// Move a file from source to destination, creating parent directories as needed.
/// If the destination already exists, appends a numeric suffix to avoid overwrites.
pub fn move_file(source: &Path, destination: &Path) -> Result<RouteResult, String> {
    // Create parent directories
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }

    let final_dest = unique_destination(destination);

    // Move the file (rename if same filesystem, copy+delete otherwise)
    if std::fs::rename(source, &final_dest).is_err() {
        // Cross-filesystem: copy then delete
        std::fs::copy(source, &final_dest)
            .map_err(|e| format!("Failed to copy {} to {}: {}", source.display(), final_dest.display(), e))?;
        std::fs::remove_file(source)
            .map_err(|e| format!("Failed to remove source file {}: {}", source.display(), e))?;
    }

    Ok(RouteResult {
        destination: final_dest.clone(),
        description: format!("Moved to {}", final_dest.display()),
    })
}

/// Sanitize a name for use as a directory name.
///
/// "acme corp" → "Acme-Corp" (title case, hyphens)
fn sanitize_dir_name(name: &str) -> String {
    name.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_dir_name() {
        assert_eq!(sanitize_dir_name("acme corp"), "Acme-Corp");
        assert_eq!(sanitize_dir_name("BETA INC"), "Beta-Inc");
        assert_eq!(sanitize_dir_name("simple"), "Simple");
    }

    #[test]
    fn test_resolve_meeting_notes() {
        let workspace = Path::new("/workspace");
        let classification = Classification::MeetingNotes {
            account: Some("acme".to_string()),
        };
        let dest = resolve_destination(&classification, workspace, "notes.md");
        assert!(dest.is_some());
        let dest = dest.unwrap();
        assert!(dest.starts_with("/workspace/_archive/"));
        assert!(dest.ends_with("notes.md"));
    }

    #[test]
    fn test_resolve_account_update() {
        let workspace = Path::new("/workspace");
        let classification = Classification::AccountUpdate {
            account: "acme corp".to_string(),
        };
        let dest = resolve_destination(&classification, workspace, "update.md");
        assert!(dest.is_some());
        let dest = dest.unwrap();
        assert_eq!(
            dest,
            PathBuf::from("/workspace/Accounts/Acme-Corp/01-Customer-Information/update.md")
        );
    }

    #[test]
    fn test_unique_destination_no_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("report.pdf");
        assert_eq!(unique_destination(&dest), dest);
    }

    #[test]
    fn test_unique_destination_with_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("report.pdf");
        std::fs::write(&dest, "existing").unwrap();
        let result = unique_destination(&dest);
        assert_eq!(result, dir.path().join("report-1.pdf"));

        // Create that too
        std::fs::write(&result, "also existing").unwrap();
        let result2 = unique_destination(&dest);
        assert_eq!(result2, dir.path().join("report-2.pdf"));
    }

    #[test]
    fn test_resolve_unknown_stays() {
        let workspace = Path::new("/workspace");
        let classification = Classification::Unknown;
        let dest = resolve_destination(&classification, workspace, "mystery.md");
        assert!(dest.is_none());
    }
}

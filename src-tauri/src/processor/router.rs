//! File routing to PARA workspace locations.
//!
//! After classification, files are moved to the appropriate location
//! in the user's workspace based on the PARA structure.

use std::path::{Path, PathBuf};

use chrono::Utc;

use super::classifier::Classification;
use crate::db::ActionDb;

/// Result of routing a file to a destination.
#[derive(Debug, Clone)]
pub struct RouteResult {
    /// Where the file was moved to.
    pub destination: PathBuf,
    /// Human-readable description of the routing.
    pub description: String,
}

/// Outcome of route resolution.
#[derive(Debug, Clone)]
pub enum RouteOutcome {
    /// File should be moved to this destination.
    Destination(PathBuf),
    /// File needs AI enrichment — stay in inbox.
    NeedsEnrichment,
    /// Classification identified an entity that doesn't exist in DB.
    NeedsEntity { suggested_name: String },
}

/// Determine the destination path for a classified file.
///
/// When `db` is provided, validates that referenced accounts exist in the DB.
/// If the account doesn't exist, returns `NeedsEntity` so the user can assign it.
/// When `entity_tracker_path` is provided (explicit user assignment), skips the check.
pub fn resolve_destination(
    classification: &Classification,
    workspace: &Path,
    filename: &str,
    entity_tracker_path: Option<&str>,
    db: Option<&ActionDb>,
) -> RouteOutcome {
    match classification {
        Classification::MeetingNotes { account } => {
            if let Some(tp) = entity_tracker_path {
                return RouteOutcome::Destination(
                    workspace.join(tp).join("Meeting-Notes").join(filename),
                );
            }
            if let Some(account) = account {
                if let Some(db) = db {
                    let exists = db.get_account_by_name(account).ok().flatten().is_some();
                    if !exists {
                        log::info!(
                            "Account '{}' not found in DB — needs entity assignment",
                            account
                        );
                        return RouteOutcome::NeedsEntity {
                            suggested_name: account.clone(),
                        };
                    }
                }
                let account_dir = sanitize_dir_name(account);
                RouteOutcome::Destination(
                    workspace
                        .join("Accounts")
                        .join(&account_dir)
                        .join("Meeting-Notes")
                        .join(filename),
                )
            } else {
                let date = Utc::now().format("%Y-%m-%d").to_string();
                RouteOutcome::Destination(workspace.join("_archive").join(&date).join(filename))
            }
        }

        Classification::AccountUpdate { account } => {
            if let Some(tp) = entity_tracker_path {
                return RouteOutcome::Destination(
                    workspace.join(tp).join("Documents").join(filename),
                );
            }
            if let Some(db) = db {
                let exists = db.get_account_by_name(account).ok().flatten().is_some();
                if !exists {
                    log::info!(
                        "Account '{}' not found in DB — needs entity assignment",
                        account
                    );
                    return RouteOutcome::NeedsEntity {
                        suggested_name: account.clone(),
                    };
                }
            }
            let account_dir = sanitize_dir_name(account);
            RouteOutcome::Destination(
                workspace
                    .join("Accounts")
                    .join(&account_dir)
                    .join("Documents")
                    .join(filename),
            )
        }

        Classification::ActionItems { .. } => {
            let date = Utc::now().format("%Y-%m-%d").to_string();
            RouteOutcome::Destination(workspace.join("_archive").join(&date).join(filename))
        }

        Classification::MeetingContext { .. } => {
            // Route to _today/ for upcoming meeting prep, or archive
            let today_dir = workspace.join("_today");
            if today_dir.exists() {
                RouteOutcome::Destination(today_dir.join(filename))
            } else {
                let date = Utc::now().format("%Y-%m-%d").to_string();
                RouteOutcome::Destination(workspace.join("_archive").join(&date).join(filename))
            }
        }

        Classification::UserContext => RouteOutcome::NeedsEnrichment,

        Classification::Unknown => RouteOutcome::NeedsEnrichment,
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
        std::fs::copy(source, &final_dest).map_err(|e| {
            format!(
                "Failed to copy {} to {}: {}",
                source.display(),
                final_dest.display(),
                e
            )
        })?;
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
    fn test_resolve_meeting_notes_with_account() {
        let workspace = Path::new("/workspace");
        let classification = Classification::MeetingNotes {
            account: Some("acme corp".to_string()),
        };
        let outcome = resolve_destination(&classification, workspace, "notes.md", None, None);
        match outcome {
            RouteOutcome::Destination(dest) => assert_eq!(
                dest,
                PathBuf::from("/workspace/Accounts/Acme-Corp/Meeting-Notes/notes.md")
            ),
            other => panic!("Expected Destination, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_meeting_notes_no_account() {
        let workspace = Path::new("/workspace");
        let classification = Classification::MeetingNotes { account: None };
        let outcome = resolve_destination(&classification, workspace, "notes.md", None, None);
        match outcome {
            RouteOutcome::Destination(dest) => {
                assert!(dest.starts_with("/workspace/_archive/"));
                assert!(dest.ends_with("notes.md"));
            }
            other => panic!("Expected Destination, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_account_update() {
        let workspace = Path::new("/workspace");
        let classification = Classification::AccountUpdate {
            account: "acme corp".to_string(),
        };
        let outcome = resolve_destination(&classification, workspace, "update.md", None, None);
        match outcome {
            RouteOutcome::Destination(dest) => assert_eq!(
                dest,
                PathBuf::from("/workspace/Accounts/Acme-Corp/Documents/update.md")
            ),
            other => panic!("Expected Destination, got {:?}", other),
        }
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
        let outcome = resolve_destination(&classification, workspace, "mystery.md", None, None);
        assert!(matches!(outcome, RouteOutcome::NeedsEnrichment));
    }

    #[test]
    fn test_resolve_meeting_notes_needs_entity_when_account_not_in_db() {
        let db = crate::db::test_utils::test_db();
        let workspace = Path::new("/workspace");

        // Account "Acme Corp" does NOT exist in DB
        let classification = Classification::MeetingNotes {
            account: Some("Acme Corp".to_string()),
        };
        let outcome = resolve_destination(&classification, workspace, "notes.md", None, Some(&db));
        match outcome {
            RouteOutcome::NeedsEntity { suggested_name } => {
                assert_eq!(suggested_name, "Acme Corp");
            }
            other => panic!("Expected NeedsEntity, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_meeting_notes_routes_when_account_exists_in_db() {
        let db = crate::db::test_utils::test_db();
        let workspace = Path::new("/workspace");

        // Insert an account into the test DB
        let account = crate::db::DbAccount {
            id: "acme-corp".to_string(),
            name: "Acme Corp".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some("Accounts/Acme-Corp".to_string()),
            parent_id: None,
            account_type: crate::db::AccountType::Customer,
            updated_at: chrono::Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_account(&account).unwrap();

        let classification = Classification::MeetingNotes {
            account: Some("Acme Corp".to_string()),
        };
        let outcome = resolve_destination(&classification, workspace, "notes.md", None, Some(&db));
        match outcome {
            RouteOutcome::Destination(dest) => {
                assert_eq!(
                    dest,
                    PathBuf::from("/workspace/Accounts/Acme-Corp/Meeting-Notes/notes.md")
                );
            }
            other => panic!("Expected Destination, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_account_update_needs_entity_when_not_in_db() {
        let db = crate::db::test_utils::test_db();
        let workspace = Path::new("/workspace");

        let classification = Classification::AccountUpdate {
            account: "Unknown Co".to_string(),
        };
        let outcome = resolve_destination(&classification, workspace, "update.md", None, Some(&db));
        match outcome {
            RouteOutcome::NeedsEntity { suggested_name } => {
                assert_eq!(suggested_name, "Unknown Co");
            }
            other => panic!("Expected NeedsEntity, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_entity_override_bypasses_db_check() {
        let db = crate::db::test_utils::test_db();
        let workspace = Path::new("/workspace");

        // Account doesn't exist in DB, but entity_tracker_path is provided (user assignment)
        let classification = Classification::MeetingNotes {
            account: Some("Nonexistent Corp".to_string()),
        };
        let outcome = resolve_destination(
            &classification,
            workspace,
            "notes.md",
            Some("Accounts/Nonexistent-Corp"),
            Some(&db),
        );
        match outcome {
            RouteOutcome::Destination(dest) => {
                assert_eq!(
                    dest,
                    PathBuf::from("/workspace/Accounts/Nonexistent-Corp/Meeting-Notes/notes.md")
                );
            }
            other => panic!("Expected Destination (entity override), got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_with_entity_override_uses_tracker_path() {
        let workspace = Path::new("/workspace");
        let classification = Classification::MeetingNotes { account: None };
        let outcome = resolve_destination(
            &classification,
            workspace,
            "notes.md",
            Some("Internal/Acme/Core-Team"),
            None,
        );
        match outcome {
            RouteOutcome::Destination(dest) => assert_eq!(
                dest,
                PathBuf::from("/workspace/Internal/Acme/Core-Team/Meeting-Notes/notes.md")
            ),
            other => panic!("Expected Destination, got {:?}", other),
        }
    }
}

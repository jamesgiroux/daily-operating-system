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

/// Infer the most specific account tracker path from frontmatter, filename,
/// enrichment hints, and obvious business-unit directories.
///
/// Returns a relative tracker path like `Accounts/Crestview Media/Corporate-Services-B2B`
/// when it can be determined confidently, otherwise `None`.
pub fn infer_entity_tracker_path(
    workspace: &Path,
    filename: &str,
    content: &str,
    account_hint: Option<&str>,
    business_unit_hint: Option<&str>,
    db: Option<&ActionDb>,
) -> Option<String> {
    let frontmatter_account = frontmatter_value(content, "account");
    let frontmatter_business_unit = frontmatter_value(content, "business_unit")
        .or_else(|| frontmatter_value(content, "business-unit"));

    for candidate in [frontmatter_account.as_deref(), account_hint]
        .into_iter()
        .flatten()
    {
        if let Some(tp) = tracker_path_from_hint(candidate) {
            if tracker_path_exists(workspace, &tp) {
                return Some(tp);
            }
        }
    }

    let top_account = frontmatter_account
        .as_deref()
        .or(account_hint)
        .map(top_account_name)
        .filter(|s| !s.is_empty());
    let business_unit = frontmatter_business_unit
        .or_else(|| business_unit_hint.map(|s| s.to_string()))
        .or_else(|| {
            frontmatter_account
                .as_deref()
                .and_then(path_hint_business_unit)
                .or_else(|| account_hint.and_then(path_hint_business_unit))
        });
    let haystack = normalized_haystack(filename, content);

    if let Some(account) = top_account.as_deref() {
        if let Some(tp) =
            resolve_child_tracker_path_from_db(db, account, business_unit.as_deref(), &haystack)
        {
            return Some(tp);
        }
        if let Some(tp) = resolve_child_tracker_path_from_fs(
            workspace,
            account,
            business_unit.as_deref(),
            &haystack,
        ) {
            return Some(tp);
        }
        if let Some(tp) = resolve_exact_tracker_path_from_db(db, account) {
            return Some(tp);
        }

        let top_level_tp = format!("Accounts/{}", sanitize_dir_name(account));
        if tracker_path_exists(workspace, &top_level_tp) {
            return Some(top_level_tp);
        }
    }

    None
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

fn frontmatter_value(content: &str, key: &str) -> Option<String> {
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }

    let prefix = format!("{key}:");
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let value = rest.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

fn normalized_haystack(filename: &str, content: &str) -> String {
    let excerpt = content.lines().take(80).collect::<Vec<_>>().join(" ");
    normalize_hint(&format!("{filename} {excerpt}"))
}

fn normalize_hint(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    let mut prev_space = true;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            prev_space = false;
        } else if !prev_space {
            normalized.push(' ');
            prev_space = true;
        }
    }

    normalized.trim().to_string()
}

fn tracker_path_exists(workspace: &Path, tracker_path: &str) -> bool {
    workspace.join(tracker_path).is_dir()
}

fn tracker_path_from_hint(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return None;
    }

    if let Some(path) = trimmed.strip_prefix("Accounts/") {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return None;
        }
        return Some(format!("Accounts/{path}"));
    }

    let delimiter = if trimmed.contains('>') {
        '>'
    } else if trimmed.contains('/') {
        '/'
    } else {
        return None;
    };

    let parts: Vec<String> = trimmed
        .split(delimiter)
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect();

    if parts.len() < 2 {
        return None;
    }

    Some(format!("Accounts/{}", parts.join("/")))
}

fn top_account_name(value: &str) -> String {
    if let Some(tp) = tracker_path_from_hint(value) {
        return tp
            .trim_start_matches("Accounts/")
            .split('/')
            .next()
            .unwrap_or("")
            .to_string();
    }

    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn path_hint_business_unit(value: &str) -> Option<String> {
    let tp = tracker_path_from_hint(value)?;
    let parts: Vec<&str> = tp.trim_start_matches("Accounts/").split('/').collect();
    if parts.len() < 2 {
        return None;
    }
    Some(parts[1..].join("/"))
}

fn resolve_exact_tracker_path_from_db(db: Option<&ActionDb>, account_name: &str) -> Option<String> {
    let db = db?;
    let account = db.get_account_by_name(account_name).ok().flatten()?;
    account
        .tracker_path
        .or_else(|| Some(format!("Accounts/{}", sanitize_dir_name(&account.name))))
}

fn resolve_child_tracker_path_from_db(
    db: Option<&ActionDb>,
    top_account: &str,
    business_unit_hint: Option<&str>,
    haystack: &str,
) -> Option<String> {
    let db = db?;
    let parent = db.get_account_by_name(top_account).ok().flatten()?;
    let children = db.get_child_accounts(&parent.id).ok()?;
    let bu_hint = business_unit_hint.map(normalize_hint);

    let mut best: Option<(usize, String)> = None;
    for child in children {
        let child_name = normalize_hint(&child.name);
        let mut score = 0usize;
        if let Some(ref target) = bu_hint {
            if child_name == *target {
                score += 1000;
            }
        }
        if !child_name.is_empty() && haystack.contains(&child_name) {
            score += child_name.len();
        }
        if score == 0 {
            continue;
        }

        let tracker_path = child.tracker_path.unwrap_or_else(|| {
            format!(
                "Accounts/{}/{}",
                sanitize_dir_name(top_account),
                sanitize_dir_name(&child.name)
            )
        });
        match &best {
            Some((best_score, _)) if *best_score >= score => {}
            _ => best = Some((score, tracker_path)),
        }
    }

    best.map(|(_, tp)| tp)
}

fn resolve_child_tracker_path_from_fs(
    workspace: &Path,
    top_account: &str,
    business_unit_hint: Option<&str>,
    haystack: &str,
) -> Option<String> {
    let account_dir = workspace
        .join("Accounts")
        .join(sanitize_dir_name(top_account));
    let entries = std::fs::read_dir(&account_dir).ok()?;
    let bu_hint = business_unit_hint.map(normalize_hint);

    let mut best: Option<(usize, String)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if !crate::accounts::is_bu_directory(&name) {
            continue;
        }

        let normalized_name = normalize_hint(&name);
        let mut score = 0usize;
        if let Some(ref target) = bu_hint {
            if normalized_name == *target {
                score += 1000;
            }
        }
        if !normalized_name.is_empty() && haystack.contains(&normalized_name) {
            score += normalized_name.len();
        }
        if score == 0 {
            continue;
        }

        let tracker_path = format!("Accounts/{}/{}", sanitize_dir_name(top_account), name);
        match &best {
            Some((best_score, _)) if *best_score >= score => {}
            _ => best = Some((score, tracker_path)),
        }
    }

    best.map(|(_, tp)| tp)
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
            commercial_stage: None,
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

    #[test]
    fn test_infer_entity_tracker_path_from_explicit_filename() {
        let workspace =
            std::env::temp_dir().join(format!("dailyos-router-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(workspace.join("Accounts/Crestview Media/Corporate-Services-B2B")).unwrap();

        let inferred = infer_entity_tracker_path(
            &workspace,
            "2026-03-24-Crestview Media--Corporate-Services-B2B-On-Site_.md",
            "Plain text content",
            Some("Crestview Media"),
            None,
            None,
        );

        assert_eq!(
            inferred,
            Some("Accounts/Crestview Media/Corporate-Services-B2B".to_string())
        );

        let _ = std::fs::remove_dir_all(workspace);
    }

    #[test]
    fn test_infer_entity_tracker_path_from_frontmatter_account_path() {
        let workspace =
            std::env::temp_dir().join(format!("dailyos-router-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(workspace.join("Accounts/Crestview Media/Corporate-Services-B2B")).unwrap();

        let content = r#"---
account: "Crestview Media / Corporate-Services-B2B"
doc_type: summary
---

Meeting notes here.
"#;

        let inferred =
            infer_entity_tracker_path(&workspace, "notes.md", content, Some("Crestview Media"), None, None);

        assert_eq!(
            inferred,
            Some("Accounts/Crestview Media/Corporate-Services-B2B".to_string())
        );

        let _ = std::fs::remove_dir_all(workspace);
    }
}

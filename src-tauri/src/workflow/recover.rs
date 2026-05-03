//! Archive recovery — re-route stranded transcripts and meeting records.
//!
//! Walks `_archive/` date directories, finds files with valid meeting_id in
//! their YAML frontmatter, looks up entity links in the DB, and moves files
//! to their correct entity directory.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::db::ActionDb;

/// Result of the archive recovery process.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryReport {
    pub files_recovered: usize,
    pub files_skipped: usize,
    pub files_failed: Vec<(String, String)>,
    pub details: Vec<RecoveryDetail>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryDetail {
    pub source: String,
    pub destination: String,
    pub routing_method: String,
    pub meeting_title: String,
}

/// Extract a value from YAML frontmatter.
///
/// Parses the `---` delimited block at the start of a markdown file
/// and returns the value for the given key.
pub fn frontmatter_value(content: &str, key: &str) -> Option<String> {
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

/// Run the archive recovery process.
///
/// Walks `_archive/` date directories, finds transcript/record files with
/// valid YAML frontmatter, resolves their entity links, and moves them
/// to the correct entity directory.
pub fn recover_archived_transcripts(
    workspace: &Path,
    db: &ActionDb,
    user_domains: &[String],
) -> Result<RecoveryReport, String> {
    let archive_dir = workspace.join("_archive");
    if !archive_dir.exists() {
        return Ok(RecoveryReport {
            files_recovered: 0,
            files_skipped: 0,
            files_failed: Vec::new(),
            details: Vec::new(),
        });
    }

    let mut recovered = 0;
    let mut skipped = 0;
    let mut failed: Vec<(String, String)> = Vec::new();
    let mut details: Vec<RecoveryDetail> = Vec::new();

    // Walk date directories
    let mut date_dirs: Vec<_> = std::fs::read_dir(&archive_dir)
        .map_err(|e| format!("Failed to read _archive: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    date_dirs.sort_by_key(|e| e.file_name());

    for date_entry in date_dirs {
        let date_dir = date_entry.path();
        let files: Vec<_> = std::fs::read_dir(&date_dir)
            .map_err(|e| format!("Failed to read {}: {}", date_dir.display(), e))?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    == Some("md")
            })
            .collect();

        for file_entry in files {
            let file_path = file_entry.path();
            let content = match std::fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(e) => {
                    failed.push((file_path.display().to_string(), e.to_string()));
                    continue;
                }
            };

            // Parse frontmatter
            let meeting_id = match frontmatter_value(&content, "meeting_id") {
                Some(id) => id,
                None => {
                    // Not a processed transcript/record — skip
                    skipped += 1;
                    continue;
                }
            };

            let source = frontmatter_value(&content, "source");
            let meeting_type_str = frontmatter_value(&content, "meeting_type");

            // Determine if this is a transcript or record based on filename
            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let is_transcript = filename.contains("-transcript");
            let is_record = filename.contains("-record");

            if !is_transcript && !is_record && source.as_deref() != Some("transcript") {
                skipped += 1;
                continue;
            }

            let subdirectory = if is_record {
                "Meeting-Records"
            } else {
                "Call-Transcripts"
            };

            // Look up meeting in DB and try to find entity links
            let destination = resolve_recovery_destination(
                &meeting_id,
                meeting_type_str.as_deref(),
                db,
                workspace,
                filename,
                subdirectory,
                user_domains,
            );

            let (dest_path, routing_method) = match destination {
                Some((path, method)) => (path, method),
                None => {
                    // Can't resolve — leave in archive
                    skipped += 1;
                    continue;
                }
            };

            // Skip if destination is the same as source (already in right place)
            if dest_path == file_path {
                skipped += 1;
                continue;
            }

            // Skip if destination already exists
            if dest_path.exists() {
                skipped += 1;
                continue;
            }

            // Move file
            if let Some(parent) = dest_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    failed.push((
                        file_path.display().to_string(),
                        format!("Failed to create dir: {}", e),
                    ));
                    continue;
                }
            }

            if let Err(e) = std::fs::rename(&file_path, &dest_path) {
                // Try copy+delete for cross-filesystem
                if let Err(e2) = std::fs::copy(&file_path, &dest_path) {
                    failed.push((
                        file_path.display().to_string(),
                        format!("Move failed: {}, Copy failed: {}", e, e2),
                    ));
                    continue;
                }
                if let Err(e) = std::fs::remove_file(&file_path) {
                    log::warn!("Failed to remove source after copy: {}", e);
                }
            }

            // Update transcript_path in DB
            let now = chrono::Utc::now().to_rfc3339();
            if let Some(dest_str) = dest_path.to_str() {
                let _ = db.update_meeting_transcript_metadata(
                    &meeting_id,
                    dest_str,
                    &now,
                    None,
                );
            }

            let meeting_title = frontmatter_value(&content, "meeting_title")
                .unwrap_or_else(|| filename.to_string());

            // Emit signal for audit trail
            let _ = crate::signals::bus::emit_signal(
                db,
                "meeting",
                &meeting_id,
                "transcript_recovered",
                "archive_recovery",
                Some(routing_method),
                0.9,
            );

            log::info!(
                "I662: Recovered '{}' via {} → '{}'",
                meeting_title,
                routing_method,
                dest_path.display()
            );

            details.push(RecoveryDetail {
                source: file_path.display().to_string(),
                destination: dest_path.display().to_string(),
                routing_method: routing_method.to_string(),
                meeting_title,
            });

            recovered += 1;
        }
    }

    log::info!(
        "I662 recovery complete: {} recovered, {} skipped, {} failed",
        recovered,
        skipped,
        failed.len()
    );

    Ok(RecoveryReport {
        files_recovered: recovered,
        files_skipped: skipped,
        files_failed: failed,
        details,
    })
}

/// Resolve where a recovered file should go based on DB entity links
/// and attendee domain matching.
fn resolve_recovery_destination(
    meeting_id: &str,
    meeting_type_str: Option<&str>,
    db: &ActionDb,
    workspace: &Path,
    filename: &str,
    subdirectory: &str,
    user_domains: &[String],
) -> Option<(PathBuf, &'static str)> {
    // Look up meeting in DB
    let meeting = db.get_meeting_by_id(meeting_id).ok().flatten();

    // Try entity links first
    if let Ok(entities) = db.get_meeting_entities(meeting_id) {
        // Account entity
        if let Some(account_entity) = entities.iter().find(|e| {
            e.entity_type == crate::entity::EntityType::Account
        }) {
            let account_dir =
                crate::processor::transcript::sanitize_account_dir(&account_entity.name);
            return Some((
                workspace
                    .join("Accounts")
                    .join(&account_dir)
                    .join(subdirectory)
                    .join(filename),
                "entity_link_account",
            ));
        }

        // Project entity
        if let Some(project_entity) = entities.iter().find(|e| {
            e.entity_type == crate::entity::EntityType::Project
        }) {
            let project_dir =
                crate::processor::transcript::sanitize_account_dir(&project_entity.name);
            return Some((
                workspace
                    .join("Projects")
                    .join(&project_dir)
                    .join(subdirectory)
                    .join(filename),
                "entity_link_project",
            ));
        }

        // Person entity (1:1 only)
        let is_one_on_one = meeting_type_str == Some("oneonone")
            || meeting
                .as_ref()
                .map(|m| m.meeting_type == "one_on_one" || m.meeting_type == "oneonone")
                .unwrap_or(false);

        if is_one_on_one {
            if let Some(person_entity) = entities.iter().find(|e| {
                e.entity_type == crate::entity::EntityType::Person
            }) {
                let person_dir =
                    crate::processor::transcript::sanitize_account_dir(&person_entity.name);
                return Some((
                    workspace
                        .join("People")
                        .join(&person_dir)
                        .join(subdirectory)
                        .join(filename),
                    "entity_link_person",
                ));
            }
        }
    }

    // Attendee domain fallback
    if let Some(ref meeting) = meeting {
        if let Some(ref attendees_raw) = meeting.attendees {
            let attendees: Vec<String> =
                if let Ok(arr) = serde_json::from_str::<Vec<String>>(attendees_raw) {
                    arr
                } else {
                    attendees_raw
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                };

            let domains =
                crate::signals::event_trigger::extract_domains_from_attendees(&attendees, user_domains);

            let mut matched_accounts: Vec<(String, String)> = Vec::new();
            let mut seen_ids: HashSet<String> = HashSet::new();
            for domain in &domains {
                if let Ok(candidates) = db.lookup_account_candidates_by_domain(domain) {
                    for acct in candidates {
                        if seen_ids.insert(acct.id.clone()) {
                            matched_accounts.push((acct.id, acct.name));
                        }
                    }
                }
            }

            if matched_accounts.len() == 1 {
                let (_, ref name) = matched_accounts[0];
                let account_dir =
                    crate::processor::transcript::sanitize_account_dir(name);
                return Some((
                    workspace
                        .join("Accounts")
                        .join(&account_dir)
                        .join(subdirectory)
                        .join(filename),
                    "attendee_domain_recovery",
                ));
            }
        }
    }

    // Title-based matching: use EntityHint slugs and keywords against the meeting title,
    // replicating the same matching logic as classify.rs resolve_entities().
    let title = meeting
        .as_ref()
        .map(|m| m.title.to_lowercase())
        .unwrap_or_default();

    if !title.is_empty() {
        let hints = crate::helpers::build_entity_hints(db);
        let mut best_match: Option<(String, String, f64)> = None; // (id, name, confidence)

        for hint in &hints {
            if hint.entity_type != crate::entity::EntityType::Account {
                continue;
            }
            // Skip internal accounts
            if hint.id.starts_with("internal-") {
                continue;
            }

            let mut confidence = 0.0_f64;

            // Keyword matching (0.70)
            for kw in &hint.keywords {
                if title.contains(&kw.to_lowercase()) {
                    confidence = 0.70;
                    break;
                }
            }

            // Slug matching (0.50) — same 4-char minimum as classify.rs
            if confidence < 0.50 {
                for slug in &hint.slugs {
                    if slug.len() >= 4 && title.contains(slug.as_str()) {
                        confidence = 0.50;
                        break;
                    }
                }
            }

            // Also try matching the account name directly (case-insensitive)
            if confidence < 0.50 {
                let name_lower = hint.name.to_lowercase();
                if name_lower.len() >= 4 && title.contains(&name_lower) {
                    confidence = 0.55;
                }
            }

            if confidence > 0.0 {
                let dominated = match &best_match {
                    Some((_, _, c)) => confidence > *c,
                    None => true,
                };
                if dominated {
                    best_match = Some((hint.id.clone(), hint.name.clone(), confidence));
                }
            }
        }

        // Only route if exactly one account matched (check for ties)
        if let Some((_, ref name, best_conf)) = best_match {
            let tie_count = hints
                .iter()
                .filter(|h| {
                    h.entity_type == crate::entity::EntityType::Account
                        && !h.id.starts_with("internal-")
                })
                .filter(|h| {
                    // Re-check: does this hint also match at best_conf level?
                    let mut c = 0.0_f64;
                    for kw in &h.keywords {
                        if title.contains(&kw.to_lowercase()) {
                            c = 0.70;
                            break;
                        }
                    }
                    if c < 0.50 {
                        for slug in &h.slugs {
                            if slug.len() >= 4 && title.contains(slug.as_str()) {
                                c = 0.50;
                                break;
                            }
                        }
                    }
                    if c < 0.50 {
                        let nl = h.name.to_lowercase();
                        if nl.len() >= 4 && title.contains(&nl) {
                            c = 0.55;
                        }
                    }
                    c >= best_conf
                })
                .count();

            if tie_count == 1 {
                let account_dir =
                    crate::processor::transcript::sanitize_account_dir(name);
                log::info!(
                    "I662: Title-matched '{}' to account '{}' (confidence {:.2})",
                    title,
                    name,
                    best_conf
                );
                return Some((
                    workspace
                        .join("Accounts")
                        .join(&account_dir)
                        .join(subdirectory)
                        .join(filename),
                    "title_match_recovery",
                ));
            }
        }
    }

    // Internal meetings are expected in archive — don't try to route them
    let is_internal = matches!(
        meeting_type_str,
        Some("internal") | Some("teamsync") | Some("allhands") | Some("personal")
    );
    if is_internal {
        return None;
    }

    // Customer/QBR meeting without resolution — can't route
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontmatter_value_extracts_meeting_id() {
        let content = "---\nmeeting_id: \"abc123\"\nmeeting_title: \"Test\"\nmeeting_type: \"customer\"\n---\nContent here";
        assert_eq!(
            frontmatter_value(content, "meeting_id"),
            Some("abc123".to_string())
        );
        assert_eq!(
            frontmatter_value(content, "meeting_type"),
            Some("customer".to_string())
        );
    }

    #[test]
    fn test_frontmatter_value_returns_none_without_frontmatter() {
        let content = "# Just a heading\nSome content";
        assert_eq!(frontmatter_value(content, "meeting_id"), None);
    }

    #[test]
    fn test_frontmatter_value_handles_unquoted() {
        let content = "---\nsource: transcript\n---\n";
        assert_eq!(
            frontmatter_value(content, "source"),
            Some("transcript".to_string())
        );
    }
}

//! Inbox file processing pipeline.
//!
//! Orchestrates: classify → route (quick) or flag for AI enrichment → log.
//!
//! Quick processing handles files with recognizable filename patterns.
//! Files classified as Unknown are flagged for AI processing (Step 2.5).

pub mod classifier;
pub mod email_actions;
pub mod embeddings;
pub mod enrich;
pub mod extract;
pub mod hooks;
pub mod matcher;
pub mod metadata;
pub mod router;
pub mod transcript;

use std::path::Path;

use chrono::Utc;

use crate::db::{ActionDb, DbProcessingLog};
use classifier::{classify_file, Classification};
use extract::SupportedFormat;
use router::{move_file, resolve_destination, RouteOutcome};

/// Result of processing a single inbox file.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ProcessingResult {
    /// File was classified and routed to a destination.
    Routed {
        classification: String,
        destination: String,
    },
    /// File needs AI enrichment — left in inbox.
    NeedsEnrichment,
    /// Classification identified an entity not in DB — needs user assignment.
    NeedsEntity {
        classification: String,
        suggested_name: String,
    },
    /// Processing failed.
    Error { message: String },
}

/// Process a single inbox file: classify, route, and log.
///
/// Returns the processing result. The file is either:
/// - Moved to its PARA destination (if classifiable)
/// - Left in _inbox/ (if it needs AI enrichment)
/// - Left in place with an error logged (if routing fails)
pub fn process_file(
    workspace: &Path,
    filename: &str,
    db: Option<&ActionDb>,
    profile: &str,
    entity_tracker_path: Option<&str>,
) -> ProcessingResult {
    // I60: validate path stays within inbox
    let file_path = match crate::util::validate_inbox_path(workspace, filename) {
        Ok(p) => p,
        Err(e) => return ProcessingResult::Error { message: e },
    };

    if !file_path.exists() {
        return ProcessingResult::Error {
            message: format!("File not found: {}", filename),
        };
    }

    // Detect format and extract text for classification
    let format = extract::detect_format(&file_path);
    if matches!(format, SupportedFormat::Unsupported) {
        return ProcessingResult::Error {
            message: format!(
                "Unsupported file format: .{}",
                file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
            ),
        };
    }

    let content = match extract::extract_text(&file_path) {
        Ok(c) => c,
        Err(e) => {
            return ProcessingResult::Error {
                message: format!("Failed to extract text: {}", e),
            }
        }
    };
    let is_non_md = !matches!(format, SupportedFormat::Markdown);

    // Classify
    let classification = classify_file(&file_path, &content);
    let class_label = classification.label().to_string();

    log::info!("Classified '{}' as '{}'", filename, class_label);

    let account_hint = match &classification {
        Classification::MeetingNotes { account } | Classification::ActionItems { account } => {
            account.as_deref()
        }
        Classification::AccountUpdate { account } => Some(account.as_str()),
        _ => None,
    };
    let inferred_tracker_path = if entity_tracker_path.is_none() {
        router::infer_entity_tracker_path(workspace, filename, &content, account_hint, None, db)
    } else {
        None
    };

    // Resolve destination (pass db for entity validation)
    let route_outcome = resolve_destination(
        &classification,
        workspace,
        filename,
        entity_tracker_path.or(inferred_tracker_path.as_deref()),
        db,
    );

    let result = match route_outcome {
        RouteOutcome::Destination(dest) => {
            // Route the file
            match move_file(&file_path, &dest) {
                Ok(route_result) => {
                    log::info!(
                        "Routed '{}' to '{}'",
                        filename,
                        route_result.destination.display()
                    );

                    // Write companion .md for non-markdown files
                    if is_non_md {
                        let companion_path = extract::companion_md_path(&route_result.destination);
                        let companion_content =
                            extract::build_companion_md(filename, format, &content);
                        if let Err(e) =
                            crate::util::atomic_write_str(&companion_path, &companion_content)
                        {
                            log::warn!("Failed to write companion .md for '{}': {}", filename, e);
                        } else {
                            log::info!("Created companion .md at '{}'", companion_path.display());
                        }
                    }

                    // Extract actions if applicable
                    if matches!(classification, Classification::ActionItems { .. }) {
                        if let Some(db) = db {
                            let account_fallback = match &classification {
                                Classification::ActionItems { account } => account.as_deref(),
                                _ => None,
                            };
                            extract_and_sync_actions(&content, filename, db, account_fallback);
                        }
                    }

                    // I474: Match MeetingNotes to historical meetings
                    if matches!(classification, Classification::MeetingNotes { .. }) {
                        if let Some(db) = db {
                            try_match_to_meeting(&classification, filename, &dest, db);
                        }
                    }

                    // Run post-enrichment hooks
                    if let Some(db) = db {
                        let ctx = hooks::EnrichmentContext {
                            workspace: workspace.to_path_buf(),
                            filename: filename.to_string(),
                            classification: class_label.clone(),
                            account: match &classification {
                                Classification::MeetingNotes { account } => account.clone(),
                                Classification::AccountUpdate { account } => Some(account.clone()),
                                Classification::ActionItems { account } => account.clone(),
                                _ => None,
                            },
                            summary: String::new(),
                            actions: Vec::new(), // actions already extracted above
                            destination_path: Some(dest.display().to_string()),
                            profile: profile.to_string(),
                            wins: Vec::new(), // quick path has no AI extraction
                            risks: Vec::new(),
                            entity_type: None,
                        };
                        let hook_results = hooks::run_post_enrichment_hooks(&ctx, db);
                        for hr in &hook_results {
                            log::info!(
                                "Post-enrichment hook '{}': {} — {}",
                                hr.hook_name,
                                if hr.success { "OK" } else { "FAILED" },
                                hr.message.as_deref().unwrap_or("")
                            );
                        }
                    }

                    ProcessingResult::Routed {
                        classification: class_label.clone(),
                        destination: dest.display().to_string(),
                    }
                }
                Err(e) => ProcessingResult::Error {
                    message: format!("Failed to route: {}", e),
                },
            }
        }
        RouteOutcome::NeedsEntity { suggested_name } => {
            log::info!(
                "'{}' needs entity assignment (suggested: '{}') — leaving in inbox",
                filename,
                suggested_name
            );
            ProcessingResult::NeedsEntity {
                classification: class_label.clone(),
                suggested_name,
            }
        }
        RouteOutcome::NeedsEnrichment => {
            log::info!("'{}' needs AI enrichment — leaving in inbox", filename);
            ProcessingResult::NeedsEnrichment
        }
    };

    // Log to database
    if let Some(db) = db {
        let log_entry = DbProcessingLog {
            id: uuid::Uuid::new_v4().to_string(),
            filename: filename.to_string(),
            source_path: file_path.display().to_string(),
            destination_path: match &result {
                ProcessingResult::Routed { destination, .. } => Some(destination.clone()),
                _ => None,
            },
            classification: class_label,
            status: match &result {
                ProcessingResult::Routed { .. } => "completed".to_string(),
                ProcessingResult::NeedsEnrichment => "needs_enrichment".to_string(),
                ProcessingResult::NeedsEntity { .. } => "needs_entity".to_string(),
                ProcessingResult::Error { .. } => "error".to_string(),
            },
            processed_at: Some(Utc::now().to_rfc3339()),
            error_message: match &result {
                ProcessingResult::Error { message } => Some(message.clone()),
                ProcessingResult::NeedsEntity { suggested_name, .. } => {
                    Some(suggested_name.clone())
                }
                _ => None,
            },
            created_at: Utc::now().to_rfc3339(),
        };

        if let Err(e) = db.insert_processing_log(&log_entry) {
            log::warn!("Failed to log processing result: {}", e);
        }
    }

    result
}

/// Process a user attachment from _user/attachments/.
///
/// Unlike inbox processing, user attachments:
/// - Stay in _user/attachments/ (no routing/move)
/// - Are indexed as `user_context` entity type
/// - Get embedded with `user_context` collection for semantic retrieval
pub fn process_user_attachment(
    workspace: &Path,
    file_path: &Path,
    db: Option<&crate::db::ActionDb>,
) -> ProcessingResult {
    if !file_path.exists() {
        return ProcessingResult::Error {
            message: format!("File not found: {}", file_path.display()),
        };
    }

    // Validate file is within _user/attachments/
    let attachments_dir = workspace.join("_user").join("attachments");
    if !file_path.starts_with(&attachments_dir) {
        return ProcessingResult::Error {
            message: "File is not in _user/attachments/".to_string(),
        };
    }

    let filename = match file_path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n.to_string(),
        None => {
            return ProcessingResult::Error {
                message: "Invalid filename".to_string(),
            }
        }
    };

    // Detect format
    let format = extract::detect_format(file_path);
    if matches!(format, SupportedFormat::Unsupported) {
        return ProcessingResult::Error {
            message: format!(
                "Unsupported file format: .{}",
                file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
            ),
        };
    }

    // Extract text
    let content = match extract::extract_text(file_path) {
        Ok(c) => c,
        Err(e) => {
            return ProcessingResult::Error {
                message: format!("Failed to extract text: {}", e),
            }
        }
    };

    // Index as a user_context content file in the DB
    if let Some(db) = db {
        let now = chrono::Utc::now().to_rfc3339();
        let metadata = std::fs::metadata(file_path).ok();
        let file_size = metadata.as_ref().map(|m| m.len() as i64).unwrap_or(0);
        let modified_at = metadata
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_else(|| now.clone());

        let relative_path = file_path
            .strip_prefix(workspace)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| filename.clone());

        let id = crate::util::slugify(&format!("user-context/{}", filename));

        // Generate a mechanical summary
        let summary = if !content.trim().is_empty() {
            Some(crate::intelligence::mechanical_summary(&content, 500))
        } else {
            None
        };

        let record = crate::db::DbContentFile {
            id,
            entity_id: "user_context".to_string(),
            entity_type: "user_context".to_string(),
            filename: filename.clone(),
            relative_path,
            absolute_path: file_path.to_string_lossy().to_string(),
            format: format!("{:?}", format),
            file_size,
            modified_at,
            indexed_at: now.clone(),
            extracted_at: Some(now.clone()),
            summary,
            embeddings_generated_at: None,
            content_type: "user_context".to_string(),
            priority: 10, // High priority — user-provided context
        };

        if let Err(e) = db.upsert_content_file(&record) {
            log::warn!("Failed to index user attachment '{}': {}", filename, e);
        } else {
            log::info!("Indexed user attachment '{}' as user_context", filename);
        }

        // Log processing
        let log_entry = DbProcessingLog {
            id: uuid::Uuid::new_v4().to_string(),
            filename: filename.clone(),
            source_path: file_path.display().to_string(),
            destination_path: Some(file_path.display().to_string()),
            classification: "user_context".to_string(),
            status: "completed".to_string(),
            processed_at: Some(now.clone()),
            error_message: None,
            created_at: now,
        };
        if let Err(e) = db.insert_processing_log(&log_entry) {
            log::warn!("Failed to log user attachment processing: {}", e);
        }
    }

    ProcessingResult::Routed {
        classification: "user_context".to_string(),
        destination: file_path.display().to_string(),
    }
}

/// Process all files in the inbox.
///
/// Returns a summary of results: (routed, needs_enrichment, errors).
pub fn process_all(
    workspace: &Path,
    db: Option<&ActionDb>,
    profile: &str,
) -> Vec<(String, ProcessingResult)> {
    let inbox_dir = workspace.join("_inbox");

    if !inbox_dir.exists() {
        return Vec::new();
    }

    let mut results = Vec::new();

    let entries = match std::fs::read_dir(&inbox_dir) {
        Ok(e) => e,
        Err(e) => {
            log::error!("Failed to read inbox directory: {}", e);
            return Vec::new();
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip directories and hidden files
        if !path.is_file() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                continue;
            }
        }

        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            let result = process_file(workspace, filename, db, profile, None);
            results.push((filename.to_string(), result));
        }
    }

    results
}

/// Extract action items from file content and sync to SQLite.
///
/// Looks for markdown checkboxes (- [ ] / - [x]) and inserts them as actions.
/// Parses inline metadata tokens (priority, @account, due date, #context, waiting).
fn extract_and_sync_actions(
    content: &str,
    source_filename: &str,
    db: &ActionDb,
    account_fallback: Option<&str>,
) {
    let now = Utc::now().to_rfc3339();
    let mut count = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // Match markdown checkboxes
        let (is_completed, raw_title) = if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            (false, rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix("- [x] ") {
            (true, rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix("* [ ] ") {
            (false, rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix("* [x] ") {
            (true, rest.trim())
        } else {
            continue;
        };

        if raw_title.is_empty() {
            continue;
        }

        let meta = metadata::parse_action_metadata(raw_title);

        // Determine status: explicit completion > unstarted
        let status = if is_completed {
            crate::action_status::COMPLETED.to_string()
        } else {
            crate::action_status::UNSTARTED.to_string()
        };

        // Account resolution: @Account in text > classifier fallback > None
        let account_id = meta
            .account
            .clone()
            .or_else(|| account_fallback.map(String::from));

        let action = crate::db::DbAction {
            id: format!(
                "inbox-{}-{}",
                source_filename.trim_end_matches(".md"),
                count
            ),
            title: meta.clean_title,
            priority: meta
                .priority
                .map(|p| crate::action_status::migrate_priority(&p))
                .unwrap_or(crate::action_status::PRIORITY_MEDIUM),
            status,
            created_at: now.clone(),
            due_date: meta.due_date,
            completed_at: if is_completed {
                Some(now.clone())
            } else {
                None
            },
            account_id,
            project_id: None,
            source_type: Some("inbox".to_string()),
            source_id: Some(raw_title.to_string()),
            source_label: Some(source_filename.to_string()),
            action_kind: crate::action_status::KIND_TASK.to_string(),
            context: meta.context,
            waiting_on: if meta.is_waiting {
                Some("true".to_string())
            } else {
                None
            },
            updated_at: now.clone(),
            person_id: None,
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
            needs_decision: false,
            decision_owner: None,
            decision_stakes: None,
            linear_identifier: None,
            linear_url: None,
        };

        if let Err(e) = db.upsert_action_if_not_completed(&action) {
            log::warn!("Failed to extract action from {}: {}", source_filename, e);
        } else {
            count += 1;
        }
    }

    if count > 0 {
        log::info!("Extracted {} actions from '{}'", count, source_filename);
    }
}

/// I474: Attempt to match a MeetingNotes document to a historical meeting.
///
/// Queries recent meetings (last 14 days), runs the multi-signal matcher,
/// and on confident match: updates the meeting's transcript metadata and
/// emits a `transcript_outcomes` signal for entity intelligence.
fn try_match_to_meeting(
    classification: &Classification,
    filename: &str,
    destination: &std::path::Path,
    db: &crate::db::ActionDb,
) {
    // Extract account name from classification for entity matching
    let account_name = match classification {
        Classification::MeetingNotes { account } => account.as_deref(),
        _ => return,
    };

    // Resolve account name to entity_id
    let doc_entity_id =
        account_name.and_then(|name| db.get_account_by_name(name).ok().flatten().map(|a| a.id));

    // Query recent meetings with entity context
    let raw_candidates = match db.get_recent_meetings_for_matching(14) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("I474: Failed to query meeting candidates: {}", e);
            return;
        }
    };

    if raw_candidates.is_empty() {
        return;
    }

    // Build matcher candidates
    let candidates: Vec<matcher::MeetingCandidate> = raw_candidates
        .iter()
        .map(
            |(id, title, start_time, entity_id)| matcher::MeetingCandidate {
                meeting_id: id.clone(),
                title: title.clone(),
                start_time: start_time.parse::<chrono::DateTime<chrono::Utc>>().ok(),
                entity_id: entity_id.clone(),
            },
        )
        .collect();

    // Use filename stem as document title for matching
    let doc_title = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename)
        .replace('-', " ");

    // Use current time as document time (inbox files processed near their creation)
    let doc_time = Some(Utc::now());

    let match_result =
        matcher::find_best_match(&doc_title, doc_time, doc_entity_id.as_deref(), &candidates);

    if let Some(m) = match_result {
        log::info!(
            "I474: Matched '{}' to meeting '{}' (score={}, confidence={:.2})",
            filename,
            m.meeting_id,
            m.score,
            m.confidence,
        );

        // Update the meeting's transcript metadata to link this document
        let now = Utc::now().to_rfc3339();
        if let Err(e) = db.update_meeting_transcript_metadata(
            &m.meeting_id,
            &destination.display().to_string(),
            &now,
            None,
        ) {
            log::warn!(
                "I474: Failed to update transcript metadata for meeting '{}': {}",
                m.meeting_id,
                e,
            );
        }

        // Emit transcript_outcomes signal for entity intelligence
        let entity_type = "account";
        let entity_id = doc_entity_id.as_deref().unwrap_or(&m.meeting_id);
        let _ = crate::signals::bus::emit_signal(
            db,
            entity_type,
            entity_id,
            "transcript_outcomes",
            "inbox_matcher",
            Some(&format!(
                "{{\"meeting_id\":\"{}\",\"source\":\"inbox_document\",\"filename\":\"{}\"}}",
                m.meeting_id, filename,
            )),
            m.confidence * 0.75, // slightly discount confidence vs direct transcript processing
        );
    } else {
        log::info!("I474: No meeting match found for '{}'", filename);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_plaintext_file_routes_and_creates_companion() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();

        // Create inbox and archive dirs
        std::fs::create_dir_all(workspace.join("_inbox")).unwrap();
        std::fs::create_dir_all(workspace.join("_archive")).unwrap();

        // Write a .txt file with meeting-notes pattern
        let filename = "acme-meeting-notes.txt";
        std::fs::write(
            workspace.join("_inbox").join(filename),
            "Meeting with Acme about renewal.",
        )
        .unwrap();

        let result = process_file(workspace, filename, None, "customer-success", None);

        // Should be classified and routed
        match &result {
            ProcessingResult::Routed {
                classification,
                destination,
            } => {
                assert_eq!(classification, "meeting_notes");
                // Companion .md should exist alongside the .txt
                let dest_path = std::path::Path::new(destination);
                let companion = dest_path.parent().unwrap().join("acme-meeting-notes.md");
                assert!(
                    companion.exists(),
                    "Companion .md should exist at {}",
                    companion.display()
                );

                let companion_content = std::fs::read_to_string(&companion).unwrap();
                assert!(companion_content.contains("source: acme-meeting-notes.txt"));
                assert!(companion_content.contains("format: plaintext"));
                assert!(companion_content.contains("Meeting with Acme about renewal."));
            }
            other => panic!("Expected Routed, got {:?}", other),
        }
    }

    #[test]
    fn test_process_md_file_no_companion() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();

        std::fs::create_dir_all(workspace.join("_inbox")).unwrap();
        std::fs::create_dir_all(workspace.join("_archive")).unwrap();

        let filename = "acme-meeting-notes.md";
        std::fs::write(
            workspace.join("_inbox").join(filename),
            "# Meeting Notes\nContent here.",
        )
        .unwrap();

        let result = process_file(workspace, filename, None, "customer-success", None);

        match &result {
            ProcessingResult::Routed { destination, .. } => {
                // For .md files, no companion should be created
                let dest_path = std::path::Path::new(destination);
                let parent = dest_path.parent().unwrap();
                // The only .md file in the dir should be the original
                let md_files: Vec<_> = std::fs::read_dir(parent)
                    .unwrap()
                    .flatten()
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                    .collect();
                assert_eq!(
                    md_files.len(),
                    1,
                    "Only the original .md should exist (no companion)"
                );
            }
            other => panic!("Expected Routed, got {:?}", other),
        }
    }

    #[test]
    fn test_process_unsupported_format_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();

        std::fs::create_dir_all(workspace.join("_inbox")).unwrap();

        let filename = "photo.png";
        std::fs::write(
            workspace.join("_inbox").join(filename),
            [0x89, 0x50, 0x4E, 0x47],
        )
        .unwrap();

        let result = process_file(workspace, filename, None, "customer-success", None);
        assert!(matches!(result, ProcessingResult::Error { .. }));
    }

    #[test]
    fn test_process_csv_file_classified_by_content() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();

        std::fs::create_dir_all(workspace.join("_inbox")).unwrap();
        std::fs::create_dir_all(workspace.join("_archive")).unwrap();

        // CSV with action items in content — content-based classification
        let filename = "tasks.csv";
        let content = "# Tasks\n\n- [ ] Item one\n- [ ] Item two\n- [ ] Item three\n";
        std::fs::write(workspace.join("_inbox").join(filename), content).unwrap();

        let result = process_file(workspace, filename, None, "customer-success", None);

        match &result {
            ProcessingResult::Routed {
                classification,
                destination,
            } => {
                assert_eq!(classification, "action_items");
                // Companion .md should exist
                let dest_path = std::path::Path::new(destination);
                let companion = dest_path.parent().unwrap().join("tasks.md");
                assert!(companion.exists());
            }
            other => panic!("Expected Routed, got {:?}", other),
        }
    }

    #[test]
    fn test_process_user_attachment_indexes_as_user_context() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();

        // Create _user/attachments/ directory
        let attachments_dir = workspace.join("_user").join("attachments");
        std::fs::create_dir_all(&attachments_dir).unwrap();

        // Write a test file
        let file_path = attachments_dir.join("my-resume.md");
        std::fs::write(
            &file_path,
            "# Resume\n\nSenior CS Manager with 10 years experience.",
        )
        .unwrap();

        let result = process_user_attachment(workspace, &file_path, None);

        match &result {
            ProcessingResult::Routed {
                classification,
                destination,
            } => {
                assert_eq!(classification, "user_context");
                // File should stay in place
                assert_eq!(destination, &file_path.display().to_string());
                assert!(file_path.exists(), "File should not be moved");
            }
            other => panic!("Expected Routed, got {:?}", other),
        }
    }

    #[test]
    fn test_process_user_attachment_rejects_outside_attachments() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();

        // Create a file outside _user/attachments/
        std::fs::create_dir_all(workspace.join("_inbox")).unwrap();
        let file_path = workspace.join("_inbox").join("some-file.md");
        std::fs::write(&file_path, "content").unwrap();

        let result = process_user_attachment(workspace, &file_path, None);
        assert!(matches!(result, ProcessingResult::Error { .. }));
    }

    #[test]
    fn test_process_user_attachment_unsupported_format() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path();

        let attachments_dir = workspace.join("_user").join("attachments");
        std::fs::create_dir_all(&attachments_dir).unwrap();

        let file_path = attachments_dir.join("photo.png");
        std::fs::write(&file_path, [0x89, 0x50, 0x4E, 0x47]).unwrap();

        let result = process_user_attachment(workspace, &file_path, None);
        assert!(matches!(result, ProcessingResult::Error { .. }));
    }
}

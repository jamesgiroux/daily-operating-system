//! Inbox file processing pipeline.
//!
//! Orchestrates: classify → route (quick) or flag for AI enrichment → log.
//!
//! Quick processing handles files with recognizable filename patterns.
//! Files classified as Unknown are flagged for AI processing (Step 2.5).

pub mod classifier;
pub mod enrich;
pub mod hooks;
pub mod metadata;
pub mod router;

use std::path::Path;

use chrono::Utc;

use crate::db::{ActionDb, DbProcessingLog};
use classifier::{classify_file, Classification};
use router::{move_file, resolve_destination};

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
) -> ProcessingResult {
    let inbox_dir = workspace.join("_inbox");
    let file_path = inbox_dir.join(filename);

    if !file_path.exists() {
        return ProcessingResult::Error {
            message: format!("File not found: {}", filename),
        };
    }

    // Read content for classification
    let content = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(e) => {
            return ProcessingResult::Error {
                message: format!("Failed to read file: {}", e),
            }
        }
    };

    // Classify
    let classification = classify_file(&file_path, &content);
    let class_label = classification.label().to_string();

    log::info!(
        "Classified '{}' as '{}'",
        filename,
        class_label
    );

    // Resolve destination
    let destination = resolve_destination(&classification, workspace, filename);

    let result = match destination {
        Some(dest) => {
            // Route the file
            match move_file(&file_path, &dest) {
                Ok(route_result) => {
                    log::info!("Routed '{}' to '{}'", filename, route_result.destination.display());

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

                    // Run post-enrichment hooks
                    if let Some(db) = db {
                        let ctx = hooks::EnrichmentContext {
                            workspace: workspace.to_path_buf(),
                            filename: filename.to_string(),
                            classification: class_label.clone(),
                            account: match &classification {
                                Classification::MeetingNotes { account } => account.clone(),
                                Classification::AccountUpdate { account } => {
                                    Some(account.clone())
                                }
                                Classification::ActionItems { account } => account.clone(),
                                _ => None,
                            },
                            summary: String::new(),
                            actions: Vec::new(), // actions already extracted above
                            destination_path: Some(dest.display().to_string()),
                            profile: profile.to_string(),
                            wins: Vec::new(),  // quick path has no AI extraction
                            risks: Vec::new(),
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
        None => {
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
                ProcessingResult::Error { .. } => "error".to_string(),
            },
            processed_at: Some(Utc::now().to_rfc3339()),
            error_message: match &result {
                ProcessingResult::Error { message } => Some(message.clone()),
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
            let result = process_file(workspace, filename, db, profile);
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

        // Determine status: explicit completion > waiting > pending
        let status = if is_completed {
            "completed".to_string()
        } else if meta.is_waiting {
            "waiting".to_string()
        } else {
            "pending".to_string()
        };

        // Account resolution: @Account in text > classifier fallback > None
        let account_id = meta
            .account
            .clone()
            .or_else(|| account_fallback.map(String::from));

        let action = crate::db::DbAction {
            id: format!("inbox-{}-{}", source_filename.trim_end_matches(".md"), count),
            title: meta.clean_title,
            priority: meta.priority.unwrap_or_else(|| "P2".to_string()),
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
            context: meta.context,
            waiting_on: if meta.is_waiting {
                Some("true".to_string())
            } else {
                None
            },
            updated_at: now.clone(),
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

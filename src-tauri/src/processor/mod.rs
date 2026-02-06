//! Inbox file processing pipeline.
//!
//! Orchestrates: classify → route (quick) or flag for AI enrichment → log.
//!
//! Quick processing handles files with recognizable filename patterns.
//! Files classified as Unknown are flagged for AI processing (Step 2.5).

pub mod classifier;
pub mod enrich;
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
                            extract_and_sync_actions(&content, filename, db);
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

        // Only process .md files
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            let result = process_file(workspace, filename, db);
            results.push((filename.to_string(), result));
        }
    }

    results
}

/// Extract action items from file content and sync to SQLite.
///
/// Looks for markdown checkboxes (- [ ] / - [x]) and inserts them as actions.
fn extract_and_sync_actions(content: &str, source_filename: &str, db: &ActionDb) {
    let now = Utc::now().to_rfc3339();
    let mut count = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // Match markdown checkboxes
        let (is_completed, title) = if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
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

        if title.is_empty() {
            continue;
        }

        let action = crate::db::DbAction {
            id: format!("inbox-{}-{}", source_filename.trim_end_matches(".md"), count),
            title: title.to_string(),
            priority: "P2".to_string(),
            status: if is_completed {
                "completed".to_string()
            } else {
                "pending".to_string()
            },
            created_at: now.clone(),
            due_date: None,
            completed_at: if is_completed {
                Some(now.clone())
            } else {
                None
            },
            account_id: None,
            project_id: None,
            source_type: Some("inbox".to_string()),
            source_id: None,
            source_label: Some(source_filename.to_string()),
            context: None,
            waiting_on: None,
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

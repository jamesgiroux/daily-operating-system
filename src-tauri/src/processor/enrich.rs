//! AI enrichment for inbox files via Claude Code.
//!
//! Files that can't be classified by filename patterns get sent to Claude Code
//! for AI-powered classification and action extraction.

use std::path::Path;

use chrono::Utc;

use crate::db::{ActionDb, DbProcessingLog};
use crate::pty::PtyManager;

use super::classifier::Classification;
use super::router::{move_file, resolve_destination};

/// Timeout for AI processing per file (2 minutes)
const AI_TIMEOUT_SECS: u64 = 120;

/// Result of AI enrichment.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EnrichResult {
    /// AI classified and routed the file.
    Routed {
        classification: String,
        destination: String,
        summary: String,
    },
    /// AI processed but couldn't determine a route. Archived.
    Archived {
        summary: String,
        destination: String,
    },
    /// AI processing failed.
    Error { message: String },
}

/// Process a file with AI enrichment via Claude Code.
///
/// Sends the file content to Claude with context about the workspace structure,
/// asks for classification, summary, and action extraction.
pub fn enrich_file(
    workspace: &Path,
    filename: &str,
    db: Option<&ActionDb>,
    profile: &str,
) -> EnrichResult {
    let inbox_dir = workspace.join("_inbox");
    let file_path = inbox_dir.join(filename);

    // Read the file
    let content = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(e) => {
            return EnrichResult::Error {
                message: format!("Failed to read file: {}", e),
            }
        }
    };

    // Build the prompt for Claude
    let prompt = build_enrichment_prompt(filename, &content);

    // Invoke Claude Code via PTY
    let pty = PtyManager::new().with_timeout(AI_TIMEOUT_SECS);
    let output = match pty.spawn_claude(workspace, &prompt) {
        Ok(o) => o.stdout,
        Err(e) => {
            log::error!("AI enrichment failed for '{}': {}", filename, e);
            return EnrichResult::Error {
                message: format!("Claude Code failed: {}", e),
            };
        }
    };

    // Parse Claude's response
    let parsed = parse_enrichment_response(&output);

    // Extract actions if any
    if let Some(ref actions_text) = parsed.actions_text {
        if let Some(db) = db {
            extract_actions_from_ai(actions_text, filename, db);
        }
    }

    // Determine destination
    let classification = match parsed.file_type.as_str() {
        "meeting_notes" => Classification::MeetingNotes {
            account: parsed.account.clone(),
        },
        "account_update" => {
            if let Some(ref account) = parsed.account {
                Classification::AccountUpdate {
                    account: account.clone(),
                }
            } else {
                Classification::Unknown
            }
        }
        "action_items" => Classification::ActionItems {
            account: parsed.account.clone(),
        },
        "meeting_context" => Classification::MeetingContext {
            meeting_name: parsed.meeting_name.clone(),
        },
        _ => Classification::Unknown,
    };

    let destination = resolve_destination(&classification, workspace, filename);

    // Capture fields before the match to avoid borrow-after-move issues
    let summary = parsed.summary.clone();
    let file_type = parsed.file_type.clone();
    let account = parsed.account.clone();

    let result = match destination {
        Some(dest) => match move_file(&file_path, &dest) {
            Ok(_) => EnrichResult::Routed {
                classification: file_type.clone(),
                destination: dest.display().to_string(),
                summary: summary.clone(),
            },
            Err(e) => EnrichResult::Error {
                message: format!("Failed to route: {}", e),
            },
        },
        None => {
            // Even if unknown, archive it with AI summary
            let date = Utc::now().format("%Y-%m-%d").to_string();
            let archive_dest = workspace.join("_archive").join(&date).join(filename);
            match move_file(&file_path, &archive_dest) {
                Ok(_) => EnrichResult::Archived {
                    summary: summary.clone(),
                    destination: archive_dest.display().to_string(),
                },
                Err(e) => EnrichResult::Error {
                    message: format!("Failed to archive: {}", e),
                },
            }
        }
    };

    // Run post-enrichment hooks
    if let Some(db) = db {
        let ctx = super::hooks::EnrichmentContext {
            workspace: workspace.to_path_buf(),
            filename: filename.to_string(),
            classification: file_type.clone(),
            account: account.clone(),
            summary: summary.clone(),
            actions: Vec::new(), // actions already extracted by extract_actions_from_ai
            destination_path: match &result {
                EnrichResult::Routed { destination, .. }
                | EnrichResult::Archived { destination, .. } => Some(destination.clone()),
                _ => None,
            },
            profile: profile.to_string(),
        };
        let hook_results = super::hooks::run_post_enrichment_hooks(&ctx, db);
        for hr in &hook_results {
            log::info!(
                "Post-enrichment hook '{}': {} — {}",
                hr.hook_name,
                if hr.success { "OK" } else { "FAILED" },
                hr.message.as_deref().unwrap_or("")
            );
        }
    }

    // Log to database
    if let Some(db) = db {
        let log_entry = DbProcessingLog {
            id: uuid::Uuid::new_v4().to_string(),
            filename: filename.to_string(),
            source_path: file_path.display().to_string(),
            destination_path: match &result {
                EnrichResult::Routed { destination, .. }
                | EnrichResult::Archived { destination, .. } => Some(destination.clone()),
                _ => None,
            },
            classification: file_type,
            status: match &result {
                EnrichResult::Routed { .. } | EnrichResult::Archived { .. } => {
                    "completed".to_string()
                }
                EnrichResult::Error { .. } => "error".to_string(),
            },
            processed_at: Some(Utc::now().to_rfc3339()),
            error_message: match &result {
                EnrichResult::Error { message } => Some(message.clone()),
                _ => None,
            },
            created_at: Utc::now().to_rfc3339(),
        };

        if let Err(e) = db.insert_processing_log(&log_entry) {
            log::warn!("Failed to log enrichment result: {}", e);
        }
    }

    result
}

/// Build the prompt for Claude Code enrichment.
fn build_enrichment_prompt(filename: &str, content: &str) -> String {
    // Truncate very long content to fit in a reasonable prompt.
    // Must find a valid UTF-8 char boundary — slicing at an arbitrary byte panics.
    let truncated = if content.len() > 8000 {
        let mut end = 8000;
        while end > 0 && !content.is_char_boundary(end) {
            end -= 1;
        }
        &content[..end]
    } else {
        content
    };

    format!(
        r#"Analyze this inbox file and respond in exactly this format:

FILE_TYPE: <one of: meeting_notes, account_update, action_items, meeting_context, general>
ACCOUNT: <account name if relevant, or NONE>
MEETING: <meeting name if relevant, or NONE>
SUMMARY: <one-line summary>
ACTIONS:
- <action item 1>
- <action item 2>
END_ACTIONS

Filename: {}
Content:
{}
"#,
        filename, truncated
    )
}

/// Parsed response from Claude Code enrichment.
struct ParsedEnrichment {
    file_type: String,
    account: Option<String>,
    meeting_name: Option<String>,
    summary: String,
    actions_text: Option<String>,
}

/// Parse Claude's enrichment response.
fn parse_enrichment_response(output: &str) -> ParsedEnrichment {
    let mut file_type = "general".to_string();
    let mut account = None;
    let mut meeting_name = None;
    let mut summary = String::new();
    let mut actions_text = None;
    let mut in_actions = false;
    let mut actions_buf = String::new();

    for line in output.lines() {
        let line = line.trim();

        if let Some(rest) = line.strip_prefix("FILE_TYPE:") {
            file_type = rest.trim().to_lowercase();
        } else if let Some(rest) = line.strip_prefix("ACCOUNT:") {
            let val = rest.trim();
            if val != "NONE" && !val.is_empty() {
                account = Some(val.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("MEETING:") {
            let val = rest.trim();
            if val != "NONE" && !val.is_empty() {
                meeting_name = Some(val.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("SUMMARY:") {
            summary = rest.trim().to_string();
        } else if line == "ACTIONS:" {
            in_actions = true;
        } else if line == "END_ACTIONS" {
            in_actions = false;
            if !actions_buf.is_empty() {
                actions_text = Some(actions_buf.clone());
            }
        } else if in_actions && line.starts_with("- ") {
            if !actions_buf.is_empty() {
                actions_buf.push('\n');
            }
            actions_buf.push_str(line);
        }
    }

    // If Claude emitted ACTIONS: but never END_ACTIONS, capture buffered actions
    if in_actions && !actions_buf.is_empty() && actions_text.is_none() {
        actions_text = Some(actions_buf);
    }

    ParsedEnrichment {
        file_type,
        account,
        meeting_name,
        summary,
        actions_text,
    }
}

/// Extract actions from AI-generated action text and sync to SQLite.
fn extract_actions_from_ai(actions_text: &str, source_filename: &str, db: &ActionDb) {
    let now = Utc::now().to_rfc3339();
    let mut count = 0;

    for line in actions_text.lines() {
        let trimmed = line.trim();
        let title = if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            rest.trim()
        } else if let Some(rest) = trimmed.strip_prefix("- ") {
            rest.trim()
        } else {
            continue;
        };

        if title.is_empty() {
            continue;
        }

        let action = crate::db::DbAction {
            id: format!(
                "ai-{}-{}",
                source_filename.trim_end_matches(".md"),
                count
            ),
            title: title.to_string(),
            priority: "P2".to_string(),
            status: "pending".to_string(),
            created_at: now.clone(),
            due_date: None,
            completed_at: None,
            account_id: None,
            project_id: None,
            source_type: Some("ai-inbox".to_string()),
            source_id: None,
            source_label: Some(source_filename.to_string()),
            context: None,
            waiting_on: None,
            updated_at: now.clone(),
        };

        if let Err(e) = db.upsert_action_if_not_completed(&action) {
            log::warn!("Failed to insert AI-extracted action: {}", e);
        } else {
            count += 1;
        }
    }

    if count > 0 {
        log::info!(
            "AI extracted {} actions from '{}'",
            count,
            source_filename
        );
    }
}

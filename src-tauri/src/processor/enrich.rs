//! AI enrichment for inbox files via Claude Code.
//!
//! Files that can't be classified by filename patterns get sent to Claude Code
//! for AI-powered classification and action extraction.

use std::path::Path;

use chrono::Utc;

use crate::db::{ActionDb, DbProcessingLog};
use crate::pty::{ModelTier, PtyManager};
use crate::types::AiModelConfig;
use crate::util::wrap_user_data;

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
    state: Option<&crate::state::AppState>,
    profile: &str,
    user_ctx: Option<&crate::types::UserContext>,
    ai_config: Option<&AiModelConfig>,
    entity_tracker_path: Option<&str>,
) -> EnrichResult {
    // I60: validate path stays within inbox
    let file_path = match crate::util::validate_inbox_path(workspace, filename) {
        Ok(p) => p,
        Err(e) => return EnrichResult::Error { message: e },
    };

    // Detect format and extract text
    let format = super::extract::detect_format(&file_path);
    let content = match super::extract::extract_text(&file_path) {
        Ok(c) => c,
        Err(e) => {
            return EnrichResult::Error {
                message: format!("Failed to extract text: {}", e),
            }
        }
    };
    let is_non_md = !matches!(format, super::extract::SupportedFormat::Markdown);

    // Build the prompt for Claude
    let prompt = build_enrichment_prompt(filename, &content, user_ctx, None);

    // Invoke Claude Code via PTY (Mechanical tier — I174)
    let default_config = AiModelConfig::default();
    let pty = PtyManager::for_tier(ModelTier::Mechanical, ai_config.unwrap_or(&default_config))
        .with_timeout(AI_TIMEOUT_SECS);
    let output = match pty.spawn_claude(workspace, &prompt) {
        Ok(o) => o.stdout,
        Err(e) => {
            log::error!("AI enrichment failed for '{}': {}", filename, e);
            return EnrichResult::Error {
                message: format!("Claude Code failed: {}", e),
            };
        }
    };

    // Audit trail (I297)
    let _ = crate::audit::write_audit_entry(workspace, "inbox_file", filename, &output);

    // Parse Claude's response
    let parsed = parse_enrichment_response(&output);

    // Extract actions if any
    if let Some(ref actions_text) = parsed.actions_text {
        if let Some(state) = state {
            if let Ok(db_guard) = state.db.lock() {
                if let Some(db) = db_guard.as_ref() {
                    extract_actions_from_ai(actions_text, filename, db, parsed.account.as_deref());
                }
            }
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

    let destination =
        resolve_destination(&classification, workspace, filename, entity_tracker_path);

    // Capture fields before the match to avoid borrow-after-move issues
    let summary = parsed.summary.clone();
    let file_type = parsed.file_type.clone();
    let account = parsed.account.clone();
    let wins = parsed.wins.clone();
    let risks = parsed.risks.clone();

    let result = match destination {
        Some(dest) => match move_file(&file_path, &dest) {
            Ok(route_result) => {
                // Write enriched companion .md for non-markdown files
                if is_non_md {
                    let companion_path =
                        super::extract::companion_md_path(&route_result.destination);
                    let companion_content = super::extract::build_enriched_companion_md(
                        filename,
                        format,
                        &content,
                        &file_type,
                        account.as_deref(),
                        &summary,
                    );
                    if let Err(e) =
                        crate::util::atomic_write_str(&companion_path, &companion_content)
                    {
                        log::warn!("Failed to write companion .md for '{}': {}", filename, e);
                    }
                }
                EnrichResult::Routed {
                    classification: file_type.clone(),
                    destination: route_result.destination.display().to_string(),
                    summary: summary.clone(),
                }
            }
            Err(e) => EnrichResult::Error {
                message: format!("Failed to route: {}", e),
            },
        },
        None => {
            // Even if unknown, archive it with AI summary
            let date = Utc::now().format("%Y-%m-%d").to_string();
            let archive_dest = workspace.join("_archive").join(&date).join(filename);
            match move_file(&file_path, &archive_dest) {
                Ok(route_result) => {
                    // Write enriched companion .md for non-markdown files
                    if is_non_md {
                        let companion_path =
                            super::extract::companion_md_path(&route_result.destination);
                        let companion_content = super::extract::build_enriched_companion_md(
                            filename,
                            format,
                            &content,
                            &file_type,
                            account.as_deref(),
                            &summary,
                        );
                        if let Err(e) =
                            crate::util::atomic_write_str(&companion_path, &companion_content)
                        {
                            log::warn!("Failed to write companion .md for '{}': {}", filename, e);
                        }
                    }
                    EnrichResult::Archived {
                        summary: summary.clone(),
                        destination: route_result.destination.display().to_string(),
                    }
                }
                Err(e) => EnrichResult::Error {
                    message: format!("Failed to archive: {}", e),
                },
            }
        }
    };

    // Run post-enrichment hooks
    if let Some(state) = state {
        if let Ok(db_guard) = state.db.lock() {
            if let Some(db) = db_guard.as_ref() {
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
                    wins: wins.clone(),
                    risks: risks.clone(),
                    entity_type: None,
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
        }
    }

    // Log to database
    if let Some(state) = state {
        if let Ok(db_guard) = state.db.lock() {
            if let Some(db) = db_guard.as_ref() {
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
        }
    }

    result
}

/// Build the prompt for Claude Code enrichment.
///
/// Detects transcript-like content and uses a richer prompt with DISCUSSION
/// section for transcript summarization (I31).
fn build_enrichment_prompt(
    filename: &str,
    content: &str,
    user_ctx: Option<&crate::types::UserContext>,
    vocabulary: Option<&crate::presets::schema::PresetVocabulary>,
) -> String {
    // Truncate very long content to fit in a reasonable prompt.
    // Must find a valid UTF-8 char boundary — slicing at an arbitrary byte panics.
    let truncated = if content.len() > 30_000 {
        let mut end = 30_000;
        while end > 0 && !content.is_char_boundary(end) {
            end -= 1;
        }
        &content[..end]
    } else {
        content
    };

    let is_transcript = detect_transcript(filename, truncated);

    let summary_instruction = if is_transcript {
        "SUMMARY: <2-3 sentence executive summary of the discussion and key outcomes>\nDISCUSSION:\n- <key topic 1: what was discussed and decided>\n- <key topic 2: what was discussed and decided>\nEND_DISCUSSION"
    } else {
        "SUMMARY: <one-line summary>"
    };

    let user_fragment = user_ctx
        .map(|ctx| ctx.prompt_fragment())
        .unwrap_or_default();

    // I313: Use vocabulary-driven entity noun when available
    let entity_noun = vocabulary
        .map(|v| v.entity_noun.as_str())
        .unwrap_or("customer/account");
    let success_verb = vocabulary
        .map(|v| v.success_verb.as_str())
        .unwrap_or("customer win");

    format!(
        r#"{user_fragment}Analyze this inbox file and respond in exactly this format:

FILE_TYPE: <one of: meeting_notes, account_update, action_items, meeting_context, general>
ACCOUNT: <account name if relevant, or NONE>
MEETING: <meeting name if relevant, or NONE>
{summary_instruction}
ACTIONS:
- <concise action title> P1/P2/P3 @Account due: YYYY-MM-DD #"context sentence"
END_ACTIONS
WINS:
- <{success_verb}, positive outcome, or success signal>
END_WINS
RISKS:
- <risk, concern, or potential issue>
END_RISKS
DECISIONS:
- <key decision made during discussion>
END_DECISIONS

Rules for actions:
- TITLE MUST be concise and imperative: verb + object, max 10 words. Not a sentence — a task.
  - Good: "Follow up on renewal pricing"
  - Bad: "Follow up with the client regarding the renewal discussion they mentioned"
- Include priority when urgency is inferable (P1=urgent, P2=normal, P3=low)
- Include @AccountName when action relates to a specific {entity_noun}
- Include due: YYYY-MM-DD when a deadline is mentioned or implied
- Include #"context" with a short sentence explaining WHY this matters. Use quotes around multi-word context.
  - Good: #"Renewal decision pending CFO approval"
  - Bad: #billing
- Use "waiting" or "blocked" in the title if action depends on someone else
- If no metadata can be inferred, just write the action text plainly
- Example: Follow up on renewal P1 @Acme due: 2026-03-15 #"CFO needs pricing comparison before Q2"

Rules for wins/risks:
- Only include if the file relates to a {entity_noun}
- Wins: successful launches, expanded usage, positive feedback, renewals, upsells
- Risks: churn signals, budget cuts, champion leaving, low adoption, complaints
- Keep each item to one concise sentence
- If none are apparent, leave the section empty (just the markers)

Rules for decisions:
- Only include if clear decisions or commitments were made
- Each item should state what was decided and who is responsible (if clear)
- If no decisions are apparent, leave the section empty (just the markers)

Filename: {filename}
Content:
{truncated}
"#,
        filename = wrap_user_data(filename),
        truncated = wrap_user_data(truncated),
    )
}

/// Detect whether content looks like a meeting transcript.
///
/// Uses filename + content heuristics: speaker labels ("Speaker 1:", "John:"),
/// timestamp patterns ("[00:12:34]"), and the word "transcript" in filename.
fn detect_transcript(filename: &str, content: &str) -> bool {
    let filename_lower = filename.to_lowercase();
    if filename_lower.contains("transcript") || filename_lower.contains("recording") {
        return true;
    }

    // Check a sample of lines for speaker-label or timestamp patterns
    let sample = &content[..content.len().min(3000)];
    let mut speaker_lines = 0;
    let mut timestamp_lines = 0;
    let mut total_lines = 0;

    for line in sample.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        total_lines += 1;

        // Speaker label: "Name:" at start of line. A speaker name is typically
        // 1-3 words and under 25 chars. Ignore markdown headers (#) and
        // key-value metadata (words like "status", "arr", "health" before colon).
        if !trimmed.starts_with('#') {
            if let Some(i) = trimmed.find(':') {
                let prefix = &trimmed[..i];
                let word_count = prefix.split_whitespace().count();
                if i > 0
                    && i < 25
                    && word_count <= 3
                    && prefix
                        .chars()
                        .all(|c| c.is_alphabetic() || c == ' ' || c == '.')
                    && prefix.chars().next().unwrap_or(' ').is_uppercase()
                {
                    speaker_lines += 1;
                }
            }
        }

        // Timestamp: [HH:MM:SS] or [MM:SS] at start
        if trimmed.starts_with('[')
            && trimmed
                .find(']')
                .map(|i| i < 12 && trimmed[1..i].contains(':'))
                .unwrap_or(false)
        {
            timestamp_lines += 1;
        }
    }

    // Transcripts are substantial — require minimum 10 non-empty lines
    // to avoid false positives on short metadata-heavy documents.
    if total_lines < 10 {
        return false;
    }

    // If >40% of lines look like speaker labels or >20% have timestamps
    let speaker_ratio = speaker_lines as f64 / total_lines as f64;
    let timestamp_ratio = timestamp_lines as f64 / total_lines as f64;

    speaker_ratio > 0.4 || timestamp_ratio > 0.2
}

/// Parsed response from Claude Code enrichment.
pub struct ParsedEnrichment {
    pub file_type: String,
    pub account: Option<String>,
    pub meeting_name: Option<String>,
    pub summary: String,
    /// Discussion highlights from transcript summarization (I31).
    pub discussion: Vec<String>,
    /// Strategic TAM-perspective analysis from transcript prompt.
    pub analysis: Option<String>,
    pub actions_text: Option<String>,
    pub wins: Vec<String>,
    pub risks: Vec<String>,
    pub decisions: Vec<String>,
}

/// Parse Claude's enrichment response.
pub fn parse_enrichment_response(output: &str) -> ParsedEnrichment {
    let mut file_type = "general".to_string();
    let mut account = None;
    let mut meeting_name = None;
    let mut summary = String::new();
    let mut discussion = Vec::new();
    let mut analysis = None;
    let mut actions_text = None;
    let mut in_actions = false;
    let mut actions_buf = String::new();
    let mut wins = Vec::new();
    let mut risks = Vec::new();
    let mut decisions = Vec::new();
    let mut in_discussion = false;
    let mut in_wins = false;
    let mut in_risks = false;
    let mut in_decisions = false;

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
        } else if let Some(rest) = line.strip_prefix("ANALYSIS:") {
            let val = rest.trim();
            if !val.is_empty() {
                analysis = Some(val.to_string());
            }
        } else if line == "DISCUSSION:" {
            in_discussion = true;
            in_actions = false;
            in_wins = false;
            in_risks = false;
            in_decisions = false;
        } else if line == "END_DISCUSSION" {
            in_discussion = false;
        } else if line == "ACTIONS:" {
            in_actions = true;
            in_discussion = false;
            in_wins = false;
            in_risks = false;
            in_decisions = false;
        } else if line == "END_ACTIONS" {
            in_actions = false;
            if !actions_buf.is_empty() {
                actions_text = Some(actions_buf.clone());
            }
        } else if line == "WINS:" {
            in_wins = true;
            in_actions = false;
            in_discussion = false;
            in_risks = false;
            in_decisions = false;
        } else if line == "END_WINS" {
            in_wins = false;
        } else if line == "RISKS:" {
            in_risks = true;
            in_actions = false;
            in_discussion = false;
            in_wins = false;
            in_decisions = false;
        } else if line == "END_RISKS" {
            in_risks = false;
        } else if line == "DECISIONS:" {
            in_decisions = true;
            in_actions = false;
            in_discussion = false;
            in_wins = false;
            in_risks = false;
        } else if line == "END_DECISIONS" {
            in_decisions = false;
        } else if in_discussion && line.starts_with("- ") {
            discussion.push(line.strip_prefix("- ").unwrap().to_string());
        } else if in_actions && line.starts_with("- ") {
            if !actions_buf.is_empty() {
                actions_buf.push('\n');
            }
            actions_buf.push_str(line);
        } else if in_wins && line.starts_with("- ") {
            wins.push(line.strip_prefix("- ").unwrap().to_string());
        } else if in_risks && line.starts_with("- ") {
            risks.push(line.strip_prefix("- ").unwrap().to_string());
        } else if in_decisions && line.starts_with("- ") {
            decisions.push(line.strip_prefix("- ").unwrap().to_string());
        }
    }

    // If Claude emitted ACTIONS: but never END_ACTIONS, capture buffered actions
    if in_actions && !actions_buf.is_empty() && actions_text.is_none() {
        actions_text = Some(actions_buf);
    }

    // Cap array sizes to prevent oversized output (I296)
    discussion.truncate(20);
    wins.truncate(10);
    risks.truncate(20);
    decisions.truncate(20);

    ParsedEnrichment {
        file_type,
        account,
        meeting_name,
        summary,
        discussion,
        analysis,
        actions_text,
        wins,
        risks,
        decisions,
    }
}

/// Extract actions from AI-generated action text and sync to SQLite.
///
/// Parses inline metadata tokens from each action line (priority, @account, etc.).
pub fn extract_actions_from_ai(
    actions_text: &str,
    source_filename: &str,
    db: &ActionDb,
    account_fallback: Option<&str>,
) {
    use super::metadata;

    let now = Utc::now().to_rfc3339();
    let mut count = 0;
    let max_actions = 50; // I296: cap parsed actions

    for line in actions_text.lines() {
        let trimmed = line.trim();
        let raw_title = if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            rest.trim()
        } else if let Some(rest) = trimmed.strip_prefix("- ") {
            rest.trim()
        } else {
            continue;
        };

        if raw_title.is_empty() {
            continue;
        }

        let meta = metadata::parse_action_metadata(raw_title);

        let status = if meta.is_waiting {
            "waiting".to_string()
        } else {
            "pending".to_string()
        };

        let account_id = meta
            .account
            .clone()
            .or_else(|| account_fallback.map(String::from));

        let action = crate::db::DbAction {
            id: format!("ai-{}-{}", source_filename.trim_end_matches(".md"), count),
            title: meta.clean_title,
            priority: meta.priority.unwrap_or_else(|| "P2".to_string()),
            status,
            created_at: now.clone(),
            due_date: meta.due_date,
            completed_at: None,
            account_id,
            project_id: None,
            source_type: Some("ai-inbox".to_string()),
            source_id: Some(raw_title.to_string()),
            source_label: Some(source_filename.to_string()),
            context: meta.context,
            waiting_on: if meta.is_waiting {
                Some("true".to_string())
            } else {
                None
            },
            updated_at: now.clone(),
            person_id: None,
        };

        if let Err(e) = db.upsert_action_if_not_completed(&action) {
            log::warn!("Failed to insert AI-extracted action: {}", e);
        } else {
            count += 1;
            if count >= max_actions {
                log::info!("extract_actions_from_ai: hit max {} actions, stopping", max_actions);
                break;
            }
        }
    }

    if count > 0 {
        log::info!("AI extracted {} actions from '{}'", count, source_filename);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_enrichment_response_with_wins_and_risks() {
        let output = "\
FILE_TYPE: account_update
ACCOUNT: Acme Corp
MEETING: NONE
SUMMARY: Quarterly review notes
ACTIONS:
- P2 @Acme Follow up on renewal
END_ACTIONS
WINS:
- Expanded deployment to 3 new teams
- NPS score increased to 9
END_WINS
RISKS:
- Budget freeze announced for Q2
END_RISKS";

        let parsed = parse_enrichment_response(output);

        assert_eq!(parsed.file_type, "account_update");
        assert_eq!(parsed.account, Some("Acme Corp".to_string()));
        assert!(parsed.actions_text.is_some());
        assert_eq!(parsed.wins.len(), 2);
        assert_eq!(parsed.wins[0], "Expanded deployment to 3 new teams");
        assert_eq!(parsed.wins[1], "NPS score increased to 9");
        assert_eq!(parsed.risks.len(), 1);
        assert_eq!(parsed.risks[0], "Budget freeze announced for Q2");
    }

    #[test]
    fn test_parse_enrichment_response_empty_wins_risks() {
        let output = "\
FILE_TYPE: general
ACCOUNT: NONE
MEETING: NONE
SUMMARY: Random document
ACTIONS:
END_ACTIONS
WINS:
END_WINS
RISKS:
END_RISKS";

        let parsed = parse_enrichment_response(output);

        assert_eq!(parsed.file_type, "general");
        assert!(parsed.wins.is_empty());
        assert!(parsed.risks.is_empty());
        assert!(parsed.actions_text.is_none());
    }

    #[test]
    fn test_parse_enrichment_response_no_wins_risks_sections() {
        // Backwards compatibility: older responses without WINS/RISKS
        let output = "\
FILE_TYPE: meeting_notes
ACCOUNT: Beta Inc
MEETING: Weekly Sync
SUMMARY: Discussed roadmap
ACTIONS:
- Review proposal
END_ACTIONS";

        let parsed = parse_enrichment_response(output);

        assert_eq!(parsed.file_type, "meeting_notes");
        assert_eq!(parsed.account, Some("Beta Inc".to_string()));
        assert!(parsed.actions_text.is_some());
        assert!(parsed.wins.is_empty());
        assert!(parsed.risks.is_empty());
    }

    #[test]
    fn test_parse_enrichment_response_wins_without_end_marker() {
        // Claude might forget END_WINS but still emit RISKS:
        let output = "\
FILE_TYPE: account_update
ACCOUNT: Acme
MEETING: NONE
SUMMARY: Update
ACTIONS:
END_ACTIONS
WINS:
- Great adoption
RISKS:
- Champion leaving
END_RISKS";

        let parsed = parse_enrichment_response(output);

        // RISKS: marker should end wins section implicitly
        assert_eq!(parsed.wins.len(), 1);
        assert_eq!(parsed.wins[0], "Great adoption");
        assert_eq!(parsed.risks.len(), 1);
        assert_eq!(parsed.risks[0], "Champion leaving");
    }

    // =========================================================================
    // Transcript detection & discussion parsing tests (I31)
    // =========================================================================

    #[test]
    fn test_detect_transcript_by_filename() {
        assert!(detect_transcript(
            "acme-transcript-2026-01-15.md",
            "some content"
        ));
        assert!(detect_transcript(
            "Meeting_Recording_Notes.md",
            "some content"
        ));
        assert!(!detect_transcript("acme-update.md", "some content"));
    }

    #[test]
    fn test_detect_transcript_by_speaker_labels() {
        let content = "\
Alice: Hi everyone, thanks for joining.
Bob: Great, let's get started.
Alice: First item on the agenda is the Q1 review.
Bob: Numbers look good. Revenue up 15%.
Alice: Excellent. Next, let's discuss the hiring plan.
Bob: We have 3 open reqs.
Alice: Okay, let's prioritize engineering.
Bob: Agreed. I'll send the updated JDs.
Alice: Any other topics to cover today?
Bob: Just the offsite planning for March.
Alice: Right, let's schedule a follow-up for that.
Bob: Sounds good. I'll send a calendar invite.";

        // Not a transcript filename, but content has >40% speaker lines
        assert!(detect_transcript("meeting-notes.md", content));
    }

    #[test]
    fn test_detect_transcript_by_timestamps() {
        let content = "\
[00:00:00] Welcome everyone
[00:00:15] Let's start with the agenda
[00:01:30] First topic: Q1 results
[00:05:22] Revenue grew 15%
[00:08:45] Moving on to hiring
[00:12:10] We need three engineers
[00:15:00] Budget discussion
[00:18:30] Wrapping up action items
[00:20:00] Next meeting scheduled
Some non-timestamped line here
Another regular line
Final notes from the call";

        assert!(detect_transcript("notes.md", content));
    }

    #[test]
    fn test_detect_transcript_negative() {
        let content = "\
# Acme Corp Account Update

Current status: Green
ARR: $120,000
Next renewal: March 2026

## Recent Activity
- Deployed v3.2 to production
- Trained 15 new users";

        assert!(!detect_transcript("acme-update.md", content));
    }

    #[test]
    fn test_transcript_prompt_includes_discussion() {
        let prompt = build_enrichment_prompt(
            "acme-transcript.md",
            "Alice: Hi\nBob: Hello\nAlice: Let's discuss the project.",
            None,
            None,
        );
        assert!(prompt.contains("DISCUSSION:"));
        assert!(prompt.contains("END_DISCUSSION"));
        assert!(prompt.contains("2-3 sentence executive summary"));
    }

    #[test]
    fn test_non_transcript_prompt_no_discussion() {
        let prompt = build_enrichment_prompt("acme-update.md", "# Account Update\nAll good.", None, None);
        assert!(!prompt.contains("DISCUSSION:"));
        assert!(!prompt.contains("END_DISCUSSION"));
        assert!(prompt.contains("one-line summary"));
    }

    #[test]
    fn test_parse_discussion_block() {
        let output = "\
FILE_TYPE: meeting_notes
ACCOUNT: Acme Corp
MEETING: Weekly Sync
SUMMARY: Discussed Q1 results and hiring plan. Revenue up 15%. Agreed to prioritize engineering hires.
DISCUSSION:
- Q1 results: Revenue grew 15% YoY, exceeding target by 3%
- Hiring plan: 3 open reqs, prioritizing engineering roles
- Product roadmap: v4.0 launch planned for March
END_DISCUSSION
ACTIONS:
- P2 @Acme Send updated JDs to recruiting
END_ACTIONS
WINS:
- Revenue exceeded Q1 target
END_WINS
RISKS:
END_RISKS
DECISIONS:
- Prioritize engineering hires over sales for Q2
END_DECISIONS";

        let parsed = parse_enrichment_response(output);

        assert_eq!(parsed.discussion.len(), 3);
        assert!(parsed.discussion[0].contains("Revenue grew 15%"));
        assert!(parsed.discussion[1].contains("Hiring plan"));
        assert!(parsed.discussion[2].contains("v4.0 launch"));
        assert!(parsed.summary.contains("Q1 results"));
        assert_eq!(parsed.decisions.len(), 1);
    }

    #[test]
    fn test_parse_no_discussion_backwards_compat() {
        // Older responses without DISCUSSION block should still parse fine
        let output = "\
FILE_TYPE: general
ACCOUNT: NONE
MEETING: NONE
SUMMARY: A simple document
ACTIONS:
END_ACTIONS
WINS:
END_WINS
RISKS:
END_RISKS";

        let parsed = parse_enrichment_response(output);
        assert!(parsed.discussion.is_empty());
        assert_eq!(parsed.summary, "A simple document");
    }
}

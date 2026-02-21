//! Meeting-scoped transcript processing (I44 / ADR-0044).
//!
//! Processes a transcript file with full meeting context, extracting outcomes
//! (summary, wins, risks, decisions, actions) and routing the file to its
//! proper workspace location.

use std::path::Path;

use chrono::Utc;

use crate::db::{ActionDb, DbProcessingLog};
use crate::pty::{ModelTier, PtyManager};
use crate::types::AiModelConfig;
use crate::types::{CalendarEvent, CapturedAction, TranscriptResult};
use crate::util::wrap_user_data;

use super::enrich::parse_enrichment_response;
use super::hooks;

/// Timeout for transcript AI processing (3 minutes — larger transcripts need more time)
const TRANSCRIPT_AI_TIMEOUT_SECS: u64 = 180;

/// Maximum transcript content sent to AI (covers ~75 min calls).
const TRANSCRIPT_MAX_CHARS: usize = 60_000;

/// Head portion kept for tail-biased truncation (attendee context, meeting opening).
const TRANSCRIPT_HEAD_KEEP: usize = 3_000;

/// Process a transcript file with meeting context.
///
/// 1. Read the source file
/// 2. Route to account dir or archive with YAML frontmatter
/// 3. Send to Claude for extraction with meeting context
/// 4. Store outcomes (wins/risks/decisions as captures, actions to SQLite)
/// 5. Run post-enrichment hooks
pub fn process_transcript(
    workspace: &Path,
    file_path: &str,
    meeting: &CalendarEvent,
    db: Option<&ActionDb>,
    profile: &str,
    ai_config: Option<&AiModelConfig>,
) -> TranscriptResult {
    let source = Path::new(file_path);

    // 1. Read the source file
    let content = match std::fs::read_to_string(source) {
        Ok(c) => c,
        Err(e) => {
            return TranscriptResult {
                status: "error".to_string(),
                message: Some(format!("Failed to read transcript: {}", e)),
                ..TranscriptResult::default()
            };
        }
    };

    // 2. Generate destination path and copy with frontmatter
    let date = meeting.end.format("%Y-%m-%d").to_string();
    let slug = slugify(&meeting.title);
    let dest_filename = format!("{}-{}-transcript.md", date, slug);

    let destination = if let Some(ref account) = meeting.account {
        let account_dir = sanitize_account_dir(account);
        workspace
            .join("Accounts")
            .join(&account_dir)
            .join("Call-Transcripts")
            .join(&dest_filename)
    } else {
        workspace.join("_archive").join(&date).join(&dest_filename)
    };

    // Build frontmatter
    let frontmatter = build_frontmatter(meeting, &date);
    let content_with_frontmatter = format!("{}\n{}", frontmatter, content);

    // Create dirs and write
    if let Some(parent) = destination.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return TranscriptResult {
                status: "error".to_string(),
                message: Some(format!("Failed to create directory: {}", e)),
                ..TranscriptResult::default()
            };
        }
    }

    if let Err(e) = std::fs::write(&destination, &content_with_frontmatter) {
        return TranscriptResult {
            status: "error".to_string(),
            message: Some(format!("Failed to write transcript: {}", e)),
            ..TranscriptResult::default()
        };
    }

    log::info!(
        "Transcript for '{}' written to '{}'",
        meeting.title,
        destination.display()
    );

    // 3. Build prompt and invoke Claude
    let prompt = build_transcript_prompt(meeting, &content);
    let default_config = AiModelConfig::default();
    let pty = PtyManager::for_tier(ModelTier::Extraction, ai_config.unwrap_or(&default_config))
        .with_timeout(TRANSCRIPT_AI_TIMEOUT_SECS);
    let output = match pty.spawn_claude(workspace, &prompt) {
        Ok(o) => o.stdout,
        Err(e) => {
            log::error!(
                "AI transcript processing failed for '{}': {}",
                meeting.title,
                e
            );
            // Return partial success — file was routed, but no AI extraction
            return TranscriptResult {
                status: "success".to_string(),
                summary: None,
                destination: Some(destination.display().to_string()),
                message: Some(format!("Transcript saved but AI extraction failed: {}", e)),
                ..TranscriptResult::default()
            };
        }
    };

    // Audit trail (I297)
    let _ = crate::audit::write_audit_entry(workspace, "transcript", &meeting.id, &output);

    // Debug: log raw Claude output for transcript processing
    log::info!(
        "Transcript AI output for '{}' ({} bytes): {}",
        meeting.title,
        output.len(),
        if output.len() > 500 {
            &output[..500]
        } else {
            &output
        }
    );

    // 4. Parse response
    let parsed = parse_enrichment_response(&output);
    let summary = parsed.summary.clone();
    let wins = parsed.wins.clone();
    let risks = parsed.risks.clone();
    let decisions = parsed.decisions.clone();
    let discussion = parsed.discussion.clone();
    let analysis = parsed.analysis.clone();

    // Extract actions to SQLite
    let mut extracted_actions = Vec::new();
    if let Some(ref actions_text) = parsed.actions_text {
        if let Some(db) = db {
            extract_transcript_actions(
                actions_text,
                &meeting.id,
                &meeting.title,
                db,
                meeting.account.as_deref(),
            );
        }
        // Parse for return value
        for line in actions_text.lines() {
            let trimmed = line.trim();
            let raw = if let Some(rest) = trimmed.strip_prefix("- ") {
                rest.trim()
            } else {
                continue;
            };
            if !raw.is_empty() {
                let meta = super::metadata::parse_action_metadata(raw);
                extracted_actions.push(CapturedAction {
                    title: meta.clean_title,
                    owner: meta.account,
                    due_date: meta.due_date,
                });
            }
        }
    }

    // Store captures (wins, risks, decisions)
    if let Some(db) = db {
        for win in &wins {
            let _ = db.insert_capture(
                &meeting.id,
                &meeting.title,
                meeting.account.as_deref(),
                "win",
                win,
            );
        }
        for risk in &risks {
            let _ = db.insert_capture(
                &meeting.id,
                &meeting.title,
                meeting.account.as_deref(),
                "risk",
                risk,
            );
        }
        for decision in &decisions {
            let _ = db.insert_capture(
                &meeting.id,
                &meeting.title,
                meeting.account.as_deref(),
                "decision",
                decision,
            );
        }
    }

    // 4b. Emit transcript signals for entity intelligence (I307 learning)
    if let Some(db) = db {
        let entity_type = meeting
            .linked_entities
            .as_ref()
            .and_then(|e| e.first())
            .map(|e| e.entity_type.as_str())
            .unwrap_or("account");
        let entity_id = meeting
            .linked_entities
            .as_ref()
            .and_then(|e| e.first())
            .map(|e| e.id.as_str())
            .or(meeting.account.as_deref())
            .unwrap_or(&meeting.id);

        let capture_count = wins.len() + risks.len() + decisions.len();
        if capture_count > 0 {
            let _ = crate::signals::bus::emit_signal(
                db,
                entity_type,
                entity_id,
                "transcript_outcomes",
                "transcript",
                Some(&format!(
                    "{{\"meeting_id\":\"{}\",\"wins\":{},\"risks\":{},\"decisions\":{}}}",
                    meeting.id,
                    wins.len(),
                    risks.len(),
                    decisions.len()
                )),
                0.75,
            );
        }
    }

    // 5. Run post-enrichment hooks
    if let Some(db) = db {
        let ctx = hooks::EnrichmentContext {
            workspace: workspace.to_path_buf(),
            filename: dest_filename.clone(),
            classification: "transcript".to_string(),
            account: meeting.account.clone(),
            summary: summary.clone(),
            actions: Vec::new(),
            destination_path: Some(destination.display().to_string()),
            profile: profile.to_string(),
            wins: wins.clone(),
            risks: risks.clone(),
            entity_type: None,
        };
        let hook_results = hooks::run_post_enrichment_hooks(&ctx, db);
        for hr in &hook_results {
            log::info!(
                "Transcript hook '{}': {} — {}",
                hr.hook_name,
                if hr.success { "OK" } else { "FAILED" },
                hr.message.as_deref().unwrap_or("")
            );
        }
    }

    // 6. Log to processing_log
    if let Some(db) = db {
        let log_entry = DbProcessingLog {
            id: uuid::Uuid::new_v4().to_string(),
            filename: dest_filename,
            source_path: file_path.to_string(),
            destination_path: Some(destination.display().to_string()),
            classification: "transcript".to_string(),
            status: "completed".to_string(),
            processed_at: Some(Utc::now().to_rfc3339()),
            error_message: None,
            created_at: Utc::now().to_rfc3339(),
        };
        if let Err(e) = db.insert_processing_log(&log_entry) {
            log::warn!("Failed to log transcript processing: {}", e);
        }
    }

    // 7. Append wins to impact log
    if !wins.is_empty() {
        append_to_impact_log(workspace, meeting, &wins);
    }

    // If summary is empty after parsing, include truncated raw output for debugging
    let debug_message = if summary.is_empty() {
        let preview = if output.len() > 200 {
            format!("{}...", &output[..200])
        } else {
            output.clone()
        };
        Some(format!("Empty parse result. Raw output: {}", preview))
    } else {
        None
    };

    TranscriptResult {
        status: "success".to_string(),
        summary: Some(summary),
        destination: Some(destination.display().to_string()),
        wins,
        risks,
        decisions,
        actions: extracted_actions,
        discussion,
        analysis,
        message: debug_message,
    }
}

/// Extract actions from AI output, using meeting ID as source_id for meeting-scoped queries.
fn extract_transcript_actions(
    actions_text: &str,
    meeting_id: &str,
    meeting_title: &str,
    db: &ActionDb,
    account_fallback: Option<&str>,
) {
    let now = Utc::now().to_rfc3339();
    let mut count = 0;

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

        let meta = super::metadata::parse_action_metadata(raw_title);

        let status = if meta.is_waiting {
            "waiting".to_string()
        } else {
            "proposed".to_string()
        };

        // Resolve @Tag to a real account ID; fall back to meeting-level account.
        // If the tag doesn't match any account, use None to avoid FK violations.
        let account_id = meta
            .account
            .as_deref()
            .and_then(|tag| {
                db.get_account_by_name(tag)
                    .ok()
                    .flatten()
                    .map(|a| a.id)
            })
            .or_else(|| account_fallback.map(String::from));

        let action = crate::db::DbAction {
            id: format!("transcript-{}-{}", meeting_id, count),
            title: meta.clean_title,
            priority: meta.priority.unwrap_or_else(|| "P2".to_string()),
            status,
            created_at: now.clone(),
            due_date: meta.due_date,
            completed_at: None,
            account_id,
            project_id: None,
            source_type: Some("transcript".to_string()),
            source_id: Some(meeting_id.to_string()),
            source_label: Some(meeting_title.to_string()),
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
        };

        if let Err(e) = db.upsert_action_if_not_completed(&action) {
            log::warn!("Failed to insert transcript action: {}", e);
        } else {
            count += 1;
        }
    }

    if count > 0 {
        log::info!(
            "Extracted {} actions from transcript for '{}'",
            count,
            meeting_title
        );
    }
}

/// Truncate transcript content with a tail-biased strategy.
///
/// For very long transcripts (>60K chars), keeps the first 3K chars (attendee
/// context, meeting opening) plus the last 57K chars (substantive discussion).
/// This prevents social preamble from dominating the AI's analysis window.
fn truncate_transcript(content: &str) -> String {
    if content.len() <= TRANSCRIPT_MAX_CHARS {
        return content.to_string();
    }

    // Find valid UTF-8 boundaries for the head slice
    let mut head_end = TRANSCRIPT_HEAD_KEEP;
    while head_end > 0 && !content.is_char_boundary(head_end) {
        head_end -= 1;
    }

    // Find valid UTF-8 boundary for the tail slice
    let tail_len = TRANSCRIPT_MAX_CHARS - head_end - 30; // 30 chars for the splice marker
    let mut tail_start = content.len() - tail_len;
    while tail_start < content.len() && !content.is_char_boundary(tail_start) {
        tail_start += 1;
    }

    format!(
        "{}\n\n[... truncated {} chars ...]\n\n{}",
        &content[..head_end],
        tail_start - head_end,
        &content[tail_start..],
    )
}

/// Build the meeting-contextualized prompt for transcript analysis.
pub fn build_transcript_prompt(meeting: &CalendarEvent, content: &str) -> String {
    let truncated = truncate_transcript(content);

    let meeting_type = format!("{:?}", meeting.meeting_type).to_lowercase();
    let title = if meeting.title.trim().is_empty() {
        "Untitled meeting"
    } else {
        &meeting.title
    };
    let account_line = match meeting.account.as_deref() {
        Some(a) if !a.trim().is_empty() => format!("Account: {}\n", wrap_user_data(a)),
        _ => String::new(),
    };
    let date = meeting.end.format("%Y-%m-%d").to_string();

    format!(
        r#"You are analyzing a transcript from a {meeting_type} meeting.

Meeting: "{title}"
{account_line}Date: {date}

IMPORTANT: Focus on the substantive business discussion. Skip social chitchat,
internal team banter, and small talk that typically occurs at the start of calls.
Prioritize customer-facing content — what the customer said, asked, or committed to.

Respond in exactly this format:

SUMMARY: <2-3 sentence executive summary focused on business outcomes and decisions, not a chronological recap. Who met, what was substantively discussed, key outcomes.>

DISCUSSION:
- <Topic 1>: <What was discussed, decided, or committed to. Include direct customer quotes where they reveal priorities, concerns, or sentiment.>
- <Topic 2>: ...
END_DISCUSSION

ANALYSIS: <1-2 sentences of strategic TAM-perspective insight — connect what happened in this meeting to account health, expansion potential, or renewal risk.>

ACTIONS:
- <concise action title> P1/P2/P3 @Account due: YYYY-MM-DD #"context sentence"
END_ACTIONS
WINS:
- <customer win, positive outcome, expansion signal>
END_WINS
RISKS:
- <churn signal, concern, blocker>
END_RISKS
DECISIONS:
- <explicit decision made, who decided, any conditions>
END_DECISIONS

Rules for actions:
- TITLE MUST be concise and imperative: verb + object, max 10 words. Not a sentence, not a description — a task.
  - Good: "Follow up on renewal pricing"
  - Bad: "Follow up with the client regarding the renewal discussion they mentioned during the quarterly business review"
- Include priority when urgency is inferable (P1=urgent, P2=normal, P3=low)
- Include @AccountName when action relates to a specific customer/account
- Include due: YYYY-MM-DD when a deadline is mentioned or implied
- Include #"context" with a short sentence explaining WHY this action matters or WHAT was discussed. Use quotes around multi-word context.
  - Good: #"Renewal decision pending CFO approval, budget freeze risk"
  - Bad: #billing
- Use "waiting" or "blocked" in the title if action depends on someone else
- If no metadata can be inferred, just write the action text plainly
- Example: Follow up on renewal P1 @Acme due: 2026-03-15 #"CFO needs pricing comparison before Q2 budget lock"

Rules for wins/risks:
- Wins: successful launches, expanded usage, positive feedback, renewals, upsells
- Risks: churn signals, budget cuts, champion leaving, low adoption, complaints
- Keep each item to one concise sentence
- If none are apparent, leave the section empty (just the markers)

Rules for decisions:
- Capture explicit decisions ("we decided to...", "agreed that...", "going with...")
- Include the decision owner or group if identifiable
- Note any conditions or caveats attached to the decision
- If no decisions were made, leave the section empty

Transcript:
{content}
"#,
        title = wrap_user_data(title),
        account_line = account_line,
        meeting_type = meeting_type,
        date = date,
        content = wrap_user_data(&truncated),
    )
}

/// Build YAML frontmatter for the transcript file.
fn build_frontmatter(meeting: &CalendarEvent, date: &str) -> String {
    let meeting_type = format!("{:?}", meeting.meeting_type).to_lowercase();
    let account_line = meeting
        .account
        .as_deref()
        .map(|a| format!("account: \"{}\"\n", a))
        .unwrap_or_default();
    let now = Utc::now().to_rfc3339();

    format!(
        "---\nmeeting_id: \"{}\"\nmeeting_title: \"{}\"\n{}meeting_type: \"{}\"\nmeeting_date: \"{}\"\nprocessed_at: \"{}\"\nsource: transcript\n---\n",
        meeting.id,
        meeting.title.replace('"', "\\\""),
        account_line,
        meeting_type,
        date,
        now,
    )
}

/// Append wins to the impact log file.
///
/// Uses `OpenOptions::append` to avoid read-modify-write race conditions (I65).
fn append_to_impact_log(workspace: &Path, meeting: &CalendarEvent, wins: &[String]) {
    let impact_log = workspace.join("_today").join("90-impact-log.md");
    let mut content = String::new();

    // If file doesn't exist yet, prepend header
    if !impact_log.exists() {
        content.push_str("# Impact Log\n\n");
    }

    let label = meeting.account.as_deref().unwrap_or(&meeting.title);
    let now = Utc::now();

    for win in wins {
        content.push_str(&format!(
            "- **{}**: {} ({})\n",
            label,
            win,
            now.format("%H:%M")
        ));
    }

    // Atomic append — no read-modify-write race
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&impact_log)
        .and_then(|mut f| std::io::Write::write_all(&mut f, content.as_bytes()));
}

use crate::util::slugify;

/// Title-case and hyphenate account name for directory routing.
fn sanitize_account_dir(name: &str) -> String {
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

impl Default for TranscriptResult {
    fn default() -> Self {
        Self {
            status: "error".to_string(),
            summary: None,
            destination: None,
            wins: Vec::new(),
            risks: Vec::new(),
            decisions: Vec::new(),
            actions: Vec::new(),
            discussion: Vec::new(),
            analysis: None,
            message: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MeetingType;
    use chrono::Utc;
    use std::path::PathBuf;

    fn test_meeting() -> CalendarEvent {
        CalendarEvent {
            id: "test-meeting-001".to_string(),
            title: "Acme QBR".to_string(),
            start: Utc::now(),
            end: Utc::now(),
            meeting_type: MeetingType::Customer,
            account: Some("Acme Corp".to_string()),
            attendees: vec![],
            is_all_day: false,
            linked_entities: None,
        }
    }

    #[test]
    fn test_build_transcript_prompt() {
        let meeting = test_meeting();
        let prompt = build_transcript_prompt(&meeting, "Hello world transcript");

        assert!(prompt.contains("Acme QBR"));
        assert!(prompt.contains("Acme Corp"));
        assert!(prompt.contains("customer"));
        assert!(prompt.contains("Hello world transcript"));
        assert!(prompt.contains("DECISIONS:"));
        assert!(prompt.contains("DISCUSSION:"));
        assert!(prompt.contains("ANALYSIS:"));
        // Verify focus on substance over chitchat
        assert!(prompt.contains("Skip social chitchat"));
        // Verify concise title instructions
        assert!(prompt.contains("max 10 words"));
        // Verify quoted context format
        assert!(prompt.contains("#\""));
    }

    #[test]
    fn test_build_transcript_prompt_null_fields() {
        let mut meeting = test_meeting();
        meeting.account = None;
        meeting.title = "".to_string();
        let prompt = build_transcript_prompt(&meeting, "Some transcript");

        assert!(prompt.contains("Untitled meeting"));
        // Account line should be omitted entirely
        assert!(!prompt.contains("Account:"));
        assert!(prompt.contains("Some transcript"));
    }

    #[test]
    fn test_truncate_transcript_short() {
        let short = "Short transcript content";
        assert_eq!(truncate_transcript(short), short);
    }

    #[test]
    fn test_truncate_transcript_long() {
        // Create content longer than TRANSCRIPT_MAX_CHARS
        let long_content = "A".repeat(70_000);
        let result = truncate_transcript(&long_content);
        assert!(result.len() < long_content.len());
        assert!(result.contains("[... truncated"));
        // Head should be preserved
        assert!(result.starts_with("AAA"));
        // Tail should be preserved
        assert!(result.ends_with("AAA"));
    }

    #[test]
    fn test_frontmatter_generation() {
        let meeting = test_meeting();
        let fm = build_frontmatter(&meeting, "2026-02-07");

        assert!(fm.starts_with("---\n"));
        assert!(fm.ends_with("---\n"));
        assert!(fm.contains("meeting_id: \"test-meeting-001\""));
        assert!(fm.contains("meeting_title: \"Acme QBR\""));
        assert!(fm.contains("account: \"Acme Corp\""));
        assert!(fm.contains("source: transcript"));
    }

    #[test]
    fn test_frontmatter_without_account() {
        let mut meeting = test_meeting();
        meeting.account = None;
        let fm = build_frontmatter(&meeting, "2026-02-07");

        assert!(!fm.contains("account:"));
        assert!(fm.contains("meeting_id:"));
    }

    #[test]
    fn test_destination_with_account() {
        let meeting = test_meeting();
        let date = "2026-02-07";
        let slug = slugify(&meeting.title);
        let dest_filename = format!("{}-{}-transcript.md", date, slug);

        let account_dir = sanitize_account_dir("Acme Corp");
        let workspace = Path::new("/workspace");
        let destination = workspace
            .join("Accounts")
            .join(&account_dir)
            .join("Call-Transcripts")
            .join(&dest_filename);

        assert_eq!(
            destination,
            PathBuf::from(
                "/workspace/Accounts/Acme-Corp/Call-Transcripts/2026-02-07-acme-qbr-transcript.md"
            )
        );
    }

    #[test]
    fn test_destination_without_account() {
        let date = "2026-02-07";
        let slug = slugify("Internal Sync");
        let dest_filename = format!("{}-{}-transcript.md", date, slug);

        let workspace = Path::new("/workspace");
        let destination = workspace.join("_archive").join(date).join(&dest_filename);

        assert_eq!(
            destination,
            PathBuf::from("/workspace/_archive/2026-02-07/2026-02-07-internal-sync-transcript.md")
        );
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Acme QBR"), "acme-qbr");
        assert_eq!(
            slugify("Weekly Sync — Team Alpha"),
            "weekly-sync-team-alpha"
        );
        assert_eq!(slugify("simple"), "simple");
    }

    #[test]
    fn test_parse_decisions() {
        let output = "\
SUMMARY: Discussed Q1 roadmap
ACTIONS:
- P2 Follow up on renewal
END_ACTIONS
WINS:
- Expanded to 3 teams
END_WINS
RISKS:
- Budget freeze Q2
END_RISKS
DECISIONS:
- Expand pilot to EMEA starting Q2 — agreed by VP Sales
- Defer mobile launch to Q3
END_DECISIONS";

        let parsed = parse_enrichment_response(output);
        assert_eq!(parsed.decisions.len(), 2);
        assert_eq!(
            parsed.decisions[0],
            "Expand pilot to EMEA starting Q2 — agreed by VP Sales"
        );
        assert_eq!(parsed.decisions[1], "Defer mobile launch to Q3");
    }
}

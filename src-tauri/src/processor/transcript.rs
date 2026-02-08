//! Meeting-scoped transcript processing (I44 / ADR-0044).
//!
//! Processes a transcript file with full meeting context, extracting outcomes
//! (summary, wins, risks, decisions, actions) and routing the file to its
//! proper workspace location.

use std::path::Path;

use chrono::Utc;

use crate::db::{ActionDb, DbProcessingLog};
use crate::pty::PtyManager;
use crate::types::{CalendarEvent, CapturedAction, TranscriptResult};

use super::enrich::parse_enrichment_response;
use super::hooks;

/// Timeout for transcript AI processing (2 minutes)
const TRANSCRIPT_AI_TIMEOUT_SECS: u64 = 120;

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
            .join("01-Customer-Information")
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
    let pty = PtyManager::new().with_timeout(TRANSCRIPT_AI_TIMEOUT_SECS);
    let output = match pty.spawn_claude(workspace, &prompt) {
        Ok(o) => o.stdout,
        Err(e) => {
            log::error!("AI transcript processing failed for '{}': {}", meeting.title, e);
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

    // 4. Parse response
    let parsed = parse_enrichment_response(&output);
    let summary = parsed.summary.clone();
    let wins = parsed.wins.clone();
    let risks = parsed.risks.clone();
    let decisions = parsed.decisions.clone();

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

    TranscriptResult {
        status: "success".to_string(),
        summary: Some(summary),
        destination: Some(destination.display().to_string()),
        wins,
        risks,
        decisions,
        actions: extracted_actions,
        message: None,
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
            "pending".to_string()
        };

        let account_id = meta
            .account
            .clone()
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

/// Build the meeting-contextualized prompt for transcript analysis.
pub fn build_transcript_prompt(meeting: &CalendarEvent, content: &str) -> String {
    // Truncate very long transcripts
    let truncated = if content.len() > 15000 {
        let mut end = 15000;
        while end > 0 && !content.is_char_boundary(end) {
            end -= 1;
        }
        &content[..end]
    } else {
        content
    };

    let meeting_type = format!("{:?}", meeting.meeting_type).to_lowercase();
    let account = meeting.account.as_deref().unwrap_or("N/A");
    let date = meeting.end.format("%Y-%m-%d").to_string();

    format!(
        r#"You are analyzing a transcript from a specific meeting. Extract outcomes.

Meeting: "{title}"
Account: {account}
Type: {meeting_type}
Date: {date}

Respond in exactly this format:

SUMMARY: <2-3 sentence summary of the meeting discussion and outcomes>
ACTIONS:
- <action with optional P1/P2/P3, @Account, due: YYYY-MM-DD, #context>
END_ACTIONS
WINS:
- <positive outcome, success signal, or customer win>
END_WINS
RISKS:
- <risk, concern, blocker, or issue raised>
END_RISKS
DECISIONS:
- <key decision made during the meeting, including who decided and the rationale if stated>
END_DECISIONS

Rules for actions:
- Include priority when urgency is inferable (P1=urgent, P2=normal, P3=low)
- Include @AccountName when action relates to a specific customer/account
- Include due: YYYY-MM-DD when a deadline is mentioned or implied
- Include #context for topic category (billing, onboarding, support, etc.)
- Use "waiting" or "blocked" if action depends on someone else
- If no metadata can be inferred, just write the action text plainly
- Example: P1 @Acme Follow up on renewal due: 2026-03-15 #billing

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
        title = meeting.title,
        account = account,
        meeting_type = meeting_type,
        date = date,
        content = truncated,
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
fn append_to_impact_log(workspace: &Path, meeting: &CalendarEvent, wins: &[String]) {
    let impact_log = workspace.join("_today").join("90-impact-log.md");
    let mut content = String::new();

    if !impact_log.exists() {
        content.push_str("# Impact Log\n\n");
    }

    let label = meeting
        .account
        .as_deref()
        .unwrap_or(&meeting.title);
    let now = Utc::now();

    for win in wins {
        content.push_str(&format!(
            "- **{}**: {} ({})\n",
            label,
            win,
            now.format("%H:%M")
        ));
    }

    if impact_log.exists() {
        let existing = std::fs::read_to_string(&impact_log).unwrap_or_default();
        let _ = std::fs::write(&impact_log, format!("{}{}", existing, content));
    } else {
        let _ = std::fs::write(&impact_log, content);
    }
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
            .join("01-Customer-Information")
            .join(&dest_filename);

        assert_eq!(
            destination,
            PathBuf::from(
                "/workspace/Accounts/Acme-Corp/01-Customer-Information/2026-02-07-acme-qbr-transcript.md"
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
        assert_eq!(slugify("Weekly Sync — Team Alpha"), "weekly-sync-team-alpha");
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

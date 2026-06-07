//! Post-meeting capture state machine
//!
//! Detects when meetings end and prompts the user for quick outcomes.
//! Only prompts for customer/external meetings. Auto-dismisses after timeout.
//!
//! ## Transcript Detection (Phase 3B)
//!
//! When a meeting ends, the system first waits for a transcript to appear
//! in `_inbox/` (from Otter, Fireflies, Fathom, etc.). If one lands within
//! `transcript_wait_minutes`, the normal inbox pipeline processes it and no
//! prompt is shown. If no transcript appears, a lightweight fallback prompt
//! is emitted instead.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::state::AppState;
use crate::types::{CalendarEvent, MeetingType};

fn capture_ref(kind: &str, value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("{kind}:{}", &digest[..12])
}

fn meeting_ref(event: &CalendarEvent) -> String {
    capture_ref("meeting", &event.id)
}

fn transcript_file_ref(filename: &str) -> String {
    capture_ref("transcript_file", filename)
}

/// State machine for a pending post-meeting prompt.
#[derive(Debug, Clone)]
enum PromptState {
    /// Waiting to see if a transcript appears in `_inbox/`
    WaitingForTranscript { deadline: DateTime<Utc> },
    /// No transcript found — ready to show lightweight fallback prompt
    FallbackReady,
    /// Transcript found — auto-processing through inbox pipeline, no prompt needed
    TranscriptDetected { filename: String },
}

/// A pending prompt waiting to be shown
#[derive(Debug, Clone)]
struct PendingPrompt {
    meeting: CalendarEvent,
    /// When the prompt should actually trigger (meeting end + delay_minutes)
    trigger_time: DateTime<Utc>,
    state: PromptState,
}

/// Check if a meeting type should trigger a capture prompt
fn should_prompt(meeting_type: &MeetingType) -> bool {
    matches!(
        meeting_type,
        MeetingType::Customer | MeetingType::Qbr | MeetingType::Partnership | MeetingType::External
    )
}

/// Check `_inbox/` for transcript files matching a meeting.
///
/// Scans for common transcript tool patterns (Otter, Fireflies, Fathom)
/// and date/account-based filenames.
fn check_for_transcript(
    workspace: &Path,
    account: Option<&str>,
    meeting_date: &str,
) -> Option<String> {
    let inbox = workspace.join("_inbox");
    let entries = match std::fs::read_dir(&inbox) {
        Ok(e) => e,
        Err(_) => return None,
    };

    for entry in entries.flatten() {
        let filename = match entry.file_name().into_string() {
            Ok(f) => f,
            Err(_) => continue,
        };

        let lower = filename.to_lowercase();

        // Skip non-text files
        if !(lower.ends_with(".md")
            || lower.ends_with(".txt")
            || lower.ends_with(".vtt")
            || lower.ends_with(".srt"))
        {
            continue;
        }

        // Check for transcript tool indicators
        let is_transcript_tool = lower.contains("otter")
            || lower.contains("fireflies")
            || lower.contains("fathom")
            || lower.contains("read.ai")
            || lower.contains("transcript");

        if !is_transcript_tool {
            continue;
        }

        // Check for date match
        let has_date = lower.contains(meeting_date);

        // Check for account match (if account is known)
        let has_account = account
            .map(|a| {
                let slug = a.to_lowercase().replace([' ', '_'], "-");
                lower.contains(&slug)
            })
            .unwrap_or(false);

        // Accept if: (transcript tool + date) or (transcript tool + account)
        if has_date || has_account {
            return Some(filename);
        }
    }

    None
}

/// Run the capture detection loop alongside calendar polling.
///
/// After each calendar-updated event, checks for ended meetings and
/// schedules prompts. Uses a state machine to wait for transcripts
/// before falling back to a lightweight prompt.
pub async fn run_capture_loop(state: Arc<AppState>, app_handle: AppHandle) {
    let mut previous_in_progress: HashMap<String, CalendarEvent> = HashMap::new();
    let mut pending_prompts: Vec<PendingPrompt> = Vec::new();

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        // Dev mode isolation: pause background processing while dev sandbox is active
        if crate::db::is_dev_db_mode() {
            continue;
        }

        // Check if capture is enabled
        let config = state.config.read().clone();
        let enabled = config
            .as_ref()
            .map(|c| c.post_meeting_capture.enabled)
            .unwrap_or(true);
        if !enabled {
            continue;
        }

        let capture_config = config
            .as_ref()
            .map(|c| c.post_meeting_capture.clone())
            .unwrap_or_default();

        let workspace_path = config.as_ref().map(|c| c.workspace_path.clone());

        let now = Utc::now();

        // Get current events
        let current_events = state.calendar.events.read().clone();

        // Find events currently in progress
        let mut current_in_progress: HashMap<String, CalendarEvent> = HashMap::new();
        for event in &current_events {
            if event.start <= now && event.end > now && !event.is_all_day {
                current_in_progress.insert(event.id.clone(), event.clone());
            }
        }

        // Guard: if calendar went from populated to empty, auth likely expired.
        // Don't treat every in-progress meeting as "just ended."
        if current_events.is_empty() && !previous_in_progress.is_empty() {
            log::debug!(
                "Capture: calendar returned 0 events but {} were in-progress — skipping detection (likely auth issue)",
                previous_in_progress.len()
            );
            // Keep previous_in_progress as-is so we re-evaluate next tick
            continue;
        }

        // Find meetings that just ended (were in progress, now aren't)
        let dismissed = state.capture.dismissed.lock().clone();
        let captured = state.capture.captured.lock().clone();

        for (id, event) in &previous_in_progress {
            if !current_in_progress.contains_key(id)
                && should_prompt(&event.meeting_type)
                && !dismissed.contains(id)
                && !captured.contains(id)
            {
                let transcript_deadline =
                    now + Duration::minutes(capture_config.transcript_wait_minutes as i64);
                let trigger_time =
                    event.end + Duration::minutes(capture_config.delay_minutes as i64);

                pending_prompts.push(PendingPrompt {
                    meeting: event.clone(),
                    trigger_time,
                    state: PromptState::WaitingForTranscript {
                        deadline: transcript_deadline,
                    },
                });
                log::info!(
                    "Meeting ended: meeting_ref={} — waiting for transcript until {}",
                    meeting_ref(event),
                    transcript_deadline
                );
            }
        }

        previous_in_progress = current_in_progress.clone();

        // Process pending prompts through the state machine
        let mut to_remove = Vec::new();
        for (i, prompt) in pending_prompts.iter_mut().enumerate() {
            match &prompt.state {
                PromptState::WaitingForTranscript { deadline } => {
                    // Check for transcript in _inbox/
                    if let Some(ref ws) = workspace_path {
                        let meeting_date = prompt.meeting.end.format("%Y-%m-%d").to_string();
                        if let Some(filename) = check_for_transcript(
                            Path::new(ws),
                            prompt.meeting.account.as_deref(),
                            &meeting_date,
                        ) {
                            log::info!(
                                "Transcript detected: meeting_ref={}, transcript_file_ref={} — skipping prompt",
                                meeting_ref(&prompt.meeting),
                                transcript_file_ref(&filename)
                            );
                            prompt.state = PromptState::TranscriptDetected { filename };
                            continue;
                        }
                    }

                    // Check if deadline passed without transcript
                    if now >= *deadline {
                        log::info!(
                            "No transcript for meeting_ref={} — switching to fallback prompt",
                            meeting_ref(&prompt.meeting)
                        );
                        prompt.state = PromptState::FallbackReady;
                    }
                }

                PromptState::FallbackReady => {
                    // Wait until trigger_time and user is not in another meeting
                    if prompt.trigger_time <= now && current_in_progress.is_empty() {
                        log::info!(
                            "Triggering fallback capture prompt for meeting_ref={}",
                            meeting_ref(&prompt.meeting)
                        );
                        #[allow(
                            clippy::let_underscore_must_use,
                            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                        )]
                        let _ = app_handle.emit("post-meeting-prompt-fallback", &prompt.meeting);
                        to_remove.push(i);
                    }
                }

                PromptState::TranscriptDetected { filename } => {
                    // Transcript found — process with full meeting context (ADR-0044)
                    if let Some(ref ws) = workspace_path {
                        let file_path = Path::new(ws).join("_inbox").join(filename.as_str());

                        // Check immutability before processing
                        let already_processed = state
                            .capture
                            .transcript_processed
                            .lock()
                            .contains_key(&prompt.meeting.id);

                        if already_processed {
                            log::info!(
                                "Transcript for meeting_ref={} already processed — skipping",
                                meeting_ref(&prompt.meeting)
                            );
                        } else {
                            log::info!(
                                "Auto-processing transcript: meeting_ref={}, transcript_file_ref={}",
                                meeting_ref(&prompt.meeting),
                                transcript_file_ref(filename)
                            );

                            let profile = config
                                .as_ref()
                                .map(|c| c.profile.clone())
                                .unwrap_or_else(|| "general".to_string());
                            let ai_config = config
                                .as_ref()
                                .map(|c| c.ai_models.clone())
                                .unwrap_or_default();

                            // Open own DB connection to avoid holding state.db Mutex
                            // during PTY subprocess (which can run for minutes).
                            let own_db = crate::db::ActionDb::open(std::sync::Arc::new(
                                crate::db::LocalKeychain::new(),
                            ))
                            .ok();
                            let db_ref = own_db.as_ref();

                            let result = crate::processor::transcript::process_transcript(
                                Path::new(ws),
                                &file_path.display().to_string(),
                                &prompt.meeting,
                                Some(&app_handle),
                                db_ref,
                                &profile,
                                Some(&ai_config),
                            );

                            if result.status == "success" {
                                // Record transcript
                                let record = crate::types::TranscriptRecord {
                                    meeting_id: prompt.meeting.id.clone(),
                                    file_path: transcript_file_ref(
                                        &file_path.display().to_string(),
                                    ),
                                    destination: result
                                        .destination
                                        .as_deref()
                                        .map(transcript_file_ref)
                                        .unwrap_or_default(),
                                    summary: result.summary.clone(),
                                    processed_at: Utc::now().to_rfc3339(),
                                };
                                {
                                    let mut guard = state.capture.transcript_processed.lock();
                                    guard.insert(prompt.meeting.id.clone(), record);
                                    #[allow(
                                        clippy::let_underscore_must_use,
                                        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                                    )]
                                    let _ = crate::state::save_transcript_records(&guard);
                                }

                                // Mark as captured
                                {
                                    let mut guard = state.capture.captured.lock();
                                    guard.insert(prompt.meeting.id.clone());
                                }

                                // Remove the source file from inbox (it's been routed)
                                if file_path.exists() {
                                    #[allow(
                                        clippy::let_underscore_must_use,
                                        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                                    )]
                                    let _ = std::fs::remove_file(&file_path);
                                }

                                // Emit event for live frontend updates
                                let outcome =
                                    build_auto_outcome(&prompt.meeting.id, &result, &state);
                                #[allow(
                                    clippy::let_underscore_must_use,
                                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                                )]
                                let _ = app_handle.emit("transcript-processed", &outcome);

                                log::info!(
                                    "Auto-processed transcript: meeting_ref={}, wins_count={}, risks_count={}, decisions_count={}",
                                    meeting_ref(&prompt.meeting),
                                    result.wins.len(),
                                    result.risks.len(),
                                    result.decisions.len(),
                                );
                            } else {
                                log::warn!(
                                    "Auto-processing transcript failed: meeting_ref={}, message={}",
                                    meeting_ref(&prompt.meeting),
                                    result.message.unwrap_or_default()
                                );
                            }
                        }
                    }
                    to_remove.push(i);
                }
            }
        }

        // Remove processed prompts (reverse to preserve indices)
        for i in to_remove.into_iter().rev() {
            pending_prompts.remove(i);
        }

        // Clean up old pending prompts (> 2 hours old)
        pending_prompts.retain(|p| now - p.trigger_time < Duration::hours(2));
    }
}

/// Build MeetingOutcomeData from an auto-processed transcript result.
fn build_auto_outcome(
    meeting_id: &str,
    result: &crate::types::TranscriptResult,
    _state: &AppState,
) -> crate::types::MeetingOutcomeData {
    let actions = crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
        .ok()
        .and_then(|db| db.get_actions_for_meeting(meeting_id).ok())
        .unwrap_or_default();

    crate::types::MeetingOutcomeData {
        meeting_id: meeting_id.to_string(),
        summary: result.summary.clone(),
        wins: result.wins.clone(),
        risks: result.risks.clone(),
        decisions: result.decisions.clone(),
        actions,
        transcript_path: None,
        processed_at: Some(Utc::now().to_rfc3339()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcript_file_ref_does_not_include_raw_filename() {
        let raw = "sensitive-customer-qbr-transcript.md";
        let redacted = transcript_file_ref(raw);

        assert!(redacted.starts_with("transcript_file:"));
        assert!(!redacted.contains("sensitive"));
        assert!(!redacted.contains("customer"));
        assert_eq!(redacted, transcript_file_ref(raw));
    }
}

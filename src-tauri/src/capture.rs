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
use tauri::{AppHandle, Emitter};

use crate::state::AppState;
use crate::types::{CalendarEvent, MeetingType};

/// State machine for a pending post-meeting prompt.
#[derive(Debug, Clone)]
enum PromptState {
    /// Waiting to see if a transcript appears in `_inbox/`
    WaitingForTranscript {
        deadline: DateTime<Utc>,
    },
    /// No transcript found — ready to show lightweight fallback prompt
    FallbackReady,
    /// Transcript found — auto-processing through inbox pipeline, no prompt needed
    TranscriptDetected {
        filename: String,
    },
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
        MeetingType::Customer
            | MeetingType::Qbr
            | MeetingType::Partnership
            | MeetingType::External
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
                let slug = a.to_lowercase().replace(' ', "-").replace('_', "-");
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

        // Check if capture is enabled
        let config = state.config.lock().ok().and_then(|g| g.clone());
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
        let current_events = state
            .calendar_events
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        // Find events currently in progress
        let mut current_in_progress: HashMap<String, CalendarEvent> = HashMap::new();
        for event in &current_events {
            if event.start <= now && event.end > now && !event.is_all_day {
                current_in_progress.insert(event.id.clone(), event.clone());
            }
        }

        // Find meetings that just ended (were in progress, now aren't)
        let dismissed = state
            .capture_dismissed
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();
        let captured = state
            .capture_captured
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();

        for (id, event) in &previous_in_progress {
            if !current_in_progress.contains_key(id)
                && should_prompt(&event.meeting_type)
                && !dismissed.contains(id)
                && !captured.contains(id)
            {
                let transcript_deadline = now
                    + Duration::minutes(capture_config.transcript_wait_minutes as i64);
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
                    "Meeting ended: '{}' — waiting for transcript until {}",
                    event.title,
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
                                "Transcript detected for '{}': {} — skipping prompt",
                                prompt.meeting.title,
                                filename
                            );
                            prompt.state = PromptState::TranscriptDetected { filename };
                            continue;
                        }
                    }

                    // Check if deadline passed without transcript
                    if now >= *deadline {
                        log::info!(
                            "No transcript for '{}' — switching to fallback prompt",
                            prompt.meeting.title
                        );
                        prompt.state = PromptState::FallbackReady;
                    }
                }

                PromptState::FallbackReady => {
                    // Wait until trigger_time and user is not in another meeting
                    if prompt.trigger_time <= now && current_in_progress.is_empty() {
                        log::info!(
                            "Triggering fallback capture prompt for '{}'",
                            prompt.meeting.title
                        );
                        let _ = app_handle
                            .emit("post-meeting-prompt-fallback", &prompt.meeting);
                        to_remove.push(i);
                    }
                }

                PromptState::TranscriptDetected { filename } => {
                    // Transcript found — the inbox pipeline will handle it.
                    log::debug!(
                        "Removing prompt for '{}' — transcript '{}' will be processed via inbox",
                        prompt.meeting.title,
                        filename
                    );
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

//! Post-meeting capture state machine
//!
//! Detects when meetings end and prompts the user for quick outcomes.
//! Only prompts for customer/external meetings. Auto-dismisses after timeout.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use tauri::{AppHandle, Emitter};

use crate::state::AppState;
use crate::types::{CalendarEvent, MeetingType};

/// A pending prompt waiting to be shown
#[derive(Debug, Clone)]
struct PendingPrompt {
    meeting: CalendarEvent,
    trigger_time: DateTime<Utc>,
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

/// Run the capture detection loop alongside calendar polling.
///
/// After each calendar-updated event, checks for ended meetings and
/// schedules prompts. When prompt time arrives and user is not in another
/// meeting, emits `post-meeting-prompt`.
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

        let delay_minutes = config
            .as_ref()
            .map(|c| c.post_meeting_capture.delay_minutes)
            .unwrap_or(5);

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
                let trigger_time = event.end + Duration::minutes(delay_minutes as i64);
                pending_prompts.push(PendingPrompt {
                    meeting: event.clone(),
                    trigger_time,
                });
                log::info!(
                    "Scheduled capture prompt for '{}' at {}",
                    event.title,
                    trigger_time
                );
            }
        }

        previous_in_progress = current_in_progress.clone();

        // Check pending prompts
        let mut triggered = Vec::new();
        for (i, prompt) in pending_prompts.iter().enumerate() {
            if prompt.trigger_time <= now {
                // Don't prompt if user is currently in another meeting
                if current_in_progress.is_empty() {
                    log::info!(
                        "Triggering capture prompt for '{}'",
                        prompt.meeting.title
                    );
                    let _ = app_handle.emit("post-meeting-prompt", &prompt.meeting);
                    triggered.push(i);
                }
                // If in a meeting, leave in pending — will trigger when free
            }
        }

        // Remove triggered prompts (reverse to preserve indices)
        for i in triggered.into_iter().rev() {
            pending_prompts.remove(i);
        }

        // Clean up old pending prompts (> 2 hours old)
        pending_prompts.retain(|p| now - p.trigger_time < Duration::hours(2));
    }
}

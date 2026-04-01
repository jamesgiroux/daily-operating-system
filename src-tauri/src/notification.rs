//! Native notification wrapper
//!
//! Provides a simple interface to send native notifications.
//! Uses tauri-plugin-notification for cross-platform support.
//!
//! Includes rate-limiting for transcript notifications (5-minute cooldown)
//! and user-configurable notification preferences.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::state::AppState;
use crate::types::NotificationConfig;

/// Cooldown between transcript-ready notifications (5 minutes).
const TRANSCRIPT_NOTIFICATION_COOLDOWN: Duration = Duration::from_secs(300);

/// Tracks the last time a transcript notification was sent.
static LAST_TRANSCRIPT_NOTIFICATION: Mutex<Option<Instant>> = Mutex::new(None);

/// Check whether a notification category is enabled given the user's preferences.
/// Also checks quiet hours — if the current local hour falls within quiet hours, suppress.
fn should_send(config: &NotificationConfig, category: NotificationCategory) -> bool {
    let enabled = match category {
        NotificationCategory::WorkflowCompletion => config.workflow_completion,
        NotificationCategory::TranscriptReady => config.transcript_ready,
        NotificationCategory::AuthExpiry => config.auth_expiry,
    };

    if !enabled {
        return false;
    }

    // Quiet hours check
    if let (Some(start), Some(end)) = (config.quiet_hours_start, config.quiet_hours_end) {
        let now_hour = chrono::Local::now().hour() as u8;
        let in_quiet = if start <= end {
            // e.g. 22..6 doesn't wrap, but 9..17 is daytime quiet
            now_hour >= start && now_hour < end
        } else {
            // Wraps midnight: e.g. start=22, end=6 → quiet from 22:00 to 05:59
            now_hour >= start || now_hour < end
        };
        if in_quiet {
            return false;
        }
    }

    true
}

use chrono::Timelike;

/// Notification categories that map to user-configurable toggles.
enum NotificationCategory {
    WorkflowCompletion,
    TranscriptReady,
    AuthExpiry,
}

/// Load notification config from AppState, falling back to defaults.
fn load_notification_config(state: &AppState) -> NotificationConfig {
    state
        .config
        .read()
        .ok()
        .and_then(|guard| guard.as_ref().map(|c| c.notifications.clone()))
        .unwrap_or_default()
}

/// Send a notification to the user
pub fn send_notification(app: &AppHandle, title: &str, body: &str) -> Result<(), String> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|e| format!("Failed to send notification: {}", e))
}

/// Send a success notification for workflow completion.
/// Respects the user's `workflow_completion` toggle and quiet hours.
pub fn notify_workflow_complete(
    app: &AppHandle,
    workflow_name: &str,
    state: &AppState,
) -> Result<(), String> {
    let config = load_notification_config(state);
    if !should_send(&config, NotificationCategory::WorkflowCompletion) {
        log::debug!("Suppressing workflow notification (disabled or quiet hours)");
        return Ok(());
    }

    let title = match workflow_name {
        "today" => "Your day is ready",
        "archive" => "Archive complete",
        _ => "Workflow complete",
    };

    let body = match workflow_name {
        "today" => "DailyOS has prepared your daily briefing.",
        "archive" => "Yesterday's files have been archived.",
        _ => "The workflow has completed successfully.",
    };

    send_notification(app, title, body)
}

/// Send a notification when a transcript has been processed.
///
/// Rate-limited: only fires if >5 minutes since the last transcript notification.
/// Subsequent transcripts within the cooldown window are silently dropped.
/// Also respects the user's `transcript_ready` toggle and quiet hours.
pub fn notify_transcript_ready(
    app: &AppHandle,
    meeting_title: &str,
    account: Option<&str>,
    state: &AppState,
) -> Result<(), String> {
    let config = load_notification_config(state);
    if !should_send(&config, NotificationCategory::TranscriptReady) {
        log::debug!("Suppressing transcript notification (disabled or quiet hours)");
        return Ok(());
    }

    // Rate-limit: check cooldown
    {
        let mut last = LAST_TRANSCRIPT_NOTIFICATION
            .lock()
            .map_err(|_| "Notification lock poisoned")?;
        if let Some(prev) = *last {
            if prev.elapsed() < TRANSCRIPT_NOTIFICATION_COOLDOWN {
                log::debug!(
                    "Suppressing transcript notification for '{}' (cooldown: {}s remaining)",
                    meeting_title,
                    (TRANSCRIPT_NOTIFICATION_COOLDOWN - prev.elapsed()).as_secs()
                );
                return Ok(());
            }
        }
        *last = Some(Instant::now());
    }

    let body = match account {
        Some(a) if !a.is_empty() => format!("{} — {}", meeting_title, a),
        _ => meeting_title.to_string(),
    };
    send_notification(app, "Meeting notes ready", &body)
}

/// Send a native notification when Google OAuth token expires.
/// Fires to macOS Notification Center so the user sees it even when the app is backgrounded.
/// Respects the user's `auth_expiry` toggle and quiet hours.
pub fn notify_auth_expired(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let config = load_notification_config(state);
    if !should_send(&config, NotificationCategory::AuthExpiry) {
        log::debug!("Suppressing auth expiry notification (disabled or quiet hours)");
        return Ok(());
    }

    send_notification(
        app,
        "DailyOS — Action Required",
        "Reconnect your Google account in Settings.",
    )
}

/// Emit a system-status event to the frontend for toast display.
pub fn emit_system_status(app: &AppHandle, status_type: &str, message: &str) {
    #[derive(serde::Serialize, Clone)]
    struct SystemStatus {
        r#type: String,
        message: String,
    }
    let _ = app.emit(
        "system-status",
        SystemStatus {
            r#type: status_type.to_string(),
            message: message.to_string(),
        },
    );
}

/// Send an error notification
pub fn notify_workflow_error(
    app: &AppHandle,
    workflow_name: &str,
    error: &str,
) -> Result<(), String> {
    let title = format!("{} workflow failed", workflow_name);
    let body = if error.len() > 100 {
        format!("{}...", &error[..100])
    } else {
        error.to_string()
    };

    send_notification(app, &title, &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_send_all_enabled() {
        let config = NotificationConfig::default();
        assert!(should_send(&config, NotificationCategory::WorkflowCompletion));
        assert!(should_send(&config, NotificationCategory::TranscriptReady));
        assert!(should_send(&config, NotificationCategory::AuthExpiry));
    }

    #[test]
    fn test_should_send_disabled() {
        let config = NotificationConfig {
            workflow_completion: false,
            transcript_ready: true,
            auth_expiry: true,
            quiet_hours_start: None,
            quiet_hours_end: None,
        };
        assert!(!should_send(&config, NotificationCategory::WorkflowCompletion));
        assert!(should_send(&config, NotificationCategory::TranscriptReady));
    }

    #[test]
    fn test_quiet_hours_no_wrap() {
        // Quiet from 9 to 17 — current hour determines result
        let config = NotificationConfig {
            workflow_completion: true,
            transcript_ready: true,
            auth_expiry: true,
            quiet_hours_start: Some(9),
            quiet_hours_end: Some(17),
        };
        // We can't control chrono::Local::now() in unit tests without mocking,
        // but we can verify the function doesn't panic
        let _ = should_send(&config, NotificationCategory::WorkflowCompletion);
    }

    #[test]
    fn test_quiet_hours_wraps_midnight() {
        let config = NotificationConfig {
            workflow_completion: true,
            transcript_ready: true,
            auth_expiry: true,
            quiet_hours_start: Some(22),
            quiet_hours_end: Some(6),
        };
        let _ = should_send(&config, NotificationCategory::TranscriptReady);
    }
}

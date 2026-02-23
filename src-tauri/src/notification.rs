//! Native notification wrapper
//!
//! Provides a simple interface to send native notifications.
//! Uses tauri-plugin-notification for cross-platform support.

use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

/// Send a notification to the user
pub fn send_notification(app: &AppHandle, title: &str, body: &str) -> Result<(), String> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|e| format!("Failed to send notification: {}", e))
}

/// Send a success notification for workflow completion
pub fn notify_workflow_complete(app: &AppHandle, workflow_name: &str) -> Result<(), String> {
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

/// Send a notification when a Quill transcript has been processed.
pub fn notify_transcript_ready(
    app: &AppHandle,
    meeting_title: &str,
    account: Option<&str>,
) -> Result<(), String> {
    let body = match account {
        Some(a) if !a.is_empty() => format!("{} — {}", meeting_title, a),
        _ => meeting_title.to_string(),
    };
    send_notification(app, "Transcript Ready", &body)
}

/// Send a native notification when Google OAuth token expires.
/// Fires to macOS Notification Center so the user sees it even when the app is backgrounded.
pub fn notify_auth_expired(app: &AppHandle) -> Result<(), String> {
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

//! Native notification wrapper
//!
//! Provides a simple interface to send native notifications.
//! Uses tauri-plugin-notification for cross-platform support.

use tauri::AppHandle;
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

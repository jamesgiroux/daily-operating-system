use std::path::Path;

use tauri::State;

use crate::parser::{calculate_stats, count_inbox, parse_actions, parse_meetings, parse_overview};
use crate::state::{reload_config, AppState};
use crate::types::{Config, DashboardData};

/// Result type for dashboard data loading
#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum DashboardResult {
    Success { data: DashboardData },
    Empty { message: String },
    Error { message: String },
}

/// Get current configuration
#[tauri::command]
pub fn get_config(state: State<AppState>) -> Result<Config, String> {
    let guard = state.config.lock().map_err(|_| "Lock poisoned")?;
    guard.clone().ok_or_else(|| {
        "No configuration loaded. Create ~/.daybreak/config.json".to_string()
    })
}

/// Reload configuration from disk
#[tauri::command]
pub fn reload_configuration(state: State<AppState>) -> Result<Config, String> {
    reload_config(&state)
}

/// Get dashboard data from workspace _today/ files
#[tauri::command]
pub fn get_dashboard_data(state: State<AppState>) -> DashboardResult {
    // Get config
    let config = match state.config.lock() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => {
                return DashboardResult::Error {
                    message: "No configuration. Create ~/.daybreak/config.json with { \"workspacePath\": \"/path/to/workspace\" }".to_string(),
                }
            }
        },
        Err(_) => {
            return DashboardResult::Error {
                message: "Internal error: config lock poisoned".to_string(),
            }
        }
    };

    let workspace = Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    // Check if _today directory exists
    if !today_dir.exists() {
        return DashboardResult::Empty {
            message: "No briefing yet. Run /today to generate your daily overview.".to_string(),
        };
    }

    // Parse overview
    let overview_path = today_dir.join("overview.md");
    let overview = if overview_path.exists() {
        match parse_overview(&overview_path) {
            Ok(o) => o,
            Err(e) => {
                return DashboardResult::Error {
                    message: format!("Failed to parse overview: {}", e),
                }
            }
        }
    } else {
        return DashboardResult::Empty {
            message: "No overview found. Run /today to generate your daily briefing.".to_string(),
        };
    };

    // Parse meetings (optional)
    let meetings_path = today_dir.join("meetings.md");
    let meetings = if meetings_path.exists() {
        parse_meetings(&meetings_path).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Parse actions (optional)
    let actions_path = today_dir.join("actions.md");
    let actions = if actions_path.exists() {
        parse_actions(&actions_path).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Count inbox
    let inbox_count = count_inbox(workspace);

    // Calculate stats
    let stats = calculate_stats(&meetings, &actions, inbox_count);

    DashboardResult::Success {
        data: DashboardData {
            overview,
            stats,
            meetings,
            actions,
        },
    }
}

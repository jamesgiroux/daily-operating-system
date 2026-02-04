use std::fs;
use std::sync::Mutex;

use crate::types::Config;

/// Application state managed by Tauri
pub struct AppState {
    pub config: Mutex<Option<Config>>,
}

impl AppState {
    pub fn new() -> Self {
        let config = load_config().ok();
        Self {
            config: Mutex::new(config),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Load configuration from ~/.daybreak/config.json
pub fn load_config() -> Result<Config, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let config_path = home.join(".daybreak").join("config.json");

    if !config_path.exists() {
        return Err(format!(
            "Config file not found at {}. Create it with: {{ \"workspacePath\": \"/path/to/workspace\" }}",
            config_path.display()
        ));
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    let config: Config = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    // Validate workspace path exists
    let workspace_path = std::path::Path::new(&config.workspace_path);
    if !workspace_path.exists() {
        return Err(format!(
            "Workspace path does not exist: {}",
            config.workspace_path
        ));
    }

    Ok(config)
}

/// Reload configuration from disk
pub fn reload_config(state: &AppState) -> Result<Config, String> {
    let config = load_config()?;
    let mut guard = state.config.lock().map_err(|_| "Lock poisoned")?;
    *guard = Some(config.clone());
    Ok(config)
}

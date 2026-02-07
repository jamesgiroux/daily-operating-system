use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{DateTime, Utc};

use crate::types::{
    CalendarEvent, Config, ExecutionRecord, ExecutionTrigger, GoogleAuthStatus, TranscriptRecord,
    WeekPlanningState, WorkflowId, WorkflowStatus,
};

/// Maximum number of execution records to keep in memory
const MAX_HISTORY_SIZE: usize = 100;

/// Application state managed by Tauri
pub struct AppState {
    pub config: Mutex<Option<Config>>,
    pub workflow_status: Mutex<HashMap<WorkflowId, WorkflowStatus>>,
    pub execution_history: Mutex<Vec<ExecutionRecord>>,
    pub last_scheduled_run: Mutex<HashMap<WorkflowId, DateTime<Utc>>>,
    pub db: Mutex<Option<crate::db::ActionDb>>,
    // Phase 3: Google + Calendar + Capture + Week Planning
    pub google_auth: Mutex<GoogleAuthStatus>,
    pub calendar_events: Mutex<Vec<CalendarEvent>>,
    pub capture_dismissed: Mutex<std::collections::HashSet<String>>,
    pub capture_captured: Mutex<std::collections::HashSet<String>>,
    pub week_planning_state: Mutex<WeekPlanningState>,
    /// Tracks processed transcripts by meeting_id for immutability (one transcript per meeting)
    pub transcript_processed: Mutex<HashMap<String, TranscriptRecord>>,
}

impl AppState {
    pub fn new() -> Self {
        let config = load_config().ok();
        let history = load_execution_history().unwrap_or_default();

        let db = match crate::db::ActionDb::open() {
            Ok(db) => Some(db),
            Err(e) => {
                log::warn!("Failed to open actions database: {e}. DB features disabled.");
                None
            }
        };

        // Detect existing Google token on startup
        let google_auth = detect_google_auth();

        // Load transcript records from disk
        let transcript_processed = load_transcript_records().unwrap_or_default();

        Self {
            config: Mutex::new(config),
            workflow_status: Mutex::new(HashMap::new()),
            execution_history: Mutex::new(history),
            last_scheduled_run: Mutex::new(HashMap::new()),
            db: Mutex::new(db),
            google_auth: Mutex::new(google_auth),
            calendar_events: Mutex::new(Vec::new()),
            capture_dismissed: Mutex::new(std::collections::HashSet::new()),
            capture_captured: Mutex::new(std::collections::HashSet::new()),
            week_planning_state: Mutex::new(WeekPlanningState::default()),
            transcript_processed: Mutex::new(transcript_processed),
        }
    }

    /// Get current status of a workflow
    pub fn get_workflow_status(&self, workflow: WorkflowId) -> WorkflowStatus {
        self.workflow_status
            .lock()
            .map(|guard| guard.get(&workflow).cloned().unwrap_or_default())
            .unwrap_or_default()
    }

    /// Update workflow status
    pub fn set_workflow_status(&self, workflow: WorkflowId, status: WorkflowStatus) {
        if let Ok(mut guard) = self.workflow_status.lock() {
            guard.insert(workflow, status);
        }
    }

    /// Add an execution record to history
    pub fn add_execution_record(&self, record: ExecutionRecord) {
        if let Ok(mut guard) = self.execution_history.lock() {
            guard.insert(0, record);

            // Trim to max size
            if guard.len() > MAX_HISTORY_SIZE {
                guard.truncate(MAX_HISTORY_SIZE);
            }
        }

        // Persist to disk (fire and forget)
        let _ = self.save_execution_history();
    }

    /// Update an existing execution record
    pub fn update_execution_record(&self, id: &str, f: impl FnOnce(&mut ExecutionRecord)) {
        if let Ok(mut guard) = self.execution_history.lock() {
            if let Some(record) = guard.iter_mut().find(|r| r.id == id) {
                f(record);
            }
        }

        // Persist to disk
        let _ = self.save_execution_history();
    }

    /// Get execution history
    pub fn get_execution_history(&self, limit: usize) -> Vec<ExecutionRecord> {
        self.execution_history
            .lock()
            .map(|guard| guard.iter().take(limit).cloned().collect())
            .unwrap_or_default()
    }

    /// Record when a scheduled run last occurred
    pub fn set_last_scheduled_run(&self, workflow: WorkflowId, time: DateTime<Utc>) {
        if let Ok(mut guard) = self.last_scheduled_run.lock() {
            guard.insert(workflow, time);
        }
    }

    /// Get when a workflow last ran on schedule
    pub fn get_last_scheduled_run(&self, workflow: WorkflowId) -> Option<DateTime<Utc>> {
        self.last_scheduled_run
            .lock()
            .ok()
            .and_then(|guard| guard.get(&workflow).cloned())
    }

    /// Save execution history to disk
    fn save_execution_history(&self) -> Result<(), String> {
        let history = self
            .execution_history
            .lock()
            .map_err(|_| "Lock poisoned")?
            .clone();

        let path = get_state_dir()?.join("execution_history.json");
        let content =
            serde_json::to_string_pretty(&history).map_err(|e| format!("Serialize error: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Write error: {}", e))?;

        Ok(())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the canonical config file path (~/.dailyos/config.json)
pub fn config_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    Ok(home.join(".dailyos").join("config.json"))
}

/// Create or update config.json atomically.
///
/// If config already exists in-memory, clones it, applies the mutator, and writes back.
/// If config is None (first-run), creates a default Config with serde defaults, applies
/// the mutator, ensures ~/.dailyos/ exists, and writes + updates in-memory state.
pub fn create_or_update_config(
    state: &AppState,
    mutator: impl FnOnce(&mut Config),
) -> Result<Config, String> {
    let mut guard = state.config.lock().map_err(|_| "Lock poisoned")?;

    let mut config = match guard.clone() {
        Some(c) => c,
        None => {
            // Create default config — workspace_path empty, will be set by mutator or later
            Config {
                workspace_path: String::new(),
                schedules: crate::types::Schedules::default(),
                profile: crate::types::profile_for_entity_mode("account"),
                profile_config: None,
                entity_mode: "account".to_string(),
                google: crate::types::GoogleConfig::default(),
                post_meeting_capture: crate::types::PostMeetingCaptureConfig::default(),
                features: std::collections::HashMap::new(),
            }
        }
    };

    mutator(&mut config);

    // Ensure ~/.dailyos/ exists
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }
    }

    // Write to disk
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    // Update in-memory state
    *guard = Some(config.clone());

    Ok(config)
}

/// Initialize workspace directory structure.
///
/// Always creates: _today/, _today/data/, _inbox/, _archive/, Projects/
/// Conditionally creates: Accounts/ if entity_mode is "account" or "both"
/// Idempotent: skips existing dirs, never overwrites files.
pub fn initialize_workspace(path: &std::path::Path, entity_mode: &str) -> Result<(), String> {
    // Validate parent exists
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            return Err(format!(
                "Parent directory does not exist: {}",
                parent.display()
            ));
        }
    }

    let dirs = vec![
        path.to_path_buf(),
        path.join("_today"),
        path.join("_today").join("data"),
        path.join("_inbox"),
        path.join("_archive"),
        path.join("Projects"),
    ];

    for dir in &dirs {
        if !dir.exists() {
            fs::create_dir_all(dir)
                .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;
        }
    }

    // Conditionally create Accounts/
    if entity_mode == "account" || entity_mode == "both" {
        let accounts_dir = path.join("Accounts");
        if !accounts_dir.exists() {
            fs::create_dir_all(&accounts_dir)
                .map_err(|e| format!("Failed to create Accounts: {}", e))?;
        }
    }

    Ok(())
}

/// Get the state directory (~/.dailyos)
fn get_state_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let state_dir = home.join(".dailyos");

    if !state_dir.exists() {
        fs::create_dir_all(&state_dir).map_err(|e| format!("Failed to create state dir: {}", e))?;
    }

    Ok(state_dir)
}

/// Load configuration from ~/.dailyos/config.json
pub fn load_config() -> Result<Config, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let config_path = home.join(".dailyos").join("config.json");

    if !config_path.exists() {
        return Err(format!(
            "Config file not found at {}. Create it with: {{ \"workspacePath\": \"/path/to/workspace\" }}",
            config_path.display()
        ));
    }

    let content =
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    let config: Config =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;

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

/// Load execution history from disk
fn load_execution_history() -> Result<Vec<ExecutionRecord>, String> {
    let path = get_state_dir()?.join("execution_history.json");

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read history: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse history: {}", e))
}

/// Load transcript records from `~/.dailyos/transcript_records.json`.
fn load_transcript_records() -> Result<HashMap<String, TranscriptRecord>, String> {
    let path = get_state_dir()?.join("transcript_records.json");
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read transcript records: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse transcript records: {}", e))
}

/// Save transcript records to `~/.dailyos/transcript_records.json`.
pub fn save_transcript_records(
    records: &HashMap<String, TranscriptRecord>,
) -> Result<(), String> {
    let path = get_state_dir()?.join("transcript_records.json");
    let content = serde_json::to_string_pretty(records)
        .map_err(|e| format!("Serialize error: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Write error: {}", e))?;
    Ok(())
}

/// Reload configuration from disk
pub fn reload_config(state: &AppState) -> Result<Config, String> {
    let config = load_config()?;
    let mut guard = state.config.lock().map_err(|_| "Lock poisoned")?;
    *guard = Some(config.clone());
    Ok(config)
}

/// Get the default Google token path
pub fn google_token_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    home.join(".dailyos").join("google").join("token.json")
}

/// Detect existing Google authentication by checking the token file on disk.
pub fn detect_google_auth() -> GoogleAuthStatus {
    let token_path = google_token_path();
    if !token_path.exists() {
        return GoogleAuthStatus::NotConfigured;
    }

    // Try to read the token file and validate it has real OAuth fields
    match fs::read_to_string(&token_path) {
        Ok(content) => {
            if let Ok(token) = serde_json::from_str::<serde_json::Value>(&content) {
                // A valid Google OAuth token must have at least a refresh_token or token field.
                // An empty {} or missing fields means auth never completed.
                let has_token = token.get("token").is_some()
                    || token.get("refresh_token").is_some();
                if !has_token {
                    return GoogleAuthStatus::NotConfigured;
                }
                // google-auth-oauthlib stores email in the "account" field, not "email"
                let email = token
                    .get("email")
                    .or_else(|| token.get("account"))
                    .and_then(|e| e.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("connected")
                    .to_string();
                GoogleAuthStatus::Authenticated { email }
            } else {
                // Token file exists but is invalid JSON
                GoogleAuthStatus::TokenExpired
            }
        }
        Err(_) => GoogleAuthStatus::NotConfigured,
    }
}

/// Create a new execution record
pub fn create_execution_record(workflow: WorkflowId, trigger: ExecutionTrigger) -> ExecutionRecord {
    ExecutionRecord {
        id: uuid::Uuid::new_v4().to_string(),
        workflow,
        started_at: Utc::now(),
        finished_at: None,
        duration_secs: None,
        success: false,
        error_message: None,
        trigger,
    }
}

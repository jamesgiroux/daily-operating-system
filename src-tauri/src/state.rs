use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{DateTime, Utc};

use crate::types::{Config, ExecutionRecord, ExecutionTrigger, WorkflowId, WorkflowStatus};

/// Maximum number of execution records to keep in memory
const MAX_HISTORY_SIZE: usize = 100;

/// Application state managed by Tauri
pub struct AppState {
    pub config: Mutex<Option<Config>>,
    pub workflow_status: Mutex<HashMap<WorkflowId, WorkflowStatus>>,
    pub execution_history: Mutex<Vec<ExecutionRecord>>,
    pub last_scheduled_run: Mutex<HashMap<WorkflowId, DateTime<Utc>>>,
    pub db: Mutex<Option<crate::db::ActionDb>>,
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

        Self {
            config: Mutex::new(config),
            workflow_status: Mutex::new(HashMap::new()),
            execution_history: Mutex::new(history),
            last_scheduled_run: Mutex::new(HashMap::new()),
            db: Mutex::new(db),
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

/// Get the state directory (~/.daybreak)
fn get_state_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let state_dir = home.join(".daybreak");

    if !state_dir.exists() {
        fs::create_dir_all(&state_dir).map_err(|e| format!("Failed to create state dir: {}", e))?;
    }

    Ok(state_dir)
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

/// Reload configuration from disk
pub fn reload_config(state: &AppState) -> Result<Config, String> {
    let config = load_config()?;
    let mut guard = state.config.lock().map_err(|_| "Lock poisoned")?;
    *guard = Some(config.clone());
    Ok(config)
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

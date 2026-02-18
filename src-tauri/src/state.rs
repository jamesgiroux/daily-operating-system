use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex, RwLock, TryLockError};
use std::time::Instant;

use chrono::{DateTime, Utc};

use crate::types::{
    CalendarEvent, Config, ExecutionRecord, ExecutionTrigger, GoogleAuthStatus, TranscriptRecord,
    WorkflowId, WorkflowStatus,
};

/// Maximum number of execution records to keep in memory
const MAX_HISTORY_SIZE: usize = 100;

/// Daily AI call budget for proactive hygiene (I146 — ADR-0058).
pub struct HygieneBudget {
    pub daily_ai_calls: AtomicU32,
    pub daily_limit: u32,
    /// ISO date string (YYYY-MM-DD) of last reset. Resets at midnight local time.
    pub last_reset: Mutex<String>,
}

impl Default for HygieneBudget {
    fn default() -> Self {
        Self::new(10) // Daytime default: 10 AI calls/day
    }
}

impl HygieneBudget {
    pub fn new(limit: u32) -> Self {
        Self {
            daily_ai_calls: AtomicU32::new(0),
            daily_limit: limit,
            last_reset: Mutex::new(chrono::Local::now().format("%Y-%m-%d").to_string()),
        }
    }

    /// Check if budget allows another AI call, resetting counter if day changed.
    pub fn try_consume(&self) -> bool {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        if let Ok(mut last) = self.last_reset.lock() {
            if *last != today {
                self.daily_ai_calls
                    .store(0, std::sync::atomic::Ordering::Relaxed);
                *last = today;
            }
        }

        let current = self
            .daily_ai_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if current < self.daily_limit {
            true
        } else {
            // Undo the increment
            self.daily_ai_calls
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            false
        }
    }

    /// Current count of AI calls used today.
    pub fn used_today(&self) -> u32 {
        self.daily_ai_calls
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Application state managed by Tauri
pub struct AppState {
    pub config: RwLock<Option<Config>>,
    pub workflow_status: RwLock<HashMap<WorkflowId, WorkflowStatus>>,
    pub execution_history: Mutex<Vec<ExecutionRecord>>,
    pub last_scheduled_run: RwLock<HashMap<WorkflowId, DateTime<Utc>>>,
    pub db: Mutex<Option<crate::db::ActionDb>>,
    // Phase 3: Google + Calendar + Capture
    pub google_auth: Mutex<GoogleAuthStatus>,
    pub calendar_events: RwLock<Vec<CalendarEvent>>,
    pub capture_dismissed: Mutex<std::collections::HashSet<String>>,
    pub capture_captured: Mutex<std::collections::HashSet<String>>,
    /// Tracks processed transcripts by meeting_id for immutability (one transcript per meeting)
    pub transcript_processed: Mutex<HashMap<String, TranscriptRecord>>,
    /// Background intelligence enrichment queue (I132)
    pub intel_queue: Arc<crate::intel_queue::IntelligenceQueue>,
    /// Shared embedding model runtime (Sprint 26).
    pub embedding_model: Arc<crate::embeddings::EmbeddingModel>,
    /// Background embedding generation queue (Sprint 26).
    pub embedding_queue: Arc<crate::processor::embeddings::EmbeddingQueue>,
    /// Last hygiene scan report (I145 — ADR-0058)
    pub last_hygiene_report: Mutex<Option<crate::hygiene::HygieneReport>>,
    /// Indicates whether a hygiene scan is currently running.
    pub hygiene_scan_running: AtomicBool,
    /// ISO timestamp for the most recent completed hygiene scan.
    pub last_hygiene_scan_at: Mutex<Option<String>>,
    /// ISO timestamp for the next scheduled hygiene scan.
    pub next_hygiene_scan_at: Mutex<Option<String>>,
    /// Daily AI budget for proactive hygiene (I146 — ADR-0058)
    pub hygiene_budget: HygieneBudget,
    /// TTL cache for live week calendar events used by proactive suggestions (W6).
    /// Stores classified CalendarEvents for Mon-Fri + the instant they were fetched.
    pub week_calendar_cache: RwLock<Option<(Vec<CalendarEvent>, Instant)>>,
    /// Whether the first-run full orphan scan has been completed (I271).
    pub hygiene_full_orphan_scan_done: AtomicBool,
    /// Stashed live workspace path before switching to dev mode (I298).
    /// `restore_live()` reads this back to return to the user's real workspace.
    pub pre_dev_workspace: Mutex<Option<String>>,
}

/// Non-blocking DB read outcome for hot command paths.
pub enum DbTryRead<T> {
    Ok(T),
    Busy,
    Unavailable,
    Poisoned,
}

impl AppState {
    pub fn new() -> Self {
        // I298 recovery: if a dev-backup config exists, the app was quit during
        // dev mode without calling restore_live(). Restore the live config before
        // loading so startup sync doesn't import mock data into the live DB.
        recover_from_unclean_dev_exit();

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

        let hygiene_budget_limit = config
            .as_ref()
            .map(|c| c.hygiene_ai_budget)
            .unwrap_or(10);

        Self {
            config: RwLock::new(config),
            workflow_status: RwLock::new(HashMap::new()),
            execution_history: Mutex::new(history),
            last_scheduled_run: RwLock::new(HashMap::new()),
            db: Mutex::new(db),
            google_auth: Mutex::new(google_auth),
            calendar_events: RwLock::new(Vec::new()),
            capture_dismissed: Mutex::new(std::collections::HashSet::new()),
            capture_captured: Mutex::new(std::collections::HashSet::new()),
            transcript_processed: Mutex::new(transcript_processed),
            intel_queue: Arc::new(crate::intel_queue::IntelligenceQueue::new()),
            embedding_model: Arc::new(crate::embeddings::EmbeddingModel::new()),
            embedding_queue: Arc::new(crate::processor::embeddings::EmbeddingQueue::new()),
            last_hygiene_report: Mutex::new(None),
            hygiene_scan_running: AtomicBool::new(false),
            last_hygiene_scan_at: Mutex::new(None),
            next_hygiene_scan_at: Mutex::new(None),
            hygiene_budget: HygieneBudget::new(hygiene_budget_limit),
            week_calendar_cache: RwLock::new(None),
            hygiene_full_orphan_scan_done: AtomicBool::new(false),
            pre_dev_workspace: Mutex::new(None),
        }
    }

    /// Get current status of a workflow
    pub fn get_workflow_status(&self, workflow: WorkflowId) -> WorkflowStatus {
        self.workflow_status
            .read()
            .map(|guard| guard.get(&workflow).cloned().unwrap_or_default())
            .unwrap_or_default()
    }

    /// Update workflow status
    pub fn set_workflow_status(&self, workflow: WorkflowId, status: WorkflowStatus) {
        if let Ok(mut guard) = self.workflow_status.write() {
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
        if let Ok(mut guard) = self.last_scheduled_run.write() {
            guard.insert(workflow, time);
        }
    }

    /// Get when a workflow last ran on schedule
    pub fn get_last_scheduled_run(&self, workflow: WorkflowId) -> Option<DateTime<Utc>> {
        self.last_scheduled_run
            .read()
            .ok()
            .and_then(|guard| guard.get(&workflow).cloned())
    }

    /// Hot-path DB read helper.
    ///
    /// Uses `try_lock()` so UI-facing read commands can degrade gracefully under
    /// contention instead of blocking the render path.
    pub fn with_db_try_read<T, F>(&self, f: F) -> DbTryRead<T>
    where
        F: FnOnce(&crate::db::ActionDb) -> T,
    {
        match self.db.try_lock() {
            Ok(guard) => {
                if let Some(db) = guard.as_ref() {
                    DbTryRead::Ok(f(db))
                } else {
                    DbTryRead::Unavailable
                }
            }
            Err(TryLockError::WouldBlock) => DbTryRead::Busy,
            Err(TryLockError::Poisoned(_)) => DbTryRead::Poisoned,
        }
    }

    /// Standard DB read helper for non-hot paths.
    ///
    /// Keep lock scope short: gather data, release lock, then do compute/network/IO.
    pub fn with_db_read<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&crate::db::ActionDb) -> Result<T, String>,
    {
        let guard = self.db.lock().map_err(|_| "DB lock poisoned".to_string())?;
        let db = guard
            .as_ref()
            .ok_or_else(|| "Database unavailable".to_string())?;
        f(db)
    }

    /// Standard DB write helper for non-hot paths.
    ///
    /// Keep lock scope short: gather -> compute -> persist.
    pub fn with_db_write<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&crate::db::ActionDb) -> Result<T, String>,
    {
        let guard = self.db.lock().map_err(|_| "DB lock poisoned".to_string())?;
        let db = guard
            .as_ref()
            .ok_or_else(|| "Database unavailable".to_string())?;
        f(db)
    }

    /// Save execution history to disk
    fn save_execution_history(&self) -> Result<(), String> {
        let history = self
            .execution_history
            .lock()
            .map_err(|_| "Lock poisoned")?
            .clone();

        let path = get_state_dir()?.join("execution_history.json");
        let content = serde_json::to_string_pretty(&history)
            .map_err(|e| format!("Serialize error: {}", e))?;

        crate::util::atomic_write_str(&path, &content)
            .map_err(|e| format!("Write error: {}", e))?;

        Ok(())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Run startup workspace sync/indexing in the background.
///
/// Uses a fresh DB connection to avoid blocking UI reads on the global DB mutex
/// during startup.
pub fn run_startup_sync(state: &AppState) {
    let config = match state.config.read().ok().and_then(|g| g.clone()) {
        Some(cfg) => cfg,
        None => {
            log::debug!("Startup sync skipped: no config loaded");
            return;
        }
    };

    let workspace = std::path::Path::new(&config.workspace_path);
    if !workspace.exists() {
        log::debug!(
            "Startup sync skipped: workspace does not exist ({})",
            workspace.display()
        );
        return;
    }

    // Refresh managed workspace files if version changed (I275)
    if let Err(e) = crate::util::write_managed_workspace_files(workspace) {
        log::warn!("Startup sync: failed to write managed workspace files: {}", e);
    }

    let db = match crate::db::ActionDb::open() {
        Ok(db) => db,
        Err(e) => {
            log::warn!("Startup sync skipped: failed to open DB: {}", e);
            return;
        }
    };

    match crate::people::sync_people_from_workspace(workspace, &db, &config.resolved_user_domains())
    {
        Ok(n) if n > 0 => log::info!("Startup sync: synced {} people from workspace", n),
        Ok(_) => {}
        Err(e) => log::warn!("Startup sync: people sync failed: {}", e),
    }

    match crate::accounts::sync_accounts_from_workspace(workspace, &db) {
        Ok(n) if n > 0 => log::info!("Startup sync: synced {} accounts from workspace", n),
        Ok(_) => {}
        Err(e) => log::warn!("Startup sync: accounts sync failed: {}", e),
    }

    match crate::projects::sync_projects_from_workspace(workspace, &db) {
        Ok(n) if n > 0 => log::info!("Startup sync: synced {} projects from workspace", n),
        Ok(_) => {}
        Err(e) => log::warn!("Startup sync: projects sync failed: {}", e),
    }

    match crate::accounts::sync_all_content_indexes(workspace, &db) {
        Ok(n) if n > 0 => log::info!("Startup sync: indexed {} content files", n),
        Ok(_) => {}
        Err(e) => log::warn!("Startup sync: content index sync failed: {}", e),
    }

    if config.embeddings.enabled {
        match db.get_entities_with_content() {
            Ok(entities) => {
                for (entity_id, entity_type) in entities {
                    state
                        .embedding_queue
                        .enqueue(crate::processor::embeddings::EmbeddingRequest {
                            entity_id,
                            entity_type,
                            requested_at: Instant::now(),
                        });
                }
            }
            Err(e) => log::warn!("Startup sync: failed to queue embedding work: {}", e),
        }
    }

    // One-off: import master-task-list.md into SQLite (DELETE after confirmed)
    import_master_task_list(workspace, &db);
}

/// Recover from an unclean dev-mode exit (app quit without restore_live).
///
/// If `config.json.dev-backup` exists, the app was in dev mode when it last
/// closed. Restore the backup so the live DB doesn't get polluted with mock
/// data during startup sync.
fn recover_from_unclean_dev_exit() {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };
    let config = home.join(".dailyos").join("config.json");
    let backup = config.with_extension("json.dev-backup");

    if backup.exists() {
        log::warn!("Detected unclean dev-mode exit — restoring live config from backup");
        match fs::copy(&backup, &config) {
            Ok(_) => {
                let _ = fs::remove_file(&backup);
                log::info!("Live config restored successfully");
            }
            Err(e) => {
                log::error!("Failed to restore config from dev backup: {}", e);
            }
        }
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
    let mut guard = state.config.write().map_err(|_| "Lock poisoned")?;

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
                quill: crate::quill::QuillConfig::default(),
                gravatar: crate::gravatar::GravatarConfig::default(),
                features: std::collections::HashMap::new(),
                user_domain: None,
                user_domains: None,
                user_name: None,
                user_company: None,
                user_title: None,
                user_focus: None,
                personality: "professional".to_string(),
                developer_mode: false,
                ai_models: crate::types::AiModelConfig::default(),
                embeddings: crate::types::EmbeddingConfig::default(),
                internal_team_setup_completed: false,
                internal_team_setup_version: 0,
                internal_org_account_id: None,
                hygiene_scan_interval_hours: 4,
                hygiene_ai_budget: 10,
                hygiene_pre_meeting_hours: 12,
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

    // Write to disk (I64: atomic write to prevent corruption on crash)
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    crate::util::atomic_write_str(&path, &content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    // Update in-memory state
    *guard = Some(config.clone());

    Ok(config)
}

/// Initialize workspace directory structure.
///
/// Always creates: _today/, _today/data/, _inbox/, _archive/
/// Conditionally creates: Accounts/ if entity_mode is "account" or "both"
/// Conditionally creates: Projects/ if entity_mode is "project" or "both"
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

    // Conditionally create Projects/
    if entity_mode == "project" || entity_mode == "both" {
        let projects_dir = path.join("Projects");
        if !projects_dir.exists() {
            fs::create_dir_all(&projects_dir)
                .map_err(|e| format!("Failed to create Projects: {}", e))?;
        }
    }

    // Write managed CLAUDE.md + .claude/settings.json (I275)
    crate::util::write_managed_workspace_files(path)?;

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
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read transcript records: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse transcript records: {}", e))
}

/// Save transcript records to `~/.dailyos/transcript_records.json`.
pub fn save_transcript_records(records: &HashMap<String, TranscriptRecord>) -> Result<(), String> {
    let path = get_state_dir()?.join("transcript_records.json");
    let content =
        serde_json::to_string_pretty(records).map_err(|e| format!("Serialize error: {}", e))?;
    crate::util::atomic_write_str(&path, &content).map_err(|e| format!("Write error: {}", e))?;
    Ok(())
}

/// Reload configuration from disk
pub fn reload_config(state: &AppState) -> Result<Config, String> {
    let config = load_config()?;
    let mut guard = state.config.write().map_err(|_| "Lock poisoned")?;
    *guard = Some(config.clone());
    Ok(config)
}

/// Get the legacy Google token file path (used for non-macOS storage and migration).
pub fn google_token_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    home.join(".dailyos").join("google").join("token.json")
}

/// Detect existing Google authentication from the configured token store.
pub fn detect_google_auth() -> GoogleAuthStatus {
    if let Some(email) = crate::google_api::token_store::peek_account_email() {
        return GoogleAuthStatus::Authenticated { email };
    }

    // Probe load for malformed payload cases that should surface as expired.
    match crate::google_api::load_token() {
        Ok(token) => GoogleAuthStatus::Authenticated {
            email: token.account.unwrap_or_else(|| "connected".to_string()),
        },
        Err(crate::google_api::GoogleApiError::TokenNotFound(_)) => GoogleAuthStatus::NotConfigured,
        Err(_) => GoogleAuthStatus::TokenExpired,
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
        error_phase: None,
        can_retry: None,
        trigger,
    }
}

/// One-off import of master-task-list.md into SQLite actions table.
/// Reads the multi-line format (checkbox + indented metadata sub-lines),
/// resolves account names to SQLite IDs, and upserts as source_type="import".
/// Creates a .imported marker so it never runs twice.
/// DELETE THIS FUNCTION after confirming the import worked.
fn import_master_task_list(workspace: &Path, db: &crate::db::ActionDb) {
    let task_file = workspace.join("_today/tasks/master-task-list.md");
    let marker = workspace.join("_today/tasks/.master-task-list-imported");

    if !task_file.exists() || marker.exists() {
        return;
    }

    let content = match fs::read_to_string(&task_file) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Could not read master-task-list.md: {}", e);
            return;
        }
    };

    // Build account name → id lookup (case-insensitive)
    let accounts = db.get_all_accounts().unwrap_or_default();
    let account_lookup: HashMap<String, String> = accounts
        .iter()
        .map(|a| (a.name.to_lowercase(), a.id.clone()))
        .collect();

    let now = Utc::now().to_rfc3339();
    let mut imported = 0;
    let mut skipped_completed = 0;
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Match checkbox lines
        let (is_completed, raw_title) = if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            (false, rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix("- [x] ") {
            (true, rest.trim())
        } else {
            i += 1;
            continue;
        };

        // Parse title: strip **bold** markers and `backtick-id`
        let title = raw_title.replace("**", "").trim().to_string();
        // Extract the backtick ID if present
        let (clean_title, task_id) = if let Some(bt_start) = title.rfind('`') {
            let before = &title[..title[..bt_start].rfind('`').unwrap_or(bt_start)];
            let id_part = title[before.len()..].trim().trim_matches('`').trim();
            (before.trim().to_string(), id_part.to_string())
        } else {
            (title.clone(), String::new())
        };

        // Read indented sub-lines for metadata
        let mut account_raw: Option<String> = None;
        let mut due_date: Option<String> = None;
        let mut priority = "P2".to_string();
        let mut context: Option<String> = None;
        let mut source: Option<String> = None;
        let mut owner: Option<String> = None;

        i += 1;
        while i < lines.len() {
            let sub = lines[i].trim();
            if !sub.starts_with("- ") || sub.starts_with("- [ ]") || sub.starts_with("- [x]") {
                break;
            }
            let sub_content = sub.strip_prefix("- ").unwrap_or(sub);

            if let Some(v) = sub_content.strip_prefix("Account:") {
                account_raw = Some(v.trim().to_string());
            } else if let Some(v) = sub_content.strip_prefix("Due:") {
                // Extract YYYY-MM-DD from due text like "2026-01-31 (book travel by mid-Feb)"
                let due_text = v.trim();
                if due_text.len() >= 10 {
                    let date_part = &due_text[..10];
                    if date_part.chars().filter(|c| *c == '-').count() == 2 && date_part.len() == 10
                    {
                        due_date = Some(date_part.to_string());
                    }
                }
            } else if let Some(v) = sub_content.strip_prefix("Priority:") {
                priority = v.trim().to_string();
            } else if let Some(v) = sub_content.strip_prefix("Context:") {
                context = Some(v.trim().to_string());
            } else if let Some(v) = sub_content.strip_prefix("Source:") {
                source = Some(v.trim().to_string());
            } else if let Some(v) = sub_content.strip_prefix("Owner:") {
                owner = Some(v.trim().to_string());
            }
            // Skip: Completed, Outcome, Contacts, Note, Status, Area, Project
            i += 1;
        }

        if is_completed {
            skipped_completed += 1;
            continue;
        }

        if clean_title.is_empty() {
            continue;
        }

        // Resolve account name to SQLite id
        // Handle "Cox / Corporate-Services-B2B" → try full name, then parent
        let account_id = account_raw.as_ref().and_then(|name| {
            let lower = name.to_lowercase();
            // Try exact match first
            if let Some(id) = account_lookup.get(&lower) {
                return Some(id.clone());
            }
            // Try parent (before " / ")
            if let Some(parent) = lower.split(" / ").next() {
                if let Some(id) = account_lookup.get(parent.trim()) {
                    return Some(id.clone());
                }
            }
            // Try child (after " / ")
            if let Some(child) = lower.split(" / ").nth(1) {
                if let Some(id) = account_lookup.get(child.trim()) {
                    return Some(id.clone());
                }
            }
            None
        });

        let action_id = if !task_id.is_empty() {
            format!("import-{}", task_id)
        } else {
            format!("import-{}", crate::util::slugify(&clean_title))
        };

        // Build context with owner + source if available
        let full_context = {
            let mut parts = Vec::new();
            if let Some(ref o) = owner {
                parts.push(format!("Owner: {}", o));
            }
            if let Some(ref s) = source {
                parts.push(format!("Source: {}", s));
            }
            if let Some(ref c) = context {
                parts.push(c.clone());
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(". "))
            }
        };

        let action = crate::db::DbAction {
            id: action_id,
            title: clean_title,
            priority,
            status: "pending".to_string(),
            created_at: now.clone(),
            due_date,
            completed_at: None,
            account_id,
            project_id: None,
            source_type: Some("import".to_string()),
            source_id: if !task_id.is_empty() {
                Some(task_id)
            } else {
                None
            },
            source_label: Some("master-task-list.md".to_string()),
            context: full_context,
            waiting_on: None,
            updated_at: now.clone(),
            person_id: None,
        };

        if db.upsert_action_if_not_completed(&action).is_ok() {
            imported += 1;
        }
    }

    // Write marker so this never runs again
    let _ = fs::write(
        &marker,
        format!(
            "Imported {} actions, skipped {} completed on {}",
            imported, skipped_completed, now
        ),
    );
    log::info!(
        "master-task-list.md import complete: {} actions imported, {} completed skipped",
        imported,
        skipped_completed
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_db_try_read_busy_returns_busy() {
        let state = AppState::new();
        let _held = state.db.lock().expect("lock db");

        match state.with_db_try_read(|_| 1_u8) {
            DbTryRead::Busy => {}
            _ => panic!("expected busy try-read result"),
        }
    }

    #[test]
    fn test_with_db_read_unavailable_maps_error() {
        let state = AppState::new();
        {
            let mut guard = state.db.lock().expect("lock db");
            *guard = None;
        }
        let err = state
            .with_db_read(|_| Ok::<u8, String>(1))
            .expect_err("db should be unavailable");
        assert_eq!(err, "Database unavailable");
    }
}

use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};

use crate::types::{
    CalendarEvent, Config, ExecutionRecord, ExecutionTrigger, GoogleAuthStatus, TranscriptRecord,
    WorkflowId, WorkflowStatus,
};

/// Maximum number of execution records to keep in memory
const MAX_HISTORY_SIZE: usize = 100;

/// Cached merged signal/email classification config for the active preset (DOS-176).
///
/// Computed once at `set_role` time by merging base lists with preset-specific
/// overrides (max-wins for duplicate keywords, additive for new ones).
/// Stored here so `score_item` / `score_single_email` can use it without
/// reaching into global state.
#[derive(Debug, Clone, Default)]
pub struct MergedSignalConfig {
    /// Merged (keyword, weight) pairs: base list + preset additions.
    /// Duplicates resolved with max-wins on weight.
    pub signal_keywords: Vec<(String, f64)>,
    /// Merged email boost signal types: base list + preset additions.
    pub email_signal_types: Vec<String>,
    /// Merged email high-priority subject keywords: base list + preset additions.
    pub email_priority_keywords: Vec<String>,
}

/// Daily AI call budget for proactive hygiene (I146 — ADR-0058).
pub struct HygieneBudget {
    pub daily_ai_calls: AtomicU32,
    pub daily_limit: u32,
    /// ISO date string (YYYY-MM-DD) of last reset. Resets at midnight local time.
    pub last_reset: Mutex<String>,
}

impl Default for HygieneBudget {
    fn default() -> Self {
        Self::unlimited()
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

    /// Create an effectively unlimited hygiene budget.
    ///
    /// DOS-279: The call-count hygiene budget is replaced by the token budget
    /// enforced at PTY call time. Hygiene uses `unlimited()` so it can enqueue
    /// freely; the PTY gate handles actual enforcement.
    pub fn unlimited() -> Self {
        Self::new(u32::MAX)
    }

    /// Check if budget allows another AI call, resetting counter if day changed.
    pub fn try_consume(&self) -> bool {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        {
            let mut last = self.last_reset.lock();
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

/// Hygiene subsystem state (I404).
pub struct HygieneState {
    pub report: Mutex<Option<crate::hygiene::HygieneReport>>,
    pub scan_running: AtomicBool,
    pub last_scan_at: Mutex<Option<String>>,
    pub next_scan_at: Mutex<Option<String>>,
    pub budget: HygieneBudget,
    pub full_orphan_scan_done: AtomicBool,
}

/// Capture subsystem state (I404).
pub struct CaptureState {
    pub dismissed: Mutex<std::collections::HashSet<String>>,
    pub captured: Mutex<std::collections::HashSet<String>>,
    pub transcript_processed: Mutex<HashMap<String, TranscriptRecord>>,
}

/// Calendar subsystem state (I404).
pub struct CalendarState {
    pub google_auth: Mutex<GoogleAuthStatus>,
    pub events: RwLock<Vec<CalendarEvent>>,
    pub week_cache: RwLock<Option<(Vec<CalendarEvent>, Instant)>>,
}

/// Workflow execution state (I404).
pub struct WorkflowState {
    pub status: RwLock<HashMap<WorkflowId, WorkflowStatus>>,
    pub history: Mutex<Vec<ExecutionRecord>>,
    pub last_scheduled_run: RwLock<HashMap<WorkflowId, DateTime<Utc>>>,
}

/// Integration poller wake signals (I405).
pub struct IntegrationState {
    pub enrichment_wake: Arc<tokio::sync::Notify>,
    pub quill_poller_wake: Arc<tokio::sync::Notify>,
    pub linear_poller_wake: Arc<tokio::sync::Notify>,
    pub email_poller_wake: Arc<tokio::sync::Notify>,
    pub granola_poller_wake: Arc<tokio::sync::Notify>,
    /// Wake signal for the Google Drive poller (I426).
    pub drive_poller_wake: Arc<tokio::sync::Notify>,
    /// Wake signal for the intelligence queue processor.
    pub intel_queue_wake: Arc<tokio::sync::Notify>,
    /// Wake signal for the meeting prep queue processor.
    pub prep_queue_wake: Arc<tokio::sync::Notify>,
    /// Wake signal for the embedding queue processor.
    pub embedding_queue_wake: Arc<tokio::sync::Notify>,
}

/// Consolidated app lock state (I610).
///
/// All lock-related fields behind a single `Mutex` so lock/unlock/check
/// operations are atomic -- no inconsistent reads between `is_locked` and
/// `failed_unlock_count`.
pub struct AppLockState {
    pub is_locked: bool,
    pub last_activity: Instant,
    pub last_failed_unlock: Option<Instant>,
    pub failed_unlock_count: u32,
}

impl Default for AppLockState {
    fn default() -> Self {
        Self {
            is_locked: false,
            last_activity: Instant::now(),
            last_failed_unlock: None,
            failed_unlock_count: 0,
        }
    }
}

/// Signal bus state (I405).
pub struct SignalState {
    pub engine: Arc<crate::signals::propagation::PropagationEngine>,
    pub prep_invalidation_queue: Arc<Mutex<Vec<String>>>,
}

/// Startup database recovery status for migration/integrity failures (I539).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseRecoveryStatus {
    pub required: bool,
    pub reason: String,
    pub detail: String,
    pub db_path: String,
}

impl DatabaseRecoveryStatus {
    pub fn not_required() -> Self {
        Self {
            required: false,
            reason: String::new(),
            detail: String::new(),
            db_path: String::new(),
        }
    }

    pub fn required(reason: impl Into<String>, detail: impl Into<String>) -> Self {
        let db_path = crate::db::ActionDb::db_path_public()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        Self {
            required: true,
            reason: reason.into(),
            detail: detail.into(),
            db_path,
        }
    }
}

/// Typed resource permits for concurrent background work (I565).
///
/// Replaces the single `heavy_work_semaphore` with per-resource permits so
/// independent workloads (e.g. embedding inference vs. Gmail fetch) can run
/// concurrently while still serializing within each resource class.
pub struct ResourcePermits {
    /// PTY subprocess — Claude Code intelligence enrichment.
    pub pty: Arc<tokio::sync::Semaphore>,
    /// User-triggered operations that should not queue behind background PTY work.
    pub user_initiated: Arc<tokio::sync::Semaphore>,
    /// Embedding model inference — CPU-intensive.
    pub embeddings: Arc<tokio::sync::Semaphore>,
    /// Gmail API pipeline — email fetch + enrichment.
    pub email: Arc<tokio::sync::Semaphore>,
    /// Daily briefing orchestration pipeline.
    pub orchestration: Arc<tokio::sync::Semaphore>,
}

impl Default for ResourcePermits {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourcePermits {
    pub fn new() -> Self {
        Self {
            pty: Arc::new(tokio::sync::Semaphore::new(1)),
            user_initiated: Arc::new(tokio::sync::Semaphore::new(1)),
            embeddings: Arc::new(tokio::sync::Semaphore::new(1)),
            email: Arc::new(tokio::sync::Semaphore::new(1)),
            orchestration: Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }
}

/// DOS-259 (W2-B cycle 3): atomic state bundle for context-mode switches.
///
/// Combines the three Arcs that all change together when the user
/// switches between Local and Glean modes:
///   - `context_provider`: ADR-0095 entity-context source.
///   - `intelligence_provider`: ADR-0091 trait Arc for `IntelligenceProvider`.
///   - `glean_intelligence_provider`: concrete Glean Arc bridge for callers
///     still on Glean-specific helpers (migrates when W3-A lands AbilityContext).
///
/// All three move together inside one `RwLock` so a write is atomic from
/// the reader's perspective. Multi-field reads route through
/// `AppState::context_snapshot()` so callers cannot interleave with a
/// writer mid-transition.
pub struct ContextProviderBundle {
    pub context_provider: Arc<dyn crate::context_provider::ContextProvider>,
    pub intelligence_provider:
        Option<Arc<dyn crate::intelligence::provider::IntelligenceProvider + Send + Sync>>,
    pub glean_intelligence_provider:
        Option<Arc<crate::intelligence::glean_provider::GleanIntelligenceProvider>>,
}

/// DOS-259 (W2-B cycle 3): coherent read snapshot of the context state.
///
/// Callers that need to make a routing decision based on multiple fields
/// (e.g., `is_remote()` AND the trait `Arc`) MUST use this snapshot
/// instead of reading the individual getters separately — that pattern
/// was the L2 codex finding #1 race. A snapshot is captured under one
/// read-lock acquisition; the caller then reasons against an immutable
/// view.
#[derive(Clone)]
pub struct ContextSnapshot {
    pub context_provider: Arc<dyn crate::context_provider::ContextProvider>,
    pub intelligence_provider:
        Option<Arc<dyn crate::intelligence::provider::IntelligenceProvider + Send + Sync>>,
    pub glean_intelligence_provider:
        Option<Arc<crate::intelligence::glean_provider::GleanIntelligenceProvider>>,
}

impl ContextSnapshot {
    pub fn is_remote(&self) -> bool {
        self.context_provider.is_remote()
    }
    pub fn remote_endpoint(&self) -> Option<String> {
        self.context_provider.remote_endpoint().map(|s| s.to_string())
    }
    pub fn provider_name(&self) -> String {
        self.context_provider.provider_name().to_string()
    }
}

/// Application state managed by Tauri
pub struct AppState {
    pub config: RwLock<Option<Config>>,
    pub workflow: WorkflowState,
    /// Async database service with read/write separation. Initialized async
    /// in Tauri setup after `AppState::new()`. Use `db_read()` / `db_write()`
    /// for async code, or `with_db_read()` / `with_db_write()` for sync code.
    ///
    /// `RwLock<Option<>>` instead of `OnceCell` so dev mode can reinitialize
    /// the service to point at `dailyos-dev.db`.
    pub db_service: tokio::sync::RwLock<Option<std::sync::Arc<crate::db_service::DbService>>>,
    /// User activity monitor for throttling background work (I426).
    pub activity: Arc<crate::activity::ActivityMonitor>,
    /// Calendar subsystem state (I404).
    pub calendar: CalendarState,
    /// Capture subsystem state (I404).
    pub capture: CaptureState,
    /// Background intelligence enrichment queue (I132)
    pub intel_queue: Arc<crate::intel_queue::IntelligenceQueue>,
    /// DOS-228 Fix 2: Per-account debouncer for post-edit health recompute.
    /// Rapid edits (10 in 2s) coalesce into a single recompute that reflects
    /// the final committed state. Replaces the old synchronous recompute in
    /// `services::accounts::update_account_field`.
    pub health_recompute_debouncer:
        Arc<crate::services::health_debouncer::HealthRecomputeDebouncer>,
    /// Shared embedding model runtime (Sprint 26).
    pub embedding_model: Arc<crate::embeddings::EmbeddingModel>,
    /// Background embedding generation queue (Sprint 26).
    pub embedding_queue: Arc<crate::processor::embeddings::EmbeddingQueue>,
    /// DOS-209 (W2-A): production clock for ServiceContext injection.
    /// Concrete `SystemClock` so it is `Sized`; tests construct their
    /// own `FixedClock` and a separate `ServiceContext::test_live`.
    pub clock: crate::services::context::SystemClock,
    /// DOS-209 (W2-A): production RNG for ServiceContext injection.
    /// Concrete `SystemRng`; tests use `SeedableRng`.
    pub rng: crate::services::context::SystemRng,
    /// DOS-209 (W2-A): mode-aware external-client wrappers for
    /// ServiceContext injection. Live mode wraps configured clients;
    /// non-Live modes hold replay/fixture wrappers per ADR-0104 §3.4.
    pub external: crate::services::context::ExternalClients,
    /// Hygiene subsystem state (I404).
    pub hygiene: HygieneState,
    /// Stashed live workspace path before switching to dev mode (I298).
    /// `restore_live()` reads this back to return to the user's real workspace.
    pub pre_dev_workspace: Mutex<Option<String>>,
    /// Signal bus state (I405).
    pub signals: SignalState,
    /// Integration poller wake signals (I405).
    pub integrations: IntegrationState,
    /// App lock state consolidated into a single mutex (I610).
    pub lock_state: Mutex<AppLockState>,
    /// True if the encryption key was not found in the Keychain on startup (I462).
    /// When set, the frontend shows a recovery screen instead of normal UI.
    pub encryption_key_missing: AtomicBool,
    /// DB recovery state when migrations/schema integrity fail on startup (I539).
    pub database_recovery_status: Mutex<DatabaseRecoveryStatus>,
    /// Tamper-evident audit log for enterprise observability (I471).
    pub audit_log: Arc<Mutex<crate::audit_log::AuditLogger>>,
    /// Active role preset loaded from config (I309).
    pub active_preset: RwLock<Option<crate::presets::schema::RolePreset>>,
    /// Cached merged signal/email config for the active preset (DOS-176).
    /// Recomputed at `set_role` time and on startup preset load. Supersedes
    /// the earlier `merged_signal_keywords` field (DOS-178) by also caching
    /// email signal types and priority keywords alongside signal keywords.
    pub merged_signal_config: RwLock<MergedSignalConfig>,
    /// Background meeting prep queue for future meetings.
    pub meeting_prep_queue: Arc<crate::meeting_prep_queue::MeetingPrepQueue>,
    /// Typed resource permits for concurrent background work (I565).
    pub permits: ResourcePermits,
    /// DOS-259 (W2-B cycle 3, L6 2026-04-30): all three context-related
    /// Arcs live behind ONE `RwLock<ContextProviderBundle>` so a settings
    /// switch updates them atomically. Multi-step reads use
    /// `context_snapshot()` to get a coherent view per ADR-0091's "switch
    /// mid-queue takes effect on next dequeue" guarantee. The previous
    /// 3-RwLock layout was a real race surface flagged by L2 codex review.
    context_state: RwLock<ContextProviderBundle>,
    /// Shared app handle for service-layer Tauri event emission.
    app_handle: RwLock<Option<tauri::AppHandle>>,
}

/// Base signal keywords applicable to any role (generic, role-neutral).
///
/// CS-specific keywords (`churn`, `cancellation`, etc.) live in the CS preset's
/// `intelligence.signal_keywords` and are merged in at `set_role` time (DOS-176).
pub const BASE_SIGNAL_KEYWORDS: &[(&str, f64)] = &[
    ("renewal", 0.15),
    ("contract", 0.12),
    ("expansion", 0.12),
    ("escalation", 0.12),
    ("qbr", 0.10),
    ("order form", 0.10),
    ("deadline", 0.08),
    ("budget", 0.08),
    ("executive", 0.06),
];

/// Base email boost signal types applicable to any role (generic, role-neutral).
///
/// CS-specific types (`churn_risk`, `renewal_approaching`, `champion_risk`) live in
/// the CS preset's `intelligence.email_signal_types` (DOS-176).
pub const BASE_EMAIL_SIGNAL_TYPES: &[&str] = &[
    "engagement_warning",
    "escalation",
    "expansion_opportunity",
    "cadence_anomaly",
    "email_cadence_drop",
    "project_health_warning",
];

/// Build a `MergedSignalConfig` from a preset by merging its intelligence config
/// with the base lists. Called at `set_role` time and on startup (DOS-176).
///
/// Merge semantics:
/// - Additive: preset keywords appended to base list.
/// - Max-wins for duplicates: if preset keyword matches base keyword, higher weight wins.
/// - Email signal types and priority keywords are additive (deduped).
pub fn build_merged_signal_config(
    preset: &crate::presets::schema::RolePreset,
) -> MergedSignalConfig {
    // --- signal_keywords ---
    // Start with base list as (String, f64).
    let mut kw_map: std::collections::HashMap<String, f64> = BASE_SIGNAL_KEYWORDS
        .iter()
        .map(|&(k, w)| (k.to_string(), w))
        .collect();

    // Merge preset keywords: max-wins on duplicate keys.
    for sk in &preset.intelligence.signal_keywords {
        let entry = kw_map.entry(sk.keyword.clone()).or_insert(0.0);
        if sk.weight > *entry {
            *entry = sk.weight;
        }
    }

    // Produce deterministic order: base keywords first (in original order), then extras.
    let mut signal_keywords: Vec<(String, f64)> = BASE_SIGNAL_KEYWORDS
        .iter()
        .filter_map(|&(k, _)| {
            kw_map.remove(k).map(|w| (k.to_string(), w))
        })
        .collect();
    // Remaining are preset-only keywords (not in base).
    let mut extras: Vec<(String, f64)> = kw_map.into_iter().collect();
    extras.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    signal_keywords.extend(extras);

    // --- email_signal_types ---
    let mut email_signal_types: Vec<String> =
        BASE_EMAIL_SIGNAL_TYPES.iter().map(|s| s.to_string()).collect();
    for t in &preset.intelligence.email_signal_types {
        if !email_signal_types.iter().any(|e| e == t) {
            email_signal_types.push(t.clone());
        }
    }

    // --- email_priority_keywords ---
    // Merge both top-level emailPriorityKeywords and intelligence.emailPriorityKeywords.
    // Top-level field is kept for backward compat and also merged here.
    let mut email_priority_keywords: Vec<String> = Vec::new();
    for kw in preset.email_priority_keywords.iter().chain(preset.intelligence.email_priority_keywords.iter()) {
        let lower = kw.to_lowercase();
        if !email_priority_keywords.iter().any(|e: &String| e.to_lowercase() == lower) {
            email_priority_keywords.push(kw.clone());
        }
    }

    MergedSignalConfig {
        signal_keywords,
        email_signal_types,
        email_priority_keywords,
    }
}

fn recovery_status_from_db_error(err: &crate::db::DbError) -> DatabaseRecoveryStatus {
    match err {
        crate::db::DbError::Migration(message) => {
            DatabaseRecoveryStatus::required("migration_failed", message.clone())
        }
        crate::db::DbError::Sqlite(message) => {
            DatabaseRecoveryStatus::required("database_open_failed", message.to_string())
        }
        crate::db::DbError::Encryption(message) => {
            DatabaseRecoveryStatus::required("database_encryption_error", message.clone())
        }
        crate::db::DbError::CreateDir(message) => {
            DatabaseRecoveryStatus::required("database_path_error", message.to_string())
        }
        crate::db::DbError::HomeDirNotFound => {
            DatabaseRecoveryStatus::required("database_path_error", "Home directory not found")
        }
        crate::db::DbError::KeyMissing { .. } => DatabaseRecoveryStatus::not_required(),
        crate::db::DbError::InvalidArgument(message) => {
            DatabaseRecoveryStatus::required("internal_invalid_argument", message.clone())
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        // I298 recovery: if a dev-backup config exists, the app was quit during
        // dev mode without calling restore_live(). Restore the live config before
        // loading so startup sync doesn't import mock data into the live DB.
        recover_from_unclean_dev_exit();

        let mut config = load_config().ok();
        let history = load_execution_history().unwrap_or_default();

        // Initialize audit logger BEFORE DB open so key events can be logged (Option B).
        let audit_path = crate::audit_log::default_audit_log_path();
        let mut audit_logger = crate::audit_log::AuditLogger::new(audit_path);

        // Rotate old records on startup
        let (records_pruned, bytes_freed) = crate::audit_log::rotate_audit_log(&mut audit_logger);
        let _ = audit_logger.append(
            "system",
            "audit_log_rotated",
            serde_json::json!({
                "records_pruned": records_pruned,
                "bytes_freed": bytes_freed,
            }),
        );

        let mut encryption_key_missing = false;
        let mut database_recovery_status = DatabaseRecoveryStatus::not_required();
        // I609: Open DB for startup validation and context_mode reading only.
        // The connection is NOT stored in AppState -- all runtime DB access goes
        // through `db_service` (async) or `ActionDb::open()` (sync helpers).
        let startup_db = match crate::db::ActionDb::open() {
            Ok(db) => {
                // Distinguish key generation (fresh install) from access (existing DB)
                let event = if crate::db::encryption::was_key_generated() {
                    "db_key_generated"
                } else {
                    "db_key_accessed"
                };
                let _ = audit_logger.append(
                    "security",
                    event,
                    serde_json::json!({"db_encrypted": true}),
                );

                // Log migration events if a plaintext→encrypted migration happened
                if crate::db::encryption::was_migration_performed() {
                    let _ = audit_logger.append(
                        "security",
                        "db_migration_started",
                        serde_json::json!({"migration_type": "plaintext_to_encrypted"}),
                    );
                    let _ = audit_logger.append(
                        "security",
                        "db_migration_completed",
                        serde_json::json!({"migration_type": "plaintext_to_encrypted"}),
                    );
                }
                Some(db)
            }
            Err(crate::db::DbError::KeyMissing { ref db_path }) => {
                log::error!(
                    "Encryption key missing for database at {db_path}. \
                     Showing recovery screen."
                );
                let _ = audit_logger.append(
                    "security",
                    "db_key_missing",
                    serde_json::json!({"recovery_screen": true}),
                );
                encryption_key_missing = true;
                None
            }
            Err(e) => {
                log::warn!("Failed to open actions database: {e}. DB features disabled.");
                let status = recovery_status_from_db_error(&e);
                if status.required {
                    let _ = audit_logger.append(
                        "security",
                        "db_recovery_required",
                        serde_json::json!({
                            "reason": status.reason.clone(),
                            "detail": status.detail.clone(),
                        }),
                    );
                    database_recovery_status = status;
                }
                None
            }
        };

        // Detect existing Google token on startup
        let google_auth = detect_google_auth();
        if let Some(cfg) = config.as_mut() {
            if reconcile_google_enabled_flag(cfg, &google_auth) {
                let persist_result = serde_json::to_string_pretty(cfg)
                    .map_err(|e| format!("Failed to serialize config: {}", e))
                    .and_then(|content| {
                        let path = config_path()?;
                        crate::util::atomic_write_str(&path, &content)
                            .map_err(|e| format!("Failed to write config: {}", e))
                    });

                match persist_result {
                    Ok(()) => {
                        log::info!(
                            "Startup repair: enabled Google polling for authenticated account"
                        );
                        let _ = audit_logger.append(
                            "config",
                            "google_enabled_repaired",
                            serde_json::json!({"trigger": "startup_auth_detected"}),
                        );
                    }
                    Err(e) => {
                        log::warn!(
                            "Startup repair: enabled Google polling in memory but failed to persist config: {}",
                            e
                        );
                    }
                }
            }
        }

        // Load transcript records from disk
        let transcript_processed = load_transcript_records().unwrap_or_default();

        // Deprecated: hygiene call-count budget replaced by token budget (DOS-279).
        let _hygiene_budget_limit = 0u32;

        // DOS-279: Sync the daily AI token budget from config to KV store so
        // the preflight gate (running in a sync context) can read it without
        // going through AppState.
        if let (Some(cfg), Some(db)) = (config.as_ref(), startup_db.as_ref()) {
            crate::pty::sync_budget_config_to_kv(db, cfg.daily_ai_token_budget);
        }

        // I309: Load active role preset from config
        let active_preset = config.as_ref().and_then(|c| {
            if let Some(ref path) = c.custom_preset_path {
                crate::presets::loader::load_custom_preset(std::path::Path::new(path)).ok()
            } else {
                crate::presets::loader::load_preset(&c.role).ok()
            }
        });
        // DOS-176: Compute merged signal config from the startup preset.
        let startup_merged_config = active_preset
            .as_ref()
            .map(build_merged_signal_config)
            .unwrap_or_default();

        // Build the prep invalidation queue and signal engine together so
        // the engine can push invalidated meeting IDs into the shared queue.
        let prep_queue = Arc::new(Mutex::new(Vec::new()));
        // Log app_started event
        let _ = audit_logger.append(
            "system",
            "app_started",
            serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "db_encrypted": !encryption_key_missing && startup_db.is_some(),
                "db_recovery_required": database_recovery_status.required,
            }),
        );
        let audit_log = Arc::new(Mutex::new(audit_logger));

        // Initialize context provider (ADR-0095).
        // Read context_mode from DB if available, else default to Local.
        let embedding_model = Arc::new(crate::embeddings::EmbeddingModel::new());
        let workspace_path = config
            .as_ref()
            .map(|c| std::path::PathBuf::from(&c.workspace_path))
            .unwrap_or_default();

        let context_mode = startup_db.as_ref().and_then(|db| {
            db.conn_ref()
                .query_row(
                    "SELECT mode_json FROM context_mode_config WHERE id = 1",
                    [],
                    |row| row.get::<_, Option<String>>(0),
                )
                .ok()
                .flatten()
                .and_then(|json| {
                    serde_json::from_str::<crate::context_provider::ContextMode>(&json).ok()
                })
        });

        let local_provider = crate::context_provider::local::LocalContextProvider::new(
            workspace_path.clone(),
            Arc::clone(&embedding_model),
        );

        // DOS-259 (W2-B): If Glean mode is configured, also seed the
        // AppState-owned `IntelligenceProvider` Arc per ADR-0091 so early
        // callers (intel_queue, services::intelligence) can route through the
        // trait without inline `GleanIntelligenceProvider::new(endpoint)`.
        let mut intelligence_provider: Option<
            Arc<dyn crate::intelligence::provider::IntelligenceProvider + Send + Sync>,
        > = None;
        let mut glean_intelligence_provider: Option<
            Arc<crate::intelligence::glean_provider::GleanIntelligenceProvider>,
        > = None;

        let context_provider: Arc<dyn crate::context_provider::ContextProvider> = match context_mode
        {
            Some(crate::context_provider::ContextMode::Glean { endpoint }) => {
                log::info!("Context mode: Glean endpoint={}", endpoint);
                let cache = Arc::new(crate::context_provider::cache::GleanCache::new());
                let glean_provider_arc = Arc::new(
                    crate::intelligence::glean_provider::GleanIntelligenceProvider::new(&endpoint),
                );
                glean_intelligence_provider = Some(Arc::clone(&glean_provider_arc));
                intelligence_provider = Some(glean_provider_arc
                    as Arc<
                        dyn crate::intelligence::provider::IntelligenceProvider + Send + Sync,
                    >);
                Arc::new(crate::context_provider::glean::GleanContextProvider::new(
                    endpoint,
                    cache,
                    local_provider,
                ))
            }
            _ => {
                // Belt-and-suspenders: if mode is Local but Keychain has a Glean token,
                // log a warning. With auto-set on auth (Step 3), this should be rare.
                if crate::glean::token_store::load_token().is_ok() {
                    log::warn!(
                        "Context mode is Local but Glean token found in Keychain. \
                         This may indicate a previous auth that didn't save the mode. \
                         Connect Glean in Settings to activate Glean mode."
                    );
                }
                log::info!("Context mode: Local");
                Arc::new(local_provider)
            }
        };

        let intel_queue_arc = Arc::new(crate::intel_queue::IntelligenceQueue::new());
        let mut signal_engine = crate::signals::propagation::default_engine();
        signal_engine.set_prep_queue(Arc::clone(&prep_queue));
        // I385: Wire intel_queue so propagated cross-entity signals trigger enrichment
        signal_engine.set_intel_queue(Arc::clone(&intel_queue_arc));

        Self {
            config: RwLock::new(config),
            workflow: WorkflowState {
                status: RwLock::new(HashMap::new()),
                history: Mutex::new(history),
                last_scheduled_run: RwLock::new(HashMap::new()),
            },
            db_service: tokio::sync::RwLock::new(None),
            activity: Arc::new(crate::activity::ActivityMonitor::new()),
            calendar: CalendarState {
                google_auth: Mutex::new(google_auth),
                events: RwLock::new(Vec::new()),
                week_cache: RwLock::new(None),
            },
            capture: CaptureState {
                dismissed: Mutex::new(std::collections::HashSet::new()),
                captured: Mutex::new(std::collections::HashSet::new()),
                transcript_processed: Mutex::new(transcript_processed),
            },
            intel_queue: intel_queue_arc,
            health_recompute_debouncer: Arc::new(
                crate::services::health_debouncer::HealthRecomputeDebouncer::new(),
            ),
            embedding_model,
            embedding_queue: Arc::new(crate::processor::embeddings::EmbeddingQueue::new()),
            clock: crate::services::context::SystemClock,
            rng: crate::services::context::SystemRng,
            external: crate::services::context::ExternalClients::default(),
            hygiene: HygieneState {
                report: Mutex::new(None),
                scan_running: AtomicBool::new(false),
                last_scan_at: Mutex::new(None),
                next_scan_at: Mutex::new(None),
                // DOS-279: Hygiene call-count budget is deprecated. Use unlimited
                // so hygiene can enqueue freely; token budget enforced at PTY call time.
                budget: HygieneBudget::unlimited(),
                full_orphan_scan_done: AtomicBool::new(false),
            },
            pre_dev_workspace: Mutex::new(None),
            signals: SignalState {
                engine: Arc::new(signal_engine),
                prep_invalidation_queue: prep_queue,
            },
            integrations: IntegrationState {
                enrichment_wake: Arc::new(tokio::sync::Notify::new()),
                quill_poller_wake: Arc::new(tokio::sync::Notify::new()),
                linear_poller_wake: Arc::new(tokio::sync::Notify::new()),
                email_poller_wake: Arc::new(tokio::sync::Notify::new()),
                granola_poller_wake: Arc::new(tokio::sync::Notify::new()),
                drive_poller_wake: Arc::new(tokio::sync::Notify::new()),
                intel_queue_wake: Arc::new(tokio::sync::Notify::new()),
                prep_queue_wake: Arc::new(tokio::sync::Notify::new()),
                embedding_queue_wake: Arc::new(tokio::sync::Notify::new()),
            },
            lock_state: Mutex::new(AppLockState::default()),
            encryption_key_missing: AtomicBool::new(encryption_key_missing),
            database_recovery_status: Mutex::new(database_recovery_status),
            audit_log,
            active_preset: RwLock::new(active_preset),
            merged_signal_config: RwLock::new(startup_merged_config),
            meeting_prep_queue: Arc::new(crate::meeting_prep_queue::MeetingPrepQueue::new()),
            permits: ResourcePermits::new(),
            context_state: RwLock::new(ContextProviderBundle {
                context_provider,
                intelligence_provider,
                glean_intelligence_provider,
            }),
            app_handle: RwLock::new(None),
        }
    }

    /// Get a snapshot of the merged signal config for the active preset (DOS-176).
    ///
    /// Returns the cached `MergedSignalConfig` — cheap clone, no recomputation.
    pub fn get_merged_signal_config(&self) -> MergedSignalConfig {
        self.merged_signal_config.read().clone()
    }

    /// Update the active preset and recompute the cached merged signal config (DOS-176).
    ///
    /// Called by the `set_role` command after loading the new preset so downstream
    /// callers (`score_item`, `score_single_email`, `boost_with_entity_context`) always
    /// get the current merged config without reaching into global state.
    pub fn set_active_preset(&self, preset: crate::presets::schema::RolePreset) {
        let merged = build_merged_signal_config(&preset);
        *self.active_preset.write() = Some(preset);
        *self.merged_signal_config.write() = merged;
    }

    /// I573: Read config (parking_lot — no poison possible).
    pub fn config_read_or_recover(
        &self,
    ) -> Result<parking_lot::RwLockReadGuard<'_, Option<Config>>, String> {
        Ok(self.config.read())
    }

    /// Get a snapshot of the current context provider (cheap Arc clone).
    ///
    /// **For multi-field reads (e.g., `is_remote()` + `intelligence_provider()`)
    /// use `context_snapshot()` instead** — separate getters can interleave
    /// with a writer mid-transition. This getter is safe in isolation.
    pub fn context_provider(&self) -> Arc<dyn crate::context_provider::ContextProvider> {
        Arc::clone(&self.context_state.read().context_provider)
    }

    /// DOS-259 (W2-B cycle 3, L6 2026-04-30): atomic snapshot of the
    /// context state. Captures all three Arcs under one read-lock
    /// acquisition so callers reasoning about routing (`is_remote()` AND
    /// the trait `Arc`) see a coherent view.
    ///
    /// **Migration sites must use this snapshot, not the individual
    /// getters.** The L2 codex review found the per-field-getter pattern
    /// races with `set_context_mode_atomic()` and could issue a remote
    /// Glean call after a Local switch.
    pub fn context_snapshot(&self) -> ContextSnapshot {
        let bundle = self.context_state.read();
        ContextSnapshot {
            context_provider: Arc::clone(&bundle.context_provider),
            intelligence_provider: bundle.intelligence_provider.clone(),
            glean_intelligence_provider: bundle.glean_intelligence_provider.clone(),
        }
    }

    /// Hot-swap ONLY the context provider (ADR-0095 dynamic mode switch).
    ///
    /// **DEPRECATED FOR PRODUCTION**: production settings flows must use
    /// `set_context_mode_atomic` (or `build_context_provider`, which now
    /// delegates to it). This single-field swap mutates only
    /// `context_provider` and leaves `intelligence_provider` /
    /// `glean_intelligence_provider` from a previous mode in place — that
    /// asymmetry was the L2 cycle-3 codex finding (concurrent
    /// `build_context_provider` + `swap_context_provider` interleaves
    /// would leave a torn bundle).
    ///
    /// Kept for tests that need to install a stub context provider
    /// without rebuilding the trait Arcs.
    pub fn swap_context_provider(&self, new: Arc<dyn crate::context_provider::ContextProvider>) {
        let mut guard = self.context_state.write();
        guard.context_provider = new;
        log::info!(
            "Context provider single-field-swapped to: {} (test-only path)",
            guard.context_provider.provider_name()
        );
    }

    /// DOS-259 (W2-B): get the configured remote `IntelligenceProvider`, if any.
    ///
    /// Per ADR-0091: read at call time so a swap mid-queue takes effect on
    /// the next dequeue. `None` means no remote provider is configured —
    /// callers route through PTY (`PtyClaudeCode` constructed per-call).
    pub fn intelligence_provider(
        &self,
    ) -> Option<Arc<dyn crate::intelligence::provider::IntelligenceProvider + Send + Sync>> {
        self.context_state.read().intelligence_provider.clone()
    }

    /// DOS-259 (W2-B): hot-swap ONLY the `IntelligenceProvider` Arc.
    ///
    /// **Prefer `set_context_mode_atomic`** for settings transitions —
    /// this single-field swap exists for tests and legacy paths.
    pub fn swap_intelligence_provider(
        &self,
        new: Option<Arc<dyn crate::intelligence::provider::IntelligenceProvider + Send + Sync>>,
    ) {
        let mut guard = self.context_state.write();
        guard.intelligence_provider = new;
    }

    /// DOS-259 (W2-B) bridge: get the concrete Glean provider Arc, if any.
    pub fn glean_intelligence_provider(
        &self,
    ) -> Option<Arc<crate::intelligence::glean_provider::GleanIntelligenceProvider>> {
        self.context_state.read().glean_intelligence_provider.clone()
    }

    /// DOS-259 (W2-B) bridge: hot-swap ONLY the concrete Glean provider Arc.
    /// **Prefer `set_context_mode_atomic`** — this single-field swap is for tests.
    pub fn swap_glean_intelligence_provider(
        &self,
        new: Option<Arc<crate::intelligence::glean_provider::GleanIntelligenceProvider>>,
    ) {
        let mut guard = self.context_state.write();
        guard.glean_intelligence_provider = new;
    }

    /// DOS-209 (W2-A): build a `Live` `ServiceContext` borrowing this
    /// `AppState`'s clock + rng + external clients. Tauri command
    /// handlers and background workers call this once per-call to get
    /// the `&ServiceContext` they pass into service mutators.
    ///
    /// ```ignore
    /// let ctx = state.live_service_context();
    /// services::accounts::create_account(&ctx, db, ...).await?;
    /// ```
    ///
    /// The returned `ServiceContext<'_>` borrows from `&self` so the
    /// caller must keep the `state` reference alive for the call's
    /// duration — which is the natural pattern for command handlers.
    pub fn live_service_context(&self) -> crate::services::context::ServiceContext<'_> {
        crate::services::context::ServiceContext::new_live(
            &self.clock,
            &self.rng,
            &self.external,
        )
    }

    /// DOS-259 (W2-B cycle 3, L6 2026-04-30): atomic context-mode transition.
    ///
    /// Updates `context_provider` + `intelligence_provider` + `glean_intelligence_provider`
    /// inside ONE write-lock critical section. Callers reading via
    /// `context_snapshot()` see either the pre-swap bundle or the post-swap
    /// bundle, never a mixed state. This closes the L2 codex finding #1
    /// race where a reader could observe `is_remote=true` + `None` Glean Arc.
    pub fn set_context_mode_atomic(&self, mode: &crate::context_provider::ContextMode) {
        let workspace_path = self
            .config_read_or_recover()
            .ok()
            .and_then(|c| {
                c.as_ref()
                    .map(|cfg| std::path::PathBuf::from(&cfg.workspace_path))
            })
            .unwrap_or_default();
        let local_provider = crate::context_provider::local::LocalContextProvider::new(
            workspace_path,
            Arc::clone(&self.embedding_model),
        );

        let new_bundle = match mode {
            crate::context_provider::ContextMode::Glean { endpoint } => {
                let cache = Arc::new(crate::context_provider::cache::GleanCache::new());
                let glean_arc = Arc::new(
                    crate::intelligence::glean_provider::GleanIntelligenceProvider::new(endpoint),
                );
                ContextProviderBundle {
                    context_provider: Arc::new(
                        crate::context_provider::glean::GleanContextProvider::new(
                            endpoint.clone(),
                            cache,
                            local_provider,
                        ),
                    ),
                    intelligence_provider: Some(Arc::clone(&glean_arc)
                        as Arc<
                            dyn crate::intelligence::provider::IntelligenceProvider + Send + Sync,
                        >),
                    glean_intelligence_provider: Some(glean_arc),
                }
            }
            crate::context_provider::ContextMode::Local => ContextProviderBundle {
                context_provider: Arc::new(local_provider),
                intelligence_provider: None,
                glean_intelligence_provider: None,
            },
        };

        let mut guard = self.context_state.write();
        *guard = new_bundle;
        log::info!(
            "Context mode atomic transition complete: {}",
            guard.context_provider.provider_name()
        );
    }

    pub fn set_app_handle(&self, handle: tauri::AppHandle) {
        let mut guard = self.app_handle.write();
        *guard = Some(handle);
    }

    pub fn app_handle(&self) -> Option<tauri::AppHandle> {
        self.app_handle.read().clone()
    }

    /// Build a context provider for the given mode, using this state's config and embedding model.
    ///
    /// DOS-259 (W2-B cycle 3): now routes through `set_context_mode_atomic`
    /// so the three context Arcs update under one write lock. Returns the
    /// new context provider Arc for callers that still want a handle, but
    /// AppState already holds it via the atomic swap — most callers can
    /// drop the return and read via `context_snapshot()`.
    pub fn build_context_provider(
        &self,
        mode: &crate::context_provider::ContextMode,
    ) -> Arc<dyn crate::context_provider::ContextProvider> {
        self.set_context_mode_atomic(mode);
        Arc::clone(&self.context_state.read().context_provider)
    }

    /// Get current status of a workflow
    pub fn get_workflow_status(&self, workflow: WorkflowId) -> WorkflowStatus {
        self.workflow
            .status
            .read()
            .get(&workflow)
            .cloned()
            .unwrap_or_default()
    }

    /// Update workflow status
    pub fn set_workflow_status(&self, workflow: WorkflowId, status: WorkflowStatus) {
        self.workflow.status.write().insert(workflow, status);
    }

    /// Add an execution record to history
    pub fn add_execution_record(&self, record: ExecutionRecord) {
        {
            let mut guard = self.workflow.history.lock();
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
        {
            let mut guard = self.workflow.history.lock();
            if let Some(record) = guard.iter_mut().find(|r| r.id == id) {
                f(record);
            }
        }

        // Persist to disk
        let _ = self.save_execution_history();
    }

    /// Get execution history
    pub fn get_execution_history(&self, limit: usize) -> Vec<ExecutionRecord> {
        self.workflow
            .history
            .lock()
            .iter()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Record when a scheduled run last occurred
    pub fn set_last_scheduled_run(&self, workflow: WorkflowId, time: DateTime<Utc>) {
        self.workflow
            .last_scheduled_run
            .write()
            .insert(workflow, time);
    }

    /// Get when a workflow last ran on schedule
    pub fn get_last_scheduled_run(&self, workflow: WorkflowId) -> Option<DateTime<Utc>> {
        self.workflow
            .last_scheduled_run
            .read()
            .get(&workflow)
            .cloned()
    }

    /// Sync DB read helper (I609).
    ///
    /// Opens a fresh `ActionDb` connection for the closure. Each call gets its
    /// own connection -- no shared mutex, no contention with async `db_service`.
    pub fn with_db_read<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&crate::db::ActionDb) -> Result<T, String>,
    {
        let db = crate::db::ActionDb::open().map_err(|e| format!("Database unavailable: {e}"))?;
        f(&db)
    }

    /// Sync DB write helper (I609).
    ///
    /// Opens a fresh `ActionDb` connection for the closure.
    pub fn with_db_write<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&crate::db::ActionDb) -> Result<T, String>,
    {
        let db = crate::db::ActionDb::open().map_err(|e| format!("Database unavailable: {e}"))?;
        f(&db)
    }

    /// Get startup database recovery status for the UI.
    pub fn get_database_recovery_status(&self) -> DatabaseRecoveryStatus {
        self.database_recovery_status.lock().clone()
    }

    /// Mark database recovery as required with a reason/detail payload.
    pub fn set_database_recovery_required(
        &self,
        reason: impl Into<String>,
        detail: impl Into<String>,
    ) {
        *self.database_recovery_status.lock() = DatabaseRecoveryStatus::required(reason, detail);
    }

    /// Clear database recovery-required state after successful restore.
    pub fn clear_database_recovery_required(&self) {
        *self.database_recovery_status.lock() = DatabaseRecoveryStatus::not_required();
    }

    /// True when startup should show DB recovery UI instead of the app.
    pub fn is_database_recovery_required(&self) -> bool {
        self.database_recovery_status.lock().required
    }

    // -------------------------------------------------------------------------
    // Async DbService helpers — use these for new/migrated command handlers.
    // -------------------------------------------------------------------------

    /// Initialize the unified DbService pool. Called from Tauri setup and on
    /// dev-mode transitions. Also installs the pool as the process-wide
    /// singleton so sync `ActionDb::open()` routes through it instead of
    /// opening a fresh `rusqlite::Connection` (which races the pool writer
    /// mid-commit under SQLCipher).
    pub async fn init_db_service(&self) -> Result<(), String> {
        let svc = crate::db_service::DbService::open()
            .await
            .map_err(|e| format!("Failed to open DbService: {e}"))?;
        crate::db_service::install_global(svc.clone());
        let mut guard = self.db_service.write().await;
        *guard = Some(svc);
        Ok(())
    }

    /// Reinitialize the DbService at the current DB path (live or dev).
    /// Called during dev mode enter/exit to switch the async connection pool.
    pub async fn reinit_db_service(&self) -> Result<(), String> {
        // Drop the old service first — both from state and the global.
        crate::db_service::uninstall_global();
        {
            let mut guard = self.db_service.write().await;
            *guard = None;
        }
        // Open a new service at the current path (respects DEV_DB_MODE)
        self.init_db_service().await
    }

    /// Run a read-only closure on a reader connection. Never blocks writes.
    ///
    /// The closure receives `&ActionDb` and runs on a dedicated OS thread —
    /// it never blocks the Tokio runtime.
    pub async fn db_read<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&crate::db::ActionDb) -> Result<T, String> + Send + 'static,
        T: Send + 'static,
    {
        // If the async service hasn't finished startup init yet, try to
        // initialize it on-demand before falling back.
        {
            let guard = self.db_service.read().await;
            if guard.is_none() {
                drop(guard);
                let _ = self.init_db_service().await;
            }
        }

        {
            let guard = self.db_service.read().await;
            if let Some(svc) = guard.as_ref() {
                return svc
                    .reader()
                    .call(move |conn| {
                        let db = crate::db::ActionDb::from_conn(conn);
                        Ok(f(db))
                    })
                    .await
                    .map_err(|e| format!("DB read error: {e}"))?;
            }
        }

        // Startup fallback: DbService not yet initialized. Open a fresh
        // connection directly (I609 -- no persistent sync handle).
        let db = crate::db::ActionDb::open()
            .map_err(|e| format!("Database unavailable: failed to open DB ({e})"))?;
        f(&db)
    }

    /// Run a mutating closure on the writer connection. Serialized -- only one
    /// write runs at a time, preventing WAL contention.
    pub async fn db_write<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&crate::db::ActionDb) -> Result<T, String> + Send + 'static,
        T: Send + 'static,
    {
        // If the async service hasn't finished startup init yet, try to
        // initialize it on-demand before falling back.
        {
            let guard = self.db_service.read().await;
            if guard.is_none() {
                drop(guard);
                let _ = self.init_db_service().await;
            }
        }

        {
            let guard = self.db_service.read().await;
            if let Some(svc) = guard.as_ref() {
                return svc
                    .writer()
                    .call(move |conn| {
                        let db = crate::db::ActionDb::from_conn(conn);
                        Ok(f(db))
                    })
                    .await
                    .map_err(|e| format!("DB write error: {e}"))?;
            }
        }

        // Startup fallback: DbService not yet initialized. Open a fresh
        // connection directly (I609 -- no persistent sync handle).
        let db = crate::db::ActionDb::open()
            .map_err(|e| format!("Database unavailable: failed to open DB ({e})"))?;
        f(&db)
    }

    /// Save execution history to disk
    fn save_execution_history(&self) -> Result<(), String> {
        let history = self.workflow.history.lock().clone();

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
    if state.is_database_recovery_required() {
        log::warn!("Startup sync skipped: database recovery required");
        return;
    }

    let config = match state.config.read().clone() {
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
        log::warn!(
            "Startup sync: failed to write managed workspace files: {}",
            e
        );
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

    // I644: One-time backfill of dashboard.json narrative fields into DB columns.
    match db.backfill_dashboard_json_to_db(workspace) {
        Ok(n) if n > 0 => log::info!("Startup sync: backfilled {} dashboard.json → DB", n),
        Ok(_) => {}
        Err(e) => log::warn!("Startup sync: dashboard.json backfill failed: {}", e),
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

    // Migrate legacy people notes to entity_context_entries (idempotent)
    let ctx = state.live_service_context();
    match crate::services::entity_context::migrate_legacy_notes(&ctx, &db) {
        Ok(n) if n > 0 => log::info!("Startup sync: migrated {} legacy notes", n),
        Ok(_) => {}
        Err(e) => log::warn!("Startup sync: legacy notes migration failed: {}", e),
    }

    // Rebuild search index (I427)
    {
        use crate::db::search::SearchDb;
        match db.conn_ref().rebuild_search_index() {
            Ok(count) => log::info!("Search index rebuilt: {} entries", count),
            Err(e) => log::warn!("Search index rebuild failed: {}", e),
        }
    }
}

/// Recover from an unclean dev-mode exit (app quit without restore_live).
///
/// Two recovery signals:
/// 1. `config.json.dev-backup` — legacy: config was backed up before dev mode
/// 2. `.dev-mode-active` sentinel — written by `enter_dev_mode()`, deleted by `exit_dev_mode()`
///
/// Either signal triggers recovery. Also cleans up `config-dev.json` (Phase 4).
fn recover_from_unclean_dev_exit() {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };
    let dailyos_dir = home.join(".dailyos");
    let config = dailyos_dir.join("config.json");
    let backup = config.with_extension("json.dev-backup");
    let sentinel = dailyos_dir.join(".dev-mode-active");
    let dev_config = dailyos_dir.join("config-dev.json");

    let needs_recovery = backup.exists() || sentinel.exists();

    // Also check: does the live config itself point at the dev workspace?
    let config_contaminated = std::fs::read_to_string(&config)
        .map(|s| s.contains("DailyOS-dev") || s.contains("\"developerMode\":true") || s.contains("\"developerMode\": true"))
        .unwrap_or(false);

    if needs_recovery || config_contaminated {
        log::warn!("Detected unclean dev-mode exit — restoring live state");

        // Prefer production snapshot (guaranteed clean) over dev-backup (might be corrupted)
        let snapshot = dailyos_dir.join("config.json.production-snapshot");
        if snapshot.exists() {
            match fs::copy(&snapshot, &config) {
                Ok(_) => {
                    let _ = fs::remove_file(&snapshot);
                    let _ = fs::remove_file(&backup);
                    log::info!("Live config restored from production snapshot");
                }
                Err(e) => {
                    log::error!("Failed to restore from production snapshot: {}", e);
                }
            }
        } else if backup.exists() {
            match fs::copy(&backup, &config) {
                Ok(_) => {
                    let _ = fs::remove_file(&backup);
                    log::info!("Live config restored from backup");
                }
                Err(e) => {
                    log::error!("Failed to restore config from dev backup: {}", e);
                }
            }
        }

        // Ensure DEV_DB_MODE is false (already defaults to false on startup, but be explicit)
        crate::db::set_dev_db_mode(false);

        // Clean up sentinel file
        let _ = fs::remove_file(&sentinel);

        // Clean up dev config
        let _ = fs::remove_file(&dev_config);

        // Clean up snapshot if still present
        let _ = fs::remove_file(dailyos_dir.join("config.json.production-snapshot"));

        log::info!("Dev mode recovery complete");
    }
}

/// Path to the dev-mode sentinel file.
pub(crate) fn dev_mode_sentinel_path() -> Result<std::path::PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    Ok(home.join(".dailyos").join(".dev-mode-active"))
}

/// Get the active config file path.
///
/// When dev mode is active, returns `~/.dailyos/config-dev.json` so the live
/// `config.json` is never modified during dev mode.
pub fn config_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let dailyos_dir = home.join(".dailyos");
    if crate::db::is_dev_db_mode() {
        Ok(dailyos_dir.join("config-dev.json"))
    } else {
        Ok(dailyos_dir.join("config.json"))
    }
}

/// Get the live config file path (always `config.json`, ignores dev mode).
pub fn live_config_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    Ok(home.join(".dailyos").join("config.json"))
}

/// Get the dev config file path (~/.dailyos/config-dev.json).
pub fn dev_config_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    Ok(home.join(".dailyos").join("config-dev.json"))
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
    let mut guard = state.config.write();

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
                granola: crate::granola::GranolaConfig::default(),
                gravatar: crate::gravatar::GravatarConfig::default(),
                clay: crate::clay::ClayConfig::default(),
                linear: crate::linear::LinearConfig::default(),
                drive: crate::types::DriveConfig::default(),
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
                ai_model_routing_version: crate::types::AI_MODEL_ROUTING_VERSION,
                embeddings: crate::types::EmbeddingConfig::default(),
                internal_team_setup_completed: false,
                internal_team_setup_version: 0,
                internal_org_account_id: None,
                role: "core".to_string(),
                custom_preset_path: None,
                icloud_warning_dismissed: None,
                app_lock_timeout_minutes: Some(15),
                hygiene_scan_interval_hours: 4,
                hygiene_ai_budget: 0,
                daily_ai_token_budget: crate::pty::DEFAULT_DAILY_AI_TOKEN_BUDGET,
                hygiene_pre_meeting_hours: 12,
                email_enrichment_timeout_seconds: 90,
                notifications: crate::types::NotificationConfig::default(),
                text_scale_percent: 100,
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

/// Load configuration from the active config path.
///
/// In dev mode, reads from `config-dev.json`. Otherwise reads `config.json`.
pub fn load_config() -> Result<Config, String> {
    let config_path = config_path()?;

    if !config_path.exists() {
        return Err(format!(
            "Config file not found at {}. Create it with: {{ \"workspacePath\": \"/path/to/workspace\" }}",
            config_path.display()
        ));
    }

    let content =
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    let mut config: Config =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;
    let original_routing_version = config.ai_model_routing_version;
    config.normalize();
    if config.ai_model_routing_version != original_routing_version {
        let normalized = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize normalized config: {}", e))?;
        crate::util::atomic_write_str(&config_path, &normalized)
            .map_err(|e| format!("Failed to persist normalized config: {}", e))?;
    }

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

fn reconcile_google_enabled_flag(config: &mut Config, google_auth: &GoogleAuthStatus) -> bool {
    if matches!(google_auth, GoogleAuthStatus::Authenticated { .. }) && !config.google.enabled {
        config.google.enabled = true;
        return true;
    }

    false
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
    let mut guard = state.config.write();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::GoogleConfig;

    #[test]
    fn test_app_lock_state_default() {
        let lock_state = AppLockState::default();
        assert!(!lock_state.is_locked);
        assert_eq!(lock_state.failed_unlock_count, 0);
        assert!(lock_state.last_failed_unlock.is_none());
    }

    #[test]
    fn test_reconcile_google_enabled_flag_repairs_authenticated_config() {
        let mut config: Config =
            serde_json::from_value(serde_json::json!({ "workspacePath": "/tmp" })).unwrap();
        config.google = GoogleConfig {
            enabled: false,
            ..GoogleConfig::default()
        };

        let changed = reconcile_google_enabled_flag(
            &mut config,
            &GoogleAuthStatus::Authenticated {
                email: "user@example.com".to_string(),
            },
        );

        assert!(changed);
        assert!(config.google.enabled);
    }

    #[test]
    fn test_reconcile_google_enabled_flag_skips_unauthenticated_config() {
        let mut config: Config =
            serde_json::from_value(serde_json::json!({ "workspacePath": "/tmp" })).unwrap();
        config.google = GoogleConfig {
            enabled: false,
            ..GoogleConfig::default()
        };

        let changed = reconcile_google_enabled_flag(&mut config, &GoogleAuthStatus::NotConfigured);

        assert!(!changed);
        assert!(!config.google.enabled);
    }
}

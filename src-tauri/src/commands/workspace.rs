use super::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyToInboxReport {
    pub copied_count: usize,
    pub copied_filenames: Vec<String>,
}

#[tauri::command]
pub async fn get_all_actions(state: State<'_, Arc<AppState>>) -> Result<ActionsResult, String> {
    Ok(crate::services::actions::get_all_actions(&state).await)
}

// =============================================================================
// Inbox Command
// =============================================================================

/// Result type for inbox files
#[derive(Debug, serde::Serialize)]
#[allow(clippy::large_enum_variant)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum InboxResult {
    Success {
        files: Vec<InboxFile>,
        count: usize,
    },
    Empty {
        message: String,
        files: Vec<InboxFile>,
        count: usize,
    },
    Error {
        message: String,
        files: Vec<InboxFile>,
        count: usize,
    },
}

/// Get files from the _inbox/ directory
#[tauri::command]
pub async fn get_inbox_files(state: State<'_, Arc<AppState>>) -> Result<InboxResult, String> {
    let config = match state.config.read() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => {
                return Ok(InboxResult::Error {
                    message: "No configuration loaded".to_string(),
                    files: Vec::new(),
                    count: 0,
                })
            }
        },
        Err(_) => {
            return Ok(InboxResult::Error {
                message: "Internal error: config lock poisoned".to_string(),
                files: Vec::new(),
                count: 0,
            })
        }
    };

    let workspace = Path::new(&config.workspace_path);
    let mut files = list_inbox_files(workspace);
    let count = files.len();

    // Enrich files with persistent processing status from DB
    if let Ok(status_map) = state
        .db_read(|db| db.get_latest_processing_status().map_err(|e| e.to_string()))
        .await
    {
        for file in &mut files {
            if let Some((status, error)) = status_map.get(&file.filename) {
                file.processing_status = Some(status.clone());
                // For needs_entity, error_message stores the suggested name
                if status == "needs_entity" {
                    file.suggested_entity_name = error.clone();
                } else {
                    file.processing_error = error.clone();
                }
            }
        }
    }

    if files.is_empty() {
        Ok(InboxResult::Empty {
            message: "Inbox is clear".to_string(),
            files,
            count,
        })
    } else {
        Ok(InboxResult::Success { files, count })
    }
}

/// Process a single inbox file (classify, route, log).
///
/// Runs on a background thread to avoid blocking the main thread.
#[tauri::command]
pub async fn process_inbox_file(
    filename: String,
    entity_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::processor::ProcessingResult, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Internal error")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace_path = config.workspace_path.clone();
    let profile = config.profile.clone();
    let entity_id = entity_id.clone();

    // Validate filename before processing (I60: path traversal guard)
    let workspace = Path::new(&workspace_path);
    crate::util::validate_inbox_path(workspace, &filename)?;

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        // Open a dedicated connection instead of holding the shared mutex
        // for the entire duration of process_file (which can take seconds).
        let db = crate::db::ActionDb::open().ok();
        let db_ref = db.as_ref();
        let entity_tracker_path = entity_id.as_deref().and_then(|eid| {
            db_ref
                .and_then(|db| db.get_entity(eid).ok().flatten())
                .and_then(|e| e.tracker_path)
        });
        crate::processor::process_file(
            workspace,
            &filename,
            db_ref,
            &profile,
            entity_tracker_path.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("Processing task failed: {}", e))
}

/// Process all inbox files (batch).
///
/// Runs on a background thread to avoid blocking the main thread.
#[tauri::command]
pub async fn process_all_inbox(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<(String, crate::processor::ProcessingResult)>, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Internal error")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace_path = config.workspace_path.clone();
    let profile = config.profile.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        // Open a dedicated connection instead of holding the shared mutex
        // for the entire batch processing duration.
        let db = crate::db::ActionDb::open().ok();
        crate::processor::process_all(workspace, db.as_ref(), &profile)
    })
    .await
    .map_err(|e| format!("Batch processing failed: {}", e))
}

/// Process an inbox file with AI enrichment via Claude Code.
///
/// Used for files that the quick classifier couldn't categorize.
/// Runs on a background thread — Claude Code can take 1-2 minutes.
#[tauri::command]
pub async fn enrich_inbox_file(
    filename: String,
    entity_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::processor::enrich::EnrichResult, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Internal error")?
        .clone()
        .ok_or("No configuration loaded")?;

    let state = state.inner().clone();
    let workspace_path = config.workspace_path.clone();
    let profile = config.profile.clone();
    let entity_id = entity_id.clone();

    // Validate filename before enriching (I60: path traversal guard)
    let workspace = Path::new(&workspace_path);
    crate::util::validate_inbox_path(workspace, &filename)?;

    let user_ctx = crate::types::UserContext::from_config(&config);
    let ai_config = config.ai_models.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        let entity_tracker_path = state
            .db
            .lock()
            .ok()
            .and_then(|g| {
                g.as_ref().and_then(|db| {
                    entity_id
                        .as_deref()
                        .and_then(|eid| db.get_entity(eid).ok().flatten())
                })
            })
            .and_then(|e| e.tracker_path);
        crate::processor::enrich::enrich_file(
            workspace,
            &filename,
            Some(&state),
            &profile,
            Some(&user_ctx),
            Some(&ai_config),
            entity_tracker_path.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("AI processing task failed: {}", e))
}

/// Get the content of a specific inbox file for preview
#[tauri::command]
pub fn get_inbox_file_content(
    filename: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = Path::new(&config.workspace_path);
    let file_path = crate::util::validate_inbox_path(workspace, &filename)?;

    if !file_path.exists() {
        return Err(format!("File not found: {}", filename));
    }

    // Extract text content — works for both text and binary document formats
    use crate::processor::extract;

    let format = extract::detect_format(&file_path);
    if matches!(format, extract::SupportedFormat::Unsupported) {
        // Truly unsupported format — show descriptive message
        let size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown");
        return Ok(format!(
            "[Unsupported file — .{} — {} bytes]\n\nText extraction is not available for this format. Use \"Process\" to archive it.",
            ext, size
        ));
    }

    match extract::extract_text(&file_path) {
        Ok(content) => Ok(content),
        Err(e) => {
            // Extraction failed — show error with fallback info
            let size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);
            let ext = file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown");
            Ok(format!(
                "[Extraction failed — .{} — {} bytes]\n\nError: {}\n\nUse \"Process\" to let DailyOS handle it.",
                ext, size, e
            ))
        }
    }
}

// =============================================================================
// Inbox Drop Zone
// =============================================================================

/// Copy files into the _inbox/ directory (used by drop zone).
///
/// Accepts absolute file paths from the drag-drop event.
/// Returns the number of files successfully copied plus the exact filenames
/// written into `_inbox/` after duplicate resolution.
#[tauri::command]
pub fn copy_to_inbox(
    paths: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<CopyToInboxReport, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = Path::new(&config.workspace_path);
    let inbox_dir = workspace.join("_inbox");

    // Ensure _inbox/ exists
    if !inbox_dir.exists() {
        std::fs::create_dir_all(&inbox_dir)
            .map_err(|e| format!("Failed to create _inbox: {}", e))?;
    }

    // Build allowlist of source directories
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let allowed_source_dirs: Vec<std::path::PathBuf> = vec![
        home.join("Documents"),
        home.join("Desktop"),
        home.join("Downloads"),
    ];

    let mut copied_filenames = Vec::new();

    for path_str in &paths {
        let source = Path::new(path_str);

        // Skip directories
        if !source.is_file() {
            continue;
        }

        // Validate source path is within allowed directories
        let canonical_source = std::fs::canonicalize(source)
            .map_err(|e| format!("Invalid source path '{}': {}", path_str, e))?;
        let source_str = canonical_source.to_string_lossy();
        let source_allowed = allowed_source_dirs.iter().any(|dir| {
            std::fs::canonicalize(dir)
                .map(|cd| source_str.starts_with(&*cd.to_string_lossy()))
                .unwrap_or(false)
        });
        if !source_allowed {
            log::warn!(
                "Rejected copy_to_inbox source outside allowed directories: {}",
                path_str
            );
            continue;
        }

        let filename = match source.file_name() {
            Some(name) => name.to_owned(),
            None => continue,
        };

        let mut dest = inbox_dir.join(&filename);

        // Handle duplicates: append (1), (2), etc.
        if dest.exists() {
            let stem = dest
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file")
                .to_string();
            let ext = dest
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();
            let mut counter = 1;
            loop {
                let new_name = if ext.is_empty() {
                    format!("{} ({})", stem, counter)
                } else {
                    format!("{} ({}).{}", stem, counter, ext)
                };
                dest = inbox_dir.join(new_name);
                if !dest.exists() {
                    break;
                }
                counter += 1;
            }
        }

        match std::fs::copy(source, &dest) {
            Ok(_) => {
                log::info!("Copied '{}' to inbox", filename.to_string_lossy());
                copied_filenames.push(
                    dest.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(path_str)
                        .to_string(),
                );
            }
            Err(e) => {
                log::warn!("Failed to copy '{}' to inbox: {}", path_str, e);
            }
        }
    }

    Ok(CopyToInboxReport {
        copied_count: copied_filenames.len(),
        copied_filenames,
    })
}

// =============================================================================
// Emails Command
// =============================================================================

/// Result type for email summary
#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum EmailsResult {
    Success { data: Vec<crate::types::Email> },
    NotFound { message: String },
    Error { message: String },
}

/// Get all emails
#[tauri::command]
pub fn get_all_emails(state: State<'_, Arc<AppState>>) -> EmailsResult {
    // Get config
    let config = match state.config.read() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => {
                return EmailsResult::Error {
                    message: "No configuration loaded".to_string(),
                }
            }
        },
        Err(_) => {
            return EmailsResult::Error {
                message: "Internal error: config lock poisoned".to_string(),
            }
        }
    };

    let workspace = Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    match load_emails_json(&today_dir) {
        Ok(emails) => {
            if emails.is_empty() {
                EmailsResult::NotFound {
                    message: "No emails found.".to_string(),
                }
            } else {
                EmailsResult::Success { data: emails }
            }
        }
        Err(e) => EmailsResult::NotFound {
            message: format!("No emails: {}", e),
        },
    }
}

/// Get emails enriched with entity signals from SQLite.
///
/// Get enriched email briefing data with signals and entity threads.
#[tauri::command]
pub async fn get_emails_enriched(
    state: State<'_, Arc<AppState>>,
) -> Result<EmailBriefingData, String> {
    crate::services::emails::get_emails_enriched(&state).await
}

/// Update the entity assignment for an email (I395 — user correction).
/// Cascades to email_signals and emits a signal bus event for relevance learning.
#[tauri::command]
pub async fn update_email_entity(
    state: State<'_, Arc<AppState>>,
    email_id: String,
    entity_id: Option<String>,
    entity_type: Option<String>,
) -> Result<(), String> {
    state
        .db_write(move |db| {
            crate::services::emails::update_email_entity(
                db,
                &email_id,
                entity_id.as_deref(),
                entity_type.as_deref(),
            )
        })
        .await
}

/// Dismiss a single email signal by ID. Sets `deactivated_at` to now.
/// Emits a signal bus event for relevance learning.
#[tauri::command]
pub async fn dismiss_email_signal(
    state: State<'_, Arc<AppState>>,
    signal_id: i64,
) -> Result<(), String> {
    state
        .db_write(move |db| crate::services::emails::dismiss_email_signal(db, signal_id))
        .await
}

/// Get email sync status: last fetch time, enrichment progress, failure count (I373).
#[tauri::command]
pub async fn get_email_sync_status(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::db::EmailSyncStats, String> {
    state.db_read(|db| db.get_email_sync_stats()).await
}

/// Get emails linked to a specific entity for entity detail pages (I368 AC5).
#[tauri::command]
pub async fn get_entity_emails(
    state: State<'_, Arc<AppState>>,
    entity_id: String,
    entity_type: String,
) -> Result<Vec<crate::db::DbEmail>, String> {
    state
        .db_read(move |db| crate::services::emails::get_entity_emails(db, &entity_id, &entity_type))
        .await
}

/// Refresh emails independently without re-running the full /today pipeline (I20).
#[tauri::command]
pub async fn refresh_emails(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    crate::services::emails::refresh_emails(state.inner(), app_handle).await
}

/// Reconcile local inbox presence with Gmail inbox in lightweight mode.
/// Marks archived/removed emails resolved without running full enrichment.
#[tauri::command]
pub async fn sync_email_inbox_presence(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<bool, String> {
    crate::services::emails::sync_email_inbox_presence(state.inner(), app_handle).await
}

/// Archive low-priority emails in Gmail and remove them from local data (I144).
#[tauri::command]
pub async fn archive_low_priority_emails(state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    crate::services::emails::archive_low_priority_emails(&state).await
}

/// Set user profile (customer-success or general)
#[tauri::command]
pub fn set_profile(profile: String, state: State<'_, Arc<AppState>>) -> Result<Config, String> {
    // Validate profile value
    if profile != "customer-success" && profile != "general" {
        return Err(format!(
            "Invalid profile: {}. Must be 'customer-success' or 'general'.",
            profile
        ));
    }

    crate::state::create_or_update_config(&state, |config| {
        config.profile = profile.clone();
    })
}

/// Set entity mode (account, project, or both)
///
/// Also derives the correct profile for backend compatibility.
/// Creates Accounts/ dir if switching to account/both mode.
#[tauri::command]
pub fn set_entity_mode(
    mode: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<Config, String> {
    let config = crate::services::settings::set_entity_mode(&mode, &state)?;
    let _ = app_handle.emit("config-updated", ());
    Ok(config)
}

/// Set workspace path and scaffold directory structure
#[tauri::command]
pub async fn set_workspace_path(
    path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    let result = crate::services::settings::set_workspace_path(&path, &state).await;
    if result.is_ok() {
        if let Ok(mut audit) = state.audit_log.lock() {
            let category = {
                let home = dirs::home_dir().unwrap_or_default();
                let documents = home.join("Documents");
                let p = std::path::Path::new(&path);
                if p.starts_with(&documents) {
                    "documents"
                } else if p.starts_with(&home) {
                    "home"
                } else {
                    "custom"
                }
            };
            let _ = audit.append(
                "config",
                "workspace_path_changed",
                serde_json::json!({"category": category}),
            );
        }
    }
    result
}

/// Toggle developer mode with full isolation.
///
/// When enabled: switches to isolated dev database, workspace, and auth.
/// When disabled: returns to live database, workspace, and real auth.
#[tauri::command]
pub async fn set_developer_mode(
    enabled: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    if enabled {
        crate::devtools::enter_dev_mode(&state)?;
    } else {
        crate::devtools::exit_dev_mode(&state)?;
    }

    // Reinitialize the async DB connection pool at the new path
    if let Err(e) = state.reinit_db_service().await {
        log::warn!("Failed to reinit db_service after dev mode toggle: {}", e);
    }

    // Return the current config (which is now either dev or live)
    let guard = state.config.read().map_err(|_| "Lock poisoned")?;
    guard
        .clone()
        .ok_or_else(|| "No configuration loaded".to_string())
}

/// Check if workspace is under iCloud sync and warning hasn't been dismissed (I464).
#[tauri::command]
pub fn check_icloud_warning(state: State<'_, Arc<AppState>>) -> Result<Option<String>, String> {
    let guard = state.config.read().map_err(|_| "Lock poisoned")?;
    let config = guard
        .clone()
        .ok_or_else(|| "No configuration loaded".to_string())?;

    if config.icloud_warning_dismissed.unwrap_or(false) {
        return Ok(None);
    }

    let workspace_path = &config.workspace_path;
    if crate::util::is_under_icloud_scope(workspace_path) {
        Ok(Some(workspace_path.clone()))
    } else {
        Ok(None)
    }
}

/// Dismiss the iCloud workspace warning permanently (I464).
#[tauri::command]
pub fn dismiss_icloud_warning(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.icloud_warning_dismissed = Some(true);
    })?;
    Ok(())
}

// =============================================================================
// App Lock (I465)
// =============================================================================

/// Get whether the app is currently locked.
#[tauri::command]
pub fn get_lock_status(state: State<'_, Arc<AppState>>) -> bool {
    state.is_locked.load(std::sync::atomic::Ordering::Relaxed)
}

/// Check if the encryption key is missing (I462 recovery screen).
#[tauri::command]
pub fn get_encryption_key_status(state: State<'_, Arc<AppState>>) -> bool {
    state
        .encryption_key_missing
        .load(std::sync::atomic::Ordering::Relaxed)
}

/// Lock the app immediately.
#[tauri::command]
pub async fn lock_app(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    state
        .is_locked
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = app.emit("app-locked", ());
    Ok(())
}

/// Attempt to unlock the app via system authentication (Touch ID / password).
#[tauri::command]
pub async fn unlock_app(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;

    // Check cooldown: 30s after 3 consecutive failures
    let failed_count = state.failed_unlock_count.load(Ordering::Relaxed);
    if failed_count >= 3 {
        if let Ok(guard) = state.last_failed_unlock.lock() {
            if let Some(last) = *guard {
                if last.elapsed().as_secs() < 30 {
                    let remaining = 30 - last.elapsed().as_secs();
                    return Err(format!(
                        "Too many failed attempts. Try again in {} seconds.",
                        remaining
                    ));
                }
            }
        }
        // Cooldown expired, reset counter
        state.failed_unlock_count.store(0, Ordering::Relaxed);
    }

    // Attempt system authentication (Touch ID / password)
    match attempt_system_auth().await {
        Ok(true) => {
            state.is_locked.store(false, Ordering::Relaxed);
            state.failed_unlock_count.store(0, Ordering::Relaxed);
            // Reset activity timer so the user gets a fresh idle window
            if let Ok(mut guard) = state.last_activity.lock() {
                *guard = std::time::Instant::now();
            }
            if let Ok(mut audit) = state.audit_log.lock() {
                let _ = audit.append("security", "app_unlock_succeeded", serde_json::json!({}));
            }
            let _ = app.emit("app-unlocked", ());
            Ok(())
        }
        Ok(false) => {
            let new_count = state.failed_unlock_count.fetch_add(1, Ordering::Relaxed) + 1;
            if let Ok(mut audit) = state.audit_log.lock() {
                let _ = audit.append(
                    "security",
                    "app_unlock_failed",
                    serde_json::json!({"consecutive_failures": new_count}),
                );
            }
            if let Ok(mut guard) = state.last_failed_unlock.lock() {
                *guard = Some(std::time::Instant::now());
            }
            if new_count >= 3 {
                Err(
                    "Authentication failed. Too many attempts — please wait 30 seconds."
                        .to_string(),
                )
            } else {
                Err("Authentication failed.".to_string())
            }
        }
        Err(e) => Err(format!("Authentication error: {}", e)),
    }
}

/// Set the app lock idle timeout in minutes (None = disabled).
#[tauri::command]
pub fn set_lock_timeout(
    minutes: Option<u32>,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    if let Some(v) = minutes {
        if ![5, 15, 30].contains(&v) {
            return Err(format!(
                "Invalid lock timeout: {}. Must be 5, 15, or 30.",
                v
            ));
        }
    }
    crate::state::create_or_update_config(&state, |config| {
        config.app_lock_timeout_minutes = minutes;
    })
}

/// Signal user activity (click/keypress) to reset the idle lock timer.
#[tauri::command]
pub fn signal_user_activity(state: State<'_, Arc<AppState>>) {
    if let Ok(mut guard) = state.last_activity.lock() {
        *guard = std::time::Instant::now();
    }
}

/// Signal window focus change to reset the idle lock timer.
#[tauri::command]
pub fn signal_window_focus(focused: bool, state: State<'_, Arc<AppState>>) {
    if focused {
        if let Ok(mut guard) = state.last_activity.lock() {
            *guard = std::time::Instant::now();
        }
    }
}

/// Attempt system-level authentication using macOS LocalAuthentication framework.
/// Calls LAContext.evaluatePolicy directly via objc2 FFI so the Touch ID dialog
/// shows "DailyOS" as the requesting app (not "osascript").
#[cfg(target_os = "macos")]
async fn attempt_system_auth() -> Result<bool, String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<bool, String>>();

    std::thread::spawn(move || {
        use block2::RcBlock;
        use objc2::runtime::Bool;
        use objc2_foundation::{NSComparisonResult, NSDate, NSRunLoop, NSString};
        use objc2_local_authentication::{LAContext, LAPolicy};

        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        unsafe {
            let context = LAContext::new();
            let policy = LAPolicy(1); // deviceOwnerAuthentication (biometrics + passcode fallback)

            // Check if any authentication method is available
            if let Err(e) = context.canEvaluatePolicy_error(policy) {
                log::warn!("Biometric authentication unavailable: {e}, auto-unlocking");
                let _ = tx.send(Ok(true));
                return;
            }

            let reason = NSString::from_str("DailyOS requires authentication to unlock.");
            let done_clone = done.clone();
            let tx = std::sync::Mutex::new(Some(tx));
            let block = RcBlock::new(move |success: Bool, err: *mut objc2_foundation::NSError| {
                let result = if success.as_bool() {
                    Ok(true)
                } else if !err.is_null() {
                    let err_ref = &*err;
                    let code = err_ref.code();
                    // LAError.userCancel = -2, LAError.systemCancel = -4
                    if code == -2 || code == -4 {
                        log::info!("Touch ID cancelled (code {code})");
                        Ok(false)
                    } else {
                        let desc = err_ref.localizedDescription();
                        log::warn!("Touch ID error (code {code}): {desc}");
                        Ok(false)
                    }
                } else {
                    Ok(false)
                };
                if let Some(tx) = tx.lock().unwrap().take() {
                    let _ = tx.send(result);
                }
                done_clone.store(true, std::sync::atomic::Ordering::Release);
            });

            context.evaluatePolicy_localizedReason_reply(policy, &reason, &block);

            // Pump the run loop until the callback fires or 30s deadline
            let deadline = NSDate::dateWithTimeIntervalSinceNow(30.0);
            while !done.load(std::sync::atomic::Ordering::Acquire) {
                let step = NSDate::dateWithTimeIntervalSinceNow(0.1);
                NSRunLoop::currentRunLoop().runUntilDate(&step);
                if NSDate::date().compare(&deadline) != NSComparisonResult::Ascending {
                    log::warn!("Touch ID run loop deadline exceeded");
                    break;
                }
            }
        }
    });

    // 35s outer timeout — the thread has its own 30s deadline,
    // but if something hangs we don't want the frontend stuck forever.
    match tokio::time::timeout(std::time::Duration::from_secs(35), rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => {
            log::warn!("Touch ID channel closed without result");
            Ok(false)
        }
        Err(_) => Err("Authentication timed out".to_string()),
    }
}

/// Non-macOS fallback: no biometric available, auto-unlock.
#[cfg(not(target_os = "macos"))]
async fn attempt_system_auth() -> Result<bool, String> {
    Ok(true)
}

/// Set UI personality tone (professional, friendly, playful)
#[tauri::command]
pub fn set_personality(
    personality: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    let normalized = personality.to_lowercase();
    crate::types::validate_personality(&normalized)?;
    crate::state::create_or_update_config(&state, |config| {
        config.personality = normalized.clone();
    })
}

/// Set AI model for a tier (synthesis, extraction, mechanical)
#[tauri::command]
pub fn set_ai_model(
    tier: String,
    model: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    crate::services::settings::set_ai_model(&tier, &model, &state)
}

/// Set hygiene configuration (I271)
#[tauri::command]
pub fn set_hygiene_config(
    scan_interval_hours: Option<u32>,
    ai_budget: Option<u32>,
    pre_meeting_hours: Option<u32>,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    crate::services::settings::set_hygiene_config(
        scan_interval_hours,
        ai_budget,
        pre_meeting_hours,
        &state,
    )
}

/// Set schedule for a workflow
#[tauri::command]
pub fn set_schedule(
    workflow: String,
    hour: u32,
    minute: u32,
    timezone: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    crate::services::settings::set_schedule(&workflow, hour, minute, &timezone, &state)
}

/// Save user profile fields (name, company, title, focus, domains)
#[tauri::command]
pub async fn set_user_profile(
    name: Option<String>,
    company: Option<String>,
    title: Option<String>,
    focus: Option<String>,
    domain: Option<String>,
    domains: Option<Vec<String>>,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    crate::services::settings::set_user_profile(
        name, company, title, focus, domain, domains, &state,
    )
    .await
}

// =============================================================================
// User Entity Commands (I411 — ADR-0089/0090)
// =============================================================================

/// Get the user entity (creates from config on first call).
#[tauri::command]
pub async fn get_user_entity(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::UserEntity, String> {
    crate::services::user_entity::get_user_entity(&state).await
}

/// Update a single field on the user entity.
#[tauri::command]
pub async fn update_user_entity_field(
    field: String,
    value: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::user_entity::update_user_entity_field(&field, &value, &state).await
}

/// Get all user context entries.
#[tauri::command]
pub async fn get_user_context_entries(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::types::UserContextEntry>, String> {
    crate::services::user_entity::get_user_context_entries(&state).await
}

/// Create a new user context entry.
#[tauri::command]
pub async fn create_user_context_entry(
    title: String,
    content: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::UserContextEntry, String> {
    crate::services::user_entity::create_user_context_entry(&title, &content, &state).await
}

/// Update an existing user context entry.
#[tauri::command]
pub async fn update_user_context_entry(
    id: String,
    title: String,
    content: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::user_entity::update_user_context_entry(&id, &title, &content, &state).await
}

/// Delete a user context entry.
#[tauri::command]
pub async fn delete_user_context_entry(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::user_entity::delete_user_context_entry(&id, &state).await
}

/// Get all entity context entries for an entity.
#[tauri::command]
pub async fn get_entity_context_entries(
    entity_type: String,
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::types::EntityContextEntry>, String> {
    crate::services::entity_context::get_entries(&entity_type, &entity_id, &state).await
}

/// Create a new entity context entry.
#[tauri::command]
pub async fn create_entity_context_entry(
    entity_type: String,
    entity_id: String,
    title: String,
    content: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::EntityContextEntry, String> {
    crate::services::entity_context::create_entry(
        &entity_type,
        &entity_id,
        &title,
        &content,
        &state,
    )
    .await
}

/// Update an existing entity context entry.
#[tauri::command]
pub async fn update_entity_context_entry(
    id: String,
    title: String,
    content: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::entity_context::update_entry(&id, &title, &content, &state).await
}

/// Delete an entity context entry.
#[tauri::command]
pub async fn delete_entity_context_entry(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::entity_context::delete_entry(&id, &state).await
}

/// Process a user attachment from the /me page dropzone.
///
/// Copies the file into _user/attachments/ (if not already there), processes it
/// through the file processor pipeline, and indexes it as user_context content.
#[tauri::command]
pub async fn process_user_attachment(
    path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Internal error")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let attachments_dir = workspace.join("_user").join("attachments");

    if !attachments_dir.exists() {
        std::fs::create_dir_all(&attachments_dir)
            .map_err(|e| format!("Failed to create _user/attachments: {}", e))?;
    }

    let source = std::path::Path::new(&path);
    if !source.is_file() {
        return Err(format!("Not a file: {}", path));
    }

    let filename = source.file_name().ok_or("Invalid filename")?;
    let dest = attachments_dir.join(filename);

    let final_path = if source.starts_with(&attachments_dir) {
        source.to_path_buf()
    } else {
        let final_dest = if dest.exists() {
            let stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let ext = dest.extension().and_then(|e| e.to_str()).unwrap_or("");
            let mut candidate = dest.clone();
            for i in 1..1000 {
                candidate = if ext.is_empty() {
                    attachments_dir.join(format!("{}-{}", stem, i))
                } else {
                    attachments_dir.join(format!("{}-{}.{}", stem, i, ext))
                };
                if !candidate.exists() {
                    break;
                }
            }
            candidate
        } else {
            dest
        };

        std::fs::copy(source, &final_dest).map_err(|e| format!("Failed to copy file: {}", e))?;
        final_dest
    };

    let state_inner = state.inner().clone();
    let workspace_owned = workspace.to_path_buf();
    let final_path_owned = final_path.clone();

    let result = tokio::task::spawn_blocking(move || {
        let db = crate::db::ActionDb::open().ok();
        let result = crate::processor::process_user_attachment(
            &workspace_owned,
            &final_path_owned,
            db.as_ref(),
        );

        if matches!(result, crate::processor::ProcessingResult::Routed { .. }) {
            state_inner
                .embedding_queue
                .enqueue(crate::processor::embeddings::EmbeddingRequest {
                    entity_id: "user_context".to_string(),
                    entity_type: "user_context".to_string(),
                    requested_at: std::time::Instant::now(),
                });
        }

        result
    })
    .await
    .map_err(|e| format!("Processing failed: {}", e))?;

    match result {
        crate::processor::ProcessingResult::Routed { destination, .. } => Ok(destination),
        crate::processor::ProcessingResult::Error { message } => Err(message),
        crate::processor::ProcessingResult::NeedsEnrichment
        | crate::processor::ProcessingResult::NeedsEntity { .. } => {
            Ok(final_path.display().to_string())
        }
    }
}

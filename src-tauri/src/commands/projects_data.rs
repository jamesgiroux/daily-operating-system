use super::*;

/// Project list item with computed fields for the list page.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListItem {
    pub id: String,
    pub name: String,
    pub status: String,
    pub milestone: Option<String>,
    pub owner: Option<String>,
    pub target_date: Option<String>,
    pub open_action_count: usize,
    pub days_since_last_meeting: Option<i64>,
    pub parent_id: Option<String>,
    pub parent_name: Option<String>,
    pub child_count: usize,
    pub is_parent: bool,
    pub archived: bool,
}

/// Full project detail for the detail page.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetailResult {
    pub id: String,
    pub name: String,
    pub status: String,
    pub milestone: Option<String>,
    pub owner: Option<String>,
    pub target_date: Option<String>,
    pub description: Option<String>,
    pub milestones: Vec<crate::projects::ProjectMilestone>,
    pub notes: Option<String>,
    pub open_actions: Vec<crate::db::DbAction>,
    pub recent_meetings: Vec<MeetingSummary>,
    pub linked_people: Vec<crate::db::DbPerson>,
    pub signals: Option<crate::db::ProjectSignals>,
    pub recent_captures: Vec<crate::db::DbCapture>,
    pub recent_email_signals: Vec<crate::db::DbEmailSignal>,
    pub archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence: Option<crate::intelligence::IntelligenceJson>,
    pub parent_id: Option<String>,
    pub parent_name: Option<String>,
    pub children: Vec<ProjectChildSummary>,
    pub parent_aggregate: Option<crate::db::ProjectParentAggregate>,
}

/// Compact child project summary for parent detail pages (I388).
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChildSummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub milestone: Option<String>,
    pub open_action_count: usize,
}

#[tauri::command]
pub async fn get_projects_list(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ProjectListItem>, String> {
    crate::services::projects::get_projects_list(&state).await
}

/// Get full detail for a project.
#[tauri::command]
pub async fn get_project_detail(
    project_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<ProjectDetailResult, String> {
    crate::services::projects::get_project_detail(&project_id, &state).await
}

/// Get child projects for a parent project (I388).
#[tauri::command]
pub async fn get_child_projects_list(
    parent_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ProjectListItem>, String> {
    crate::services::projects::get_child_projects_list(&parent_id, &state).await
}

/// I388: Get ancestor projects for breadcrumb navigation.
#[tauri::command]
pub async fn get_project_ancestors(
    project_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbProject>, String> {
    state
        .db_read(move |db| {
            db.get_project_ancestors(&project_id)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Create a new project.
#[tauri::command]
pub async fn create_project(
    name: String,
    parent_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let ctx = state.live_service_context();
    crate::services::projects::create_project(&ctx, &name, parent_id, &state).await
}

/// Update a single structured field on a project.
#[tauri::command]
pub async fn update_project_field(
    project_id: String,
    field: String,
    value: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let ctx = state.live_service_context();
    crate::services::projects::update_project_field(&ctx, &project_id, &field, &value, &state)
        .await
}

/// Update the notes field on a project.
#[tauri::command]
pub async fn update_project_notes(
    project_id: String,
    notes: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::projects::update_project_notes(&project_id, &notes, &state).await
}

/// Enrich a project via Claude Code intelligence enrichment.
#[tauri::command]
pub async fn enrich_project(
    app_handle: tauri::AppHandle,
    project_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::intelligence::IntelligenceJson, String> {
    let app_state = state.inner().clone();
    let ctx = app_state.live_service_context();
    crate::services::intelligence::enrich_entity(
        &ctx,
        project_id,
        "project".to_string(),
        &app_state,
        Some(&app_handle),
    )
    .await
}

// ── I76: Database Backup & Rebuild ──────────────────────────────────

#[tauri::command]
pub async fn backup_database(state: tauri::State<'_, Arc<AppState>>) -> Result<String, String> {
    state.db_read(crate::db_backup::backup_database).await
}

#[tauri::command]
pub fn get_database_recovery_status(
    state: State<'_, Arc<AppState>>,
) -> crate::state::DatabaseRecoveryStatus {
    state.get_database_recovery_status()
}

#[tauri::command]
pub fn list_database_backups() -> Result<Vec<crate::db_backup::BackupInfo>, String> {
    crate::db_backup::list_database_backups()
}

#[tauri::command]
pub async fn restore_database_from_backup(
    backup_path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // I566: Timeout on user-facing permit acquisition.
    let _permit = match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        state.permits.user_initiated.acquire(),
    )
    .await
    {
        Ok(Ok(permit)) => permit,
        Ok(Err(_)) => return Err("PTY permit closed".to_string()),
        Err(_) => return Err("Background work in progress — please try again shortly".to_string()),
    };

    // I609: Drop async DB service before swapping files on disk.
    {
        let mut db_service_guard = state.db_service.write().await;
        *db_service_guard = None;
    }

    if let Err(e) = crate::db_backup::restore_database_from_backup(Path::new(&backup_path)) {
        // Best-effort recovery: re-init DB service if restore failed.
        let _ = state.init_db_service().await;
        return Err(e);
    }

    if let Err(e) = state.init_db_service().await {
        state.set_database_recovery_required("restore_reopen_failed", e.clone());
        return Err(format!(
            "Restore succeeded but failed to reinitialize DB service: {e}"
        ));
    }

    state.clear_database_recovery_required();
    Ok(())
}

#[tauri::command]
pub async fn start_fresh_database(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    // I609: Drop async DB service before deleting files.
    {
        let mut db_service_guard = state.db_service.write().await;
        *db_service_guard = None;
    }
    crate::db_backup::start_fresh_database()
}

#[tauri::command]
pub async fn export_database_copy(destination: String) -> Result<(), String> {
    crate::db_backup::export_database_copy(&destination)
}

#[tauri::command]
pub fn get_database_info() -> Result<crate::db_backup::DatabaseInfo, String> {
    crate::db_backup::get_database_info()
}

#[tauri::command]
pub async fn rebuild_database(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(usize, usize, usize), String> {
    let (workspace_path, user_domains) = {
        let guard = state.config.read();
        let config = guard.as_ref().ok_or("Config not loaded")?;
        (
            config.workspace_path.clone(),
            config.resolved_user_domains(),
        )
    };

    state
        .db_write(move |db| {
            crate::db_backup::rebuild_from_filesystem(
                std::path::Path::new(&workspace_path),
                db,
                &user_domains,
            )
        })
        .await
}

/// Get the latest hygiene scan report
#[tauri::command]
pub fn get_hygiene_report(
    state: State<'_, Arc<AppState>>,
) -> Result<Option<crate::hygiene::HygieneReport>, String> {
    Ok(state.hygiene.report.lock().clone())
}

/// Get a prose narrative summarizing the last hygiene scan.
#[tauri::command]
pub fn get_hygiene_narrative(
    state: State<'_, Arc<AppState>>,
) -> Result<Option<crate::hygiene::HygieneNarrativeView>, String> {
    let report = state.hygiene.report.lock();
    Ok(report
        .as_ref()
        .and_then(crate::hygiene::build_hygiene_narrative))
}

/// Get the current Intelligence Hygiene status view model.
#[tauri::command]
pub fn get_intelligence_hygiene_status(
    state: State<'_, Arc<AppState>>,
) -> Result<HygieneStatusView, String> {
    let report = state.hygiene.report.lock().clone();
    Ok(build_intelligence_hygiene_status(&state, report.as_ref()))
}

/// Run a hygiene scan immediately and return the updated status.
#[tauri::command]
pub fn run_hygiene_scan_now(state: State<'_, Arc<AppState>>) -> Result<HygieneStatusView, String> {
    if state
        .hygiene
        .scan_running
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
        )
        .is_err()
    {
        return Err("A hygiene scan is already running".to_string());
    }

    let scan_result = (|| -> Result<crate::hygiene::HygieneReport, String> {
        let config = state
            .config
            .read()
            .clone()
            .ok_or("No configuration loaded".to_string())?;

        let db = crate::db::ActionDb::open().map_err(|e| e.to_string())?;
        let workspace = std::path::Path::new(&config.workspace_path);
        let report = crate::hygiene::run_hygiene_scan(
            &db,
            &config,
            workspace,
            Some(&state.hygiene.budget),
            Some(&state.intel_queue),
            false,
            Some(state.embedding_model.as_ref()),
        );

        // Prune old audit trail files (I297)
        let pruned = crate::audit::prune_audit_files(workspace);
        if pruned > 0 {
            log::info!("run_hygiene_scan_now: pruned {} old audit files", pruned);
        }

        *state.hygiene.report.lock() = Some(report.clone());
        *state.hygiene.last_scan_at.lock() = Some(report.scanned_at.clone());
        *state.hygiene.next_scan_at.lock() = Some(
            (chrono::Utc::now()
                + chrono::Duration::seconds(
                    crate::hygiene::scan_interval_secs(Some(&config)) as i64
                ))
            .to_rfc3339(),
        );

        Ok(report)
    })();

    state
        .hygiene
        .scan_running
        .store(false, std::sync::atomic::Ordering::Release);

    let report = scan_result?;
    Ok(build_intelligence_hygiene_status(&state, Some(&report)))
}

/// Detect potential duplicate people (I172).
#[tauri::command]
pub async fn get_duplicate_people(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::hygiene::DuplicateCandidate>, String> {
    state.db_read(crate::hygiene::detect_duplicate_people).await
}

/// Detect potential duplicate people for a specific person (I172).
#[tauri::command]
pub async fn get_duplicate_people_for_person(
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::hygiene::DuplicateCandidate>, String> {
    state
        .db_read(move |db| {
            let dupes = crate::hygiene::detect_duplicate_people(db)?;
            Ok(dupes
                .into_iter()
                .filter(|d| d.person1_id == person_id || d.person2_id == person_id)
                .collect())
        })
        .await
}

// =============================================================================
// I176: Archive / Unarchive Entities
// =============================================================================

/// Archive or unarchive an account. Cascades to children when archiving.
#[tauri::command]
pub async fn archive_account(
    id: String,
    archived: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::archive_account(&ctx, db, &app_state, &id, archived)
        })
        .await
}

/// Merge source account into target account.
#[tauri::command]
pub async fn merge_accounts(
    from_id: String,
    into_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::db::MergeResult, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::merge_accounts(&ctx, db, &app_state, &from_id, &into_id)
        })
        .await
}

/// Archive or unarchive a project.
#[tauri::command]
pub async fn archive_project(
    id: String,
    archived: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::projects::archive_project(&ctx, db, &app_state, &id, archived)
        })
        .await
}

/// Archive or unarchive a person.
#[tauri::command]
pub async fn archive_person(
    id: String,
    archived: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::people::archive_person(&ctx, db, &app_state, &id, archived)
        })
        .await
}

/// Get archived accounts.
#[tauri::command]
pub async fn get_archived_accounts(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAccount>, String> {
    state
        .db_read(|db| db.get_archived_accounts().map_err(|e| e.to_string()))
        .await
}

/// Get archived projects.
#[tauri::command]
pub async fn get_archived_projects(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbProject>, String> {
    state
        .db_read(|db| db.get_archived_projects().map_err(|e| e.to_string()))
        .await
}

/// Get archived people with signals.
#[tauri::command]
pub async fn get_archived_people(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::PersonListItem>, String> {
    state
        .db_read(|db| {
            db.get_archived_people_with_signals()
                .map_err(|e| e.to_string())
        })
        .await
}

/// Restore an archived account with optional child restoration (I199).
#[tauri::command]
pub async fn restore_account(
    account_id: String,
    restore_children: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<usize, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::restore_account(&ctx, db, &account_id, restore_children)
        })
        .await
}

// =============================================================================
// I171: Multi-Domain Config
// =============================================================================

/// Set multiple user domains for multi-org meeting classification.
#[tauri::command]
pub async fn set_user_domains(
    domains: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    let ctx = state.live_service_context();
    crate::services::settings::set_user_domains(&ctx, &domains, &state).await
}

// =============================================================================
// I162: Bulk Entity Creation
// =============================================================================

/// Bulk-create accounts from a list of names. Returns created account IDs.
#[tauri::command]
pub async fn bulk_create_accounts(
    names: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let workspace_path = state
        .config
        .read()
        .as_ref()
        .ok_or("Config not loaded")?
        .workspace_path
        .clone();
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            let workspace = Path::new(&workspace_path);
            crate::services::accounts::bulk_create_accounts(&ctx, db, workspace, &names)
        })
        .await
}

/// Bulk-create projects from a list of names. Returns created project IDs.
#[tauri::command]
pub async fn bulk_create_projects(
    names: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let workspace_path = state
        .config
        .read()
        .as_ref()
        .ok_or("Config not loaded")?
        .workspace_path
        .clone();
    let state_for_ctx = state.inner().clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            let workspace = Path::new(&workspace_path);
            crate::services::projects::bulk_create_projects(&ctx, db, workspace, &names)
        })
        .await
}

// =============================================================================
// I143: Account Events
// =============================================================================

/// Record an account lifecycle event (expansion, downsell, churn, etc.)
#[tauri::command]
pub async fn record_account_event(
    account_id: String,
    event_type: String,
    event_date: String,
    arr_impact: Option<f64>,
    notes: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<i64, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::record_account_event(
                &ctx,
                db,
                &app_state,
                &account_id,
                &event_type,
                &event_date,
                arr_impact,
                notes.as_deref(),
            )
        })
        .await
}

/// Get account events for a given account.
#[tauri::command]
pub async fn get_account_events(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAccountEvent>, String> {
    state
        .db_read(move |db| {
            db.get_account_events(&account_id)
                .map_err(|e| e.to_string())
        })
        .await
}

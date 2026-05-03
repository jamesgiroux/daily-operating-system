// Suppress dead_code — serde struct fields appear unused to the compiler but
// are required for forward-compatible JSON deserialization. Parser/notification
// functions are reserved for future use.
#![allow(dead_code)]
// Devtools mock data uses large tuple types for seed fixtures.
#![allow(clippy::type_complexity)]

pub mod accounts;
pub mod action_status;
pub mod activity;
pub mod abilities;
mod audit;
pub mod audit_log;
mod backfill_meetings;
mod calendar_merge;
mod capture;
pub mod clay;
mod commands;
mod connectivity;
pub mod context_provider;
pub mod db;
mod db_backup;
pub mod db_service;
pub mod demo;
mod devtools;
pub mod embeddings;
mod enrichment;
pub mod entity;
pub mod entity_io;
mod error;
mod executor;
mod export;
mod focus_capacity;
mod focus_prioritization;
pub mod glean;
mod google;
pub mod google_api;
pub mod google_drive;
pub mod granola;
pub mod gravatar;
pub mod helpers;
mod hygiene;
mod intel_queue;
pub mod intelligence;
pub mod observability;
pub mod json_loader;
mod latency;
pub mod linear;
pub mod meeting_prep_queue;
mod migrations;
mod notification;
pub mod oauth;
mod parser;
pub mod people;
pub mod prepare;
pub mod presets;
mod privacy;
pub mod proactive;
mod processor;
pub mod projects;
pub mod pty;
pub mod queries;
pub mod quill;
pub mod reports;
mod risk_briefing;
mod scheduler;
pub mod self_healing;
pub mod services;
pub mod signals;
pub mod state;
mod task_supervisor;
pub mod types;
pub mod util;
mod watcher;
mod workflow;

use std::sync::Arc;

use state::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tokio::sync::mpsc;

/// Channel buffer size for scheduler messages
const SCHEDULER_CHANNEL_SIZE: usize = 32;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logger — writes to stderr, filtered by RUST_LOG env var.
    // Default: info level for dailyos, warn for everything else.
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("dailyos_lib=info,warn"),
    )
    .format_timestamp_millis()
    .init();

    log::info!("DailyOS starting");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // Create shared state
            let state = Arc::new(AppState::new());
            state.set_app_handle(app.handle().clone());

            // One-time filesystem hardening: permissions + Time Machine exclusion
            if let Some(home) = dirs::home_dir() {
                let dailyos_dir = home.join(".dailyos");
                if dailyos_dir.is_dir() {
                    db::hardening::harden_data_directory(&dailyos_dir);
                }
            }

            // One-time migration: move Gravatar API key from config.json to Keychain
            gravatar::keychain::migrate_from_config(&state);

            // Initialize async DbService (read/write separated connections).
            // Skip when startup recovery screens are active.
            if !state
                .encryption_key_missing
                .load(std::sync::atomic::Ordering::Relaxed)
                && !state.is_database_recovery_required()
            {
                let init_state = state.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = init_state.init_db_service().await {
                        log::warn!("DbService init failed: {e}. Falling back to sync mutex.");
                    } else {
                        log::info!("DbService initialized (1 writer + 2 readers)");
                        // drain persisted
                        // health_recompute_pending markers that survived a
                        // prior crash. Runs once on startup; failures leave
                        // markers in place for the next attempt.
                        let clock = crate::services::context::SystemClock;
                        let rng = crate::services::context::SystemRng;
                        let ext = crate::services::context::ExternalClients::default();
                        let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
                        crate::services::health_debouncer::drain_pending(&ctx, &init_state).await;

                        // Run the claims cutover (rekey
                        // m1-m8 to runtime dedup_key shape + JSON-blob
                        // mechanism 9 backfill + reconcile) once after the
                        // SQL migrations 129-131 land. Idempotent — guarded
                        // by `migration_state.dos7_cutover_completed_at`.
                        let workspace_root: std::path::PathBuf = init_state
                            .config_read_or_recover()
                            .ok()
                            .and_then(|c| c.as_ref().map(|cfg| std::path::PathBuf::from(&cfg.workspace_path)))
                            .unwrap_or_default();
                        if workspace_root.as_os_str().is_empty() {
                            log::debug!(
                                "[DOS-7 cutover] startup hook: workspace_path empty; skipping until configured"
                            );
                        } else {
                            let cutover_result = init_state
                                .db_write(move |db| {
                                    let clock = crate::services::context::SystemClock;
                                    let rng = crate::services::context::SystemRng;
                                    let ext = crate::services::context::ExternalClients::default();
                                    let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
                                    crate::services::claims_backfill::run_dos7_cutover_if_pending(
                                        &ctx,
                                        db,
                                        &workspace_root,
                                    )
                                    .map_err(|e| format!("DOS-7 cutover: {e}"))
                                })
                                .await;
                            match cutover_result {
                                Ok(Some(report)) => {
                                    log::info!(
                                        "[DOS-7 cutover] completed at startup: \
                                         schema_epoch {}→{}, \
                                         m1-m8 rekey {}/{}, \
                                         m9 inserted {}, \
                                         reconcile findings {}",
                                        report.schema_epoch_before,
                                        report.schema_epoch_after,
                                        report.rekey_report.rows_rewritten,
                                        report.rekey_report.rows_examined,
                                        report.json_blob_report.claims_inserted,
                                        report.reconcile_findings,
                                    );
                                }
                                Ok(None) => {
                                    log::debug!(
                                        "[DOS-7 cutover] startup hook: already complete or claims schema absent"
                                    );
                                }
                                Err(e) => {
                                    log::warn!(
                                        "[DOS-7 cutover] startup hook failed: {e}; legacy is_suppressed remains authoritative until next startup retry"
                                    );
                                }
                            }
                        }
                    }
                });
            } else {
                log::warn!("DbService init skipped: startup recovery required");
            }

            // Initialize embedding model asynchronously (nomic-embed-text-v1.5).
            // Downloads ~137MB on first run, caches in ~/.dailyos/models/.
            // Runs in background so the window appears immediately. The embedding
            // processor has a 20-second startup delay and checks is_ready() before
            // processing, so there's no race condition.
            {
                let model = state.embedding_model.clone();
                tauri::async_runtime::spawn(async move {
                    let models_dir = dirs::home_dir()
                        .unwrap_or_default()
                        .join(".dailyos")
                        .join("models");
                    match tokio::task::spawn_blocking(move || model.initialize(models_dir)).await {
                        Ok(Ok(())) => log::info!("Embedding model ready (background init)"),
                        Ok(Err(e)) => log::warn!("Embedding model unavailable: {}", e),
                        Err(e) => log::warn!("Embedding model init panicked: {}", e),
                    }
                });
            }

            // Create channel for scheduler -> executor communication
            let (scheduler_tx, scheduler_rx) = mpsc::channel(SCHEDULER_CHANNEL_SIZE);

            // Store sender in app state for manual triggers
            app.manage(SchedulerSender(scheduler_tx.clone()));

            // Manage the state
            app.manage(state.clone());
            app.manage(crate::services::ServiceLayer::new(state.clone()));

            // Defer startup workspace sync/indexing so app setup stays responsive.
            let startup_state = state.clone();
            tauri::async_runtime::spawn_blocking(move || {
                crate::state::run_startup_sync(&startup_state);
            });

            // Spawn scheduler
            let scheduler_state = state.clone();
            let scheduler_sender = scheduler_tx.clone();
            let scheduler_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let scheduler =
                    scheduler::Scheduler::new(scheduler_state, scheduler_sender, scheduler_handle);
                scheduler.run().await;
            });

            // Spawn executor
            let executor_state = state.clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let executor = executor::Executor::new(executor_state, app_handle);
                executor.run(scheduler_rx).await;
            });

            // Start inbox file watcher
            watcher::start_watcher(state.clone(), app.handle().clone());

            // Spawn calendar poller (Phase 3A) — supervised
            let poller_state = state.clone();
            let poller_handle = app.handle().clone();
            task_supervisor::spawn_supervised("CalendarPoller", move || {
                let s = poller_state.clone();
                let h = poller_handle.clone();
                async move { google::run_calendar_poller(s, h).await }
            });

            // Spawn email poller — supervised
            let email_poller_state = state.clone();
            let email_poller_handle = app.handle().clone();
            task_supervisor::spawn_supervised("EmailPoller", move || {
                let s = email_poller_state.clone();
                let h = email_poller_handle.clone();
                async move { google::run_email_poller(s, h).await }
            });

            // Spawn capture detection loop (Phase 3B) — supervised
            let capture_state = state.clone();
            let capture_handle = app.handle().clone();
            task_supervisor::spawn_supervised("CaptureLoop", move || {
                let s = capture_state.clone();
                let h = capture_handle.clone();
                async move { capture::run_capture_loop(s, h).await }
            });

            // Spawn intelligence enrichment processor  — supervised
            let intel_state = state.clone();
            let intel_handle = app.handle().clone();
            task_supervisor::spawn_supervised("IntelProcessor", move || {
                let s = intel_state.clone();
                let h = intel_handle.clone();
                async move { intel_queue::run_intel_processor(s, h).await }
            });

            // Spawn meeting prep queue processor — supervised
            let prep_state = state.clone();
            let prep_handle = app.handle().clone();
            task_supervisor::spawn_supervised("MeetingPrepProcessor", move || {
                let s = prep_state.clone();
                let h = prep_handle.clone();
                async move { meeting_prep_queue::run_meeting_prep_processor(s, h).await }
            });

            // Spawn background embedding processor (Sprint 26) — supervised
            let embedding_state = state.clone();
            let embedding_handle = app.handle().clone();
            task_supervisor::spawn_supervised("EmbeddingProcessor", move || {
                let s = embedding_state.clone();
                let h = embedding_handle.clone();
                async move {
                    processor::embeddings::run_embedding_processor(s, h).await;
                }
            });

            // Spawn hygiene scanner loop (ADR-0058) — supervised
            let hygiene_state = state.clone();
            let hygiene_handle = app.handle().clone();
            task_supervisor::spawn_supervised("HygieneLoop", move || {
                let s = hygiene_state.clone();
                let h = hygiene_handle.clone();
                async move { hygiene::run_hygiene_loop(s, h).await }
            });

            // Spawn Quill transcript poller — supervised
            let quill_state = state.clone();
            let quill_handle = app.handle().clone();
            task_supervisor::spawn_supervised("QuillPoller", move || {
                let s = quill_state.clone();
                let h = quill_handle.clone();
                async move { quill::poller::run_quill_poller(s, h).await }
            });

            // Spawn Granola transcript poller  — supervised
            let granola_state = state.clone();
            let granola_handle = app.handle().clone();
            task_supervisor::spawn_supervised("GranolaPoller", move || {
                let s = granola_state.clone();
                let h = granola_handle.clone();
                async move { granola::poller::run_granola_poller(s, h).await }
            });

            // Spawn unified enrichment processor (Clay + Gravatar) — supervised
            let enrichment_state = state.clone();
            task_supervisor::spawn_supervised("EnrichmentProcessor", move || {
                let s = enrichment_state.clone();
                async move { enrichment::run_enrichment_processor(s).await }
            });

            // Spawn Linear sync poller  — supervised
            let linear_state = state.clone();
            task_supervisor::spawn_supervised("LinearPoller", move || {
                let s = linear_state.clone();
                async move { linear::poller::run_linear_poller(s).await }
            });

            // Spawn Google Drive poller  — supervised
            let drive_state = state.clone();
            task_supervisor::spawn_supervised("DrivePoller", move || {
                let s = drive_state.clone();
                async move { google_drive::poller::run_drive_poller(s).await }
            });

            // legacy entity resolution trigger removed. Entity
            // linking now runs on every calendar poll via
            // `services::entity_linking::calendar_adapter::evaluate_meeting`.

            // Tier 4: one-shot post-upgrade sweep that self-corrects
            // weak-primary entity links (rule:P5 / rule:P11) whose graph
            // version has drifted since the evidence-hierarchy fix shipped.
            // Gated by entity_graph_version.last_migration_sweep_at so it
            // runs at most once per upgrade. Fire-and-forget — failures
            // just log a warning, the periodic graph-drift pass handles
            // anything this sweep missed.
            let sweep_state = state.clone();
            tauri::async_runtime::spawn(async move {
                // Tiny delay so the app UI lands first.
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                match services::entity_linking::rescan::run_one_shot_upgrade_sweep(
                    sweep_state,
                )
                .await
                {
                    Ok(n) => {
                        if n > 0 {
                            log::info!("entity_linking one-shot sweep: re-evaluated {n} weak primaries");
                        }
                    }
                    Err(e) => log::warn!("entity_linking one-shot sweep failed: {e}"),
                }
            });

            // meeting-type reclassification runs on every boot so
            // existing meetings get re-labelled whenever attendee evidence
            // changes (stakeholder added, domain registered, relationship
            // flipped). Ungated — the entity-linking one-shot sweep above
            // only handles primary-entity drift, not meeting_type, and the
            // function is idempotent + cheap. Fire-and-forget.
            let reclassify_state = state.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(35)).await;
                let result = reclassify_state
                    .db_write(|db| {
                        db.reclassify_meeting_types_from_attendees()
                            .map_err(|e| e.to_string())
                    })
                    .await;
                match result {
                    Ok(n) if n > 0 => log::info!(
                        "reclassify_meeting_types_from_attendees: re-labelled {n} meetings"
                    ),
                    Ok(_) => {}
                    Err(e) => log::warn!("reclassify_meeting_types_from_attendees failed: {e}"),
                }
            });

            // stakeholder-derived domain backfill runs on every boot so
            // new users AND existing users self-heal any account whose stakeholder
            // graph has domains that aren't yet registered. Idempotent — repeat
            // runs on unchanged data are cheap. Spawned ~40s after init to avoid
            // UI contention while still fronting the first calendar poll.
            let stakeholder_domains_state = state.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(40)).await;
                let user_domains = crate::state::load_config()
                    .map(|c| c.resolved_user_domains())
                    .unwrap_or_default();
                let result = stakeholder_domains_state
                    .db_write(move |db| {
                        crate::services::entity_linking::stakeholder_domains::backfill_domains_for_all_accounts(
                            db,
                            &user_domains,
                        )
                    })
                    .await;
                match result {
                    Ok((touched, new)) if new > 0 => log::info!(
                        "stakeholder_domains boot sweep: {} new domains across {} accounts",
                        new,
                        touched
                    ),
                    Ok(_) => log::debug!(
                        "stakeholder_domains boot sweep: no new domains to register"
                    ),
                    Err(e) => log::warn!("stakeholder_domains boot sweep failed: {e}"),
                }
            });

            // Create tray menu
            let open_item = MenuItem::with_id(app, "open", "Open DailyOS", true, None::<&str>)?;
            let run_now_item =
                MenuItem::with_id(app, "run_now", "Run Briefing Now", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_item, &run_now_item, &quit_item])?;

            // Build tray icon with macOS template image
            let tray_icon = tauri::image::Image::from_bytes(include_bytes!(
                "../icons/tray-iconTemplate@2x.png"
            ))?;
            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "run_now" => {
                        if let Some(sender) = app.try_state::<SchedulerSender>() {
                            let _ = executor::request_workflow_execution(
                                &sender.0,
                                types::WorkflowId::Today,
                            );
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Handle window close: hide instead of quit + track focus for lock
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                let activity_tracker = state.clone();
                window.on_window_event(move |event| match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                    tauri::WindowEvent::Focused(true) => {
                        {
                            let mut ls = activity_tracker.lock_state.lock();
                            ls.last_activity = std::time::Instant::now();
                        }
                    }
                    _ => {}
                });
            }

            // Spawn app lock idle timer
            let lock_state_timer = state.clone();
            let lock_handle_timer = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

                    let timeout_mins = {
                        let config = lock_state_timer.config.read();
                        config
                            .as_ref()
                            .and_then(|c| c.app_lock_timeout_minutes)
                            .unwrap_or(0)
                    };

                    if timeout_mins == 0 {
                        continue; // Disabled
                    }

                    // Single lock acquisition for read + conditional write
                    let should_lock = {
                        let ls = lock_state_timer.lock_state.lock();
                        if ls.is_locked {
                            false // Already locked
                        } else {
                            ls.last_activity.elapsed()
                                >= std::time::Duration::from_secs(u64::from(timeout_mins) * 60)
                        }
                    };

                    if should_lock {
                        {
                            let mut ls = lock_state_timer.lock_state.lock();
                            ls.is_locked = true;
                        }
                        let _ = lock_handle_timer.emit("app-locked", ());
                        log::info!("App locked after {} minutes idle", timeout_mins);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Core
            commands::get_config,
            commands::reload_configuration,
            commands::get_dashboard_data,
            commands::run_workflow,
            commands::get_workflow_status,
            commands::get_execution_history,
            commands::get_next_run_time,
            commands::get_meeting_intelligence,
            commands::refresh_meeting_briefing,
            commands::generate_meeting_intelligence,
            commands::enrich_meeting_background,
            commands::get_meeting_prep,
            commands::backfill_prep_semantics,
            commands::get_all_actions,
            commands::get_all_emails,
            commands::get_emails_enriched,
            commands::get_email_sync_status,
            commands::update_email_entity,
            // entity linking read + manual overrides + admin
            commands::get_linked_entities_for_owner,
            commands::rebuild_account_domains,
            commands::set_entity_link_primary,
            commands::dismiss_entity_link,
            commands::restore_entity_link,
            commands::dismiss_email_signal,
            commands::get_entity_emails,
            commands::get_inbox_files,
            commands::get_inbox_file_content,
            commands::process_inbox_file,
            commands::process_all_inbox,
            commands::enrich_inbox_file,
            commands::copy_to_inbox,
            commands::list_meeting_preps,
            commands::set_profile,
            commands::set_entity_mode,
            commands::set_workspace_path,
            commands::set_developer_mode,
            commands::set_personality,
            commands::set_text_scale,
            commands::set_ai_model,
            commands::reset_ai_models_to_recommended,
            commands::set_google_poll_settings,
            commands::set_hygiene_config,
            commands::set_daily_ai_budget,
            commands::set_notification_config,
            commands::set_schedule,
            commands::get_actions_from_db,
            commands::complete_action,
            commands::reopen_action,
            commands::accept_suggested_action,
            commands::reject_suggested_action,
            commands::dismiss_suggested_action,
            commands::attach_meeting_transcript_text,
            commands::mark_reply_sent,
            commands::dismiss_gone_quiet,
            commands::archive_email,
            commands::unarchive_email,
            commands::unsuppress_email,
            commands::pin_email,
            commands::promote_commitment_to_action,
            commands::dismiss_email_item,
            commands::list_dismissed_email_items,
            commands::reset_email_preferences,
            commands::resolve_decision,
            commands::get_suggested_actions,
            // DOS Work-tab Phase 3: per-account Work chapter reads
            commands::get_account_commitments,
            commands::get_account_suggestions,
            commands::get_account_recently_landed,
            commands::get_meeting_history,
            commands::get_meeting_history_detail,
            commands::search_meetings,
            commands::get_action_detail,
            commands::backfill_historical_meetings,
            commands::backfill_account_domains,
            commands::recover_archived_transcripts,
            // Phase 3.0: Google Auth
            commands::get_google_auth_status,
            commands::start_google_auth,
            commands::disconnect_google,
            // Phase 3A: Calendar
            commands::get_calendar_events,
            commands::get_current_meeting,
            commands::get_next_meeting,
            // Phase 3B: Post-Meeting Capture
            commands::capture_meeting_outcome,
            commands::dismiss_meeting_prompt,
            commands::get_capture_settings,
            commands::set_capture_enabled,
            commands::set_capture_delay,
            // Phase 3C: Weekly Planning
            commands::get_week_data,
            commands::get_live_proactive_suggestions,
            commands::refresh_meeting_preps,
            // Transcript Intake & Meeting Outcomes
            commands::attach_meeting_transcript,
            commands::reprocess_meeting_transcript,
            commands::get_meeting_outcomes,
            commands::get_meeting_post_intelligence,
            commands::update_capture,
            commands::update_action_priority,
            // Manual Action CRUD
            commands::create_action,
            commands::update_action,
            // Executive Intelligence
            commands::get_executive_intelligence,
            // I6: Processing History
            commands::get_processing_history,
            // Email Refresh
            commands::refresh_emails,
            commands::sync_email_inbox_presence,
            // Archive low-priority emails
            commands::archive_low_priority_emails,
            commands::retry_failed_emails,
            // actionable failure UX
            commands::list_permanently_failed_emails,
            commands::skip_failed_emails,
            // Onboarding / Demo / App State
            commands::install_demo_data,
            commands::clear_demo_data,
            commands::get_app_state,
            commands::set_tour_completed,
            commands::set_wizard_completed,
            commands::set_wizard_step,
            commands::populate_workspace,
            commands::set_user_profile,
            // User Entity
            commands::get_user_entity,
            commands::update_user_entity_field,
            commands::get_user_context_entries,
            commands::create_user_context_entry,
            commands::update_user_context_entry,
            commands::delete_user_context_entry,
            commands::get_entity_context_entries,
            commands::create_entity_context_entry,
            commands::update_entity_context_entry,
            commands::delete_entity_context_entry,
            commands::process_user_attachment,
            commands::get_internal_team_setup_status,
            commands::create_internal_organization,
            commands::get_onboarding_priming_context,
            commands::check_claude_status,
            commands::launch_claude_login,
            commands::clear_claude_status_cache,
            commands::install_claude_cli,
            commands::get_latency_rollups,
            commands::get_ai_usage_diagnostics,
            commands::install_inbox_sample,
            commands::get_frequent_correspondents,
            // Dev Tools
            commands::dev_apply_scenario,
            commands::dev_get_state,
            commands::dev_run_today_mechanical,
            commands::dev_run_today_full,
            commands::dev_restore_live,
            commands::dev_purge_mock_data,
            commands::dev_clean_artifacts,
            commands::dev_set_auth_override,
            commands::dev_onboarding_scenario,
            // Cross-entity contamination audit + cleanup
            commands::devtools_audit_cross_contamination,
            commands::devtools_clear_contaminated_enrichment,
            // Meeting-Entity M2M
            commands::link_meeting_entity,
            commands::unlink_meeting_entity,
            // meeting entity dismissal dictionary
            commands::dismiss_meeting_entity,
            commands::restore_meeting_entity,
            commands::get_meeting_entities,
            commands::update_meeting_entity,
            // Additive multi-entity link/unlink
            commands::add_meeting_entity,
            commands::remove_meeting_entity,
            // Entity keyword management
            commands::remove_project_keyword,
            commands::remove_account_keyword,
            // Person Creation
            commands::create_person,
            commands::merge_people,
            commands::delete_person,
            // People
            commands::get_people,
            commands::get_person_detail,
            commands::search_people,
            commands::update_person,
            commands::link_person_entity,
            commands::unlink_person_entity,
            commands::get_people_for_entity,
            commands::get_meeting_attendees,
            // Entity Enrichment
            commands::enrich_account,
            commands::enrich_person,
            // Content Index
            commands::get_entity_files,
            commands::index_entity_files,
            commands::reveal_in_finder,
            commands::export_briefing_html,
            commands::chat_query_entity,
            commands::chat_search_content,
            commands::chat_get_briefing,
            commands::chat_list_entities,
            // Account Dashboards
            commands::get_accounts_list,
            commands::get_accounts_for_picker,
            commands::get_child_accounts_list,
            commands::get_account_ancestors,
            commands::get_descendant_accounts,
            commands::get_account_detail,
            commands::get_account_team,
            commands::update_account_field,
            commands::update_technical_footprint_field,
            commands::set_user_health_sentiment,
            commands::update_latest_sentiment_note,
            commands::snooze_triage_item,
            commands::resolve_triage_item,
            commands::list_triage_snoozes,
            commands::confirm_lifecycle_change,
            commands::correct_lifecycle_change,
            commands::correct_account_product,
            commands::accept_account_field_conflict,
            commands::dismiss_account_field_conflict,
            commands::update_account_notes,
            commands::update_account_programs,
            commands::add_account_team_member,
            commands::set_team_member_role,
            commands::remove_account_team_member,
            // Person-first stakeholder commands
            commands::get_person_stakeholder_roles,
            commands::update_stakeholder_engagement,
            commands::update_stakeholder_assessment,
            commands::add_stakeholder_role,
            commands::remove_stakeholder_role,
            commands::get_stakeholder_suggestions,
            commands::accept_stakeholder_suggestion,
            commands::dismiss_stakeholder_suggestion,
            // pending stakeholder review queue
            commands::get_pending_stakeholder_suggestions,
            commands::confirm_pending_stakeholder,
            commands::dismiss_pending_stakeholder,
            commands::create_account,
            commands::create_child_account,
            commands::create_team,
            commands::backfill_internal_meeting_associations,
            // Project Dashboards
            commands::get_projects_list,
            commands::get_project_detail,
            commands::get_child_projects_list,
            commands::get_project_ancestors,
            commands::create_project,
            commands::update_project_field,
            commands::update_project_notes,
            commands::enrich_project,
            // Database Backup & Rebuild
            commands::backup_database,
            commands::rebuild_database,
            commands::get_database_recovery_status,
            commands::list_database_backups,
            commands::restore_database_from_backup,
            commands::start_fresh_database,
            commands::export_database_copy,
            commands::get_database_info,
            // Hygiene
            commands::get_hygiene_report,
            commands::get_intelligence_hygiene_status,
            commands::get_hygiene_narrative,
            commands::run_hygiene_scan_now,
            // Duplicate People Detection
            commands::get_duplicate_people,
            commands::get_duplicate_people_for_person,
            // Archive / Unarchive Entities
            commands::archive_account,
            commands::archive_project,
            commands::archive_person,
            commands::get_archived_accounts,
            commands::get_archived_projects,
            commands::get_archived_people,
            // Account Merge
            commands::merge_accounts,
            // Account Recovery
            commands::restore_account,
            // Multi-Domain Config
            commands::set_user_domains,
            // Bulk Entity Creation
            commands::bulk_create_accounts,
            commands::bulk_create_projects,
            // Account Events
            commands::record_account_event,
            commands::get_account_events,
            commands::create_objective,
            commands::update_objective,
            commands::complete_objective,
            commands::abandon_objective,
            commands::delete_objective,
            commands::create_milestone,
            commands::update_milestone,
            commands::complete_milestone,
            commands::skip_milestone,
            commands::delete_milestone,
            commands::link_action_to_objective,
            commands::unlink_action_from_objective,
            commands::reorder_objectives,
            commands::reorder_milestones,
            commands::get_objective_suggestions,
            commands::create_objective_from_suggestion,
            commands::list_success_plan_templates,
            commands::apply_success_plan_template,
            // User Agenda + Notes (ADR-0065)
            commands::apply_meeting_prep_prefill,
            commands::generate_meeting_agenda_message_draft,
            commands::update_meeting_user_agenda,
            commands::update_meeting_user_notes,
            commands::update_meeting_prep_field,
            // Risk Briefing
            commands::generate_risk_briefing,
            commands::get_risk_briefing,
            // Regression guard: Risk briefing retry (surfaces failed jobs)
            commands::retry_risk_briefing,
            // Reports (v0.15.0)
            commands::generate_report,
            commands::get_report,
            commands::get_reports_for_entity,
            commands::save_report,
            // Intelligence Field Editing
            commands::update_intelligence_field,
            commands::dismiss_intelligence_item,
            commands::update_stakeholders,
            // Recommended Actions from Intelligence
            commands::track_recommendation,
            commands::dismiss_recommendation,
            commands::mark_commitment_done,
            commands::create_person_from_stakeholder,
            // MCP: Claude Desktop (ADR-0075)
            commands::configure_claude_desktop,
            commands::get_claude_desktop_status,
            // Cowork Plugins
            commands::export_cowork_plugin,
            commands::get_cowork_plugins_status,
            // Quill MCP Integration
            commands::get_quill_status,
            commands::set_quill_enabled,
            commands::test_quill_connection,
            commands::get_quill_sync_states,
            commands::set_quill_poll_interval,
            commands::start_quill_backfill,
            commands::trigger_quill_sync_for_meeting,
            // Granola Integration
            commands::get_granola_status,
            commands::trigger_granola_sync_for_meeting,
            commands::set_granola_enabled,
            commands::set_granola_poll_interval,
            commands::start_granola_backfill,
            commands::test_granola_cache,
            // Gravatar MCP Integration
            commands::get_gravatar_status,
            commands::set_gravatar_enabled,
            commands::set_gravatar_api_key,
            commands::fetch_gravatar,
            commands::bulk_fetch_gravatars,
            commands::get_person_avatar,
            // Clay Integration  via Smithery Connect
            commands::get_clay_status,
            commands::set_clay_enabled,
            commands::set_clay_api_key,
            commands::set_clay_auto_enrich,
            commands::test_clay_connection,
            commands::enrich_person_from_clay,
            commands::enrich_account_from_clay,
            commands::start_clay_bulk_enrich,
            commands::get_enrichment_log,
            commands::detect_smithery_settings,
            commands::save_smithery_api_key,
            commands::set_smithery_connection,
            commands::disconnect_smithery,
            commands::get_smithery_status,
            // Linear Integration
            commands::get_linear_status,
            commands::set_linear_enabled,
            commands::set_linear_api_key,
            commands::test_linear_connection,
            commands::start_linear_sync,
            commands::get_linear_recent_issues,
            commands::get_linear_entity_links,
            commands::get_linear_projects,
            commands::create_linear_entity_link,
            commands::run_linear_auto_link,
            commands::delete_linear_entity_link,
            // /51: Push Action to Linear
            commands::get_linear_teams,
            commands::push_action_to_linear,
            // Role Presets
            commands::set_role,
            commands::get_active_preset,
            commands::get_available_presets,
            // Entity Metadata
            commands::update_entity_metadata,
            commands::get_entity_metadata,
            // Email Disposition Correction
            commands::correct_email_disposition,
            // Meeting Timeline
            commands::get_meeting_timeline,
            // Person Relationships (ADR-0088)
            commands::upsert_person_relationship,
            commands::delete_person_relationship,
            commands::get_person_relationships,
            // Google Drive Connector
            commands::get_google_access_token,
            commands::get_google_client_id,
            commands::get_google_drive_status,
            commands::set_google_drive_enabled,
            commands::trigger_drive_sync_now,
            commands::import_google_drive_file,
            commands::add_google_drive_watch,
            commands::remove_google_drive_watch,
            commands::get_google_drive_watches,
            // iCloud Workspace Warning
            commands::check_icloud_warning,
            commands::dismiss_icloud_warning,
            // App Lock
            commands::get_lock_status,
            commands::get_encryption_key_status,
            commands::lock_app,
            commands::unlock_app,
            commands::set_lock_timeout,
            commands::signal_user_activity,
            commands::signal_window_focus,
            // Audit Log
            commands::get_audit_log_records,
            commands::export_audit_log,
            commands::verify_audit_log_integrity,
            // ADR-0095: Context Mode (Local / Glean)
            commands::get_context_mode,
            commands::set_context_mode,
            commands::start_glean_auth,
            commands::get_glean_auth_status,
            commands::get_glean_token_health,
            commands::disconnect_glean,
            // Glean Agent Validation Spike (temporary dev exploration)
            commands::dev_explore_glean_tools,
            // Discover accounts from Glean
            commands::discover_accounts_from_glean,
            commands::import_account_from_glean,
            // Onboarding — Three Connectors
            commands::onboarding_import_accounts,
            commands::onboarding_prefill_profile,
            commands::onboarding_enrichment_status,
            // Ephemeral Account Query via Glean
            commands::query_ephemeral_account,
            // Global Search
            commands::search_global,
            commands::rebuild_search_index,
            // Connectivity
            commands::get_sync_freshness,
            // Data Export
            commands::export_all_data,
            // Privacy Controls
            commands::get_data_summary,
            commands::clear_intelligence,
            commands::delete_all_data,
            // Feature Flags
            commands::get_feature_flags,
            // DB Growth Monitoring
            commands::get_db_growth_report,
            // Health Scoring Recalibration
            commands::bulk_recompute_health,
            // Meeting Intelligence
            commands::get_prediction_scorecard,
            commands::get_meeting_continuity_thread,
            // Intelligence Quality Feedback
            commands::submit_intelligence_feedback,
            commands::get_entity_feedback,
            commands::get_entity_suppressions,
            // Consolidated intelligence correction
            commands::submit_intelligence_correction,
            // Feedback & Suppression Diagnostics
            commands::get_feedback_diagnostics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Wrapper for scheduler sender to allow Tauri state management
pub struct SchedulerSender(pub mpsc::Sender<scheduler::SchedulerMessage>);

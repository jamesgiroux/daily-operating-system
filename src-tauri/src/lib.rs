// I149: Suppress dead_code — serde struct fields appear unused to the compiler but
// are required for forward-compatible JSON deserialization. Parser/notification
// functions are reserved for future use.
#![allow(dead_code)]
// Devtools mock data uses large tuple types for seed fixtures.
#![allow(clippy::type_complexity)]

pub mod accounts;
mod audit;
mod backfill_meetings;
mod calendar_merge;
mod capture;
pub mod clay;
mod commands;
pub mod linear;
pub mod db;
mod db_backup;
mod devtools;
pub mod embeddings;
pub mod entity;
pub mod entity_intel;
pub mod entity_io;
mod error;
mod executor;
mod focus_capacity;
mod focus_prioritization;
mod google;
pub mod google_api;
pub mod gravatar;
pub mod helpers;
mod hygiene;
mod intel_queue;
pub mod intelligence;
pub mod json_loader;
mod latency;
mod migrations;
mod notification;
mod parser;
pub mod people;
pub mod prepare;
pub mod presets;
pub mod proactive;
mod processor;
pub mod projects;
mod pty;
pub mod granola;
pub mod quill;
pub mod queries;
mod risk_briefing;
mod scheduler;
pub mod signals;
pub mod state;
pub mod types;
pub mod util;
mod watcher;
mod workflow;

use std::sync::Arc;

use state::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tokio::sync::mpsc;

/// Channel buffer size for scheduler messages
const SCHEDULER_CHANNEL_SIZE: usize = 32;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // Create shared state
            let state = Arc::new(AppState::new());

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

            // Defer startup workspace sync/indexing so app setup stays responsive.
            let startup_state = state.clone();
            tauri::async_runtime::spawn_blocking(move || {
                crate::state::run_startup_sync(&startup_state);
            });

            // Spawn scheduler
            let scheduler_state = state.clone();
            let scheduler_sender = scheduler_tx.clone();
            tauri::async_runtime::spawn(async move {
                let scheduler = scheduler::Scheduler::new(scheduler_state, scheduler_sender);
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

            // Spawn calendar poller (Phase 3A)
            let poller_state = state.clone();
            let poller_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                google::run_calendar_poller(poller_state, poller_handle).await;
            });

            // Spawn capture detection loop (Phase 3B)
            let capture_state = state.clone();
            let capture_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                capture::run_capture_loop(capture_state, capture_handle).await;
            });

            // Spawn intelligence enrichment processor (I132)
            let intel_state = state.clone();
            let intel_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                intel_queue::run_intel_processor(intel_state, intel_handle).await;
            });

            // Spawn background embedding processor (Sprint 26)
            let embedding_state = state.clone();
            let embedding_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                processor::embeddings::run_embedding_processor(embedding_state, embedding_handle)
                    .await;
            });

            // Spawn hygiene scanner loop (I145 — ADR-0058)
            let hygiene_state = state.clone();
            let hygiene_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                hygiene::run_hygiene_loop(hygiene_state, hygiene_handle).await;
            });

            // Spawn Quill transcript poller
            let quill_state = state.clone();
            let quill_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                quill::poller::run_quill_poller(quill_state, quill_handle).await;
            });

            // Spawn Granola transcript poller (I226)
            let granola_state = state.clone();
            let granola_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                granola::poller::run_granola_poller(granola_state, granola_handle).await;
            });

            // Spawn Gravatar avatar fetcher (I229)
            let gravatar_state = state.clone();
            tauri::async_runtime::spawn(async move {
                gravatar::client::run_gravatar_fetcher(gravatar_state).await;
            });

            // Spawn Clay enrichment poller (I228)
            let clay_state = state.clone();
            let clay_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                clay::poller::run_clay_poller(clay_state, clay_handle).await;
            });

            // Spawn Linear sync poller (I346)
            let linear_state = state.clone();
            tauri::async_runtime::spawn(async move {
                linear::poller::run_linear_poller(linear_state).await;
            });

            // Spawn event-driven entity resolution trigger (I308)
            let entity_res_state = state.clone();
            tauri::async_runtime::spawn(async move {
                signals::event_trigger::run_entity_resolution_trigger(entity_res_state).await;
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

            // Handle window close: hide instead of quit
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });
            }

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
            commands::get_meeting_prep,
            commands::backfill_prep_semantics,
            commands::get_all_actions,
            commands::get_all_emails,
            commands::get_emails_enriched,
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
            commands::set_ai_model,
            commands::set_hygiene_config,
            commands::set_schedule,
            commands::get_actions_from_db,
            commands::complete_action,
            commands::reopen_action,
            commands::accept_proposed_action,
            commands::reject_proposed_action,
            commands::get_proposed_actions,
            commands::get_meeting_history,
            commands::get_meeting_history_detail,
            commands::search_meetings,
            commands::get_action_detail,
            commands::backfill_historical_meetings,
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
            commands::retry_week_enrichment,
            // I44/I45: Transcript Intake & Meeting Outcomes
            commands::attach_meeting_transcript,
            commands::get_meeting_outcomes,
            commands::update_capture,
            commands::update_action_priority,
            // I127/I128: Manual Action CRUD
            commands::create_action,
            commands::update_action,
            // I42: Executive Intelligence
            commands::get_executive_intelligence,
            // I6: Processing History
            commands::get_processing_history,
            // I20: Email Refresh
            commands::refresh_emails,
            // I144: Archive low-priority emails
            commands::archive_low_priority_emails,
            // I39: Feature Toggles
            commands::get_features,
            commands::set_feature_enabled,
            // Onboarding
            commands::install_demo_data,
            commands::populate_workspace,
            commands::set_user_profile,
            commands::get_internal_team_setup_status,
            commands::create_internal_organization,
            commands::get_onboarding_priming_context,
            commands::check_claude_status,
            commands::get_latency_rollups,
            commands::install_inbox_sample,
            commands::get_frequent_correspondents,
            // Dev Tools
            commands::dev_apply_scenario,
            commands::dev_get_state,
            commands::dev_run_today_mechanical,
            commands::dev_run_today_full,
            commands::dev_run_week_mechanical,
            commands::dev_run_week_full,
            commands::dev_restore_live,
            commands::dev_purge_mock_data,
            commands::dev_clean_artifacts,
            // I52: Meeting-Entity M2M
            commands::link_meeting_entity,
            commands::unlink_meeting_entity,
            commands::get_meeting_entities,
            commands::update_meeting_entity,
            // I184: Additive multi-entity link/unlink
            commands::add_meeting_entity,
            commands::remove_meeting_entity,
            // I305: Entity keyword management
            commands::remove_project_keyword,
            commands::remove_account_keyword,
            // I129: Person Creation
            commands::create_person,
            commands::merge_people,
            commands::delete_person,
            // I51: People
            commands::get_people,
            commands::get_person_detail,
            commands::search_people,
            commands::update_person,
            commands::link_person_entity,
            commands::unlink_person_entity,
            commands::get_people_for_entity,
            commands::get_meeting_attendees,
            // I74/I136: Entity Enrichment
            commands::enrich_account,
            commands::enrich_person,
            // I124: Content Index
            commands::get_entity_files,
            commands::index_entity_files,
            commands::reveal_in_finder,
            commands::export_briefing_html,
            commands::chat_query_entity,
            commands::chat_search_content,
            commands::chat_get_briefing,
            commands::chat_list_entities,
            // I72: Account Dashboards
            commands::get_accounts_list,
            commands::get_accounts_for_picker,
            commands::get_child_accounts_list,
            commands::get_account_ancestors,
            commands::get_descendant_accounts,
            commands::get_account_detail,
            commands::get_account_team,
            commands::update_account_field,
            commands::update_account_notes,
            commands::update_account_programs,
            commands::add_account_team_member,
            commands::remove_account_team_member,
            commands::create_account,
            commands::create_child_account,
            commands::create_team,
            commands::backfill_internal_meeting_associations,
            // I50: Project Dashboards
            commands::get_projects_list,
            commands::get_project_detail,
            commands::create_project,
            commands::update_project_field,
            commands::update_project_notes,
            commands::enrich_project,
            // I76: Database Backup & Rebuild
            commands::backup_database,
            commands::rebuild_database,
            // I148: Hygiene
            commands::get_hygiene_report,
            commands::get_intelligence_hygiene_status,
            commands::get_hygiene_narrative,
            commands::run_hygiene_scan_now,
            // I172: Duplicate People Detection
            commands::get_duplicate_people,
            commands::get_duplicate_people_for_person,
            // I176: Archive / Unarchive Entities
            commands::archive_account,
            commands::archive_project,
            commands::archive_person,
            commands::get_archived_accounts,
            commands::get_archived_projects,
            commands::get_archived_people,
            // I198: Account Merge
            commands::merge_accounts,
            // I199: Account Recovery
            commands::restore_account,
            // I171: Multi-Domain Config
            commands::set_user_domains,
            // I162: Bulk Entity Creation
            commands::bulk_create_accounts,
            commands::bulk_create_projects,
            // I143: Account Events
            commands::record_account_event,
            commands::get_account_events,
            // I194: User Agenda + Notes (ADR-0065)
            commands::apply_meeting_prep_prefill,
            commands::generate_meeting_agenda_message_draft,
            commands::update_meeting_user_agenda,
            commands::update_meeting_user_notes,
            // Risk Briefing
            commands::generate_risk_briefing,
            commands::get_risk_briefing,
            commands::save_risk_briefing,
            // I261: Intelligence Field Editing
            commands::update_intelligence_field,
            commands::update_stakeholders,
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
            // Granola Integration (I226)
            commands::get_granola_status,
            commands::set_granola_enabled,
            commands::set_granola_poll_interval,
            commands::start_granola_backfill,
            commands::test_granola_cache,
            // Gravatar MCP Integration (I229)
            commands::get_gravatar_status,
            commands::set_gravatar_enabled,
            commands::set_gravatar_api_key,
            commands::fetch_gravatar,
            commands::bulk_fetch_gravatars,
            commands::get_person_avatar,
            // Clay Integration (I228)
            commands::get_clay_status,
            commands::set_clay_enabled,
            commands::set_clay_api_key,
            commands::set_clay_auto_enrich,
            commands::test_clay_connection,
            commands::enrich_person_from_clay,
            commands::enrich_account_from_clay,
            commands::start_clay_bulk_enrich,
            commands::get_enrichment_log,
            // Linear Integration (I346)
            commands::get_linear_status,
            commands::set_linear_enabled,
            commands::set_linear_api_key,
            commands::test_linear_connection,
            commands::start_linear_sync,
            // I309: Role Presets
            commands::set_role,
            commands::get_active_preset,
            commands::get_available_presets,
            // I311: Entity Metadata
            commands::update_entity_metadata,
            commands::get_entity_metadata,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Wrapper for scheduler sender to allow Tauri state management
pub struct SchedulerSender(pub mpsc::Sender<scheduler::SchedulerMessage>);

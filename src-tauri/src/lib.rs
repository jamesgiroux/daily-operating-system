mod calendar_merge;
mod capture;
mod commands;
mod db;
mod error;
mod executor;
mod google;
mod json_loader;
mod notification;
mod parser;
mod processor;
mod pty;
mod scheduler;
mod state;
mod types;
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
        .setup(|app| {
            // Create shared state
            let state = Arc::new(AppState::new());

            // Create channel for scheduler -> executor communication
            let (scheduler_tx, scheduler_rx) = mpsc::channel(SCHEDULER_CHANNEL_SIZE);

            // Store sender in app state for manual triggers
            app.manage(SchedulerSender(scheduler_tx.clone()));

            // Manage the state
            app.manage(state.clone());

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
            commands::get_meeting_prep,
            commands::get_all_actions,
            commands::get_all_emails,
            commands::get_inbox_files,
            commands::get_inbox_file_content,
            commands::process_inbox_file,
            commands::process_all_inbox,
            commands::enrich_inbox_file,
            commands::copy_to_inbox,
            commands::list_meeting_preps,
            commands::set_profile,
            commands::get_actions_from_db,
            commands::complete_action,
            commands::get_meeting_history,
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
            commands::get_week_planning_state,
            commands::get_week_prep_data,
            commands::submit_week_priorities,
            commands::submit_focus_blocks,
            commands::skip_week_planning,
            commands::get_focus_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Wrapper for scheduler sender to allow Tauri state management
pub struct SchedulerSender(pub mpsc::Sender<scheduler::SchedulerMessage>);

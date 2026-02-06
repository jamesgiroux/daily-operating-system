//! File watcher for _inbox/ directory
//!
//! Watches the workspace's _inbox/ directory for file changes and emits
//! Tauri events so the frontend can update the inbox badge and file list.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::parser::count_inbox;
use crate::state::AppState;

/// Debounce window for file system events
const DEBOUNCE_MS: u64 = 500;

/// Payload emitted to the frontend on inbox changes
#[derive(Debug, Clone, serde::Serialize)]
pub struct InboxUpdate {
    pub count: usize,
}

/// Start watching the _inbox/ directory for changes.
///
/// Spawns a background task that:
/// 1. Resolves workspace path from config
/// 2. Creates _inbox/ if it doesn't exist
/// 3. Watches for create/modify/delete events
/// 4. Debounces rapid changes (500ms window)
/// 5. Emits `inbox-updated` Tauri event with current count
///
/// Returns immediately. The watcher runs for the lifetime of the app.
pub fn start_watcher(state: Arc<AppState>, app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Get workspace path from config
        let workspace = match get_workspace_from_config(&state) {
            Some(path) => path,
            None => {
                log::warn!("Watcher: no workspace configured, inbox watcher disabled");
                return;
            }
        };

        let inbox_dir = workspace.join("_inbox");

        // Create _inbox/ if it doesn't exist
        if !inbox_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&inbox_dir) {
                log::warn!("Watcher: failed to create _inbox/: {}", e);
                return;
            }
            log::info!("Watcher: created _inbox/ directory");
        }

        // Emit initial count so sidebar badge is correct on launch
        let initial_count = count_inbox(&workspace);
        let _ = app_handle.emit("inbox-updated", InboxUpdate { count: initial_count });

        // Channel for forwarding notify events to the async debouncer
        let (fs_tx, mut fs_rx) = mpsc::channel::<()>(64);

        // Create the filesystem watcher
        let tx = fs_tx.clone();
        let mut watcher = match RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    // Only care about create, modify, remove events
                    if matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                    ) {
                        // Filter out hidden/temp files, accept all others.
                        // The inbox pipeline handles .md, .txt, .vtt, .srt, and more.
                        let dominated_by_relevant = event.paths.is_empty()
                            || event.paths.iter().any(|p| {
                                p.file_name()
                                    .and_then(|n| n.to_str())
                                    .map(|n| !n.starts_with('.'))
                                    .unwrap_or(false)
                            });

                        if dominated_by_relevant {
                            let _ = tx.try_send(());
                        }
                    }
                }
            },
            notify::Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Watcher: failed to create filesystem watcher: {}", e);
                return;
            }
        };

        // Start watching _inbox/
        if let Err(e) = watcher.watch(&inbox_dir, RecursiveMode::NonRecursive) {
            log::error!("Watcher: failed to watch {}: {}", inbox_dir.display(), e);
            return;
        }

        log::info!("Watcher: watching {} for changes", inbox_dir.display());

        // Debounce loop: coalesce rapid events into a single update
        loop {
            // Wait for an event
            if fs_rx.recv().await.is_none() {
                break; // Channel closed, watcher dropped
            }

            // Debounce: drain any events that arrive within the window
            sleep(Duration::from_millis(DEBOUNCE_MS)).await;
            while fs_rx.try_recv().is_ok() {}

            // Count current inbox files and emit
            let count = count_inbox(&workspace);
            log::debug!("Watcher: inbox changed, count={}", count);
            let _ = app_handle.emit("inbox-updated", InboxUpdate { count });
        }

        log::info!("Watcher: stopped");
    });
}

/// Read workspace path from the config state
fn get_workspace_from_config(state: &AppState) -> Option<PathBuf> {
    let guard = state.config.lock().ok()?;
    let config = guard.as_ref()?;
    let path = PathBuf::from(&config.workspace_path);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

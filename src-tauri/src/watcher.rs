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

use crate::accounts;
use crate::parser::count_inbox;
use crate::people;
use crate::projects;
use crate::state::AppState;

/// Debounce window for file system events
const DEBOUNCE_MS: u64 = 500;

/// Payload emitted to the frontend on inbox changes
#[derive(Debug, Clone, serde::Serialize)]
pub struct InboxUpdate {
    pub count: usize,
}

/// Distinguishes which watched directory fired
#[derive(Debug, Clone)]
enum WatchSource {
    Inbox,
    People(PathBuf),
    Accounts(PathBuf),
    Projects(PathBuf),
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
        let people_dir = workspace.join("People");
        let accounts_dir = workspace.join("Accounts");
        let projects_dir = workspace.join("Projects");

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
        let (fs_tx, mut fs_rx) = mpsc::channel::<WatchSource>(64);

        // Create the filesystem watcher
        let tx = fs_tx.clone();
        let inbox_dir_clone = inbox_dir.clone();
        let people_dir_clone = people_dir.clone();
        let accounts_dir_clone = accounts_dir.clone();
        let projects_dir_clone = projects_dir.clone();
        let mut watcher = match RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    // Only care about create, modify, remove events
                    if matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                    ) {
                        // Filter out hidden/temp files
                        let dominated_by_relevant = event.paths.is_empty()
                            || event.paths.iter().any(|p| {
                                p.file_name()
                                    .and_then(|n| n.to_str())
                                    .map(|n| !n.starts_with('.'))
                                    .unwrap_or(false)
                            });

                        if !dominated_by_relevant {
                            return;
                        }

                        // Determine source based on path
                        let is_people = event.paths.iter().any(|p| {
                            p.starts_with(&people_dir_clone)
                                && p.file_name()
                                    .and_then(|n| n.to_str())
                                    .is_some_and(|n| n == "person.json")
                        });

                        let is_accounts = event.paths.iter().any(|p| {
                            p.starts_with(&accounts_dir_clone)
                                && p.file_name()
                                    .and_then(|n| n.to_str())
                                    .is_some_and(|n| n == "dashboard.json")
                        });

                        let is_projects = event.paths.iter().any(|p| {
                            p.starts_with(&projects_dir_clone)
                                && p.file_name()
                                    .and_then(|n| n.to_str())
                                    .is_some_and(|n| n == "dashboard.json")
                        });

                        if is_people {
                            // Send the changed person.json path
                            if let Some(path) = event.paths.iter().find(|p| {
                                p.file_name()
                                    .and_then(|n| n.to_str())
                                    .is_some_and(|n| n == "person.json")
                            }) {
                                let _ = tx.try_send(WatchSource::People(path.clone()));
                            }
                        } else if is_accounts {
                            if let Some(path) = event.paths.iter().find(|p| {
                                p.starts_with(&accounts_dir_clone)
                                    && p.file_name()
                                        .and_then(|n| n.to_str())
                                        .is_some_and(|n| n == "dashboard.json")
                            }) {
                                let _ = tx.try_send(WatchSource::Accounts(path.clone()));
                            }
                        } else if is_projects {
                            if let Some(path) = event.paths.iter().find(|p| {
                                p.starts_with(&projects_dir_clone)
                                    && p.file_name()
                                        .and_then(|n| n.to_str())
                                        .is_some_and(|n| n == "dashboard.json")
                            }) {
                                let _ = tx.try_send(WatchSource::Projects(path.clone()));
                            }
                        } else if event
                            .paths
                            .iter()
                            .any(|p| p.starts_with(&inbox_dir_clone))
                        {
                            let _ = tx.try_send(WatchSource::Inbox);
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

        // Start watching People/ (recursive to catch People/*/person.json)
        if people_dir.exists() {
            if let Err(e) = watcher.watch(&people_dir, RecursiveMode::Recursive) {
                log::warn!(
                    "Watcher: failed to watch People/: {}. People sync disabled.",
                    e
                );
            } else {
                log::info!("Watcher: watching {} for changes", people_dir.display());
            }
        }

        // Start watching Accounts/ (recursive to catch Accounts/*/dashboard.json)
        if accounts_dir.exists() {
            if let Err(e) = watcher.watch(&accounts_dir, RecursiveMode::Recursive) {
                log::warn!(
                    "Watcher: failed to watch Accounts/: {}. Account sync disabled.",
                    e
                );
            } else {
                log::info!("Watcher: watching {} for changes", accounts_dir.display());
            }
        }

        // Start watching Projects/ (recursive to catch Projects/*/dashboard.json)
        if projects_dir.exists() {
            if let Err(e) = watcher.watch(&projects_dir, RecursiveMode::Recursive) {
                log::warn!(
                    "Watcher: failed to watch Projects/: {}. Project sync disabled.",
                    e
                );
            } else {
                log::info!("Watcher: watching {} for changes", projects_dir.display());
            }
        }

        // Debounce loop: coalesce rapid events into a single update
        let mut inbox_dirty = false;
        let mut people_dirty: Vec<PathBuf> = Vec::new();
        let mut accounts_dirty: Vec<PathBuf> = Vec::new();
        let mut projects_dirty: Vec<PathBuf> = Vec::new();
        loop {
            // Wait for an event
            let source = match fs_rx.recv().await {
                Some(s) => s,
                None => break, // Channel closed, watcher dropped
            };

            match source {
                WatchSource::Inbox => inbox_dirty = true,
                WatchSource::People(p) => {
                    if !people_dirty.contains(&p) {
                        people_dirty.push(p);
                    }
                }
                WatchSource::Accounts(p) => {
                    if !accounts_dirty.contains(&p) {
                        accounts_dirty.push(p);
                    }
                }
                WatchSource::Projects(p) => {
                    if !projects_dirty.contains(&p) {
                        projects_dirty.push(p);
                    }
                }
            }

            // Debounce: drain any events that arrive within the window
            sleep(Duration::from_millis(DEBOUNCE_MS)).await;
            while let Ok(src) = fs_rx.try_recv() {
                match src {
                    WatchSource::Inbox => inbox_dirty = true,
                    WatchSource::People(p) => {
                        if !people_dirty.contains(&p) {
                            people_dirty.push(p);
                        }
                    }
                    WatchSource::Accounts(p) => {
                        if !accounts_dirty.contains(&p) {
                            accounts_dirty.push(p);
                        }
                    }
                    WatchSource::Projects(p) => {
                        if !projects_dirty.contains(&p) {
                            projects_dirty.push(p);
                        }
                    }
                }
            }

            // Process inbox changes
            if inbox_dirty {
                let count = count_inbox(&workspace);
                log::debug!("Watcher: inbox changed, count={}", count);
                let _ = app_handle.emit("inbox-updated", InboxUpdate { count });
                inbox_dirty = false;
            }

            // Process people changes (I51: external person.json edits)
            if !people_dirty.is_empty() {
                handle_people_changes(&people_dirty, &state, &workspace);
                let _ = app_handle.emit("people-updated", ());
                people_dirty.clear();
            }

            // Process account changes (I75: external dashboard.json edits)
            if !accounts_dirty.is_empty() {
                handle_account_changes(&accounts_dirty, &state, &workspace);
                let _ = app_handle.emit("accounts-updated", ());
                accounts_dirty.clear();
            }

            // Process project changes (I50: external dashboard.json edits)
            if !projects_dirty.is_empty() {
                handle_project_changes(&projects_dirty, &state, &workspace);
                let _ = app_handle.emit("projects-updated", ());
                projects_dirty.clear();
            }
        }

        log::info!("Watcher: stopped");
    });
}

/// Handle detected changes to People/*/person.json files (I51).
///
/// Reads the changed JSON files, syncs to SQLite, regenerates person.md.
fn handle_people_changes(paths: &[PathBuf], state: &AppState, workspace: &Path) {
    let db_guard = match state.db.lock().ok() {
        Some(g) => g,
        None => return,
    };
    let db = match db_guard.as_ref() {
        Some(db) => db,
        None => return,
    };

    let user_domain = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().and_then(|c| c.user_domain.clone()));

    for path in paths {
        if !path.exists() {
            continue;
        }

        match people::read_person_json(path) {
            Ok(people::ReadPersonResult { mut person, linked_entities }) => {
                // Classify relationship if unknown
                if person.relationship == "unknown" {
                    person.relationship = crate::util::classify_relationship(
                        &person.email,
                        user_domain.as_deref(),
                    );
                }

                if db.upsert_person(&person).is_ok() {
                    // Restore entity links from JSON (ADR-0048)
                    for entity_id in &linked_entities {
                        let _ = db.link_person_to_entity(
                            &person.id, entity_id, "associated",
                        );
                    }
                    let _ = people::write_person_markdown(workspace, &person, db);
                    log::info!(
                        "Watcher: synced external edit to {}",
                        path.display()
                    );
                }
            }
            Err(e) => {
                log::warn!("Watcher: failed to read {}: {}", path.display(), e);
            }
        }
    }
}

/// Handle detected changes to Accounts/*/dashboard.json files (I75).
///
/// Reads the changed JSON files, syncs to SQLite, regenerates dashboard.md.
fn handle_account_changes(paths: &[PathBuf], state: &AppState, workspace: &Path) {
    let db_guard = match state.db.lock().ok() {
        Some(g) => g,
        None => return,
    };
    let db = match db_guard.as_ref() {
        Some(db) => db,
        None => return,
    };

    for path in paths {
        if !path.exists() {
            continue;
        }

        match accounts::read_account_json(path) {
            Ok(accounts::ReadAccountResult { account, json }) => {
                if db.upsert_account(&account).is_ok() {
                    let _ = accounts::write_account_markdown(
                        workspace, &account, Some(&json), db,
                    );
                    log::info!(
                        "Watcher: synced external edit to {}",
                        path.display()
                    );
                }
            }
            Err(e) => {
                log::warn!("Watcher: failed to read {}: {}", path.display(), e);
            }
        }
    }
}

/// Handle detected changes to Projects/*/dashboard.json files (I50).
///
/// Reads the changed JSON files, syncs to SQLite, regenerates dashboard.md.
fn handle_project_changes(paths: &[PathBuf], state: &AppState, workspace: &Path) {
    let db_guard = match state.db.lock().ok() {
        Some(g) => g,
        None => return,
    };
    let db = match db_guard.as_ref() {
        Some(db) => db,
        None => return,
    };

    for path in paths {
        if !path.exists() {
            continue;
        }

        match projects::read_project_json(path) {
            Ok(projects::ReadProjectResult { project, json }) => {
                if db.upsert_project(&project).is_ok() {
                    let _ = projects::write_project_markdown(
                        workspace, &project, Some(&json), db,
                    );
                    log::info!(
                        "Watcher: synced external edit to {}",
                        path.display()
                    );
                }
            }
            Err(e) => {
                log::warn!("Watcher: failed to read {}: {}", path.display(), e);
            }
        }
    }
}

/// Read workspace path from the config state
fn get_workspace_from_config(state: &AppState) -> Option<PathBuf> {
    let guard = state.config.read().ok()?;
    let config = guard.as_ref()?;
    let path = PathBuf::from(&config.workspace_path);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

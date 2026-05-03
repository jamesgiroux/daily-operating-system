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

/// Payload emitted to the frontend when content files change in entity dirs.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentChangePayload {
    pub entity_ids: Vec<String>,
    pub count: usize,
}

/// Distinguishes which watched directory fired
#[derive(Debug, Clone)]
enum WatchSource {
    Inbox,
    People(PathBuf),
    Accounts(PathBuf),
    AccountContent(PathBuf),
    Projects(PathBuf),
    ProjectContent(PathBuf),
    UserAttachments(PathBuf),
    /// New directory created under Accounts/ or Projects
    NewEntityDir,
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
        let user_attachments_dir = workspace.join("_user").join("attachments");

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
        let _ = app_handle.emit(
            "inbox-updated",
            InboxUpdate {
                count: initial_count,
            },
        );

        // Channel for forwarding notify events to the async debouncer
        let (fs_tx, mut fs_rx) = mpsc::channel::<WatchSource>(64);

        // Create the filesystem watcher
        let tx = fs_tx.clone();
        let inbox_dir_clone = inbox_dir.clone();
        let people_dir_clone = people_dir.clone();
        let accounts_dir_clone = accounts_dir.clone();
        let projects_dir_clone = projects_dir.clone();
        let user_attachments_dir_clone = user_attachments_dir.clone();
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

                        // Non-dashboard files in Accounts/ dirs
                        let is_account_content = !is_accounts
                            && event.paths.iter().any(|p| {
                                p.starts_with(&accounts_dir_clone)
                                    && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                                        n != "dashboard.json"
                                            && n != "dashboard.md"
                                            && !n.starts_with('.')
                                            && !n.starts_with('_')
                                    })
                                    && p.is_file()
                            });

                        let is_projects = event.paths.iter().any(|p| {
                            p.starts_with(&projects_dir_clone)
                                && p.file_name()
                                    .and_then(|n| n.to_str())
                                    .is_some_and(|n| n == "dashboard.json")
                        });

                        // Non-dashboard files in Projects/ dirs
                        let is_project_content = !is_projects
                            && event.paths.iter().any(|p| {
                                p.starts_with(&projects_dir_clone)
                                    && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                                        n != "dashboard.json"
                                            && n != "dashboard.md"
                                            && n != "intelligence.json"
                                            && !n.starts_with('.')
                                            && !n.starts_with('_')
                                    })
                                    && p.is_file()
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
                        } else if is_account_content {
                            // Content file changed in an account dir
                            for path in &event.paths {
                                if path.starts_with(&accounts_dir_clone) {
                                    let _ = tx.try_send(WatchSource::AccountContent(path.clone()));
                                }
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
                        } else if is_project_content {
                            // Content file changed in a project dir
                            for path in &event.paths {
                                if path.starts_with(&projects_dir_clone) {
                                    let _ = tx.try_send(WatchSource::ProjectContent(path.clone()));
                                }
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
                        // Detect new or renamed directories under Accounts/ or Projects.
                        // Create: user made a new folder. Rename/Modify: user renamed it
                        // in Finder. Both trigger a workspace resync so the DB stays in
                        // sync with the filesystem. The sync is idempotent — existing
                        // accounts with matching IDs won't be duplicated, and renamed
                        // directories get a fresh bootstrap under the new name.
                        } else if matches!(
                            event.kind,
                            EventKind::Create(_) | EventKind::Modify(_)
                        ) && event.paths.iter().any(|p| {
                            p.is_dir()
                                && (p.parent() == Some(accounts_dir_clone.as_path())
                                    || p.parent() == Some(projects_dir_clone.as_path()))
                                && p.file_name()
                                    .and_then(|n| n.to_str())
                                    .is_some_and(|n| {
                                        !n.starts_with('_')
                                            && !n.starts_with('.')
                                            && n != "Internal"
                                    })
                        }) {
                            let _ = tx.try_send(WatchSource::NewEntityDir);
                        } else if event
                            .paths
                            .iter()
                            .any(|p| p.starts_with(&user_attachments_dir_clone))
                        {
                            for path in &event.paths {
                                if path.starts_with(&user_attachments_dir_clone) && path.is_file() {
                                    let _ = tx.try_send(WatchSource::UserAttachments(path.clone()));
                                }
                            }
                        } else if event.paths.iter().any(|p| p.starts_with(&inbox_dir_clone)) {
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

        // Start watching _user/attachments/ for user context documents
        if user_attachments_dir.exists() {
            if let Err(e) = watcher.watch(&user_attachments_dir, RecursiveMode::NonRecursive) {
                log::warn!(
                    "Watcher: failed to watch _user/attachments/: {}. User attachment auto-processing disabled.",
                    e
                );
            } else {
                log::info!(
                    "Watcher: watching {} for changes",
                    user_attachments_dir.display()
                );
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
        let mut account_content_dirty: Vec<PathBuf> = Vec::new();
        let mut projects_dirty: Vec<PathBuf> = Vec::new();
        let mut project_content_dirty: Vec<PathBuf> = Vec::new();
        let mut user_attachments_dirty: Vec<PathBuf> = Vec::new();
        let mut new_entity_dirty = false;
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
                WatchSource::AccountContent(p) => {
                    if !account_content_dirty.contains(&p) {
                        account_content_dirty.push(p);
                    }
                }
                WatchSource::Projects(p) => {
                    if !projects_dirty.contains(&p) {
                        projects_dirty.push(p);
                    }
                }
                WatchSource::ProjectContent(p) => {
                    if !project_content_dirty.contains(&p) {
                        project_content_dirty.push(p);
                    }
                }
                WatchSource::UserAttachments(p) => {
                    if !user_attachments_dirty.contains(&p) {
                        user_attachments_dirty.push(p);
                    }
                }
                WatchSource::NewEntityDir => new_entity_dirty = true,
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
                    WatchSource::AccountContent(p) => {
                        if !account_content_dirty.contains(&p) {
                            account_content_dirty.push(p);
                        }
                    }
                    WatchSource::Projects(p) => {
                        if !projects_dirty.contains(&p) {
                            projects_dirty.push(p);
                        }
                    }
                    WatchSource::ProjectContent(p) => {
                        if !project_content_dirty.contains(&p) {
                            project_content_dirty.push(p);
                        }
                    }
                    WatchSource::UserAttachments(p) => {
                        if !user_attachments_dirty.contains(&p) {
                            user_attachments_dirty.push(p);
                        }
                    }
                    WatchSource::NewEntityDir => new_entity_dirty = true,
                }
            }

            // Process inbox changes
            if inbox_dirty {
                let count = count_inbox(&workspace);
                log::debug!("Watcher: inbox changed, count={}", count);
                let _ = app_handle.emit("inbox-updated", InboxUpdate { count });
                inbox_dirty = false;
            }

            // Process people changes (external person.json edits)
            if !people_dirty.is_empty() {
                handle_people_changes(&people_dirty, &state, &workspace);
                let _ = app_handle.emit("people-updated", ());
                people_dirty.clear();
            }

            // Process account changes (external dashboard.json edits)
            if !accounts_dirty.is_empty() {
                handle_account_changes(&accounts_dirty, &state, &workspace);
                let _ = app_handle.emit("accounts-updated", ());
                accounts_dirty.clear();
            }

            // Process account content changes (non-dashboard files)
            if !account_content_dirty.is_empty() {
                let payload =
                    handle_account_content_changes(&account_content_dirty, &state, &workspace);
                if let Some(ref payload) = payload {
                    // Queue intelligence refresh for affected entities
                    for entity_id in &payload.entity_ids {
                        state.embedding_queue.enqueue(
                            crate::processor::embeddings::EmbeddingRequest {
                                entity_id: entity_id.clone(),
                                entity_type: "account".to_string(),
                                requested_at: std::time::Instant::now(),
                            },
                        );
                        let _ = state                            .intel_queue
                            .enqueue(crate::intel_queue::IntelRequest::new(
                                entity_id.clone(),
                                "account".to_string(),
                                crate::intel_queue::IntelPriority::ContentChange,
                            ));
                    }
                    state.integrations.embedding_queue_wake.notify_one();
                    state.integrations.intel_queue_wake.notify_one();
                    let _ = app_handle.emit("content-changed", payload.clone());
                }
                account_content_dirty.clear();
            }

            // Process project changes (external dashboard.json edits)
            if !projects_dirty.is_empty() {
                handle_project_changes(&projects_dirty, &state, &workspace);
                let _ = app_handle.emit("projects-updated", ());
                projects_dirty.clear();
            }

            // Process project content changes (non-dashboard files in Projects)
            if !project_content_dirty.is_empty() {
                let payload =
                    handle_project_content_changes(&project_content_dirty, &state, &workspace);
                if let Some(ref payload) = payload {
                    // Queue intelligence refresh for affected project entities
                    for entity_id in &payload.entity_ids {
                        state.embedding_queue.enqueue(
                            crate::processor::embeddings::EmbeddingRequest {
                                entity_id: entity_id.clone(),
                                entity_type: "project".to_string(),
                                requested_at: std::time::Instant::now(),
                            },
                        );
                        let _ = state                            .intel_queue
                            .enqueue(crate::intel_queue::IntelRequest::new(
                                entity_id.clone(),
                                "project".to_string(),
                                crate::intel_queue::IntelPriority::ContentChange,
                            ));
                    }
                    state.integrations.embedding_queue_wake.notify_one();
                    state.integrations.intel_queue_wake.notify_one();
                    let _ = app_handle.emit("content-changed", payload.clone());
                }
                project_content_dirty.clear();
            }

            // Process user attachment changes (_user/attachments)
            if !user_attachments_dirty.is_empty() {
                handle_user_attachment_changes(&user_attachments_dirty, &state, &workspace);
                user_attachments_dirty.clear();
            }

            // New entity directory discovered — lightweight bootstrap only.
            // Creates DB records and writes dashboard files so the account/project
            // appears in the UI immediately. Does NOT trigger expensive PTY/intel
            // operations — those happen lazily on next scheduled intel cycle or
            // when the user opens the entity detail page.
            //
            // Extra 3s delay: when a user creates a directory in Finder, the OS
            // fires Create with a temporary name ("untitled folder"), then a Rename
            // event once the user types the real name. Without this delay we'd
            // bootstrap with the wrong name and write dashboard files before the
            // rename completes.
            if new_entity_dirty {
                // Short delay to let Finder complete rename operations.
                // Without this, we bootstrap "untitled folder" before the
                // user finishes typing the real name.
                sleep(Duration::from_secs(5)).await;
                log::info!("DOS-44: New entity directory detected, running workspace sync");
                if let Ok(db) = crate::db::ActionDb::open() {
                    let accounts_synced =
                        crate::accounts::sync_accounts_from_workspace(&workspace, &db)
                            .unwrap_or(0);
                    let projects_synced =
                        crate::projects::sync_projects_from_workspace(&workspace, &db)
                            .unwrap_or(0);
                    if accounts_synced > 0 {
                        log::info!(
                            "DOS-44: Bootstrapped {} new account(s) from workspace",
                            accounts_synced
                        );
                        let _ = app_handle.emit("accounts-updated", ());
                    }
                    if projects_synced > 0 {
                        log::info!(
                            "DOS-44: Bootstrapped {} new project(s) from workspace",
                            projects_synced
                        );
                        let _ = app_handle.emit("projects-updated", ());
                    }
                }
                new_entity_dirty = false;
            }
        }

        log::info!("Watcher: stopped");
    });
}

/// Handle detected changes to People/*/person.json files.
///
/// Reads the changed JSON files, syncs to SQLite, regenerates person.md.
fn handle_people_changes(paths: &[PathBuf], state: &AppState, workspace: &Path) {
    // Skip in dev DB mode
    if crate::db::is_dev_db_mode() {
        log::debug!("Watcher: skipping people sync — dev DB mode active");
        return;
    }

    // Own DB connection to avoid holding state.db Mutex during watcher I/O
    let db = match crate::db::ActionDb::open().ok() {
        Some(db) => db,
        None => return,
    };

    let user_domains = {
        let g = state.config.read();
        g.as_ref().map(|c| c.resolved_user_domains()).unwrap_or_default()
    };

    for path in paths {
        if !path.exists() {
            continue;
        }

        match people::read_person_json(path) {
            Ok(people::ReadPersonResult {
                mut person,
                linked_entities,
            }) => {
                // Classify relationship if unknown
                if person.relationship == "unknown" {
                    person.relationship =
                        crate::util::classify_relationship_multi(&person.email, &user_domains);
                }

                if db.upsert_person(&person).is_ok() {
                    // Restore entity links from JSON (ADR-0048)
                    for entity_id in &linked_entities {
                        let _ = db.link_person_to_entity(&person.id, entity_id, "associated");
                    }
                    let _ = people::write_person_markdown(workspace, &person, &db);
                    log::info!("Watcher: synced external edit to {}", path.display());
                }
            }
            Err(e) => {
                log::warn!("Watcher: failed to read {}: {}", path.display(), e);
            }
        }
    }
}

/// Handle detected changes to Accounts/*/dashboard.json files.
///
/// Reads the changed JSON files, syncs to SQLite, regenerates dashboard.md.
fn handle_account_changes(paths: &[PathBuf], _state: &AppState, workspace: &Path) {
    // Skip watcher sync when dev DB mode is active. The watcher watches
    // the live workspace, but the DB is pointing at the dev DB — syncing would
    // leak live account data into the dev sandbox.
    if crate::db::is_dev_db_mode() {
        log::debug!("Watcher: skipping account sync — dev DB mode active");
        return;
    }

    // Own DB connection to avoid holding state.db Mutex during watcher I/O
    let db = match crate::db::ActionDb::open().ok() {
        Some(db) => db,
        None => return,
    };

    for path in paths {
        if !path.exists() {
            continue;
        }

        match accounts::read_account_json(path) {
            Ok(accounts::ReadAccountResult { mut account, json }) => {
                // Preserve DB-authoritative fields that dashboard.json doesn't track.
                if let Ok(Some(existing)) = db.get_account(&account.id) {
                    account.name = existing.name;
                    account.account_type = existing.account_type;
                    account.archived = existing.archived;
                }
                if db.upsert_account(&account).is_ok() {
                    let _ = accounts::write_account_markdown(workspace, &account, Some(&json), &db);
                    log::info!("Watcher: synced external edit to {}", path.display());
                }
            }
            Err(e) => {
                log::warn!("Watcher: failed to read {}: {}", path.display(), e);
            }
        }
    }
}

/// Handle detected changes to Projects/*/dashboard.json files.
///
/// Reads the changed JSON files, syncs to SQLite, regenerates dashboard.md.
fn handle_project_changes(paths: &[PathBuf], _state: &AppState, workspace: &Path) {
    // Skip watcher sync in dev DB mode (same rationale as accounts above)
    if crate::db::is_dev_db_mode() {
        log::debug!("Watcher: skipping project sync — dev DB mode active");
        return;
    }

    // Own DB connection to avoid holding state.db Mutex during watcher I/O
    let db = match crate::db::ActionDb::open().ok() {
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
                    let _ = projects::write_project_markdown(workspace, &project, Some(&json), &db);
                    log::info!("Watcher: synced external edit to {}", path.display());
                }
            }
            Err(e) => {
                log::warn!("Watcher: failed to read {}: {}", path.display(), e);
            }
        }
    }
}

/// Handle detected changes to non-dashboard files in Accounts/ dirs.
///
/// Extracts affected account IDs from the file paths, syncs their content index,
/// and returns a payload for the frontend event.
fn handle_account_content_changes(
    paths: &[PathBuf],
    _state: &AppState,
    workspace: &Path,
) -> Option<ContentChangePayload> {
    // Skip in dev DB mode
    if crate::db::is_dev_db_mode() {
        return None;
    }

    // Own DB connection to avoid holding state.db Mutex during content indexing
    let db = crate::db::ActionDb::open().ok()?;

    let accounts_dir = workspace.join("Accounts");
    let mut affected_entity_ids = std::collections::HashSet::new();

    for path in paths {
        // Extract account dir name from path: Accounts/{name}/somefile.txt
        if let Ok(relative) = path.strip_prefix(&accounts_dir) {
            if let Some(account_dir_name) = relative.iter().next() {
                let name = account_dir_name.to_string_lossy();
                let id = crate::util::slugify(&name);
                affected_entity_ids.insert(id);
            }
        }
    }

    let mut total_changes = 0;
    for entity_id in &affected_entity_ids {
        if let Ok(Some(account)) = db.get_account(entity_id) {
            match accounts::sync_content_index_for_account(workspace, &db, &account) {
                Ok((added, updated, removed)) => {
                    total_changes += added + updated + removed;
                    log::debug!(
                        "Watcher: content index for '{}': +{} ~{} -{}",
                        account.name,
                        added,
                        updated,
                        removed
                    );
                }
                Err(e) => {
                    log::warn!(
                        "Watcher: content index sync failed for {}: {}",
                        entity_id,
                        e
                    );
                }
            }
        }
    }

    if total_changes > 0 {
        Some(ContentChangePayload {
            entity_ids: affected_entity_ids.into_iter().collect(),
            count: total_changes,
        })
    } else {
        None
    }
}

/// Handle detected changes to non-dashboard files in Projects/ dirs.
///
/// Parallel to `handle_account_content_changes` — extracts affected project IDs,
/// syncs their content index, and returns a payload for the frontend event.
fn handle_project_content_changes(
    paths: &[PathBuf],
    _state: &AppState,
    workspace: &Path,
) -> Option<ContentChangePayload> {
    // Skip in dev DB mode
    if crate::db::is_dev_db_mode() {
        return None;
    }

    // Own DB connection to avoid holding state.db Mutex during content indexing
    let db = crate::db::ActionDb::open().ok()?;

    let projects_dir = workspace.join("Projects");
    let mut affected_entity_ids = std::collections::HashSet::new();

    for path in paths {
        // Extract project dir name from path: Projects/{name}/somefile.txt
        if let Ok(relative) = path.strip_prefix(&projects_dir) {
            if let Some(project_dir_name) = relative.iter().next() {
                let name = project_dir_name.to_string_lossy();
                let id = crate::util::slugify(&name);
                affected_entity_ids.insert(id);
            }
        }
    }

    let mut total_changes = 0;
    for entity_id in &affected_entity_ids {
        if let Ok(Some(project)) = db.get_project(entity_id) {
            match projects::sync_content_index_for_project(workspace, &db, &project) {
                Ok((added, updated, removed)) => {
                    total_changes += added + updated + removed;
                    log::debug!(
                        "Watcher: content index for project '{}': +{} ~{} -{}",
                        project.name,
                        added,
                        updated,
                        removed
                    );
                }
                Err(e) => {
                    log::warn!(
                        "Watcher: content index sync failed for project {}: {}",
                        entity_id,
                        e
                    );
                }
            }
        }
    }

    if total_changes > 0 {
        Some(ContentChangePayload {
            entity_ids: affected_entity_ids.into_iter().collect(),
            count: total_changes,
        })
    } else {
        None
    }
}

/// Handle detected changes to _user/attachments/ files.
///
/// Processes new or modified user attachment files through the pipeline
/// and queues embedding generation.
fn handle_user_attachment_changes(paths: &[PathBuf], state: &AppState, workspace: &Path) {
    if crate::db::is_dev_db_mode() {
        log::debug!("Watcher: skipping user attachment processing — dev DB mode active");
        return;
    }

    let db = match crate::db::ActionDb::open().ok() {
        Some(db) => db,
        None => return,
    };

    for path in paths {
        if !path.exists() || !path.is_file() {
            continue;
        }

        let result = crate::processor::process_user_attachment(workspace, path, Some(&db));
        match &result {
            crate::processor::ProcessingResult::Routed { .. } => {
                log::info!("Watcher: processed user attachment {}", path.display());
                // Queue embedding generation
                state
                    .embedding_queue
                    .enqueue(crate::processor::embeddings::EmbeddingRequest {
                        entity_id: "user_context".to_string(),
                        entity_type: "user_context".to_string(),
                        requested_at: std::time::Instant::now(),
                    });
                state.integrations.embedding_queue_wake.notify_one();
            }
            crate::processor::ProcessingResult::Error { message } => {
                log::warn!(
                    "Watcher: failed to process user attachment {}: {}. Enqueuing for retry via embedding queue.",
                    path.display(),
                    message
                );
                // Acceptance criterion: Enqueue for retry — the next hygiene/embedding cycle will
                // re-attempt processing when the embedding worker picks up this request.
                state
                    .embedding_queue
                    .enqueue(crate::processor::embeddings::EmbeddingRequest {
                        entity_id: "user_context".to_string(),
                        entity_type: "user_context".to_string(),
                        requested_at: std::time::Instant::now(),
                    });
                state.integrations.embedding_queue_wake.notify_one();
            }
            _ => {}
        }
    }
}

/// Read workspace path from the config state
fn get_workspace_from_config(state: &AppState) -> Option<PathBuf> {
    let guard = state.config.read();
    let config = guard.as_ref()?;
    let path = PathBuf::from(&config.workspace_path);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

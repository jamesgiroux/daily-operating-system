//! Background polling for Google Drive changes (I426).

use std::sync::Arc;
use std::time::Duration;

use crate::state::AppState;
use crate::activity::adaptive_network_interval;
use super::client;
use super::sync;

/// Run the Google Drive poller background task.
///
/// Polls for changes to watched Drive sources and syncs them to entity Documents/ folders.
/// Uses 60-minute polling interval (configurable via DriveConfig).
pub async fn run_drive_poller(state: Arc<AppState>) {
    log::info!("GoogleDrivePoller: started");

    loop {
        // Check if Drive is enabled
        let enabled = state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|c| c.drive.enabled))
            .unwrap_or(false);

        if !enabled {
            // Drive is disabled, sleep longer
            tokio::time::sleep(Duration::from_secs(60)).await;
            continue;
        }

        // Check if user is authenticated with Google
        let authenticated = state
            .calendar
            .google_auth
            .lock()
            .map(|guard| matches!(*guard, crate::types::GoogleAuthStatus::Authenticated { .. }))
            .unwrap_or(false);

        if !authenticated {
            log::debug!("GoogleDrivePoller: not authenticated, skipping");
            tokio::time::sleep(Duration::from_secs(300)).await;
            continue;
        }

        // Get watched sources
        let sources = match state.db.lock() {
            Ok(db_guard) => {
                if let Some(db) = db_guard.as_ref() {
                    sync::get_all_watched_sources(db).unwrap_or_default()
                } else {
                    Vec::new()
                }
            }
            Err(_) => {
                log::warn!("GoogleDrivePoller: DB lock poisoned");
                Vec::new()
            }
        };

        if sources.is_empty() {
            // No watches, sleep longer
            tokio::time::sleep(Duration::from_secs(300)).await;
            continue;
        }

        // Sync each watched source
        for source in sources {
            if let Err(e) = sync_watched_source(&state, &source).await {
                log::warn!(
                    "GoogleDrivePoller: failed to sync {}: {}",
                    source.google_id,
                    e
                );
            }
        }

        // Use adaptive network interval (respects user activity)
        let interval = adaptive_network_interval(&state.activity);
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = state.integrations.drive_poller_wake.notified() => {
                // Manual wake-up signal (e.g., user clicked "Sync Now")
                log::debug!("GoogleDrivePoller: woken by manual trigger");
            }
        }
    }
}

/// Sync a single watched source from Google Drive.
async fn sync_watched_source(state: &Arc<AppState>, source: &sync::WatchedSource) -> Result<(), String> {
    // Get changes since last sync
    let page_token = source.changes_token.as_deref().unwrap_or("");
    let (changes, next_token) = client::get_changes(page_token).await?;

    if changes.is_empty() {
        return Ok(());
    }

    log::info!(
        "GoogleDrivePoller: syncing {} changes for {}",
        changes.len(),
        source.google_id
    );

    // Download and save files to entity Documents/ folder
    for change in changes {
        if change.removed {
            // File was deleted in Drive, could delete local copy
            log::info!("GoogleDrivePoller: file {} removed in Drive", change.file_id);
            continue;
        }

        if let Some(file) = &change.file {
            match download_and_save_file(state, source, file).await {
                Ok(path) => {
                    log::info!(
                        "GoogleDrivePoller: synced {} to {}",
                        file.name,
                        path.display()
                    );
                }
                Err(e) => {
                    log::warn!("GoogleDrivePoller: failed to download {}: {}", file.name, e);
                }
            }
        }
    }

    // Update the changes token
    if let Ok(db_guard) = state.db.lock() {
        if let Some(db) = db_guard.as_ref() {
            let _ = sync::mark_synced(db, &source.id, &next_token);
        }
    }

    Ok(())
}

/// Download a file and save it to the entity's Documents/ folder.
async fn download_and_save_file(
    state: &Arc<AppState>,
    source: &sync::WatchedSource,
    file: &client::DriveFile,
) -> Result<std::path::PathBuf, String> {
    let content = client::download_file_as_markdown(&file.id).await?;

    // Get workspace and entity path
    let workspace = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.workspace_path.clone()))
        .ok_or("Workspace not configured")?;

    let base_path = std::path::Path::new(&workspace);
    let entity_dir = base_path.join(&source.entity_type).join(&source.entity_id);
    let docs_dir = entity_dir.join("Documents");

    // Create Documents directory if needed
    std::fs::create_dir_all(&docs_dir)
        .map_err(|e| format!("Failed to create Documents directory: {}", e))?;

    // Save file
    let filename = format!(
        "{}.md",
        file.name.replace("/", "-").replace("\\", "-")
    );
    let file_path = docs_dir.join(&filename);

    std::fs::write(&file_path, &content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    // File watcher will detect the change and enqueue intel_queue automatically
    Ok(file_path)
}

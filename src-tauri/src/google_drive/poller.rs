//! Background polling for Google Drive changes.

use std::sync::Arc;
use std::time::Duration;

use super::client;
use super::sync;
use crate::activity::adaptive_network_interval;
use crate::state::AppState;

/// Run the Google Drive poller background task.
///
/// Polls for changes to watched Drive sources and syncs them to entity Documents/ folders.
/// Uses 60-minute polling interval (configurable via DriveConfig).
pub async fn run_drive_poller(state: Arc<AppState>) {
    log::info!("GoogleDrivePoller: started");

    // Drive always polls in Glean mode — additive strategy merges local + Glean signals.

    loop {
        // Dev mode isolation: pause background processing while dev sandbox is active
        if crate::db::is_dev_db_mode() {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            continue;
        }

        // Check if Drive is enabled
        let enabled = state
            .config
            .read()
            .as_ref()
            .map(|c| c.drive.enabled)
            .unwrap_or(false);

        if !enabled {
            // Drive is disabled, sleep longer
            tokio::time::sleep(Duration::from_secs(60)).await;
            continue;
        }

        // Check if user is authenticated with Google
        let authenticated = {
            let guard = state.calendar.google_auth.lock();
            matches!(*guard, crate::types::GoogleAuthStatus::Authenticated { .. })
        };

        if !authenticated {
            log::debug!("GoogleDrivePoller: not authenticated, skipping");
            tokio::time::sleep(Duration::from_secs(300)).await;
            continue;
        }

        // Get watched sources
        let sources = match crate::db::ActionDb::open() {
            Ok(db) => sync::get_all_watched_sources(&db).unwrap_or_default(),
            Err(e) => {
                log::warn!("GoogleDrivePoller: DB open failed: {e}");
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
async fn sync_watched_source(
    state: &Arc<AppState>,
    source: &sync::WatchedSource,
) -> Result<(), String> {
    if source.changes_token.is_none() {
        // Initial sync: download the file directly and get a start page token
        // for subsequent change-based polling.
        log::info!(
            "GoogleDrivePoller: initial sync for {} ({})",
            source.name,
            source.google_id
        );

        let content = client::download_file_as_markdown(&source.google_id).await?;
        let path = save_content_to_entity(state, source, &source.name, &content)?;
        log::info!(
            "GoogleDrivePoller: initial sync saved {} to {}",
            source.name,
            path.display()
        );

        // Get a start page token so future polls use the Changes API
        let start_token = client::get_start_page_token().await?;
        if let Ok(db) = crate::db::ActionDb::open() {
            let _ = sync::mark_synced(&db, &source.id, &start_token);
        }

        return Ok(());
    }

    // Subsequent syncs: poll for changes
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
            log::info!(
                "GoogleDrivePoller: file {} removed in Drive",
                change.file_id
            );
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
    if !next_token.is_empty() {
        if let Ok(db) = crate::db::ActionDb::open() {
            let _ = sync::mark_synced(&db, &source.id, &next_token);
        }
    }

    Ok(())
}

/// Save content to an entity's Documents/ folder as a markdown file.
pub fn save_to_entity_docs(
    workspace: &str,
    entity_type: &str,
    entity_id: &str,
    name: &str,
    content: &str,
) -> Result<std::path::PathBuf, String> {
    let base_path = std::path::Path::new(workspace);
    let docs_dir = base_path
        .join(entity_type)
        .join(entity_id)
        .join("Documents");

    std::fs::create_dir_all(&docs_dir)
        .map_err(|e| format!("Failed to create Documents directory: {}", e))?;

    let filename = format!(
        "{}.md",
        name.replace("/", "-").replace("\\", "-").replace(":", "-")
    );
    let file_path = docs_dir.join(&filename);

    std::fs::write(&file_path, content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(file_path)
}

/// Save content to the entity's Documents/ folder as a markdown file (watched source variant).
fn save_content_to_entity(
    state: &Arc<AppState>,
    source: &sync::WatchedSource,
    name: &str,
    content: &str,
) -> Result<std::path::PathBuf, String> {
    let workspace = state
        .config
        .read()
        .as_ref()
        .map(|c| c.workspace_path.clone())
        .ok_or("Workspace not configured")?;

    save_to_entity_docs(
        &workspace,
        &source.entity_type,
        &source.entity_id,
        name,
        content,
    )
}

/// Download a file and save it to the entity's Documents/ folder.
async fn download_and_save_file(
    state: &Arc<AppState>,
    source: &sync::WatchedSource,
    file: &client::DriveFile,
) -> Result<std::path::PathBuf, String> {
    let content = client::download_file_as_markdown(&file.id).await?;
    save_content_to_entity(state, source, &file.name, &content)
}

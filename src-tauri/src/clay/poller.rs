//! Background Clay enrichment poller.
//!
//! Runs as a long-lived async task: 60 s startup delay, then loops through
//! pending `clay_sync_state` rows and sweeps unenriched people. Follows the
//! same architectural pattern as the Quill poller (`quill/poller.rs`).

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::state::AppState;

// ---------------------------------------------------------------------------
// Main poller loop
// ---------------------------------------------------------------------------

/// Background Clay enrichment poller.
///
/// - 60 s startup delay
/// - Check `config.clay.enabled` + `api_key` present
/// - Process pending `clay_sync_state` rows
/// - Sweep: people with `last_enriched_at IS NULL` or older than 30 days
/// - Rate limit: 5 s pause between Clay MCP calls, `max_per_sweep` cap
/// - Sleep `sweep_interval_hours` between cycles
pub async fn run_clay_poller(state: Arc<AppState>, _app_handle: tauri::AppHandle) {
    // 60-second startup delay to let other subsystems initialize
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    loop {
        // Read Clay config from shared state
        let (enabled, api_key, sweep_interval, max_per_sweep) = {
            let config = state.config.read().ok();
            match config.as_ref().and_then(|g| g.as_ref()) {
                Some(c) => (
                    c.clay.enabled,
                    c.clay.api_key.clone(),
                    c.clay.sweep_interval_hours,
                    c.clay.max_per_sweep,
                ),
                None => (false, None, 24, 20),
            }
        };

        if !enabled || api_key.is_none() {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(300)) => {},
                _ = state.clay_poller_wake.notified() => {
                    log::info!("Clay poller: woken by bulk enrich signal (disabled path)");
                },
            }
            continue;
        }

        let api_key = api_key.unwrap();
        log::info!("Clay poller: starting enrichment sweep");

        // Connect to Clay MCP server
        match crate::clay::client::ClayClient::connect(&api_key).await {
            Ok(client) => {
                // Phase 1: process pending clay_sync_state rows
                let pending_ids = get_pending_sync_ids(&state, max_per_sweep as usize);

                let mut enriched: u32 = 0;
                for person_id in &pending_ids {
                    match crate::clay::enricher::enrich_person_from_clay_with_client(
                        &state, person_id, &client,
                    )
                    .await
                    {
                        Ok(result) => {
                            update_clay_sync_completed(&state, person_id);
                            log::info!(
                                "Clay enriched {}: {} fields updated",
                                person_id,
                                result.fields_updated.len()
                            );
                            enriched += 1;
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            update_clay_sync_failed(&state, person_id, &msg);
                            log::warn!("Clay enrichment failed for {}: {}", person_id, e);
                        }
                    }
                    // Rate limit: 5 seconds between MCP calls
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    if enriched >= max_per_sweep {
                        break;
                    }
                }

                // Phase 2: if under limit, sweep unenriched people
                if enriched < max_per_sweep {
                    let unenriched =
                        get_unenriched_people(&state, (max_per_sweep - enriched) as usize);
                    for person_id in &unenriched {
                        // Insert a sync state row so we track this attempt
                        insert_clay_sync(&state, person_id);

                        match crate::clay::enricher::enrich_person_from_clay_with_client(
                            &state, person_id, &client,
                        )
                        .await
                        {
                            Ok(result) => {
                                update_clay_sync_completed(&state, person_id);
                                log::info!(
                                    "Clay sweep enriched {}: {} fields",
                                    person_id,
                                    result.fields_updated.len()
                                );
                                enriched += 1;
                            }
                            Err(e) => {
                                let msg = e.to_string();
                                update_clay_sync_failed(&state, person_id, &msg);
                                log::warn!("Clay sweep failed for {}: {}", person_id, e);
                            }
                        }
                        // Rate limit: 5 seconds between MCP calls
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        if enriched >= max_per_sweep {
                            break;
                        }
                    }
                }

                client.disconnect().await;
                log::info!("Clay poller: sweep complete, enriched {} people", enriched);
            }
            Err(e) => {
                log::warn!("Clay poller: connection failed: {}", e);
            }
        }

        // Sleep until next sweep cycle, or wake immediately on bulk enrich signal
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(
                sweep_interval as u64 * 3600,
            )) => {},
            _ = state.clay_poller_wake.notified() => {
                log::info!("Clay poller: woken by bulk enrich signal");
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Database helpers
// ---------------------------------------------------------------------------

/// Query `clay_sync_state` for rows with `state = 'pending'` that have not
/// exceeded their max attempts, returning up to `limit` entity IDs.
fn get_pending_sync_ids(state: &AppState, limit: usize) -> Vec<String> {
    let db_guard = match state.db.lock() {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };
    let db = match db_guard.as_ref() {
        Some(d) => d,
        None => return Vec::new(),
    };

    let mut stmt = match db.conn_ref().prepare(
        "SELECT entity_id FROM clay_sync_state
         WHERE state = 'pending' AND attempts < max_attempts
         ORDER BY created_at ASC
         LIMIT ?1",
    ) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Clay poller: failed to query pending syncs: {}", e);
            return Vec::new();
        }
    };

    let rows = stmt
        .query_map(rusqlite::params![limit as i64], |row| row.get::<_, String>(0))
        .ok()
        .map(|iter| iter.filter_map(|r| r.ok()).collect::<Vec<_>>())
        .unwrap_or_default();

    rows
}

/// Query the `people` table for persons who have never been enriched or whose
/// last enrichment is older than 30 days, returning up to `limit` IDs.
fn get_unenriched_people(state: &AppState, limit: usize) -> Vec<String> {
    let db_guard = match state.db.lock() {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };
    let db = match db_guard.as_ref() {
        Some(d) => d,
        None => return Vec::new(),
    };

    let cutoff = (Utc::now() - chrono::Duration::days(30))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    let mut stmt = match db.conn_ref().prepare(
        "SELECT id FROM people
         WHERE last_enriched_at IS NULL
            OR last_enriched_at < ?1
         ORDER BY last_enriched_at ASC NULLS FIRST
         LIMIT ?2",
    ) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Clay poller: failed to query unenriched people: {}", e);
            return Vec::new();
        }
    };

    let rows = stmt
        .query_map(rusqlite::params![cutoff, limit as i64], |row| {
            row.get::<_, String>(0)
        })
        .ok()
        .map(|iter| iter.filter_map(|r| r.ok()).collect::<Vec<_>>())
        .unwrap_or_default();

    rows
}

/// Insert a new `clay_sync_state` row for a person, marking it as pending.
/// Uses INSERT OR IGNORE to avoid duplicates (the table has a UNIQUE
/// constraint on `(entity_type, entity_id)`).
fn insert_clay_sync(state: &AppState, person_id: &str) {
    let db_guard = match state.db.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let db = match db_guard.as_ref() {
        Some(d) => d,
        None => return,
    };

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    if let Err(e) = db.conn_ref().execute(
        "INSERT OR IGNORE INTO clay_sync_state (id, entity_type, entity_id, state, created_at, updated_at)
         VALUES (?1, 'person', ?2, 'pending', ?3, ?3)",
        rusqlite::params![id, person_id, now],
    ) {
        log::warn!(
            "Clay poller: failed to insert sync row for {}: {}",
            person_id,
            e
        );
    }
}

/// Mark a `clay_sync_state` row as completed for a given person.
fn update_clay_sync_completed(state: &AppState, person_id: &str) {
    let db_guard = match state.db.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let db = match db_guard.as_ref() {
        Some(d) => d,
        None => return,
    };

    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    if let Err(e) = db.conn_ref().execute(
        "UPDATE clay_sync_state
         SET state = 'completed', completed_at = ?1, updated_at = ?1
         WHERE entity_type = 'person' AND entity_id = ?2 AND state = 'pending'",
        rusqlite::params![now, person_id],
    ) {
        log::warn!(
            "Clay poller: failed to mark sync completed for {}: {}",
            person_id,
            e
        );
    }
}

/// Mark a `clay_sync_state` row as failed, incrementing the attempt counter.
fn update_clay_sync_failed(state: &AppState, person_id: &str, error: &str) {
    let db_guard = match state.db.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let db = match db_guard.as_ref() {
        Some(d) => d,
        None => return,
    };

    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    if let Err(e) = db.conn_ref().execute(
        "UPDATE clay_sync_state
         SET state = CASE WHEN attempts + 1 >= max_attempts THEN 'failed' ELSE 'pending' END,
             attempts = attempts + 1,
             last_attempt_at = ?1,
             error_message = ?2,
             updated_at = ?1
         WHERE entity_type = 'person' AND entity_id = ?3
           AND state = 'pending'",
        rusqlite::params![now, error, person_id],
    ) {
        log::warn!(
            "Clay poller: failed to mark sync failed for {}: {}",
            person_id,
            e
        );
    }
}

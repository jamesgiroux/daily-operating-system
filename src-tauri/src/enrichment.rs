//! Unified enrichment processor.
//!
//! Single background task that processes enrichment requests from all sources
//! (Clay via Smithery, Gravatar). Replaces the separate Clay poller and
//! Gravatar fetcher loops.

use std::sync::Arc;
use std::time::Duration;

use crate::state::AppState;

/// Background enrichment processor.
///
/// - 60s startup delay
/// - Processes pending `clay_sync_state` rows via Clay/Smithery
/// - Processes stale `gravatar_cache` rows via Gravatar MCP
/// - Rate limits between requests (5s Clay, 1s Gravatar)
/// - Wakes on `enrichment_wake` signal or sleeps between sweeps
pub async fn run_enrichment_processor(state: Arc<AppState>) {
    log::info!("Enrichment processor: starting 60s startup delay");
    tokio::time::sleep(Duration::from_secs(60)).await;
    log::info!("Enrichment processor: startup delay complete, entering loop");

    loop {
        let enriched = process_one_sweep(&state).await;
        log::info!("Enrichment processor: sweep complete, {} processed", enriched);

        // Sleep until next sweep or wake signal
        let sweep_hours = state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|c| c.clay.sweep_interval_hours))
            .unwrap_or(24);

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(sweep_hours as u64 * 3600)) => {},
            _ = state.integrations.enrichment_wake.notified() => {
                log::info!("Enrichment processor: woken by signal");
            },
        }
    }
}

/// Run one sweep: Clay enrichment first (higher priority), then Gravatar.
async fn process_one_sweep(state: &AppState) -> u32 {
    let mut total = 0;

    // Phase 1: Clay enrichment via Smithery
    total += process_clay_queue(state).await;

    // Phase 2: Gravatar enrichment
    total += process_gravatar_queue(state).await;

    total
}

/// Process pending Clay enrichment requests.
async fn process_clay_queue(state: &AppState) -> u32 {
    // Read config
    let (enabled, max_per_sweep) = {
        let config = state.config.read().ok();
        match config.as_ref().and_then(|g| g.as_ref()) {
            Some(c) => (c.clay.enabled, c.clay.max_per_sweep),
            None => (false, 20),
        }
    };

    if !enabled {
        return 0;
    }

    // Resolve Smithery credentials
    let api_key = match crate::clay::oauth::get_smithery_api_key() {
        Some(k) => k,
        None => return 0,
    };
    let (namespace, connection_id) = {
        let config = state.config.read().ok();
        match config.as_ref().and_then(|g| g.as_ref()) {
            Some(c) => match (&c.clay.smithery_namespace, &c.clay.smithery_connection_id) {
                (Some(ns), Some(conn)) => (ns.clone(), conn.clone()),
                _ => return 0,
            },
            None => return 0,
        }
    };

    // Connect to Clay via Smithery
    let client = match crate::clay::client::ClayClient::connect(&api_key, &namespace, &connection_id).await {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Enrichment processor: Clay connection failed: {}", e);
            return 0;
        }
    };

    log::info!("Enrichment processor: Clay connected, processing queue (max={})", max_per_sweep);

    // Get pending sync IDs
    let pending_ids = get_pending_clay_ids(state, max_per_sweep as usize);
    let mut enriched: u32 = 0;

    for person_id in &pending_ids {
        match crate::clay::enricher::enrich_person_from_clay_with_client(state, person_id, &client).await {
            Ok(result) => {
                mark_clay_completed(state, person_id);
                log::info!("Clay enriched {}: {} fields", person_id, result.fields_updated.len());
                enriched += 1;
            }
            Err(e) => {
                mark_clay_failed(state, person_id, &e.to_string());
                log::warn!("Clay enrichment failed for {}: {}", person_id, e);
            }
        }
        // Rate limit: 5s between Clay calls
        tokio::time::sleep(Duration::from_secs(5)).await;
        if enriched >= max_per_sweep {
            break;
        }
    }

    // Sweep: unenriched people not in clay_sync_state
    if enriched < max_per_sweep {
        let unenriched = get_unenriched_people(state, (max_per_sweep - enriched) as usize);
        for person_id in &unenriched {
            insert_clay_sync(state, person_id);
            match crate::clay::enricher::enrich_person_from_clay_with_client(state, person_id, &client).await {
                Ok(result) => {
                    mark_clay_completed(state, person_id);
                    log::info!("Clay sweep enriched {}: {} fields", person_id, result.fields_updated.len());
                    enriched += 1;
                }
                Err(e) => {
                    mark_clay_failed(state, person_id, &e.to_string());
                    log::warn!("Clay sweep failed for {}: {}", person_id, e);
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
            if enriched >= max_per_sweep {
                break;
            }
        }
    }

    client.disconnect().await;
    enriched
}

/// Process stale Gravatar profiles.
async fn process_gravatar_queue(state: &AppState) -> u32 {
    let (enabled, api_key) = {
        let config = state.config.read().ok();
        match config.as_ref().and_then(|g| g.as_ref()) {
            Some(c) => (c.gravatar.enabled, c.gravatar.api_key.clone()),
            None => (false, None),
        }
    };

    if !enabled {
        return 0;
    }

    // Get people needing gravatar fetch
    let emails_to_fetch: Vec<(String, Option<String>)> = {
        let db_guard = state.db.lock().ok();
        match db_guard.as_ref().and_then(|g| g.as_ref()) {
            Some(db) => crate::gravatar::cache::get_stale_emails(db.conn_ref(), 50)
                .unwrap_or_default(),
            None => Vec::new(),
        }
    };

    if emails_to_fetch.is_empty() {
        return 0;
    }

    log::info!("Enrichment processor: {} gravatar profiles to fetch", emails_to_fetch.len());

    let client = match crate::gravatar::client::GravatarClient::connect(api_key.as_deref()).await {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Enrichment processor: Gravatar connection failed: {}", e);
            return 0;
        }
    };

    let data_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".dailyos")
        .join("avatars");
    let _ = std::fs::create_dir_all(&data_dir);

    let mut enriched: u32 = 0;

    for (email, person_id) in &emails_to_fetch {
        let profile = client.get_profile(email).await.unwrap_or_default();

        let avatar_path = match client.get_avatar(email, 200).await {
            Ok(Some(bytes)) => {
                use sha2::{Digest, Sha256};
                let hash = Sha256::digest(email.as_bytes());
                let hash_hex = hex::encode(&hash[..8]);
                let path = data_dir.join(format!("{}.png", hash_hex));
                if std::fs::write(&path, &bytes).is_ok() {
                    Some(path.to_string_lossy().to_string())
                } else {
                    None
                }
            }
            _ => None,
        };

        let interests = client.get_interests(email).await.unwrap_or_default();

        let has_gravatar = profile.display_name.is_some() || avatar_path.is_some();
        let cache_entry = crate::gravatar::cache::CachedGravatar {
            email: email.clone(),
            avatar_url: avatar_path,
            display_name: profile.display_name,
            bio: profile.bio,
            location: profile.location,
            company: profile.company,
            job_title: profile.job_title,
            interests_json: if interests.is_empty() {
                None
            } else {
                serde_json::to_string(&interests).ok()
            },
            has_gravatar,
            fetched_at: chrono::Utc::now().to_rfc3339(),
            person_id: person_id.clone(),
        };

        if let Ok(db_guard) = state.db.lock() {
            if let Some(db) = db_guard.as_ref() {
                let _ = crate::gravatar::cache::upsert_cache(db.conn_ref(), &cache_entry);

                if has_gravatar {
                    if let Some(ref pid) = person_id {
                        let update = crate::db::people::ProfileUpdate {
                            photo_url: cache_entry.avatar_url.clone(),
                            bio: cache_entry.bio.clone(),
                            organization: cache_entry.company.clone(),
                            role: cache_entry.job_title.clone(),
                            ..Default::default()
                        };
                        let _ = db.update_person_profile(pid, &update, "gravatar");

                        let value = serde_json::json!({
                            "display_name": cache_entry.display_name,
                            "company": cache_entry.company,
                            "job_title": cache_entry.job_title,
                        })
                        .to_string();
                        let _ = crate::signals::bus::emit_signal_and_propagate(
                            db,
                            &state.signals.engine,
                            "person",
                            pid,
                            "profile_discovered",
                            "gravatar",
                            Some(&value),
                            0.7,
                        );

                        enriched += 1;
                    }
                }
            }
        }

        // Rate limit: 1s between Gravatar calls
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    client.disconnect().await;
    log::info!("Enrichment processor: Gravatar batch complete, {} enriched", enriched);
    enriched
}

// ---------------------------------------------------------------------------
// Clay sync state helpers (moved from clay/poller.rs)
// ---------------------------------------------------------------------------

fn get_pending_clay_ids(state: &AppState, limit: usize) -> Vec<String> {
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
         ORDER BY created_at ASC LIMIT ?1",
    ) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Enrichment processor: failed to query pending syncs: {}", e);
            return Vec::new();
        }
    };

    stmt.query_map(rusqlite::params![limit as i64], |row| row.get::<_, String>(0))
        .ok()
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
}

fn get_unenriched_people(state: &AppState, limit: usize) -> Vec<String> {
    let db_guard = match state.db.lock() {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };
    let db = match db_guard.as_ref() {
        Some(d) => d,
        None => return Vec::new(),
    };

    let cutoff = (chrono::Utc::now() - chrono::Duration::days(30))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    let mut stmt = match db.conn_ref().prepare(
        "SELECT id FROM people
         WHERE last_enriched_at IS NULL OR last_enriched_at < ?1
         ORDER BY last_enriched_at ASC NULLS FIRST LIMIT ?2",
    ) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Enrichment processor: failed to query unenriched people: {}", e);
            return Vec::new();
        }
    };

    stmt.query_map(rusqlite::params![cutoff, limit as i64], |row| row.get::<_, String>(0))
        .ok()
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
}

fn insert_clay_sync(state: &AppState, person_id: &str) {
    let db_guard = match state.db.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let db = match db_guard.as_ref() {
        Some(d) => d,
        None => return,
    };

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    let _ = db.conn_ref().execute(
        "INSERT OR IGNORE INTO clay_sync_state (id, entity_type, entity_id, state, created_at, updated_at)
         VALUES (?1, 'person', ?2, 'pending', ?3, ?3)",
        rusqlite::params![id, person_id, now],
    );
}

fn mark_clay_completed(state: &AppState, person_id: &str) {
    let db_guard = match state.db.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let db = match db_guard.as_ref() {
        Some(d) => d,
        None => return,
    };

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let _ = db.conn_ref().execute(
        "UPDATE clay_sync_state SET state = 'completed', completed_at = ?1, updated_at = ?1
         WHERE entity_type = 'person' AND entity_id = ?2 AND state = 'pending'",
        rusqlite::params![now, person_id],
    );
}

fn mark_clay_failed(state: &AppState, person_id: &str, error: &str) {
    let db_guard = match state.db.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let db = match db_guard.as_ref() {
        Some(d) => d,
        None => return,
    };

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let _ = db.conn_ref().execute(
        "UPDATE clay_sync_state
         SET state = CASE WHEN attempts + 1 >= max_attempts THEN 'failed' ELSE 'pending' END,
             attempts = attempts + 1, last_attempt_at = ?1, error_message = ?2, updated_at = ?1
         WHERE entity_type = 'person' AND entity_id = ?3 AND state = 'pending'",
        rusqlite::params![now, error, person_id],
    );
}

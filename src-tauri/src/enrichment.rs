//! Unified enrichment processor.
//!
//! Single background task that processes enrichment requests from all sources
//! (Clay via Smithery, Gravatar). Replaces the separate Clay poller and
//! Gravatar fetcher loops.
//!
//! Worker loop drains until the queue is empty, then sleeps until woken.

use std::sync::Arc;
use std::time::Duration;

use crate::state::AppState;

/// Background enrichment processor.
///
/// - 60s startup delay
/// - Drains pending queue completely (Clay first, then Gravatar)
/// - Sleeps until woken by `enrichment_wake` or sweep_interval_hours
/// - Caps attempts per sweep, not successes
pub async fn run_enrichment_processor(state: Arc<AppState>) {
    log::info!("Enrichment processor: starting 60s startup delay");
    tokio::time::sleep(Duration::from_secs(60)).await;
    log::info!("Enrichment processor: delay complete, entering loop");

    loop {
        // Dev mode isolation: pause background processing while dev sandbox is active
        if crate::db::is_dev_db_mode() {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            continue;
        }

        // Drain: keep sweeping while there's pending work
        let mut total_this_cycle = 0u32;
        loop {
            let swept = process_one_sweep(&state).await;
            total_this_cycle += swept;
            if swept == 0 {
                break; // Queue empty — exit drain loop
            }
            // Brief pause between sweeps to avoid hammering
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        log::info!(
            "Enrichment processor: cycle complete, {} total processed. Sleeping until wake.",
            total_this_cycle
        );

        // Sleep until woken or sweep_interval_hours elapses
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

/// Check if we're in Glean mode (any strategy). In Glean mode, Clay and
/// Gravatar enrichment are disabled — Glean replaces these data sources.
fn is_glean_mode(state: &AppState) -> bool {
    state.context_provider().provider_name() == "glean"
}

/// Run one sweep: Clay enrichment first (higher priority), then Gravatar.
/// Returns total number of rows *attempted* (not just successes).
/// In Glean mode, Clay and Gravatar are skipped entirely.
async fn process_one_sweep(state: &AppState) -> u32 {
    if is_glean_mode(state) {
        log::info!("Enrichment: Glean mode active, skipping Clay/Gravatar");
        return 0;
    }

    let mut total = 0;
    let clay_count = process_clay_queue(state).await;
    if clay_count > 0 {
        if let Ok(mut audit) = state.audit_log.lock() {
            let _ = audit.append(
                "data_access",
                "clay_enrichment",
                serde_json::json!({"entity_type": "person", "count": clay_count}),
            );
        }
    }
    total += clay_count;
    let gravatar_count = process_gravatar_queue(state).await;
    if gravatar_count > 0 {
        if let Ok(mut audit) = state.audit_log.lock() {
            let _ = audit.append(
                "data_access",
                "gravatar_lookup",
                serde_json::json!({"count": gravatar_count}),
            );
        }
    }
    total += gravatar_count;
    total
}

/// Process pending Clay enrichment requests.
/// Budget caps by *attempts*, not successes — so a failure-heavy batch
/// still makes progress through the queue without infinite retry loops.
async fn process_clay_queue(state: &AppState) -> u32 {
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
        None => {
            log::info!("Enrichment: no Smithery API key, skipping Clay");
            return 0;
        }
    };
    let (namespace, connection_id) = {
        let config = state.config.read().ok();
        match config.as_ref().and_then(|g| g.as_ref()) {
            Some(c) => match (&c.clay.smithery_namespace, &c.clay.smithery_connection_id) {
                (Some(ns), Some(conn)) => (ns.clone(), conn.clone()),
                _ => {
                    log::info!("Enrichment: Smithery namespace/connection not set, skipping Clay");
                    return 0;
                }
            },
            None => return 0,
        }
    };

    let client = match crate::clay::client::ClayClient::connect(
        &api_key,
        &namespace,
        &connection_id,
    )
    .await
    {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Enrichment: Clay connection failed: {}", e);
            return 0;
        }
    };

    // Phase 1: pending clay_sync_state rows
    let pending_ids = get_pending_clay_ids(state, max_per_sweep as usize).await;
    let mut attempted: u32 = 0;

    log::info!(
        "Enrichment: Clay connected, {} pending (max={})",
        pending_ids.len(),
        max_per_sweep
    );

    for person_id in &pending_ids {
        attempted += 1;
        match crate::clay::enricher::enrich_person_from_clay_with_client(state, person_id, &client)
            .await
        {
            Ok(result) => {
                mark_clay_completed(state, person_id).await;
                log::info!(
                    "Enrichment: Clay OK {} ({} fields)",
                    person_id,
                    result.fields_updated.len()
                );
            }
            Err(e) => {
                mark_clay_failed(state, person_id, &e).await;
                log::warn!("Enrichment: Clay FAIL {}: {}", person_id, e);
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
        if attempted >= max_per_sweep {
            break;
        }
    }

    // Phase 2: sweep unenriched people not yet in clay_sync_state
    if attempted < max_per_sweep {
        let unenriched = get_unenriched_people(state, (max_per_sweep - attempted) as usize).await;
        for person_id in &unenriched {
            insert_clay_sync(state, person_id).await;
            attempted += 1;
            match crate::clay::enricher::enrich_person_from_clay_with_client(
                state, person_id, &client,
            )
            .await
            {
                Ok(result) => {
                    mark_clay_completed(state, person_id).await;
                    log::info!(
                        "Enrichment: Clay sweep OK {} ({} fields)",
                        person_id,
                        result.fields_updated.len()
                    );
                }
                Err(e) => {
                    mark_clay_failed(state, person_id, &e).await;
                    log::warn!("Enrichment: Clay sweep FAIL {}: {}", person_id, e);
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
            if attempted >= max_per_sweep {
                break;
            }
        }
    }

    client.disconnect().await;
    attempted
}

/// Process stale Gravatar profiles.
async fn process_gravatar_queue(state: &AppState) -> u32 {
    let enabled = {
        let config = state.config.read().ok();
        config
            .as_ref()
            .and_then(|g| g.as_ref())
            .map(|c| c.gravatar.enabled)
            .unwrap_or(false)
    };

    if !enabled {
        return 0;
    }

    let api_key = crate::gravatar::keychain::get_gravatar_api_key();

    let emails_to_fetch: Vec<(String, Option<String>)> = state
        .db_read(move |db| {
            Ok(crate::gravatar::cache::get_stale_emails(db.conn_ref(), 50).unwrap_or_default())
        })
        .await
        .unwrap_or_default();

    if emails_to_fetch.is_empty() {
        return 0;
    }

    log::info!(
        "Enrichment: {} gravatar profiles to fetch",
        emails_to_fetch.len()
    );

    let client = match crate::gravatar::client::GravatarClient::connect(api_key.as_deref()).await {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Enrichment: Gravatar connection failed: {}", e);
            return 0;
        }
    };

    let data_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".dailyos")
        .join("avatars");
    let _ = std::fs::create_dir_all(&data_dir);

    let mut attempted: u32 = 0;

    for (email, person_id) in &emails_to_fetch {
        attempted += 1;
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

        let engine = Arc::clone(&state.signals.engine);
        let _ = state
            .db_write(move |db| {
                let _ = crate::gravatar::cache::upsert_cache(db.conn_ref(), &cache_entry);

                if has_gravatar {
                    if let Some(ref pid) = cache_entry.person_id {
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
                            &engine,
                            "person",
                            pid,
                            "profile_discovered",
                            "gravatar",
                            Some(&value),
                            0.7,
                        );
                    }
                }

                Ok(())
            })
            .await;

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    client.disconnect().await;
    log::info!(
        "Enrichment: Gravatar batch complete, {} attempted",
        attempted
    );
    attempted
}

// ---------------------------------------------------------------------------
// Clay sync state helpers
// ---------------------------------------------------------------------------

async fn get_pending_clay_ids(state: &AppState, limit: usize) -> Vec<String> {
    state
        .db_read(move |db| {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT entity_id FROM clay_sync_state
                 WHERE state = 'pending' AND attempts < max_attempts
                 ORDER BY created_at ASC LIMIT ?1",
                )
                .map_err(|e| format!("failed to query pending syncs: {}", e))?;

            let rows = stmt
                .query_map(rusqlite::params![limit as i64], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(|e| format!("query_map failed: {}", e))?
                .filter_map(|r| r.ok())
                .collect();

            Ok(rows)
        })
        .await
        .unwrap_or_default()
}

async fn get_unenriched_people(state: &AppState, limit: usize) -> Vec<String> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(30))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    state
        .db_read(move |db| {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT id FROM people
                 WHERE last_enriched_at IS NULL OR last_enriched_at < ?1
                 ORDER BY last_enriched_at ASC NULLS FIRST LIMIT ?2",
                )
                .map_err(|e| format!("failed to query unenriched people: {}", e))?;

            let rows = stmt
                .query_map(rusqlite::params![cutoff, limit as i64], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(|e| format!("query_map failed: {}", e))?
                .filter_map(|r| r.ok())
                .collect();

            Ok(rows)
        })
        .await
        .unwrap_or_default()
}

async fn insert_clay_sync(state: &AppState, person_id: &str) {
    let person_id = person_id.to_string();
    let _ = state
        .db_write(move |db| {
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
            db.conn_ref().execute(
                "INSERT OR IGNORE INTO clay_sync_state (id, entity_type, entity_id, state, created_at, updated_at)
                 VALUES (?1, 'person', ?2, 'pending', ?3, ?3)",
                rusqlite::params![id, person_id, now],
            ).map_err(|e| format!("insert_clay_sync failed: {}", e))?;
            Ok(())
        })
        .await;
}

async fn mark_clay_completed(state: &AppState, person_id: &str) {
    let person_id = person_id.to_string();
    let _ = state
        .db_write(move |db| {
            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
            db.conn_ref().execute(
                "UPDATE clay_sync_state SET state = 'completed', completed_at = ?1, updated_at = ?1
                 WHERE entity_type = 'person' AND entity_id = ?2 AND state = 'pending'",
                rusqlite::params![now, person_id],
            ).map_err(|e| format!("mark_clay_completed failed: {}", e))?;
            Ok(())
        })
        .await;
}

async fn mark_clay_failed(state: &AppState, person_id: &str, error: &str) {
    let person_id = person_id.to_string();
    let error = error.to_string();
    let _ = state
        .db_write(move |db| {
            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
            db.conn_ref().execute(
                "UPDATE clay_sync_state
                 SET state = CASE WHEN attempts + 1 >= max_attempts THEN 'failed' ELSE 'pending' END,
                     attempts = attempts + 1, last_attempt_at = ?1, error_message = ?2, updated_at = ?1
                 WHERE entity_type = 'person' AND entity_id = ?3 AND state = 'pending'",
                rusqlite::params![now, error, person_id],
            ).map_err(|e| format!("mark_clay_failed failed: {}", e))?;
            Ok(())
        })
        .await;
}

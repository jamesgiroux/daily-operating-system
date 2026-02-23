//! MCP client for communicating with the Gravatar MCP server.
//!
//! Uses rmcp's child process transport to spawn `npx @automattic/mcp-server-gravatar`.

use std::sync::Arc;

use base64::Engine;
use rmcp::model::CallToolRequestParam;
use rmcp::service::RunningService;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::{RoleClient, ServiceExt};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// Profile data returned by the Gravatar MCP server.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GravatarProfile {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub company: Option<String>,
    pub job_title: Option<String>,
}

/// Errors from Gravatar MCP operations.
#[derive(Debug, thiserror::Error)]
pub enum GravatarError {
    #[error("npx not found on PATH")]
    NpxNotFound,
    #[error("Failed to spawn npx process: {0}")]
    SpawnFailed(String),
    #[error("MCP connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Tool call failed: {0}")]
    ToolCallFailed(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// MCP client wrapper for the Gravatar server.
pub struct GravatarClient {
    service: RunningService<RoleClient, ()>,
}

impl GravatarClient {
    /// Connect to the Gravatar MCP server by spawning npx.
    pub async fn connect(api_key: Option<&str>) -> Result<Self, GravatarError> {
        let npx_path = crate::util::resolve_npx_binary()
            .ok_or(GravatarError::NpxNotFound)?;

        let mut cmd = tokio::process::Command::new(npx_path);
        cmd.arg("@automattic/mcp-server-gravatar");

        if let Some(key) = api_key {
            cmd.env("GRAVATAR_API_KEY", key);
        }

        let transport =
            TokioChildProcess::new(&mut cmd).map_err(|e| GravatarError::SpawnFailed(e.to_string()))?;

        let service = ()
            .serve(transport)
            .await
            .map_err(|e| GravatarError::ConnectionFailed(e.to_string()))?;

        Ok(Self { service })
    }

    /// Fetch a Gravatar profile by email address.
    pub async fn get_profile(&self, email: &str) -> Result<GravatarProfile, GravatarError> {
        let result = self
            .service
            .call_tool(CallToolRequestParam {
                name: "get_profile_by_email".into(),
                arguments: serde_json::json!({ "email": email })
                    .as_object()
                    .cloned(),
            })
            .await
            .map_err(|e| GravatarError::ToolCallFailed(e.to_string()))?;

        if result.is_error == Some(true) {
            let msg = result
                .content
                .first()
                .and_then(|c| c.as_text())
                .map(|t| t.text.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(GravatarError::ToolCallFailed(msg));
        }

        let text: String = result
            .content
            .iter()
            .filter_map(|c| c.as_text().map(|t| t.text.as_str()))
            .collect();

        // The MCP server returns JSON profile data
        Ok(serde_json::from_str(&text).unwrap_or_default())
    }

    /// Fetch an avatar image by email. Returns None if the email has no Gravatar.
    pub async fn get_avatar(
        &self,
        email: &str,
        size: u32,
    ) -> Result<Option<Vec<u8>>, GravatarError> {
        let result = self
            .service
            .call_tool(CallToolRequestParam {
                name: "get_avatar_by_email".into(),
                arguments: serde_json::json!({
                    "email": email,
                    "size": size,
                    "defaultOption": "404"
                })
                .as_object()
                .cloned(),
            })
            .await
            .map_err(|e| GravatarError::ToolCallFailed(e.to_string()))?;

        if result.is_error == Some(true) {
            // 404 means no avatar — return None instead of error
            let msg = result
                .content
                .first()
                .and_then(|c| c.as_text())
                .map(|t| t.text.clone())
                .unwrap_or_default();
            if msg.contains("404") || msg.contains("not found") {
                return Ok(None);
            }
            return Err(GravatarError::ToolCallFailed(msg));
        }

        // Extract image bytes from the response
        for content in &result.content {
            if let Some(img) = content.as_image() {
                if let Ok(bytes) =
                    base64::engine::general_purpose::STANDARD.decode(&img.data)
                {
                    return Ok(Some(bytes));
                }
            }
            // Some MCP servers return base64 in text
            if let Some(text) = content.as_text() {
                if let Ok(bytes) =
                    base64::engine::general_purpose::STANDARD.decode(text.text.trim())
                {
                    return Ok(Some(bytes));
                }
            }
        }

        Ok(None)
    }

    /// Fetch AI-inferred interests for an email address.
    pub async fn get_interests(&self, email: &str) -> Result<Vec<String>, GravatarError> {
        let result = self
            .service
            .call_tool(CallToolRequestParam {
                name: "get_inferred_interests_by_email".into(),
                arguments: serde_json::json!({ "email": email })
                    .as_object()
                    .cloned(),
            })
            .await
            .map_err(|e| GravatarError::ToolCallFailed(e.to_string()))?;

        if result.is_error == Some(true) {
            return Ok(Vec::new()); // Interests are optional, don't fail
        }

        let text: String = result
            .content
            .iter()
            .filter_map(|c| c.as_text().map(|t| t.text.as_str()))
            .collect();

        // Try parsing as JSON array of strings
        Ok(serde_json::from_str(&text).unwrap_or_else(|_| {
            // Fallback: split comma-separated text
            if text.is_empty() {
                Vec::new()
            } else {
                text.split(',').map(|s| s.trim().to_string()).collect()
            }
        }))
    }

    /// Disconnect from the Gravatar MCP server.
    pub async fn disconnect(self) {
        let _ = self.service.cancel().await;
    }

    /// Verify that npx is available (checks PATH and common install locations).
    pub fn npx_available() -> bool {
        crate::util::resolve_npx_binary().is_some()
    }
}

/// Background fetcher that periodically syncs Gravatar data for all people.
pub async fn run_gravatar_fetcher(state: Arc<AppState>) {
    // 60-second startup delay
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    loop {
        // Check if enabled
        let (enabled, api_key) = {
            let config = state.config.read().ok();
            match config.as_ref().and_then(|g| g.as_ref()) {
                Some(c) => (c.gravatar.enabled, c.gravatar.api_key.clone()),
                None => (false, None),
            }
        };

        if !enabled {
            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
            continue;
        }

        log::info!("Gravatar fetcher: starting batch sync");

        // Get people needing fetch
        let emails_to_fetch: Vec<(String, Option<String>)> = {
            let db_guard = state.db.lock().ok();
            match db_guard.as_ref().and_then(|g| g.as_ref()) {
                Some(db) => super::cache::get_stale_emails(db.conn_ref(), 50)
                    .unwrap_or_default(),
                None => Vec::new(),
            }
        };

        if emails_to_fetch.is_empty() {
            log::info!("Gravatar fetcher: all profiles up to date");
            tokio::time::sleep(std::time::Duration::from_secs(6 * 3600)).await;
            continue;
        }

        log::info!(
            "Gravatar fetcher: {} profiles to fetch",
            emails_to_fetch.len()
        );

        // Connect once for the batch
        match GravatarClient::connect(api_key.as_deref()).await {
            Ok(client) => {
                let data_dir = dirs::home_dir()
                    .unwrap_or_default()
                    .join(".dailyos")
                    .join("avatars");
                let _ = std::fs::create_dir_all(&data_dir);

                for (email, person_id) in &emails_to_fetch {
                    // Fetch profile
                    let profile = client.get_profile(email).await.unwrap_or_default();

                    // Fetch avatar
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

                    // Fetch interests
                    let interests = client.get_interests(email).await.unwrap_or_default();

                    // Cache result
                    let has_gravatar = profile.display_name.is_some() || avatar_path.is_some();
                    let cache_entry = super::cache::CachedGravatar {
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
                            let _ = super::cache::upsert_cache(db.conn_ref(), &cache_entry);

                            // I423: Write photo_url back to people table (respects source priority)
                            if has_gravatar {
                                if let Some(ref avatar_url) = cache_entry.avatar_url {
                                    if let Some(ref pid) = person_id {
                                        writeback_photo_url(db, pid, avatar_url);
                                    }
                                }
                            }

                            // I423: Emit profile_discovered signal with propagation
                            if has_gravatar {
                                if let Some(ref pid) = person_id {
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
                                }
                            }
                        }
                    }

                    // Rate limit: 1 req/sec
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }

                client.disconnect().await;
                log::info!("Gravatar fetcher: batch complete");
            }
            Err(e) => {
                log::warn!("Gravatar fetcher: connection failed: {}", e);
            }
        }

        // Re-run every 6 hours
        tokio::time::sleep(std::time::Duration::from_secs(6 * 3600)).await;
    }
}

/// Write gravatar photo_url back to the people table, respecting source priority.
/// Gravatar has priority 2 (Clay=3 > Gravatar=2 > AI=1). Only writes if no
/// higher-priority source has already set photo_url.
fn writeback_photo_url(db: &crate::db::ActionDb, person_id: &str, avatar_url: &str) {
    let conn = db.conn_ref();

    // Read current photo_url and enrichment_sources
    let row: Option<(Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT photo_url, enrichment_sources FROM people WHERE id = ?1",
            rusqlite::params![person_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    let (current_photo, sources_json) = match row {
        Some(r) => r,
        None => return,
    };

    // Check source priority -- only write if allowed
    if !crate::clay::enricher::can_write_field(sources_json.as_deref(), "photo_url", "gravatar") {
        return;
    }

    // Update photo_url and record provenance
    let mut sources: std::collections::HashMap<String, crate::clay::enricher::FieldSource> =
        sources_json
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default();

    sources.insert(
        "photo_url".to_string(),
        crate::clay::enricher::FieldSource {
            source: "gravatar".to_string(),
            at: chrono::Utc::now().to_rfc3339(),
        },
    );

    let new_sources_json = serde_json::to_string(&sources).unwrap_or_default();

    let _ = conn.execute(
        "UPDATE people SET photo_url = ?2, enrichment_sources = ?3, updated_at = ?4 WHERE id = ?1",
        rusqlite::params![
            person_id,
            avatar_url,
            new_sources_json,
            chrono::Utc::now().to_rfc3339(),
        ],
    );

    if current_photo.is_none() {
        log::info!(
            "Gravatar: wrote photo_url for person {} (was NULL)",
            person_id
        );
    } else {
        log::info!(
            "Gravatar: updated photo_url for person {} (lower-priority source)",
            person_id
        );
    }
}


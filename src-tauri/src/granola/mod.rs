//! Granola integration for local cache transcript sync.
//!
//! Reads meeting data from Granola's local companion bridge when available,
//! with a legacy fallback to `~/Library/Application Support/Granola/cache-v*.json`.
//! The cache filename is auto-detected (highest version number wins).

pub mod cache;
pub mod companion;
pub mod matcher;
pub mod mcp_client;
pub mod poller;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Granola integration configuration stored in ~/.dailyos/config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GranolaConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Resolved at runtime via `resolve_cache_path()`. This field stores a
    /// user override; when empty, auto-detection kicks in.
    #[serde(default)]
    pub cache_path: String,
    #[serde(default = "default_poll_interval_minutes")]
    pub poll_interval_minutes: u32,
    #[serde(default = "default_mcp_endpoint")]
    pub mcp_endpoint: String,
}

fn default_poll_interval_minutes() -> u32 {
    10
}

fn default_mcp_endpoint() -> String {
    crate::granola_oauth::DEFAULT_GRANOLA_MCP_ENDPOINT.to_string()
}

impl Default for GranolaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cache_path: String::new(),
            poll_interval_minutes: default_poll_interval_minutes(),
            mcp_endpoint: default_mcp_endpoint(),
        }
    }
}

/// Return the Granola Application Support directory.
pub(crate) fn granola_dir() -> PathBuf {
    dirs::data_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_default()
        .join("Granola")
}

/// Find the highest-versioned `cache-v*.json` in the Granola directory.
/// Returns `None` if no matching file exists.
pub fn detect_cache_path() -> Option<PathBuf> {
    let dir = granola_dir();
    let entries = std::fs::read_dir(&dir).ok()?;

    entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            // Match pattern: cache-vN.json where N is one or more digits
            let version = name
                .strip_prefix("cache-v")?
                .strip_suffix(".json")?
                .parse::<u32>()
                .ok()?;
            Some((version, e.path()))
        })
        .max_by_key(|(v, _)| *v)
        .map(|(_, path)| path)
}

/// Find the highest-versioned encrypted `cache-v*.json.enc` in the Granola directory.
/// DailyOS does not read this directly; it is used to explain why the legacy
/// plain JSON fallback may be stale while Granola still has current data.
pub fn detect_encrypted_cache_path() -> Option<PathBuf> {
    let dir = granola_dir();
    let entries = std::fs::read_dir(&dir).ok()?;

    entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let version = name
                .strip_prefix("cache-v")?
                .strip_suffix(".json.enc")?
                .parse::<u32>()
                .ok()?;
            Some((version, e.path()))
        })
        .max_by_key(|(v, _)| *v)
        .map(|(_, path)| path)
}

/// Resolve the effective cache path: user override if non-empty, otherwise auto-detect.
pub fn resolve_cache_path(config: &GranolaConfig) -> Option<PathBuf> {
    if !config.cache_path.is_empty() {
        let p = PathBuf::from(&config.cache_path);
        if p.exists() {
            return Some(p);
        }
    }
    detect_cache_path()
}

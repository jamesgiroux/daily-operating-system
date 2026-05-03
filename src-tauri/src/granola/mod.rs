//! Granola integration for local cache transcript sync.
//!
//! Reads meeting data from Granola's local cache file at
//! `~/Library/Application Support/Granola/cache-v*.json`.
//! No API keys or authentication required — purely local file access.
//! The cache filename is auto-detected (highest version number wins).

pub mod cache;
pub mod matcher;
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
}

fn default_poll_interval_minutes() -> u32 {
    10
}

impl Default for GranolaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cache_path: String::new(),
            poll_interval_minutes: default_poll_interval_minutes(),
        }
    }
}

/// Return the Granola Application Support directory.
fn granola_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/Application Support/Granola")
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

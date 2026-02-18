//! Granola integration for local cache transcript sync (I226).
//!
//! Reads meeting data from Granola's local cache file at
//! `~/Library/Application Support/Granola/cache-v3.json`.
//! No API keys or authentication required — purely local file access.

pub mod cache;
pub mod matcher;
pub mod poller;

use serde::{Deserialize, Serialize};

/// Granola integration configuration stored in ~/.dailyos/config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GranolaConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_cache_path")]
    pub cache_path: String,
    #[serde(default = "default_poll_interval_minutes")]
    pub poll_interval_minutes: u32,
}

fn default_cache_path() -> String {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/Application Support/Granola/cache-v3.json")
        .to_string_lossy()
        .to_string()
}

fn default_poll_interval_minutes() -> u32 {
    10
}

impl Default for GranolaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cache_path: default_cache_path(),
            poll_interval_minutes: default_poll_interval_minutes(),
        }
    }
}

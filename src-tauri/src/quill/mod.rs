//! Quill MCP client integration for automatic transcript sync.
//!
//! Connects to Quill's local MCP server to fetch meeting transcripts
//! after meetings end, processes them through the existing AI pipeline,
//! and enriches workspace data.

pub mod client;
pub mod matcher;
pub mod poller;
pub mod sync;

use serde::{Deserialize, Serialize};

/// Quill integration configuration stored in ~/.dailyos/config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuillConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_delay_minutes")]
    pub delay_minutes: u32,
    #[serde(default = "default_bridge_path")]
    pub bridge_path: String,
    #[serde(default)]
    pub auto_sync_contacts: bool,
    #[serde(default = "default_poll_interval_minutes")]
    pub poll_interval_minutes: u32,
}

fn default_delay_minutes() -> u32 {
    10
}

fn default_poll_interval_minutes() -> u32 {
    5
}

fn default_bridge_path() -> String {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/Application Support/Quill/mcp-stdio-bridge.js")
        .to_string_lossy()
        .to_string()
}

impl Default for QuillConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            delay_minutes: default_delay_minutes(),
            bridge_path: default_bridge_path(),
            auto_sync_contacts: false,
            poll_interval_minutes: default_poll_interval_minutes(),
        }
    }
}

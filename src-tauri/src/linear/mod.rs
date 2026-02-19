//! Linear issue tracker integration (I346).
//!
//! Syncs assigned issues and projects from Linear via their GraphQL API.
//! Follows the same architectural pattern as Clay (clay/mod.rs).

pub mod client;
pub mod poller;
pub mod sync;

use serde::{Deserialize, Serialize};

/// Linear integration configuration stored in ~/.dailyos/config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinearConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default = "default_poll_interval_minutes")]
    pub poll_interval_minutes: u32,
}

fn default_poll_interval_minutes() -> u32 {
    60
}

impl Default for LinearConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            poll_interval_minutes: default_poll_interval_minutes(),
        }
    }
}

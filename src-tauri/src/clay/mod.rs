//! Clay.earth MCP integration for contact and company enrichment (I228).
//!
//! Connects to the Clay MCP server (SSE primary, npm stdio fallback) to enrich
//! people with social profiles, bios, title history, and company firmographics.
//! Introduces a source attribution system so data provenance is tracked per-field.

pub mod client;
pub mod enricher;
pub mod poller;
pub mod signals;

use serde::{Deserialize, Serialize};

/// Clay integration configuration stored in ~/.dailyos/config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClayConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default)]
    pub auto_enrich_on_create: bool,
    #[serde(default = "default_sweep_interval_hours")]
    pub sweep_interval_hours: u32,
    #[serde(default = "default_max_per_sweep")]
    pub max_per_sweep: u32,
}

fn default_sweep_interval_hours() -> u32 {
    24
}

fn default_max_per_sweep() -> u32 {
    20
}

impl Default for ClayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            auto_enrich_on_create: false,
            sweep_interval_hours: default_sweep_interval_hours(),
            max_per_sweep: default_max_per_sweep(),
        }
    }
}

//! Clay.earth MCP integration for contact and company enrichment.
//!
//! Connects to Clay via Smithery Connect — Smithery manages Clay OAuth and
//! credentials. DailyOS sends JSON-RPC over HTTP to the Smithery endpoint.

pub mod client;
pub mod enricher;
pub mod oauth;
pub mod signals;

use serde::{Deserialize, Serialize};

/// Clay integration configuration stored in ~/.dailyos/config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClayConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Legacy API key field — unused since Smithery migration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default)]
    pub auto_enrich_on_create: bool,
    #[serde(default = "default_sweep_interval_hours")]
    pub sweep_interval_hours: u32,
    #[serde(default = "default_max_per_sweep")]
    pub max_per_sweep: u32,
    /// Smithery namespace (e.g. "viper-RmaO").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smithery_namespace: Option<String>,
    /// Smithery connection ID for Clay MCP (e.g. "clay-mcp-vGfX").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smithery_connection_id: Option<String>,
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
            smithery_namespace: None,
            smithery_connection_id: None,
        }
    }
}

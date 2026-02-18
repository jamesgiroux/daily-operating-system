//! Gravatar MCP server integration for avatar and profile enrichment (I229).
//!
//! Connects to the `@automattic/mcp-server-gravatar` MCP server via npx to fetch
//! profile pictures, bios, and inferred interests for people in the database.

pub mod cache;
pub mod client;

use serde::{Deserialize, Serialize};

/// Gravatar integration configuration stored in ~/.dailyos/config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GravatarConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

impl Default for GravatarConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
        }
    }
}

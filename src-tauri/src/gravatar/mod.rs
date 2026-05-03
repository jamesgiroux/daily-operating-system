//! Gravatar MCP server integration for avatar and profile enrichment.
//!
//! Connects to the `@automattic/mcp-server-gravatar` MCP server via npx to fetch
//! profile pictures, bios, and inferred interests for people in the database.

pub mod cache;
pub mod client;
pub mod keychain;

use serde::{Deserialize, Serialize};

/// Gravatar integration configuration stored in ~/.dailyos/config.json.
///
/// The API key is stored in macOS Keychain (service `com.dailyos.desktop.gravatar`),
/// not in the JSON config file. The `api_key` field is retained only for migration
/// from older versions — `skip_serializing` ensures it is never written back.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GravatarConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Legacy field — read during migration, never persisted.
    #[serde(default, skip_serializing)]
    pub api_key: Option<String>,
}

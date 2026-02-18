//! MCP client for communicating with the Clay MCP server.
//!
//! Dual transport strategy:
//! 1. SSE primary — connects to `https://mcp.clay.earth/mcp` with Bearer auth
//! 2. Stdio fallback — spawns `npx -y @clayhq/clay-mcp` as a child process
//!
//! Since the rmcp crate in this project does not include an SSE transport feature,
//! the SSE path is attempted via raw reqwest + manual JSON-RPC, and the stdio path
//! uses the standard rmcp `TokioChildProcess` transport (same pattern as Gravatar).

use rmcp::model::CallToolRequestParam;
use rmcp::service::RunningService;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::{RoleClient, ServiceExt};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// A contact summary returned by Clay search.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClayContact {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub company: Option<String>,
    #[serde(default)]
    pub photo_url: Option<String>,
    #[serde(default)]
    pub linkedin_url: Option<String>,
    #[serde(default)]
    pub twitter_handle: Option<String>,
}

/// Extended contact detail with bio, title history, and company firmographics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClayContactDetail {
    // Base fields
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub company: Option<String>,
    #[serde(default)]
    pub photo_url: Option<String>,
    #[serde(default)]
    pub linkedin_url: Option<String>,
    #[serde(default)]
    pub twitter_handle: Option<String>,

    // Extended fields
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub title_history: Vec<TitleHistoryEntry>,
    #[serde(default)]
    pub company_industry: Option<String>,
    #[serde(default)]
    pub company_size: Option<String>,
    #[serde(default)]
    pub company_hq: Option<String>,
    #[serde(default)]
    pub company_funding: Option<String>,
}

/// A single entry in a contact's title/role history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleHistoryEntry {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub company: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
}

/// Interaction statistics for a contact.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClayContactStats {
    #[serde(default)]
    pub total_interactions: u64,
    #[serde(default)]
    pub last_interaction_at: Option<String>,
    #[serde(default)]
    pub top_channels: Vec<String>,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from Clay MCP operations.
#[derive(Debug, thiserror::Error)]
pub enum ClayError {
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
    #[error("No API key configured for Clay")]
    NoApiKey,
}

// ---------------------------------------------------------------------------
// Transport wrapper
// ---------------------------------------------------------------------------

/// Which transport the client connected through.
#[derive(Debug)]
enum Transport {
    /// Connected via SSE to mcp.clay.earth (future — reserved).
    /// Currently unused because rmcp doesn't ship an SSE transport feature,
    /// but the variant is kept so the enum is non-exhaustive-ready.
    #[allow(dead_code)]
    Sse,
    /// Connected via stdio child process (npx @clayhq/clay-mcp).
    Stdio,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// MCP client wrapper for the Clay server.
pub struct ClayClient {
    service: RunningService<RoleClient, ()>,
    #[allow(dead_code)]
    transport: Transport,
}

impl ClayClient {
    /// Connect to the Clay MCP server.
    ///
    /// Strategy:
    /// 1. Try SSE to `https://mcp.clay.earth/mcp` (currently skipped — rmcp has
    ///    no SSE transport feature compiled in; left as a TODO for when the crate
    ///    adds `transport-sse`).
    /// 2. Fall back to spawning `npx -y @clayhq/clay-mcp` via stdio.
    ///
    /// The API key is passed as `CLAY_API_KEY` env var to the child process.
    pub async fn connect(api_key: &str) -> Result<Self, ClayError> {
        if api_key.is_empty() {
            return Err(ClayError::NoApiKey);
        }

        // TODO: When rmcp gains `transport-sse`, try SSE first:
        //   SseTransport::new("https://mcp.clay.earth/mcp")
        //       .with_header("Authorization", format!("Bearer {}", api_key))
        // For now, go straight to stdio fallback.

        Self::connect_stdio(api_key).await
    }

    /// Stdio fallback: spawn `npx -y @clayhq/clay-mcp`.
    async fn connect_stdio(api_key: &str) -> Result<Self, ClayError> {
        if !Self::npx_available() {
            return Err(ClayError::NpxNotFound);
        }

        let mut cmd = tokio::process::Command::new("npx");
        cmd.arg("-y").arg("@clayhq/clay-mcp");
        cmd.env("CLAY_API_KEY", api_key);

        let transport = TokioChildProcess::new(&mut cmd)
            .map_err(|e| ClayError::SpawnFailed(e.to_string()))?;

        let service = ()
            .serve(transport)
            .await
            .map_err(|e| ClayError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            service,
            transport: Transport::Stdio,
        })
    }

    // -----------------------------------------------------------------------
    // Tool methods
    // -----------------------------------------------------------------------

    /// Search for contacts matching a free-text query (name, email, company, etc.).
    pub async fn search_contact(&self, query: &str) -> Result<Vec<ClayContact>, ClayError> {
        let result = self
            .service
            .call_tool(CallToolRequestParam {
                name: "search_contacts".into(),
                arguments: serde_json::json!({ "query": query })
                    .as_object()
                    .cloned(),
            })
            .await
            .map_err(|e| ClayError::ToolCallFailed(e.to_string()))?;

        if result.is_error == Some(true) {
            return Err(ClayError::ToolCallFailed(Self::extract_error_text(&result)));
        }

        let text = Self::extract_text(&result);
        serde_json::from_str(&text).map_err(|e| {
            ClayError::ParseError(format!("search_contacts response: {}: {}", e, text))
        })
    }

    /// Fetch full detail for a specific contact by ID.
    pub async fn get_contact_detail(
        &self,
        contact_id: &str,
    ) -> Result<ClayContactDetail, ClayError> {
        let result = self
            .service
            .call_tool(CallToolRequestParam {
                name: "get_contact".into(),
                arguments: serde_json::json!({ "contactId": contact_id })
                    .as_object()
                    .cloned(),
            })
            .await
            .map_err(|e| ClayError::ToolCallFailed(e.to_string()))?;

        if result.is_error == Some(true) {
            return Err(ClayError::ToolCallFailed(Self::extract_error_text(&result)));
        }

        let text = Self::extract_text(&result);
        serde_json::from_str(&text).map_err(|e| {
            ClayError::ParseError(format!("get_contact response: {}: {}", e, text))
        })
    }

    /// Fetch interaction statistics for a contact.
    pub async fn get_contact_stats(
        &self,
        contact_id: &str,
    ) -> Result<ClayContactStats, ClayError> {
        let result = self
            .service
            .call_tool(CallToolRequestParam {
                name: "get_contact_stats".into(),
                arguments: serde_json::json!({ "contactId": contact_id })
                    .as_object()
                    .cloned(),
            })
            .await
            .map_err(|e| ClayError::ToolCallFailed(e.to_string()))?;

        if result.is_error == Some(true) {
            return Err(ClayError::ToolCallFailed(Self::extract_error_text(&result)));
        }

        let text = Self::extract_text(&result);
        serde_json::from_str(&text).map_err(|e| {
            ClayError::ParseError(format!("get_contact_stats response: {}: {}", e, text))
        })
    }

    /// Add a note to a contact record in Clay.
    pub async fn add_note(&self, contact_id: &str, note: &str) -> Result<(), ClayError> {
        let result = self
            .service
            .call_tool(CallToolRequestParam {
                name: "add_note".into(),
                arguments: serde_json::json!({
                    "contactId": contact_id,
                    "note": note,
                })
                .as_object()
                .cloned(),
            })
            .await
            .map_err(|e| ClayError::ToolCallFailed(e.to_string()))?;

        if result.is_error == Some(true) {
            return Err(ClayError::ToolCallFailed(Self::extract_error_text(&result)));
        }

        Ok(())
    }

    /// Disconnect from the Clay MCP server, shutting down the child process.
    pub async fn disconnect(self) {
        let _ = self.service.cancel().await;
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Verify that npx is available on PATH.
    pub fn npx_available() -> bool {
        std::process::Command::new("npx")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Concatenate all text content from a tool call result.
    fn extract_text(result: &rmcp::model::CallToolResult) -> String {
        result
            .content
            .iter()
            .filter_map(|c| c.as_text().map(|t| t.text.as_str()))
            .collect()
    }

    /// Extract a human-readable error message from a tool call result.
    fn extract_error_text(result: &rmcp::model::CallToolResult) -> String {
        result
            .content
            .first()
            .and_then(|c| c.as_text())
            .map(|t| t.text.clone())
            .unwrap_or_else(|| "Unknown error".to_string())
    }
}

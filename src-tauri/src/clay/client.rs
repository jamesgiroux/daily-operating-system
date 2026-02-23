//! MCP client for communicating with the Clay MCP server.
//!
//! Dual transport strategy:
//! 1. SSE primary — connects to `https://mcp.clay.earth/mcp` with Bearer auth
//! 2. Stdio fallback — spawns `npx -y @clayhq/clay-mcp` as a child process
//!
//! Since the rmcp crate in this project does not include an SSE transport feature,
//! the SSE path uses raw reqwest + manual JSON-RPC over HTTP (MCP Streamable HTTP).
//! The stdio path uses the standard rmcp `TokioChildProcess` transport.

use rmcp::model::CallToolRequestParam;
use rmcp::service::RunningService;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::{RoleClient, ServiceExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

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
    #[error("SSE error: {0}")]
    SseError(String),
}

// ---------------------------------------------------------------------------
// JSON-RPC types for SSE transport
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    #[serde(default)]
    message: String,
}

// ---------------------------------------------------------------------------
// Transport wrapper
// ---------------------------------------------------------------------------

/// SSE transport state for direct HTTP communication with Clay MCP.
struct SseTransport {
    client: reqwest::Client,
    token: String,
    endpoint: String,
    next_id: AtomicU64,
}

/// Which transport the client connected through.
enum Transport {
    /// Connected via SSE/HTTP to mcp.clay.earth with Bearer auth.
    Sse(SseTransport),
    /// Connected via stdio child process (npx @clayhq/clay-mcp).
    Stdio(RunningService<RoleClient, ()>),
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// MCP client wrapper for the Clay server.
pub struct ClayClient {
    transport: Transport,
}

impl ClayClient {
    /// Connect to the Clay MCP server.
    ///
    /// Strategy:
    /// 1. If an OAuth token exists in keychain, try SSE to `https://mcp.clay.earth/mcp`.
    /// 2. Fall back to spawning `npx -y @clayhq/clay-mcp` via stdio with the API key.
    pub async fn connect(api_key: &str) -> Result<Self, ClayError> {
        if api_key.is_empty() {
            return Err(ClayError::NoApiKey);
        }

        // Try SSE first if we have an OAuth token from keychain
        if let Some(token) = super::oauth::get_clay_token() {
            match Self::connect_sse(&token).await {
                Ok(client) => return Ok(client),
                Err(e) => {
                    log::warn!("Clay SSE connection failed, falling back to stdio: {}", e);
                }
            }
        }

        Self::connect_stdio(api_key).await
    }

    /// Connect via SSE to the Clay MCP endpoint.
    async fn connect_sse(token: &str) -> Result<Self, ClayError> {
        let endpoint = "https://mcp.clay.earth/mcp".to_string();
        let client = reqwest::Client::new();

        let sse = SseTransport {
            client,
            token: token.to_string(),
            endpoint,
            next_id: AtomicU64::new(1),
        };

        // Send initialize request to verify the connection works
        let init_request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: sse.next_id.fetch_add(1, Ordering::Relaxed),
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "dailyos",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            })),
        };

        let response = sse
            .client
            .post(&sse.endpoint)
            .header("Authorization", format!("Bearer {}", sse.token))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&init_request)
            .send()
            .await
            .map_err(|e| ClayError::SseError(format!("Initialize request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ClayError::SseError(format!(
                "Initialize returned status {}",
                response.status()
            )));
        }

        // Parse the response — it may come as SSE or direct JSON
        let body = response
            .text()
            .await
            .map_err(|e| ClayError::SseError(format!("Failed to read init response: {}", e)))?;

        let json_text = extract_json_from_sse_or_raw(&body);
        let resp: JsonRpcResponse = serde_json::from_str(&json_text).map_err(|e| {
            ClayError::SseError(format!("Failed to parse init response: {}: {}", e, body))
        })?;

        if let Some(err) = resp.error {
            return Err(ClayError::SseError(format!(
                "Initialize error: {}",
                err.message
            )));
        }

        // Send initialized notification (no response expected)
        let notif = JsonRpcRequest {
            jsonrpc: "2.0",
            id: sse.next_id.fetch_add(1, Ordering::Relaxed),
            method: "notifications/initialized".to_string(),
            params: None,
        };

        let _ = sse
            .client
            .post(&sse.endpoint)
            .header("Authorization", format!("Bearer {}", sse.token))
            .header("Content-Type", "application/json")
            .json(&notif)
            .send()
            .await;

        log::info!("Clay SSE transport connected successfully");
        Ok(Self {
            transport: Transport::Sse(sse),
        })
    }

    /// Stdio fallback: spawn `npx -y @clayhq/clay-mcp`.
    async fn connect_stdio(api_key: &str) -> Result<Self, ClayError> {
        let npx_path = crate::util::resolve_npx_binary()
            .ok_or(ClayError::NpxNotFound)?;

        let mut cmd = tokio::process::Command::new(npx_path);
        cmd.arg("-y").arg("@clayhq/clay-mcp");
        cmd.env("CLAY_API_KEY", api_key);

        let transport = TokioChildProcess::new(&mut cmd)
            .map_err(|e| ClayError::SpawnFailed(e.to_string()))?;

        let service = ()
            .serve(transport)
            .await
            .map_err(|e| ClayError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            transport: Transport::Stdio(service),
        })
    }

    // -----------------------------------------------------------------------
    // Internal: call_tool dispatch
    // -----------------------------------------------------------------------

    /// Call an MCP tool, dispatching to the appropriate transport.
    async fn call_tool_inner(
        &self,
        name: String,
        arguments: serde_json::Value,
    ) -> Result<String, ClayError> {
        match &self.transport {
            Transport::Sse(sse) => {
                let request = JsonRpcRequest {
                    jsonrpc: "2.0",
                    id: sse.next_id.fetch_add(1, Ordering::Relaxed),
                    method: "tools/call".to_string(),
                    params: Some(serde_json::json!({
                        "name": name,
                        "arguments": arguments.as_object(),
                    })),
                };

                let response = sse
                    .client
                    .post(&sse.endpoint)
                    .header("Authorization", format!("Bearer {}", sse.token))
                    .header("Content-Type", "application/json")
                    .header("Accept", "application/json, text/event-stream")
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| {
                        ClayError::ToolCallFailed(format!("HTTP request failed: {}", e))
                    })?;

                if !response.status().is_success() {
                    return Err(ClayError::ToolCallFailed(format!(
                        "HTTP {} from Clay MCP",
                        response.status()
                    )));
                }

                let body = response.text().await.map_err(|e| {
                    ClayError::ToolCallFailed(format!("Failed to read response: {}", e))
                })?;

                let json_text = extract_json_from_sse_or_raw(&body);
                let resp: JsonRpcResponse =
                    serde_json::from_str(&json_text).map_err(|e| {
                        ClayError::ParseError(format!(
                            "JSON-RPC parse error: {}: {}",
                            e, body
                        ))
                    })?;

                if let Some(err) = resp.error {
                    return Err(ClayError::ToolCallFailed(err.message));
                }

                // Extract text content from the MCP tool result
                let result = resp.result.unwrap_or(serde_json::Value::Null);
                extract_tool_text_from_result(&result)
            }
            Transport::Stdio(service) => {
                let result = service
                    .call_tool(CallToolRequestParam {
                        name: name.into(),
                        arguments: arguments.as_object().cloned(),
                    })
                    .await
                    .map_err(|e| ClayError::ToolCallFailed(e.to_string()))?;

                if result.is_error == Some(true) {
                    return Err(ClayError::ToolCallFailed(Self::extract_error_text(
                        &result,
                    )));
                }

                Ok(Self::extract_text(&result))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Tool methods
    // -----------------------------------------------------------------------

    /// Search for contacts matching a free-text query (name, email, company, etc.).
    pub async fn search_contact(&self, query: &str) -> Result<Vec<ClayContact>, ClayError> {
        let text = self
            .call_tool_inner("search_contacts".to_string(), serde_json::json!({ "query": query }))
            .await?;

        serde_json::from_str(&text).map_err(|e| {
            ClayError::ParseError(format!("search_contacts response: {}: {}", e, text))
        })
    }

    /// Fetch full detail for a specific contact by ID.
    pub async fn get_contact_detail(
        &self,
        contact_id: &str,
    ) -> Result<ClayContactDetail, ClayError> {
        let text = self
            .call_tool_inner(
                "get_contact".to_string(),
                serde_json::json!({ "contactId": contact_id }),
            )
            .await?;

        serde_json::from_str(&text).map_err(|e| {
            ClayError::ParseError(format!("get_contact response: {}: {}", e, text))
        })
    }

    /// Fetch interaction statistics for a contact.
    pub async fn get_contact_stats(
        &self,
        contact_id: &str,
    ) -> Result<ClayContactStats, ClayError> {
        let text = self
            .call_tool_inner(
                "get_contact_stats".to_string(),
                serde_json::json!({ "contactId": contact_id }),
            )
            .await?;

        serde_json::from_str(&text).map_err(|e| {
            ClayError::ParseError(format!("get_contact_stats response: {}: {}", e, text))
        })
    }

    /// Add a note to a contact record in Clay.
    pub async fn add_note(&self, contact_id: &str, note: &str) -> Result<(), ClayError> {
        self.call_tool_inner(
            "add_note".to_string(),
            serde_json::json!({
                "contactId": contact_id,
                "note": note,
            }),
        )
        .await?;
        Ok(())
    }

    /// Disconnect from the Clay MCP server, shutting down the child process.
    pub async fn disconnect(self) {
        match self.transport {
            Transport::Sse(_) => {
                // HTTP transport has no persistent connection to close
            }
            Transport::Stdio(service) => {
                let _ = service.cancel().await;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Verify that npx is available (checks PATH and common install locations).
    pub fn npx_available() -> bool {
        crate::util::resolve_npx_binary().is_some()
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

// ---------------------------------------------------------------------------
// SSE response parsing helpers
// ---------------------------------------------------------------------------

/// Extract JSON from a response body that may be either raw JSON or SSE format.
/// SSE responses contain lines like `data: {...}` — we extract the JSON from those.
fn extract_json_from_sse_or_raw(body: &str) -> String {
    let trimmed = body.trim();

    // If it starts with '{', it's already raw JSON
    if trimmed.starts_with('{') {
        return trimmed.to_string();
    }

    // Otherwise, look for `data:` lines in SSE format and concatenate the JSON
    let mut json_parts = Vec::new();
    for line in trimmed.lines() {
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if !data.is_empty() && data != "[DONE]" {
                json_parts.push(data.to_string());
            }
        }
    }

    if json_parts.is_empty() {
        // Fallback: return the whole body and let the caller's JSON parser fail
        trimmed.to_string()
    } else {
        // Usually there's one JSON object in the data lines
        json_parts.join("")
    }
}

/// Extract text content from a JSON-RPC tool call result.
/// MCP tool results have the shape: `{ "content": [{ "type": "text", "text": "..." }] }`
fn extract_tool_text_from_result(result: &serde_json::Value) -> Result<String, ClayError> {
    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        let text: String = content
            .iter()
            .filter_map(|item| {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    item.get("text").and_then(|t| t.as_str()).map(String::from)
                } else {
                    None
                }
            })
            .collect();
        Ok(text)
    } else {
        // If the result itself is a string or something else, stringify it
        Ok(result.to_string())
    }
}

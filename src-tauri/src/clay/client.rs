//! MCP client for communicating with the Clay MCP server via Smithery Connect.
//!
//! Smithery manages Clay OAuth and credentials. DailyOS sends JSON-RPC over
//! HTTP to `https://api.smithery.ai/connect/{namespace}/{connectionId}/mcp`
//! with a Smithery API key as Bearer auth.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// A contact summary returned by Clay search.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClayContact {
    #[serde(default, alias = "objectID")]
    pub id: String,
    #[serde(default, alias = "fullName", alias = "displayName")]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, alias = "organization")]
    pub company: Option<String>,
    #[serde(default, alias = "avatarURL")]
    pub photo_url: Option<String>,
    #[serde(default, alias = "linkedinURL")]
    pub linkedin_url: Option<String>,
    #[serde(default, alias = "twitterHandle")]
    pub twitter_handle: Option<String>,
}

/// Extended contact detail with bio, title history, and company firmographics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClayContactDetail {
    #[serde(default, alias = "objectID")]
    pub id: String,
    #[serde(default, alias = "fullName", alias = "displayName")]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, alias = "organization")]
    pub company: Option<String>,
    #[serde(default, alias = "avatarURL")]
    pub photo_url: Option<String>,
    #[serde(default, alias = "linkedinURL")]
    pub linkedin_url: Option<String>,
    #[serde(default, alias = "twitterHandle")]
    pub twitter_handle: Option<String>,

    // Extended fields
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default, alias = "organizations")]
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
    #[serde(default, alias = "name")]
    pub company: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from Clay MCP operations.
#[derive(Debug, thiserror::Error)]
pub enum ClayError {
    #[error("MCP connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Tool call failed: {0}")]
    ToolCallFailed(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("No Smithery credentials configured")]
    NoCredentials,
}

// ---------------------------------------------------------------------------
// JSON-RPC types
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
// Client
// ---------------------------------------------------------------------------

/// MCP client wrapper for Clay via Smithery Connect.
pub struct ClayClient {
    client: reqwest::Client,
    api_key: String,
    endpoint: String,
    next_id: AtomicU64,
}

impl ClayClient {
    /// Connect to Clay MCP via Smithery Connect.
    ///
    /// Endpoint: `https://api.smithery.ai/connect/{namespace}/{connection_id}/mcp`
    pub async fn connect(
        api_key: &str,
        namespace: &str,
        connection_id: &str,
    ) -> Result<Self, ClayError> {
        if api_key.is_empty() || namespace.is_empty() || connection_id.is_empty() {
            return Err(ClayError::NoCredentials);
        }

        let endpoint = format!(
            "https://api.smithery.ai/connect/{}/{}/mcp",
            namespace, connection_id
        );
        let client = reqwest::Client::new();

        let me = Self {
            client,
            api_key: api_key.to_string(),
            endpoint,
            next_id: AtomicU64::new(1),
        };

        // Verify connection with initialize handshake
        let init_request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: me.next_id.fetch_add(1, Ordering::Relaxed),
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

        let response = me
            .client
            .post(&me.endpoint)
            .header("Authorization", format!("Bearer {}", me.api_key))
            .header("Content-Type", "application/json")
            .json(&init_request)
            .send()
            .await
            .map_err(|e| ClayError::ConnectionFailed(format!("Initialize failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ClayError::ConnectionFailed(format!(
                "Initialize returned status {}",
                response.status()
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| ClayError::ConnectionFailed(format!("Read init response: {}", e)))?;

        let resp: JsonRpcResponse = serde_json::from_str(&body).map_err(|e| {
            ClayError::ConnectionFailed(format!("Parse init response: {}: {}", e, body))
        })?;

        if let Some(err) = resp.error {
            return Err(ClayError::ConnectionFailed(format!(
                "Initialize error: {}",
                err.message
            )));
        }

        // Send initialized notification (fire-and-forget, no id for notifications)
        let notif = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
        });

        let _ = me
            .client
            .post(&me.endpoint)
            .header("Authorization", format!("Bearer {}", me.api_key))
            .header("Content-Type", "application/json")
            .json(&notif)
            .send()
            .await;

        log::info!("Clay MCP connected via Smithery");
        Ok(me)
    }

    // -----------------------------------------------------------------------
    // Internal: JSON-RPC tool call
    // -----------------------------------------------------------------------

    async fn call_tool_inner(
        &self,
        name: String,
        arguments: serde_json::Value,
    ) -> Result<String, ClayError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: self.next_id.fetch_add(1, Ordering::Relaxed),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": name,
                "arguments": arguments.as_object(),
            })),
        };

        let response = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ClayError::ToolCallFailed(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ClayError::ToolCallFailed(format!(
                "HTTP {} from Smithery",
                response.status()
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| ClayError::ToolCallFailed(format!("Read response: {}", e)))?;

        let resp: JsonRpcResponse = serde_json::from_str(&body).map_err(|e| {
            ClayError::ParseError(format!("JSON-RPC parse: {}: {}", e, body))
        })?;

        if let Some(err) = resp.error {
            return Err(ClayError::ToolCallFailed(err.message));
        }

        // Extract text content from MCP tool result
        let result = resp.result.unwrap_or(serde_json::Value::Null);
        extract_tool_text(&result)
    }

    // -----------------------------------------------------------------------
    // Tool methods
    // -----------------------------------------------------------------------

    /// Search for contacts matching a query (email, name, keywords).
    pub async fn search_contact(&self, query: &str) -> Result<Vec<ClayContact>, ClayError> {
        let text = self
            .call_tool_inner(
                "searchContacts".to_string(),
                serde_json::json!({ "keywords": [query], "limit": 10 }),
            )
            .await?;

        // Smithery returns the contacts directly as a JSON array or object
        // Try parsing as array first, then as single object
        if let Ok(contacts) = serde_json::from_str::<Vec<ClayContact>>(&text) {
            return Ok(contacts);
        }

        // Clay sometimes returns a wrapper object with contacts nested
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
            // Try common wrapper shapes
            if let Some(arr) = val.as_array() {
                let contacts: Vec<ClayContact> = arr
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                return Ok(contacts);
            }
            // Single contact returned
            if let Ok(contact) = serde_json::from_value::<ClayContact>(val) {
                return Ok(vec![contact]);
            }
        }

        Err(ClayError::ParseError(format!(
            "searchContacts response: {}",
            &text[..text.len().min(200)]
        )))
    }

    /// Fetch full detail for a specific contact by ID.
    pub async fn get_contact_detail(
        &self,
        contact_id: &str,
    ) -> Result<ClayContactDetail, ClayError> {
        let id_num: serde_json::Value = contact_id
            .parse::<u64>()
            .map(|n| serde_json::json!(n))
            .unwrap_or_else(|_| serde_json::json!(contact_id));

        let text = self
            .call_tool_inner(
                "getContact".to_string(),
                serde_json::json!({ "contact_id": id_num }),
            )
            .await?;

        serde_json::from_str(&text).map_err(|e| {
            ClayError::ParseError(format!(
                "getContact response: {}: {}",
                e,
                &text[..text.len().min(200)]
            ))
        })
    }

    /// Add a note to a contact record in Clay.
    pub async fn add_note(&self, contact_id: &str, note: &str) -> Result<(), ClayError> {
        let id_num: serde_json::Value = contact_id
            .parse::<u64>()
            .map(|n| serde_json::json!(n))
            .unwrap_or_else(|_| serde_json::json!(contact_id));

        self.call_tool_inner(
            "createNote".to_string(),
            serde_json::json!({
                "contact_id": id_num,
                "content": note,
            }),
        )
        .await?;
        Ok(())
    }

    /// Disconnect (no-op for stateless HTTP).
    pub async fn disconnect(self) {
        // Smithery Connect is stateless HTTP — nothing to close
    }
}

// ---------------------------------------------------------------------------
// Response parsing helpers
// ---------------------------------------------------------------------------

/// Extract text content from a JSON-RPC tool call result.
/// MCP tool results have the shape: `{ "content": [{ "type": "text", "text": "..." }] }`
fn extract_tool_text(result: &serde_json::Value) -> Result<String, ClayError> {
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
        Ok(result.to_string())
    }
}

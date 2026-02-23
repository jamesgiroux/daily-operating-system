//! MCP client for communicating with the Clay MCP server via Smithery Connect.
//!
//! Smithery manages Clay OAuth and credentials. DailyOS sends JSON-RPC over
//! HTTP to `https://api.smithery.ai/connect/{namespace}/{connectionId}/mcp`
//! with a Smithery API key as Bearer auth.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Deserialize an ID that may be either a JSON number or string.
fn deserialize_id<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let val = serde_json::Value::deserialize(d)?;
    match val {
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::String(s) => Ok(s),
        _ => Ok(String::new()),
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// A contact summary returned by Clay search.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClayContact {
    #[serde(default, alias = "objectID")]
    pub id: serde_json::Value,
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

impl ClayContact {
    /// Extract contact ID as a string (handles both number and string JSON values).
    pub fn id_str(&self) -> String {
        match &self.id {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        }
    }
}

/// Extended contact detail with bio, title history, and company firmographics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClayContactDetail {
    #[serde(default, alias = "objectID")]
    pub id: serde_json::Value,
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
            .header("Accept", "application/json, text/event-stream")
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

        let json_text = extract_json_from_response(&body);
        let resp: JsonRpcResponse = serde_json::from_str(&json_text).map_err(|e| {
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
            .header("Accept", "application/json, text/event-stream")
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
            .header("Accept", "application/json, text/event-stream")
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

        let json_text = extract_json_from_response(&body);
        let resp: JsonRpcResponse = serde_json::from_str(&json_text).map_err(|e| {
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
        // Use name filter for name queries, keywords for emails
        let args = if query.contains('@') {
            serde_json::json!({ "keywords": [query], "limit": 10 })
        } else {
            serde_json::json!({ "name": [query], "limit": 10 })
        };

        let text = self
            .call_tool_inner("searchContacts".to_string(), args)
            .await?;

        // Smithery Clay returns: {"total": N, "results": [...]}
        let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
            ClayError::ParseError(format!("searchContacts: {}: {}", e, &text[..text.len().min(200)]))
        })?;

        let arr = val.get("results").and_then(|r| r.as_array())
            .or_else(|| val.as_array());

        let contacts = match arr {
            Some(arr) => arr.iter().map(|v| {
                let mut contact: ClayContact = serde_json::from_value(v.clone()).unwrap_or_default();
                // Smithery search results don't have an email field — but if
                // displayName looks like an email, use it
                if contact.email.is_none() {
                    if let Some(name) = &contact.name {
                        if name.contains('@') {
                            contact.email = Some(name.clone());
                        }
                    }
                }
                contact
            }).collect(),
            None => vec![],
        };

        Ok(contacts)
    }

    /// Fetch full detail for a specific contact by ID.
    ///
    /// Smithery Clay returns a different schema than our struct, so we parse
    /// the raw JSON and extract fields manually.
    pub async fn get_contact_detail(
        &self,
        contact_id: &str,
    ) -> Result<ClayContactDetail, ClayError> {
        // Smithery Clay requires contact_id as a number
        let id_num: u64 = contact_id.parse::<u64>().map_err(|_| {
            ClayError::ToolCallFailed(format!("contact_id '{}' is not a valid number", contact_id))
        })?;

        eprintln!("[clay] getContact contact_id={} (u64)", id_num);
        let text = self
            .call_tool_inner(
                "getContact".to_string(),
                serde_json::json!({ "contact_id": id_num }),
            )
            .await?;

        let raw: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
            ClayError::ParseError(format!("getContact JSON: {}: {}", e, &text[..text.len().min(200)]))
        })?;

        // Extract first email from emails array
        let email = raw.get("emails")
            .and_then(|e| e.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .map(String::from);

        // Extract first phone from phone_numbers array
        let phone = raw.get("phone_numbers")
            .and_then(|p| p.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .map(String::from);

        // Extract linkedin/twitter from social_links array
        let social_links: Vec<&str> = raw.get("social_links")
            .and_then(|s| s.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        let linkedin_url = social_links.iter()
            .find(|u| u.contains("linkedin.com"))
            .map(|u| u.to_string());
        let twitter_handle = social_links.iter()
            .find(|u| u.contains("twitter.com") || u.contains("x.com"))
            .map(|u| u.to_string());

        // Extract work history and derive current title/company
        let work_history: Vec<TitleHistoryEntry> = raw.get("work_history")
            .and_then(|w| w.as_array())
            .map(|arr| arr.iter().map(|entry| TitleHistoryEntry {
                title: entry.get("title").and_then(|t| t.as_str()).map(String::from),
                company: entry.get("company").and_then(|c| c.as_str()).map(String::from),
                start_date: entry.get("start_year").map(|y| y.to_string()),
                end_date: entry.get("end_year").map(|y| y.to_string()),
            }).collect())
            .unwrap_or_default();

        let current_job = work_history.first();
        let title = current_job.and_then(|j| j.title.clone());
        let company = current_job.and_then(|j| j.company.clone());

        Ok(ClayContactDetail {
            id: serde_json::Value::String(contact_id.to_string()),
            name: raw.get("displayName").or_else(|| raw.get("name"))
                .and_then(|n| n.as_str()).map(String::from),
            email,
            title,
            company,
            photo_url: raw.get("avatarURL").and_then(|u| u.as_str()).map(String::from),
            linkedin_url,
            twitter_handle,
            bio: raw.get("bio").and_then(|b| b.as_str()).map(String::from),
            phone,
            title_history: work_history,
            // Firmographics not available from Smithery getContact
            company_industry: None,
            company_size: None,
            company_hq: raw.get("location").and_then(|l| l.as_str()).map(String::from),
            company_funding: None,
        })
    }

    /// Add a note to a contact record in Clay.
    pub async fn add_note(&self, contact_id: &str, note: &str) -> Result<(), ClayError> {
        let id_num: u64 = contact_id.parse::<u64>().map_err(|_| {
            ClayError::ToolCallFailed(format!("contact_id '{}' is not a valid number", contact_id))
        })?;

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

/// Extract JSON from a response body that may be either raw JSON or SSE format.
/// Smithery may return SSE-formatted responses with `data:` prefixed lines.
fn extract_json_from_response(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.starts_with('{') {
        return trimmed.to_string();
    }
    // Look for `data:` lines in SSE format
    for line in trimmed.lines() {
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if !data.is_empty() && data != "[DONE]" && data.starts_with('{') {
                return data.to_string();
            }
        }
    }
    trimmed.to_string()
}

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

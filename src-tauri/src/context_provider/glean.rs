//! Glean context provider — gathers entity context from Glean's knowledge graph.
//!
//! Uses Glean's MCP server (`/mcp/default`) via HTTP+SSE transport.
//! Two MCP tools used:
//! - `search` — search Glean's index for entity-related documents
//! - `read_document` — fetch full document content by URL
//!
//! Authentication: OAuth token stored in macOS Keychain.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::context_provider::cache::{CacheKind, GleanCache};
use crate::context_provider::{ContextError, ContextProvider};
use crate::db::ActionDb;
use crate::intelligence::prompts::{GapQueryItem, IntelligenceContext};
use crate::intelligence::IntelligenceJson;

/// Timeout for lightweight non-chat Glean API calls (list_sources, search,
/// read_document). 30s leaves headroom for internet latency + Glean
/// backend variance without letting a dead endpoint stall forever.
const GLEAN_CALL_TIMEOUT: Duration = Duration::from_secs(30);
/// Longer timeout for Glean `chat` tool — AI synthesis takes 10-30s.
// 240s: Glean chat is agentic — runs internal search tool-calls before
// returning the final answer. For well-documented accounts (lots of Gong
// transcripts / SFDC records / Zendesk tickets), chat can take minutes.
// 60s was too tight and caused cascade timeout → PTY fallback → items
// tagged with local source instead of glean_*. 240s matches the outer
// timeout in enrich_entity_parallel so slow-but-valid responses complete.
const GLEAN_CHAT_TIMEOUT: Duration = Duration::from_secs(240);

/// Maximum documents to fetch per entity context gather.
const MAX_DOCUMENTS_PER_ENTITY: usize = 10;
/// Hard recency window for Glean documents used in account intelligence.
const GLEAN_DOCUMENT_RECENCY_DAYS: i64 = 365;

// ---------------------------------------------------------------------------
// Glean MCP response types
// ---------------------------------------------------------------------------

/// A search result from Glean's search tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GleanSearchResult {
    pub title: Option<String>,
    pub url: Option<String>,
    pub snippet: Option<String>,
    /// Document type in Glean (e.g., "confluence_page", "google_doc", "salesforce_account")
    #[serde(rename = "type")]
    pub doc_type: Option<String>,
    /// Author/owner of the document
    pub author: Option<String>,
    /// Last updated timestamp
    pub updated_at: Option<String>,
}

/// A person result from Glean's people search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GleanPersonResult {
    pub name: Option<String>,
    pub email: Option<String>,
    pub title: Option<String>,
    pub department: Option<String>,
    pub manager: Option<String>,
    pub location: Option<String>,
    pub start_date: Option<String>,
}

fn parse_glean_updated_at(updated_at: Option<&str>) -> Option<DateTime<Utc>> {
    updated_at
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
}

fn prioritize_recent_results(mut results: Vec<GleanSearchResult>) -> Vec<GleanSearchResult> {
    results.sort_by(|a, b| {
        parse_glean_updated_at(b.updated_at.as_deref())
            .cmp(&parse_glean_updated_at(a.updated_at.as_deref()))
            .then_with(|| a.title.cmp(&b.title))
    });

    let cutoff = Utc::now() - chrono::Duration::days(GLEAN_DOCUMENT_RECENCY_DAYS);
    let has_recent_dated = results.iter().any(|result| {
        parse_glean_updated_at(result.updated_at.as_deref()).is_some_and(|ts| ts >= cutoff)
    });
    if !has_recent_dated {
        return results;
    }

    results
        .into_iter()
        .filter(|result| {
            parse_glean_updated_at(result.updated_at.as_deref()).is_some_and(|ts| ts >= cutoff)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Glean MCP Client
// ---------------------------------------------------------------------------

/// HTTP client for Glean's MCP server.
///
/// The Glean MCP server exposes tools via the standard MCP protocol over
/// HTTP+SSE transport. We call it directly via `reqwest` using the JSON-RPC
/// wire format rather than using the `rmcp` crate, because:
/// 1. Glean uses HTTP+SSE transport (not stdio)
/// 2. We need fine-grained timeout control per-call
/// 3. The MCP tool calls are simple JSON-RPC POST requests
pub struct GleanMcpClient {
    endpoint: String,
    client: reqwest::Client,
}

impl GleanMcpClient {
    /// Create a new client for the given MCP endpoint.
    ///
    /// Does NOT take a static token. Each request fetches a fresh
    /// (possibly refreshed) token via `glean::get_valid_access_token()`.
    pub fn new(endpoint: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(GLEAN_CALL_TIMEOUT)
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                headers.insert(
                    reqwest::header::ACCEPT,
                    reqwest::header::HeaderValue::from_static(
                        "application/json, text/event-stream",
                    ),
                );
                headers
            })
            .build()
            .unwrap_or_default();

        Self {
            endpoint: endpoint.to_string(),
            client,
        }
    }

    /// Get a valid access token for this request.
    ///
    /// Transparently refreshes expired tokens via Keychain + token endpoint.
    fn get_token(&self) -> Result<String, ContextError> {
        // Never refresh tokens on the caller's Tokio worker. Use a
        // dedicated OS thread + single-thread runtime for the async refresh.
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    ContextError::Other(format!("Failed to build Glean token runtime: {}", e))
                })
                .and_then(|rt| {
                    rt.block_on(crate::glean::get_valid_access_token())
                        .map_err(|e| ContextError::Auth(format!("Glean token error: {}", e)))
                });
            let _ = tx.send(result);
        });
        rx.recv()
            .map_err(|_| ContextError::Auth("Token thread panicked".to_string()))?
    }

    /// Search Glean for documents related to a query.
    pub async fn search(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<GleanSearchResult>, ContextError> {
        let token = self.get_token()?;
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "search",
                "arguments": {
                    "query": query,
                    "maxResults": max_results
                }
            }
        });

        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ContextError::Timeout(format!("Glean search timed out: {}", e))
                } else if e.is_status() {
                    ContextError::Auth(format!("Glean auth error: {}", e))
                } else {
                    ContextError::Other(format!("Glean search failed: {}", e))
                }
            })?;

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(ContextError::Auth(format!("Glean returned {}", status)));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ContextError::Other(format!("Failed to parse Glean response: {}", e)))?;

        // Extract results from MCP tool response
        // MCP response: { "result": { "content": [{ "type": "text", "text": "..." }] } }
        let content_text = json
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("[]");

        let results: Vec<GleanSearchResult> =
            serde_json::from_str(content_text).unwrap_or_default();

        Ok(results)
    }

    /// Search for people in Glean's org graph.
    pub async fn search_people(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<GleanPersonResult>, ContextError> {
        let token = self.get_token()?;
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "search",
                "arguments": {
                    "query": format!("people: {}", query),
                    "maxResults": max_results
                }
            }
        });

        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ContextError::Timeout(format!("Glean people search timed out: {}", e))
                } else {
                    ContextError::Other(format!("Glean people search failed: {}", e))
                }
            })?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ContextError::Other(format!("Failed to parse response: {}", e)))?;

        let content_text = json
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("[]");

        let results: Vec<GleanPersonResult> =
            serde_json::from_str(content_text).unwrap_or_default();

        Ok(results)
    }

    /// Read a specific document's content from Glean.
    pub async fn read_document(&self, url: &str) -> Result<Option<String>, ContextError> {
        let token = self.get_token()?;
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "read_document",
                "arguments": {
                    "url": url
                }
            }
        });

        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ContextError::Timeout(format!("Glean read_document timed out: {}", e))
                } else {
                    ContextError::Other(format!("Glean read_document failed: {}", e))
                }
            })?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ContextError::Other(format!("Failed to parse response: {}", e)))?;

        let content = json
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());

        Ok(content)
    }

    /// AI-powered chat for structured intelligence queries.
    ///
    /// Calls the Glean MCP `chat` tool which synthesizes across all connected
    /// data sources (Salesforce, Zendesk, Gong, Slack, etc.) with multi-step
    /// reasoning. Returns the final AI-generated text response.
    ///
    /// Uses a longer timeout (60s) than search (10s) because AI synthesis
    /// involves multiple internal search + read steps.
    ///
    /// The response is the raw text from Glean's AI — callers are responsible
    /// for parsing JSON if they requested structured output.
    pub async fn chat(
        &self,
        message: &str,
        context: Option<Vec<String>>,
    ) -> Result<String, ContextError> {
        self.chat_inner(message, context)
            .await
            .map(|resp| resp.text)
    }

    /// AI-powered chat that also returns citation metadata.
    ///
    /// Same wire call as [`chat`], but extracts the citation count from the
    /// response envelope so callers (e.g., the peer-benchmark cell) can
    /// render a "Drawn from N Glean sources" footer.
    ///
    /// `source_count` is best-effort: if Glean's envelope doesn't carry
    /// citation metadata in a recognised shape, it is `0` rather than
    /// erroring. Callers should treat 0 as "no citation count available"
    /// and degrade gracefully.
    pub async fn chat_with_citations(
        &self,
        message: &str,
        context: Option<Vec<String>>,
    ) -> Result<ChatResponse, ContextError> {
        self.chat_inner(message, context).await
    }

    /// Shared implementation for [`chat`] and [`chat_with_citations`].
    ///
    /// Performs the JSON-RPC `tools/call` for the `chat` tool, walks the
    /// MCP envelope to extract the final GLEAN_AI message text, and counts
    /// citations attached to that message.
    async fn chat_inner(
        &self,
        message: &str,
        context: Option<Vec<String>>,
    ) -> Result<ChatResponse, ContextError> {
        let token = self.get_token()?;

        let mut arguments = serde_json::json!({
            "message": message,
        });
        if let Some(ctx) = context {
            arguments["context"] = serde_json::json!(ctx);
        }

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "chat",
                "arguments": arguments
            }
        });

        // Build a client with the longer chat timeout
        let chat_client = reqwest::Client::builder()
            .timeout(GLEAN_CHAT_TIMEOUT)
            .build()
            .unwrap_or_default();

        let response = chat_client
            .post(&self.endpoint)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ContextError::Timeout(format!("Glean chat timed out after 60s: {}", e))
                } else if e.is_status() {
                    ContextError::Auth(format!("Glean chat auth error: {}", e))
                } else {
                    ContextError::Other(format!("Glean chat failed: {}", e))
                }
            })?;

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(ContextError::Auth(format!(
                "Glean chat returned {}",
                status
            )));
        }

        let json: serde_json::Value = response.json().await.map_err(|e| {
            ContextError::Other(format!("Failed to parse Glean chat response: {}", e))
        })?;

        // The chat tool returns a different structure than search:
        // { "result": { "content": [{ "type": "text", "text": "{\"messages\":[...]}" }] } }
        // Inside the text field is a JSON object with a `messages` array.
        // We need the last GLEAN_AI message that isn't an UPDATE (search step).
        let content_text = json
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| {
                // Check for error response
                if let Some(err) = json.get("error") {
                    ContextError::Other(format!("Glean chat error: {}", err))
                } else {
                    ContextError::Other("Glean chat returned empty response".to_string())
                }
            })?;

        // Check if content_text is an error message
        if content_text.starts_with("Error running tool") {
            return Err(ContextError::Other(format!(
                "Glean chat tool error: {}",
                content_text
            )));
        }

        // Try to parse the messages envelope and extract the final AI response.
        if let Ok(envelope) = serde_json::from_str::<serde_json::Value>(content_text) {
            if let Some(messages) = envelope.get("messages").and_then(|m| m.as_array()) {
                // Find the last GLEAN_AI message that is CONTENT type (not UPDATE)
                for msg in messages.iter().rev() {
                    let is_ai = msg
                        .get("author")
                        .and_then(|a| a.as_str())
                        .map(|a| a == "GLEAN_AI")
                        .unwrap_or(false);
                    let is_content = msg
                        .get("messageType")
                        .and_then(|t| t.as_str())
                        .map(|t| t != "UPDATE")
                        .unwrap_or(true);

                    if is_ai && is_content {
                        // Concatenate all text fragments from this message
                        let text = msg
                            .get("fragments")
                            .and_then(|f| f.as_array())
                            .map(|frags| {
                                frags
                                    .iter()
                                    .filter_map(|f| f.get("text").and_then(|t| t.as_str()))
                                    .collect::<Vec<_>>()
                                    .join("")
                            })
                            .unwrap_or_default();

                        if !text.is_empty() {
                            let source_count = count_citations(msg, &envelope);
                            return Ok(ChatResponse { text, source_count });
                        }
                    }
                }
            }
        }

        // Fallback: return the raw content text (might already be the final answer).
        // No structured envelope -> no citation count.
        Ok(ChatResponse {
            text: content_text.to_string(),
            source_count: 0,
        })
    }
}

/// Response from a Glean chat call including citation metadata.
///
/// `source_count` is the number of distinct sources Glean cited when
/// producing `text`. A value of 0 means either Glean returned no citations
/// or the envelope shape didn't expose them in a way we recognised.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub text: String,
    pub source_count: u32,
}

/// Count citations attached to a GLEAN_AI content message.
///
/// Glean's MCP envelope shape for citations is not formally documented and
/// has varied across versions. We probe the most likely locations in
/// preference order and return the first non-zero count:
///
/// 1. CITATION-typed fragments inside `msg.fragments[]`
/// 2. A `citations` / `citationList` / `sources` array on `msg`
/// 3. A top-level `citations` / `citationList` array on the envelope
///
/// Citation entries are deduped by URL (or `documentId`/`opaqueRef` when no
/// URL is present) so a multi-fragment citation of the same Salesforce
/// record counts once. Returns 0 when no recognised citation field is
/// present — callers treat that as "unknown" and render the cell without
/// a source-count footer.
fn count_citations(msg: &serde_json::Value, envelope: &serde_json::Value) -> u32 {
    use std::collections::HashSet;

    fn absorb_array(arr: &[serde_json::Value], keys: &mut HashSet<String>) {
        for item in arr {
            // Prefer URL → documentId → opaqueRef → id → full json string as the dedupe key
            let key = item
                .get("url")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("sourceUrl").and_then(|v| v.as_str()))
                .or_else(|| item.get("documentId").and_then(|v| v.as_str()))
                .or_else(|| item.get("opaqueRef").and_then(|v| v.as_str()))
                .or_else(|| item.get("id").and_then(|v| v.as_str()))
                .map(|s| s.to_string())
                .unwrap_or_else(|| item.to_string());
            keys.insert(key);
        }
    }

    let mut keys: HashSet<String> = HashSet::new();

    // 1. CITATION-typed fragments on the message
    if let Some(frags) = msg.get("fragments").and_then(|f| f.as_array()) {
        let cit_frags: Vec<&serde_json::Value> = frags
            .iter()
            .filter(|f| {
                f.get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t.eq_ignore_ascii_case("CITATION"))
                    .unwrap_or(false)
                    || f.get("citation").is_some()
            })
            .collect();
        if !cit_frags.is_empty() {
            for f in cit_frags {
                // Citation payload may live under `citation`, `source`, or be the fragment itself.
                let payload = f.get("citation").or_else(|| f.get("source")).unwrap_or(f);
                if let Some(arr) = payload.as_array() {
                    absorb_array(arr, &mut keys);
                } else {
                    absorb_array(std::slice::from_ref(payload), &mut keys);
                }
            }
            if !keys.is_empty() {
                return keys.len() as u32;
            }
        }
    }

    // 2. Citations / citationList / sources array on the message itself
    for field in &["citations", "citationList", "sources"] {
        if let Some(arr) = msg.get(*field).and_then(|v| v.as_array()) {
            absorb_array(arr, &mut keys);
        }
    }
    if !keys.is_empty() {
        return keys.len() as u32;
    }

    // 3. Top-level citations / citationList on the envelope
    for field in &["citations", "citationList"] {
        if let Some(arr) = envelope.get(*field).and_then(|v| v.as_array()) {
            absorb_array(arr, &mut keys);
        }
    }

    keys.len() as u32
}

// ---------------------------------------------------------------------------
// Org Health Data Parsing
// ---------------------------------------------------------------------------

/// Parse org-level health data from Glean search results.
///
/// Looks for health signals in Salesforce, Zendesk, and other CRM-type documents.
/// Priority: salesforce_account > zendesk_organization > other doc types.
fn parse_org_health_data(
    results: &[GleanSearchResult],
    _account_name: &str,
) -> Option<crate::intelligence::io::OrgHealthData> {
    // Sort results by doc_type priority
    let mut prioritized: Vec<&GleanSearchResult> =
        results.iter().filter(|r| r.snippet.is_some()).collect();

    prioritized.sort_by(|a, b| {
        let priority = |dt: Option<&str>| match dt {
            Some(t) if t.contains("salesforce") => 0,
            Some(t) if t.contains("zendesk") => 1,
            Some(t) if t.contains("hubspot") || t.contains("gainsight") => 2,
            _ => 3,
        };
        priority(a.doc_type.as_deref()).cmp(&priority(b.doc_type.as_deref()))
    });

    let mut health_band: Option<String> = None;
    let mut renewal_likelihood: Option<String> = None;
    let mut growth_tier: Option<String> = None;
    let mut customer_stage: Option<String> = None;
    let mut support_tier: Option<String> = None;
    let mut icp_fit: Option<String> = None;
    let mut best_source = String::new();

    for result in &prioritized {
        let snippet = result.snippet.as_deref().unwrap_or("");
        let title = result.title.as_deref().unwrap_or("");
        let combined = format!("{} {}", title, snippet).to_lowercase();

        // Health band detection
        if health_band.is_none() {
            if combined.contains("health_score_3_green")
                || combined.contains("health score: green")
                || combined.contains("health: green")
            {
                health_band = Some("green".to_string());
            } else if combined.contains("health_score_2_yellow")
                || combined.contains("health score: yellow")
                || combined.contains("health: yellow")
            {
                health_band = Some("yellow".to_string());
            } else if combined.contains("health_score_1_red")
                || combined.contains("health score: red")
                || combined.contains("health: red")
            {
                health_band = Some("red".to_string());
            }
        }

        // Field pattern matching (case-insensitive on combined text)
        for line in snippet.lines().chain(title.lines()) {
            let line_lower = line.to_lowercase();

            if renewal_likelihood.is_none() {
                if let Some(rest) = line_lower.strip_prefix("renewal likelihood:") {
                    renewal_likelihood = Some(rest.trim().to_string());
                }
            }
            if growth_tier.is_none() {
                if let Some(rest) = line_lower.strip_prefix("growth tier:") {
                    growth_tier = Some(rest.trim().to_string());
                }
            }
            if customer_stage.is_none() {
                if let Some(rest) = line_lower.strip_prefix("customer stage:") {
                    customer_stage = Some(rest.trim().to_string());
                }
            }
            if support_tier.is_none() {
                if let Some(rest) = line_lower.strip_prefix("support tier:") {
                    support_tier = Some(rest.trim().to_string());
                }
            }
            if icp_fit.is_none() {
                if let Some(rest) = line_lower.strip_prefix("icp fit:") {
                    icp_fit = Some(rest.trim().to_string());
                }
            }
        }

        // Track best source
        if best_source.is_empty()
            && (health_band.is_some()
                || renewal_likelihood.is_some()
                || growth_tier.is_some()
                || customer_stage.is_some())
        {
            best_source = result.doc_type.as_deref().unwrap_or("unknown").to_string();
        }
    }

    // Only return if we found at least one health-relevant field
    if health_band.is_none()
        && renewal_likelihood.is_none()
        && growth_tier.is_none()
        && customer_stage.is_none()
        && support_tier.is_none()
        && icp_fit.is_none()
    {
        return None;
    }

    Some(crate::intelligence::io::OrgHealthData {
        health_band,
        health_score: None,
        renewal_likelihood,
        growth_tier,
        customer_stage,
        support_tier,
        icp_fit,
        source: best_source,
        gathered_at: chrono::Utc::now().to_rfc3339(),
    })
}

// ---------------------------------------------------------------------------
// GleanContextProvider
// ---------------------------------------------------------------------------

/// Enterprise context provider: gathers entity context from Glean search.
///
/// Two-phase gather:
/// - Phase A (DB, ms): Read always-local data (meetings, actions, captures, user_context)
/// - Phase B (network, 200-2000ms): Query Glean for documents, people, org graph
pub struct GleanContextProvider {
    /// Glean MCP server endpoint.
    endpoint: String,
    /// In-memory + DB cache for Glean responses.
    cache: Arc<GleanCache>,
    /// Fallback: local provider for always-local data and Glean outages.
    local_fallback: super::local::LocalContextProvider,
}

impl GleanContextProvider {
    pub fn new(
        endpoint: String,
        cache: Arc<GleanCache>,
        local_fallback: super::local::LocalContextProvider,
    ) -> Self {
        Self {
            endpoint,
            cache,
            local_fallback,
        }
    }

    /// Build Glean search queries for an entity.
    fn entity_search_queries(
        &self,
        db: &ActionDb,
        entity_id: &str,
        entity_type: &str,
    ) -> Vec<String> {
        let mut queries = Vec::new();

        match entity_type {
            "account" => {
                if let Ok(Some(acct)) = db.get_account(entity_id) {
                    queries.push(format!("{} account", acct.name));
                    // Search with domain if available
                    if let Ok(domains) = db.get_account_domains(entity_id) {
                        for d in domains.iter().take(2) {
                            queries.push(d.clone());
                        }
                    }
                }
            }
            "project" => {
                if let Ok(Some(proj)) = db.get_project(entity_id) {
                    queries.push(format!("{} project", proj.name));
                }
            }
            "person" => {
                if let Ok(Some(person)) = db.get_person(entity_id) {
                    queries.push(person.name.clone());
                    queries.push(person.email.clone());
                }
            }
            _ => {}
        }

        queries
    }

    /// Gather Glean-sourced context for file_contents and stakeholders.
    ///
    /// `gap_queries`: dimension-aware gap queries for fan-out search.
    async fn gather_glean_context(
        &self,
        db: &ActionDb,
        entity_id: &str,
        entity_type: &str,
        gap_queries: &[GapQueryItem],
    ) -> Result<GleanEntityData, ContextError> {
        let client = GleanMcpClient::new(&self.endpoint);

        let mut queries = self.entity_search_queries(db, entity_id, entity_type);
        // Append gap queries for dimension-aware Glean fan-out (cap at 3).
        for q in gap_queries.iter().take(3) {
            queries.push(q.query.clone());
        }
        let mut all_results: Vec<GleanSearchResult> = Vec::new();
        let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Search Glean with each query
        for query in &queries {
            // Check cache first
            if let Some(cached) =
                self.cache
                    .get_with_db(CacheKind::Document, &format!("search:{}", query), db)
            {
                if let Ok(results) = serde_json::from_str::<Vec<GleanSearchResult>>(&cached) {
                    for r in results {
                        if let Some(ref url) = r.url {
                            if seen_urls.insert(url.clone()) {
                                all_results.push(r);
                            }
                        }
                    }
                    continue;
                }
            }

            match client.search(query, MAX_DOCUMENTS_PER_ENTITY).await {
                Ok(results) => {
                    // Cache the raw results
                    if let Ok(json) = serde_json::to_string(&results) {
                        self.cache.put(
                            CacheKind::Document,
                            &format!("search:{}", query),
                            &json,
                            db,
                        );
                    }
                    for r in results {
                        if let Some(ref url) = r.url {
                            if seen_urls.insert(url.clone()) {
                                all_results.push(r);
                            }
                        }
                    }
                }
                Err(ContextError::Timeout(msg)) => {
                    log::warn!("Glean search timeout for '{}': {}", query, msg);
                    // Continue with other queries
                }
                Err(e) => return Err(e),
            }
        }

        let all_results = prioritize_recent_results(all_results);

        // Emit Glean document signals for new/updated results
        for result in &all_results {
            let Some(ref snippet) = result.snippet else {
                continue;
            };
            if snippet.is_empty() {
                continue;
            }
            let url = result.url.as_deref().unwrap_or("");
            if url.is_empty() {
                continue;
            }

            let updated_at = result.updated_at.as_deref();
            // Skip if we already have a fresh signal for this document
            let has_existing =
                match db.has_glean_signal_for_url(entity_type, entity_id, url, updated_at, 30) {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!(
                            "I487: failed dedupe check for {} {} url={}: {}",
                            entity_type,
                            entity_id,
                            url,
                            e
                        );
                        false
                    }
                };
            if has_existing {
                continue;
            }

            let value = format!(
                "{}|{}|{}",
                result.title.as_deref().unwrap_or(""),
                result.doc_type.as_deref().unwrap_or(""),
                url,
            );
            let _ = crate::signals::bus::emit_signal(
                db,
                entity_type,
                entity_id,
                "glean_document",
                "glean_search",
                Some(&value),
                0.7,
            );
        }

        // Parse org health data from CRM-type documents (accounts only)
        let org_health = if entity_type == "account" {
            let account_name = db
                .get_account(entity_id)
                .ok()
                .flatten()
                .map(|a| a.name)
                .unwrap_or_default();
            parse_org_health_data(&all_results, &account_name)
        } else {
            None
        };

        // Build file_contents from search results (snippets)
        let mut file_parts: Vec<String> = Vec::new();
        let mut total_bytes = 0usize;
        let max_bytes = 10_000;

        for result in &all_results {
            let title = result.title.as_deref().unwrap_or("Untitled");
            let doc_type = result.doc_type.as_deref().unwrap_or("unknown");
            let snippet = result.snippet.as_deref().unwrap_or("");
            let updated = result.updated_at.as_deref().unwrap_or("unknown");

            let entry = format!(
                "--- {} [{}] ({}) ---\n{}",
                title, doc_type, updated, snippet
            );
            let entry_bytes = entry.len();

            if total_bytes + entry_bytes > max_bytes {
                break;
            }

            file_parts.push(entry);
            total_bytes += entry_bytes;
        }

        // People/org graph search (for stakeholders enrichment)
        let people_results = if entity_type == "account" || entity_type == "project" {
            let entity_name = match entity_type {
                "account" => db.get_account(entity_id).ok().flatten().map(|a| a.name),
                "project" => db.get_project(entity_id).ok().flatten().map(|p| p.name),
                _ => None,
            };

            if let Some(name) = entity_name {
                let cache_key = format!("org:{}", name);
                if let Some(cached) = self.cache.get_with_db(CacheKind::OrgGraph, &cache_key, db) {
                    serde_json::from_str(&cached).unwrap_or_default()
                } else {
                    match client.search_people(&name, 50).await {
                        Ok(results) => {
                            if let Ok(json) = serde_json::to_string(&results) {
                                self.cache.put(CacheKind::OrgGraph, &cache_key, &json, db);
                            }
                            results
                        }
                        Err(e) => {
                            log::warn!("Glean people search failed: {}", e);
                            Vec::new()
                        }
                    }
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(GleanEntityData {
            file_contents: if file_parts.is_empty() {
                String::new()
            } else {
                file_parts.join("\n\n")
            },
            people: people_results,
            search_results: all_results,
            org_health,
        })
    }
}

// ---------------------------------------------------------------------------
// Standalone Glean context gather — runs on a separate OS thread with its
// own DB connection to avoid blocking Tokio worker threads.
// ---------------------------------------------------------------------------

/// Standalone version of `gather_glean_context` that takes pre-extracted search
/// queries and a thread-local DB connection. Used by the spawned thread in
/// `gather_entity_context` to keep network calls off the Tokio thread pool.
async fn gather_glean_context_standalone(
    endpoint: &str,
    cache: &GleanCache,
    db: &crate::db::ActionDb,
    entity_id: &str,
    entity_type: &str,
    pre_queries: &[String],
    gap_queries: &[GapQueryItem],
) -> Result<GleanEntityData, ContextError> {
    let client = GleanMcpClient::new(endpoint);

    let mut queries: Vec<String> = pre_queries.to_vec();
    // Append gap queries for dimension-aware Glean fan-out (cap at 3).
    for q in gap_queries.iter().take(3) {
        queries.push(q.query.clone());
    }

    let mut all_results: Vec<GleanSearchResult> = Vec::new();
    let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Search Glean with each query
    for query in &queries {
        // Check cache first
        if let Some(cached) =
            cache.get_with_db(CacheKind::Document, &format!("search:{}", query), db)
        {
            if let Ok(results) = serde_json::from_str::<Vec<GleanSearchResult>>(&cached) {
                for r in results {
                    if let Some(ref url) = r.url {
                        if seen_urls.insert(url.clone()) {
                            all_results.push(r);
                        }
                    }
                }
                continue;
            }
        }

        match client.search(query, MAX_DOCUMENTS_PER_ENTITY).await {
            Ok(results) => {
                // Cache the raw results
                if let Ok(json) = serde_json::to_string(&results) {
                    cache.put(CacheKind::Document, &format!("search:{}", query), &json, db);
                }
                for r in results {
                    if let Some(ref url) = r.url {
                        if seen_urls.insert(url.clone()) {
                            all_results.push(r);
                        }
                    }
                }
            }
            Err(ContextError::Timeout(msg)) => {
                log::warn!("Glean search timeout for '{}': {}", query, msg);
            }
            Err(e) => return Err(e),
        }
    }

    let all_results = prioritize_recent_results(all_results);

    // Emit Glean document signals for new/updated results
    for result in &all_results {
        let Some(ref snippet) = result.snippet else {
            continue;
        };
        if snippet.is_empty() {
            continue;
        }
        let url = result.url.as_deref().unwrap_or("");
        if url.is_empty() {
            continue;
        }

        let updated_at = result.updated_at.as_deref();
        let has_existing =
            match db.has_glean_signal_for_url(entity_type, entity_id, url, updated_at, 30) {
                Ok(v) => v,
                Err(e) => {
                    log::warn!(
                        "I487: failed dedupe check for {} {} url={}: {}",
                        entity_type,
                        entity_id,
                        url,
                        e
                    );
                    false
                }
            };
        if has_existing {
            continue;
        }

        let value = format!(
            "{}|{}|{}",
            result.title.as_deref().unwrap_or(""),
            result.doc_type.as_deref().unwrap_or(""),
            url,
        );
        let _ = crate::signals::bus::emit_signal(
            db,
            entity_type,
            entity_id,
            "glean_document",
            "glean_search",
            Some(&value),
            0.7,
        );
    }

    // Parse org health data from CRM-type documents (accounts only)
    let org_health = if entity_type == "account" {
        let account_name = db
            .get_account(entity_id)
            .ok()
            .flatten()
            .map(|a| a.name)
            .unwrap_or_default();
        parse_org_health_data(&all_results, &account_name)
    } else {
        None
    };

    // Build file_contents from search results (snippets)
    let mut file_parts: Vec<String> = Vec::new();
    let mut total_bytes = 0usize;
    let max_bytes = 10_000;

    for result in &all_results {
        let title = result.title.as_deref().unwrap_or("Untitled");
        let doc_type = result.doc_type.as_deref().unwrap_or("unknown");
        let snippet = result.snippet.as_deref().unwrap_or("");
        let updated = result.updated_at.as_deref().unwrap_or("unknown");

        let entry = format!(
            "--- {} [{}] ({}) ---\n{}",
            title, doc_type, updated, snippet
        );
        let entry_bytes = entry.len();

        if total_bytes + entry_bytes > max_bytes {
            break;
        }

        file_parts.push(entry);
        total_bytes += entry_bytes;
    }

    // People/org graph search (for stakeholders enrichment)
    let people_results = if entity_type == "account" || entity_type == "project" {
        let entity_name = match entity_type {
            "account" => db.get_account(entity_id).ok().flatten().map(|a| a.name),
            "project" => db.get_project(entity_id).ok().flatten().map(|p| p.name),
            _ => None,
        };

        if let Some(name) = entity_name {
            let cache_key = format!("org:{}", name);
            if let Some(cached) = cache.get_with_db(CacheKind::OrgGraph, &cache_key, db) {
                serde_json::from_str(&cached).unwrap_or_default()
            } else {
                match client.search_people(&name, 50).await {
                    Ok(results) => {
                        if let Ok(json) = serde_json::to_string(&results) {
                            cache.put(CacheKind::OrgGraph, &cache_key, &json, db);
                        }
                        results
                    }
                    Err(e) => {
                        log::warn!("Glean people search failed: {}", e);
                        Vec::new()
                    }
                }
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    Ok(GleanEntityData {
        file_contents: if file_parts.is_empty() {
            String::new()
        } else {
            file_parts.join("\n\n")
        },
        people: people_results,
        search_results: all_results,
        org_health,
    })
}

// ---------------------------------------------------------------------------
// Glean contact processing — DB writes from Glean people results
// ---------------------------------------------------------------------------

/// Map a Glean title to an account stakeholder role context.
fn map_glean_role(glean_title: &str) -> String {
    let lower = glean_title.to_lowercase();
    if lower.contains("primary") {
        "primary_contact".into()
    } else if lower.contains("support") {
        "support_contact".into()
    } else if lower.contains("developer")
        || lower.contains("engineer")
        || lower.contains("technical")
    {
        "technical".into()
    } else {
        "associated".into()
    }
}

/// Process Glean people results into DB records.
///
/// Two-pass approach:
/// - Pass 1: Create/update all people, link to account, detect internal team
/// - Pass 2: Extract manager relationships (needs all people created first)
///
/// Returns count of newly created contacts.
fn process_glean_contacts(
    db: &ActionDb,
    entity_id: &str,
    people: &[GleanPersonResult],
    user_domains: &[String],
) -> usize {
    use crate::db::people::ProfileUpdate;

    let mut new_count = 0;
    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
    // Map email → person_id for pass 2 manager lookups
    let mut email_to_person: Vec<(String, String)> = Vec::new();

    // --- Pass 1: Create/update people, link to account, internal team ---
    for contact in people {
        let email = match contact.email.as_deref() {
            Some(e) if !e.is_empty() => e,
            _ => continue,
        };
        let title_raw = contact.title.as_deref().unwrap_or("").trim();
        let mapped_role = map_glean_role(title_raw);
        let is_internal =
            crate::util::classify_relationship_multi(email, user_domains).as_str() == "internal";
        let role_for_link = if is_internal && !title_raw.is_empty() {
            title_raw.to_string()
        } else {
            mapped_role
        };

        // Look up existing person by email
        let person_id = match db.get_person_by_email_or_alias(email) {
            Ok(Some(existing)) => {
                // Update profile with Glean fields (respects source priority)
                let update = ProfileUpdate {
                    role: contact.title.clone(),
                    organization: contact.department.clone(),
                    company_hq: contact.location.clone(),
                    linkedin_url: None,
                    twitter_handle: None,
                    phone: None,
                    photo_url: None,
                    bio: None,
                    title_history: None,
                    company_industry: None,
                    company_size: None,
                };
                match db.update_person_profile(&existing.id, &update, "glean") {
                    Ok(result) => {
                        // Avoid false-positive profile_enriched signals when no fields were written.
                        if !result.fields_updated.is_empty() {
                            if let Err(e) = crate::services::signals::emit(
                                &ctx,
                                db,
                                "person",
                                &existing.id,
                                "profile_enriched",
                                "glean",
                                None,
                                0.7,
                            ) {
                                log::warn!(
                                    "I505: Failed emitting profile_enriched for {}: {}",
                                    existing.id,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("I505: Failed to update profile for {}: {}", existing.id, e);
                    }
                }

                existing.id
            }
            _ => {
                // New discovery — create minimal person
                let pid = format!("p-glean-{}", uuid::Uuid::new_v4());
                if let Err(e) = db.create_person_minimal(
                    &pid,
                    email,
                    contact.name.as_deref(),
                    contact.title.as_deref(),
                    contact.department.as_deref(),
                    contact.location.as_deref(),
                ) {
                    log::warn!("I505: Failed to create person for {}: {}", email, e);
                    continue;
                }

                // Emit glean_contact_discovered signal
                let _ = crate::services::signals::emit(
                    &ctx,
                    db,
                    "person",
                    &pid,
                    "glean_contact_discovered",
                    "glean",
                    Some(email),
                    0.8,
                );

                new_count += 1;
                pid
            }
        };

        // Link to account with source tracking
        if let Err(e) =
            db.link_person_to_account_with_source(entity_id, &person_id, &role_for_link, "glean")
        {
            log::warn!(
                "I505: Failed linking person {} to account {}: {}",
                person_id,
                entity_id,
                e
            );
        }

        email_to_person.push((email.to_string(), person_id));
    }

    // --- Pass 2: Manager relationships (all people exist in DB now) ---
    let has_managers = people
        .iter()
        .any(|c| c.manager.as_deref().is_some_and(|m| !m.trim().is_empty()));
    if has_managers {
        // Load people list once for name matching (not per-contact)
        let all_people = db.get_people(None).unwrap_or_default();

        for contact in people {
            let manager_name = match contact.manager.as_deref() {
                Some(m) if !m.trim().is_empty() => m.trim(),
                _ => continue,
            };

            // Find the person_id for this contact
            let email = match contact.email.as_deref() {
                Some(e) if !e.is_empty() => e,
                _ => continue,
            };
            let person_id = match email_to_person.iter().find(|(e, _)| e == email) {
                Some((_, pid)) => pid,
                None => continue,
            };

            // Search for exact name match (case-insensitive)
            let matches: Vec<&crate::db::types::DbPerson> = all_people
                .iter()
                .filter(|p| p.name.eq_ignore_ascii_case(manager_name))
                .collect();

            if matches.len() == 1 {
                let manager = &matches[0];
                let rel_id = format!("pr-glean-mgr-{}-{}", person_id, manager.id);
                if let Err(e) = db.upsert_person_relationship(
                    &crate::db::person_relationships::UpsertRelationship {
                        id: &rel_id,
                        from_person_id: person_id,
                        to_person_id: &manager.id,
                        relationship_type: "manager",
                        direction: "directed",
                        confidence: 0.8,
                        context_entity_id: Some(entity_id),
                        context_entity_type: Some("account"),
                        source: "glean",
                        rationale: Some(&format!(
                            "Manager relationship from Glean org graph: {} reports to {}",
                            contact.name.as_deref().unwrap_or(email),
                            manager_name
                        )),
                    },
                ) {
                    log::warn!(
                        "I505: Failed upserting manager relationship {} -> {}: {}",
                        person_id,
                        manager.id,
                        e
                    );
                }
            }
            // Zero or multiple matches: skip (no phantom people, no wrong guesses)
        }
    }

    new_count
}

/// Intermediate struct holding Glean-sourced data before merging into IntelligenceContext.
struct GleanEntityData {
    file_contents: String,
    people: Vec<GleanPersonResult>,
    search_results: Vec<GleanSearchResult>,
    /// Parsed org-level health data from CRM docs.
    org_health: Option<crate::intelligence::io::OrgHealthData>,
}

impl ContextProvider for GleanContextProvider {
    fn gather_entity_context(
        &self,
        db: &ActionDb,
        entity_id: &str,
        entity_type: &str,
        prior: Option<&IntelligenceJson>,
    ) -> Result<IntelligenceContext, ContextError> {
        // Phase A: Always-local data via the local fallback provider.
        // This gives us meetings, actions, captures, facts, user_context, etc.
        let mut ctx =
            self.local_fallback
                .gather_entity_context(db, entity_id, entity_type, prior)?;

        // Phase B: Glean-sourced data (network calls).
        // Use std::thread::spawn + oneshot to move network calls off Tokio threads
        // entirely, avoiding block_in_place deadlock risk under contention.
        let glean_data = {
            // Pre-extract DB-dependent search queries on the current thread
            let queries = self.entity_search_queries(db, entity_id, entity_type);
            let gap_q = ctx.gap_queries.clone();
            let endpoint = self.endpoint.clone();
            let cache = Arc::clone(&self.cache);
            let entity_id_owned = entity_id.to_string();
            let entity_type_owned = entity_type.to_string();

            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            std::thread::spawn(move || {
                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| {
                        ContextError::Other(format!("Failed to build Glean runtime: {}", e))
                    })
                    .and_then(|rt| {
                        rt.block_on(async {
                            // Open a fresh DB connection for this thread (ActionDb is !Send)
                            let thread_db = crate::db::ActionDb::open()
                                .map_err(|e| ContextError::Db(format!("Thread DB: {}", e)))?;
                            gather_glean_context_standalone(
                                &endpoint,
                                &cache,
                                &thread_db,
                                &entity_id_owned,
                                &entity_type_owned,
                                &queries,
                                &gap_q,
                            )
                            .await
                        })
                    });
                let _ = tx.send(result);
            });

            match rx.recv() {
                Ok(Ok(data)) => Some(data),
                Ok(Err(ContextError::Timeout(msg))) => {
                    log::warn!(
                        "Glean timeout for {} {}, using local-only context: {}",
                        entity_type,
                        entity_id,
                        msg
                    );
                    None
                }
                Ok(Err(ContextError::Auth(msg))) => {
                    log::warn!("Glean auth error: {}. Falling back to local.", msg);
                    None
                }
                Ok(Err(e)) => {
                    log::warn!("Glean context gather failed: {}. Falling back to local.", e);
                    None
                }
                Err(_) => {
                    log::error!("Glean context thread panicked. Falling back to local.");
                    None
                }
            }
        };

        // Merge Glean data into the context
        if let Some(glean) = glean_data {
            let has_recent_local_stakeholder_context = !ctx.meeting_history.is_empty()
                || ctx.verified_stakeholder_presence.is_some()
                || !ctx.stakeholders.trim().is_empty();

            // Merge Glean documents with local file contents (always additive)
            if !glean.file_contents.is_empty() {
                if ctx.file_contents.is_empty() {
                    ctx.file_contents = glean.file_contents;
                } else {
                    ctx.file_contents = format!(
                        "{}\n\n--- Glean Documents ---\n\n{}",
                        ctx.file_contents, glean.file_contents
                    );
                }
            }

            // Process Glean contacts into DB (person records, account links, relationships)
            if !glean.people.is_empty()
                && entity_type == "account"
                && !has_recent_local_stakeholder_context
            {
                let user_domains = crate::state::load_config()
                    .map(|config| config.resolved_user_domains())
                    .unwrap_or_default();
                let new_count = process_glean_contacts(db, entity_id, &glean.people, &user_domains);
                if new_count > 0 {
                    log::info!(
                        "I505: Created {} new contacts from Glean for {} {}",
                        new_count,
                        entity_type,
                        entity_id
                    );
                }
            }

            // Enrich stakeholders with Glean org graph data
            if !glean.people.is_empty() && !has_recent_local_stakeholder_context {
                let glean_people_lines: Vec<String> = glean
                    .people
                    .iter()
                    .filter_map(|p| {
                        let name = p.name.as_deref()?;
                        let title = p.title.as_deref().unwrap_or("unknown role");
                        let dept = p.department.as_deref().unwrap_or("");
                        Some(format!("- {} | {} | {} [Glean]", name, title, dept))
                    })
                    .collect();

                if !glean_people_lines.is_empty() {
                    let glean_section = format!(
                        "\nGlean Org Graph ({} people):\n{}",
                        glean_people_lines.len(),
                        glean_people_lines.join("\n")
                    );
                    ctx.stakeholders.push_str(&glean_section);
                }
            }

            // Store org health in DB and make available on context
            if let Some(ref org_health) = glean.org_health {
                if let Ok(json) = serde_json::to_string(org_health) {
                    // Write to entity_assessment.org_health column
                    let _ = db.conn_ref().execute(
                        "INSERT INTO entity_assessment (entity_id, entity_type, org_health_json, updated_at)
                         VALUES (?1, ?2, ?3, datetime('now'))
                         ON CONFLICT(entity_id) DO UPDATE SET
                             org_health_json = excluded.org_health_json,
                             updated_at = datetime('now')",
                        rusqlite::params![entity_id, entity_type, json],
                    );
                    log::info!(
                        "I500: Stored org health for {} (band={:?}, source={})",
                        entity_id,
                        org_health.health_band,
                        org_health.source,
                    );
                }
                // Make org_health available on context for health scoring
                ctx.org_health = Some(org_health.clone());

                // Emit conflict signal when Glean CRM values differ from
                // current account fields. build_account_field_conflicts() in
                // services/accounts reads these signals to surface conflicts.
                if entity_type == "account" {
                    if let Ok(Some(account)) = db.get_account(entity_id) {
                        let source = if org_health.source.is_empty() {
                            "glean_crm"
                        } else {
                            &org_health.source
                        };
                        if let Some(stage) = org_health.customer_stage.as_deref() {
                            let current = account.lifecycle.as_deref().unwrap_or("");
                            let normalized = crate::services::accounts::normalized_lifecycle(stage);
                            if !current.is_empty()
                                && crate::services::accounts::normalized_lifecycle(current)
                                    != normalized
                            {
                                let payload = serde_json::json!({
                                    "lifecycle": normalized,
                                });
                                let _ = crate::signals::bus::emit_signal(
                                    db,
                                    "account",
                                    entity_id,
                                    "glean_field_suggestion",
                                    source,
                                    Some(&payload.to_string()),
                                    0.7,
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(ctx)
    }

    fn provider_name(&self) -> &str {
        "glean"
    }

    fn is_remote(&self) -> bool {
        true
    }

    fn remote_endpoint(&self) -> Option<&str> {
        Some(&self.endpoint)
    }
}

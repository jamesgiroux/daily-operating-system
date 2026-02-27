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

use serde::{Deserialize, Serialize};

use crate::context_provider::cache::{CacheKind, GleanCache};
use crate::context_provider::{ContextError, ContextProvider};
use crate::db::ActionDb;
use crate::intelligence::prompts::IntelligenceContext;
use crate::intelligence::IntelligenceJson;

/// Timeout for individual Glean API calls.
const GLEAN_CALL_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum documents to fetch per entity context gather.
const MAX_DOCUMENTS_PER_ENTITY: usize = 10;

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
    /// Create a new client with the given MCP endpoint and OAuth token.
    pub fn new(endpoint: &str, oauth_token: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(GLEAN_CALL_TIMEOUT)
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", oauth_token))
                        .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("")),
                );
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

    /// Search Glean for documents related to a query.
    pub async fn search(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<GleanSearchResult>, ContextError> {
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
    /// Keychain key for the OAuth token.
    keychain_key: String,
    /// Whether to suppress local connectors (Governed) or merge (Additive).
    strategy: super::GleanStrategy,
    /// In-memory + DB cache for Glean responses.
    cache: Arc<GleanCache>,
    /// Fallback: local provider for always-local data and Glean outages.
    local_fallback: super::local::LocalContextProvider,
}

impl GleanContextProvider {
    pub fn new(
        endpoint: String,
        keychain_key: String,
        strategy: super::GleanStrategy,
        cache: Arc<GleanCache>,
        local_fallback: super::local::LocalContextProvider,
    ) -> Self {
        Self {
            endpoint,
            keychain_key,
            strategy,
            cache,
            local_fallback,
        }
    }

    /// Resolve the OAuth token from the macOS Keychain.
    fn resolve_token(&self) -> Result<String, ContextError> {
        let output = std::process::Command::new("security")
            .args(["find-generic-password", "-s", &self.keychain_key, "-w"])
            .output()
            .map_err(|e| ContextError::Auth(format!("Keychain access failed: {}", e)))?;

        if !output.status.success() {
            return Err(ContextError::Auth(format!(
                "Glean OAuth token not found in Keychain (key: {})",
                self.keychain_key
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
    async fn gather_glean_context(
        &self,
        db: &ActionDb,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<GleanEntityData, ContextError> {
        let token = self.resolve_token()?;
        let client = GleanMcpClient::new(&self.endpoint, &token);

        let queries = self.entity_search_queries(db, entity_id, entity_type);
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
                    match client.search_people(&name, 20).await {
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
        })
    }
}

/// Intermediate struct holding Glean-sourced data before merging into IntelligenceContext.
struct GleanEntityData {
    file_contents: String,
    people: Vec<GleanPersonResult>,
    search_results: Vec<GleanSearchResult>,
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
        // We use tokio::runtime::Handle to run async code from this sync trait method.
        // SAFETY: block_in_place requires a multi-threaded tokio runtime and must NOT
        // be called from inside spawn_blocking. All current callers (intel_queue,
        // report generators) run on async tasks, so this is safe.
        let glean_data = match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                // We're inside a tokio runtime — use block_in_place to avoid deadlock
                match tokio::task::block_in_place(|| {
                    handle.block_on(self.gather_glean_context(db, entity_id, entity_type))
                }) {
                    Ok(data) => Some(data),
                    Err(ContextError::Timeout(msg)) => {
                        log::warn!(
                            "Glean timeout for {} {}, using local-only context: {}",
                            entity_type,
                            entity_id,
                            msg
                        );
                        None
                    }
                    Err(ContextError::Auth(msg)) => {
                        log::warn!("Glean auth error: {}. Falling back to local.", msg);
                        None
                    }
                    Err(e) => {
                        log::warn!("Glean context gather failed: {}. Falling back to local.", e);
                        None
                    }
                }
            }
            Err(_) => {
                log::warn!("No tokio runtime available for Glean calls. Using local-only context.");
                None
            }
        };

        // Merge Glean data into the context
        if let Some(glean) = glean_data {
            // Replace file_contents with Glean documents (Additive: merge, Governed: replace)
            match self.strategy {
                super::GleanStrategy::Governed => {
                    if !glean.file_contents.is_empty() {
                        ctx.file_contents = glean.file_contents;
                    }
                }
                super::GleanStrategy::Additive => {
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
                }
            }

            // Enrich stakeholders with Glean org graph data
            if !glean.people.is_empty() {
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
        }

        Ok(ctx)
    }

    fn provider_name(&self) -> &str {
        "glean"
    }

    fn is_remote(&self) -> bool {
        true
    }
}

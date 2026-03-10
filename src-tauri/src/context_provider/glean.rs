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
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| {
                handle.block_on(crate::glean::get_valid_access_token())
            })
            .map_err(|e| ContextError::Auth(format!("Glean token error: {}", e))),
            Err(_) => match crate::glean::token_store::load_token() {
                Ok(token) => Ok(token.access_token),
                Err(e) => Err(ContextError::Auth(format!("Glean token not found: {}", e))),
            },
        }
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
}

// ---------------------------------------------------------------------------
// I500: Org Health Data Parsing
// ---------------------------------------------------------------------------

/// I500: Parse org-level health data from Glean search results.
///
/// Looks for health signals in Salesforce, Zendesk, and other CRM-type documents.
/// Priority: salesforce_account > zendesk_organization > other doc types.
fn parse_org_health_data(
    results: &[GleanSearchResult],
    _account_name: &str,
) -> Option<crate::intelligence::io::OrgHealthData> {
    // Sort results by doc_type priority
    let mut prioritized: Vec<&GleanSearchResult> = results
        .iter()
        .filter(|r| r.snippet.is_some())
        .collect();

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
        strategy: super::GleanStrategy,
        cache: Arc<GleanCache>,
        local_fallback: super::local::LocalContextProvider,
    ) -> Self {
        Self {
            endpoint,
            strategy,
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
    /// `gap_queries`: I508c dimension-aware gap queries for fan-out search.
    async fn gather_glean_context(
        &self,
        db: &ActionDb,
        entity_id: &str,
        entity_type: &str,
        gap_queries: &[String],
    ) -> Result<GleanEntityData, ContextError> {
        let client = GleanMcpClient::new(&self.endpoint);

        let mut queries = self.entity_search_queries(db, entity_id, entity_type);
        // I508c: Append gap queries for dimension-aware Glean fan-out (cap at 3)
        for q in gap_queries.iter().take(3) {
            queries.push(q.clone());
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

        // I487: Emit Glean document signals for new/updated results
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
            if db
                .has_glean_signal_for_url(entity_id, url, updated_at, 30)
                .unwrap_or(true)
            {
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

        // I500: Parse org health data from CRM-type documents (accounts only)
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
            org_health,
        })
    }
}

/// Intermediate struct holding Glean-sourced data before merging into IntelligenceContext.
struct GleanEntityData {
    file_contents: String,
    people: Vec<GleanPersonResult>,
    search_results: Vec<GleanSearchResult>,
    /// I500: Parsed org-level health data from CRM docs.
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
        // We use tokio::runtime::Handle to run async code from this sync trait method.
        // SAFETY: block_in_place requires a multi-threaded tokio runtime and must NOT
        // be called from inside spawn_blocking. All current callers (intel_queue,
        // report generators) run on async tasks, so this is safe.
        let glean_data = match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                // We're inside a tokio runtime — use block_in_place to avoid deadlock
                let gap_q = ctx.gap_queries.clone();
                match tokio::task::block_in_place(|| {
                    handle.block_on(self.gather_glean_context(db, entity_id, entity_type, &gap_q))
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

            // I500: Store org health in DB and make available on context
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
                // Make org_health available on context for I499 health scoring
                ctx.org_health = Some(org_health.clone());
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

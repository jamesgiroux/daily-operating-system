//! DailyOS MCP Server — exposes workspace intelligence to Claude Desktop.
//!
//! Standalone binary that communicates over stdio using the Model Context Protocol.
//! Opens the SQLite database read-only so it can run safely alongside the Tauri app.
//!
//! Build: `cargo build --features mcp --bin dailyos-mcp`
//! Usage: spawned by Claude Desktop as configured in claude_desktop_config.json.

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;

use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::*;
use rmcp::schemars::JsonSchema;
use rmcp::service::RequestContext;
use rmcp::{tool, Error as McpError, RoleServer, ServerHandler, ServiceExt};
use serde::{Deserialize, Serialize};

use dailyos_lib::abilities::{AbilityDescriptor, AbilityRegistry};
use dailyos_lib::bridges::mcp::McpAbilityBridge;
use dailyos_lib::bridges::{BridgeSurfaceError, McpSessionId};
use dailyos_lib::db::ActionDb;
use dailyos_lib::embeddings::EmbeddingModel;
use dailyos_lib::state::load_config;
use dailyos_lib::types::Config;

// =============================================================================
// Server State
// =============================================================================

/// Read-only MCP server for DailyOS workspace intelligence.
#[derive(Clone)]
struct DailyOsMcp {
    /// Read-only database connection. Wrapped in Arc<Mutex> because rusqlite::Connection
    /// is not Send+Sync, and MCP tool calls are sequential over stdio anyway.
    db: Arc<Mutex<ActionDb>>,
    config: Config,
    /// Embedding model for semantic search (nomic-embed-text-v1.5).
    embedding_model: Arc<EmbeddingModel>,
    /// Registry-backed ability bridge for Phase 2 hybrid MCP tools.
    ability_bridge: Arc<McpAbilityBridge<'static>>,
    /// Process-scoped MCP session id. Stdio transport lifetime is process lifetime.
    mcp_session_id: McpSessionId,
}

// =============================================================================
// Tool Parameter Types
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
struct GetBriefingParams {
    /// Optional date in YYYY-MM-DD format. Defaults to today.
    #[schemars(description = "Date for the briefing (YYYY-MM-DD). Defaults to today.")]
    date: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct QueryEntityParams {
    /// Name or ID of the entity to query.
    #[schemars(description = "Entity name or ID to look up")]
    query: String,
    /// Entity type filter: "account", "project", "person", or "all" (default).
    #[schemars(description = "Filter: account, project, person, or all")]
    entity_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListEntitiesParams {
    /// Filter by entity type: "account", "project", "person", or "all" (default).
    #[schemars(description = "Filter: account, project, person, or all")]
    entity_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchMeetingsParams {
    /// Search query for meeting titles, summaries, and prep context.
    #[schemars(description = "Search query text")]
    query: String,
    /// Maximum number of results (default 20, max 50).
    #[schemars(description = "Max results (default 20, max 50)")]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchContentParams {
    /// Entity name or ID to search within.
    #[schemars(description = "Entity name or ID to search within")]
    entity_id: String,
    /// Natural language search query.
    #[schemars(description = "What to search for in workspace files")]
    query: String,
    /// Maximum number of results (default 10, max 30).
    #[schemars(description = "Max results (default 10, max 30)")]
    top_k: Option<usize>,
}

// =============================================================================
// Response Types
// =============================================================================

#[derive(Serialize)]
struct BriefingResponse {
    date: String,
    schedule: Option<serde_json::Value>,
    actions: Option<serde_json::Value>,
    emails: Option<serde_json::Value>,
    briefing: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct EntityResult {
    id: String,
    name: String,
    entity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    health: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lifecycle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    intelligence_summary: Option<String>,
    open_actions: Vec<ActionSummary>,
    upcoming_meetings: Vec<MeetingSummary>,
}

#[derive(Serialize)]
struct ActionSummary {
    id: String,
    title: String,
    priority: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    due_date: Option<String>,
}

#[derive(Serialize)]
struct MeetingSummary {
    id: String,
    title: String,
    start_time: String,
    meeting_type: String,
}

#[derive(Serialize)]
struct EntityListItem {
    id: String,
    name: String,
    entity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    health: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

#[derive(Serialize)]
struct MeetingSearchItem {
    id: String,
    title: String,
    meeting_type: String,
    start_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
}

// =============================================================================
// Tool implementations
// =============================================================================

#[tool(tool_box)]
impl DailyOsMcp {
    fn new(
        db: ActionDb,
        config: Config,
        embedding_model: Arc<EmbeddingModel>,
        ability_bridge: Arc<McpAbilityBridge<'static>>,
        mcp_session_id: McpSessionId,
    ) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            config,
            embedding_model,
            ability_bridge,
            mcp_session_id,
        }
    }

    #[tool(
        description = "Get the daily briefing for DailyOS. Returns today's schedule, actions, emails, and AI-generated narrative briefing. Use this when the user asks about their day, schedule, meetings, or what they need to focus on."
    )]
    fn get_briefing(&self, #[tool(aggr)] params: GetBriefingParams) -> String {
        let today_dir = PathBuf::from(&self.config.workspace_path).join("_today");

        let date = params
            .date
            .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());

        let response = BriefingResponse {
            date,
            schedule: read_json_file(&today_dir.join("data/schedule.json")),
            actions: read_json_file(&today_dir.join("data/actions.json")),
            emails: read_json_file(&today_dir.join("data/emails.json")),
            briefing: read_json_file(&today_dir.join("data/briefing.json")),
        };

        serde_json::to_string_pretty(&response).unwrap_or_else(|e| format!("Error: {e}"))
    }

    #[tool(
        description = "Look up a specific account, project, or person in the DailyOS workspace. Returns entity details, intelligence summary, open actions, and upcoming meetings. Use this when the user asks about a specific customer, project, or contact."
    )]
    fn query_entity(&self, #[tool(aggr)] params: QueryEntityParams) -> String {
        let db = self.db.lock();
        let query_lower = params.query.to_lowercase();
        let entity_type = params.entity_type.as_deref().unwrap_or("all");

        let mut result: Option<EntityResult> = None;

        // Search accounts
        if entity_type == "all" || entity_type == "account" {
            if let Ok(accounts) = db.get_all_accounts() {
                for acct in &accounts {
                    if acct.id == params.query || acct.name.to_lowercase().contains(&query_lower) {
                        let actions = db.get_account_actions(&acct.id).unwrap_or_default();
                        let meetings = db
                            .get_upcoming_meetings_for_account(&acct.id, 5)
                            .unwrap_or_default();
                        let intel = db
                            .get_entity_intelligence(&acct.id)
                            .ok()
                            .flatten()
                            .and_then(|i| i.executive_assessment);

                        result = Some(build_entity_result(
                            &acct.id,
                            &acct.name,
                            "account",
                            acct.health.as_deref(),
                            None,
                            acct.lifecycle.as_deref(),
                            intel.as_deref(),
                            &actions,
                            &meetings,
                        ));
                        break;
                    }
                }
            }
        }

        // Search projects
        if result.is_none() && (entity_type == "all" || entity_type == "project") {
            if let Ok(projects) = db.get_all_projects() {
                for proj in &projects {
                    if proj.id == params.query || proj.name.to_lowercase().contains(&query_lower) {
                        let actions = db.get_project_actions(&proj.id).unwrap_or_default();
                        let meetings = db.get_meetings_for_project(&proj.id, 5).unwrap_or_default();
                        let intel = db
                            .get_entity_intelligence(&proj.id)
                            .ok()
                            .flatten()
                            .and_then(|i| i.executive_assessment);

                        result = Some(build_entity_result(
                            &proj.id,
                            &proj.name,
                            "project",
                            None,
                            Some(&proj.status),
                            None,
                            intel.as_deref(),
                            &actions,
                            &meetings,
                        ));
                        break;
                    }
                }
            }
        }

        // Search people
        if result.is_none() && (entity_type == "all" || entity_type == "person") {
            if let Ok(people) = db.get_people(None) {
                for person in &people {
                    if person.id == params.query
                        || person.name.to_lowercase().contains(&query_lower)
                        || person.email.to_lowercase().contains(&query_lower)
                    {
                        let intel = db
                            .get_entity_intelligence(&person.id)
                            .ok()
                            .flatten()
                            .and_then(|i| i.executive_assessment);
                        result = Some(EntityResult {
                            id: person.id.clone(),
                            name: person.name.clone(),
                            entity_type: "person".to_string(),
                            health: None,
                            status: Some(person.relationship.clone()),
                            lifecycle: None,
                            intelligence_summary: intel,
                            open_actions: Vec::new(),
                            upcoming_meetings: Vec::new(),
                        });
                        break;
                    }
                }
            }
        }

        match result {
            Some(entity) => serde_json::to_string_pretty(&entity)
                .unwrap_or_else(|e| format!("Error: {e}")),
            None => format!(
                "No entity found matching '{}'. Try a different name or check the entity type filter.",
                params.query
            ),
        }
    }

    #[tool(
        description = "List all accounts, projects, or people in the DailyOS workspace. Use this when the user asks to see their portfolio, customer list, project list, or contacts."
    )]
    fn list_entities(&self, #[tool(aggr)] params: ListEntitiesParams) -> String {
        let db = self.db.lock();
        let entity_type = params.entity_type.as_deref().unwrap_or("all");
        let mut items: Vec<EntityListItem> = Vec::new();

        if entity_type == "all" || entity_type == "account" {
            if let Ok(accounts) = db.get_all_accounts() {
                items.extend(accounts.into_iter().map(|a| EntityListItem {
                    id: a.id,
                    name: a.name,
                    entity_type: "account".to_string(),
                    health: a.health,
                    status: a.lifecycle,
                }));
            }
        }

        if entity_type == "all" || entity_type == "project" {
            if let Ok(projects) = db.get_all_projects() {
                items.extend(projects.into_iter().map(|p| EntityListItem {
                    id: p.id,
                    name: p.name,
                    entity_type: "project".to_string(),
                    health: None,
                    status: Some(p.status),
                }));
            }
        }

        if entity_type == "all" || entity_type == "person" {
            if let Ok(people) = db.get_people(None) {
                items.extend(people.into_iter().map(|p| EntityListItem {
                    id: p.id,
                    name: p.name,
                    entity_type: "person".to_string(),
                    health: None,
                    status: Some(p.relationship),
                }));
            }
        }

        serde_json::to_string_pretty(&items).unwrap_or_else(|e| format!("Error: {e}"))
    }

    #[tool(
        description = "Search past meetings in DailyOS by title, summary, or prep content. Use this when the user asks about past meetings, what was discussed, or wants to find a specific meeting."
    )]
    fn search_meetings(&self, #[tool(aggr)] params: SearchMeetingsParams) -> String {
        if params.query.trim().is_empty() {
            return "[]".to_string();
        }

        let db = self.db.lock();
        let pattern = format!("%{}%", params.query.trim());
        let limit = params.limit.unwrap_or(20).min(50) as i64;

        let mut stmt = match db.conn_ref().prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time,
                    (SELECT me.entity_id FROM meeting_entities me
                     WHERE me.meeting_id = m.id AND me.entity_type = 'account' LIMIT 1) AS account_id,
                    mt.summary, mp.prep_context_json
             FROM meetings m
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             LEFT JOIN meeting_prep mp ON mp.meeting_id = m.id
             WHERE m.title LIKE ?1
                OR mt.summary LIKE ?1
                OR mp.prep_context_json LIKE ?1
             ORDER BY m.start_time DESC
             LIMIT ?2",
        ) {
            Ok(s) => s,
            Err(e) => return format!("Error: {e}"),
        };

        let rows = match stmt.query_map(rusqlite::params![&pattern, limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        }) {
            Ok(r) => r,
            Err(e) => return format!("Error: {e}"),
        };

        let mut results = Vec::new();
        for row in rows {
            let Ok((id, title, meeting_type, start_time, account_id, summary, prep_json)) = row
            else {
                continue;
            };

            let match_snippet = summary.or_else(|| {
                prep_json.and_then(|json| {
                    serde_json::from_str::<serde_json::Value>(&json)
                        .ok()
                        .and_then(|v| {
                            v.get("intelligenceSummary")
                                .and_then(|s| s.as_str().map(|s| s.to_string()))
                        })
                })
            });

            let account_name = account_id
                .as_ref()
                .and_then(|aid| db.get_account(aid).ok().flatten())
                .map(|a| a.name);

            results.push(MeetingSearchItem {
                id,
                title,
                meeting_type,
                start_time,
                account_name,
                summary: match_snippet,
            });
        }

        serde_json::to_string_pretty(&results).unwrap_or_else(|e| format!("Error: {e}"))
    }

    #[tool(
        description = "Semantic search over workspace files for an entity. Returns the most relevant text passages from documents, transcripts, and notes. Use when the user asks about specific details, information, or topics within their files for a particular account, project, or person."
    )]
    fn search_content(&self, #[tool(aggr)] params: SearchContentParams) -> String {
        if params.query.trim().is_empty() {
            return "[]".to_string();
        }

        let db = self.db.lock();

        // Resolve entity_id: try exact match first, then fuzzy match on name
        let resolved_id = resolve_entity_id(&db, &params.entity_id);
        let entity_id = resolved_id.as_deref().unwrap_or(&params.entity_id);

        let top_k = params.top_k.unwrap_or(10).min(30);
        let model_ref = if self.embedding_model.is_ready() {
            Some(self.embedding_model.as_ref())
        } else {
            None
        };

        match dailyos_lib::queries::search::search_entity_content(
            &db,
            model_ref,
            entity_id,
            &params.query,
            top_k,
            0.7,
            0.3,
        ) {
            Ok(matches) if matches.is_empty() => {
                format!(
                    "No content found for entity '{}' matching '{}'.",
                    params.entity_id, params.query
                )
            }
            Ok(matches) => {
                let mut output = String::new();
                for (i, m) in matches.iter().enumerate() {
                    output.push_str(&format!(
                        "## Result {} — {} ({})\n**File:** {}\n**Score:** {:.2} (vector: {:.2}, text: {:.2})\n\n{}\n\n---\n\n",
                        i + 1,
                        m.filename,
                        m.content_type,
                        m.relative_path,
                        m.combined_score,
                        m.vector_score,
                        m.text_score,
                        m.chunk_text.chars().take(1000).collect::<String>(),
                    ));
                }
                output
            }
            Err(e) => format!("Search error: {e}"),
        }
    }
}

// =============================================================================
// ServerHandler — manually routes static tools plus registry-backed abilities
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpToolRoute {
    Static,
    Ability,
}

pub fn mcp_route_for_tool_name(name: &str) -> McpToolRoute {
    if DailyOsMcp::tool_box().map.contains_key(name) {
        McpToolRoute::Static
    } else {
        McpToolRoute::Ability
    }
}

pub fn list_hybrid_tools_for_bridge(ability_bridge: &McpAbilityBridge<'_>) -> Vec<Tool> {
    let mut tools = DailyOsMcp::tool_box().list();
    tools.extend(
        ability_bridge
            .list_descriptors()
            .iter()
            .map(|descriptor| ability_descriptor_to_tool(descriptor)),
    );
    tools
}

fn ability_descriptor_to_tool(descriptor: &AbilityDescriptor) -> Tool {
    let input_schema = match (descriptor.input_schema)() {
        serde_json::Value::Object(object) => object,
        _ => JsonObject::new(),
    };

    Tool::new(
        descriptor.name,
        format!(
            "DailyOS {:?} ability `{}`.",
            descriptor.category, descriptor.name
        ),
        input_schema,
    )
}

pub async fn invoke_mcp_ability_tool(
    ability_bridge: &McpAbilityBridge<'_>,
    session_id: McpSessionId,
    request: CallToolRequestParam,
) -> Result<CallToolResult, McpError> {
    let ability_name = request.name.to_string();
    let input_json = serde_json::Value::Object(request.arguments.unwrap_or_default());
    let response = ability_bridge
        .invoke_ability(session_id, &ability_name, input_json, false, None)
        .await
        .map_err(mcp_error_from_bridge_surface_error)?;
    let content = Content::json(response)?;

    Ok(CallToolResult::success(vec![content]))
}

pub fn mcp_error_from_bridge_surface_error(error: BridgeSurfaceError) -> McpError {
    let data = serde_json::to_value(error)
        .unwrap_or_else(|_| serde_json::Value::String("ability_unavailable".to_string()));
    McpError::invalid_params(error.to_string(), Some(data))
}

impl ServerHandler for DailyOsMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "dailyos".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            instructions: Some(
                "DailyOS MCP server. Provides read-only access to your daily briefing, \
                 accounts, projects, people, meeting history, and workspace file contents. \
                 Use get_briefing for today's schedule, query_entity for entity details, \
                 list_entities for portfolio overview, search_meetings for meeting history, \
                 and search_content for semantic search over workspace files."
                    .to_string(),
            ),
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult {
            next_cursor: None,
            tools: list_hybrid_tools_for_bridge(&self.ability_bridge),
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        match mcp_route_for_tool_name(&request.name) {
            McpToolRoute::Static => {
                let context = ToolCallContext::new(self, request, context);
                Self::tool_box().call(context).await
            }
            McpToolRoute::Ability => {
                invoke_mcp_ability_tool(&self.ability_bridge, self.mcp_session_id, request).await
            }
        }
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Resolve a user-provided entity identifier (name or ID) to an entity ID.
/// Tries exact ID match first, then fuzzy name match across accounts, projects, people.
fn resolve_entity_id(db: &ActionDb, query: &str) -> Option<String> {
    let query_lower = query.to_lowercase();

    // Check accounts
    if let Ok(accounts) = db.get_all_accounts() {
        for acct in &accounts {
            if acct.id == query {
                return Some(acct.id.clone());
            }
            if acct.name.to_lowercase().contains(&query_lower) {
                return Some(acct.id.clone());
            }
        }
    }

    // Check projects
    if let Ok(projects) = db.get_all_projects() {
        for proj in &projects {
            if proj.id == query {
                return Some(proj.id.clone());
            }
            if proj.name.to_lowercase().contains(&query_lower) {
                return Some(proj.id.clone());
            }
        }
    }

    // Check people
    if let Ok(people) = db.get_people(None) {
        for person in &people {
            if person.id == query {
                return Some(person.id.clone());
            }
            if person.name.to_lowercase().contains(&query_lower)
                || person.email.to_lowercase().contains(&query_lower)
            {
                return Some(person.id.clone());
            }
        }
    }

    None
}

fn read_json_file(path: &std::path::Path) -> Option<serde_json::Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

#[allow(clippy::too_many_arguments)]
fn build_entity_result(
    id: &str,
    name: &str,
    entity_type: &str,
    health: Option<&str>,
    status: Option<&str>,
    lifecycle: Option<&str>,
    intelligence_summary: Option<&str>,
    actions: &[dailyos_lib::db::DbAction],
    meetings: &[dailyos_lib::db::DbMeeting],
) -> EntityResult {
    EntityResult {
        id: id.to_string(),
        name: name.to_string(),
        entity_type: entity_type.to_string(),
        health: health.map(str::to_string),
        status: status.map(str::to_string),
        lifecycle: lifecycle.map(str::to_string),
        intelligence_summary: intelligence_summary.map(str::to_string),
        open_actions: actions
            .iter()
            .filter(|a| matches!(a.status.as_str(), "backlog" | "unstarted" | "started"))
            .take(10)
            .map(|a| ActionSummary {
                id: a.id.clone(),
                title: a.title.clone(),
                priority: a.priority.to_string(),
                due_date: a.due_date.clone(),
            })
            .collect(),
        upcoming_meetings: meetings
            .iter()
            .take(5)
            .map(|m| MeetingSummary {
                id: m.id.clone(),
                title: m.title.clone(),
                start_time: m.start_time.clone(),
                meeting_type: m.meeting_type.clone(),
            })
            .collect(),
    }
}

// =============================================================================
// Main
// =============================================================================

/// Temporarily redirect stdout (fd 1) to stderr for the duration of `f`.
///
/// The MCP server communicates over stdio. Any writes to stdout before rmcp
/// takes over the channel corrupt the JSON-RPC stream. Native libraries
/// (ONNX Runtime, fastembed) may write to stdout during initialisation, so
/// we redirect stdout → stderr for that window only.
fn with_stdout_suppressed<F: FnOnce() -> R, R>(f: F) -> R {
    unsafe {
        let saved = libc::dup(libc::STDOUT_FILENO);
        libc::dup2(libc::STDERR_FILENO, libc::STDOUT_FILENO);
        let result = f();
        libc::dup2(saved, libc::STDOUT_FILENO);
        libc::close(saved);
        result
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config =
        load_config().map_err(|e| anyhow::anyhow!("Failed to load DailyOS config: {e}"))?;

    let db =
        ActionDb::open_readonly().map_err(|e| anyhow::anyhow!("Failed to open database: {e}"))?;

    // Initialize embedding model synchronously — MCP server starts before any
    // tool calls so this won't block user interaction.
    // stdout is redirected to stderr during init to prevent native library
    // output (ONNX Runtime, fastembed) from corrupting the MCP JSON-RPC stream.
    let embedding_model = Arc::new(EmbeddingModel::new());
    with_stdout_suppressed(|| {
        let models_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".dailyos")
            .join("models");
        if let Err(e) = embedding_model.initialize(models_dir) {
            eprintln!("Embedding model unavailable: {e}");
        }
    });

    let ability_registry = match AbilityRegistry::global_checked() {
        Ok(registry) => registry,
        Err(violations) => {
            eprintln!(
                "Ability registry unavailable for MCP dynamic tools; serving static tools only: {violations:?}"
            );
            Box::leak(Box::new(
                AbilityRegistry::from_descriptors_checked(Vec::new())
                    .expect("empty ability registry should be valid"),
            ))
        }
    };
    let ability_bridge = Arc::new(McpAbilityBridge::new(ability_registry));
    let mcp_session_id = McpSessionId::new_process_scoped();
    let server = DailyOsMcp::new(db, config, embedding_model, ability_bridge, mcp_session_id);

    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;

    use serde_json::json;

    use super::*;
    use dailyos_lib::abilities::registry::{AbilityPolicy, SignalPolicy};
    use dailyos_lib::abilities::{
        AbilityCategory, AbilityContext, AbilityError, AbilityRegistry, Actor,
    };
    use dailyos_lib::services::context::ExecutionMode;

    const AGENT_ACTORS: &[Actor] = &[Actor::Agent];
    const USER_ACTORS: &[Actor] = &[Actor::User];
    const LIVE_MODES: &[ExecutionMode] = &[ExecutionMode::Live];

    type ErasedFuture<'a> =
        Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

    fn success_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            Ok(json!({
                "data": {
                    "input": input,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str()
                },
                "ability_version": { "major": 1, "minor": 0 },
                "diagnostics": { "warnings": [] },
                "provenance": {
                    "invocation_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                    "ability_name": "fixture",
                    "ability_version": { "major": 1, "minor": 0 },
                    "ability_schema_version": 1,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str(),
                    "warnings": []
                }
            }))
        })
    }

    fn descriptor(
        name: &'static str,
        category: AbilityCategory,
        actors: &'static [Actor],
        modes: &'static [ExecutionMode],
    ) -> AbilityDescriptor {
        AbilityDescriptor {
            name,
            version: "1.0.0",
            schema_version: 1,
            category,
            policy: AbilityPolicy {
                allowed_actors: actors,
                allowed_modes: modes,
                requires_confirmation: false,
                may_publish: false,
            },
            composes: &[],
            mutates: &[],
            experimental: false,
            registered_at: None,
            signal_policy: SignalPolicy::default(),
            invoke_erased: success_erased,
            input_schema: closed_object_schema,
            output_schema: closed_object_schema,
        }
    }

    fn closed_object_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false
        })
    }

    fn registry_with(descriptors: Vec<AbilityDescriptor>) -> AbilityRegistry {
        AbilityRegistry::from_descriptors_checked(descriptors).unwrap()
    }

    fn session(index: u128) -> McpSessionId {
        McpSessionId::from_uuid(uuid::Uuid::from_u128(index))
    }

    fn request(name: &'static str, arguments: serde_json::Value) -> CallToolRequestParam {
        let arguments = match arguments {
            serde_json::Value::Object(object) => Some(object),
            _ => None,
        };
        CallToolRequestParam {
            name: name.into(),
            arguments,
        }
    }

    #[test]
    fn mcp_serverhandler_tool_box_macro_removed_and_manual_routes_static_or_ability() {
        let source = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/mcp/main.rs"
        ))
        .unwrap();
        let inherent_tool_box = concat!("#[tool", "(tool_box)]\nimpl DailyOsMcp");
        let serverhandler_tool_box = concat!(
            "#[tool",
            "(tool_box)]\nimpl ServerHandler for DailyOsMcp"
        );

        assert!(source.contains(inherent_tool_box));
        assert!(!source.contains(serverhandler_tool_box));
        assert!(source.contains("impl ServerHandler for DailyOsMcp"));
        assert!(source.contains("McpToolRoute::Static"));
        assert!(source.contains("invoke_mcp_ability_tool"));
    }

    #[test]
    fn mcp_list_tools_includes_inherent_static_tools_and_ability_descriptors_filtered_by_agent_actor(
    ) {
        let registry = registry_with(vec![
            descriptor(
                "agent_fixture_ability",
                AbilityCategory::Read,
                AGENT_ACTORS,
                LIVE_MODES,
            ),
            descriptor(
                "user_fixture_ability",
                AbilityCategory::Read,
                USER_ACTORS,
                LIVE_MODES,
            ),
        ]);
        let bridge = McpAbilityBridge::new(&registry);
        let tools = list_hybrid_tools_for_bridge(&bridge);
        let names = tools
            .iter()
            .map(|tool| tool.name.as_ref())
            .collect::<Vec<_>>();

        assert!(names.contains(&"get_briefing"));
        assert!(names.contains(&"agent_fixture_ability"));
        assert!(!names.contains(&"user_fixture_ability"));

        let ability_tool = tools
            .iter()
            .find(|tool| tool.name == "agent_fixture_ability")
            .unwrap();
        assert_eq!(
            ability_tool
                .input_schema
                .get("additionalProperties")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn mcp_call_tool_routes_to_inherent_for_static_name() {
        assert_eq!(mcp_route_for_tool_name("get_briefing"), McpToolRoute::Static);
        assert_eq!(mcp_route_for_tool_name("query_entity"), McpToolRoute::Static);
    }

    #[tokio::test]
    async fn mcp_call_tool_routes_to_invoke_ability_for_registered_ability_name() {
        let registry = registry_with(vec![descriptor(
            "agent_fixture_ability",
            AbilityCategory::Read,
            AGENT_ACTORS,
            LIVE_MODES,
        )]);
        let bridge = McpAbilityBridge::new(&registry);
        let result = invoke_mcp_ability_tool(
            &bridge,
            session(1),
            request("agent_fixture_ability", json!({ "subject": "dailyos" })),
        )
        .await
        .unwrap();

        assert_eq!(result.is_error, Some(false));
        let text = result.content[0].as_text().unwrap().text.as_str();
        let value: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(value["ability_name"], "agent_fixture_ability");
        assert_eq!(value["data"]["input"]["subject"], "dailyos");
    }

    #[tokio::test]
    async fn mcp_call_tool_unknown_name_returns_byte_equal_unavailable() {
        let registry = registry_with(vec![]);
        let bridge = McpAbilityBridge::new(&registry);
        let unknown = invoke_mcp_ability_tool(&bridge, session(1), request("unknown", json!({})))
            .await
            .unwrap_err();
        let expected = mcp_error_from_bridge_surface_error(BridgeSurfaceError::AbilityUnavailable);

        assert_eq!(
            serde_json::to_vec(&unknown).unwrap(),
            serde_json::to_vec(&expected).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&unknown.data).unwrap(),
            br#""ability_unavailable""#
        );
    }
}

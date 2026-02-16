//! DailyOS MCP Server — exposes workspace intelligence to Claude Desktop.
//!
//! Standalone binary that communicates over stdio using the Model Context Protocol.
//! Opens the SQLite database read-only so it can run safely alongside the Tauri app.
//!
//! Build: `cargo build --features mcp --bin dailyos-mcp`
//! Usage: spawned by Claude Desktop as configured in claude_desktop_config.json.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rmcp::model::*;
use rmcp::schemars::JsonSchema;
use rmcp::{tool, ServerHandler, ServiceExt};
use serde::{Deserialize, Serialize};

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
    fn new(db: ActionDb, config: Config, embedding_model: Arc<EmbeddingModel>) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            config,
            embedding_model,
        }
    }

    #[tool(description = "Get the daily briefing for DailyOS. Returns today's schedule, actions, emails, and AI-generated narrative briefing. Use this when the user asks about their day, schedule, meetings, or what they need to focus on.")]
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

    #[tool(description = "Look up a specific account, project, or person in the DailyOS workspace. Returns entity details, intelligence summary, open actions, and upcoming meetings. Use this when the user asks about a specific customer, project, or contact.")]
    fn query_entity(&self, #[tool(aggr)] params: QueryEntityParams) -> String {
        let db = match self.db.lock() {
            Ok(db) => db,
            Err(_) => return "Error: DB lock poisoned".to_string(),
        };
        let query_lower = params.query.to_lowercase();
        let entity_type = params.entity_type.as_deref().unwrap_or("all");
        let workspace = &self.config.workspace_path;

        let mut result: Option<EntityResult> = None;

        // Search accounts
        if entity_type == "all" || entity_type == "account" {
            if let Ok(accounts) = db.get_all_accounts() {
                for acct in &accounts {
                    if acct.id == params.query
                        || acct.name.to_lowercase().contains(&query_lower)
                    {
                        let actions = db.get_account_actions(&acct.id).unwrap_or_default();
                        let meetings = db
                            .get_upcoming_meetings_for_account(&acct.id, 5)
                            .unwrap_or_default();
                        let intel = read_entity_intelligence(workspace, acct.tracker_path.as_deref());

                        result = Some(build_entity_result(
                            &acct.id, &acct.name, "account",
                            acct.health.as_deref(), None,
                            acct.lifecycle.as_deref(), intel.as_deref(),
                            &actions, &meetings,
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
                    if proj.id == params.query
                        || proj.name.to_lowercase().contains(&query_lower)
                    {
                        let actions = db.get_project_actions(&proj.id).unwrap_or_default();
                        let meetings =
                            db.get_meetings_for_project(&proj.id, 5).unwrap_or_default();
                        let intel = read_entity_intelligence(workspace, proj.tracker_path.as_deref());

                        result = Some(build_entity_result(
                            &proj.id, &proj.name, "project",
                            None, Some(&proj.status),
                            None, intel.as_deref(),
                            &actions, &meetings,
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
                        let intel = read_entity_intelligence(workspace, person.tracker_path.as_deref());
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

    #[tool(description = "List all accounts, projects, or people in the DailyOS workspace. Use this when the user asks to see their portfolio, customer list, project list, or contacts.")]
    fn list_entities(&self, #[tool(aggr)] params: ListEntitiesParams) -> String {
        let db = match self.db.lock() {
            Ok(db) => db,
            Err(_) => return "Error: DB lock poisoned".to_string(),
        };
        let entity_type = params.entity_type.as_deref().unwrap_or("all");
        let mut items: Vec<EntityListItem> = Vec::new();

        if entity_type == "all" || entity_type == "account" {
            if let Ok(accounts) = db.get_all_accounts() {
                items.extend(accounts.into_iter().map(|a| EntityListItem {
                    id: a.id, name: a.name,
                    entity_type: "account".to_string(),
                    health: a.health, status: a.lifecycle,
                }));
            }
        }

        if entity_type == "all" || entity_type == "project" {
            if let Ok(projects) = db.get_all_projects() {
                items.extend(projects.into_iter().map(|p| EntityListItem {
                    id: p.id, name: p.name,
                    entity_type: "project".to_string(),
                    health: None, status: Some(p.status),
                }));
            }
        }

        if entity_type == "all" || entity_type == "person" {
            if let Ok(people) = db.get_people(None) {
                items.extend(people.into_iter().map(|p| EntityListItem {
                    id: p.id, name: p.name,
                    entity_type: "person".to_string(),
                    health: None, status: Some(p.relationship),
                }));
            }
        }

        serde_json::to_string_pretty(&items).unwrap_or_else(|e| format!("Error: {e}"))
    }

    #[tool(description = "Search past meetings in DailyOS by title, summary, or prep content. Use this when the user asks about past meetings, what was discussed, or wants to find a specific meeting.")]
    fn search_meetings(&self, #[tool(aggr)] params: SearchMeetingsParams) -> String {
        if params.query.trim().is_empty() {
            return "[]".to_string();
        }

        let db = match self.db.lock() {
            Ok(db) => db,
            Err(_) => return "Error: DB lock poisoned".to_string(),
        };
        let pattern = format!("%{}%", params.query.trim());
        let limit = params.limit.unwrap_or(20).min(50) as i64;

        let mut stmt = match db.conn_ref().prepare(
            "SELECT id, title, meeting_type, start_time, account_id, summary, prep_context_json
             FROM meetings_history
             WHERE title LIKE ?1
                OR summary LIKE ?1
                OR prep_context_json LIKE ?1
             ORDER BY start_time DESC
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
                id, title, meeting_type, start_time, account_name,
                summary: match_snippet,
            });
        }

        serde_json::to_string_pretty(&results).unwrap_or_else(|e| format!("Error: {e}"))
    }

    #[tool(description = "Semantic search over workspace files for an entity. Returns the most relevant text passages from documents, transcripts, and notes. Use when the user asks about specific details, information, or topics within their files for a particular account, project, or person.")]
    fn search_content(&self, #[tool(aggr)] params: SearchContentParams) -> String {
        if params.query.trim().is_empty() {
            return "[]".to_string();
        }

        let db = match self.db.lock() {
            Ok(db) => db,
            Err(_) => return "Error: DB lock poisoned".to_string(),
        };

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
            &db, model_ref, entity_id, &params.query, top_k, 0.7, 0.3,
        ) {
            Ok(matches) if matches.is_empty() => {
                format!("No content found for entity '{}' matching '{}'.", params.entity_id, params.query)
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
// ServerHandler — wires tool_box into the MCP protocol
// =============================================================================

#[tool(tool_box)]
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

fn read_entity_intelligence(workspace: &str, tracker_path: Option<&str>) -> Option<String> {
    tracker_path.and_then(|tp| {
        let dir = PathBuf::from(workspace).join(tp);
        dailyos_lib::entity_intel::read_intelligence_json(&dir)
            .ok()
            .and_then(|i| i.executive_assessment)
    })
}

#[allow(clippy::too_many_arguments)]
fn build_entity_result(
    id: &str, name: &str, entity_type: &str,
    health: Option<&str>, status: Option<&str>,
    lifecycle: Option<&str>, intelligence_summary: Option<&str>,
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
        open_actions: actions.iter()
            .filter(|a| a.status == "pending")
            .take(10)
            .map(|a| ActionSummary {
                id: a.id.clone(), title: a.title.clone(),
                priority: a.priority.clone(), due_date: a.due_date.clone(),
            })
            .collect(),
        upcoming_meetings: meetings.iter()
            .take(5)
            .map(|m| MeetingSummary {
                id: m.id.clone(), title: m.title.clone(),
                start_time: m.start_time.clone(), meeting_type: m.meeting_type.clone(),
            })
            .collect(),
    }
}

// =============================================================================
// Main
// =============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config =
        load_config().map_err(|e| anyhow::anyhow!("Failed to load DailyOS config: {e}"))?;

    let db =
        ActionDb::open_readonly().map_err(|e| anyhow::anyhow!("Failed to open database: {e}"))?;

    // Initialize embedding model synchronously — MCP server starts before any
    // tool calls so this won't block user interaction.
    let embedding_model = Arc::new(EmbeddingModel::new());
    let models_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".dailyos")
        .join("models");
    if let Err(e) = embedding_model.initialize(models_dir) {
        eprintln!("Embedding model unavailable: {e}");
    }

    let server = DailyOsMcp::new(db, config, embedding_model);

    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;

    Ok(())
}

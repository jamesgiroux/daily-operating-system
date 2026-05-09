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

use dailyos_lib::abilities::provenance::{
    build_ownership_policy_for_invocation, validate_serialized_subject_ownership, InvocationId,
};
use dailyos_lib::abilities::{AbilityDescriptor, AbilityRegistry};
use dailyos_lib::bridges::mcp::McpAbilityBridge;
use dailyos_lib::bridges::tauri::TauriAbilityBridge;
use dailyos_lib::bridges::{BridgeSurfaceError, McpSessionId};
use dailyos_lib::db::ActionDb;
use dailyos_lib::embeddings::EmbeddingModel;
use dailyos_lib::services::sensitivity::{
    render_mcp_static_json_for_surface, render_mcp_static_text_for_surface, McpStaticTextClass,
    RenderableMcpClaimText, RenderableMcpStaticText, RenderableMcpText,
};
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
    /// Tauri-host confirmation bridge for scoped confirmation-token issuance.
    tauri_bridge: Arc<TauriAbilityBridge<'static>>,
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
        db: Arc<Mutex<ActionDb>>,
        config: Config,
        embedding_model: Arc<EmbeddingModel>,
        ability_bridge: Arc<McpAbilityBridge<'static>>,
        tauri_bridge: Arc<TauriAbilityBridge<'static>>,
        mcp_session_id: McpSessionId,
    ) -> Self {
        Self {
            db,
            config,
            embedding_model,
            ability_bridge,
            tauri_bridge,
            mcp_session_id,
        }
    }

    #[tool(
        description = "Get the daily briefing for DailyOS. Returns today's schedule, actions, emails, and AI-generated narrative briefing. Use this when the user asks about their day, schedule, meetings, or what they need to focus on."
    )]
    fn get_briefing(&self, #[tool(aggr)] params: GetBriefingParams) -> String {
        let today_dir = PathBuf::from(&self.config.workspace_path).join("_today");
        let db = self.db.lock();

        let date = params
            .date
            .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());

        let response = BriefingResponse {
            date,
            schedule: read_json_file(&today_dir.join("data/schedule.json")).and_then(|value| {
                render_mcp_static_json_for_surface(&db, value, &briefing_static_text_class)
            }),
            actions: read_json_file(&today_dir.join("data/actions.json")).and_then(|value| {
                render_mcp_static_json_for_surface(&db, value, &briefing_static_text_class)
            }),
            emails: read_json_file(&today_dir.join("data/emails.json")).and_then(|value| {
                render_mcp_static_json_for_surface(&db, value, &briefing_static_text_class)
            }),
            briefing: read_json_file(&today_dir.join("data/briefing.json")).and_then(|value| {
                render_mcp_static_json_for_surface(&db, value, &briefing_static_text_class)
            }),
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
                        let legacy_intel = db
                            .get_entity_intelligence(&acct.id)
                            .ok()
                            .flatten()
                            .and_then(|i| i.executive_assessment);
                        let intel =
                            mcp_entity_summary(&db, "account", &acct.id, legacy_intel.as_deref());

                        result = Some(build_entity_result(
                            &db,
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
                        let legacy_intel = db
                            .get_entity_intelligence(&proj.id)
                            .ok()
                            .flatten()
                            .and_then(|i| i.executive_assessment);
                        let intel =
                            mcp_entity_summary(&db, "project", &proj.id, legacy_intel.as_deref());

                        result = Some(build_entity_result(
                            &db,
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
                        let legacy_intel = db
                            .get_entity_intelligence(&person.id)
                            .ok()
                            .flatten()
                            .and_then(|i| i.executive_assessment);
                        let intel =
                            mcp_entity_summary(&db, "person", &person.id, legacy_intel.as_deref());
                        result = Some(EntityResult {
                            id: person.id.clone(),
                            name: render_mcp_static_text_for_surface(
                                &db,
                                RenderableMcpText::Static(RenderableMcpStaticText::new(
                                    person.name.clone(),
                                    McpStaticTextClass::PersonName,
                                )),
                            )
                            .unwrap_or_default(), // dos412-render-policy-covered: person names are explicit MCP non-claim metadata.
                            entity_type: "person".to_string(),
                            health: None,
                            status: render_mcp_static_text_for_surface(
                                &db,
                                RenderableMcpText::Static(RenderableMcpStaticText::new(
                                    person.relationship.clone(),
                                    McpStaticTextClass::EntityStatus,
                                )),
                            ),
                            lifecycle: None,
                            intelligence_summary: intel, // dos412-render-policy-covered: intel is returned by mcp_entity_summary.
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
                items.extend(accounts.into_iter().filter_map(|a| {
                    Some(EntityListItem {
                        id: a.id,
                        name: render_mcp_static_text_for_surface(
                            &db,
                            RenderableMcpText::Static(RenderableMcpStaticText::new(
                                a.name,
                                McpStaticTextClass::AccountName,
                            )),
                        )?,
                        entity_type: "account".to_string(),
                        health: a.health.and_then(|health| {
                            render_mcp_static_text_for_surface(
                                &db,
                                RenderableMcpText::Static(RenderableMcpStaticText::new(
                                    health,
                                    McpStaticTextClass::EntityHealth,
                                )),
                            )
                        }),
                        status: a.lifecycle.and_then(|lifecycle| {
                            render_mcp_static_text_for_surface(
                                &db,
                                RenderableMcpText::Static(RenderableMcpStaticText::new(
                                    lifecycle,
                                    McpStaticTextClass::EntityLifecycle,
                                )),
                            )
                        }),
                    })
                }));
            }
        }

        if entity_type == "all" || entity_type == "project" {
            if let Ok(projects) = db.get_all_projects() {
                items.extend(projects.into_iter().filter_map(|p| {
                    Some(EntityListItem {
                        id: p.id,
                        name: render_mcp_static_text_for_surface(
                            &db,
                            RenderableMcpText::Static(RenderableMcpStaticText::new(
                                p.name,
                                McpStaticTextClass::ProjectName,
                            )),
                        )?,
                        entity_type: "project".to_string(),
                        health: None,
                        status: render_mcp_static_text_for_surface(
                            &db,
                            RenderableMcpText::Static(RenderableMcpStaticText::new(
                                p.status,
                                McpStaticTextClass::EntityStatus,
                            )),
                        ),
                    })
                }));
            }
        }

        if entity_type == "all" || entity_type == "person" {
            if let Ok(people) = db.get_people(None) {
                items.extend(people.into_iter().filter_map(|p| {
                    Some(EntityListItem {
                        id: p.id,
                        name: render_mcp_static_text_for_surface(
                            &db,
                            RenderableMcpText::Static(RenderableMcpStaticText::new(
                                p.name,
                                McpStaticTextClass::PersonName,
                            )),
                        )?,
                        entity_type: "person".to_string(),
                        health: None,
                        status: render_mcp_static_text_for_surface(
                            &db,
                            RenderableMcpText::Static(RenderableMcpStaticText::new(
                                p.relationship,
                                McpStaticTextClass::EntityStatus,
                            )),
                        ),
                    })
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

            let Some(title) = render_mcp_static_text_for_surface(
                &db,
                RenderableMcpText::Static(RenderableMcpStaticText::new(
                    title,
                    McpStaticTextClass::MeetingTitle,
                )),
            ) else {
                continue;
            };

            let match_snippet = summary
                .map(|snippet| (snippet, McpStaticTextClass::MeetingSummary))
                .or_else(|| {
                    prep_json.and_then(|json| {
                        serde_json::from_str::<serde_json::Value>(&json)
                            .ok()
                            .and_then(|v| {
                                v.get("intelligenceSummary")
                                    .and_then(|s| s.as_str().map(|s| s.to_string()))
                            })
                            .map(|snippet| (snippet, McpStaticTextClass::MeetingPrepSummary))
                    })
                });
            let match_snippet = match_snippet.and_then(|(snippet, surface_class)| {
                render_mcp_static_text_for_surface(
                    &db,
                    RenderableMcpText::Static(RenderableMcpStaticText::new(snippet, surface_class)),
                )
            });

            let account_name = account_id
                .as_ref()
                .and_then(|aid| db.get_account(aid).ok().flatten())
                .and_then(|a| {
                    render_mcp_static_text_for_surface(
                        &db,
                        RenderableMcpText::Static(RenderableMcpStaticText::new(
                            a.name,
                            McpStaticTextClass::AccountName,
                        )),
                    )
                });

            results.push(MeetingSearchItem {
                id,
                title,
                meeting_type: render_mcp_static_text_for_surface(
                    &db,
                    RenderableMcpText::Static(RenderableMcpStaticText::new(
                        meeting_type,
                        McpStaticTextClass::MeetingType,
                    )),
                )
                .unwrap_or_default(),
                start_time: render_mcp_static_text_for_surface(
                    &db,
                    RenderableMcpText::Static(RenderableMcpStaticText::new(
                        start_time,
                        McpStaticTextClass::DateTime,
                    )),
                )
                .unwrap_or_default(),
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
                let mut rendered_count = 0;
                for m in matches.iter() {
                    let raw_chunk = m.chunk_text.chars().take(1000).collect::<String>();
                    let Some(chunk_text) = render_mcp_static_text_for_surface(
                        &db,
                        RenderableMcpText::Static(RenderableMcpStaticText::new(
                            raw_chunk,
                            McpStaticTextClass::ContentChunk,
                        )),
                    ) else {
                        continue;
                    };
                    let Some(filename) = render_mcp_static_text_for_surface(
                        &db,
                        RenderableMcpText::Static(RenderableMcpStaticText::new(
                            m.filename.clone(),
                            McpStaticTextClass::ContentFilename,
                        )),
                    ) else {
                        continue;
                    };
                    let Some(content_type) = render_mcp_static_text_for_surface(
                        &db,
                        RenderableMcpText::Static(RenderableMcpStaticText::new(
                            m.content_type.clone(),
                            McpStaticTextClass::ContentType,
                        )),
                    ) else {
                        continue;
                    };
                    let Some(relative_path) = render_mcp_static_text_for_surface(
                        &db,
                        RenderableMcpText::Static(RenderableMcpStaticText::new(
                            m.relative_path.clone(),
                            McpStaticTextClass::ContentRelativePath,
                        )),
                    ) else {
                        continue;
                    };
                    rendered_count += 1;
                    output.push_str(&format!(
                        "## Result {} — {} ({})\n**File:** {}\n**Score:** {:.2} (vector: {:.2}, text: {:.2})\n\n{}\n\n---\n\n",
                        rendered_count,
                        filename,
                        content_type,
                        relative_path,
                        m.combined_score,
                        m.vector_score,
                        m.text_score,
                        chunk_text,
                    ));
                }
                if output.is_empty() {
                    format!(
                        "No renderable content found for entity '{}' matching '{}'.",
                        params.entity_id, params.query
                    )
                } else {
                    output
                }
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
    GetProvenance,
    RequestConfirmation,
    Ability,
}

pub fn mcp_route_for_tool_name(name: &str) -> McpToolRoute {
    if DailyOsMcp::tool_box().map.contains_key(name) {
        McpToolRoute::Static
    } else if name == "get_provenance" {
        McpToolRoute::GetProvenance
    } else if name == "request_confirmation" {
        McpToolRoute::RequestConfirmation
    } else {
        McpToolRoute::Ability
    }
}

pub fn list_hybrid_tools_for_bridge(ability_bridge: &McpAbilityBridge<'_>) -> Vec<Tool> {
    let mut tools = DailyOsMcp::tool_box().list();
    tools.push(get_provenance_tool_descriptor());
    // Hide request_confirmation from the advertised tool set while the gate
    // is off so MCP clients don't see a tool that always returns
    // ability_unavailable.
    if ability_bridge.confirmation_enabled() {
        tools.push(request_confirmation_tool_descriptor());
    }
    // Keep this descriptor-based listing until the deferred MCP server migration
    // moves tool discovery to the contract-first operation registry.
    tools.extend(
        ability_bridge
            .list_descriptors()
            .iter()
            .map(|descriptor| ability_descriptor_to_tool(descriptor)),
    );
    tools
}

fn get_provenance_tool_descriptor() -> Tool {
    let mut invocation_id_schema = JsonObject::new();
    invocation_id_schema.insert(
        "type".to_string(),
        serde_json::Value::String("string".to_string()),
    );

    let mut properties = JsonObject::new();
    properties.insert(
        "invocation_id".to_string(),
        serde_json::Value::Object(invocation_id_schema),
    );

    let mut schema = JsonObject::new();
    schema.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );
    schema.insert(
        "additionalProperties".to_string(),
        serde_json::Value::Bool(false),
    );
    schema.insert(
        "properties".to_string(),
        serde_json::Value::Object(properties),
    );
    schema.insert(
        "required".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::String("invocation_id".to_string())]),
    );

    Tool::new(
        "get_provenance",
        "Fetch detailed rendered provenance for a prior MCP ability invocation in this session.",
        schema,
    )
}

fn request_confirmation_tool_descriptor() -> Tool {
    let mut ability_schema = JsonObject::new();
    ability_schema.insert(
        "type".to_string(),
        serde_json::Value::String("string".to_string()),
    );

    let mut input_json_schema = JsonObject::new();
    input_json_schema.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );

    let mut properties = JsonObject::new();
    properties.insert(
        "ability".to_string(),
        serde_json::Value::Object(ability_schema),
    );
    properties.insert(
        "input_json".to_string(),
        serde_json::Value::Object(input_json_schema),
    );

    let mut schema = JsonObject::new();
    schema.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );
    schema.insert(
        "additionalProperties".to_string(),
        serde_json::Value::Bool(false),
    );
    schema.insert(
        "properties".to_string(),
        serde_json::Value::Object(properties),
    );
    schema.insert(
        "required".to_string(),
        serde_json::Value::Array(vec![
            serde_json::Value::String("ability".to_string()),
            serde_json::Value::String("input_json".to_string()),
        ]),
    );

    Tool::new(
        "request_confirmation",
        "Request a scoped confirmation token from the Tauri host for a later ability invocation.",
        schema,
    )
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
    let input_for_policy = input_json.clone();
    let confirmation =
        ability_bridge.take_confirmation_token(session_id, &ability_name, &input_json);
    let response = ability_bridge
        .invoke_ability(session_id, &ability_name, input_json, false, confirmation)
        .await
        .map_err(mcp_error_from_bridge_surface_error)?;
    if ability_name != "get_entity_context" {
        let ability_meta = ability_bridge
            .list_descriptors()
            .iter()
            .find(|descriptor| descriptor.name == ability_name)
            .copied()
            .ok_or_else(|| {
                mcp_error_from_bridge_surface_error(BridgeSurfaceError::AbilityUnavailable)
            })?;
        let policy = build_ownership_policy_for_invocation(
            ability_meta,
            &input_for_policy,
            response.raw_provenance_value(),
        )
        .map_err(|error| mcp_error_from_bridge_surface_error(BridgeSurfaceError::Ownership(error)))?
        .rejecting_sources_outside_subject_scope();
        validate_serialized_subject_ownership(
            response.data.clone(),
            response.raw_provenance_value().clone(),
            response.diagnostics.clone(),
            &[],
            policy,
        )
        .map_err(|error| {
            mcp_error_from_bridge_surface_error(BridgeSurfaceError::Ownership(error))
        })?;
    }
    let content = Content::json(response)?;

    Ok(CallToolResult::success(vec![content]))
}

pub fn invoke_mcp_get_provenance_tool(
    ability_bridge: &McpAbilityBridge<'_>,
    session_id: McpSessionId,
    request: CallToolRequestParam,
) -> Result<CallToolResult, McpError> {
    let invocation_id = get_provenance_invocation_id(&request)?;
    Ok(ability_bridge.get_provenance_tool_response(session_id, invocation_id))
}

pub async fn invoke_mcp_request_confirmation_tool(
    ability_bridge: &McpAbilityBridge<'_>,
    session_id: McpSessionId,
    request: CallToolRequestParam,
    tauri_bridge: &TauriAbilityBridge<'_>,
) -> Result<CallToolResult, McpError> {
    let (ability, input_json) = request_confirmation_args(&request)?;
    ability_bridge
        .request_confirmation_tool(session_id, &ability, &input_json, tauri_bridge)
        .await
}

fn request_confirmation_args(
    request: &CallToolRequestParam,
) -> Result<(String, serde_json::Value), McpError> {
    let Some(arguments) = request.arguments.as_ref() else {
        return Err(mcp_error_from_bridge_surface_error(
            BridgeSurfaceError::AbilityUnavailable,
        ));
    };

    if arguments.len() != 2 {
        return Err(mcp_error_from_bridge_surface_error(
            BridgeSurfaceError::AbilityUnavailable,
        ));
    }

    let Some(ability) = arguments.get("ability").and_then(serde_json::Value::as_str) else {
        return Err(mcp_error_from_bridge_surface_error(
            BridgeSurfaceError::AbilityUnavailable,
        ));
    };
    let Some(input_json) = arguments.get("input_json") else {
        return Err(mcp_error_from_bridge_surface_error(
            BridgeSurfaceError::AbilityUnavailable,
        ));
    };
    if !input_json.is_object() {
        return Err(mcp_error_from_bridge_surface_error(
            BridgeSurfaceError::AbilityUnavailable,
        ));
    }

    Ok((ability.to_string(), input_json.clone()))
}

fn get_provenance_invocation_id(request: &CallToolRequestParam) -> Result<InvocationId, McpError> {
    let Some(arguments) = request.arguments.as_ref() else {
        return Err(mcp_error_from_bridge_surface_error(
            BridgeSurfaceError::AbilityUnavailable,
        ));
    };

    if arguments.len() != 1 {
        return Err(mcp_error_from_bridge_surface_error(
            BridgeSurfaceError::AbilityUnavailable,
        ));
    }

    let Some(invocation_id) = arguments
        .get("invocation_id")
        .and_then(serde_json::Value::as_str)
    else {
        return Err(mcp_error_from_bridge_surface_error(
            BridgeSurfaceError::AbilityUnavailable,
        ));
    };

    InvocationId::parse(invocation_id)
        .map_err(|_| mcp_error_from_bridge_surface_error(BridgeSurfaceError::AbilityUnavailable))
}

pub fn mcp_error_from_bridge_surface_error(error: BridgeSurfaceError) -> McpError {
    let data = serde_json::to_value(&error)
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
            McpToolRoute::GetProvenance => {
                invoke_mcp_get_provenance_tool(&self.ability_bridge, self.mcp_session_id, request)
            }
            McpToolRoute::RequestConfirmation => {
                invoke_mcp_request_confirmation_tool(
                    &self.ability_bridge,
                    self.mcp_session_id,
                    request,
                    &self.tauri_bridge,
                )
                .await
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

fn mcp_entity_summary(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    _legacy_summary: Option<&str>,
) -> Option<String> {
    let subject_ref = serde_json::json!({
        "kind": entity_type,
        "id": entity_id,
    })
    .to_string();
    let claims =
        dailyos_lib::services::claims::load_claims_active(db, &subject_ref, Some("entity_summary"))
            .ok()?;
    if claims.is_empty() {
        return None;
    }

    claims.into_iter().find_map(|claim| {
        let text = claim
            .metadata_json
            .as_deref()
            .and_then(|metadata| {
                serde_json::from_str::<serde_json::Value>(metadata)
                    .ok()?
                    .get("legacy_projection_value")
                    .cloned()
            })
            .and_then(|value| {
                value.as_str().map(str::to_string).or_else(|| {
                    value
                        .get("text")
                        .and_then(|text| text.as_str())
                        .map(str::to_string)
                })
            })
            .unwrap_or_else(|| claim.text.clone()); // dos412-render-policy-covered: claim carrier keeps id+sensitivity for MCP rendering.
        render_mcp_static_text_for_surface(
            db,
            RenderableMcpText::Claim(RenderableMcpClaimText::from_claim_value(&claim, text)),
        )
    })
}

fn read_json_file(path: &std::path::Path) -> Option<serde_json::Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

fn briefing_static_text_class(path: &[String], _text: &str) -> Option<McpStaticTextClass> {
    let root = path.first().map(String::as_str)?;
    let leaf = path.last().map(String::as_str)?;
    match (root, leaf) {
        ("schedule", "title") => Some(McpStaticTextClass::MeetingTitle),
        ("schedule", "meeting_type") | ("schedule", "type") => {
            Some(McpStaticTextClass::MeetingType)
        }
        ("schedule", "start_time")
        | ("schedule", "end_time")
        | ("schedule", "date")
        | ("actions", "due_date")
        | ("actions", "created_at")
        | ("briefing", "date") => Some(McpStaticTextClass::DateTime),
        ("schedule", "account_name") | ("schedule", "account") => {
            Some(McpStaticTextClass::AccountName)
        }
        ("schedule", "project_name") | ("schedule", "project") => {
            Some(McpStaticTextClass::ProjectName)
        }
        ("schedule", "person_name")
        | ("schedule", "attendee")
        | ("schedule", "attendees")
        | ("emails", "sender_name")
        | ("emails", "from_name") => Some(McpStaticTextClass::PersonName),
        ("actions", "priority") => Some(McpStaticTextClass::ActionPriority),
        ("actions", "status") => Some(McpStaticTextClass::EntityStatus),
        ("actions", "title") => Some(McpStaticTextClass::ActionTitle),
        ("emails", "subject") => Some(McpStaticTextClass::EmailSubject),
        ("emails", "snippet") | ("emails", "summary") => Some(McpStaticTextClass::EmailSnippet),
        ("briefing", "narrative") | ("briefing", "summary") | ("briefing", "text") => {
            Some(McpStaticTextClass::BriefingNarrative)
        }
        _ => None,
    }
}

fn entity_name_static_class(entity_type: &str) -> McpStaticTextClass {
    match entity_type {
        "account" => McpStaticTextClass::AccountName,
        "project" => McpStaticTextClass::ProjectName,
        "person" => McpStaticTextClass::PersonName,
        _ => McpStaticTextClass::EntityType,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_entity_result(
    db: &ActionDb,
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
        name: render_mcp_static_text_for_surface(
            db,
            RenderableMcpText::Static(RenderableMcpStaticText::new(
                name,
                entity_name_static_class(entity_type),
            )),
        )
        .unwrap_or_default(), // dos412-render-policy-covered: entity names are explicit MCP non-claim metadata.
        entity_type: entity_type.to_string(),
        health: health.and_then(|health| {
            render_mcp_static_text_for_surface(
                db,
                RenderableMcpText::Static(RenderableMcpStaticText::new(
                    health,
                    McpStaticTextClass::EntityHealth,
                )),
            )
        }),
        status: status.and_then(|status| {
            render_mcp_static_text_for_surface(
                db,
                RenderableMcpText::Static(RenderableMcpStaticText::new(
                    status,
                    McpStaticTextClass::EntityStatus,
                )),
            )
        }),
        lifecycle: lifecycle.and_then(|lifecycle| {
            render_mcp_static_text_for_surface(
                db,
                RenderableMcpText::Static(RenderableMcpStaticText::new(
                    lifecycle,
                    McpStaticTextClass::EntityLifecycle,
                )),
            )
        }),
        intelligence_summary: intelligence_summary.map(str::to_string), // dos412-render-policy-covered: caller supplies mcp_entity_summary-rendered text.
        open_actions: actions
            .iter()
            .filter(|a| matches!(a.status.as_str(), "backlog" | "unstarted" | "started"))
            .take(10)
            .filter_map(|a| {
                render_mcp_static_text_for_surface(
                    db,
                    RenderableMcpText::Static(RenderableMcpStaticText::new(
                        a.title.clone(),
                        McpStaticTextClass::ActionTitle,
                    )),
                )
                .map(|title| ActionSummary {
                    id: a.id.clone(),
                    title,
                    priority: render_mcp_static_text_for_surface(
                        db,
                        RenderableMcpText::Static(RenderableMcpStaticText::new(
                            a.priority.to_string(),
                            McpStaticTextClass::ActionPriority,
                        )),
                    )
                    .unwrap_or_default(),
                    due_date: a.due_date.clone().and_then(|due_date| {
                        render_mcp_static_text_for_surface(
                            db,
                            RenderableMcpText::Static(RenderableMcpStaticText::new(
                                due_date,
                                McpStaticTextClass::DateTime,
                            )),
                        )
                    }),
                })
            })
            .collect(),
        upcoming_meetings: meetings
            .iter()
            .take(5)
            .filter_map(|m| {
                render_mcp_static_text_for_surface(
                    db,
                    RenderableMcpText::Static(RenderableMcpStaticText::new(
                        m.title.clone(),
                        McpStaticTextClass::MeetingTitle,
                    )),
                )
                .map(|title| MeetingSummary {
                    id: m.id.clone(),
                    title,
                    start_time: render_mcp_static_text_for_surface(
                        db,
                        RenderableMcpText::Static(RenderableMcpStaticText::new(
                            m.start_time.clone(),
                            McpStaticTextClass::DateTime,
                        )),
                    )
                    .unwrap_or_default(),
                    meeting_type: render_mcp_static_text_for_surface(
                        db,
                        RenderableMcpText::Static(RenderableMcpStaticText::new(
                            m.meeting_type.clone(),
                            McpStaticTextClass::MeetingType,
                        )),
                    )
                    .unwrap_or_default(),
                })
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

    let db = ActionDb::open_readonly(std::sync::Arc::new(dailyos_lib::db::LocalKeychain::new()))
        .map_err(|e| anyhow::anyhow!("Failed to open database: {e}"))?;

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
    let db = Arc::new(Mutex::new(db));
    let ability_bridge = Arc::new(McpAbilityBridge::new_with_action_db_readers(
        ability_registry,
        Arc::clone(&db),
    ));
    let tauri_bridge = Arc::new(TauriAbilityBridge::new(ability_registry));
    let mcp_session_id = McpSessionId::new_process_scoped();
    let server = DailyOsMcp::new(
        db,
        config,
        embedding_model,
        ability_bridge,
        tauri_bridge,
        mcp_session_id,
    );

    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use dailyos_lib::abilities::provenance::{provenance_for_test, SubjectAttribution, SubjectRef};
    use dailyos_lib::abilities::registry::{AbilityPolicy, SignalPolicy};
    use dailyos_lib::abilities::{
        AbilityCategory, AbilityContext, AbilityError, AbilityRegistry, Actor,
    };
    use dailyos_lib::bridges::tauri::UserAttestationHost;
    use dailyos_lib::bridges::UserAttestationRequest;
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
            let subject = input
                .get("subject")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("dailyos")
                .to_string();
            let produced_at = chrono::DateTime::parse_from_rfc3339("2026-05-08T00:00:00Z")
                .expect("static RFC3339")
                .with_timezone(&chrono::Utc);
            let provenance = provenance_for_test(
                "fixture",
                produced_at,
                SubjectAttribution::direct_confident(SubjectRef::Account(subject)),
                Vec::new(),
                Vec::new(),
                BTreeMap::new(),
                None,
                Vec::new(),
            );

            Ok(json!({
                "data": {
                    "routed": true,
                    "input": input,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str()
                },
                "ability_version": { "major": 1, "minor": 0 },
                "diagnostics": { "warnings": [] },
                "provenance": serde_json::to_value(provenance).expect("fixture provenance")
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

    fn confirmation_descriptor(mut descriptor: AbilityDescriptor) -> AbilityDescriptor {
        descriptor.policy.requires_confirmation = true;
        descriptor
    }

    fn closed_object_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "subject": { "type": "string" }
            }
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

    fn tool_result_json(result: &CallToolResult) -> serde_json::Value {
        let text = result.content[0].as_text().unwrap().text.as_str();
        serde_json::from_str(text).unwrap()
    }

    #[derive(Default)]
    struct ApprovingAttestationHost;

    impl UserAttestationHost for ApprovingAttestationHost {
        fn request_user_attestation<'a>(
            &'a self,
            _request: UserAttestationRequest,
        ) -> Pin<Box<dyn Future<Output = Result<(), BridgeSurfaceError>> + Send + 'a>> {
            Box::pin(async { Ok(()) })
        }
    }

    #[test]
    fn mcp_serverhandler_tool_box_macro_removed_and_manual_routes_static_or_ability() {
        let source =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/mcp/main.rs"))
                .unwrap();
        let inherent_tool_box = concat!("#[tool", "(tool_box)]\nimpl DailyOsMcp");
        let serverhandler_tool_box =
            concat!("#[tool", "(tool_box)]\nimpl ServerHandler for DailyOsMcp");

        assert!(source.contains(inherent_tool_box));
        assert!(!source.contains(serverhandler_tool_box));
        assert!(source.contains("impl ServerHandler for DailyOsMcp"));
        assert!(source.contains("McpToolRoute::Static"));
        assert!(source.contains("invoke_mcp_ability_tool"));
        assert!(source.contains("invoke_mcp_request_confirmation_tool"));
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
        // Confirmation flow is gated off by default until the W5/W6 prompt UI
        // ships; tools that need to assert request_confirmation appears in the
        // advertised list flip the gate on for that scope only.
        let bridge = McpAbilityBridge::new(&registry).with_confirmation_enabled();
        let tools = list_hybrid_tools_for_bridge(&bridge);
        let names = tools
            .iter()
            .map(|tool| tool.name.as_ref())
            .collect::<Vec<_>>();

        assert!(names.contains(&"get_briefing"));
        assert!(names.contains(&"get_provenance"));
        assert!(names.contains(&"request_confirmation"));
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

        let get_provenance_tool = tools
            .iter()
            .find(|tool| tool.name == "get_provenance")
            .unwrap();
        assert_eq!(
            get_provenance_tool
                .input_schema
                .get("additionalProperties")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );

        let request_confirmation_tool = tools
            .iter()
            .find(|tool| tool.name == "request_confirmation")
            .unwrap();
        assert_eq!(
            request_confirmation_tool
                .input_schema
                .get("additionalProperties")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn mcp_list_tools_omits_request_confirmation_when_gate_disabled() {
        let registry = registry_with(vec![]);
        let bridge = McpAbilityBridge::new(&registry);
        let tools = list_hybrid_tools_for_bridge(&bridge);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(
            !names.contains(&"request_confirmation"),
            "request_confirmation must not be advertised while the gate is off"
        );
        assert!(
            names.contains(&"get_provenance"),
            "other inherent tools should still appear"
        );
    }

    #[test]
    fn mcp_tool_descriptor_uses_registry_input_schema() {
        let descriptor = descriptor(
            "agent_fixture_ability",
            AbilityCategory::Read,
            AGENT_ACTORS,
            LIVE_MODES,
        );
        let expected_schema = match (descriptor.input_schema)() {
            serde_json::Value::Object(object) => object,
            _ => JsonObject::new(),
        };
        let registry = registry_with(vec![descriptor]);
        let bridge = McpAbilityBridge::new(&registry);

        let tools = list_hybrid_tools_for_bridge(&bridge);
        let ability_tool = tools
            .iter()
            .find(|tool| tool.name == "agent_fixture_ability")
            .expect("registry ability tool descriptor");

        assert_eq!(ability_tool.input_schema.as_ref(), &expected_schema);
    }

    #[test]
    fn mcp_call_tool_routes_to_inherent_for_static_name() {
        assert_eq!(
            mcp_route_for_tool_name("get_briefing"),
            McpToolRoute::Static
        );
        assert_eq!(
            mcp_route_for_tool_name("query_entity"),
            McpToolRoute::Static
        );
        assert_eq!(
            mcp_route_for_tool_name("get_provenance"),
            McpToolRoute::GetProvenance
        );
        assert_eq!(
            mcp_route_for_tool_name("request_confirmation"),
            McpToolRoute::RequestConfirmation
        );
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
        assert_eq!(value["data"]["routed"], true);
    }

    #[tokio::test]
    async fn mcp_call_tool_consumes_confirmation_token_via_session_cache() {
        let registry = registry_with(vec![confirmation_descriptor(descriptor(
            "agent_confirmed",
            AbilityCategory::Read,
            AGENT_ACTORS,
            LIVE_MODES,
        ))]);
        let ability_bridge = McpAbilityBridge::new(&registry).with_confirmation_enabled();
        let tauri_bridge = TauriAbilityBridge::new_with_attestation_host(
            &registry,
            Arc::new(ApprovingAttestationHost),
        );
        let session_id = session(1);
        let input = json!({ "subject": "dailyos" });

        let confirmation_result = invoke_mcp_request_confirmation_tool(
            &ability_bridge,
            session_id,
            request(
                "request_confirmation",
                json!({ "ability": "agent_confirmed", "input_json": input.clone() }),
            ),
            &tauri_bridge,
        )
        .await
        .unwrap();
        assert_eq!(confirmation_result.is_error, Some(false));

        let ability_result = invoke_mcp_ability_tool(
            &ability_bridge,
            session_id,
            request("agent_confirmed", input.clone()),
        )
        .await
        .unwrap();
        assert_eq!(ability_result.is_error, Some(false));

        let second_call = invoke_mcp_ability_tool(
            &ability_bridge,
            session_id,
            request("agent_confirmed", input),
        )
        .await
        .unwrap_err();
        let expected = mcp_error_from_bridge_surface_error(BridgeSurfaceError::AbilityUnavailable);
        assert_eq!(
            serde_json::to_vec(&second_call).unwrap(),
            serde_json::to_vec(&expected).unwrap()
        );
    }

    #[tokio::test]
    async fn mcp_call_tool_with_no_token_for_privileged_ability_returns_byte_equal_unavailable() {
        let registry = registry_with(vec![confirmation_descriptor(descriptor(
            "agent_confirmed",
            AbilityCategory::Read,
            AGENT_ACTORS,
            LIVE_MODES,
        ))]);
        let ability_bridge = McpAbilityBridge::new(&registry);

        let err = invoke_mcp_ability_tool(
            &ability_bridge,
            session(1),
            request("agent_confirmed", json!({ "subject": "dailyos" })),
        )
        .await
        .unwrap_err();
        let expected = mcp_error_from_bridge_surface_error(BridgeSurfaceError::AbilityUnavailable);

        assert_eq!(
            serde_json::to_vec(&err).unwrap(),
            serde_json::to_vec(&expected).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&err.data).unwrap(),
            br#""ability_unavailable""#
        );
    }

    #[tokio::test]
    async fn mcp_call_tool_with_mismatched_args_after_token_issuance_returns_byte_equal_unavailable(
    ) {
        let registry = registry_with(vec![confirmation_descriptor(descriptor(
            "agent_confirmed",
            AbilityCategory::Read,
            AGENT_ACTORS,
            LIVE_MODES,
        ))]);
        let ability_bridge = McpAbilityBridge::new(&registry).with_confirmation_enabled();
        let tauri_bridge = TauriAbilityBridge::new_with_attestation_host(
            &registry,
            Arc::new(ApprovingAttestationHost),
        );
        let session_id = session(1);

        invoke_mcp_request_confirmation_tool(
            &ability_bridge,
            session_id,
            request(
                "request_confirmation",
                json!({
                    "ability": "agent_confirmed",
                    "input_json": { "subject": "issued" }
                }),
            ),
            &tauri_bridge,
        )
        .await
        .unwrap();

        let err = invoke_mcp_ability_tool(
            &ability_bridge,
            session_id,
            request("agent_confirmed", json!({ "subject": "different" })),
        )
        .await
        .unwrap_err();
        let expected = mcp_error_from_bridge_surface_error(BridgeSurfaceError::AbilityUnavailable);

        assert_eq!(
            serde_json::to_vec(&err).unwrap(),
            serde_json::to_vec(&expected).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&err.data).unwrap(),
            br#""ability_unavailable""#
        );
    }

    #[tokio::test]
    async fn mcp_call_tool_routes_get_provenance_to_bridge_session_scoped_lookup() {
        let registry = registry_with(vec![descriptor(
            "agent_fixture_ability",
            AbilityCategory::Read,
            AGENT_ACTORS,
            LIVE_MODES,
        )]);
        let bridge = McpAbilityBridge::new(&registry);
        let session_id = session(1);

        let ability_result = invoke_mcp_ability_tool(
            &bridge,
            session_id,
            request("agent_fixture_ability", json!({ "subject": "dailyos" })),
        )
        .await
        .unwrap();
        let ability_value = tool_result_json(&ability_result);
        let invocation_id = ability_value["invocation_id"].as_str().unwrap();

        let provenance_result = invoke_mcp_get_provenance_tool(
            &bridge,
            session_id,
            request("get_provenance", json!({ "invocation_id": invocation_id })),
        )
        .unwrap();
        let provenance_value = tool_result_json(&provenance_result);

        assert_eq!(provenance_result.is_error, Some(false));
        assert_eq!(provenance_value["surface"], "mcp_tool_detail");
        assert_eq!(provenance_value["value"]["invocation_id"], invocation_id);

        let cross_session = invoke_mcp_get_provenance_tool(
            &bridge,
            session(2),
            request("get_provenance", json!({ "invocation_id": invocation_id })),
        )
        .unwrap();
        assert_eq!(cross_session.is_error, Some(true));
        assert_eq!(
            tool_result_json(&cross_session),
            json!("ability_unavailable")
        );
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

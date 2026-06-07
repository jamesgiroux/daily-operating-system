//! `dailyos.submit.action` MCP handler.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::commands::CreateActionRequest;
use crate::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};
use crate::services::mcp_v2::handler_context::McpHandlerContext;
use crate::services::mcp_v2::target_handles::{
    mint_replacing_target_handle, resolve_target_handle, resolved_target_watermark_matches,
    MintTargetHandle, ResolveTargetHandle, TargetHandleResolutionError, TargetKind,
};
use crate::signals::propagation::PropagationEngine;

use super::tool_utils::{
    bad_params, current_entity_watermark, deterministic_uuid_from_replay_key,
    mcp_submit_replay_key, mutation_cursor_for_target, reject_raw_id_params, required_str,
    unavailable_payload, validate_bounded_string, validate_yyyy_mm_dd,
};

const SCHEMA_VERSION: &str = "mcp.action_submit.v1";

pub struct CreateActionHandler {
    description: ToolDescription,
    signal_engine: Arc<PropagationEngine>,
}

impl CreateActionHandler {
    pub fn new(description: ToolDescription, signal_engine: Arc<PropagationEngine>) -> Self {
        Self {
            description,
            signal_engine,
        }
    }
}

impl McpToolHandler for CreateActionHandler {
    fn description(&self) -> &ToolDescription {
        &self.description
    }

    fn invoke(
        &self,
        ctx: &McpHandlerContext,
        actor: &McpActor,
        params: Value,
    ) -> Result<Value, ToolError> {
        reject_raw_id_params(&params)?;
        let description = validate_bounded_string(
            &required_str(&params, "description")?,
            "description",
            1,
            280,
        )?;
        let due_date = super::tool_utils::optional_str(&params, "due_date")?;
        if let Some(date) = due_date.as_deref() {
            validate_yyyy_mm_dd(date, "due_date")?;
        }
        let priority = super::tool_utils::optional_str(&params, "priority")?
            .map(|value| priority_to_service_value(&value))
            .transpose()?;
        let entity_handle = super::tool_utils::optional_str(&params, "entity_handle")?;

        let result = ctx.with_conn(|db| {
            let (account_id, project_id, person_id) = if let Some(handle) = entity_handle.as_deref()
            {
                match resolve_target_handle(
                    db,
                    ResolveTargetHandle {
                        actor,
                        handle,
                        expected_kind: TargetKind::Entity,
                        current_watermark_material: None,
                    },
                ) {
                    Ok(resolved) => {
                        let Some(current_watermark) =
                            current_entity_watermark(db, &resolved.target_ref)?
                        else {
                            return Ok(unavailable_payload(
                                SCHEMA_VERSION,
                                &self.description.name,
                                true,
                                Some(handle),
                            ));
                        };
                        if !resolved_target_watermark_matches(&resolved, &current_watermark)
                            .map_err(|error| ToolError::Internal {
                                trace_id: format!("mcp_action_entity_handle_watermark:{error}"),
                            })?
                        {
                            return Ok(unavailable_payload(
                                SCHEMA_VERSION,
                                &self.description.name,
                                true,
                                Some(handle),
                            ));
                        }
                        entity_target_for_action(&resolved.target_ref)?
                    }
                    Err(TargetHandleResolutionError::Unavailable { refresh_required }) => {
                        return Ok(unavailable_payload(
                            SCHEMA_VERSION,
                            &self.description.name,
                            refresh_required,
                            Some(handle),
                        ));
                    }
                    Err(TargetHandleResolutionError::Internal(detail)) => {
                        return Err(ToolError::Internal {
                            trace_id: format!("mcp_action_entity_handle_resolve:{detail}"),
                        });
                    }
                }
            } else {
                (None, None, None)
            };

            let clock = SystemClock;
            let rng = SystemRng;
            let external = ExternalClients::default();
            let service_ctx =
                ServiceContext::new_live(&clock, &rng, &external).with_actor("mcp_client");
            let replay_key = mcp_submit_replay_key(
                actor,
                &self.description.name,
                "submit.action",
                &json!({
                    "description": description.clone(),
                    "priority": priority.clone(),
                    "due_date": due_date.clone(),
                    "account_id": account_id.clone(),
                    "project_id": project_id.clone(),
                    "person_id": person_id.clone(),
                }),
            )?;
            let deterministic_action_id =
                deterministic_uuid_from_replay_key("action", &replay_key)?;
            let action_id = crate::services::actions::create_action_with_db_with_id(
                &service_ctx,
                CreateActionRequest {
                    title: description.clone(),
                    priority,
                    due_date,
                    account_id,
                    project_id,
                    person_id,
                    context: None,
                    source_label: Some("MCP".to_string()),
                    action_kind: Some(crate::action_status::KIND_TASK.to_string()),
                },
                db,
                &self.signal_engine,
                Some(&deterministic_action_id),
            )
            .map_err(|error| ToolError::UpstreamFailure { detail: error })?;
            let action = db
                .get_action_by_id(&action_id)
                .map_err(|error| ToolError::UpstreamFailure {
                    detail: error.to_string(),
                })?
                .ok_or_else(|| ToolError::UpstreamFailure {
                    detail: "created action could not be reloaded".to_string(),
                })?;
            let watermark = action_watermark(&action);
            let action_handle = mint_replacing_target_handle(
                db,
                MintTargetHandle {
                    actor,
                    originating_tool: &self.description.name,
                    result_item_path: "/action",
                    target_kind: TargetKind::Action,
                    target_ref: json!({ "action_id": action.id }),
                    sensitivity_tier: "internal",
                    provenance_material: &format!("mcp_submit_action:{}", action.id),
                    watermark_material: &watermark,
                },
            )
            .map_err(|error| ToolError::Internal {
                trace_id: format!("mcp_action_handle_mint:{}", error),
            })?;

            Ok(json!({
                "schema_version": SCHEMA_VERSION,
                "tool_name": self.description.name.as_str(),
                "status": "ok",
                "action_handle": action_handle,
                "semantic_status": "created",
                "stored_status": action.status,
                "mutation_cursor": mutation_cursor_for_target(
                    &self.description.name,
                    "ok",
                    "action",
                    &action_handle,
                ),
            }))
        });
        result.unwrap_or_else(|| {
            Err(ToolError::Internal {
                trace_id: "mcp_submit_action_missing_owned_connection".to_string(),
            })
        })
    }
}

fn priority_to_service_value(value: &str) -> Result<String, ToolError> {
    match value {
        "none" => Ok(crate::action_status::PRIORITY_NONE.to_string()),
        "urgent" => Ok(crate::action_status::PRIORITY_URGENT.to_string()),
        "high" => Ok(crate::action_status::PRIORITY_HIGH.to_string()),
        "medium" => Ok(crate::action_status::PRIORITY_MEDIUM.to_string()),
        "low" => Ok(crate::action_status::PRIORITY_LOW.to_string()),
        other => Err(bad_params(format!("unsupported priority `{other}`"))),
    }
}

type EntityActionTarget = (Option<String>, Option<String>, Option<String>);

fn entity_target_for_action(target_ref: &Value) -> Result<EntityActionTarget, ToolError> {
    let entity_type = target_ref
        .get("entity_type")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_params("entity handle is unavailable"))?;
    let entity_id = target_ref
        .get("entity_id")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_params("entity handle is unavailable"))?
        .to_string();
    Ok(match entity_type {
        "account" => (Some(entity_id), None, None),
        "project" => (None, Some(entity_id), None),
        "person" => (None, None, Some(entity_id)),
        _ => return Err(bad_params("entity handle cannot target an action")),
    })
}

fn action_watermark(action: &crate::db::DbAction) -> String {
    format!(
        "action:{}:{}:{}",
        action.id, action.updated_at, action.status
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use chrono::Utc;
    use serde_json::json;

    use super::*;
    use crate::db::{ActionDb, DbAccount};
    use crate::services::mcp_v2::contracts::{
        McpClientId, OpaqueConversationHandle, ParamSchema, ReturnSpec, Scope, ScopedName, Side,
    };
    use crate::services::mcp_v2::handlers::tool_utils::entity_watermark_from_parts;
    use crate::services::mcp_v2::local_runtime;
    use crate::services::mcp_v2::target_handles::mint_target_handle;

    fn description() -> ToolDescription {
        ToolDescription {
            name: ScopedName::new("dailyos.submit.action"),
            summary: String::new(),
            when_to_call: String::new(),
            when_not_to_call: String::new(),
            side: Side::SubmitCorrection,
            parameters: vec![],
            returns: ReturnSpec {
                schema: ParamSchema(json!({})),
                description: String::new(),
            },
            examples: vec![],
            scopes_required: vec![Scope::new("dailyos.submit.action")],
        }
    }

    fn actor() -> McpActor {
        McpActor::Client {
            client_id: McpClientId::new("client-action-submit-test"),
            conversation_handle: Some(OpaqueConversationHandle::new(
                "conversation-action-submit-test",
            )),
            tool_name: ScopedName::new("dailyos.submit.action"),
            granted_scopes: vec![Scope::new("dailyos.submit.action")],
        }
    }

    fn account(id: &str, name: &str, updated_at: &str, archived: bool) -> DbAccount {
        DbAccount {
            id: id.to_string(),
            name: name.to_string(),
            updated_at: updated_at.to_string(),
            archived,
            ..Default::default()
        }
    }

    fn handler() -> CreateActionHandler {
        CreateActionHandler::new(description(), Arc::new(PropagationEngine::new()))
    }

    #[test]
    fn submit_action_replay_reuses_durable_action_row() {
        local_runtime::with_target_handle_key_for_tests([21_u8; 32], || {
            let dir = tempfile::tempdir().expect("tempdir");
            let db =
                ActionDb::open_at_unencrypted(dir.path().join("submit-action-replay.db")).unwrap();
            let db = Arc::new(Mutex::new(db));
            let ctx = McpHandlerContext::with_owned_connection(Arc::clone(&db));
            let actor = actor();
            let handler = handler();
            let params = json!({ "description": "Follow up on contract review" });

            let first = handler
                .invoke(&ctx, &actor, params.clone())
                .expect("first action response");
            let second = handler
                .invoke(&ctx, &actor, params)
                .expect("second action response");

            assert_eq!(first["status"], "ok");
            assert_eq!(second["status"], "ok");
            let count: i64 = db
                .lock()
                .unwrap()
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM actions WHERE title = ?1",
                    ["Follow up on contract review"],
                    |row| row.get(0),
                )
                .expect("action count");
            assert_eq!(count, 1, "MCP retry must not duplicate durable actions");
            let handle_count: i64 = db
                .lock()
                .unwrap()
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM mcp_target_handles WHERE originating_tool = ?1",
                    ["dailyos.submit.action"],
                    |row| row.get(0),
                )
                .expect("target handle count");
            assert_eq!(
                handle_count, 1,
                "MCP retry must not duplicate durable action handle rows"
            );
        });
    }

    #[test]
    fn submit_action_rejects_archived_entity_handle() {
        local_runtime::with_target_handle_key_for_tests([22_u8; 32], || {
            let dir = tempfile::tempdir().expect("tempdir");
            let db =
                ActionDb::open_at_unencrypted(dir.path().join("submit-action-stale.db")).unwrap();
            let actor = actor();
            let active_updated_at = "2026-06-01T00:00:00Z";
            let active = account("acct-stale", "Stale Account", active_updated_at, false);
            db.upsert_account(&active).expect("seed account");
            let watermark = entity_watermark_from_parts(
                "account",
                &active.id,
                &active.updated_at,
                active.archived,
            );
            let entity_handle = mint_target_handle(
                &db,
                MintTargetHandle {
                    actor: &actor,
                    originating_tool: &ScopedName::new("dailyos.read.account_status"),
                    result_item_path: "/subject",
                    target_kind: TargetKind::Entity,
                    target_ref: json!({
                        "entity_type": "account",
                        "entity_id": active.id,
                    }),
                    sensitivity_tier: "internal",
                    provenance_material: "test-account",
                    watermark_material: &watermark,
                },
            )
            .expect("mint entity handle");
            let mut archived = active;
            archived.archived = true;
            archived.updated_at = Utc::now().to_rfc3339();
            db.upsert_account(&archived).expect("archive account");

            let db = Arc::new(Mutex::new(db));
            let ctx = McpHandlerContext::with_owned_connection(Arc::clone(&db));
            let payload = handler()
                .invoke(
                    &ctx,
                    &actor,
                    json!({
                        "description": "Follow up with archived account",
                        "entity_handle": entity_handle,
                    }),
                )
                .expect("handler response");

            assert_eq!(payload["status"], "unavailable");
            assert_eq!(payload["refresh_required"], true);
            let count: i64 = db
                .lock()
                .unwrap()
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM actions WHERE title = ?1",
                    ["Follow up with archived account"],
                    |row| row.get(0),
                )
                .expect("action count");
            assert_eq!(count, 0, "stale entity handles must not create actions");
        });
    }
}

//! `dailyos.submit.action_status` MCP handler.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};
use crate::services::mcp_v2::handler_context::McpHandlerContext;
use crate::services::mcp_v2::target_handles::{
    resolve_target_handle, resolved_target_watermark_matches, ResolveTargetHandle,
    TargetHandleResolutionError, TargetKind,
};
use crate::signals::propagation::PropagationEngine;

use super::tool_utils::{
    bad_params, mutation_cursor_for_target, reject_raw_id_params, required_str,
    unavailable_payload, validate_bounded_string, validate_yyyy_mm_dd,
};

const SCHEMA_VERSION: &str = "mcp.action_status.v1";

pub struct UpdateActionStatusHandler {
    description: ToolDescription,
    signal_engine: Arc<PropagationEngine>,
}

impl UpdateActionStatusHandler {
    pub fn new(description: ToolDescription, signal_engine: Arc<PropagationEngine>) -> Self {
        Self {
            description,
            signal_engine,
        }
    }
}

impl McpToolHandler for UpdateActionStatusHandler {
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
        let action_handle = required_str(&params, "action_handle")?;
        let semantic_status = required_str(&params, "status")?;
        let reason = super::tool_utils::optional_str(&params, "reason")?;
        if let Some(reason) = reason.as_deref() {
            validate_bounded_string(reason, "reason", 1, 500)?;
        }
        let defer_date = super::tool_utils::optional_str(&params, "defer_date")?;
        match semantic_status.as_str() {
            "deferred" => {
                let Some(date) = defer_date.as_deref() else {
                    return Err(bad_params("`defer_date` is required for deferred"));
                };
                validate_yyyy_mm_dd(date, "defer_date")?;
            }
            "done" | "dropped" => {
                if defer_date.is_some() {
                    return Err(bad_params("`defer_date` is only valid for deferred"));
                }
            }
            other => return Err(bad_params(format!("unsupported status `{other}`"))),
        }

        let result = ctx.with_conn(|db| {
            let resolved = match resolve_target_handle(
                db,
                ResolveTargetHandle {
                    actor,
                    handle: &action_handle,
                    expected_kind: TargetKind::Action,
                    current_watermark_material: None,
                },
            ) {
                Ok(resolved) => resolved,
                Err(TargetHandleResolutionError::Unavailable { refresh_required }) => {
                    return Ok(unavailable_payload(
                        SCHEMA_VERSION,
                        &self.description.name,
                        refresh_required,
                        Some(&action_handle),
                    ));
                }
                Err(TargetHandleResolutionError::Internal(detail)) => {
                    return Err(ToolError::Internal {
                        trace_id: format!("mcp_action_handle_resolve:{detail}"),
                    });
                }
            };
            let action_id = resolved
                .target_ref
                .get("action_id")
                .and_then(Value::as_str)
                .ok_or_else(|| bad_params("action handle is unavailable"))?
                .to_string();
            let Some(action) =
                db.get_action_by_id(&action_id)
                    .map_err(|error| ToolError::UpstreamFailure {
                        detail: error.to_string(),
                    })?
            else {
                return Ok(unavailable_payload(
                    SCHEMA_VERSION,
                    &self.description.name,
                    false,
                    Some(&action_handle),
                ));
            };
            let current_watermark = action_watermark_for_handle(&action);
            if !resolved_target_watermark_matches(&resolved, &current_watermark).map_err(
                |error| ToolError::Internal {
                    trace_id: format!("mcp_action_handle_watermark:{error}"),
                },
            )? {
                return Ok(unavailable_payload(
                    SCHEMA_VERSION,
                    &self.description.name,
                    true,
                    Some(&action_handle),
                ));
            }

            let clock = SystemClock;
            let rng = SystemRng;
            let external = ExternalClients::default();
            let service_ctx =
                ServiceContext::new_live(&clock, &rng, &external).with_actor("mcp_client");
            let (status, stored_status) = match semantic_status.as_str() {
                "done" if action.status == crate::action_status::COMPLETED => {
                    ("already_current", action.status)
                }
                "done"
                    if matches!(
                        action.status.as_str(),
                        crate::action_status::UNSTARTED | crate::action_status::STARTED
                    ) =>
                {
                    let Some(stored) = crate::services::actions::complete_action_for_mcp(
                        &service_ctx,
                        db,
                        &self.signal_engine,
                        &action_id,
                    )
                    .map_err(|error| ToolError::UpstreamFailure { detail: error })?
                    else {
                        return Ok(unavailable_payload(
                            SCHEMA_VERSION,
                            &self.description.name,
                            false,
                            Some(&action_handle),
                        ));
                    };
                    ("ok", stored)
                }
                "done" => {
                    return Ok(unavailable_payload(
                        SCHEMA_VERSION,
                        &self.description.name,
                        false,
                        Some(&action_handle),
                    ));
                }
                "deferred"
                    if !matches!(
                        action.status.as_str(),
                        crate::action_status::UNSTARTED | crate::action_status::STARTED
                    ) =>
                {
                    return Ok(unavailable_payload(
                        SCHEMA_VERSION,
                        &self.description.name,
                        false,
                        Some(&action_handle),
                    ));
                }
                "deferred" => {
                    let Some(stored) = crate::services::actions::defer_action_for_mcp(
                        &service_ctx,
                        db,
                        &self.signal_engine,
                        &action_id,
                        defer_date.as_deref().expect("validated"),
                        reason.as_deref(),
                    )
                    .map_err(|error| ToolError::UpstreamFailure { detail: error })?
                    else {
                        return Ok(unavailable_payload(
                            SCHEMA_VERSION,
                            &self.description.name,
                            false,
                            Some(&action_handle),
                        ));
                    };
                    ("ok", stored)
                }
                "dropped"
                    if matches!(
                        action.status.as_str(),
                        crate::action_status::CANCELLED | crate::action_status::ARCHIVED
                    ) =>
                {
                    ("already_current", action.status)
                }
                "dropped"
                    if !matches!(
                        action.status.as_str(),
                        crate::action_status::BACKLOG
                            | crate::action_status::UNSTARTED
                            | crate::action_status::STARTED
                    ) =>
                {
                    return Ok(unavailable_payload(
                        SCHEMA_VERSION,
                        &self.description.name,
                        false,
                        Some(&action_handle),
                    ));
                }
                "dropped" => {
                    let Some(stored) = crate::services::actions::drop_action_for_mcp(
                        &service_ctx,
                        db,
                        &self.signal_engine,
                        &action_id,
                        reason.as_deref(),
                    )
                    .map_err(|error| ToolError::UpstreamFailure { detail: error })?
                    else {
                        return Ok(unavailable_payload(
                            SCHEMA_VERSION,
                            &self.description.name,
                            false,
                            Some(&action_handle),
                        ));
                    };
                    ("ok", stored)
                }
                _ => {
                    return Ok(unavailable_payload(
                        SCHEMA_VERSION,
                        &self.description.name,
                        false,
                        Some(&action_handle),
                    ));
                }
            };

            Ok(json!({
                "schema_version": SCHEMA_VERSION,
                "tool_name": self.description.name.as_str(),
                "status": status,
                "semantic_status": semantic_status,
                "stored_status": stored_status,
                "mutation_cursor": mutation_cursor_for_target(
                    &self.description.name,
                    status,
                    "action",
                    &action_handle,
                ),
            }))
        });
        result.unwrap_or_else(|| {
            Err(ToolError::Internal {
                trace_id: "mcp_action_status_missing_owned_connection".to_string(),
            })
        })
    }
}

fn action_watermark_for_handle(action: &crate::db::DbAction) -> String {
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
    use crate::db::{ActionDb, DbAction};
    use crate::services::mcp_v2::contracts::{
        McpClientId, OpaqueConversationHandle, ParamSchema, ReturnSpec, Scope, ScopedName, Side,
    };
    use crate::services::mcp_v2::local_runtime;
    use crate::services::mcp_v2::target_handles::{mint_target_handle, MintTargetHandle};

    fn description() -> ToolDescription {
        ToolDescription {
            name: ScopedName::new("dailyos.submit.action_status"),
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
            scopes_required: vec![Scope::new("dailyos.submit.action_status")],
        }
    }

    fn actor() -> McpActor {
        McpActor::Client {
            client_id: McpClientId::new("client-action-status-test"),
            conversation_handle: Some(OpaqueConversationHandle::new(
                "conversation-action-status-test",
            )),
            tool_name: ScopedName::new("dailyos.submit.action_status"),
            granted_scopes: vec![Scope::new("dailyos.submit.action_status")],
        }
    }

    fn action(id: &str, status: &str) -> DbAction {
        let now = Utc::now().to_rfc3339();
        DbAction {
            id: id.to_string(),
            title: "Follow up".to_string(),
            priority: crate::action_status::PRIORITY_MEDIUM,
            status: status.to_string(),
            created_at: now.clone(),
            due_date: None,
            completed_at: (status == crate::action_status::COMPLETED).then(|| now.clone()),
            account_id: None,
            project_id: None,
            source_type: None,
            source_id: None,
            source_label: None,
            action_kind: crate::action_status::KIND_TASK.to_string(),
            commitment_id: None,
            owner_raw: None,
            owner_entity_id: None,
            owner_confidence: None,
            owner_source: None,
            trust_score: None,
            trust_band: None,
            commitment_source_count: None,
            context: None,
            waiting_on: None,
            updated_at: now,
            person_id: None,
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
            needs_decision: false,
            decision_owner: None,
            decision_stakes: None,
            linear_identifier: None,
            linear_url: None,
        }
    }

    #[test]
    fn completed_action_defer_returns_unavailable_payload_with_cursor() {
        local_runtime::with_target_handle_key_for_tests([11_u8; 32], || {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("action-status-unavailable.db");
            let db = ActionDb::open_at_unencrypted(path).expect("open db");
            let actor = actor();
            let description = description();
            let action = action("action-completed", crate::action_status::COMPLETED);
            db.upsert_action(&action).expect("seed action");
            let watermark = action_watermark_for_handle(&action);
            let handle = mint_target_handle(
                &db,
                MintTargetHandle {
                    actor: &actor,
                    originating_tool: &description.name,
                    result_item_path: "/action",
                    target_kind: TargetKind::Action,
                    target_ref: json!({ "action_id": action.id }),
                    sensitivity_tier: "internal",
                    provenance_material: "test-action",
                    watermark_material: &watermark,
                },
            )
            .expect("mint action handle");
            let handler =
                UpdateActionStatusHandler::new(description, Arc::new(PropagationEngine::new()));
            let ctx = McpHandlerContext::with_owned_connection(Arc::new(Mutex::new(db)));

            let payload = handler
                .invoke(
                    &ctx,
                    &actor,
                    json!({
                        "action_handle": handle,
                        "status": "deferred",
                        "defer_date": "2026-06-08"
                    }),
                )
                .expect("handler response");

            assert_eq!(payload["status"], "unavailable");
            assert_eq!(payload["reason"], "target_unavailable");
            assert_eq!(payload["refresh_required"], false);
            assert!(payload["mutation_cursor"].is_object());
        });
    }
}

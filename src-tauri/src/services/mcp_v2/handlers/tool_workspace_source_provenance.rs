//! `dailyos.read.workspace_source_provenance` MCP handler.

use serde_json::{json, Value};

use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};
use crate::services::mcp_v2::handler_context::McpHandlerContext;
use crate::services::mcp_v2::target_handles::{
    resolve_target_handle, resolved_target_watermark_matches, ResolveTargetHandle,
    TargetHandleResolutionError, TargetKind,
};

use super::tool_utils::{
    internal_trace, reject_raw_id_params, required_str, source_provenance_watermark,
    unavailable_payload,
};

const SCHEMA_VERSION: &str = "mcp.workspace_source_provenance.v1";

pub struct WorkspaceSourceProvenanceHandler {
    description: ToolDescription,
}

impl WorkspaceSourceProvenanceHandler {
    pub fn new(description: ToolDescription) -> Self {
        Self { description }
    }
}

impl McpToolHandler for WorkspaceSourceProvenanceHandler {
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
        let handle = required_str(&params, "source_provenance_handle")?;
        let result = ctx.with_conn(|db| {
            let resolved = match resolve_target_handle(
                db,
                ResolveTargetHandle {
                    actor,
                    handle: &handle,
                    expected_kind: TargetKind::SourceProvenance,
                    current_watermark_material: None,
                },
            ) {
                Ok(resolved) => resolved,
                Err(TargetHandleResolutionError::Unavailable { refresh_required }) => {
                    return Ok(unavailable_payload(
                        SCHEMA_VERSION,
                        &self.description.name,
                        refresh_required,
                        None,
                    ));
                }
                Err(TargetHandleResolutionError::Internal(detail)) => {
                    return Err(internal_trace("mcp_source_handle_resolve", detail));
                }
            };
            let current_source_watermark = source_provenance_watermark(&resolved.target_ref);
            if !resolved_target_watermark_matches(&resolved, &current_source_watermark)
                .map_err(|error| internal_trace("mcp_source_handle_watermark", error))?
            {
                return Ok(unavailable_payload(
                    SCHEMA_VERSION,
                    &self.description.name,
                    true,
                    None,
                ));
            }
            let mut source = json!({
                "label": resolved.target_ref.get("label").and_then(Value::as_str).unwrap_or("DailyOS source"),
                "source_type": resolved.target_ref.get("source_type").and_then(Value::as_str).unwrap_or("source"),
                "sourceType": resolved.target_ref.get("source_type").and_then(Value::as_str).unwrap_or("source"),
                "source_asof": resolved.target_ref.get("source_asof").cloned().unwrap_or(Value::Null),
                "sourceAsof": resolved.target_ref.get("source_asof").cloned().unwrap_or(Value::Null),
                "trust_band": resolved.target_ref.get("trust_band").cloned().unwrap_or(Value::Null),
                "trustBand": resolved.target_ref.get("trust_band").cloned().unwrap_or(Value::Null),
                "redaction_applied": resolved.target_ref.get("redaction_applied").and_then(Value::as_bool).unwrap_or(true),
                "redactionApplied": resolved.target_ref.get("redaction_applied").and_then(Value::as_bool).unwrap_or(true),
            });
            if let Some(object) = source.as_object_mut() {
                if let Some(workspace_file_kind) = resolved
                    .target_ref
                    .get("workspace_file_kind")
                    .or_else(|| resolved.target_ref.get("workspaceFileKind"))
                    .and_then(Value::as_str)
                {
                    object.insert(
                        "workspace_file_kind".to_string(),
                        Value::String(workspace_file_kind.to_string()),
                    );
                    object.insert(
                        "workspaceFileKind".to_string(),
                        Value::String(workspace_file_kind.to_string()),
                    );
                }
                object.remove("source_id");
                object.remove("path");
                object.remove("file_path");
            }
            Ok(json!({
                "schema_version": SCHEMA_VERSION,
                "tool_name": self.description.name.as_str(),
                "status": "ok",
                "source": source,
                "provenance": {
                    "render_policy_version": resolved.render_policy_version,
                    "sensitivity_tier": resolved.sensitivity_tier,
                    "raw_source_ids_included": false,
                    "raw_paths_included": false,
                },
            }))
        });
        result.unwrap_or_else(|| {
            Err(ToolError::Internal {
                trace_id: "mcp_source_provenance_missing_owned_connection".to_string(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use serde_json::json;

    use super::*;
    use crate::db::ActionDb;
    use crate::services::mcp_v2::contracts::{
        McpClientId, OpaqueConversationHandle, ParamSchema, ReturnSpec, Scope, ScopedName, Side,
    };
    use crate::services::mcp_v2::local_runtime;
    use crate::services::mcp_v2::target_handles::{mint_target_handle, MintTargetHandle};

    fn description() -> ToolDescription {
        ToolDescription {
            name: ScopedName::new("dailyos.read.workspace_source_provenance"),
            summary: String::new(),
            when_to_call: String::new(),
            when_not_to_call: String::new(),
            side: Side::Read,
            parameters: vec![],
            returns: ReturnSpec {
                schema: ParamSchema(json!({})),
                description: String::new(),
            },
            examples: vec![],
            scopes_required: vec![Scope::new("dailyos.read.workspace_source_provenance")],
        }
    }

    fn actor() -> McpActor {
        McpActor::Client {
            client_id: McpClientId::new("client-source-provenance-test"),
            conversation_handle: Some(OpaqueConversationHandle::new(
                "conversation-source-provenance-test",
            )),
            tool_name: ScopedName::new("dailyos.read.workspace_source_provenance"),
            granted_scopes: vec![Scope::new("dailyos.read.workspace_source_provenance")],
        }
    }

    #[test]
    fn source_provenance_rejects_stale_watermark_before_display() {
        local_runtime::with_target_handle_key_for_tests([41_u8; 32], || {
            let dir = tempfile::tempdir().expect("tempdir");
            let db = ActionDb::open_at_unencrypted(dir.path().join("source-provenance-stale.db"))
                .expect("open db");
            let actor = actor();
            let target_ref = json!({
                "label": "DailyOS source",
                "source_type": "claim",
                "source_asof": "2026-06-01T00:00:00Z",
                "trust_band": "likely_current",
                "redaction_applied": true,
            });
            let handle = mint_target_handle(
                &db,
                MintTargetHandle {
                    actor: &actor,
                    originating_tool: &ScopedName::new("dailyos.read.account_status"),
                    result_item_path: "/provenance/sources/0",
                    target_kind: TargetKind::SourceProvenance,
                    target_ref,
                    sensitivity_tier: "internal",
                    provenance_material: "legacy-source-material",
                    watermark_material: "legacy-source-material",
                },
            )
            .expect("mint source handle");
            let db = Arc::new(Mutex::new(db));
            let ctx = McpHandlerContext::with_owned_connection(db);
            let payload = WorkspaceSourceProvenanceHandler::new(description())
                .invoke(&ctx, &actor, json!({ "source_provenance_handle": handle }))
                .expect("handler response");

            assert_eq!(payload["status"], "unavailable");
            assert_eq!(payload["refresh_required"], true);
        });
    }

    #[test]
    fn source_provenance_preserves_workspace_file_kind() {
        local_runtime::with_target_handle_key_for_tests([42_u8; 32], || {
            let dir = tempfile::tempdir().expect("tempdir");
            let db = ActionDb::open_at_unencrypted(dir.path().join("source-provenance-kind.db"))
                .expect("open db");
            let actor = actor();
            let target_ref = json!({
                "label": "Workspace file (Quill transcript)",
                "source_type": "workspace_file",
                "source_asof": "2026-06-01T00:00:00Z",
                "trust_band": "needs_verification",
                "redaction_applied": true,
                "workspace_file_kind": "quill_transcript",
                "workspaceFileKind": "quill_transcript",
            });
            let watermark = source_provenance_watermark(&target_ref);
            let handle = mint_target_handle(
                &db,
                MintTargetHandle {
                    actor: &actor,
                    originating_tool: &ScopedName::new("dailyos.read.account_status"),
                    result_item_path: "/provenance/sources/0",
                    target_kind: TargetKind::SourceProvenance,
                    target_ref,
                    sensitivity_tier: "internal",
                    provenance_material: &watermark,
                    watermark_material: &watermark,
                },
            )
            .expect("mint source handle");
            let db = Arc::new(Mutex::new(db));
            let ctx = McpHandlerContext::with_owned_connection(db);
            let payload = WorkspaceSourceProvenanceHandler::new(description())
                .invoke(&ctx, &actor, json!({ "source_provenance_handle": handle }))
                .expect("handler response");

            assert_eq!(payload["status"], "ok");
            assert_eq!(
                payload["source"]["workspace_file_kind"],
                json!("quill_transcript")
            );
            assert_eq!(
                payload["source"]["workspaceFileKind"],
                json!("quill_transcript")
            );
            assert!(payload["source"].get("path").is_none());
            assert!(payload["source"].get("source_id").is_none());
        });
    }
}

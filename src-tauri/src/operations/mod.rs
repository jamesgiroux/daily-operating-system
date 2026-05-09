//! Contract-first operations registry.
//!
//! Phase 1 keeps abilities as the single execution source while publishing a
//! kebab-case operation contract for external surfaces.
#![allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::abilities::provenance::{
    build_ownership_policy_for_invocation, validate_serialized_subject_ownership,
};
use crate::abilities::{AbilityRegistry, Actor};
use crate::bridges::tauri::{TauriAbilityBridge, TauriInvokeContext};
use crate::bridges::{BridgeSurface, BridgeSurfaceError, ConfirmationToken};
use crate::state::AppState;

pub type OperationFuture = Pin<Box<dyn Future<Output = OperationResult> + Send>>;
pub type OperationResult = Result<serde_json::Value, BridgeSurfaceError>;
pub type OperationExecutor = fn(OperationInvocation) -> OperationFuture;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum OperationCategory {
    Read,
    Transform,
    Publish,
    Maintenance,
}

#[derive(Clone, Copy)]
pub struct OperationDef {
    pub name: &'static str,
    pub description: &'static str,
    pub remote: bool,
    pub category: OperationCategory,
    pub input_schema: &'static str,
    pub output_schema: &'static str,
    pub requires_scope: Option<&'static str>,
    pub executor: OperationExecutor,
}

#[derive(Clone)]
pub struct OperationInvocation {
    pub state: Arc<AppState>,
    pub actor: Actor,
    pub surface: BridgeSurface,
    pub input_json: serde_json::Value,
    pub dry_run: bool,
    pub confirmation: Option<ConfirmationToken>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpOperationTool {
    pub name: String,
    pub description: String,
    pub category: OperationCategory,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub requires_scope: Option<String>,
}

macro_rules! operation_def {
    (
        name: $name:literal,
        description: $description:literal,
        remote: $remote:expr,
        category: $category:ident,
        input_schema: $input_schema:expr,
        output_schema: $output_schema:expr,
        requires_scope: $requires_scope:expr,
        executor: $executor:path $(,)?
    ) => {{
        const _: OperationExecutor = $executor;
        OperationDef {
            name: $name,
            description: $description,
            remote: $remote,
            category: OperationCategory::$category,
            input_schema: $input_schema,
            output_schema: $output_schema,
            requires_scope: $requires_scope,
            executor: $executor,
        }
    }};
}

pub const OPERATIONS: &[OperationDef] = &[
    operation_def! {
        name: "get-entity-context",
        description: "Read claim-backed context entries for an account, project, person, or meeting.",
        remote: true,
        category: Read,
        input_schema: include_str!("schemas/get-entity-context.input.schema.json"),
        output_schema: include_str!("schemas/get-entity-context.output.schema.json"),
        requires_scope: Some("entity:read"),
        executor: read_get_entity_context_executor,
    },
    operation_def! {
        name: "internal-debug-dump",
        description: "Return local diagnostic counts for the operation registry and app runtime.",
        remote: false,
        category: Maintenance,
        input_schema: include_str!("schemas/internal-debug-dump.input.schema.json"),
        output_schema: include_str!("schemas/internal-debug-dump.output.schema.json"),
        requires_scope: None,
        executor: maintenance_internal_debug_dump_executor,
    },
];

pub fn operation_by_name(name: &str) -> Option<&'static OperationDef> {
    OPERATIONS.iter().find(|operation| operation.name == name)
}

/// Returns the operation-contract MCP tool view for remote-capable operations.
///
/// Live MCP server discovery still uses the ability bridge descriptor list.
/// Wiring `src-tauri/src/mcp/main.rs` to this contract helper is intentionally
/// out of scope; tracked as a maintenance follow-up.
pub fn mcp_tool_list() -> Vec<McpOperationTool> {
    OPERATIONS
        .iter()
        .filter(|operation| operation.remote)
        .map(|operation| McpOperationTool {
            name: operation.name.to_string(),
            description: operation.description.to_string(),
            category: operation.category,
            input_schema: parse_schema(operation.name, "input", operation.input_schema),
            output_schema: parse_schema(operation.name, "output", operation.output_schema),
            requires_scope: operation.requires_scope.map(str::to_string),
        })
        .collect()
}

#[tauri::command]
pub async fn invoke_operation(
    state: State<'_, Arc<AppState>>,
    operation_name: String,
    input_json: serde_json::Value,
    dry_run: Option<bool>,
    confirmation: Option<ConfirmationToken>,
) -> OperationResult {
    if state.lock_state.lock().is_locked {
        return Err(BridgeSurfaceError::AbilityUnavailable);
    }

    invoke_operation_for_surface(
        state.inner().clone(),
        operation_name,
        input_json,
        dry_run.unwrap_or(false),
        confirmation,
        Actor::User,
        BridgeSurface::TauriApp,
    )
    .await
}

pub async fn invoke_operation_for_surface(
    state: Arc<AppState>,
    operation_name: String,
    input_json: Value,
    dry_run: bool,
    confirmation: Option<ConfirmationToken>,
    actor: Actor,
    surface: BridgeSurface,
) -> OperationResult {
    let operation =
        operation_by_name(&operation_name).ok_or(BridgeSurfaceError::AbilityUnavailable)?;
    if !operation.remote && (is_remote_bound_surface(surface) || actor == Actor::Agent) {
        return Err(BridgeSurfaceError::Validation(format!(
            "operation {} is not remote-invokable",
            operation.name
        )));
    }
    invoke_operation_with_def(
        state,
        operation,
        input_json,
        dry_run,
        confirmation,
        actor,
        surface,
    )
    .await
}

fn is_remote_bound_surface(surface: BridgeSurface) -> bool {
    match surface {
        BridgeSurface::McpTool | BridgeSurface::McpToolDetail => true,
        BridgeSurface::TauriApp | BridgeSurface::Worker | BridgeSurface::Eval => false,
    }
}

async fn invoke_operation_with_def(
    state: Arc<AppState>,
    operation: &OperationDef,
    input_json: Value,
    dry_run: bool,
    confirmation: Option<ConfirmationToken>,
    actor: Actor,
    surface: BridgeSurface,
) -> OperationResult {
    let input_for_policy = input_json.clone();
    let response = (operation.executor)(OperationInvocation {
        state,
        actor,
        surface,
        input_json,
        dry_run,
        confirmation,
    })
    .await?;
    validate_operation_response_ownership(actor, &input_for_policy, &response)?;
    Ok(response)
}

fn validate_operation_response_ownership(
    actor: Actor,
    input_json: &Value,
    response: &Value,
) -> Result<(), BridgeSurfaceError> {
    let Some(ability_name) = response.get("ability_name").and_then(Value::as_str) else {
        return Ok(());
    };
    let data = response
        .get("data")
        .cloned()
        .ok_or(BridgeSurfaceError::AbilityUnavailable)?;
    let rendered_provenance = response
        .get("rendered_provenance")
        .ok_or(BridgeSurfaceError::AbilityUnavailable)?;
    let provenance = rendered_provenance
        .get("value")
        .unwrap_or(rendered_provenance)
        .clone();
    let diagnostics = response
        .get("diagnostics")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "warnings": [] }));

    let registry =
        AbilityRegistry::global_checked().map_err(|_| BridgeSurfaceError::AbilityUnavailable)?;
    let ability_meta = registry
        .iter_for(actor)
        .find(|descriptor| descriptor.name == ability_name)
        .ok_or(BridgeSurfaceError::AbilityUnavailable)?;
    let policy = build_ownership_policy_for_invocation(ability_meta, input_json, &provenance)?;
    validate_serialized_subject_ownership(data, provenance, diagnostics, &[], policy)?;
    Ok(())
}

fn read_get_entity_context_executor(invocation: OperationInvocation) -> OperationFuture {
    Box::pin(async move {
        let registry = AbilityRegistry::global_checked()
            .map_err(|_| BridgeSurfaceError::AbilityUnavailable)?;
        let response = TauriAbilityBridge::new(registry)
            .invoke(
                invocation.state.as_ref(),
                "get_entity_context",
                invocation.input_json,
                TauriInvokeContext::new(
                    invocation.actor,
                    invocation.surface,
                    invocation.dry_run,
                    invocation.confirmation.as_ref(),
                ),
            )
            .await?;
        serde_json::to_value(response).map_err(|_| BridgeSurfaceError::AbilityUnavailable)
    })
}

fn maintenance_internal_debug_dump_executor(invocation: OperationInvocation) -> OperationFuture {
    Box::pin(async move {
        let snapshot = invocation.state.context_snapshot();
        let data_summary = invocation
            .state
            .db_read(crate::privacy::get_data_summary)
            .await
            .ok();
        let latency_rollups = serde_json::to_value(crate::latency::get_rollups())
            .map_err(|_| BridgeSurfaceError::AbilityUnavailable)?;

        Ok(serde_json::json!({
            "schemaVersion": 1,
            "operationCount": OPERATIONS.len(),
            "remoteOperationCount": OPERATIONS.iter().filter(|operation| operation.remote).count(),
            "contextProvider": snapshot.provider_name(),
            "contextRemote": snapshot.is_remote(),
            "dataSummary": data_summary,
            "latencyRollups": latency_rollups,
        }))
    })
}

fn parse_schema(operation: &str, side: &str, schema: &str) -> serde_json::Value {
    serde_json::from_str(schema).unwrap_or_else(|error| {
        panic!("invalid {side} schema for operation `{operation}`: {error}");
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    use crate::abilities::provenance::{
        DataSource, EntityId, FieldAttribution, FieldPath, ProvenanceBuilder,
        ProvenanceBuilderConfig, SourceAttribution, SourceIdentifier, SubjectAttribution,
        SubjectRef,
    };

    const TEST_SCHEMA: &str = "{}";

    #[test]
    fn operation_registry_contract_mcp_tool_list_excludes_remote_false_operations() {
        let tools = mcp_tool_list();
        let names = tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>();

        assert!(names.contains(&"get-entity-context"));
        assert!(!names.contains(&"internal-debug-dump"));

        for tool in tools {
            let operation = operation_by_name(&tool.name).expect("tool maps back to operation");
            assert!(operation.remote);
            assert_eq!(tool.description, operation.description);
            assert_eq!(tool.category, operation.category);
            assert!(tool.input_schema.is_object());
            assert!(tool.output_schema.is_object());
        }
    }

    #[test]
    #[ignore = "maintenance follow-up: wire live MCP discovery to operations::mcp_tool_list()"]
    fn mcp_discovery_wiring_uses_operations_contract_follow_up() {}

    #[test]
    fn operations_use_kebab_case_names() {
        for operation in OPERATIONS {
            assert!(
                operation
                    .name
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'),
                "operation `{}` must be kebab-case",
                operation.name
            );
            assert!(operation.name.contains('-'));
        }
    }

    #[tokio::test]
    async fn invoke_operation_rejects_get_entity_context_cross_subject_response() {
        let operation = OperationDef {
            name: "get-entity-context",
            description: "test get entity context operation",
            remote: true,
            category: OperationCategory::Read,
            input_schema: TEST_SCHEMA,
            output_schema: TEST_SCHEMA,
            requires_scope: Some("entity:read"),
            executor: cross_subject_get_entity_context_executor,
        };
        let input_json = json!({
            "schema_version": 1,
            "entity_type": "account",
            "entity_id": "acct-target",
            "depth": "standard",
        });

        let err = invoke_operation_with_def(
            Arc::new(AppState::new()),
            &operation,
            input_json,
            false,
            None,
            Actor::User,
            BridgeSurface::TauriApp,
        )
        .await
        .expect_err("ownership validation must reject the serialized response");

        assert!(
            matches!(err, BridgeSurfaceError::Ownership(_)),
            "expected ownership error, got {err:?}"
        );
    }

    #[tokio::test]
    async fn invoke_operation_with_agent_mcp_context_returns_actor_filtered_entity_context() {
        let operation = OperationDef {
            name: "get-entity-context",
            description: "test actor-filtered get entity context operation",
            remote: true,
            category: OperationCategory::Read,
            input_schema: TEST_SCHEMA,
            output_schema: TEST_SCHEMA,
            requires_scope: Some("entity:read"),
            executor: actor_filtered_get_entity_context_executor,
        };
        let input_json = json!({
            "schema_version": 1,
            "entity_type": "account",
            "entity_id": "acct-target",
            "depth": "standard",
        });

        let user_response = invoke_operation_with_def(
            Arc::new(AppState::new()),
            &operation,
            input_json.clone(),
            false,
            None,
            Actor::User,
            BridgeSurface::TauriApp,
        )
        .await
        .expect("user operation response passes ownership validation");
        let agent_response = invoke_operation_with_def(
            Arc::new(AppState::new()),
            &operation,
            input_json,
            false,
            None,
            Actor::Agent,
            BridgeSurface::McpTool,
        )
        .await
        .expect("agent MCP operation response passes ownership validation");

        let user_ids = response_entry_ids(&user_response);
        let agent_ids = response_entry_ids(&agent_response);
        assert_eq!(
            user_ids,
            vec!["claim-agent-readable", "claim-user-only"],
            "User/Tauri view should keep the full response"
        );
        assert_eq!(
            agent_ids,
            vec!["claim-agent-readable"],
            "Agent/MCP view must use the actor-filtered response"
        );
        assert_eq!(
            agent_response["rendered_provenance"]["surface"],
            json!("mcp_tool")
        );
    }

    #[tokio::test]
    async fn invoke_operation_rejects_remote_false_for_mcp_or_agent() {
        for (actor, surface) in [
            (Actor::User, BridgeSurface::McpTool),
            (Actor::Agent, BridgeSurface::TauriApp),
            (Actor::Agent, BridgeSurface::McpTool),
        ] {
            let err = invoke_operation_for_surface(
                Arc::new(AppState::new()),
                "internal-debug-dump".to_string(),
                json!({}),
                false,
                None,
                actor,
                surface,
            )
            .await
            .expect_err("remote=false operation should be rejected for MCP or agent callers");

            assert_eq!(
                err,
                BridgeSurfaceError::Validation(
                    "operation internal-debug-dump is not remote-invokable".to_string()
                )
            );
        }
    }

    #[tokio::test]
    async fn invoke_operation_rejects_remote_false_for_mcp_tool_detail() {
        let err = invoke_operation_for_surface(
            Arc::new(AppState::new()),
            "internal-debug-dump".to_string(),
            json!({}),
            false,
            None,
            Actor::User,
            BridgeSurface::McpToolDetail,
        )
        .await
        .expect_err("remote=false operation should be rejected for MCP detail callers");

        assert_eq!(
            err,
            BridgeSurfaceError::Validation(
                "operation internal-debug-dump is not remote-invokable".to_string()
            )
        );
    }

    #[tokio::test]
    async fn invoke_operation_allows_remote_false_for_tauri_user() {
        let response = invoke_operation_for_surface(
            Arc::new(AppState::new()),
            "internal-debug-dump".to_string(),
            json!({}),
            false,
            None,
            Actor::User,
            BridgeSurface::TauriApp,
        )
        .await
        .expect("Tauri/User callers may invoke local-only operations");

        assert_eq!(response["schemaVersion"], json!(1));
        assert_eq!(response["remoteOperationCount"], json!(1));
        assert_eq!(response["operationCount"], json!(OPERATIONS.len()));
    }

    fn cross_subject_get_entity_context_executor(
        _invocation: OperationInvocation,
    ) -> OperationFuture {
        Box::pin(async { Ok(cross_subject_get_entity_context_response()) })
    }

    fn actor_filtered_get_entity_context_executor(
        invocation: OperationInvocation,
    ) -> OperationFuture {
        Box::pin(async move {
            Ok(actor_filtered_get_entity_context_response(
                invocation.actor,
                invocation.surface,
            ))
        })
    }

    fn response_entry_ids(response: &Value) -> Vec<&str> {
        response["data"]
            .as_array()
            .expect("response data is an array")
            .iter()
            .map(|entry| {
                entry["id"]
                    .as_str()
                    .expect("entity context entry has string id")
            })
            .collect()
    }

    fn actor_filtered_get_entity_context_response(actor: Actor, surface: BridgeSurface) -> Value {
        let produced_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
        let subject =
            SubjectAttribution::direct_confident(SubjectRef::Account("acct-target".into()));
        let mut entries = vec![json!({
            "id": "claim-agent-readable",
            "entityType": "account",
            "entityId": "acct-target",
            "title": "risk: renewal",
            "content": "Agent-readable renewal risk.",
            "createdAt": "2026-05-06T12:00:00Z",
            "updatedAt": "2026-05-06T12:00:00Z",
        })];
        if actor != Actor::Agent {
            entries.push(json!({
                "id": "claim-user-only",
                "entityType": "account",
                "entityId": "acct-target",
                "title": "user_note: negotiation",
                "content": "User-only negotiation note.",
                "createdAt": "2026-05-06T12:00:00Z",
                "updatedAt": "2026-05-06T12:00:00Z",
            }));
        }

        let mut builder = ProvenanceBuilder::new(ProvenanceBuilderConfig::new(
            "get_entity_context",
            produced_at,
        ));
        builder.set_subject(subject.clone());
        for index in 0..entries.len() {
            let source_index = builder.add_source(
                SourceAttribution::new(
                    DataSource::User,
                    vec![SourceIdentifier::Entity {
                        entity_id: EntityId::new("acct-target"),
                        field: Some("claim".to_string()),
                    }],
                    produced_at,
                    Some(produced_at),
                    1.0,
                    None,
                )
                .expect("source attribution builds"),
            );
            builder
                .attribute_subtree(
                    FieldPath::new(format!("/{index}")).expect("field path builds"),
                    FieldAttribution::direct(subject.clone(), source_index),
                )
                .expect("field attribution records");
        }
        let output = builder
            .finalize(Value::Array(entries))
            .expect("provenance output finalizes");
        let output = serde_json::to_value(output).expect("output serializes");

        json!({
            "invocation_id": output["provenance"]["invocation_id"].clone(),
            "ability_name": "get_entity_context",
            "ability_version": "1.0.0",
            "schema_version": 1,
            "data": output["data"].clone(),
            "rendered_provenance": {
                "surface": surface,
                "value": output["provenance"].clone(),
            },
            "diagnostics": output["diagnostics"].clone(),
        })
    }

    fn cross_subject_get_entity_context_response() -> Value {
        let produced_at = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
        let subject =
            SubjectAttribution::direct_confident(SubjectRef::Account("acct-target".into()));
        let mut builder = ProvenanceBuilder::new(ProvenanceBuilderConfig::new(
            "get_entity_context",
            produced_at,
        ));
        builder.set_subject(subject.clone());
        let source_index = builder.add_source(
            SourceAttribution::new(
                DataSource::User,
                vec![SourceIdentifier::Entity {
                    entity_id: EntityId::new("acct-other"),
                    field: Some("claim".to_string()),
                }],
                produced_at,
                Some(produced_at),
                1.0,
                None,
            )
            .expect("source attribution builds"),
        );
        builder
            .attribute_subtree(
                FieldPath::new("/0").expect("field path builds"),
                FieldAttribution::direct(subject, source_index),
            )
            .expect("field attribution records");
        let output = builder
            .finalize(json!([{
                "id": "claim-cross-subject",
                "entityType": "account",
                "entityId": "acct-other",
                "title": "account: renewal",
                "content": "Other account renewal risk must not render for the target account.",
                "createdAt": "2026-05-06T12:00:00Z",
                "updatedAt": "2026-05-06T12:00:00Z",
            }]))
            .expect("provenance output finalizes");
        let output = serde_json::to_value(output).expect("output serializes");

        json!({
            "invocation_id": output["provenance"]["invocation_id"].clone(),
            "ability_name": "get_entity_context",
            "ability_version": "1.0.0",
            "schema_version": 1,
            "data": output["data"].clone(),
            "rendered_provenance": {
                "surface": "tauri_app",
                "value": output["provenance"].clone(),
            },
            "diagnostics": output["diagnostics"].clone(),
        })
    }
}

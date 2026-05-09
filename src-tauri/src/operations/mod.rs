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
use tauri::State;

use crate::abilities::AbilityRegistry;
use crate::bridges::tauri::TauriAbilityBridge;
use crate::bridges::{BridgeSurfaceError, ConfirmationToken};
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

    let operation =
        operation_by_name(&operation_name).ok_or(BridgeSurfaceError::AbilityUnavailable)?;
    (operation.executor)(OperationInvocation {
        state: state.inner().clone(),
        input_json,
        dry_run: dry_run.unwrap_or(false),
        confirmation,
    })
    .await
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
                invocation.dry_run,
                invocation.confirmation.as_ref(),
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

    #[test]
    fn operation_registry_round_trip_mcp_tool_list_excludes_remote_false_operations() {
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
}

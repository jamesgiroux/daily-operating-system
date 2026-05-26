//! `dailyos.search.workspace_memory` MCP tool handler.
//!
//! This is the v1.4.7 pass-through exception: hosts get the frozen
//! `WorkspaceGraphResponse v1` shape from the v1.4.5 workspace substrate. The
//! handler only adapts MCP parameters, applies MCP scope-driven redaction for
//! entity names, and delegates to the workspace graph read service.

use abilities_runtime::abilities::workspace_graph::contracts::{
    WorkspaceGraphInput, WorkspaceGraphPrivacyProfile, WorkspaceGraphReadRequest,
};
use abilities_runtime::services::context::WorkspaceGraphReadError;
use serde_json::{json, Value};

use crate::services::mcp_v2::contracts::{
    McpActor, McpToolHandler, Scope, ToolDescription, ToolError,
};

const TOOL_NAME: &str = "dailyos.search.workspace_memory";
const SCHEMA_VERSION: u32 = 1;
const DEFAULT_PAGE_SIZE: u32 = 50;

pub struct WorkspaceSearchHandler {
    description: ToolDescription,
}

impl WorkspaceSearchHandler {
    pub fn new(description: ToolDescription) -> Self {
        Self { description }
    }
}

impl McpToolHandler for WorkspaceSearchHandler {
    fn description(&self) -> &ToolDescription {
        &self.description
    }

    fn invoke(&self, actor: &McpActor, params: Value) -> Result<Value, ToolError> {
        let McpActor::Client { granted_scopes, .. } = actor;
        let input = workspace_graph_input(&params, granted_scopes)?;
        let request = WorkspaceGraphReadRequest {
            input,
            privacy_profile: WorkspaceGraphPrivacyProfile::SurfaceClient,
        };
        let response =
            crate::services::workspace_ingestion::graph::read_workspace_graph_from_local_db(
                request,
            )
            .map_err(map_workspace_graph_error)?;
        serde_json::to_value(response).map_err(|_| ToolError::Internal {
            trace_id: "workspace_graph_serialize".to_string(),
        })
    }
}

fn workspace_graph_input(
    params: &Value,
    granted_scopes: &[Scope],
) -> Result<WorkspaceGraphInput, ToolError> {
    let include_entity_names =
        extract_bool_alias(params, &["includeEntityNames", "include_entity_names"])?
            && has_scope(granted_scopes, "read.entity_names");
    let mut input = json!({
        "schemaVersion": extract_u32_alias(params, &["schemaVersion", "schema_version"])?.unwrap_or(SCHEMA_VERSION),
        "pageSize": extract_u32_alias(params, &["pageSize", "page_size", "limit"])?.unwrap_or(DEFAULT_PAGE_SIZE),
        "includeEntityNames": include_entity_names,
    });

    if let Some(cursor) = extract_string_alias(params, &["cursor"])? {
        input["cursor"] = Value::String(cursor);
    }
    if let Some(etag) = extract_string_alias(params, &["ifNoneMatch", "if_none_match"])? {
        input["ifNoneMatch"] = Value::String(etag);
    }
    if let Some(filter) = entity_filter(params)? {
        input["entityFilter"] = filter;
    }
    if let Some(categories) =
        string_list_alias(params, &["categoryFilter", "category_filter", "categories"])?
    {
        input["categoryFilter"] = Value::Array(categories.into_iter().map(Value::String).collect());
    }

    serde_json::from_value(input).map_err(|error| ToolError::BadParams {
        detail: format!("invalid workspace graph input: {error}"),
    })
}

fn entity_filter(params: &Value) -> Result<Option<Value>, ToolError> {
    let mut entity_types = string_list_alias(
        params,
        &["entityTypes", "entity_types", "entityType", "entity_type"],
    )?;
    let mut entity_ids = string_list_alias(
        params,
        &["entityIds", "entity_ids", "entityId", "entity_id"],
    )?;

    if let Some(raw_filter) = params
        .get("entityFilter")
        .or_else(|| params.get("entity_filter"))
    {
        let object = raw_filter.as_object().ok_or_else(|| ToolError::BadParams {
            detail: "entityFilter must be an object".to_string(),
        })?;
        let filter_value = Value::Object(object.clone());
        if entity_types.is_none() {
            entity_types = string_list_alias(
                &filter_value,
                &["entityTypes", "entity_types", "entityType", "entity_type"],
            )?;
        }
        if entity_ids.is_none() {
            entity_ids = string_list_alias(
                &filter_value,
                &["entityIds", "entity_ids", "entityId", "entity_id"],
            )?;
        }
    }

    if entity_types.is_none() && entity_ids.is_none() {
        return Ok(None);
    }
    let mut filter = serde_json::Map::new();
    if let Some(types) = entity_types {
        filter.insert(
            "entityTypes".to_string(),
            Value::Array(types.into_iter().map(Value::String).collect()),
        );
    }
    if let Some(ids) = entity_ids {
        filter.insert(
            "entityIds".to_string(),
            Value::Array(ids.into_iter().map(Value::String).collect()),
        );
    }
    Ok(Some(Value::Object(filter)))
}

fn extract_bool_alias(params: &Value, names: &[&str]) -> Result<bool, ToolError> {
    let Some(value) = names.iter().find_map(|name| params.get(*name)) else {
        return Ok(false);
    };
    match value {
        Value::Bool(value) => Ok(*value),
        Value::String(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "true" => Ok(true),
            "false" | "" => Ok(false),
            _ => Err(ToolError::BadParams {
                detail: format!("{} must be a boolean", names[0]),
            }),
        },
        Value::Null => Ok(false),
        _ => Err(ToolError::BadParams {
            detail: format!("{} must be a boolean", names[0]),
        }),
    }
}

fn extract_u32_alias(params: &Value, names: &[&str]) -> Result<Option<u32>, ToolError> {
    let Some(value) = names.iter().find_map(|name| params.get(*name)) else {
        return Ok(None);
    };
    let parsed = match value {
        Value::Number(number) => number.as_u64().and_then(|value| u32::try_from(value).ok()),
        Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                trimmed.parse::<u32>().ok()
            }
        }
        Value::Null => None,
        _ => None,
    };
    parsed.map(Some).ok_or_else(|| ToolError::BadParams {
        detail: format!("{} must be a positive integer", names[0]),
    })
}

fn extract_string_alias(params: &Value, names: &[&str]) -> Result<Option<String>, ToolError> {
    let Some(value) = names.iter().find_map(|name| params.get(*name)) else {
        return Ok(None);
    };
    match value {
        Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        Value::Null => Ok(None),
        _ => Err(ToolError::BadParams {
            detail: format!("{} must be a string", names[0]),
        }),
    }
}

fn string_list_alias(params: &Value, names: &[&str]) -> Result<Option<Vec<String>>, ToolError> {
    let Some(value) = names.iter().find_map(|name| params.get(*name)) else {
        return Ok(None);
    };
    let values = match value {
        Value::Array(items) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
                    .ok_or_else(|| ToolError::BadParams {
                        detail: format!("{} must contain only strings", names[0]),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?,
        Value::String(raw) => raw
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect(),
        Value::Null => Vec::new(),
        _ => {
            return Err(ToolError::BadParams {
                detail: format!("{} must be a string or array of strings", names[0]),
            });
        }
    };
    if values.is_empty() {
        Ok(None)
    } else {
        Ok(Some(values))
    }
}

fn has_scope(granted_scopes: &[Scope], expected: &str) -> bool {
    granted_scopes
        .iter()
        .any(|scope| scope.as_str() == expected)
}

fn map_workspace_graph_error(error: WorkspaceGraphReadError) -> ToolError {
    match error {
        WorkspaceGraphReadError::InvalidCursor(message)
        | WorkspaceGraphReadError::InvalidFilter(message) => {
            ToolError::BadParams { detail: message }
        }
        WorkspaceGraphReadError::PageSizeTooLarge { requested, max } => ToolError::BadParams {
            detail: format!("pageSize {requested} exceeds max {max}"),
        },
        WorkspaceGraphReadError::ReadFailed(message)
        | WorkspaceGraphReadError::AuditFailed(message) => {
            eprintln!("{TOOL_NAME} failed: {message}");
            ToolError::Internal {
                trace_id: "workspace_graph_read".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_graph_input_from_plan_aligned_filters() {
        let input = workspace_graph_input(
            &json!({
                "entityFilter": {
                    "entityTypes": ["account"],
                    "entityIds": ["acct-1"]
                },
                "categoryFilter": ["notes"],
                "pageSize": "25",
                "cursor": "opaque",
                "includeEntityNames": true
            }),
            &[
                Scope::new("read.workspace_graph"),
                Scope::new("read.entity_names"),
            ],
        )
        .expect("input");

        assert_eq!(input.page_size, 25);
        assert!(input.include_entity_names);
        assert_eq!(
            input.cursor.as_ref().map(|cursor| cursor.as_str()),
            Some("opaque")
        );
        assert_eq!(
            input.category_filter.as_deref(),
            Some(&["notes".to_string()][..])
        );
        let filter = input.entity_filter.expect("entity filter");
        assert_eq!(
            filter.entity_types.as_deref(),
            Some(&["account".to_string()][..])
        );
        assert_eq!(
            filter.entity_ids.as_deref(),
            Some(&["acct-1".to_string()][..])
        );
    }

    #[test]
    fn redacts_entity_names_without_scope() {
        let input = workspace_graph_input(
            &json!({
                "includeEntityNames": true,
                "entityType": "account",
                "entityId": "acct-1"
            }),
            &[Scope::new("read.workspace_graph")],
        )
        .expect("input");

        assert!(!input.include_entity_names);
        let filter = input.entity_filter.expect("entity filter");
        assert_eq!(
            filter.entity_types.as_deref(),
            Some(&["account".to_string()][..])
        );
        assert_eq!(
            filter.entity_ids.as_deref(),
            Some(&["acct-1".to_string()][..])
        );
    }

    #[test]
    fn rejects_non_numeric_page_size() {
        let err = workspace_graph_input(
            &json!({ "pageSize": "many" }),
            &[Scope::new("read.workspace_graph")],
        )
        .expect_err("bad page size");
        assert!(matches!(err, ToolError::BadParams { .. }));
    }
}

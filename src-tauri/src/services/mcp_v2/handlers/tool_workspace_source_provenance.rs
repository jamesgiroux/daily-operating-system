//! `dailyos.read.workspace_source_provenance` MCP tool handler.
//!
//! Thin adapter over the service-owned workspace source provenance reader.

use serde_json::Value;

use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};
use crate::services::workspace_source_provenance::{
    normalize_workspace_source_entry_id, read_workspace_source_provenance_from_local_db,
    WorkspaceSourceProvenanceReadError,
};

const TOOL_NAME: &str = "dailyos.read.workspace_source_provenance";

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

    fn invoke(&self, _actor: &McpActor, params: Value) -> Result<Value, ToolError> {
        let entry_id = extract_entry_id(&params)?;
        read_workspace_source_provenance_from_local_db(&entry_id)
            .map_err(map_read_error)?
            .ok_or_else(|| ToolError::NotFound {
                resource: "workspace source provenance".to_string(),
            })
    }
}

fn extract_entry_id(params: &Value) -> Result<String, ToolError> {
    let value = params
        .get("entry_id")
        .or_else(|| params.get("entryId"))
        .or_else(|| params.get("source_key"))
        .or_else(|| params.get("sourceKey"))
        .or_else(|| params.get("link_handle"))
        .or_else(|| params.get("linkHandle"))
        .or_else(|| params.get("source_handle"))
        .or_else(|| params.get("sourceHandle"))
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::BadParams {
            detail: "entry_id must be a non-empty opaque workspace memory identifier".to_string(),
        })?;
    normalize_workspace_source_entry_id(value).map_err(map_read_error)
}

fn map_read_error(error: WorkspaceSourceProvenanceReadError) -> ToolError {
    match error {
        WorkspaceSourceProvenanceReadError::InvalidHandle(detail) => {
            ToolError::BadParams { detail }
        }
        WorkspaceSourceProvenanceReadError::ReadFailed(message) => {
            eprintln!("{TOOL_NAME} failed: {message}");
            ToolError::Internal {
                trace_id: "workspace_source_provenance".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::services::workspace_source_provenance::trust_band_label;

    use super::*;

    #[test]
    fn accepts_known_opaque_handle_aliases() {
        assert_eq!(
            extract_entry_id(&json!({ "sourceKey": "source:v1:abc_123" })).expect("source key"),
            "source:v1:abc_123"
        );
        assert_eq!(
            extract_entry_id(&json!({ "linkHandle": "11111111-2222-4333-8444-555555555555" }))
                .expect("link handle"),
            "11111111-2222-4333-8444-555555555555"
        );
    }

    #[test]
    fn rejects_path_like_entry_ids() {
        let err = extract_entry_id(&json!({ "entry_id": "../private.md" }))
            .expect_err("path-like ids rejected");
        assert!(matches!(err, ToolError::BadParams { .. }));
    }

    #[test]
    fn trust_band_labels_are_host_stable() {
        assert_eq!(trust_band_label(Some(0.95)), "likely_current");
        assert_eq!(trust_band_label(None), "unscored");
    }
}

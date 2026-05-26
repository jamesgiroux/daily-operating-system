//! `dailyos.submit.action_status` MCP tool handler.
//!
//! Status transitions route through `services::actions` so signals, claim sync,
//! and lifecycle semantics stay centralized.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::db::{ActionDb, LocalKeychain};
use crate::services::actions::{
    submit_action_status_in_db, ActionStatusSubmissionReceipt, ActionStatusSubmissionRequest,
    SubmittedActionStatus,
};
use crate::services::context::{
    attach_live_workspace_readers_with_signal_engine, ExternalClients, ServiceContext, SystemClock,
    SystemRng,
};
use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};
use crate::signals::propagation::PropagationEngine;

const ACTOR_LABEL: &str = concat!("agent:dailyos-mcp-v2:", env!("CARGO_PKG_VERSION"));

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

    fn invoke_with_services(
        &self,
        _actor: &McpActor,
        params: Value,
        services: &ServiceContext<'_>,
        db: &ActionDb,
    ) -> Result<Value, ToolError> {
        let request = status_request_from_params(params)?;
        let receipt = submit_action_status_in_db(services, db, &self.signal_engine, request)
            .map_err(map_status_error)?;
        Ok(receipt_json(receipt))
    }
}

impl McpToolHandler for UpdateActionStatusHandler {
    fn description(&self) -> &ToolDescription {
        &self.description
    }

    fn invoke(&self, actor: &McpActor, params: Value) -> Result<Value, ToolError> {
        let db = ActionDb::open(Arc::new(LocalKeychain::new())).map_err(|error| {
            ToolError::UpstreamFailure {
                detail: format!("action database unavailable: {error}"),
            }
        })?;
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let services = attach_live_workspace_readers_with_signal_engine(
            ServiceContext::new_live(&clock, &rng, &external).with_actor(ACTOR_LABEL),
            Some(Arc::clone(&self.signal_engine)),
        );
        self.invoke_with_services(actor, params, &services, &db)
    }
}

fn receipt_json(receipt: ActionStatusSubmissionReceipt) -> Value {
    json!({
        "action_id": receipt.action_id,
        "status": receipt.status,
        "mutation_cursor": {
            "action_id": receipt.mutation_cursor.action_id,
            "status": receipt.mutation_cursor.status
        }
    })
}

fn status_request_from_params(params: Value) -> Result<ActionStatusSubmissionRequest, ToolError> {
    let object = params.as_object().ok_or_else(|| ToolError::BadParams {
        detail: "params must be an object".to_string(),
    })?;
    let action_id =
        required_string(object, "action_id").or_else(|_| required_string(object, "actionId"))?;
    let status = required_string(object, "new_status")
        .or_else(|_| required_string(object, "newStatus"))
        .or_else(|_| required_string(object, "status"))?;
    let Some(new_status) = SubmittedActionStatus::parse(&status) else {
        return Err(ToolError::BadParams {
            detail: format!("unsupported action status: {status}"),
        });
    };
    Ok(ActionStatusSubmissionRequest {
        action_id,
        new_status,
        note: optional_string(object, "note")?,
    })
}

fn required_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, ToolError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| ToolError::BadParams {
            detail: format!("{field} is required"),
        })
}

fn optional_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<String>, ToolError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => {
            let value = value.trim();
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(value.to_string()))
            }
        }
        _ => Err(ToolError::BadParams {
            detail: format!("{field} must be a string"),
        }),
    }
}

fn map_status_error(message: String) -> ToolError {
    if message.contains("not found") || message.contains("Not found") {
        ToolError::NotFound {
            resource: "action".to_string(),
        }
    } else if message.contains("Invalid")
        || message.contains("must be")
        || message.contains("required")
        || message.contains("too long")
    {
        ToolError::BadParams { detail: message }
    } else {
        ToolError::UpstreamFailure { detail: message }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_parser_accepts_plan_shape() {
        let request = status_request_from_params(json!({
            "action_id": "action-1",
            "new_status": "done",
            "note": "Finished in the customer call"
        }))
        .expect("request");

        assert_eq!(request.action_id, "action-1");
        assert_eq!(request.new_status, SubmittedActionStatus::Done);
        assert_eq!(
            request.note.as_deref(),
            Some("Finished in the customer call")
        );
    }

    #[test]
    fn request_parser_accepts_status_alias_for_catalog_compatibility() {
        let request = status_request_from_params(json!({
            "actionId": "action-1",
            "status": "dropped"
        }))
        .expect("request");

        assert_eq!(request.action_id, "action-1");
        assert_eq!(request.new_status, SubmittedActionStatus::Dropped);
    }

    #[test]
    fn request_parser_rejects_unknown_status() {
        let error = status_request_from_params(json!({
            "action_id": "action-1",
            "new_status": "maybe"
        }))
        .unwrap_err();
        assert!(matches!(error, ToolError::BadParams { .. }));
    }
}

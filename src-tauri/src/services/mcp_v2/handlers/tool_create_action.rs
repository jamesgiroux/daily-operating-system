//! `dailyos.submit.action` MCP tool handler.
//!
//! The handler is a bridge edge only: request parsing and MCP error mapping
//! live here, while action persistence, signal emission, and claim sync stay in
//! `services::actions`.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::commands::CreateActionRequest;
use crate::db::{ActionDb, LocalKeychain};
use crate::services::actions::{
    create_action_in_db, ActionCreationAttribution, ActionCreationReceipt,
};
use crate::services::context::{
    attach_live_workspace_readers_with_signal_engine, ExternalClients, ServiceContext, SystemClock,
    SystemRng,
};
use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};
use crate::signals::propagation::PropagationEngine;

const ACTOR_LABEL: &str = concat!("agent:dailyos-mcp-v2:", env!("CARGO_PKG_VERSION"));

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

    fn invoke_with_services(
        &self,
        _actor: &McpActor,
        params: Value,
        services: &ServiceContext<'_>,
        db: &ActionDb,
    ) -> Result<Value, ToolError> {
        let request = create_action_request_from_params(params)?;
        let receipt = create_action_in_db(
            services,
            db,
            &self.signal_engine,
            request,
            ActionCreationAttribution::mcp_submit_action(),
        )
        .map_err(map_action_error)?;
        Ok(receipt_json(receipt))
    }
}

impl McpToolHandler for CreateActionHandler {
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

fn receipt_json(receipt: ActionCreationReceipt) -> Value {
    json!({
        "action_id": receipt.action_id,
        "mutation_cursor": {
            "action_id": receipt.mutation_cursor.action_id
        }
    })
}

fn create_action_request_from_params(params: Value) -> Result<CreateActionRequest, ToolError> {
    let object = params.as_object().ok_or_else(|| ToolError::BadParams {
        detail: "params must be an object".to_string(),
    })?;

    let entity_type = required_string(object, "entity_type")
        .or_else(|_| required_string(object, "entityType"))?;
    let entity_id =
        required_string(object, "entity_id").or_else(|_| required_string(object, "entityId"))?;
    let title = required_string(object, "title")?;

    let (account_id, project_id, person_id) = match entity_type.as_str() {
        "account" => (Some(entity_id), None, None),
        "project" => (None, Some(entity_id), None),
        "person" => (None, None, Some(entity_id)),
        other => {
            return Err(ToolError::BadParams {
                detail: format!("unsupported entity_type: {other}"),
            })
        }
    };

    Ok(CreateActionRequest {
        title,
        priority: optional_priority(object)?,
        due_date: optional_due_date(object)?,
        account_id,
        project_id,
        person_id,
        context: optional_string(object, "context")?,
        source_label: optional_source_label(object)?,
        action_kind: Some(crate::action_status::KIND_TASK.to_string()),
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

fn optional_priority(object: &serde_json::Map<String, Value>) -> Result<Option<String>, ToolError> {
    match object.get("priority") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => Ok(Some(number.to_string())),
        Some(Value::String(value)) => {
            let value = value.trim();
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(value.to_string()))
            }
        }
        _ => Err(ToolError::BadParams {
            detail: "priority must be an integer 0-4".to_string(),
        }),
    }
}

fn optional_due_date(object: &serde_json::Map<String, Value>) -> Result<Option<String>, ToolError> {
    let value = optional_string(object, "due_date")?
        .or(optional_string(object, "dueDate")?)
        .or(optional_string(object, "due")?)
        .or(optional_string(object, "due_at")?);
    Ok(value.map(|date| date.split('T').next().unwrap_or(&date).to_string()))
}

fn optional_source_label(
    object: &serde_json::Map<String, Value>,
) -> Result<Option<String>, ToolError> {
    match object
        .get("source_attribution")
        .or_else(|| object.get("sourceAttribution"))
    {
        None | Some(Value::Null) => Ok(
            optional_string(object, "source_label")?.or(optional_string(object, "sourceLabel")?)
        ),
        Some(Value::String(value)) => {
            let value = value.trim();
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(value.to_string()))
            }
        }
        Some(Value::Object(source)) => {
            for key in ["label", "source_label", "sourceLabel", "source"] {
                if let Some(value) = source.get(key).and_then(Value::as_str) {
                    let value = value.trim();
                    if !value.is_empty() {
                        return Ok(Some(value.to_string()));
                    }
                }
            }
            Ok(Some("MCP conversation".to_string()))
        }
        _ => Err(ToolError::BadParams {
            detail: "source_attribution must be a string or object".to_string(),
        }),
    }
}

fn map_action_error(message: String) -> ToolError {
    if message.contains("Invalid")
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
    fn request_parser_requires_supported_entity_binding() {
        let request = create_action_request_from_params(json!({
            "entity_type": "account",
            "entity_id": "acct-1",
            "title": "Follow up",
            "due_date": "2026-05-30",
            "priority": 2,
            "source_attribution": { "label": "MCP note" }
        }))
        .expect("request");

        assert_eq!(request.account_id.as_deref(), Some("acct-1"));
        assert_eq!(request.project_id, None);
        assert_eq!(request.person_id, None);
        assert_eq!(request.title, "Follow up");
        assert_eq!(request.due_date.as_deref(), Some("2026-05-30"));
        assert_eq!(request.priority.as_deref(), Some("2"));
        assert_eq!(request.source_label.as_deref(), Some("MCP note"));
    }

    #[test]
    fn request_parser_rejects_unbound_action() {
        let error = create_action_request_from_params(json!({
            "title": "Follow up"
        }))
        .unwrap_err();
        assert!(matches!(error, ToolError::BadParams { .. }));
    }

    #[test]
    fn request_parser_rejects_unknown_entity_type() {
        let error = create_action_request_from_params(json!({
            "entity_type": "ticket",
            "entity_id": "ticket-1",
            "title": "Follow up"
        }))
        .unwrap_err();
        assert!(matches!(error, ToolError::BadParams { .. }));
    }
}

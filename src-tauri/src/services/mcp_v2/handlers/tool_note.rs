//! `dailyos.submit.note` MCP tool handler.
//!
//! The handler is intentionally thin: MCP-specific parsing and error mapping
//! live here, while note persistence, signal emission, and claim semantics stay
//! in `services::entity_context`.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::db::{ActionDb, LocalKeychain};
use crate::services::context::{
    attach_live_workspace_readers_with_signal_engine, ExternalClients, ServiceContext, SystemClock,
    SystemRng,
};
use crate::services::entity_context::{
    create_user_note_claim_in_db, EntityContextNoteAttribution, EntityContextNoteCreationReceipt,
    EntityContextNoteCreationRequest,
};
use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};
use crate::signals::propagation::PropagationEngine;

const ACTOR_LABEL: &str = concat!("user:dailyos-mcp-v2:", env!("CARGO_PKG_VERSION"));

pub struct NoteHandler {
    description: ToolDescription,
    signal_engine: Arc<PropagationEngine>,
}

impl NoteHandler {
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
        let request = note_request_from_params(params)?;
        let receipt = create_user_note_claim_in_db(
            services,
            db,
            &self.signal_engine,
            request,
            EntityContextNoteAttribution::mcp_submit_note(),
            None,
        )
        .map_err(map_note_error)?;
        Ok(receipt_json(receipt))
    }
}

impl McpToolHandler for NoteHandler {
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

fn receipt_json(receipt: EntityContextNoteCreationReceipt) -> Value {
    json!({
        "note_id": receipt.note_id,
        "mutation_cursor": {
            "note_id": receipt.mutation_cursor.note_id
        }
    })
}

fn note_request_from_params(params: Value) -> Result<EntityContextNoteCreationRequest, ToolError> {
    let object = params.as_object().ok_or_else(|| ToolError::BadParams {
        detail: "params must be an object".to_string(),
    })?;

    let entity_type = required_string(object, "entity_type")
        .or_else(|_| required_string(object, "entityType"))?;
    let entity_id =
        required_string(object, "entity_id").or_else(|_| required_string(object, "entityId"))?;
    let content =
        required_string(object, "content").or_else(|_| required_string(object, "text"))?;
    let title = optional_string(object, "title")?.unwrap_or_else(|| "Note".to_string());
    let source_attribution = optional_source_attribution(object)?;

    Ok(EntityContextNoteCreationRequest {
        entity_type,
        entity_id,
        title,
        content,
        source_attribution,
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

fn optional_source_attribution(
    object: &serde_json::Map<String, Value>,
) -> Result<Option<Value>, ToolError> {
    match object
        .get("source_attribution")
        .or_else(|| object.get("sourceAttribution"))
    {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => {
            let value = value.trim();
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(json!({ "label": value })))
            }
        }
        Some(Value::Object(source)) => Ok(Some(Value::Object(source.clone()))),
        _ => Err(ToolError::BadParams {
            detail: "source_attribution must be a string or object".to_string(),
        }),
    }
}

fn map_note_error(message: String) -> ToolError {
    if message.contains("Unsupported")
        || message.contains("cannot be")
        || message.contains("must be")
        || message.contains("required")
        || message.contains("too long")
        || message.contains("too short")
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
    fn request_parser_requires_entity_binding() {
        let error = note_request_from_params(json!({
            "content": "Capture this account observation"
        }))
        .unwrap_err();

        assert!(matches!(error, ToolError::BadParams { .. }));
    }

    #[test]
    fn request_parser_accepts_plan_shape() {
        let request = note_request_from_params(json!({
            "entity_type": "account",
            "entity_id": "acct-1",
            "content": "Sponsor wants a readiness recap",
            "source_attribution": { "label": "MCP conversation" }
        }))
        .expect("request");

        assert_eq!(request.entity_type, "account");
        assert_eq!(request.entity_id, "acct-1");
        assert_eq!(request.title, "Note");
        assert_eq!(request.content, "Sponsor wants a readiness recap");
        assert_eq!(
            request
                .source_attribution
                .as_ref()
                .and_then(|value| value.get("label"))
                .and_then(Value::as_str),
            Some("MCP conversation")
        );
    }

    #[test]
    fn receipt_cursor_does_not_echo_note_content() {
        let receipt = EntityContextNoteCreationReceipt {
            note_id: "note-1".to_string(),
            mutation_cursor: crate::services::entity_context::EntityContextNoteMutationCursor {
                note_id: "note-1".to_string(),
            },
            entry: crate::types::EntityContextEntry {
                id: "note-1".to_string(),
                entity_type: "account".to_string(),
                entity_id: "acct-1".to_string(),
                title: crate::types::EntityContextText::Plain("Note".to_string()),
                content: crate::types::EntityContextText::Plain("Sensitive note".to_string()),
                created_at: "2026-05-26T00:00:00Z".to_string(),
                updated_at: "2026-05-26T00:00:00Z".to_string(),
            },
        };

        assert_eq!(
            receipt_json(receipt),
            json!({
                "note_id": "note-1",
                "mutation_cursor": { "note_id": "note-1" }
            })
        );
    }
}

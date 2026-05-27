use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SubjectAttribution, SubjectRef,
};
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};
use crate::services::workspace_intake::{
    PlacementError, PlacementErrorCode, PlacementInvocationContext, WorkspacePlaceDocumentInput,
    WorkspacePlaceDocumentReceipt, WorkspacePlaceDocumentRequest,
    WORKSPACE_PLACE_DOCUMENT_CONTENT_B64_MAX_BYTES, WORKSPACE_PLACE_DOCUMENT_SCHEMA_VERSION,
    WORKSPACE_PLACE_DOCUMENT_SERIALIZED_ARGUMENTS_MAX_BYTES, WORKSPACE_PLACE_DOCUMENT_TOOL_NAME,
};

pub async fn workspace_place_document(
    ctx: &AbilityContext<'_>,
    input: WorkspacePlaceDocumentInput,
) -> AbilityResult<WorkspacePlaceDocumentReceipt> {
    let input = parse_placement_input(input)?;
    let invocation = placement_invocation_context(ctx)?;
    let receipt = ctx
        .services()
        .workspace_intake()
        .ok_or_else(|| {
            ability_error(PlacementError::internal(
                "workspace placement service unavailable",
            ))
        })?
        .place_document(ctx, invocation, input)
        .await
        .map_err(ability_error)?;

    finalize_workspace_place_document_output(ctx, receipt)
}

fn validate_pre_service(input: &WorkspacePlaceDocumentRequest) -> Result<(), AbilityError> {
    if input.schema_version != WORKSPACE_PLACE_DOCUMENT_SCHEMA_VERSION {
        return Err(ability_error(PlacementError::new(
            PlacementErrorCode::UnsupportedSchemaVersion,
            "schema_version must be 1",
        )));
    }
    if input.category.trim().is_empty() {
        return Err(ability_error(PlacementError::new(
            PlacementErrorCode::InvalidRequestShape,
            "category is required",
        )));
    }
    if input.content_b64.len() > WORKSPACE_PLACE_DOCUMENT_CONTENT_B64_MAX_BYTES {
        return Err(ability_error(PlacementError::new(
            PlacementErrorCode::ContentTooLarge,
            "content_b64 exceeds the encoded limit",
        )));
    }
    Ok(())
}

fn parse_placement_input(
    input: WorkspacePlaceDocumentInput,
) -> Result<WorkspacePlaceDocumentRequest, AbilityError> {
    let raw = input.into_raw();
    let argument_bytes = serde_json::to_vec(&raw)
        .map_err(|error| ability_error(PlacementError::internal(error.to_string())))?
        .len();
    if argument_bytes > WORKSPACE_PLACE_DOCUMENT_SERIALIZED_ARGUMENTS_MAX_BYTES {
        return Err(ability_error(PlacementError::new(
            PlacementErrorCode::ContentTooLarge,
            "serialized tool arguments exceed the placement limit",
        )));
    }

    validate_raw_schema_version(&raw)?;
    let request: WorkspacePlaceDocumentRequest = serde_json::from_value(raw).map_err(|_| {
        ability_error(PlacementError::new(
            PlacementErrorCode::InvalidRequestShape,
            "tool arguments do not match the workspace placement schema",
        ))
    })?;
    validate_pre_service(&request)?;
    Ok(request)
}

fn validate_raw_schema_version(raw: &serde_json::Value) -> Result<(), AbilityError> {
    let Some(value) = raw.get("schema_version") else {
        return Err(ability_error(PlacementError::new(
            PlacementErrorCode::InvalidRequestShape,
            "schema_version is required",
        )));
    };
    let serde_json::Value::Number(number) = value else {
        return Err(ability_error(PlacementError::new(
            PlacementErrorCode::InvalidRequestShape,
            "schema_version must be an integer",
        )));
    };
    if number.as_u64() == Some(WORKSPACE_PLACE_DOCUMENT_SCHEMA_VERSION as u64) {
        return Ok(());
    }
    if number.as_i64().is_some() || number.as_u64().is_some() {
        return Err(ability_error(PlacementError::new(
            PlacementErrorCode::UnsupportedSchemaVersion,
            "schema_version must be 1",
        )));
    }
    Err(ability_error(PlacementError::new(
        PlacementErrorCode::InvalidRequestShape,
        "schema_version must be an integer",
    )))
}

fn placement_invocation_context(
    ctx: &AbilityContext<'_>,
) -> Result<PlacementInvocationContext, AbilityError> {
    let Actor::McpClient { client_id, .. } = &ctx.actor else {
        return Err(ability_error(PlacementError::new(
            PlacementErrorCode::TargetNotFoundOrUnauthorized,
            "workspace placement is only available to paired MCP clients",
        )));
    };

    Ok(PlacementInvocationContext {
        actor_id: client_id.as_str().to_string(),
        tool_name: WORKSPACE_PLACE_DOCUMENT_TOOL_NAME.to_string(),
        can_read_entity_names: false,
    })
}

fn finalize_workspace_place_document_output(
    ctx: &AbilityContext<'_>,
    output: WorkspacePlaceDocumentReceipt,
) -> AbilityResult<WorkspacePlaceDocumentReceipt> {
    let mut config =
        ProvenanceBuilderConfig::new("workspace_place_document", ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(WORKSPACE_PLACE_DOCUMENT_SCHEMA_VERSION);
    config.actor = provenance_actor(ctx.actor.clone());
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Transform;

    let mut builder = ProvenanceBuilder::new(config);
    let subject_attr = SubjectAttribution::direct_confident(SubjectRef::Global);
    builder.set_subject(subject_attr.clone());
    builder
        .attribute_subtree(
            FieldPath::new("").map_err(field_error)?,
            FieldAttribution::constant(subject_attr),
        )
        .map_err(provenance_error)?;
    builder.finalize(output).map_err(provenance_error)
}

fn provenance_actor(actor: Actor) -> crate::abilities::provenance::Actor {
    match actor {
        Actor::McpClient { .. } => crate::abilities::provenance::Actor::External {
            source: "mcp".to_string(),
        },
        Actor::Agent => crate::abilities::provenance::Actor::Agent {
            name: "agent:workspace_place_document".to_string(),
            version: "unknown".to_string(),
        },
        Actor::System => crate::abilities::provenance::Actor::System {
            component: "system:workspace_place_document".to_string(),
        },
        Actor::User | Actor::Admin | Actor::SurfaceClient { .. } => {
            crate::abilities::provenance::Actor::User
        }
    }
}

fn ability_error(error: PlacementError) -> AbilityError {
    let code = error.code.clone();
    let message = serde_json::to_string(&error).unwrap_or(error.message);
    AbilityError {
        kind: AbilityErrorKind::HardError(code),
        message,
    }
}

fn field_error(error: impl std::fmt::Display) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: format!("field attribution path failed: {error}"),
    }
}

fn provenance_error(error: impl std::fmt::Display) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: format!("provenance construction failed: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input(raw: serde_json::Value) -> WorkspacePlaceDocumentInput {
        WorkspacePlaceDocumentInput { raw }
    }

    fn placement_code(raw: serde_json::Value) -> String {
        let error = parse_placement_input(input(raw)).expect_err("input should be rejected");
        let AbilityErrorKind::HardError(code) = error.kind else {
            panic!("expected placement hard error, got {:?}", error.kind);
        };
        code
    }

    fn valid_raw() -> serde_json::Value {
        json!({
            "schema_version": 1,
            "entity": {
                "entity_type": "account",
                "entity_id": "acct_123"
            },
            "content_b64": "aGVsbG8=",
            "content_type": "text/markdown",
            "category": "notes"
        })
    }

    #[test]
    fn missing_required_fields_map_to_frozen_placement_error_code() {
        let mut raw = valid_raw();
        raw.as_object_mut().expect("object").remove("category");

        assert_eq!(
            placement_code(raw),
            PlacementErrorCode::InvalidRequestShape.as_str()
        );
    }

    #[test]
    fn non_integer_schema_version_maps_to_invalid_shape() {
        let mut raw = valid_raw();
        raw["schema_version"] = json!("1");

        assert_eq!(
            placement_code(raw),
            PlacementErrorCode::InvalidRequestShape.as_str()
        );
    }

    #[test]
    fn integer_schema_version_mismatch_maps_to_unsupported_schema_version() {
        let mut raw = valid_raw();
        raw["schema_version"] = json!(2);

        assert_eq!(
            placement_code(raw),
            PlacementErrorCode::UnsupportedSchemaVersion.as_str()
        );
    }

    #[test]
    fn serialized_arguments_limit_runs_before_typed_request_parse() {
        let mut raw = valid_raw();
        raw["extra"] = json!("x".repeat(WORKSPACE_PLACE_DOCUMENT_SERIALIZED_ARGUMENTS_MAX_BYTES));

        assert_eq!(
            placement_code(raw),
            PlacementErrorCode::ContentTooLarge.as_str()
        );
    }

    #[test]
    fn ability_error_preserves_optional_placement_error_fields() {
        let error = ability_error(
            PlacementError::new(
                PlacementErrorCode::CategoryNotAllowed,
                "category is not allowed",
            )
            .with_allowed(vec!["notes".to_string()]),
        );

        assert_eq!(
            error.kind,
            AbilityErrorKind::HardError(
                PlacementErrorCode::CategoryNotAllowed.as_str().to_string()
            )
        );
        assert!(
            error.message.contains(r#""allowed":["notes"]"#),
            "placement envelope should be preserved in the safe error message: {}",
            error.message
        );
    }
}

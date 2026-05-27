//! Producer for the `markdown_preview` read ability.

use crate::abilities::markdown_preview::contracts::{
    MarkdownPreviewInput, MarkdownPreviewOutput, MarkdownPreviewReadRequest,
};
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SubjectAttribution, SubjectRef,
};
use crate::abilities::registry::{Actor, SurfaceScope};
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult,
};
use crate::services::context::MarkdownPreviewReadError;

pub async fn markdown_preview(
    ctx: &AbilityContext<'_>,
    input: MarkdownPreviewInput,
) -> AbilityResult<MarkdownPreviewOutput> {
    authorize(ctx)?;
    let schema_version = input.schema_version;
    let response = ctx
        .services()
        .read_markdown_preview(MarkdownPreviewReadRequest { input })
        .await
        .map_err(read_error)?;
    finalize_output(ctx, schema_version, response)
}

fn authorize(ctx: &AbilityContext<'_>) -> Result<(), AbilityError> {
    match &ctx.actor {
        Actor::SurfaceClient { scopes, .. } => {
            if !scopes.contains(&SurfaceScope::new("read.markdown_preview")) {
                return Err(permission_denied("read.markdown_preview_required"));
            }
            Ok(())
        }
        _ => Err(AbilityError {
            kind: AbilityErrorKind::Capability,
            message: "permission_denied: surface_client_required".to_string(),
        }),
    }
}

fn permission_denied(reason: &str) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Capability,
        message: format!("permission_denied: {reason}"),
    }
}

fn read_error(error: MarkdownPreviewReadError) -> AbilityError {
    match error {
        MarkdownPreviewReadError::InvalidSourceHandle(message) => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("invalid_source_handle: {message}"),
        },
        MarkdownPreviewReadError::SourceNotFound => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: "source_not_found".to_string(),
        },
        MarkdownPreviewReadError::SourceUnavailable(message) => AbilityError {
            kind: AbilityErrorKind::HardError(message.clone()),
            message: format!("source_unavailable: {message}"),
        },
        MarkdownPreviewReadError::UnsupportedSource(message) => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("unsupported_source: {message}"),
        },
    }
}

fn finalize_output(
    ctx: &AbilityContext<'_>,
    schema_version: u32,
    response: MarkdownPreviewOutput,
) -> AbilityResult<MarkdownPreviewOutput> {
    let mut builder = ProvenanceBuilder::new(provenance_config(ctx, schema_version));
    let subject_attribution = SubjectAttribution::direct_confident(SubjectRef::Global);
    builder.set_subject(subject_attribution.clone());
    builder
        .attribute_subtree(
            FieldPath::root(),
            FieldAttribution::constant(subject_attribution),
        )
        .map_err(provenance_error)?;
    builder.finalize(response).map_err(provenance_error)
}

fn provenance_config(ctx: &AbilityContext<'_>, schema_version: u32) -> ProvenanceBuilderConfig {
    let mut config = ProvenanceBuilderConfig::new("markdown_preview", ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(schema_version);
    config.actor = provenance_actor(ctx.actor.clone());
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Read;
    config
}

fn provenance_actor(actor: Actor) -> crate::abilities::provenance::Actor {
    match actor {
        Actor::SurfaceClient { .. } => crate::abilities::provenance::Actor::Human {
            role: "surface_client".to_string(),
            id: "surface_client".to_string(),
        },
        Actor::User => crate::abilities::provenance::Actor::User,
        Actor::Agent => crate::abilities::provenance::Actor::Agent {
            name: "agent".to_string(),
            version: "unknown".to_string(),
        },
        Actor::Admin => crate::abilities::provenance::Actor::Human {
            role: "admin".to_string(),
            id: "admin".to_string(),
        },
        Actor::System => crate::abilities::provenance::Actor::System {
            component: "dailyos".to_string(),
        },
        Actor::McpClient { .. } => crate::abilities::provenance::Actor::Agent {
            name: "mcp_client".to_string(),
            version: "unknown".to_string(),
        },
    }
}

fn provenance_error(error: impl std::fmt::Display) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: format!("provenance construction failed: {error}"),
    }
}

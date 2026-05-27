//! Producer for the `workspace_graph` read ability.

use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SubjectAttribution, SubjectRef,
};
use crate::abilities::registry::{Actor, SurfaceScope};
use crate::abilities::workspace_graph::contracts::{
    WorkspaceGraphInput, WorkspaceGraphPrivacyProfile, WorkspaceGraphReadRequest,
    WorkspaceGraphResponse,
};
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult,
};
use crate::services::context::WorkspaceGraphReadError;

pub async fn workspace_graph(
    ctx: &AbilityContext<'_>,
    input: WorkspaceGraphInput,
) -> AbilityResult<WorkspaceGraphResponse> {
    let privacy_profile = authorize(ctx, &input)?;
    let schema_version = input.schema_version;
    let response = ctx
        .services()
        .read_workspace_graph(WorkspaceGraphReadRequest {
            input,
            privacy_profile,
        })
        .await
        .map_err(read_error)?;
    finalize_output(ctx, schema_version, response)
}

fn authorize(
    ctx: &AbilityContext<'_>,
    input: &WorkspaceGraphInput,
) -> Result<WorkspaceGraphPrivacyProfile, AbilityError> {
    match &ctx.actor {
        Actor::SurfaceClient { scopes, .. } => {
            if !scopes.contains(&SurfaceScope::new("read.workspace_graph")) {
                return Err(permission_denied("read.workspace_graph_required"));
            }
            if input.include_entity_names
                && !scopes.contains(&SurfaceScope::new("read.entity_names"))
            {
                return Err(permission_denied("read.entity_names_required"));
            }
            Ok(WorkspaceGraphPrivacyProfile::SurfaceClient)
        }
        Actor::Agent => Ok(WorkspaceGraphPrivacyProfile::SurfaceClient),
        Actor::User | Actor::System => Ok(WorkspaceGraphPrivacyProfile::FirstParty),
        _ => Err(permission_denied("actor_not_allowed")),
    }
}

fn permission_denied(reason: &str) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Capability,
        message: format!("permission_denied: {reason}"),
    }
}

fn read_error(error: WorkspaceGraphReadError) -> AbilityError {
    match error {
        WorkspaceGraphReadError::InvalidCursor(message) => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("invalid_cursor: {message}"),
        },
        WorkspaceGraphReadError::InvalidFilter(message) => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("invalid_filter: {message}"),
        },
        WorkspaceGraphReadError::PageSizeTooLarge { requested, max } => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("page_size_too_large: requested {requested}, max {max}"),
        },
        WorkspaceGraphReadError::ReadFailed(message) => AbilityError {
            kind: AbilityErrorKind::HardError(message.clone()),
            message: format!("read_unavailable: {message}"),
        },
        WorkspaceGraphReadError::AuditFailed(message) => AbilityError {
            kind: AbilityErrorKind::HardError(message.clone()),
            message: format!("audit_failed: {message}"),
        },
    }
}

fn finalize_output(
    ctx: &AbilityContext<'_>,
    schema_version: u32,
    response: WorkspaceGraphResponse,
) -> AbilityResult<WorkspaceGraphResponse> {
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
    let mut config = ProvenanceBuilderConfig::new("workspace_graph", ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(schema_version);
    config.actor = provenance_actor(ctx.actor.clone());
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Read;
    config
}

fn provenance_actor(actor: Actor) -> crate::abilities::provenance::Actor {
    match actor {
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
        Actor::SurfaceClient { .. } => crate::abilities::provenance::Actor::Human {
            role: "surface_client".to_string(),
            id: "surface_client".to_string(),
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

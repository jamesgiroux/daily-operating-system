//! Producer for the `source_management_ledger` read ability.

use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SubjectAttribution, SubjectRef,
};
use crate::abilities::registry::{Actor, SurfaceScope};
use crate::abilities::source_management_ledger::contracts::{
    SourceManagementActionInput, SourceManagementActionReceipt, SourceManagementActionRequest,
    SourceManagementLedgerInput, SourceManagementLedgerPrivacyProfile,
    SourceManagementLedgerReadRequest, SourceManagementLedgerResponse,
};
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult,
};
use crate::services::context::SourceManagementLedgerReadError;

pub async fn source_management_ledger(
    ctx: &AbilityContext<'_>,
    input: SourceManagementLedgerInput,
) -> AbilityResult<SourceManagementLedgerResponse> {
    let privacy_profile = authorize(ctx)?;
    let schema_version = input.schema_version;
    let response = ctx
        .services()
        .read_source_management_ledger(SourceManagementLedgerReadRequest {
            input,
            privacy_profile,
        })
        .await
        .map_err(read_error)?;
    finalize_output(ctx, schema_version, response)
}

pub async fn source_management_action(
    ctx: &AbilityContext<'_>,
    input: SourceManagementActionInput,
) -> AbilityResult<SourceManagementActionReceipt> {
    let actor_id = authorize_action(ctx)?;
    let schema_version = input.schema_version;
    let receipt = ctx
        .services()
        .apply_source_management_action(SourceManagementActionRequest { input, actor_id })
        .await
        .map_err(action_error)?;
    finalize_action_output(ctx, schema_version, receipt)
}

fn authorize(ctx: &AbilityContext<'_>) -> Result<SourceManagementLedgerPrivacyProfile, AbilityError> {
    match &ctx.actor {
        Actor::SurfaceClient { scopes, .. } => {
            if !scopes.contains(&SurfaceScope::new("read.workspace_sources")) {
                return Err(permission_denied("read.workspace_sources_required"));
            }
            Ok(SourceManagementLedgerPrivacyProfile::SurfaceClient)
        }
        Actor::User | Actor::System => Ok(SourceManagementLedgerPrivacyProfile::FirstParty),
        _ => Err(permission_denied("actor_not_allowed")),
    }
}

fn authorize_action(ctx: &AbilityContext<'_>) -> Result<String, AbilityError> {
    match &ctx.actor {
        Actor::SurfaceClient { instance, scopes } => {
            if !scopes.contains(&SurfaceScope::new("write.entity_intake")) {
                return Err(permission_denied("write.entity_intake_required"));
            }
            Ok(format!("surface_client:{}", instance.as_str()))
        }
        _ => Err(permission_denied("actor_not_allowed")),
    }
}

fn permission_denied(reason: &str) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Capability,
        message: format!("permission_denied: {reason}"),
    }
}

fn read_error(error: SourceManagementLedgerReadError) -> AbilityError {
    match error {
        SourceManagementLedgerReadError::InvalidCursor(message) => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("invalid_cursor: {message}"),
        },
        SourceManagementLedgerReadError::InvalidFilter(message) => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("invalid_filter: {message}"),
        },
        SourceManagementLedgerReadError::PageSizeTooLarge { requested, max } => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("page_size_too_large: requested {requested}, max {max}"),
        },
        SourceManagementLedgerReadError::ReadFailed(message) => AbilityError {
            kind: AbilityErrorKind::HardError(message.clone()),
            message: format!("read_unavailable: {message}"),
        },
    }
}

fn action_error(error: crate::services::context::SourceManagementActionError) -> AbilityError {
    match error {
        crate::services::context::SourceManagementActionError::InvalidRequest(message) => {
            AbilityError {
                kind: AbilityErrorKind::Validation,
                message: format!("invalid_request: {message}"),
            }
        }
        crate::services::context::SourceManagementActionError::ActionFailed(message) => {
            AbilityError {
                kind: AbilityErrorKind::HardError(message.clone()),
                message: format!("source_action_failed: {message}"),
            }
        }
    }
}

fn finalize_output(
    ctx: &AbilityContext<'_>,
    schema_version: u32,
    response: SourceManagementLedgerResponse,
) -> AbilityResult<SourceManagementLedgerResponse> {
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

fn finalize_action_output(
    ctx: &AbilityContext<'_>,
    schema_version: u32,
    receipt: SourceManagementActionReceipt,
) -> AbilityResult<SourceManagementActionReceipt> {
    let mut builder = ProvenanceBuilder::new(provenance_action_config(ctx, schema_version));
    let subject_attribution = SubjectAttribution::direct_confident(SubjectRef::Global);
    builder.set_subject(subject_attribution.clone());
    builder
        .attribute_subtree(
            FieldPath::root(),
            FieldAttribution::constant(subject_attribution),
        )
        .map_err(provenance_error)?;
    builder.finalize(receipt).map_err(provenance_error)
}

fn provenance_config(ctx: &AbilityContext<'_>, schema_version: u32) -> ProvenanceBuilderConfig {
    let mut config =
        ProvenanceBuilderConfig::new("source_management_ledger", ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(schema_version);
    config.actor = provenance_actor(ctx.actor.clone());
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Read;
    config
}

fn provenance_action_config(ctx: &AbilityContext<'_>, schema_version: u32) -> ProvenanceBuilderConfig {
    let mut config =
        ProvenanceBuilderConfig::new("source_management_action", ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(schema_version);
    config.actor = provenance_actor(ctx.actor.clone());
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Transform;
    config
}

fn provenance_actor(actor: Actor) -> crate::abilities::provenance::Actor {
    match actor {
        Actor::User => crate::abilities::provenance::Actor::User,
        Actor::System => crate::abilities::provenance::Actor::System {
            component: "dailyos".to_string(),
        },
        Actor::SurfaceClient { .. } => crate::abilities::provenance::Actor::Human {
            role: "surface_client".to_string(),
            id: "surface_client".to_string(),
        },
        Actor::Agent => crate::abilities::provenance::Actor::Agent {
            name: "agent".to_string(),
            version: "unknown".to_string(),
        },
        Actor::Admin => crate::abilities::provenance::Actor::Human {
            role: "admin".to_string(),
            id: "admin".to_string(),
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

//! `claim_files` projection and correction abilities.

pub mod contracts;

pub use contracts::{
    ApplyClaimFileCorrectionsInput, ClaimFileApplyFailure, ClaimFileApplyRequest,
    ClaimFileApplyResult, ClaimFileOperationError, ClaimFileProjectionResult,
    ClaimFileRenderRequest, RenderEntityClaimFileInput, APPLY_CLAIM_FILE_CORRECTIONS_ABILITY_NAME,
    CLAIM_FILES_SCHEMA_VERSION, RENDER_ENTITY_CLAIM_FILE_ABILITY_NAME,
};

use dailyos_abilities_macro::ability;

use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SubjectAttribution, SubjectRef,
};
use crate::abilities::registry::Actor;
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult,
};
use crate::services::context::ServiceError;
use crate::types::{subject_ref_from_json, ClaimSubjectRef};

#[ability(
    name = "render_entity_claim_file",
    category = Maintenance,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = true,
    mcp_exposure = None,
    client_side_executable = false,
    mutates = [claim_file_projection_runs, claim_file_projection_run_claims],
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn render_entity_claim_file(
    ctx: &AbilityContext<'_>,
    input: RenderEntityClaimFileInput,
) -> AbilityResult<ClaimFileProjectionResult> {
    authorize_user(ctx)?;
    validate_schema_version(input.schema_version, RENDER_ENTITY_CLAIM_FILE_ABILITY_NAME)?;
    ctx.services()
        .check_mutation_allowed()
        .map_err(service_error)?;

    let subject = subject_for_value(&input.subject_ref);
    let schema_version = input.schema_version;
    let response = ctx
        .services()
        .render_entity_claim_file(ClaimFileRenderRequest { input })
        .await
        .map_err(operation_error)?;
    finalize_output(
        ctx,
        RENDER_ENTITY_CLAIM_FILE_ABILITY_NAME,
        AbilityCategory::Maintenance,
        schema_version,
        subject,
        response,
    )
}

#[ability(
    name = "apply_claim_file_corrections",
    category = Maintenance,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = false,
    mcp_exposure = None,
    client_side_executable = false,
    mutates = [intelligence_claims, claim_feedback, claim_file_projection_runs, claim_file_projection_run_claims],
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn apply_claim_file_corrections(
    ctx: &AbilityContext<'_>,
    input: ApplyClaimFileCorrectionsInput,
) -> AbilityResult<ClaimFileApplyResult> {
    let actor_principal_id = authorize_user(ctx)?;
    validate_schema_version(
        input.schema_version,
        APPLY_CLAIM_FILE_CORRECTIONS_ABILITY_NAME,
    )?;
    ctx.services()
        .check_mutation_allowed()
        .map_err(service_error)?;

    let schema_version = input.schema_version;
    let response = ctx
        .services()
        .apply_claim_file_corrections(ClaimFileApplyRequest {
            input,
            actor_principal_id,
        })
        .await
        .map_err(operation_error)?;
    finalize_output(
        ctx,
        APPLY_CLAIM_FILE_CORRECTIONS_ABILITY_NAME,
        AbilityCategory::Maintenance,
        schema_version,
        SubjectRef::Global,
        response,
    )
}

fn authorize_user(ctx: &AbilityContext<'_>) -> Result<String, AbilityError> {
    match &ctx.actor {
        Actor::User => Ok("user".to_string()),
        _ => Err(permission_denied("actor_not_allowed")),
    }
}

fn validate_schema_version(schema_version: u32, ability_name: &str) -> Result<(), AbilityError> {
    if schema_version == CLAIM_FILES_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("unsupported schema_version `{schema_version}` for `{ability_name}`"),
        })
    }
}

fn subject_for_value(value: &serde_json::Value) -> SubjectRef {
    match subject_ref_from_json(value) {
        Ok(subject) => provenance_subject(subject),
        Err(_) => SubjectRef::Unknown,
    }
}

fn provenance_subject(subject: ClaimSubjectRef) -> SubjectRef {
    match subject {
        ClaimSubjectRef::Account { id } => SubjectRef::Account(id),
        ClaimSubjectRef::Project { id } => SubjectRef::Project(id),
        ClaimSubjectRef::Person { id } => SubjectRef::Person(id),
        ClaimSubjectRef::Action { id } => SubjectRef::Action(id),
        ClaimSubjectRef::Meeting { id } => SubjectRef::Meeting(id),
        ClaimSubjectRef::Global => SubjectRef::Global,
        ClaimSubjectRef::Multi(subjects) => {
            SubjectRef::Multi(subjects.into_iter().map(provenance_subject).collect())
        }
        ClaimSubjectRef::Email { .. } => SubjectRef::Unknown,
    }
}

fn finalize_output<T>(
    ctx: &AbilityContext<'_>,
    ability_name: &'static str,
    category: AbilityCategory,
    schema_version: u32,
    subject: SubjectRef,
    response: T,
) -> AbilityResult<T>
where
    T: serde::Serialize,
{
    let mut builder = ProvenanceBuilder::new(provenance_config(
        ctx,
        ability_name,
        category,
        schema_version,
    ));
    let subject_attribution = SubjectAttribution::direct_confident(subject);
    builder.set_subject(subject_attribution.clone());
    builder
        .attribute_subtree(
            FieldPath::root(),
            FieldAttribution::constant(subject_attribution),
        )
        .map_err(provenance_error)?;
    builder.finalize(response).map_err(provenance_error)
}

fn provenance_config(
    ctx: &AbilityContext<'_>,
    ability_name: &'static str,
    category: AbilityCategory,
    schema_version: u32,
) -> ProvenanceBuilderConfig {
    let mut config = ProvenanceBuilderConfig::new(ability_name, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(schema_version);
    config.actor = provenance_actor(ctx.actor.clone());
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = category;
    config
}

fn provenance_actor(actor: Actor) -> crate::abilities::provenance::Actor {
    match actor {
        Actor::User => crate::abilities::provenance::Actor::User,
        Actor::SurfaceClient { .. } => crate::abilities::provenance::Actor::Human {
            role: "surface_client".to_string(),
            id: "surface_client".to_string(),
        },
        Actor::System => crate::abilities::provenance::Actor::System {
            component: "dailyos".to_string(),
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

fn service_error(error: ServiceError) -> AbilityError {
    match error {
        ServiceError::WriteBlockedByMode(mode) => AbilityError {
            kind: AbilityErrorKind::Capability,
            message: format!("write_blocked_by_mode: {mode:?}"),
        },
        other => AbilityError {
            kind: AbilityErrorKind::HardError(other.to_string()),
            message: other.to_string(),
        },
    }
}

fn operation_error(error: ClaimFileOperationError) -> AbilityError {
    match error {
        ClaimFileOperationError::InvalidRequest(message) => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("invalid_claim_file_request: {message}"),
        },
        ClaimFileOperationError::MutationBlocked(message) => AbilityError {
            kind: AbilityErrorKind::Capability,
            message: format!("claim_file_mutation_blocked: {message}"),
        },
        ClaimFileOperationError::OperationFailed(message) => AbilityError {
            kind: AbilityErrorKind::HardError(message.clone()),
            message: format!("claim_file_operation_failed: {message}"),
        },
    }
}

fn permission_denied(reason: &str) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Capability,
        message: format!("permission_denied: {reason}"),
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
    use crate::abilities::registry::{AbilityRegistry, ActorKind, McpExposure};

    #[test]
    fn descriptors_are_registered_and_hidden_from_mcp() {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let render = registry
            .iter_all()
            .find(|descriptor| descriptor.name == RENDER_ENTITY_CLAIM_FILE_ABILITY_NAME)
            .expect("render ability is registered");
        let apply = registry
            .iter_all()
            .find(|descriptor| descriptor.name == APPLY_CLAIM_FILE_CORRECTIONS_ABILITY_NAME)
            .expect("apply ability is registered");

        assert_eq!(render.category, AbilityCategory::Maintenance);
        assert_eq!(apply.category, AbilityCategory::Maintenance);
        assert!(render.policy.may_publish);
        assert!(!apply.policy.may_publish);
        assert_eq!(render.policy.mcp_exposure, McpExposure::None);
        assert_eq!(apply.policy.mcp_exposure, McpExposure::None);
        assert!(render.policy.allowed_actors.contains(&ActorKind::User));
        assert!(apply.policy.allowed_actors.contains(&ActorKind::User));
        assert!(!render
            .policy
            .allowed_actors
            .contains(&ActorKind::SurfaceClient));
        assert!(!apply
            .policy
            .allowed_actors
            .contains(&ActorKind::SurfaceClient));
        assert!(render.policy.required_scopes.is_empty());
        assert!(apply.policy.required_scopes.is_empty());
    }

    #[test]
    fn subject_for_value_maps_entity_subjects() {
        assert_eq!(
            subject_for_value(&serde_json::json!({"kind": "account", "id": "acct-1"})),
            SubjectRef::Account("acct-1".to_string())
        );
        assert_eq!(
            subject_for_value(&serde_json::json!({"kind": "project", "id": "proj-1"})),
            SubjectRef::Project("proj-1".to_string())
        );
    }
}

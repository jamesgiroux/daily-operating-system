use crate::abilities::provenance::trust::claim_trust_band_from_score;
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SubjectAttribution, SubjectRef,
};
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};
use crate::sensitivity::{renderable_claim_text_with_value, RenderActor, RenderSurface};
use crate::services::workspace_intake::{WorkspaceIntakeError, WorkspaceIntakeRequest};
use crate::types::IntelligenceClaim;

use super::contracts::{
    EntityIntakeClaim, EntityIntakeInput, EntityIntakeOutput, EntityIntakeRenderInput,
};

pub async fn entity_intake(
    ctx: &AbilityContext<'_>,
    input: EntityIntakeInput,
) -> AbilityResult<EntityIntakeOutput> {
    let entity_seed = input.entity_seed.clone();
    let receipt = ctx
        .services()
        .workspace_intake()
        .ok_or_else(|| {
            hard_error(
                "WorkspaceIntakeUnavailable",
                "workspace intake service unavailable",
            )
        })?
        .ingest(
            ctx,
            WorkspaceIntakeRequest {
                file_ref: input.file_ref,
                source_type_slug: "entity_doc".to_string(),
                entity: input.entity_seed,
                mode_slug: "entity_seeded".to_string(),
                category_slug: input.category,
            },
        )
        .await
        .map_err(workspace_intake_error)?;

    let claims = match entity_seed {
        Some(entity) => {
            read_claims_for_block_render(ctx, &entity.entity_type_slug, &entity.entity_id).await?
        }
        None => Vec::new(),
    };

    finalize_entity_intake_output(
        ctx,
        "entity_intake",
        1,
        AbilityCategory::Transform,
        SubjectRef::Global,
        EntityIntakeOutput {
            run_id: receipt.run_id,
            file_id: receipt.file_id,
            resolved_path: receipt.resolved_path,
            claims,
        },
    )
}

pub async fn entity_intake_render(
    ctx: &AbilityContext<'_>,
    input: EntityIntakeRenderInput,
) -> AbilityResult<EntityIntakeOutput> {
    let claims = read_claims_for_block_render(ctx, &input.entity_type, &input.entity_id).await?;
    finalize_entity_intake_output(
        ctx,
        "entity_intake_render",
        1,
        AbilityCategory::Read,
        SubjectRef::Global,
        EntityIntakeOutput {
            run_id: String::new(),
            file_id: input.file_ref,
            resolved_path: None,
            claims,
        },
    )
}

async fn read_claims_for_block_render(
    ctx: &AbilityContext<'_>,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<EntityIntakeClaim>, AbilityError> {
    let claims = ctx
        .services()
        .read_entity_context_claims(
            entity_type.to_string(),
            entity_id.to_string(),
            ctx.entity_context_claim_surface(),
            2,
        )
        .await
        .map_err(|error| hard_error("EntityIntakeClaimReadFailed", error))?;

    let render_actor = render_actor_for_context(ctx);
    Ok(claims
        .into_iter()
        .filter_map(|claim| project_claim(claim, &render_actor))
        .collect())
}

fn project_claim(
    claim: IntelligenceClaim,
    render_actor: &RenderActor,
) -> Option<EntityIntakeClaim> {
    let rendered = renderable_claim_text_with_value(
        &claim,
        &claim.text,
        RenderSurface::TauriEntityDetail,
        render_actor,
    )?;

    Some(EntityIntakeClaim {
        claim_id: claim.id,
        display_text: rendered.text,
        trust_band: claim_trust_band_from_score(claim.trust_score),
        sensitivity: claim.sensitivity,
    })
}

fn finalize_entity_intake_output(
    ctx: &AbilityContext<'_>,
    ability_name: &'static str,
    ability_schema_version: u32,
    category: AbilityCategory,
    subject: SubjectRef,
    output: EntityIntakeOutput,
) -> AbilityResult<EntityIntakeOutput> {
    let mut config = ProvenanceBuilderConfig::new(ability_name, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(ability_schema_version);
    config.actor = provenance_actor(ctx.actor.clone(), ability_name);
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = category;

    let mut builder = ProvenanceBuilder::new(config);
    let subject_attr = SubjectAttribution::direct_confident(subject);
    builder.set_subject(subject_attr.clone());
    builder
        .attribute_subtree(
            FieldPath::new("").map_err(field_error)?,
            FieldAttribution::constant(subject_attr),
        )
        .map_err(provenance_error)?;
    builder.finalize(output).map_err(provenance_error)
}

fn workspace_intake_error(error: WorkspaceIntakeError) -> AbilityError {
    match error {
        WorkspaceIntakeError::InvalidEntityTypeSlug(value) => {
            hard_error("InvalidEntityType", value)
        }
        WorkspaceIntakeError::InvalidEntityId => {
            hard_error("InvalidEntityId", "entity_id is invalid")
        }
        WorkspaceIntakeError::EntityNotFound => hard_error("EntityNotFound", "entity not found"),
        WorkspaceIntakeError::InvalidCategorySlug(value) => {
            hard_error("InvalidCategorySlug", value)
        }
        WorkspaceIntakeError::CategoryNotAllowed { allowed } => hard_error(
            "CategoryNotAllowed",
            format!("category not allowed; allowed={}", allowed.join(",")),
        ),
        WorkspaceIntakeError::FileNotFound => hard_error("FileNotFound", "file not found"),
        WorkspaceIntakeError::PathTraversalAttempt | WorkspaceIntakeError::OutsideWorkspace => {
            hard_error("PathTraversalAttempt", "file_ref is outside the workspace")
        }
        WorkspaceIntakeError::ManagedOutputRoot => hard_error(
            "ManagedOutputRoot",
            "file_ref points at DailyOS managed output, not source evidence",
        ),
        WorkspaceIntakeError::InvalidSourceTypeSlug(value) => {
            hard_error("InvalidSourceType", value)
        }
        WorkspaceIntakeError::InvalidModeSlug(value) => hard_error("InvalidMode", value),
        WorkspaceIntakeError::InvalidEntityName(value) => hard_error("InvalidEntityName", value),
        WorkspaceIntakeError::SymlinkRaced => hard_error("PathTraversalAttempt", "symlink race"),
        WorkspaceIntakeError::FileTooLarge => hard_error("IngestionFailed", "file too large"),
        WorkspaceIntakeError::UnsupportedFormat => {
            hard_error("IngestionFailed", "unsupported format")
        }
        WorkspaceIntakeError::AlreadyProcessed { existing_run_id } => {
            hard_error("IngestionFailed", existing_run_id)
        }
        WorkspaceIntakeError::Io(message) | WorkspaceIntakeError::DbError(message) => {
            hard_error("IngestionFailed", message)
        }
    }
}

fn render_actor_for_context(ctx: &AbilityContext<'_>) -> RenderActor {
    match &ctx.actor {
        Actor::User | Actor::Admin | Actor::SurfaceClient { .. } => {
            RenderActor::user("surface", None::<String>)
        }
        Actor::Agent => RenderActor::agent("agent"),
        Actor::System => RenderActor::agent("system"),
        Actor::McpClient { .. } => RenderActor::agent("mcp_client"),
    }
}

fn provenance_actor(
    actor: Actor,
    ability_name: &'static str,
) -> crate::abilities::provenance::Actor {
    match actor {
        Actor::User | Actor::Admin => crate::abilities::provenance::Actor::User,
        Actor::Agent => crate::abilities::provenance::Actor::Agent {
            name: format!("agent:{ability_name}"),
            version: "unknown".to_string(),
        },
        Actor::System => crate::abilities::provenance::Actor::System {
            component: format!("system:{ability_name}"),
        },
        Actor::SurfaceClient { .. } => crate::abilities::provenance::Actor::System {
            component: "surface_client".to_string(),
        },
        Actor::McpClient { .. } => crate::abilities::provenance::Actor::Agent {
            name: "mcp".to_string(),
            version: "unknown".to_string(),
        },
    }
}

fn hard_error(code: impl Into<String>, message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::HardError(code.into()),
        message: message.into(),
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

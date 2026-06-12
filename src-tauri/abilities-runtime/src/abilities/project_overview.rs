use std::cmp::Ordering;
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use dailyos_abilities_macro::ability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::abilities::claims::{metadata_for_name, ClaimType, FreshnessDecayClass};
use crate::abilities::composition::{
    AbilityRef, BindingRole, Block, BlockId, BlockType, ClaimRef, Composition, CompositionDocId,
    CompositionKind, CompositionMetadata, CompositionVersion, EntityRef, FieldBinding,
    ProvenanceRef, Salience, SalienceBand, Section, SectionId, SectionLayout,
};
use crate::abilities::provenance::source_time::{parse_source_timestamp, SourceTimestampStatus};
use crate::abilities::provenance::trust::{claim_trust_band_from_score, most_cautious_trust_band};
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, Confidence, DataSource, EntityId, FieldAttribution,
    FieldPath, GleanDownstream, InputsSnapshot, InvocationId, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SourceAttribution, SourceIdentifier, SourceName,
    SourceRef, SubjectAttribution, SubjectRef,
};
use crate::abilities::trust::TrustBand;
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};
use crate::services::context::{
    CompositionCommitError, CompositionProposal, ProjectCompositionProvenanceKind,
    ProjectCompositionSnapshot, ProjectCompositionSnapshotField,
    ProjectCompositionSnapshotReadError,
};
use crate::types::{
    prompt_input_sensitivity_allowed, subject_ref_from_json, ClaimState, ClaimSubjectRef,
    IntelligenceClaim, SurfacingState,
};

const ABILITY_NAME: &str = "dailyos/project-overview";
const ABILITY_SCHEMA_VERSION: u32 = 1;
const PROJECT_CLAIM_DEPTH: usize = 3;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ProjectOverviewInput {
    pub schema_version: u32,
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub expected_composition_version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition_id: Option<String>,
}

#[derive(Debug, Clone)]
struct NormalizedInput {
    project_id: String,
    expected_composition_version: u64,
    composition_id: CompositionDocId,
}

struct PreparedProjectOverview {
    proposal: CompositionProposal,
    provenance_builder: ProvenanceBuilder,
}

#[derive(Debug, Clone)]
struct ClaimProjection {
    claim: IntelligenceClaim,
    claim_type: ClaimType,
    placement: ClaimPlacement,
    source_index: crate::abilities::provenance::SourceIndex,
    trust_band: TrustBand,
    rendered_text: String,
    parsed_source_asof: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct SnapshotReadOutcome {
    snapshot: Option<ProjectCompositionSnapshot>,
    degraded_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaimPlacement {
    Momentum,
    Risk,
    Commitment,
    Relationship,
    Context,
    Ignored,
}

#[ability(
    name = "dailyos/project-overview",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, SurfaceClient],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.project_overview"],
    mcp_exposure = Invocable,
    client_side_executable = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [
        "claim.version",
        "project_subject.claim_changed",
        "claim.lifecycle",
        "claim.dismissal",
        "source.freshness",
        "source.revocation",
        "project.field_changed"
    ], coalesce = true }
)]
pub async fn project_overview(
    ctx: &AbilityContext<'_>,
    input: ProjectOverviewInput,
) -> AbilityResult<Composition> {
    let input = normalize_input(input)?;
    let prepared = prepare_project_overview(ctx, &input).await?;
    let committed = ctx
        .services()
        .commit_composition(prepared.proposal)
        .await
        .map_err(composition_commit_error)?;
    let output = prepared
        .provenance_builder
        .finalize(committed.composition)
        .map_err(provenance_error)?;
    validate_block_provenance(output.data(), output.provenance())?;
    Ok(output)
}

fn normalize_input(input: ProjectOverviewInput) -> Result<NormalizedInput, AbilityError> {
    if input.schema_version != ABILITY_SCHEMA_VERSION {
        return Err(validation_error(format!(
            "unsupported schema_version `{}` for `{ABILITY_NAME}`",
            input.schema_version
        )));
    }
    let project_id = input.project_id.trim();
    if project_id.is_empty() {
        return Err(validation_error("project_id must be non-empty"));
    }
    validate_entity_envelope(
        "project",
        project_id,
        input.entity_type.as_deref(),
        input.entity_id.as_deref(),
    )?;
    let composition_id = input
        .composition_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("dailyos/project-overview:project:{project_id}"));

    Ok(NormalizedInput {
        project_id: project_id.to_string(),
        expected_composition_version: input.expected_composition_version,
        composition_id: CompositionDocId::new(composition_id),
    })
}

fn validate_entity_envelope(
    expected_type: &str,
    expected_id: &str,
    entity_type: Option<&str>,
    entity_id: Option<&str>,
) -> Result<(), AbilityError> {
    if let Some(value) = entity_type {
        let value = value.trim();
        if value != expected_type {
            return Err(validation_error(format!(
                "entity_type must be `{expected_type}`"
            )));
        }
    }
    if let Some(value) = entity_id {
        let value = value.trim();
        if value != expected_id {
            return Err(validation_error(format!(
                "entity_id must match {expected_type}_id"
            )));
        }
    }
    Ok(())
}

async fn prepare_project_overview(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
) -> Result<PreparedProjectOverview, AbilityError> {
    let claims = ctx
        .services()
        .read_entity_context_claims(
            "project".to_string(),
            input.project_id.clone(),
            ctx.entity_context_claim_surface(),
            PROJECT_CLAIM_DEPTH,
        )
        .await
        .map_err(|error| hard_error("project_overview_claim_read", error))?;

    let subject_ref = SubjectRef::Project(input.project_id.clone());
    let subject = SubjectAttribution::direct_confident(subject_ref);
    let provenance_config = provenance_config(ctx);
    let invocation_id = provenance_config.invocation_id;
    let mut provenance_builder = ProvenanceBuilder::new(provenance_config);
    provenance_builder.set_subject(subject.clone());

    let mut projections = Vec::new();
    for claim in claims {
        let Some(projection) =
            project_claim(ctx, &input.project_id, claim, &mut provenance_builder)?
        else {
            continue;
        };
        projections.push(projection);
    }
    projections.sort_by(compare_claim_projection);

    let snapshot = read_project_snapshot(ctx, input).await?;
    let composition = build_composition(
        ctx,
        input,
        &projections,
        &snapshot,
        &subject,
        invocation_id,
        &mut provenance_builder,
    )?;

    Ok(PreparedProjectOverview {
        proposal: CompositionProposal {
            composition_id: input.composition_id.clone(),
            expected_composition_version: input.expected_composition_version,
            composition,
        },
        provenance_builder,
    })
}

async fn read_project_snapshot(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
) -> Result<SnapshotReadOutcome, AbilityError> {
    match ctx
        .services()
        .read_project_composition_snapshot(
            input.project_id.clone(),
            ctx.entity_context_claim_surface(),
        )
        .await
    {
        Ok(snapshot) => Ok(SnapshotReadOutcome {
            snapshot: Some(snapshot),
            degraded_reason: None,
        }),
        Err(ProjectCompositionSnapshotReadError::ProjectNotFound(project_id)) => Err(
            validation_error(format!("project `{project_id}` was not found")),
        ),
        Err(ProjectCompositionSnapshotReadError::ReadFailed(_)) => Ok(SnapshotReadOutcome {
            snapshot: None,
            degraded_reason: Some("project_snapshot_unavailable".to_string()),
        }),
    }
}

fn project_claim(
    ctx: &AbilityContext<'_>,
    project_id: &str,
    claim: IntelligenceClaim,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Option<ClaimProjection>, AbilityError> {
    if !claim_is_eligible_for_project_overview(&claim, project_id)? {
        return Ok(None);
    }
    let Some(metadata) = metadata_for_name(&claim.claim_type) else {
        return Err(validation_error(format!(
            "unknown claim_type `{}` in project overview input",
            claim.claim_type
        )));
    };
    let placement = placement_for_claim_type(metadata.kind);
    if placement == ClaimPlacement::Ignored {
        return Ok(None);
    }
    let rendered_text = claim.text.trim().to_string();
    if rendered_text.is_empty() {
        return Ok(None);
    }

    let source = source_for_claim(ctx, project_id, &claim)?;
    let parsed_source_asof = source.source_asof;
    let source_index = provenance_builder.add_source(source);
    let trust_band = resolved_claim_trust_band(&claim, metadata.kind, ctx.services().clock.now());
    provenance_builder.set_source_trust_band(source_index, trust_band);

    Ok(Some(ClaimProjection {
        claim,
        claim_type: metadata.kind,
        placement,
        source_index,
        trust_band,
        rendered_text,
        parsed_source_asof,
    }))
}

fn claim_is_eligible_for_project_overview(
    claim: &IntelligenceClaim,
    project_id: &str,
) -> Result<bool, AbilityError> {
    if claim.claim_state != ClaimState::Active || claim.surfacing_state != SurfacingState::Active {
        return Ok(false);
    }
    if claim.superseded_by.is_some()
        || claim.retraction_reason.is_some()
        || claim
            .demotion_reason
            .as_deref()
            .is_some_and(|reason| reason.eq_ignore_ascii_case("dismissed"))
    {
        return Ok(false);
    }
    if !prompt_input_sensitivity_allowed(&claim.sensitivity) {
        return Ok(false);
    }

    let value: Value = serde_json::from_str(&claim.subject_ref)
        .map_err(|error| validation_error(format!("invalid claim subject_ref JSON: {error}")))?;
    match subject_ref_from_json(&value)
        .map_err(|error| validation_error(format!("invalid claim subject_ref: {error}")))?
    {
        ClaimSubjectRef::Project { id } => Ok(id == project_id),
        ClaimSubjectRef::Account { .. }
        | ClaimSubjectRef::Action { .. }
        | ClaimSubjectRef::Person { .. }
        | ClaimSubjectRef::Meeting { .. }
        | ClaimSubjectRef::Email { .. }
        | ClaimSubjectRef::Multi(_)
        | ClaimSubjectRef::Global => Ok(false),
    }
}

fn placement_for_claim_type(kind: ClaimType) -> ClaimPlacement {
    match kind {
        ClaimType::Risk | ClaimType::EntityRisk => ClaimPlacement::Risk,
        ClaimType::Win
        | ClaimType::EntityWin
        | ClaimType::ValueDelivered
        | ClaimType::EntityCurrentState
        | ClaimType::EntitySummary => ClaimPlacement::Momentum,
        ClaimType::Commitment | ClaimType::OpenLoop | ClaimType::Recommendation => {
            ClaimPlacement::Commitment
        }
        ClaimType::StakeholderEngagement
        | ClaimType::StakeholderAssessment
        | ClaimType::StakeholderRole => ClaimPlacement::Relationship,
        ClaimType::CompanyContext
        | ClaimType::AccountFact
        | ClaimType::EntityIdentity
        | ClaimType::UserNote => ClaimPlacement::Context,
        ClaimType::LinkingDismissed
        | ClaimType::EmailDismissed
        | ClaimType::IntelligenceFieldDismissed
        | ClaimType::FeedbackFieldDismissed
        | ClaimType::TriageSnooze
        | ClaimType::MeetingEntityDismissed
        | ClaimType::AccountFieldCorrection
        | ClaimType::DismissedItem
        | ClaimType::BriefingCalloutDismissed
        | ClaimType::NudgeDismissed
        | ClaimType::MeetingReadiness
        | ClaimType::MeetingTopic
        | ClaimType::MeetingEventNote
        | ClaimType::AttendeeContext
        | ClaimType::MeetingChangeMarker
        | ClaimType::SuggestedOutcome => ClaimPlacement::Ignored,
    }
}

fn build_composition(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    projections: &[ClaimProjection],
    snapshot_outcome: &SnapshotReadOutcome,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Composition, AbilityError> {
    let snapshot = snapshot_outcome.snapshot.as_ref();
    let mut sections = Vec::new();

    sections.push(variant_section(
        "headline",
        "Headline",
        vec![build_overview_block(
            ctx,
            input,
            projections,
            snapshot_outcome,
            "/sections/0/blocks/0",
            subject,
            invocation_id,
            provenance_builder,
        )?],
        SectionLayout::Stacked,
        salience(0.95, SalienceBand::Critical, "project masthead"),
    ));

    let mut next_section_index = 1;
    let portfolio_fields = snapshot_fields_for_section(snapshot, "portfolio");
    if snapshot.is_some_and(|snapshot| snapshot.is_parent) || !portfolio_fields.is_empty() {
        sections.push(variant_section(
            "portfolio",
            "Portfolio",
            build_claim_or_snapshot_section_blocks(
                ctx,
                input,
                "portfolio",
                next_section_index,
                Vec::new(),
                portfolio_fields,
                EmptySectionCopy {
                    title: "No portfolio signals",
                    body: "This project has no source-backed portfolio detail yet.",
                    status: "empty",
                },
                subject,
                invocation_id,
                provenance_builder,
            )?,
            SectionLayout::Grid,
            salience(0.74, SalienceBand::Contextual, "project portfolio"),
        ));
        next_section_index += 1;
    }

    let trajectory_claims = projections
        .iter()
        .filter(|projection| projection.placement == ClaimPlacement::Momentum)
        .collect::<Vec<_>>();
    sections.push(variant_section(
        "trajectory",
        "Trajectory",
        build_claim_or_snapshot_section_blocks(
            ctx,
            input,
            "trajectory",
            next_section_index,
            trajectory_claims,
            snapshot_fields_for_section(snapshot, "trajectory"),
            EmptySectionCopy {
                title: "No trajectory signals",
                body: "No active project trajectory claims are eligible for this surface.",
                status: "empty",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.86, SalienceBand::Important, "project trajectory"),
    ));
    next_section_index += 1;

    sections.push(variant_section(
        "the-horizon",
        "The horizon",
        build_claim_or_snapshot_section_blocks(
            ctx,
            input,
            "the-horizon",
            next_section_index,
            Vec::new(),
            snapshot_fields_for_section(snapshot, "the-horizon"),
            EmptySectionCopy {
                title: "No horizon signals",
                body: "Target-date and activity signals are not yet grounded for this project.",
                status: "source_gap",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.8, SalienceBand::Important, "project horizon"),
    ));
    next_section_index += 1;

    let landscape_claims = projections
        .iter()
        .filter(|projection| {
            matches!(
                projection.placement,
                ClaimPlacement::Risk | ClaimPlacement::Context
            )
        })
        .collect::<Vec<_>>();
    sections.push(variant_section(
        "the-landscape",
        "The landscape",
        build_claim_or_snapshot_section_blocks(
            ctx,
            input,
            "the-landscape",
            next_section_index,
            landscape_claims,
            snapshot_fields_for_section(snapshot, "the-landscape"),
            EmptySectionCopy {
                title: "Landscape not yet grounded",
                body: "No renderable project context or risk claims are available.",
                status: "needs_grounding",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.76, SalienceBand::Important, "project landscape"),
    ));
    next_section_index += 1;

    let room_claims = projections
        .iter()
        .filter(|projection| projection.placement == ClaimPlacement::Relationship)
        .collect::<Vec<_>>();
    sections.push(variant_section(
        "the-room",
        "The room",
        build_claim_or_snapshot_section_blocks(
            ctx,
            input,
            "the-room",
            next_section_index,
            room_claims,
            snapshot_fields_for_section(snapshot, "the-room"),
            EmptySectionCopy {
                title: "No team signals",
                body: "People and relationship inputs are not yet grounded for this project.",
                status: "empty",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Grid,
        salience(0.7, SalienceBand::Contextual, "project people"),
    ));
    next_section_index += 1;

    let next_claims = projections
        .iter()
        .filter(|projection| projection.placement == ClaimPlacement::Commitment)
        .collect::<Vec<_>>();
    sections.push(variant_section(
        "whats-next",
        "What's next",
        build_claim_or_snapshot_section_blocks(
            ctx,
            input,
            "whats-next",
            next_section_index,
            next_claims.clone(),
            Vec::new(),
            EmptySectionCopy {
                title: "No open next steps",
                body: "There are no renderable commitments or next-step records for this project.",
                status: "empty",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.78, SalienceBand::Important, "project next steps"),
    ));
    next_section_index += 1;

    sections.push(variant_section(
        "the-record",
        "The record",
        build_record_section_blocks(
            ctx,
            input,
            projections,
            snapshot_fields_for_section(snapshot, "the-record"),
            next_section_index,
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.58, SalienceBand::Contextual, "project evidence record"),
    ));
    next_section_index += 1;

    sections.push(variant_section(
        "the-work",
        "The work",
        build_claim_or_snapshot_section_blocks(
            ctx,
            input,
            "the-work",
            next_section_index,
            next_claims,
            snapshot_fields_for_section(snapshot, "the-work"),
            EmptySectionCopy {
                title: "No active work items",
                body: "No active project work records are currently grounded.",
                status: "empty",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.5, SalienceBand::Background, "project work"),
    ));

    let section_count = sections.len();
    let generated_at = ctx.services().clock.now();
    let composition = Composition::new(
        input.composition_id.clone(),
        CompositionKind::EntityPage,
        Some(EntityRef::new(format!("project:{}", input.project_id))),
        sections,
        salience(0.9, SalienceBand::Important, "project overview"),
        generated_at,
        AbilityRef::new(ABILITY_NAME),
        CompositionMetadata {
            schema_version: SchemaVersion(ABILITY_SCHEMA_VERSION),
            generated_at,
            composition_version: CompositionVersion::new(0),
            generated_by: ABILITY_NAME.to_string(),
        },
    );

    attribute_static_composition_fields(provenance_builder, subject, section_count)?;
    Ok(composition)
}

#[derive(Debug, Clone, Copy)]
struct EmptySectionCopy {
    title: &'static str,
    body: &'static str,
    status: &'static str,
}

fn variant_section(
    id: &str,
    label: &str,
    blocks: Vec<Block>,
    layout: SectionLayout,
    salience: Salience,
) -> Section {
    let mut section = Section::new(SectionId::new(id), blocks);
    section.label = Some(label.to_string());
    section.layout = layout;
    section.salience = salience;
    section
}

#[allow(clippy::too_many_arguments)]
fn build_claim_or_snapshot_section_blocks(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    section_id: &str,
    section_index: usize,
    projections: Vec<&ClaimProjection>,
    snapshot_fields: Vec<&ProjectCompositionSnapshotField>,
    empty_copy: EmptySectionCopy,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Vec<Block>, AbilityError> {
    let has_projections = !projections.is_empty();
    let mut blocks = if has_projections {
        build_projection_blocks(
            input,
            section_id,
            section_index,
            projections,
            subject,
            invocation_id,
            provenance_builder,
        )?
    } else {
        Vec::new()
    };
    if has_projections {
        if !snapshot_fields.is_empty() {
            let block_index = blocks.len();
            blocks.push(build_snapshot_fields_block(
                ctx,
                input,
                section_id,
                section_index,
                block_index,
                snapshot_fields,
                subject,
                invocation_id,
                provenance_builder,
            )?);
        }
        return Ok(blocks);
    }
    if !snapshot_fields.is_empty() {
        return Ok(vec![build_snapshot_fields_block(
            ctx,
            input,
            section_id,
            section_index,
            0,
            snapshot_fields,
            subject,
            invocation_id,
            provenance_builder,
        )?]);
    }
    Ok(vec![build_section_state_block(
        input,
        section_id,
        empty_copy,
        section_index,
        0,
        subject,
        invocation_id,
        provenance_builder,
    )?])
}

fn build_projection_blocks(
    input: &NormalizedInput,
    section_id: &str,
    section_index: usize,
    projections: Vec<&ClaimProjection>,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Vec<Block>, AbilityError> {
    let mut blocks = Vec::with_capacity(projections.len());
    for projection in projections {
        let block_index = blocks.len();
        blocks.push(build_claim_block(
            input,
            section_id,
            projection,
            subject,
            invocation_id,
            &format!("/sections/{section_index}/blocks/{block_index}"),
            provenance_builder,
        )?);
    }
    Ok(blocks)
}

#[allow(clippy::too_many_arguments)]
fn build_record_section_blocks(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    projections: &[ClaimProjection],
    snapshot_fields: Vec<&ProjectCompositionSnapshotField>,
    section_index: usize,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Vec<Block>, AbilityError> {
    if projections.is_empty() && snapshot_fields.is_empty() {
        return Ok(vec![build_section_state_block(
            input,
            "the-record",
            EmptySectionCopy {
                title: "No project record yet",
                body: "No source-backed project events are available for this record.",
                status: "empty",
            },
            section_index,
            0,
            subject,
            invocation_id,
            provenance_builder,
        )?]);
    }

    let mut claim_refs = Vec::new();
    let mut source_indexes = Vec::new();
    let mut items = Vec::new();
    for projection in projections {
        claim_refs.push(claim_ref_for_projection(projection)?);
        source_indexes.push(projection.source_index);
        items.push(json!({
            "label": projection.rendered_text,
            "source_label": projection.claim.data_source,
            "source_asof": projection.claim.source_asof,
        }));
    }
    source_indexes.extend(snapshot_source_indexes(
        ctx,
        input,
        &snapshot_fields,
        provenance_builder,
    )?);
    for field in snapshot_fields {
        items.extend(snapshot_field_evidence_items(field));
    }

    let composition_block_path = format!("/sections/{section_index}/blocks/0");
    let item_count = items.len();
    let mut block = Block::new(
        BlockId::new(block_id(input, "the-record", "evidence_list", "sources")),
        BlockType::EvidenceList,
        json!({ "title": "Evidence", "items": items }),
        claim_refs,
        ProvenanceRef::new(
            invocation_id,
            FieldPath::new(&composition_block_path).map_err(field_error)?,
        ),
        None,
    )
    .map_err(block_error)?;
    block.field_bindings = evidence_list_display_bindings(item_count)?;
    block.salience = salience(0.58, SalienceBand::Contextual, "source record");
    attribute_block(
        provenance_builder,
        &composition_block_path,
        subject,
        source_indexes,
    )?;
    Ok(vec![block])
}

#[allow(clippy::too_many_arguments)]
fn build_snapshot_fields_block(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    section_id: &str,
    section_index: usize,
    block_index: usize,
    fields: Vec<&ProjectCompositionSnapshotField>,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let source_indexes = snapshot_source_indexes(ctx, input, &fields, provenance_builder)?;
    let items = fields
        .iter()
        .flat_map(|field| snapshot_field_evidence_items(field))
        .collect::<Vec<_>>();
    let composition_block_path = format!("/sections/{section_index}/blocks/{block_index}");
    let item_count = items.len();
    let mut block = Block::new(
        BlockId::new(block_id(input, section_id, "evidence_list", "snapshot")),
        BlockType::EvidenceList,
        json!({ "title": section_title(section_id), "items": items }),
        Vec::new(),
        ProvenanceRef::new(
            invocation_id,
            FieldPath::new(&composition_block_path).map_err(field_error)?,
        ),
        None,
    )
    .map_err(block_error)?;
    block.field_bindings = evidence_list_display_bindings(item_count)?;
    block.salience = salience(0.54, SalienceBand::Contextual, "snapshot fields");
    attribute_block(
        provenance_builder,
        &composition_block_path,
        subject,
        source_indexes,
    )?;
    Ok(block)
}

fn section_title(section_id: &str) -> &'static str {
    match section_id {
        "portfolio" => "Portfolio",
        "trajectory" => "Trajectory",
        "the-horizon" => "Horizon",
        "the-landscape" => "Landscape",
        "the-room" => "Team",
        "the-record" => "Record",
        "the-work" => "Work",
        _ => "Evidence",
    }
}

fn snapshot_field_evidence_items(field: &ProjectCompositionSnapshotField) -> Vec<Value> {
    let source_label = field
        .source_label
        .as_deref()
        .unwrap_or(match field.provenance_kind {
            ProjectCompositionProvenanceKind::NonSensitiveIdentity => "identity",
            ProjectCompositionProvenanceKind::ManualUser => "user",
            ProjectCompositionProvenanceKind::SourceField => "source",
            ProjectCompositionProvenanceKind::SystemConfig => "system_config",
            ProjectCompositionProvenanceKind::Derived => "derived",
            ProjectCompositionProvenanceKind::Unavailable => "unavailable",
        });
    match &field.value {
        Value::Array(items) => items
            .iter()
            .take(12)
            .map(|item| {
                json!({
                    "label": format!("{}: {}", field.label, compact_item_label(item)),
                    "source_label": source_label,
                    "source_asof": field.source_asof,
                })
            })
            .collect(),
        _ => vec![json!({
            "label": format!("{}: {}", field.label, snapshot_value_text(&field.value)),
            "source_label": source_label,
            "source_asof": field.source_asof,
        })],
    }
}

fn compact_item_label(value: &Value) -> String {
    let Some(object) = value.as_object() else {
        return snapshot_value_text(value);
    };
    for key in ["title", "name", "signal_text", "content"] {
        if let Some(label) = object.get(key).and_then(Value::as_str) {
            if !label.trim().is_empty() {
                return label.to_string();
            }
        }
    }
    snapshot_value_text(value)
}

#[allow(clippy::too_many_arguments)]
fn build_section_state_block(
    input: &NormalizedInput,
    section_id: &str,
    copy: EmptySectionCopy,
    section_index: usize,
    block_index: usize,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let composition_block_path = format!("/sections/{section_index}/blocks/{block_index}");
    let mut block = Block::new(
        BlockId::new(block_id(input, section_id, "empty_state", copy.status)),
        BlockType::ClaimSummary,
        json!({
            "title": copy.title,
            "body": copy.body,
            "status": copy.status,
            "empty_state": true,
            "trust_band": "needs_verification",
        }),
        Vec::new(),
        ProvenanceRef::new(
            invocation_id,
            FieldPath::new(&composition_block_path).map_err(field_error)?,
        ),
        None,
    )
    .map_err(block_error)?;
    block.field_bindings = vec![
        display_only_binding("/title")?,
        display_only_binding("/body")?,
        display_only_binding("/status")?,
        display_only_binding("/empty_state")?,
    ];
    block.salience = salience(0.28, SalienceBand::Background, copy.status);
    attribute_block(
        provenance_builder,
        &composition_block_path,
        subject,
        Vec::new(),
    )?;
    Ok(block)
}

#[allow(clippy::too_many_arguments)]
fn build_overview_block(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    projections: &[ClaimProjection],
    snapshot_outcome: &SnapshotReadOutcome,
    composition_block_path: &str,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let snapshot = snapshot_outcome.snapshot.as_ref();
    let headline_fields = snapshot_fields_for_section(snapshot, "headline");
    let mut source_indexes = projections
        .iter()
        .map(|projection| projection.source_index)
        .collect::<Vec<_>>();
    source_indexes.extend(snapshot_source_indexes(
        ctx,
        input,
        &headline_fields,
        provenance_builder,
    )?);
    let overview_claims = projections
        .iter()
        .enumerate()
        .filter(|(_, projection)| {
            matches!(
                projection.placement,
                ClaimPlacement::Momentum | ClaimPlacement::Context
            )
        })
        .collect::<Vec<_>>();
    let all_refs = projections
        .iter()
        .map(claim_ref_for_projection)
        .collect::<Result<Vec<_>, _>>()?;
    let context = overview_claims
        .iter()
        .map(|(_, projection)| {
            json!({
                "claim_id": projection.claim.id,
                "text": projection.rendered_text,
                "trust_band": trust_band_label(projection.trust_band),
                "source_asof": projection.claim.source_asof,
            })
        })
        .collect::<Vec<_>>();
    let overview_claim_indexes = overview_claims
        .iter()
        .map(|(projection_index, _)| *projection_index)
        .collect::<Vec<_>>();
    let trust_band = block_trust_band(projections.iter().map(|projection| projection.trust_band));
    let counts_by_band = trust_band_counts(projections);
    let project_display_name = snapshot
        .map(|snapshot| snapshot_value_text(&snapshot.display_name.value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| input.project_id.clone());
    let status = snapshot
        .and_then(|snapshot| snapshot.status.as_ref())
        .filter(|field| field.sensitivity.is_render_safe())
        .map(|field| snapshot_value_text(&field.value));
    let vitals = headline_fields
        .iter()
        .map(|field| {
            json!({
                "label": field.label,
                "value": snapshot_value_text(&field.value),
                "source_label": field.source_label,
                "source_asof": field.source_asof,
                "trust_band": trust_band_label(field.trust_band),
            })
        })
        .collect::<Vec<_>>();
    let vitals_len = vitals.len();
    let attributes = json!({
        "entity_type": "project",
        "project_id": input.project_id,
        "account": {
            "id": input.project_id,
            "display_name": project_display_name,
            "type": "Project",
        },
        "project": {
            "id": input.project_id,
            "display_name": project_display_name,
            "status": status,
        },
        "title": "Project overview",
        "summary": overview_claims
            .first()
            .map(|(_, projection)| projection.rendered_text.as_str())
            .unwrap_or("Project composition is grounded in current renderable claims and source-backed project fields."),
        "claim_count": projections.len(),
        "trust_band": trust_band_label(trust_band),
        "counts_by_trust_band": counts_by_band,
        "context": context,
        "vitals": vitals,
        "snapshot_degraded": snapshot_outcome.degraded_reason.as_deref().unwrap_or(""),
    });
    let mut block = Block::new(
        BlockId::new(block_id(input, "headline", "account_overview", "summary")),
        BlockType::AccountOverview,
        attributes,
        all_refs,
        ProvenanceRef::new(
            invocation_id,
            FieldPath::new(composition_block_path).map_err(field_error)?,
        ),
        None,
    )
    .map_err(block_error)?;
    block.salience = salience(0.95, SalienceBand::Critical, "summary");

    let mut display_bindings = vec![
        display_only_binding("/title")?,
        display_only_binding("/account/id")?,
        display_only_binding("/account/display_name")?,
        display_only_binding("/account/type")?,
        display_only_binding("/project/id")?,
        display_only_binding("/project/display_name")?,
        display_only_binding("/project/status")?,
        display_only_binding("/snapshot_degraded")?,
    ];
    display_bindings.extend(vitals_display_bindings(vitals_len)?);

    if projections.is_empty() {
        block.field_bindings = display_bindings;
    } else {
        let mut bindings = vec![computed_binding("/claim_count", 0..projections.len())?];
        bindings.extend(trust_band_count_computed_bindings(projections.len())?);
        bindings.extend(context_computed_bindings(&overview_claim_indexes)?);
        bindings.extend(display_bindings);
        block.field_bindings = bindings;
    }
    attribute_block(
        provenance_builder,
        composition_block_path,
        subject,
        source_indexes,
    )?;
    Ok(block)
}

fn build_claim_block(
    input: &NormalizedInput,
    section_id: &str,
    projection: &ClaimProjection,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    composition_block_path: &str,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let claim_ref = claim_ref_for_projection(projection)?;
    let trust_band = trust_band_label(projection.trust_band);
    let (block_type, attributes, bindings, salience_value, salience_band, salience_reason) =
        match projection.placement {
            ClaimPlacement::Risk => (
                BlockType::RiskCallout,
                json!({
                    "claim_id": projection.claim.id,
                    "text": projection.rendered_text,
                    "claim_type": projection.claim.claim_type,
                    "trust_band": trust_band,
                    "source_asof": projection.claim.source_asof,
                }),
                source_feedback_computed_bindings("/text", "/trust_band")?,
                0.9,
                SalienceBand::Critical,
                "risk claim",
            ),
            ClaimPlacement::Commitment => (
                BlockType::ActionList,
                json!({
                    "title": "Actions",
                    "items": [{
                        "claim_id": projection.claim.id,
                        "text": projection.rendered_text,
                        "trust_band": trust_band,
                        "source_asof": projection.claim.source_asof,
                    }],
                    "claim_type": projection.claim.claim_type,
                    "trust_band": trust_band,
                    "source_asof": projection.claim.source_asof,
                }),
                source_feedback_computed_bindings("/items/0/text", "/items/0/trust_band")?,
                0.78,
                SalienceBand::Important,
                "commitment claim",
            ),
            ClaimPlacement::Relationship => (
                BlockType::RelationshipMap,
                json!({
                    "nodes": [{
                        "claim_id": projection.claim.id,
                        "text": projection.rendered_text,
                        "trust_band": trust_band,
                        "source_asof": projection.claim.source_asof,
                    }],
                    "claim_type": projection.claim.claim_type,
                }),
                source_feedback_computed_bindings("/nodes/0/text", "/nodes/0/trust_band")?,
                0.62,
                SalienceBand::Contextual,
                "relationship claim",
            ),
            ClaimPlacement::Momentum | ClaimPlacement::Context => (
                BlockType::ClaimSummary,
                json!({
                    "intent": if projection.placement == ClaimPlacement::Momentum { "trajectory" } else { "context" },
                    "claim_id": projection.claim.id,
                    "text": projection.rendered_text,
                    "claim_type": projection.claim.claim_type,
                    "trust_band": trust_band,
                    "source_asof": projection.claim.source_asof,
                }),
                source_feedback_computed_bindings("/text", "/trust_band")?,
                0.66,
                SalienceBand::Important,
                "project claim",
            ),
            ClaimPlacement::Ignored => {
                return Err(validation_error(
                    "unexpected project overview block placement",
                ));
            }
        };

    let mut block = Block::new(
        BlockId::new(block_id(
            input,
            section_id,
            block_type.type_id(),
            &projection.claim.id,
        )),
        block_type,
        attributes,
        vec![claim_ref],
        ProvenanceRef::new(
            invocation_id,
            FieldPath::new(composition_block_path).map_err(field_error)?,
        ),
        None,
    )
    .map_err(block_error)?;
    block.field_bindings = bindings;
    block.salience = salience(salience_value, salience_band, salience_reason);
    attribute_block(
        provenance_builder,
        composition_block_path,
        subject,
        vec![projection.source_index],
    )?;
    Ok(block)
}

fn snapshot_fields_for_section<'a>(
    snapshot: Option<&'a ProjectCompositionSnapshot>,
    section_id: &str,
) -> Vec<&'a ProjectCompositionSnapshotField> {
    let Some(snapshot) = snapshot else {
        return Vec::new();
    };
    snapshot
        .fields
        .iter()
        .filter(|field| field.sensitivity.is_render_safe())
        .filter(|field| snapshot_field_belongs_to_section(&field.field_path, section_id))
        .collect()
}

fn snapshot_field_belongs_to_section(field_path: &str, section_id: &str) -> bool {
    match section_id {
        "headline" => field_path.starts_with("/vitals/") || field_path.starts_with("/identity/"),
        "portfolio" => field_path.starts_with("/portfolio/"),
        "trajectory" => field_path.starts_with("/trajectory/"),
        "the-horizon" => field_path.starts_with("/the-horizon/"),
        "the-landscape" => field_path.starts_with("/state/"),
        "the-room" => field_path.starts_with("/the-room/"),
        "the-record" => field_path.starts_with("/the-record/"),
        "the-work" => field_path.starts_with("/the-work/"),
        _ => false,
    }
}

fn snapshot_source_indexes(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    fields: &[&ProjectCompositionSnapshotField],
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Vec<crate::abilities::provenance::SourceIndex>, AbilityError> {
    let mut indexes = Vec::new();
    for field in fields {
        let Some(source) = source_for_snapshot_field(ctx, input, field)? else {
            continue;
        };
        let index = provenance_builder.add_source(source);
        provenance_builder.set_source_trust_band(index, visible_trust_band(field.trust_band));
        indexes.push(index);
    }
    Ok(indexes)
}

fn source_for_snapshot_field(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    field: &ProjectCompositionSnapshotField,
) -> Result<Option<SourceAttribution>, AbilityError> {
    if matches!(
        field.provenance_kind,
        ProjectCompositionProvenanceKind::NonSensitiveIdentity
            | ProjectCompositionProvenanceKind::Unavailable
    ) && field.source_label.is_none()
        && field.source_ref.is_none()
        && field.source_asof.is_none()
    {
        return Ok(None);
    }

    let now = ctx.services().clock.now();
    let observed_at = match parse_source_timestamp(field.source_asof.as_deref(), now, None) {
        SourceTimestampStatus::Accepted(parsed)
        | SourceTimestampStatus::Implausible { parsed, .. } => parsed,
        SourceTimestampStatus::Malformed(_) | SourceTimestampStatus::Missing => now,
    };
    let source_asof = match parse_source_timestamp(field.source_asof.as_deref(), now, None) {
        SourceTimestampStatus::Accepted(parsed)
        | SourceTimestampStatus::Implausible { parsed, .. } => Some(parsed),
        SourceTimestampStatus::Malformed(_) | SourceTimestampStatus::Missing => None,
    };
    SourceAttribution::new(
        data_source_for_snapshot_field(field),
        vec![SourceIdentifier::Entity {
            entity_id: EntityId::new(input.project_id.clone()),
            field: Some(field.field_path.clone()),
        }],
        observed_at,
        source_asof,
        1.0,
        None,
    )
    .map(Some)
    .map_err(|error| validation_error(format!("invalid snapshot source attribution: {error}")))
}

fn data_source_for_snapshot_field(field: &ProjectCompositionSnapshotField) -> DataSource {
    match field.provenance_kind {
        ProjectCompositionProvenanceKind::ManualUser => DataSource::User,
        ProjectCompositionProvenanceKind::SystemConfig => DataSource::LocalEnrichment,
        _ => field
            .source_label
            .as_deref()
            .map(data_source_for_claim)
            .unwrap_or_else(|| DataSource::Other(SourceName::new("project_snapshot"))),
    }
}

fn snapshot_value_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

fn attribute_static_composition_fields(
    builder: &mut ProvenanceBuilder,
    subject: &SubjectAttribution,
    section_count: usize,
) -> Result<(), AbilityError> {
    for path in [
        "",
        "/id",
        "/kind/kind",
        "/subject",
        "/generated_at",
        "/generated_by",
        "/metadata",
        "/salience",
    ] {
        builder
            .attribute(
                FieldPath::new(path).map_err(field_error)?,
                FieldAttribution::constant(subject.clone()),
            )
            .map_err(provenance_error)?;
    }
    for section_index in 0..section_count {
        for suffix in ["id", "label", "layout", "salience"] {
            builder
                .attribute(
                    FieldPath::new(format!("/sections/{section_index}/{suffix}"))
                        .map_err(field_error)?,
                    FieldAttribution::constant(subject.clone()),
                )
                .map_err(provenance_error)?;
        }
    }
    builder
        .attribute(
            FieldPath::root(),
            FieldAttribution::constant(subject.clone()),
        )
        .map_err(provenance_error)?;
    builder
        .attribute(
            FieldPath::new("/metadata").map_err(field_error)?,
            FieldAttribution::constant(subject.clone()),
        )
        .map_err(provenance_error)?;
    Ok(())
}

fn attribute_block(
    builder: &mut ProvenanceBuilder,
    composition_block_path: &str,
    subject: &SubjectAttribution,
    source_indexes: Vec<crate::abilities::provenance::SourceIndex>,
) -> Result<(), AbilityError> {
    let path = FieldPath::new(composition_block_path).map_err(field_error)?;
    let attribution = if source_indexes.is_empty() {
        FieldAttribution::constant(subject.clone())
    } else if source_indexes.len() == 1 {
        FieldAttribution::direct(subject.clone(), source_indexes[0])
    } else {
        FieldAttribution::computed(
            subject.clone(),
            "dailyos.project_overview.v1",
            source_indexes
                .into_iter()
                .map(|source_index| SourceRef::Source { source_index })
                .collect(),
            Confidence::computed(1.0).map_err(field_error)?,
        )
        .map_err(field_error)?
    };
    builder
        .attribute(path, attribution)
        .map_err(provenance_error)?;
    Ok(())
}

fn source_feedback_computed_bindings(
    source_path: &str,
    computed_path: &str,
) -> Result<Vec<FieldBinding>, AbilityError> {
    Ok(vec![
        binding(source_path, BindingRole::Source, vec![0])?,
        binding(source_path, BindingRole::FeedbackTarget, vec![0])?,
        binding(computed_path, BindingRole::ComputedFrom, vec![0])?,
    ])
}

fn computed_binding(
    field_path: &str,
    indexes: std::ops::Range<usize>,
) -> Result<FieldBinding, AbilityError> {
    binding(field_path, BindingRole::ComputedFrom, indexes.collect())
}

fn computed_binding_for_indexes(
    field_path: &str,
    indexes: Vec<usize>,
) -> Result<FieldBinding, AbilityError> {
    binding(field_path, BindingRole::ComputedFrom, indexes)
}

fn display_only_binding(field_path: &str) -> Result<FieldBinding, AbilityError> {
    binding(field_path, BindingRole::DisplayOnly, Vec::new())
}

fn trust_band_count_computed_bindings(
    projection_count: usize,
) -> Result<Vec<FieldBinding>, AbilityError> {
    let indexes = (0..projection_count).collect::<Vec<_>>();
    let mut bindings = Vec::with_capacity(3);
    for field in [
        "/counts_by_trust_band/likely_current",
        "/counts_by_trust_band/use_with_caution",
        "/counts_by_trust_band/needs_verification",
    ] {
        bindings.push(computed_binding_for_indexes(field, indexes.clone())?);
    }
    Ok(bindings)
}

fn context_computed_bindings(
    overview_claim_indexes: &[usize],
) -> Result<Vec<FieldBinding>, AbilityError> {
    let mut bindings = Vec::with_capacity(overview_claim_indexes.len() * 4);
    for (item_index, projection_index) in overview_claim_indexes.iter().copied().enumerate() {
        for field in ["claim_id", "text", "trust_band", "source_asof"] {
            bindings.push(computed_binding_for_indexes(
                &format!("/context/{item_index}/{field}"),
                vec![projection_index],
            )?);
        }
    }
    Ok(bindings)
}

fn vitals_display_bindings(item_count: usize) -> Result<Vec<FieldBinding>, AbilityError> {
    let mut bindings = Vec::with_capacity(item_count * 5);
    for index in 0..item_count {
        for field in [
            "label",
            "value",
            "source_label",
            "source_asof",
            "trust_band",
        ] {
            bindings.push(display_only_binding(&format!("/vitals/{index}/{field}"))?);
        }
    }
    Ok(bindings)
}

fn evidence_list_display_bindings(item_count: usize) -> Result<Vec<FieldBinding>, AbilityError> {
    let mut bindings = Vec::with_capacity(item_count * 3 + 1);
    bindings.push(display_only_binding("/title")?);
    for index in 0..item_count {
        bindings.push(display_only_binding(&format!("/items/{index}/label"))?);
        bindings.push(display_only_binding(&format!(
            "/items/{index}/source_label"
        ))?);
        bindings.push(display_only_binding(&format!(
            "/items/{index}/source_asof"
        ))?);
    }
    Ok(bindings)
}

fn binding(
    field_path: &str,
    role: BindingRole,
    indexes: Vec<usize>,
) -> Result<FieldBinding, AbilityError> {
    Ok(FieldBinding {
        field_path: FieldPath::new(field_path).map_err(field_error)?,
        role,
        claim_refs: indexes
            .into_iter()
            .map(crate::abilities::composition::ClaimRefIndex)
            .collect(),
    })
}

fn claim_ref_for_projection(projection: &ClaimProjection) -> Result<ClaimRef, AbilityError> {
    Ok(ClaimRef::with_field(
        projection.claim.id.clone(),
        projection.claim.claim_version,
        claim_field_path(&projection.claim)?,
    ))
}

fn claim_field_path(claim: &IntelligenceClaim) -> Result<FieldPath, AbilityError> {
    let raw = claim
        .field_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("/text");
    let pointer = if raw.starts_with('/') {
        raw.to_string()
    } else {
        format!("/{raw}")
    };
    FieldPath::new(pointer).map_err(field_error)
}

fn resolved_claim_trust_band(
    claim: &IntelligenceClaim,
    claim_type: ClaimType,
    now: DateTime<Utc>,
) -> TrustBand {
    if claim.trust_score.is_none() {
        return TrustBand::NeedsVerification;
    }

    let score_band = visible_trust_band(claim_trust_band_from_score(claim.trust_score));
    if !freshness_cap_applies(claim_type) {
        return score_band;
    }

    let Some(source_asof) = parse_claim_source_asof(claim, now) else {
        return TrustBand::NeedsVerification;
    };
    let age_days = now.signed_duration_since(source_asof).num_days();
    if age_days < 7 {
        score_band
    } else if age_days <= 30 {
        block_trust_band([score_band, TrustBand::UseWithCaution])
    } else {
        TrustBand::NeedsVerification
    }
}

fn freshness_cap_applies(claim_type: ClaimType) -> bool {
    let metadata = crate::abilities::claims::metadata_for_claim_type(claim_type);
    !matches!(
        (claim_type, metadata.freshness_decay_class),
        (ClaimType::CompanyContext, _) | (_, FreshnessDecayClass::Static)
    )
}

fn visible_trust_band(band: TrustBand) -> TrustBand {
    match band {
        TrustBand::Unscored => TrustBand::NeedsVerification,
        other => other,
    }
}

fn block_trust_band(bands: impl IntoIterator<Item = TrustBand>) -> TrustBand {
    most_cautious_trust_band(bands.into_iter().map(visible_trust_band))
        .map(visible_trust_band)
        .unwrap_or(TrustBand::NeedsVerification)
}

fn trust_band_label(band: TrustBand) -> &'static str {
    match visible_trust_band(band) {
        TrustBand::LikelyCurrent => "likely_current",
        TrustBand::UseWithCaution => "use_with_caution",
        TrustBand::NeedsVerification | TrustBand::Unscored => "needs_verification",
    }
}

fn trust_band_counts(projections: &[ClaimProjection]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::from([
        ("likely_current", 0),
        ("use_with_caution", 0),
        ("needs_verification", 0),
    ]);
    for projection in projections {
        let key = trust_band_label(projection.trust_band);
        counts.entry(key).and_modify(|value| *value += 1);
    }
    counts
}

fn compare_claim_projection(left: &ClaimProjection, right: &ClaimProjection) -> Ordering {
    placement_rank(left.placement)
        .cmp(&placement_rank(right.placement))
        .then_with(|| trust_rank(right.trust_band).cmp(&trust_rank(left.trust_band)))
        .then_with(|| {
            right
                .parsed_source_asof
                .cmp(&left.parsed_source_asof)
                .then_with(|| {
                    if left.parsed_source_asof.is_none() == right.parsed_source_asof.is_none() {
                        Ordering::Equal
                    } else if left.parsed_source_asof.is_none() {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                })
        })
        .then_with(|| left.claim_type.as_str().cmp(right.claim_type.as_str()))
        .then_with(|| left.claim.id.cmp(&right.claim.id))
}

fn placement_rank(placement: ClaimPlacement) -> u8 {
    match placement {
        ClaimPlacement::Risk => 0,
        ClaimPlacement::Momentum => 1,
        ClaimPlacement::Commitment => 2,
        ClaimPlacement::Relationship => 3,
        ClaimPlacement::Context => 4,
        ClaimPlacement::Ignored => 5,
    }
}

fn trust_rank(band: TrustBand) -> u8 {
    match visible_trust_band(band) {
        TrustBand::LikelyCurrent => 3,
        TrustBand::UseWithCaution => 2,
        TrustBand::NeedsVerification | TrustBand::Unscored => 1,
    }
}

fn source_for_claim(
    ctx: &AbilityContext<'_>,
    project_id: &str,
    claim: &IntelligenceClaim,
) -> Result<SourceAttribution, AbilityError> {
    let now = ctx.services().clock.now();
    let observed_at = parse_observed_at(claim, now);
    let source_asof = parse_claim_source_asof(claim, now);
    SourceAttribution::new(
        data_source_for_claim(&claim.data_source),
        vec![SourceIdentifier::Entity {
            entity_id: EntityId::new(project_id.to_string()),
            field: Some(
                claim
                    .field_path
                    .clone()
                    .unwrap_or_else(|| claim.claim_type.clone()),
            ),
        }],
        observed_at,
        source_asof,
        1.0,
        None,
    )
    .map_err(|error| validation_error(format!("invalid source attribution: {error}")))
}

fn parse_observed_at(claim: &IntelligenceClaim, now: DateTime<Utc>) -> DateTime<Utc> {
    for candidate in [claim.observed_at.as_str(), claim.created_at.as_str()] {
        match parse_source_timestamp(Some(candidate), now, None) {
            SourceTimestampStatus::Accepted(parsed)
            | SourceTimestampStatus::Implausible { parsed, .. } => return parsed,
            SourceTimestampStatus::Malformed(_) | SourceTimestampStatus::Missing => {}
        }
    }
    now
}

fn parse_claim_source_asof(claim: &IntelligenceClaim, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match parse_source_timestamp(claim.source_asof.as_deref(), now, None) {
        SourceTimestampStatus::Accepted(parsed)
        | SourceTimestampStatus::Implausible { parsed, .. } => Some(parsed),
        SourceTimestampStatus::Malformed(_) | SourceTimestampStatus::Missing => None,
    }
}

fn data_source_for_claim(value: &str) -> DataSource {
    match value.trim().to_ascii_lowercase().as_str() {
        "user" | "human" | "manual" => DataSource::User,
        "google" => DataSource::Google,
        "glean" => DataSource::Glean {
            downstream: GleanDownstream::Documents,
        },
        "ai" | "agent" => DataSource::Ai,
        "local_enrichment" => DataSource::LocalEnrichment,
        "legacy_unattributed" => DataSource::LegacyUnattributed,
        other => DataSource::Other(SourceName::new(other)),
    }
}

fn block_id(input: &NormalizedInput, section: &str, kind: &str, source: &str) -> String {
    let raw = format!(
        "{}:{section}:{kind}:{source}",
        input.composition_id.as_str()
    );
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn salience(weight: f32, band: SalienceBand, reason: &str) -> Salience {
    Salience {
        weight,
        band,
        reason: reason.to_string(),
    }
}

fn provenance_config(ctx: &AbilityContext<'_>) -> ProvenanceBuilderConfig {
    let mut config = ProvenanceBuilderConfig::new(ABILITY_NAME, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(ABILITY_SCHEMA_VERSION);
    config.actor = provenance_actor(ctx.actor.clone());
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Read;
    config.inputs_snapshot = InputsSnapshot::default();
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
        Actor::SurfaceClient { instance, .. } => crate::abilities::provenance::Actor::External {
            source: format!("surface_client:{}", instance.as_str()),
        },
        Actor::McpClient { client_id, .. } => crate::abilities::provenance::Actor::External {
            source: format!("mcp_client:{}", client_id.as_str()),
        },
    }
}

fn validate_block_provenance(
    composition: &Composition,
    provenance: &crate::abilities::provenance::Provenance,
) -> Result<(), AbilityError> {
    for block in composition.blocks() {
        block.validate_against(provenance).map_err(block_error)?;
        for binding in &block.field_bindings {
            if matches!(
                binding.role,
                BindingRole::Source | BindingRole::FeedbackTarget
            ) {
                for index in &binding.claim_refs {
                    let Some(claim_ref) = block.claim_refs.get(index.0) else {
                        return Err(validation_error(
                            "field binding claim_ref index out of range",
                        ));
                    };
                    if claim_ref.field_path.is_none() {
                        return Err(validation_error(
                            "Source and FeedbackTarget bindings require field-aware ClaimRef",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn composition_commit_error(error: CompositionCommitError) -> AbilityError {
    let message = error.to_string();
    match error {
        CompositionCommitError::StaleVersion {
            composition_id,
            expected,
            current,
        }
        | CompositionCommitError::InflatedVersion {
            composition_id,
            expected,
            current,
        } => AbilityError {
            kind: AbilityErrorKind::StaleComposition {
                composition_id,
                expected,
                current,
            },
            message,
        },
        CompositionCommitError::Overflow { composition_id } => AbilityError {
            kind: AbilityErrorKind::CompositionVersionOverflow { composition_id },
            message,
        },
        CompositionCommitError::EmptyCompositionId
        | CompositionCommitError::Transaction(_)
        | CompositionCommitError::Mode(_)
        | CompositionCommitError::Unavailable(_) => hard_error("composition_commit", error),
    }
}

fn validation_error(message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: message.into(),
    }
}

fn hard_error(code: impl Into<String>, message: impl std::fmt::Display) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::HardError(code.into()),
        message: message.to_string(),
    }
}

fn field_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("field path construction failed: {error}"))
}

fn block_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("composition block construction failed: {error}"))
}

fn provenance_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("provenance construction failed: {error}"))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use chrono::TimeZone;

    use super::*;
    use crate::abilities::registry::{AbilityRegistry, ActorKind, McpExposure, ScopeSet};
    use crate::abilities::{
        project_composition_for_surface, FallbackProjectionContext, ProjectionError, SurfaceKind,
        NOOP_ABILITY_TRACER,
    };
    use crate::intelligence::provider::{
        Completion, FingerprintMetadata, IntelligenceProvider, ModelName, ModelTier, PromptInput,
        ProviderError, ProviderKind,
    };
    use crate::sensitivity::{ClaimDismissalSurface, ClaimVerificationState};
    use crate::services::context::{
        CompositionCommitFuture, CompositionCommitHandle, CompositionCommitRequest,
        EntityContextClaimReadFuture, EntityContextClaimReadHandle, ExternalClients, FixedClock,
        ProjectCompositionSnapshotReadFuture, ProjectCompositionSnapshotReadHandle,
        ProjectCompositionSnapshotSensitivity, SeedableRng, ServiceContext,
    };
    use crate::types::{ClaimSensitivity, TemporalScope};

    #[derive(Default)]
    struct SpyClaimReader {
        claims: Mutex<Vec<IntelligenceClaim>>,
        calls: AtomicUsize,
        last_surface: Mutex<Option<ClaimDismissalSurface>>,
    }

    impl SpyClaimReader {
        fn new(claims: Vec<IntelligenceClaim>) -> Self {
            Self {
                claims: Mutex::new(claims),
                calls: AtomicUsize::new(0),
                last_surface: Mutex::new(None),
            }
        }
    }

    impl EntityContextClaimReadHandle for SpyClaimReader {
        fn read_entity_context_claims<'a>(
            &'a self,
            entity_type: String,
            entity_id: String,
            surface: ClaimDismissalSurface,
            _depth: usize,
        ) -> EntityContextClaimReadFuture<'a> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                *self.last_surface.lock().expect("claim surface lock") = Some(surface);
                assert_eq!(entity_type, "project");
                assert_eq!(entity_id, "project-fixture-1");
                Ok(self.claims.lock().expect("claim lock").clone())
            })
        }
    }

    #[derive(Default)]
    struct RecordingCommitter {
        calls: AtomicUsize,
        expected_versions: Mutex<Vec<u64>>,
    }

    impl CompositionCommitHandle for RecordingCommitter {
        fn commit_composition<'a>(
            &'a self,
            request: CompositionCommitRequest,
        ) -> CompositionCommitFuture<'a> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.expected_versions
                    .lock()
                    .expect("version lock")
                    .push(request.proposal.expected_composition_version);
                let composition = Composition {
                    id: request.proposal.composition_id.clone(),
                    metadata: CompositionMetadata {
                        composition_version: CompositionVersion::new(1),
                        ..request.proposal.composition.metadata
                    },
                    ..request.proposal.composition
                };
                Ok(crate::services::context::CommittedComposition {
                    composition_id: request.proposal.composition_id,
                    composition_version: 1,
                    composition,
                })
            })
        }
    }

    struct SpySnapshotReader {
        result: Mutex<Result<ProjectCompositionSnapshot, ProjectCompositionSnapshotReadError>>,
        calls: AtomicUsize,
        last_surface: Mutex<Option<ClaimDismissalSurface>>,
    }

    impl SpySnapshotReader {
        fn new(
            result: Result<ProjectCompositionSnapshot, ProjectCompositionSnapshotReadError>,
        ) -> Self {
            Self {
                result: Mutex::new(result),
                calls: AtomicUsize::new(0),
                last_surface: Mutex::new(None),
            }
        }
    }

    impl ProjectCompositionSnapshotReadHandle for SpySnapshotReader {
        fn read_project_composition_snapshot<'a>(
            &'a self,
            project_id: String,
            surface: ClaimDismissalSurface,
        ) -> ProjectCompositionSnapshotReadFuture<'a> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                *self.last_surface.lock().expect("snapshot surface lock") = Some(surface);
                assert_eq!(project_id, "project-fixture-1");
                self.result.lock().expect("snapshot result lock").clone()
            })
        }
    }

    struct StaticProvider;

    #[async_trait]
    impl IntelligenceProvider for StaticProvider {
        async fn complete(
            &self,
            _prompt: PromptInput,
            _tier: ModelTier,
        ) -> Result<Completion, ProviderError> {
            Ok(Completion {
                text: String::new(),
                fingerprint_metadata: FingerprintMetadata {
                    provider: ProviderKind::Other("test"),
                    model: ModelName::new("unused"),
                    temperature: 0.0,
                    top_p: None,
                    seed: None,
                    tokens_input: None,
                    tokens_output: None,
                    provider_completion_id: None,
                },
            })
        }

        fn provider_kind(&self) -> ProviderKind {
            ProviderKind::Other("test")
        }

        fn current_model(&self, _tier: ModelTier) -> ModelName {
            ModelName::new("unused")
        }
    }

    fn fixture_parts(
        claims: Vec<IntelligenceClaim>,
    ) -> (
        FixedClock,
        SeedableRng,
        ExternalClients,
        Arc<SpyClaimReader>,
        Arc<RecordingCommitter>,
        StaticProvider,
    ) {
        (
            FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap()),
            SeedableRng::new(42),
            ExternalClients::default(),
            Arc::new(SpyClaimReader::new(claims)),
            Arc::new(RecordingCommitter::default()),
            StaticProvider,
        )
    }

    fn services<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        external: &'a ExternalClients,
        reader: Arc<SpyClaimReader>,
        committer: Arc<RecordingCommitter>,
        snapshot_reader: Arc<SpySnapshotReader>,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, external)
            .with_actor("surface_client")
            .with_ability_id(ABILITY_NAME)
            .with_entity_context_claim_reader(reader)
            .with_project_composition_snapshot_reader(snapshot_reader)
            .with_composition_commit_handle(committer)
    }

    fn ability_ctx<'a>(
        services: &'a ServiceContext<'a>,
        provider: &'a StaticProvider,
    ) -> AbilityContext<'a> {
        crate::abilities::registry::install_full_producer_test_allowlist();
        AbilityContext::new(
            services,
            provider,
            &NOOP_ABILITY_TRACER,
            Actor::SurfaceClient {
                instance: crate::abilities::registry::SurfaceClientId::new("sc_project_fixture"),
                scopes: ScopeSet::new([crate::abilities::registry::SurfaceScope::new(
                    "read.project_overview",
                )])
                .expect("scope set"),
            },
            None,
            ClaimDismissalSurface::LogStructured,
        )
    }

    fn input() -> ProjectOverviewInput {
        ProjectOverviewInput {
            schema_version: ABILITY_SCHEMA_VERSION,
            project_id: "project-fixture-1".to_string(),
            entity_type: None,
            entity_id: None,
            expected_composition_version: 0,
            composition_id: Some("project-overview-fixture".to_string()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn claim(
        id: &str,
        subject_ref: Value,
        claim_type: &str,
        field_path: &str,
        text: &str,
        trust_score: Option<f64>,
        source_asof: Option<&str>,
        sensitivity: ClaimSensitivity,
    ) -> IntelligenceClaim {
        IntelligenceClaim {
            id: id.to_string(),
            claim_version: 2,
            subject_ref: subject_ref.to_string(),
            claim_type: claim_type.to_string(),
            field_path: Some(field_path.to_string()),
            topic_key: None,
            text: text.to_string(),
            dedup_key: format!("dedup-{id}"),
            item_hash: None,
            actor: "agent:test".to_string(),
            data_source: "google".to_string(),
            source_ref: Some(format!("source-{id}")),
            source_asof: source_asof.map(ToString::to_string),
            observed_at: "2026-05-15T10:00:00Z".to_string(),
            created_at: "2026-05-15T10:00:00Z".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            claim_state: ClaimState::Active,
            surfacing_state: SurfacingState::Active,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason: None,
            expires_at: None,
            superseded_by: None,
            trust_score,
            trust_computed_at: None,
            trust_version: Some(1),
            thread_id: None,
            temporal_scope: TemporalScope::State,
            sensitivity,
            verification_state: ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        }
    }

    fn project_claim(
        id: &str,
        claim_type: &str,
        field_path: &str,
        text: &str,
        trust_score: Option<f64>,
        source_asof: Option<&str>,
        sensitivity: ClaimSensitivity,
    ) -> IntelligenceClaim {
        claim(
            id,
            json!({"kind": "project", "id": "project-fixture-1"}),
            claim_type,
            field_path,
            text,
            trust_score,
            source_asof,
            sensitivity,
        )
    }

    fn snapshot_field(
        field_path: &str,
        label: &str,
        value: Value,
        sensitivity: ProjectCompositionSnapshotSensitivity,
        source_label: Option<&str>,
        source_asof: Option<&str>,
    ) -> ProjectCompositionSnapshotField {
        ProjectCompositionSnapshotField {
            field_path: field_path.to_string(),
            label: label.to_string(),
            value,
            sensitivity,
            source_label: source_label.map(ToString::to_string),
            source_ref: source_label.map(|source| format!("{source}:fixture")),
            source_asof: source_asof.map(ToString::to_string),
            trust_band: TrustBand::UseWithCaution,
            trust_status: "use_with_caution".to_string(),
            provenance_kind: ProjectCompositionProvenanceKind::SourceField,
        }
    }

    fn identity_snapshot_field(
        field_path: &str,
        label: &str,
        value: &str,
    ) -> ProjectCompositionSnapshotField {
        ProjectCompositionSnapshotField {
            field_path: field_path.to_string(),
            label: label.to_string(),
            value: Value::String(value.to_string()),
            sensitivity: ProjectCompositionSnapshotSensitivity::NonSensitiveIdentity,
            source_label: None,
            source_ref: None,
            source_asof: None,
            trust_band: TrustBand::LikelyCurrent,
            trust_status: "likely_current".to_string(),
            provenance_kind: ProjectCompositionProvenanceKind::NonSensitiveIdentity,
        }
    }

    fn snapshot_fixture(
        is_parent: bool,
        fields: Vec<ProjectCompositionSnapshotField>,
    ) -> ProjectCompositionSnapshot {
        ProjectCompositionSnapshot {
            project_id: "project-fixture-1".to_string(),
            display_name: identity_snapshot_field(
                "/identity/display_name",
                "Project",
                "Example Project",
            ),
            status: Some(identity_snapshot_field(
                "/identity/status",
                "Status",
                "active",
            )),
            is_parent,
            fields,
        }
    }

    fn output_json(output: &crate::abilities::provenance::AbilityOutput<Composition>) -> Value {
        serde_json::to_value(output).expect("output serializes")
    }

    #[test]
    fn entity_envelope_accepts_matching_project_and_rejects_mismatch() {
        let ok = ProjectOverviewInput {
            entity_type: Some("project".to_string()),
            entity_id: Some("project-fixture-1".to_string()),
            ..input()
        };
        normalize_input(ok).expect("matching envelope is accepted");

        let wrong_type = ProjectOverviewInput {
            entity_type: Some("person".to_string()),
            entity_id: Some("project-fixture-1".to_string()),
            ..input()
        };
        assert_eq!(
            normalize_input(wrong_type)
                .expect_err("type mismatch rejects")
                .kind,
            AbilityErrorKind::Validation
        );

        let wrong_id = ProjectOverviewInput {
            entity_type: Some("project".to_string()),
            entity_id: Some("other-project".to_string()),
            ..input()
        };
        assert_eq!(
            normalize_input(wrong_id)
                .expect_err("id mismatch rejects")
                .kind,
            AbilityErrorKind::Validation
        );
    }

    #[test]
    fn registry_declaration_pins_project_policy_and_invalidation_signals() {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let descriptor = registry
            .iter_all()
            .find(|descriptor| descriptor.name == ABILITY_NAME)
            .expect("project overview ability registered");

        assert_eq!(descriptor.name, ABILITY_NAME);
        assert_eq!(descriptor.category, AbilityCategory::Read);
        assert_eq!(
            descriptor.policy.allowed_actors,
            &[ActorKind::User, ActorKind::SurfaceClient]
        );
        assert_eq!(
            descriptor.policy.required_scopes,
            &["read.project_overview"]
        );
        assert_eq!(descriptor.policy.mcp_exposure, McpExposure::Invocable);
        assert!(!descriptor.policy.client_side_executable);
        assert!(descriptor.mutates.is_empty());
        assert!(descriptor.composes.is_empty());
        assert!(descriptor.signal_policy.coalesce);
        for signal in [
            "claim.version",
            "project_subject.claim_changed",
            "claim.lifecycle",
            "claim.dismissal",
            "source.freshness",
            "source.revocation",
            "project.field_changed",
        ] {
            assert!(
                descriptor
                    .signal_policy
                    .emits_on_output_change
                    .contains(&signal),
                "project overview declares invalidation signal {signal}"
            );
        }
    }

    #[tokio::test]
    async fn missing_project_rejects_before_composition_commit() {
        let snapshot_reader = Arc::new(SpySnapshotReader::new(Err(
            ProjectCompositionSnapshotReadError::ProjectNotFound("project-fixture-1".to_string()),
        )));
        let (clock, rng, external, reader, committer, provider) = fixture_parts(Vec::new());
        let services = services(
            &clock,
            &rng,
            &external,
            reader.clone(),
            committer.clone(),
            snapshot_reader.clone(),
        );
        let ctx = ability_ctx(&services, &provider);

        let err = match project_overview(&ctx, input()).await {
            Ok(_) => panic!("missing project rejects"),
            Err(error) => error,
        };

        assert_eq!(err.kind, AbilityErrorKind::Validation);
        assert!(err.message.contains("project-fixture-1"));
        assert_eq!(reader.calls.load(Ordering::SeqCst), 1);
        assert_eq!(snapshot_reader.calls.load(Ordering::SeqCst), 1);
        assert_eq!(committer.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn committed_output_filters_project_claims_and_wraps_snapshot_fields() {
        let mut dismissed = project_claim(
            "claim-dismissed",
            "entity_risk",
            "/risk/hidden",
            "Dismissed risk should not render",
            Some(0.99),
            Some("2026-05-15T09:00:00Z"),
            ClaimSensitivity::Internal,
        );
        dismissed.demotion_reason = Some("dismissed".to_string());
        let claims = vec![
            project_claim(
                "claim-risk",
                "entity_risk",
                "/risk/current",
                "Implementation risk is rising",
                Some(0.97),
                Some("2026-05-15T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            project_claim(
                "claim-win",
                "entity_win",
                "/wins/latest",
                "Launch path is clearer",
                Some(0.92),
                Some("2026-05-14T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            project_claim(
                "claim-commitment",
                "commitment",
                "/commitments/next",
                "Review the rollout plan",
                Some(0.86),
                Some("2026-04-01T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            claim(
                "claim-account-scope",
                json!({"kind": "account", "id": "acct-fixture"}),
                "entity_win",
                "/wins/account",
                "Account-scoped claim should not render",
                Some(0.92),
                Some("2026-05-14T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            project_claim(
                "claim-confidential",
                "company_context",
                "/context/confidential",
                "Confidential project detail should not render",
                Some(0.9),
                Some("2026-05-14T09:00:00Z"),
                ClaimSensitivity::Confidential,
            ),
            dismissed,
        ];
        let snapshot_reader = Arc::new(SpySnapshotReader::new(Ok(snapshot_fixture(
            true,
            vec![
                snapshot_field(
                    "/vitals/status",
                    "Status",
                    Value::String("active".to_string()),
                    ProjectCompositionSnapshotSensitivity::Internal,
                    Some("project"),
                    Some("2026-05-14T09:00:00Z"),
                ),
                snapshot_field(
                    "/the-horizon/signals",
                    "Project signals",
                    json!({"open_action_count": 3, "trend": "improving"}),
                    ProjectCompositionSnapshotSensitivity::Internal,
                    Some("project_signals"),
                    Some("2026-05-14T09:00:00Z"),
                ),
                snapshot_field(
                    "/portfolio/children",
                    "Sub-projects",
                    json!([{"id": "child-1", "name": "Child Project"}]),
                    ProjectCompositionSnapshotSensitivity::Internal,
                    Some("project"),
                    Some("2026-05-14T09:00:00Z"),
                ),
                snapshot_field(
                    "/the-record/private",
                    "Private field",
                    Value::String("private snapshot value".to_string()),
                    ProjectCompositionSnapshotSensitivity::Confidential,
                    Some("project"),
                    Some("2026-05-14T09:00:00Z"),
                ),
            ],
        ))));
        let (clock, rng, external, reader, committer, provider) = fixture_parts(claims);
        let services = services(
            &clock,
            &rng,
            &external,
            reader.clone(),
            committer.clone(),
            snapshot_reader.clone(),
        );
        let ctx = ability_ctx(&services, &provider);

        let output = project_overview(&ctx, input())
            .await
            .expect("project overview succeeds");
        let composition = output.data();
        let serialized = output_json(&output);
        let serialized_text = serialized.to_string();

        assert_eq!(reader.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            *reader.last_surface.lock().expect("claim surface lock"),
            Some(ClaimDismissalSurface::LogStructured)
        );
        assert_eq!(snapshot_reader.calls.load(Ordering::SeqCst), 1);
        assert_eq!(committer.calls.load(Ordering::SeqCst), 1);
        assert_eq!(composition.metadata.composition_version.0, 1);
        assert_eq!(composition.generated_by.as_str(), ABILITY_NAME);
        assert_eq!(composition.metadata.generated_by, ABILITY_NAME);
        assert_eq!(
            composition.subject.as_ref().map(|subject| subject.as_str()),
            Some("project:project-fixture-1")
        );

        let section_ids = composition
            .sections
            .iter()
            .map(|section| section.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(section_ids[0], "headline");
        assert!(section_ids.contains(&"portfolio"));
        assert!(section_ids.contains(&"the-horizon"));
        assert!(section_ids.contains(&"the-work"));

        assert!(serialized_text.contains("claim-risk"));
        assert!(serialized_text.contains("claim-win"));
        assert!(serialized_text.contains("claim-commitment"));
        assert!(!serialized_text.contains("claim-account-scope"));
        assert!(!serialized_text.contains("claim-confidential"));
        assert!(!serialized_text.contains("claim-dismissed"));
        assert!(!serialized_text.contains("private snapshot value"));
        assert!(serialized_text.contains("Project signals"));
        assert!(serialized_text.contains("needs_verification"));

        let blocks = composition.blocks().collect::<Vec<_>>();
        assert!(blocks
            .iter()
            .any(|block| block.block_type == BlockType::RiskCallout));
        assert!(blocks
            .iter()
            .any(|block| block.block_type == BlockType::ActionList));
        assert!(blocks
            .iter()
            .any(|block| block.block_type == BlockType::EvidenceList));
        for block in blocks {
            block
                .validate_against(output.provenance())
                .expect("block provenance resolves");
            assert!(
                !block.field_bindings.is_empty(),
                "block {} has field bindings",
                block.id.as_str()
            );
            for binding in &block.field_bindings {
                if matches!(
                    binding.role,
                    BindingRole::Source | BindingRole::FeedbackTarget
                ) {
                    for index in &binding.claim_refs {
                        assert!(block.claim_refs[index.0].field_path.is_some());
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn dos852_producer_output_passes_w4d_projection() {
        let claims = vec![
            project_claim(
                "claim-risk",
                "entity_risk",
                "/risk/current",
                "Implementation risk is rising",
                Some(0.97),
                Some("2026-05-15T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            project_claim(
                "claim-win",
                "entity_win",
                "/wins/latest",
                "Launch path is clearer",
                Some(0.92),
                Some("2026-05-14T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            project_claim(
                "claim-commitment",
                "commitment",
                "/commitments/next",
                "Review the rollout plan",
                Some(0.86),
                Some("2026-04-01T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
        ];
        let snapshot_reader = Arc::new(SpySnapshotReader::new(Ok(snapshot_fixture(
            true,
            vec![
                snapshot_field(
                    "/vitals/status",
                    "Status",
                    Value::String("active".to_string()),
                    ProjectCompositionSnapshotSensitivity::Internal,
                    Some("project"),
                    Some("2026-05-14T09:00:00Z"),
                ),
                snapshot_field(
                    "/the-horizon/signals",
                    "Project signals",
                    json!({"open_action_count": 3, "trend": "improving"}),
                    ProjectCompositionSnapshotSensitivity::Internal,
                    Some("project_signals"),
                    Some("2026-05-14T09:00:00Z"),
                ),
                snapshot_field(
                    "/portfolio/children",
                    "Sub-projects",
                    json!([{"id": "child-1", "name": "Child Project"}]),
                    ProjectCompositionSnapshotSensitivity::Internal,
                    Some("project"),
                    Some("2026-05-14T09:00:00Z"),
                ),
            ],
        ))));
        let (clock, rng, external, reader, committer, provider) = fixture_parts(claims);
        let services = services(
            &clock,
            &rng,
            &external,
            reader,
            committer,
            snapshot_reader,
        );
        let ctx = ability_ctx(&services, &provider);

        let output = project_overview(&ctx, input())
            .await
            .expect("project overview succeeds");
        let composition = output.data();
        let emitted_block_count = composition.blocks().count();

        let proj_ctx = FallbackProjectionContext::new(
            Actor::SurfaceClient {
                instance: crate::abilities::registry::SurfaceClientId::new("sc_project_fixture"),
                scopes: ScopeSet::new([crate::abilities::registry::SurfaceScope::new(
                    "read.project_overview",
                )])
                .expect("scope set"),
            },
            SurfaceKind::SurfaceClient,
            3,
        );

        let projection = project_composition_for_surface(composition, &proj_ctx);
        if let Err(ProjectionError::InvalidProducerOutput { reason }) = &projection {
            panic!("projection rejected producer output with InvalidProducerOutput: {reason:?}");
        }
        let (projected, _audits) =
            projection.expect("projection must accept producer output (DOS-852 contract)");

        assert_eq!(
            projected.blocks.len(),
            emitted_block_count,
            "every emitted block must project"
        );
        for block in composition.blocks() {
            assert!(
                projected
                    .blocks
                    .iter()
                    .any(|projected| projected.block_id.as_str() == block.id.as_str()),
                "emitted block {} must project",
                block.id.as_str()
            );
        }
    }

    #[tokio::test]
    async fn empty_claim_set_returns_project_sections_without_frontend_fallback() {
        let snapshot_reader = Arc::new(SpySnapshotReader::new(Ok(snapshot_fixture(
            false,
            Vec::new(),
        ))));
        let (clock, rng, external, reader, committer, provider) = fixture_parts(Vec::new());
        let services = services(&clock, &rng, &external, reader, committer, snapshot_reader);
        let ctx = ability_ctx(&services, &provider);

        let output = project_overview(&ctx, input())
            .await
            .expect("empty project overview succeeds");
        let composition = output.data();
        let section_ids = composition
            .sections
            .iter()
            .map(|section| section.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            section_ids,
            vec![
                "headline",
                "trajectory",
                "the-horizon",
                "the-landscape",
                "the-room",
                "whats-next",
                "the-record",
                "the-work",
            ]
        );
        assert!(!section_ids.contains(&"portfolio"));
        let empty_blocks = composition
            .sections
            .iter()
            .flat_map(|section| section.blocks.iter())
            .filter(|block| {
                block
                    .attributes
                    .pointer("/empty_state")
                    .and_then(Value::as_bool)
                    == Some(true)
            })
            .collect::<Vec<_>>();
        assert!(
            empty_blocks.len() >= 7,
            "non-headline project sections render producer-authored empty blocks"
        );
        for block in empty_blocks {
            assert!(block.claim_refs.is_empty());
            assert!(block.field_bindings.iter().all(|binding| {
                binding.role == BindingRole::DisplayOnly && binding.claim_refs.is_empty()
            }));
            block
                .validate_against(output.provenance())
                .expect("empty block provenance resolves");
        }
    }

    #[tokio::test]
    async fn snapshot_read_failure_degrades_in_headline_without_blocking_claim_render() {
        let snapshot_reader = Arc::new(SpySnapshotReader::new(Err(
            ProjectCompositionSnapshotReadError::ReadFailed("fixture failure".to_string()),
        )));
        let claims = vec![project_claim(
            "claim-win",
            "entity_win",
            "/wins/latest",
            "Launch path is clearer",
            Some(0.92),
            Some("2026-05-14T09:00:00Z"),
            ClaimSensitivity::Internal,
        )];
        let (clock, rng, external, reader, committer, provider) = fixture_parts(claims);
        let services = services(
            &clock,
            &rng,
            &external,
            reader,
            committer.clone(),
            snapshot_reader,
        );
        let ctx = ability_ctx(&services, &provider);

        let output = project_overview(&ctx, input())
            .await
            .expect("snapshot degradation still renders claims");
        let headline = &output.data().sections[0].blocks[0];

        assert_eq!(committer.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            headline
                .attributes
                .pointer("/snapshot_degraded")
                .and_then(Value::as_str),
            Some("project_snapshot_unavailable")
        );
        assert!(output_json(&output).to_string().contains("claim-win"));
    }
}

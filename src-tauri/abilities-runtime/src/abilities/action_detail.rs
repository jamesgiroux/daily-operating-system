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
    ActionCompositionProvenanceKind, ActionCompositionSnapshot, ActionCompositionSnapshotField,
    ActionCompositionSnapshotReadError, CompositionCommitError, CompositionProposal,
};
use crate::types::{
    prompt_input_sensitivity_allowed, subject_ref_from_json, ClaimState, ClaimSubjectRef,
    IntelligenceClaim, SurfacingState,
};

const ABILITY_NAME: &str = "dailyos/action-detail";
const ABILITY_SCHEMA_VERSION: u32 = 1;
const ACTION_CLAIM_DEPTH: usize = 1;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ActionDetailInput {
    pub schema_version: u32,
    pub action_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_ref: Option<Value>,
    #[serde(default)]
    pub expected_composition_version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition_id: Option<String>,
}

#[derive(Debug, Clone)]
struct NormalizedInput {
    action_id: String,
    expected_composition_version: u64,
    composition_id: CompositionDocId,
}

struct PreparedActionDetail {
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
    snapshot: Option<ActionCompositionSnapshot>,
    degraded_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaimPlacement {
    Status,
    Context,
    Reference,
    Work,
    Risk,
    Ignored,
}

#[ability(
    name = "dailyos/action-detail",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, SurfaceClient],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.action_detail"],
    mcp_exposure = Invocable,
    client_side_executable = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [
        "claim.version",
        "action_subject.claim_changed",
        "claim.lifecycle",
        "claim.dismissal",
        "source.freshness",
        "source.revocation",
        "action.field_changed",
        "action_completed",
        "action_reopened",
        "action_pushed_to_linear"
    ], coalesce = true }
)]
pub async fn action_detail(
    ctx: &AbilityContext<'_>,
    input: ActionDetailInput,
) -> AbilityResult<Composition> {
    let input = normalize_input(input)?;
    let prepared = prepare_action_detail(ctx, &input).await?;
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

fn normalize_input(input: ActionDetailInput) -> Result<NormalizedInput, AbilityError> {
    if input.schema_version != ABILITY_SCHEMA_VERSION {
        return Err(validation_error(format!(
            "unsupported schema_version `{}` for `{ABILITY_NAME}`",
            input.schema_version
        )));
    }
    let action_id = input.action_id.trim();
    if action_id.is_empty() {
        return Err(validation_error("action_id must be non-empty"));
    }
    validate_subject_ref(action_id, input.subject_ref.as_ref())?;
    let composition_id = input
        .composition_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("dailyos/action-detail:action:{action_id}"));
    validate_composition_id(action_id, &composition_id)?;

    Ok(NormalizedInput {
        action_id: action_id.to_string(),
        expected_composition_version: input.expected_composition_version,
        composition_id: CompositionDocId::new(composition_id),
    })
}

fn validate_subject_ref(action_id: &str, subject_ref: Option<&Value>) -> Result<(), AbilityError> {
    let Some(subject_ref) = subject_ref else {
        return Ok(());
    };
    let Some(action_ref) = subject_ref.get("action").and_then(Value::as_str) else {
        return Err(validation_error("subject_ref must contain action"));
    };
    if action_ref.trim() != action_id {
        return Err(validation_error("subject_ref action must match action_id"));
    }
    Ok(())
}

fn validate_composition_id(action_id: &str, composition_id: &str) -> Result<(), AbilityError> {
    let parts = composition_id.split(':').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != ABILITY_NAME || parts[1] != "action" || parts[2] != action_id
    {
        return Err(validation_error(format!(
            "composition_id must be `{ABILITY_NAME}:action:{action_id}`"
        )));
    }
    if composition_id.chars().any(char::is_control) {
        return Err(validation_error(
            "composition_id must not contain control characters",
        ));
    }
    Ok(())
}

async fn prepare_action_detail(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
) -> Result<PreparedActionDetail, AbilityError> {
    let snapshot = read_action_snapshot(ctx, input).await?;
    let claims = ctx
        .services()
        .read_entity_context_claims(
            "action".to_string(),
            input.action_id.clone(),
            ctx.entity_context_claim_surface(),
            ACTION_CLAIM_DEPTH,
        )
        .await
        .map_err(|error| hard_error("action_detail_claim_read", error))?;

    let subject_ref = SubjectRef::Action(input.action_id.clone());
    let subject = SubjectAttribution::direct_confident(subject_ref);
    let provenance_config = provenance_config(ctx);
    let invocation_id = provenance_config.invocation_id;
    let mut provenance_builder = ProvenanceBuilder::new(provenance_config);
    provenance_builder.set_subject(subject.clone());

    let mut projections = Vec::new();
    for claim in claims {
        let Some(projection) = action_claim(ctx, &input.action_id, claim, &mut provenance_builder)?
        else {
            continue;
        };
        projections.push(projection);
    }
    projections.sort_by(compare_claim_projection);

    let composition = build_composition(
        ctx,
        input,
        &projections,
        &snapshot,
        &subject,
        invocation_id,
        &mut provenance_builder,
    )?;

    Ok(PreparedActionDetail {
        proposal: CompositionProposal {
            composition_id: input.composition_id.clone(),
            expected_composition_version: input.expected_composition_version,
            composition,
        },
        provenance_builder,
    })
}

async fn read_action_snapshot(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
) -> Result<SnapshotReadOutcome, AbilityError> {
    match ctx
        .services()
        .read_action_composition_snapshot(
            input.action_id.clone(),
            ctx.entity_context_claim_surface(),
        )
        .await
    {
        Ok(snapshot) => Ok(SnapshotReadOutcome {
            snapshot: Some(snapshot),
            degraded_reason: None,
        }),
        Err(ActionCompositionSnapshotReadError::ActionNotFound(action_id)) => Err(
            validation_error(format!("action `{action_id}` was not found")),
        ),
        Err(ActionCompositionSnapshotReadError::ReadFailed(_)) => Ok(SnapshotReadOutcome {
            snapshot: None,
            degraded_reason: Some("action_snapshot_unavailable".to_string()),
        }),
    }
}

fn action_claim(
    ctx: &AbilityContext<'_>,
    action_id: &str,
    claim: IntelligenceClaim,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Option<ClaimProjection>, AbilityError> {
    if !claim_is_eligible_for_action_detail(&claim, action_id)? {
        return Ok(None);
    }
    let Some(metadata) = metadata_for_name(&claim.claim_type) else {
        return Err(validation_error(format!(
            "unknown claim_type `{}` in action detail input",
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

    let source = source_for_claim(ctx, action_id, &claim)?;
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

fn claim_is_eligible_for_action_detail(
    claim: &IntelligenceClaim,
    action_id: &str,
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
        ClaimSubjectRef::Action { id } => Ok(id == action_id),
        ClaimSubjectRef::Account { .. }
        | ClaimSubjectRef::Person { .. }
        | ClaimSubjectRef::Project { .. }
        | ClaimSubjectRef::Meeting { .. }
        | ClaimSubjectRef::Email { .. }
        | ClaimSubjectRef::Multi(_)
        | ClaimSubjectRef::Global => Ok(false),
    }
}

fn placement_for_claim_type(kind: ClaimType) -> ClaimPlacement {
    match kind {
        ClaimType::Risk | ClaimType::EntityRisk => ClaimPlacement::Risk,
        ClaimType::Commitment | ClaimType::OpenLoop | ClaimType::Recommendation => {
            ClaimPlacement::Work
        }
        ClaimType::EntityCurrentState | ClaimType::EntitySummary => ClaimPlacement::Status,
        ClaimType::UserNote | ClaimType::CompanyContext | ClaimType::AccountFact => {
            ClaimPlacement::Context
        }
        ClaimType::EntityIdentity | ClaimType::ValueDelivered | ClaimType::EntityWin => {
            ClaimPlacement::Reference
        }
        ClaimType::Win
        | ClaimType::StakeholderEngagement
        | ClaimType::StakeholderAssessment
        | ClaimType::StakeholderRole
        | ClaimType::LinkingDismissed
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

#[allow(clippy::too_many_arguments)]
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
    sections.push(section(
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
        salience(0.96, SalienceBand::Critical, "action masthead"),
    ));

    for (section_index, (section_id, label, layout, band, reason)) in [
        (
            "status",
            "Status",
            SectionLayout::Stacked,
            SalienceBand::Important,
            "action status",
        ),
        (
            "priority",
            "Priority",
            SectionLayout::Inline,
            SalienceBand::Important,
            "action priority",
        ),
        (
            "context",
            "Context",
            SectionLayout::Stacked,
            SalienceBand::Important,
            "action context",
        ),
        (
            "reference",
            "Reference",
            SectionLayout::Grid,
            SalienceBand::Contextual,
            "action reference",
        ),
        (
            "linear",
            "Linear",
            SectionLayout::Stacked,
            SalienceBand::Contextual,
            "action linear state",
        ),
        (
            "action-bar",
            "Action bar",
            SectionLayout::Inline,
            SalienceBand::Background,
            "action controls",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let index = section_index + 1;
        let claims = projections
            .iter()
            .filter(|projection| claim_belongs_to_section(projection.placement, section_id))
            .collect::<Vec<_>>();
        let fields = snapshot_fields_for_section(snapshot, section_id);
        let blocks = build_claim_or_snapshot_blocks(
            ctx,
            input,
            section_id,
            index,
            claims,
            fields,
            EmptySectionCopy {
                title: empty_title(section_id),
                body: empty_body(section_id),
                status: "empty",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?;
        sections.push(section(
            section_id,
            label,
            blocks,
            layout,
            salience(0.74, band, reason),
        ));
    }

    let section_count = sections.len();
    let generated_at = ctx.services().clock.now();
    let composition = Composition::new(
        input.composition_id.clone(),
        CompositionKind::Custom {
            type_id: "action_detail".to_string(),
        },
        Some(EntityRef::new(format!("action:{}", input.action_id))),
        sections,
        salience(0.9, SalienceBand::Important, "action detail"),
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

fn claim_belongs_to_section(placement: ClaimPlacement, section_id: &str) -> bool {
    matches!(
        (placement, section_id),
        (ClaimPlacement::Status, "status")
            | (ClaimPlacement::Context, "context")
            | (ClaimPlacement::Reference, "reference")
            | (ClaimPlacement::Work, "action-bar")
            | (ClaimPlacement::Risk, "status")
    )
}

#[derive(Debug, Clone, Copy)]
struct EmptySectionCopy {
    title: &'static str,
    body: &'static str,
    status: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct SectionBlockPosition<'a> {
    section_id: &'a str,
    section_index: usize,
    block_index: usize,
}

impl SectionBlockPosition<'_> {
    fn composition_path(&self) -> String {
        format!(
            "/sections/{}/blocks/{}",
            self.section_index, self.block_index
        )
    }
}

fn section(
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
fn build_claim_or_snapshot_blocks(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    section_id: &str,
    section_index: usize,
    projections: Vec<&ClaimProjection>,
    snapshot_fields: Vec<&ActionCompositionSnapshotField>,
    empty_copy: EmptySectionCopy,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Vec<Block>, AbilityError> {
    let mut blocks = Vec::new();
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
    if blocks.is_empty() {
        blocks.push(build_section_state_block(
            input,
            SectionBlockPosition {
                section_id,
                section_index,
                block_index: 0,
            },
            empty_copy,
            subject,
            invocation_id,
            provenance_builder,
        )?);
    }
    Ok(blocks)
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

    let all_refs = projections
        .iter()
        .map(claim_ref_for_projection)
        .collect::<Result<Vec<_>, _>>()?;
    let trust_band = block_trust_band(projections.iter().map(|projection| projection.trust_band));
    let counts_by_band = trust_band_counts(projections);
    let title = snapshot
        .map(|snapshot| snapshot_value_text(&snapshot.title.value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| input.action_id.clone());
    let status = snapshot
        .and_then(|snapshot| snapshot.status.as_ref())
        .map(|field| snapshot_value_text(&field.value));
    let priority = snapshot
        .and_then(|snapshot| snapshot.priority.as_ref())
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
    let context = projections
        .iter()
        .map(|projection| {
            json!({
                "claim_id": projection.claim.id,
                "text": projection.rendered_text,
                "trust_band": trust_band_label(projection.trust_band),
                "source_asof": projection.claim.source_asof,
            })
        })
        .collect::<Vec<_>>();

    let vitals_len = vitals.len();
    let attributes = json!({
        "entity_type": "action",
        "action_id": input.action_id,
        "account": {
            "id": input.action_id,
            "display_name": title,
            "type": "Action",
        },
        "action": {
            "id": input.action_id,
            "title": title,
            "status": status,
            "priority": priority,
        },
        "title": "Action detail",
        "summary": context
            .first()
            .and_then(|value| value.get("text"))
            .and_then(Value::as_str)
            .unwrap_or("Action detail is grounded in the current action record and any action-local claims."),
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
    block.salience = salience(0.96, SalienceBand::Critical, "summary");

    let mut bindings = vec![
        display_only_binding("/title")?,
        display_only_binding("/account/id")?,
        display_only_binding("/account/display_name")?,
        display_only_binding("/account/type")?,
        display_only_binding("/action/id")?,
        display_only_binding("/action/title")?,
        display_only_binding("/action/status")?,
        display_only_binding("/action/priority")?,
        display_only_binding("/snapshot_degraded")?,
    ];
    bindings.extend(vitals_display_bindings(vitals_len)?);
    if !projections.is_empty() {
        bindings.push(computed_binding("/claim_count", 0..projections.len())?);
        bindings.extend(trust_band_count_computed_bindings(projections.len())?);
        bindings.extend(context_computed_bindings(projections.len())?);
    }
    block.field_bindings = bindings;
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
    let (block_type, attributes, bindings, weight, band, reason) = match projection.placement {
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
            "action risk claim",
        ),
        ClaimPlacement::Work => (
            BlockType::ActionList,
            json!({
                "title": "Related work",
                "items": [{
                    "claim_id": projection.claim.id,
                    "text": projection.rendered_text,
                    "status": "action-local",
                    "trust_band": trust_band,
                    "source_asof": projection.claim.source_asof,
                }],
                "trust_band": trust_band,
            }),
            source_feedback_computed_bindings("/items/0/text", "/items/0/trust_band")?,
            0.72,
            SalienceBand::Important,
            "action work claim",
        ),
        ClaimPlacement::Status | ClaimPlacement::Context | ClaimPlacement::Reference => (
            BlockType::ClaimSummary,
            json!({
                "title": section_title(section_id),
                "claim_id": projection.claim.id,
                "text": projection.rendered_text,
                "claim_type": projection.claim.claim_type,
                "trust_band": trust_band,
                "source_asof": projection.claim.source_asof,
            }),
            source_feedback_computed_bindings("/text", "/trust_band")?,
            0.66,
            SalienceBand::Important,
            "action claim",
        ),
        ClaimPlacement::Ignored => {
            return Err(validation_error("unexpected action detail block placement"));
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
    block.salience = salience(weight, band, reason);
    attribute_block(
        provenance_builder,
        composition_block_path,
        subject,
        vec![projection.source_index],
    )?;
    Ok(block)
}

#[allow(clippy::too_many_arguments)]
fn build_snapshot_fields_block(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    section_id: &str,
    section_index: usize,
    block_index: usize,
    fields: Vec<&ActionCompositionSnapshotField>,
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
    block.salience = salience(0.58, SalienceBand::Contextual, "action snapshot fields");
    attribute_block(
        provenance_builder,
        &composition_block_path,
        subject,
        source_indexes,
    )?;
    Ok(block)
}

fn build_section_state_block(
    input: &NormalizedInput,
    position: SectionBlockPosition<'_>,
    copy: EmptySectionCopy,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let composition_block_path = position.composition_path();
    let mut block = Block::new(
        BlockId::new(block_id(
            input,
            position.section_id,
            "empty_state",
            copy.status,
        )),
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
    block.salience = salience(0.24, SalienceBand::Background, copy.status);
    attribute_block(
        provenance_builder,
        &composition_block_path,
        subject,
        Vec::new(),
    )?;
    Ok(block)
}

fn snapshot_fields_for_section<'a>(
    snapshot: Option<&'a ActionCompositionSnapshot>,
    section_id: &str,
) -> Vec<&'a ActionCompositionSnapshotField> {
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
        "headline" => field_path.starts_with("/headline/"),
        "status" => field_path.starts_with("/status/"),
        "priority" => field_path.starts_with("/priority/"),
        "context" => field_path.starts_with("/context/"),
        "reference" => field_path.starts_with("/reference/"),
        "linear" => field_path.starts_with("/linear/"),
        "action-bar" => field_path.starts_with("/action-bar/"),
        _ => false,
    }
}

fn snapshot_source_indexes(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    fields: &[&ActionCompositionSnapshotField],
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
    field: &ActionCompositionSnapshotField,
) -> Result<Option<SourceAttribution>, AbilityError> {
    if matches!(
        field.provenance_kind,
        ActionCompositionProvenanceKind::NonSensitiveIdentity
            | ActionCompositionProvenanceKind::Unavailable
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
            entity_id: EntityId::new(input.action_id.clone()),
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

fn data_source_for_snapshot_field(field: &ActionCompositionSnapshotField) -> DataSource {
    match field.provenance_kind {
        ActionCompositionProvenanceKind::ManualUser => DataSource::User,
        ActionCompositionProvenanceKind::SystemConfig => DataSource::LocalEnrichment,
        _ => field
            .source_label
            .as_deref()
            .map(data_source_for_claim)
            .unwrap_or_else(|| DataSource::Other(SourceName::new("action_snapshot"))),
    }
}

fn snapshot_field_evidence_items(field: &ActionCompositionSnapshotField) -> Vec<Value> {
    let source_label = field
        .source_label
        .as_deref()
        .unwrap_or(match field.provenance_kind {
            ActionCompositionProvenanceKind::NonSensitiveIdentity => "identity",
            ActionCompositionProvenanceKind::ManualUser => "user",
            ActionCompositionProvenanceKind::SourceField => "source",
            ActionCompositionProvenanceKind::SystemConfig => "system_config",
            ActionCompositionProvenanceKind::Derived => "derived",
            ActionCompositionProvenanceKind::Unavailable => "unavailable",
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
        Value::Object(_) => vec![json!({
            "label": format!("{}: {}", field.label, compact_item_label(&field.value)),
            "source_label": source_label,
            "source_asof": field.source_asof,
        })],
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
    for key in ["title", "name", "label", "identifier", "status", "value"] {
        if let Some(label) = object.get(key).and_then(Value::as_str) {
            if !label.trim().is_empty() {
                return label.to_string();
            }
        }
    }
    snapshot_value_text(value)
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

fn section_title(section_id: &str) -> &'static str {
    match section_id {
        "status" => "Status",
        "priority" => "Priority",
        "context" => "Context",
        "reference" => "Reference",
        "linear" => "Linear",
        "action-bar" => "Action bar",
        _ => "Action detail",
    }
}

fn empty_title(section_id: &str) -> &'static str {
    match section_id {
        "context" => "No context",
        "linear" => "No Linear issue",
        "reference" => "No reference metadata",
        _ => "No action details",
    }
}

fn empty_body(section_id: &str) -> &'static str {
    match section_id {
        "context" => "No source-backed context is currently attached to this action.",
        "linear" => "This action has not been pushed to Linear.",
        "reference" => "No account, source, or due-date reference is currently attached.",
        _ => "No renderable action fields are currently available for this section.",
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
            "dailyos.action_detail.v1",
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

fn context_computed_bindings(projection_count: usize) -> Result<Vec<FieldBinding>, AbilityError> {
    let mut bindings = Vec::with_capacity(projection_count * 4);
    for index in 0..projection_count {
        for field in ["claim_id", "text", "trust_band", "source_asof"] {
            bindings.push(computed_binding_for_indexes(
                &format!("/context/{index}/{field}"),
                vec![index],
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
        ClaimPlacement::Status => 1,
        ClaimPlacement::Work => 2,
        ClaimPlacement::Context => 3,
        ClaimPlacement::Reference => 4,
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
    action_id: &str,
    claim: &IntelligenceClaim,
) -> Result<SourceAttribution, AbilityError> {
    let now = ctx.services().clock.now();
    let observed_at = parse_observed_at(claim, now);
    let source_asof = parse_claim_source_asof(claim, now);
    SourceAttribution::new(
        data_source_for_claim(&claim.data_source),
        vec![SourceIdentifier::Entity {
            entity_id: EntityId::new(action_id.to_string()),
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
    use crate::abilities::NOOP_ABILITY_TRACER;
    use crate::intelligence::provider::{
        Completion, FingerprintMetadata, IntelligenceProvider, ModelName, ModelTier, PromptInput,
        ProviderError, ProviderKind,
    };
    use crate::sensitivity::{ClaimDismissalSurface, ClaimVerificationState};
    use crate::services::context::{
        ActionCompositionSnapshotReadFuture, ActionCompositionSnapshotReadHandle,
        ActionCompositionSnapshotSensitivity, CompositionCommitFuture, CompositionCommitHandle,
        CompositionCommitRequest, EntityContextClaimReadFuture, EntityContextClaimReadHandle,
        ExternalClients, FixedClock, SeedableRng, ServiceContext,
    };
    use crate::types::{ClaimSensitivity, TemporalScope};

    #[derive(Default)]
    struct SpyClaimReader {
        claims: Mutex<Vec<IntelligenceClaim>>,
        calls: AtomicUsize,
    }

    impl SpyClaimReader {
        fn new(claims: Vec<IntelligenceClaim>) -> Self {
            Self {
                claims: Mutex::new(claims),
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl EntityContextClaimReadHandle for SpyClaimReader {
        fn read_entity_context_claims<'a>(
            &'a self,
            entity_type: String,
            entity_id: String,
            _surface: ClaimDismissalSurface,
            _depth: usize,
        ) -> EntityContextClaimReadFuture<'a> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                assert_eq!(entity_type, "action");
                assert_eq!(entity_id, "action-fixture-1");
                Ok(self.claims.lock().expect("claim lock").clone())
            })
        }
    }

    #[derive(Default)]
    struct RecordingCommitter {
        calls: AtomicUsize,
    }

    impl CompositionCommitHandle for RecordingCommitter {
        fn commit_composition<'a>(
            &'a self,
            request: CompositionCommitRequest,
        ) -> CompositionCommitFuture<'a> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
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
        result: Mutex<Result<ActionCompositionSnapshot, ActionCompositionSnapshotReadError>>,
        calls: AtomicUsize,
    }

    impl SpySnapshotReader {
        fn new(
            result: Result<ActionCompositionSnapshot, ActionCompositionSnapshotReadError>,
        ) -> Self {
            Self {
                result: Mutex::new(result),
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl ActionCompositionSnapshotReadHandle for SpySnapshotReader {
        fn read_action_composition_snapshot<'a>(
            &'a self,
            action_id: String,
            _surface: ClaimDismissalSurface,
        ) -> ActionCompositionSnapshotReadFuture<'a> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                assert_eq!(action_id, "action-fixture-1");
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
            .with_action_composition_snapshot_reader(snapshot_reader)
            .with_composition_commit_handle(committer)
    }

    fn ability_ctx<'a>(
        services: &'a ServiceContext<'a>,
        provider: &'a StaticProvider,
    ) -> AbilityContext<'a> {
        AbilityContext::new(
            services,
            provider,
            &NOOP_ABILITY_TRACER,
            Actor::SurfaceClient {
                instance: crate::abilities::registry::SurfaceClientId::new("sc_action_fixture"),
                scopes: ScopeSet::new([crate::abilities::registry::SurfaceScope::new(
                    "read.action_detail",
                )])
                .expect("scope set"),
            },
            None,
            ClaimDismissalSurface::LogStructured,
        )
    }

    fn input() -> ActionDetailInput {
        ActionDetailInput {
            schema_version: ABILITY_SCHEMA_VERSION,
            action_id: "action-fixture-1".to_string(),
            subject_ref: None,
            expected_composition_version: 0,
            composition_id: Some("dailyos/action-detail:action:action-fixture-1".to_string()),
        }
    }

    fn claim(id: &str, subject_ref: Value, text: &str) -> IntelligenceClaim {
        IntelligenceClaim {
            id: id.to_string(),
            claim_version: 2,
            subject_ref: subject_ref.to_string(),
            claim_type: "user_note".to_string(),
            field_path: Some("/text".to_string()),
            topic_key: None,
            text: text.to_string(),
            dedup_key: format!("dedup-{id}"),
            item_hash: None,
            actor: "agent:test".to_string(),
            data_source: "google".to_string(),
            source_ref: Some(format!("source-{id}")),
            source_asof: Some("2026-05-15T10:00:00Z".to_string()),
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
            trust_score: Some(0.92),
            trust_computed_at: None,
            trust_version: Some(1),
            thread_id: None,
            temporal_scope: TemporalScope::State,
            sensitivity: ClaimSensitivity::Internal,
            verification_state: ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        }
    }

    fn action_claim(id: &str, text: &str) -> IntelligenceClaim {
        claim(
            id,
            json!({"kind": "action", "id": "action-fixture-1"}),
            text,
        )
    }

    fn snapshot_field(
        field_path: &str,
        label: &str,
        value: Value,
    ) -> ActionCompositionSnapshotField {
        ActionCompositionSnapshotField {
            field_path: field_path.to_string(),
            label: label.to_string(),
            value,
            sensitivity: ActionCompositionSnapshotSensitivity::Internal,
            source_label: Some("action".to_string()),
            source_ref: Some("action:fixture".to_string()),
            source_asof: Some("2026-05-15T10:00:00Z".to_string()),
            trust_band: TrustBand::UseWithCaution,
            trust_status: "use_with_caution".to_string(),
            provenance_kind: ActionCompositionProvenanceKind::SourceField,
        }
    }

    fn snapshot_fixture(fields: Vec<ActionCompositionSnapshotField>) -> ActionCompositionSnapshot {
        let title = snapshot_field(
            "/headline/title",
            "Title",
            Value::String("Review renewal plan".to_string()),
        );
        let status = snapshot_field(
            "/headline/status",
            "Status",
            Value::String("unstarted".to_string()),
        );
        let priority = snapshot_field(
            "/headline/priority",
            "Priority",
            json!({"value": 1, "label": "Urgent"}),
        );
        let mut all_fields = vec![title.clone(), status.clone(), priority.clone()];
        all_fields.extend(fields);
        ActionCompositionSnapshot {
            action_id: "action-fixture-1".to_string(),
            title,
            status: Some(status),
            priority: Some(priority),
            fields: all_fields,
        }
    }

    fn output_json(output: &crate::abilities::provenance::AbilityOutput<Composition>) -> Value {
        serde_json::to_value(output).expect("output serializes")
    }

    #[test]
    fn registry_declaration_pins_action_policy_and_invalidation_signals() {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let descriptor = registry
            .iter_all()
            .find(|descriptor| descriptor.name == ABILITY_NAME)
            .expect("action detail ability registered");

        assert_eq!(descriptor.category, AbilityCategory::Read);
        assert_eq!(
            descriptor.policy.allowed_actors,
            &[ActorKind::User, ActorKind::SurfaceClient]
        );
        assert_eq!(descriptor.policy.required_scopes, &["read.action_detail"]);
        assert_eq!(descriptor.policy.mcp_exposure, McpExposure::Invocable);
        assert!(!descriptor.policy.client_side_executable);
        assert!(descriptor.mutates.is_empty());
        assert!(descriptor.signal_policy.coalesce);
        for signal in [
            "claim.version",
            "action_subject.claim_changed",
            "claim.lifecycle",
            "claim.dismissal",
            "source.freshness",
            "source.revocation",
            "action.field_changed",
            "action_completed",
            "action_reopened",
            "action_pushed_to_linear",
        ] {
            assert!(
                descriptor
                    .signal_policy
                    .emits_on_output_change
                    .contains(&signal),
                "action detail declares invalidation signal {signal}"
            );
        }
    }

    #[tokio::test]
    async fn missing_action_rejects_before_claim_read_or_commit() {
        let snapshot_reader = Arc::new(SpySnapshotReader::new(Err(
            ActionCompositionSnapshotReadError::ActionNotFound("action-fixture-1".to_string()),
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

        let err = match action_detail(&ctx, input()).await {
            Ok(_) => panic!("missing action rejects"),
            Err(error) => error,
        };

        assert_eq!(err.kind, AbilityErrorKind::Validation);
        assert!(err.message.contains("action-fixture-1"));
        assert_eq!(snapshot_reader.calls.load(Ordering::SeqCst), 1);
        assert_eq!(reader.calls.load(Ordering::SeqCst), 0);
        assert_eq!(committer.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn committed_output_filters_action_claims_and_emits_expected_sections() {
        let claims = vec![
            action_claim("claim-action", "Action-local context"),
            claim(
                "claim-account",
                json!({"kind": "account", "id": "acct-1"}),
                "Wrong subject context",
            ),
        ];
        let snapshot_reader = Arc::new(SpySnapshotReader::new(Ok(snapshot_fixture(vec![
            snapshot_field(
                "/context/body",
                "Context",
                Value::String("Existing action context".to_string()),
            ),
            snapshot_field(
                "/reference/source",
                "Source",
                json!({"type": "manual", "label": "User"}),
            ),
            snapshot_field(
                "/linear/issue",
                "Linear issue",
                json!({"identifier": "DOS-123", "url": "https://linear.example/DOS-123"}),
            ),
            snapshot_field(
                "/action-bar/status_toggle",
                "Action bar",
                json!({"can_complete": true, "status": "unstarted"}),
            ),
        ]))));
        let (clock, rng, external, reader, committer, provider) = fixture_parts(claims);
        let services = services(
            &clock,
            &rng,
            &external,
            reader.clone(),
            committer.clone(),
            snapshot_reader,
        );
        let ctx = ability_ctx(&services, &provider);

        let output = action_detail(&ctx, input()).await.expect("action detail");
        let output_value = output_json(&output);
        let rendered = output_value.to_string();

        assert!(rendered.contains("Action-local context"));
        assert!(!rendered.contains("Wrong subject context"));
        let sections = output
            .data()
            .sections
            .iter()
            .map(|section| section.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sections,
            vec![
                "headline",
                "status",
                "priority",
                "context",
                "reference",
                "linear",
                "action-bar"
            ]
        );
        assert!(matches!(
            output.provenance().subject.subject,
            SubjectRef::Action(ref id) if id == "action-fixture-1"
        ));
        assert_eq!(reader.calls.load(Ordering::SeqCst), 1);
        assert_eq!(committer.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn snapshot_read_failure_degrades_without_blocking_claim_render() {
        let snapshot_reader = Arc::new(SpySnapshotReader::new(Err(
            ActionCompositionSnapshotReadError::ReadFailed("boom".to_string()),
        )));
        let (clock, rng, external, reader, committer, provider) =
            fixture_parts(vec![action_claim(
                "claim-action",
                "Claim survives degradation",
            )]);
        let services = services(
            &clock,
            &rng,
            &external,
            reader,
            committer.clone(),
            snapshot_reader,
        );
        let ctx = ability_ctx(&services, &provider);

        let output = action_detail(&ctx, input())
            .await
            .expect("degraded action detail");
        let rendered = output_json(&output).to_string();

        assert!(rendered.contains("Claim survives degradation"));
        assert!(rendered.contains("action_snapshot_unavailable"));
        assert_eq!(committer.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn composition_id_rejects_subject_mismatch_and_extra_segments() {
        let mismatch = ActionDetailInput {
            composition_id: Some("dailyos/action-detail:action:other-action".to_string()),
            ..input()
        };
        let err = normalize_input(mismatch).expect_err("mismatch rejects");
        assert_eq!(err.kind, AbilityErrorKind::Validation);

        let extra = ActionDetailInput {
            composition_id: Some("dailyos/action-detail:action:action-fixture-1:extra".to_string()),
            ..input()
        };
        let err = normalize_input(extra).expect_err("extra segment rejects");
        assert_eq!(err.kind, AbilityErrorKind::Validation);
    }

    #[test]
    fn subject_ref_accepts_matching_action_and_rejects_mismatch() {
        let ok = ActionDetailInput {
            subject_ref: Some(json!({ "action": "action-fixture-1" })),
            ..input()
        };
        normalize_input(ok).expect("matching action subject ref is accepted");

        let missing = ActionDetailInput {
            subject_ref: Some(json!({ "person": "action-fixture-1" })),
            ..input()
        };
        assert_eq!(
            normalize_input(missing)
                .expect_err("missing action rejects")
                .kind,
            AbilityErrorKind::Validation
        );

        let mismatch = ActionDetailInput {
            subject_ref: Some(json!({ "action": "other-action" })),
            ..input()
        };
        assert_eq!(
            normalize_input(mismatch)
                .expect_err("action mismatch rejects")
                .kind,
            AbilityErrorKind::Validation
        );
    }
}

use std::cmp::Ordering;
use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
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
    data_source_from_key, workspace_file_id_from_source_ref, AbilityExecutionMode, AbilityVersion,
    Confidence, DataSource, DocumentId, EntityId, FieldAttribution, FieldPath, GleanDownstream,
    InputsSnapshot, InvocationId, ProvenanceBuilder, ProvenanceBuilderConfig, SchemaVersion,
    SourceAttribution, SourceIdentifier, SourceName, SourceRef, SubjectAttribution, SubjectRef,
};
use crate::abilities::trust::TrustBand;
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};
use crate::services::context::{
    AccountCompositionProvenanceKind, AccountCompositionSnapshot, AccountCompositionSnapshotField,
    AccountCompositionSnapshotReadError, CompositionCommitError, CompositionProposal,
};
use crate::types::{
    prompt_input_sensitivity_allowed, subject_ref_from_json, ClaimSensitivity, ClaimState,
    ClaimSubjectRef, IntelligenceClaim, SurfacingState,
};

const ABILITY_NAME: &str = "dailyos/account-overview";
const ABILITY_SCHEMA_VERSION: u32 = 1;
const ACCOUNT_CLAIM_DEPTH: usize = 3;

const VARIANT_D_SECTIONS: [(&str, &str); 18] = [
    ("headline", "Headline"),
    ("your-assessment", "Your assessment"),
    ("on-track", "On Track"),
    ("needs-attention", "Needs attention"),
    ("outlook", "Outlook"),
    ("relationship-health", "The Read"),
    ("about-intelligence", "About this intelligence"),
    ("thesis", "Thesis"),
    ("the-room", "The Room"),
    ("what-matters", "What matters"),
    ("value-commitments", "What we've built"),
    ("their-voice", "Their voice"),
    ("commercial-shape", "Commercial shape"),
    ("technical-shape", "Technical shape"),
    ("relationship-fabric", "Relationship fabric"),
    ("about-dossier", "About the dossier"),
    ("outputs", "Outputs"),
    ("the-record", "The Record"),
];

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AccountOverviewInput {
    pub schema_version: u32,
    pub account_id: String,
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
    account_id: String,
    expected_composition_version: u64,
    composition_id: CompositionDocId,
}

struct PreparedAccountOverview {
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
    snapshot: Option<AccountCompositionSnapshot>,
    degraded_reason: Option<String>,
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaimPlacement {
    Overview,
    Risk,
    Win,
    Value,
    Commitment,
    Relationship,
    Health,
    Ignored,
}

#[ability(
    name = "dailyos/account-overview",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, SurfaceClient],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.account_overview"],
    mcp_exposure = Invocable,
    client_side_executable = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [
        "claim.version",
        "account_subject.claim_changed",
        "claim.lifecycle",
        "claim.dismissal",
        "source.freshness",
        "source.revocation"
    ], coalesce = true }
)]
pub async fn account_overview(
    ctx: &AbilityContext<'_>,
    input: AccountOverviewInput,
) -> AbilityResult<Composition> {
    let input = normalize_input(input)?;
    let prepared = prepare_account_overview(ctx, &input).await?;
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

fn normalize_input(input: AccountOverviewInput) -> Result<NormalizedInput, AbilityError> {
    if input.schema_version != ABILITY_SCHEMA_VERSION {
        return Err(validation_error(format!(
            "unsupported schema_version `{}` for `{ABILITY_NAME}`",
            input.schema_version
        )));
    }
    let account_id = input.account_id.trim();
    if account_id.is_empty() {
        return Err(validation_error("account_id must be non-empty"));
    }
    validate_entity_envelope(
        "account",
        account_id,
        input.entity_type.as_deref(),
        input.entity_id.as_deref(),
    )?;
    let composition_id = input
        .composition_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("dailyos/account-overview:account:{account_id}"));

    Ok(NormalizedInput {
        account_id: account_id.to_string(),
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

async fn prepare_account_overview(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
) -> Result<PreparedAccountOverview, AbilityError> {
    // The reader currently takes (entity_type, entity_id, surface, depth)
    // without an actor/scope discriminator. The substrate-side SurfaceClient
    // scope contract (W4-B §16) calls for SQL-layer projection keyed on
    // Actor::SurfaceClient { scopes }; until that lands, scope filtering is
    // enforced one layer up by prompt_input_sensitivity_allowed, which gates
    // Confidential+ before any block, ClaimRef, or count flows into the
    // composition. Behavior is preserved; the longer-term tightening is
    // tracked in the maintenance project.
    let claims = ctx
        .services()
        .read_entity_context_claims(
            "account".to_string(),
            input.account_id.clone(),
            ctx.entity_context_claim_surface(),
            ACCOUNT_CLAIM_DEPTH,
        )
        .await
        .map_err(|error| hard_error("account_overview_claim_read", error))?;

    let subject_ref = SubjectRef::Account(input.account_id.clone());
    let subject = SubjectAttribution::direct_confident(subject_ref);
    let provenance_config = provenance_config(ctx);
    let invocation_id = provenance_config.invocation_id;
    let mut provenance_builder = ProvenanceBuilder::new(provenance_config);
    provenance_builder.set_subject(subject.clone());

    let mut projections = Vec::new();
    for claim in claims {
        let Some(projection) =
            project_claim(ctx, &input.account_id, claim, &mut provenance_builder)?
        else {
            continue;
        };
        projections.push(projection);
    }
    projections.sort_by(compare_claim_projection);

    let snapshot = read_account_snapshot(ctx, input).await?;
    let composition = build_composition(
        ctx,
        input,
        &projections,
        &snapshot,
        &subject,
        invocation_id,
        &mut provenance_builder,
    )?;

    Ok(PreparedAccountOverview {
        proposal: CompositionProposal {
            composition_id: input.composition_id.clone(),
            expected_composition_version: input.expected_composition_version,
            composition,
        },
        provenance_builder,
    })
}

async fn read_account_snapshot(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
) -> Result<SnapshotReadOutcome, AbilityError> {
    match ctx
        .services()
        .read_account_composition_snapshot(
            input.account_id.clone(),
            ctx.entity_context_claim_surface(),
        )
        .await
    {
        Ok(snapshot) => Ok(SnapshotReadOutcome {
            snapshot: Some(snapshot),
            degraded_reason: None,
        }),
        Err(AccountCompositionSnapshotReadError::AccountNotFound(account_id)) => Err(
            validation_error(format!("account `{account_id}` was not found")),
        ),
        Err(AccountCompositionSnapshotReadError::ReadFailed(_)) => Ok(SnapshotReadOutcome {
            snapshot: None,
            degraded_reason: Some("account_snapshot_unavailable".to_string()),
        }),
    }
}

fn project_claim(
    ctx: &AbilityContext<'_>,
    account_id: &str,
    claim: IntelligenceClaim,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Option<ClaimProjection>, AbilityError> {
    if !claim_is_eligible_for_account_overview(&claim, account_id)? {
        return Ok(None);
    }
    let Some(metadata) = metadata_for_name(&claim.claim_type) else {
        return Err(validation_error(format!(
            "unknown claim_type `{}` in account overview input",
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

    let source = source_for_claim(ctx, account_id, &claim)?;
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

fn claim_is_eligible_for_account_overview(
    claim: &IntelligenceClaim,
    account_id: &str,
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
        ClaimSubjectRef::Account { id } => Ok(id == account_id),
        ClaimSubjectRef::Action { .. }
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
        ClaimType::Win | ClaimType::EntityWin => ClaimPlacement::Win,
        ClaimType::ValueDelivered => ClaimPlacement::Value,
        ClaimType::Commitment | ClaimType::OpenLoop | ClaimType::Recommendation => {
            ClaimPlacement::Commitment
        }
        ClaimType::StakeholderEngagement
        | ClaimType::StakeholderAssessment
        | ClaimType::StakeholderRole => ClaimPlacement::Relationship,
        ClaimType::EntityCurrentState => ClaimPlacement::Health,
        ClaimType::CompanyContext
        | ClaimType::AccountFact
        | ClaimType::EntityIdentity
        | ClaimType::EntitySummary
        | ClaimType::UserNote => ClaimPlacement::Overview,
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
    let mut sections = Vec::with_capacity(VARIANT_D_SECTIONS.len());
    let block_context = AccountBlockBuildContext {
        ctx,
        input,
        subject,
        invocation_id,
    };

    let headline_block = build_overview_block(
        &block_context,
        projections,
        snapshot_outcome,
        "/sections/0/blocks/0",
        provenance_builder,
    )?;
    sections.push(variant_section(
        "headline",
        "Headline",
        vec![headline_block],
        SectionLayout::Stacked,
        salience(0.95, SalienceBand::Critical, "account masthead"),
    ));

    // Production spine: one section per main-branch chapter, claims routed by
    // placement to their natural section, every other section driven by the
    // snapshot domain payloads. Section emission is uniform: the group builder
    // decides whether the section has content.
    let claims_for = |placements: &[ClaimPlacement]| -> Vec<&ClaimProjection> {
        projections
            .iter()
            .filter(|projection| placements.contains(&projection.placement))
            .collect()
    };
    type SectionPlanRow<'p> = (
        &'static str,
        &'static str,
        Vec<&'p ClaimProjection>,
        SectionLayout,
        f32,
        SalienceBand,
        &'static str,
    );
    let plan: [SectionPlanRow<'_>; 15] = [
        ("your-assessment", "Your assessment", Vec::new(), SectionLayout::Stacked, 0.92, SalienceBand::Critical, "user assessment"),
        ("on-track", "On Track", claims_for(&[ClaimPlacement::Win, ClaimPlacement::Value, ClaimPlacement::Overview]), SectionLayout::Stacked, 0.84, SalienceBand::Important, "what is on track"),
        ("needs-attention", "Needs attention", claims_for(&[ClaimPlacement::Risk]), SectionLayout::Stacked, 0.9, SalienceBand::Critical, "needs attention"),
        ("outlook", "Outlook", claims_for(&[ClaimPlacement::Health]), SectionLayout::Stacked, 0.86, SalienceBand::Important, "renewal outlook"),
        ("relationship-health", "The Read", Vec::new(), SectionLayout::Stacked, 0.6, SalienceBand::Contextual, "health vs signals"),
        ("about-intelligence", "About this intelligence", Vec::new(), SectionLayout::Stacked, 0.3, SalienceBand::Background, "about this intelligence"),
        ("thesis", "Thesis", Vec::new(), SectionLayout::Stacked, 0.7, SalienceBand::Important, "editorial thesis"),
        ("the-room", "The Room", claims_for(&[ClaimPlacement::Relationship]), SectionLayout::Stacked, 0.72, SalienceBand::Important, "stakeholders"),
        ("what-matters", "What matters", Vec::new(), SectionLayout::Stacked, 0.62, SalienceBand::Contextual, "what matters to them"),
        ("value-commitments", "What we've built", claims_for(&[ClaimPlacement::Win, ClaimPlacement::Value]), SectionLayout::Stacked, 0.72, SalienceBand::Important, "value and commitments"),
        ("their-voice", "Their voice", claims_for(&[ClaimPlacement::Overview, ClaimPlacement::Risk, ClaimPlacement::Win, ClaimPlacement::Value, ClaimPlacement::Commitment, ClaimPlacement::Relationship, ClaimPlacement::Health]), SectionLayout::Stacked, 0.5, SalienceBand::Contextual, "their voice"),
        ("commercial-shape", "Commercial shape", Vec::new(), SectionLayout::Stacked, 0.55, SalienceBand::Contextual, "commercial shape"),
        ("technical-shape", "Technical shape", Vec::new(), SectionLayout::Stacked, 0.45, SalienceBand::Background, "technical shape"),
        ("relationship-fabric", "Relationship fabric", Vec::new(), SectionLayout::Stacked, 0.45, SalienceBand::Background, "relationship fabric"),
        ("about-dossier", "About the dossier", Vec::new(), SectionLayout::Stacked, 0.25, SalienceBand::Background, "about the dossier"),
    ];
    for (section_id, label, section_claims, layout, weight, band, reason) in plan {
        let section_index = sections.len();
        let blocks = build_claim_or_snapshot_section_blocks(
            ctx,
            input,
            section_id,
            section_index,
            section_claims,
            Vec::new(),
            snapshot,
            EmptySectionCopy {
                title: "No content yet",
                body: "DailyOS has not found renderable content for this chapter.",
                status: "empty",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?;
        sections.push(variant_section(section_id, label, blocks, layout, salience(weight, band, reason)));
    }

    // Outputs: report grid is frontend config keyed by account — the block is
    // a pure marker the renderer resolves.
    {
        let section_index = sections.len();
        let composition_block_path = format!("/sections/{section_index}/blocks/0");
        let mut block = Block::new(
            BlockId::new(block_id(input, "outputs", "action_list", "outputs")),
            BlockType::ActionList,
            json!({ "block": "outputs", "items": [] }),
            Vec::new(),
            ProvenanceRef::new(
                invocation_id,
                FieldPath::new(&composition_block_path).map_err(field_error)?,
            ),
            None,
        )
        .map_err(block_error)?;
        block.field_bindings = vec![display_only_binding("/block")?];
        block.salience = salience(0.35, SalienceBand::Background, "report outputs");
        attribute_block(provenance_builder, &composition_block_path, subject, Vec::new())?;
        sections.push(variant_section(
            "outputs",
            "Outputs",
            vec![block],
            SectionLayout::Grid,
            salience(0.35, SalienceBand::Background, "report outputs"),
        ));
    }

    // The Record: production UnifiedTimeline over the record bundle.
    {
        let section_index = sections.len();
        let record_value = snapshot.and_then(|snap| snap.record.clone());
        let mut blocks = Vec::new();
        if let Some(record) = record_value {
            let composition_block_path = format!("/sections/{section_index}/blocks/0");
            let mut block = Block::new(
                BlockId::new(block_id(input, "the-record", "evidence_list", "record")),
                BlockType::EvidenceList,
                json!({ "block": "record", "record": record }),
                Vec::new(),
                ProvenanceRef::new(
                    invocation_id,
                    FieldPath::new(&composition_block_path).map_err(field_error)?,
                ),
                None,
            )
            .map_err(block_error)?;
            block.field_bindings = vec![
                display_only_binding("/block")?,
                display_only_binding("/record")?,
            ];
            block.salience = salience(0.4, SalienceBand::Background, "account record");
            attribute_block(provenance_builder, &composition_block_path, subject, Vec::new())?;
            blocks.push(block);
        }
        sections.push(variant_section(
            "the-record",
            "The Record",
            blocks,
            SectionLayout::Stacked,
            salience(0.4, SalienceBand::Background, "account record"),
        ));
    }

    let section_count = sections.len();
    let generated_at = ctx.services().clock.now();
    let composition = Composition::new(
        input.composition_id.clone(),
        CompositionKind::EntityPage,
        Some(EntityRef::new(format!("account:{}", input.account_id))),
        sections,
        salience(0.9, SalienceBand::Important, "account overview"),
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

#[derive(Clone, Copy)]
struct AccountBlockBuildContext<'a, 'ctx> {
    ctx: &'a AbilityContext<'ctx>,
    input: &'a NormalizedInput,
    subject: &'a SubjectAttribution,
    invocation_id: InvocationId,
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
    snapshot_fields: Vec<&AccountCompositionSnapshotField>,
    snapshot: Option<&AccountCompositionSnapshot>,
    empty_copy: EmptySectionCopy,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Vec<Block>, AbilityError> {
    // A section renders when any of its production blocks has content —
    // claims, enriched intelligence, or a domain payload (glean, sentiment,
    // stakeholders, footprint). Claim coverage alone no longer gates it.
    let groups = section_block_groups(section_id, &projections, snapshot);
    let has_chapter_content = groups.iter().any(|group| {
        !group.projections.is_empty()
            || group.intelligence.is_some()
            || group.extras.len() > 1
    });
    let mut blocks = if has_chapter_content {
        build_projection_blocks(
            input,
            section_id,
            section_index,
            groups,
            subject,
            invocation_id,
            provenance_builder,
        )?
    } else {
        Vec::new()
    };
    if has_chapter_content {
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

/// Chapter-shaped aggregate emission. Each VARIANT_D chapter renders as ONE
/// aggregate block (state-of-play as up to three intent groups) whose payload
/// carries every claim as an item with its own claim_ref, provenance_kind,
/// and `/items/N/text` feedback binding — so per-claim trust (opacity) and
/// confirm/contest survive inside chapter layouts, and every surface that
/// consumes the composition (Tauri, MCP, WP) receives the chapter shape
/// instead of re-deriving it from a stack of single-claim blocks.
fn build_projection_blocks(
    input: &NormalizedInput,
    section_id: &str,
    section_index: usize,
    groups: Vec<SectionBlockGroup<'_>>,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Vec<Block>, AbilityError> {
    let mut blocks = Vec::new();
    for group in groups {
        if group.projections.is_empty() && group.intelligence.is_none() && group.extras.len() <= 1 {
            continue;
        }
        let block_index = blocks.len();
        blocks.push(build_aggregate_claim_block(
            input,
            section_id,
            group,
            subject,
            invocation_id,
            &format!("/sections/{section_index}/blocks/{block_index}"),
            provenance_builder,
        )?);
    }
    Ok(blocks)
}

/// Per-chapter subset of the account's enriched intelligence payload — the
/// SAME content contract the production account-detail chapters render
/// (StateOfPlay reads currentState, WatchList reads risks/recentWins, …).
/// Returns None when the account has no enriched content for the chapter so
/// the section can fall back to claims or its empty copy.
fn section_intelligence_subset(
    section_id: &str,
    intelligence: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    let intelligence = intelligence?.as_object()?;
    let content_keys: &[&str] = match section_id {
        "on-track" => &["currentState", "executiveAssessment", "pullQuote"],
        "outlook" => &[
            "agreementOutlook",
            "contractContext",
            "expansionSignals",
            "health",
            "consistencyFindings",
        ],
        "the-room" => &["stakeholderInsights"],
        "what-matters" => &[
            "strategicPriorities",
            "competitiveContext",
            "marketContext",
            "regulatoryContext",
        ],
        "value-commitments" => &["valueDelivered", "successMetrics", "openCommitments"],
        "thesis" => &["pullQuote"],
        _ => return None,
    };
    let mut subset = serde_json::Map::new();
    for key in content_keys {
        match intelligence.get(*key) {
            Some(serde_json::Value::Null) | None => {}
            Some(value) if value.as_array().is_some_and(Vec::is_empty) => {}
            Some(value) => {
                subset.insert((*key).to_string(), value.clone());
            }
        }
    }
    if subset.is_empty() {
        return None;
    }
    if let Some(enriched_at) = intelligence.get("enrichedAt") {
        subset.insert("enrichedAt".to_string(), enriched_at.clone());
    }
    Some(serde_json::Value::Object(subset))
}

/// One emitted aggregate block: a block type, the payload item key, an
/// optional intent tag (drives grouping labels/accents on the surface), the
/// member claims, and the block's salience.
struct SectionBlockGroup<'a> {
    block_type: BlockType,
    item_key: &'static str,
    intent: Option<&'static str>,
    group_key: &'static str,
    projections: Vec<&'a ClaimProjection>,
    salience_value: f32,
    salience_band: SalienceBand,
    salience_reason: &'static str,
    /// Chapter intelligence subset riding this block (production content).
    intelligence: Option<serde_json::Value>,
    /// Production-block payload: a "block" discriminator naming the
    /// main-branch component this block renders through, plus the domain
    /// values that component consumes (gleanSignals, sentiment,
    /// stakeholders, technicalFootprint, findings, …).
    extras: serde_json::Map<String, serde_json::Value>,
}

fn section_block_groups<'a>(
    section_id: &str,
    projections: &[&'a ClaimProjection],
    snapshot: Option<&AccountCompositionSnapshot>,
) -> Vec<SectionBlockGroup<'a>> {
    let all = || projections.to_vec();
    let intelligence_value = snapshot.and_then(|snap| snap.intelligence.as_ref());
    let subset = || section_intelligence_subset(section_id, intelligence_value);
    let domain = |key: &str| -> Option<serde_json::Value> {
        let snap = snapshot?;
        match key {
            "gleanSignals" => snap.glean_signals.clone(),
            "sentiment" => snap.sentiment.clone(),
            "stakeholders" => snap.stakeholders.clone(),
            "technicalFootprint" => snap.technical_footprint.clone(),
            "commercial" => snap.commercial.clone(),
            "fabric" => snap.fabric.clone(),
            _ => None,
        }
    };
    let intel_keys = |keys: &[&str]| -> Option<serde_json::Value> {
        let intelligence = intelligence_value?.as_object()?;
        let mut out = serde_json::Map::new();
        for key in keys {
            match intelligence.get(*key) {
                Some(serde_json::Value::Null) | None => {}
                Some(value) if value.as_array().is_some_and(Vec::is_empty) => {}
                Some(value) => {
                    out.insert((*key).to_string(), value.clone());
                }
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(out))
        }
    };
    let extras = |block: &str, entries: Vec<(&str, Option<serde_json::Value>)>| {
        let mut map = serde_json::Map::new();
        map.insert("block".to_string(), serde_json::json!(block));
        for (key, value) in entries {
            if let Some(value) = value {
                map.insert(key.to_string(), value);
            }
        }
        map
    };
    // A domain-only block: emitted iff its payload exists (extras > 1 key).
    let domain_block = |block_type: BlockType,
                        group_key: &'static str,
                        block: &str,
                        entries: Vec<(&str, Option<serde_json::Value>)>,
                        weight: f32,
                        band: SalienceBand,
                        reason: &'static str| {
        let has_content = entries.iter().any(|(_, value)| value.is_some());
        SectionBlockGroup {
            block_type,
            item_key: "items",
            intent: None,
            group_key,
            projections: Vec::new(),
            salience_value: weight,
            salience_band: band,
            salience_reason: reason,
            intelligence: None,
            extras: if has_content {
                extras(block, entries)
            } else {
                serde_json::Map::new()
            },
        }
    };

    match section_id {
        "your-assessment" => vec![domain_block(
            BlockType::HealthSnapshot,
            "sentiment-hero",
            "sentiment_hero",
            vec![("sentiment", domain("sentiment"))],
            0.92,
            SalienceBand::Critical,
            "user assessment",
        )],
        "on-track" => vec![SectionBlockGroup {
            block_type: BlockType::ClaimSummary,
            item_key: "items",
            intent: Some("context"),
            group_key: "on-track",
            projections: all(),
            salience_value: 0.84,
            salience_band: SalienceBand::Important,
            salience_reason: "what is on track",
            intelligence: subset(),
            extras: extras("on_track", vec![]),
        }],
        "needs-attention" => vec![
            SectionBlockGroup {
                block_type: BlockType::RiskCallout,
                item_key: "items",
                intent: Some("risk"),
                group_key: "triage",
                projections: all(),
                salience_value: 0.9,
                salience_band: SalienceBand::Critical,
                salience_reason: "needs attention",
                intelligence: None,
                extras: {
                    let triage_intel = intel_keys(&[
                        "risks",
                        "recentWins",
                        "currentState",
                        "blockers",
                        "enrichedAt",
                    ]);
                    if triage_intel.is_none() && domain("gleanSignals").is_none() {
                        serde_json::Map::new()
                    } else {
                        extras(
                            "triage",
                            vec![
                                ("intelligence", triage_intel),
                                ("gleanSignals", domain("gleanSignals")),
                                ("sentiment", domain("sentiment")),
                            ],
                        )
                    }
                },
            },
            SectionBlockGroup {
                block_type: BlockType::RiskCallout,
                item_key: "items",
                intent: Some("risk"),
                group_key: "divergence",
                projections: Vec::new(),
                salience_value: 0.7,
                salience_band: SalienceBand::Important,
                salience_reason: "consistency divergence",
                intelligence: None,
                extras: {
                    let findings = intelligence_value
                        .and_then(|intel| intel.get("consistencyFindings"))
                        .filter(|value| value.as_array().is_some_and(|rows| !rows.is_empty()))
                        .cloned();
                    if findings.is_none() {
                        serde_json::Map::new()
                    } else {
                        extras(
                            "divergence",
                            vec![
                                ("findings", findings),
                                ("gleanSignals", domain("gleanSignals")),
                            ],
                        )
                    }
                },
            },
        ],
        "outlook" => vec![SectionBlockGroup {
            block_type: BlockType::HealthSnapshot,
            item_key: "items",
            intent: None,
            group_key: "outlook",
            projections: all(),
            salience_value: 0.86,
            salience_band: SalienceBand::Important,
            salience_reason: "renewal outlook",
            intelligence: subset(),
            extras: extras("outlook_panel", vec![]),
        }],
        "relationship-health" => vec![domain_block(
            BlockType::HealthSnapshot,
            "supporting-tension",
            "supporting_tension",
            vec![
                ("intelligence", intel_keys(&["health", "enrichedAt"])),
                ("gleanSignals", domain("gleanSignals")),
            ],
            0.6,
            SalienceBand::Contextual,
            "health vs signals",
        )],
        "about-intelligence" => vec![domain_block(
            BlockType::HealthSnapshot,
            "about-intelligence",
            "about_intelligence",
            vec![
                (
                    "intelligence",
                    intel_keys(&["enrichedAt", "sourceFileCount", "sourceManifest"]),
                ),
                ("gleanSignals", domain("gleanSignals")),
            ],
            0.3,
            SalienceBand::Background,
            "about this intelligence",
        )],
        "thesis" => vec![domain_block(
            BlockType::ClaimSummary,
            "thesis",
            "thesis",
            vec![("intelligence", intel_keys(&["pullQuote", "executiveAssessment", "enrichedAt"]))],
            0.7,
            SalienceBand::Important,
            "editorial thesis",
        )],
        "the-room" => vec![SectionBlockGroup {
            block_type: BlockType::RelationshipMap,
            item_key: "nodes",
            intent: None,
            group_key: "room",
            projections: all(),
            salience_value: 0.72,
            salience_band: SalienceBand::Important,
            salience_reason: "stakeholders",
            intelligence: subset(),
            extras: extras("stakeholder_grid", vec![("stakeholders", domain("stakeholders"))]),
        }],
        "what-matters" => vec![SectionBlockGroup {
            block_type: BlockType::ClaimSummary,
            item_key: "items",
            intent: Some("context"),
            group_key: "what-matters",
            projections: Vec::new(),
            salience_value: 0.62,
            salience_band: SalienceBand::Contextual,
            salience_reason: "what matters to them",
            intelligence: subset(),
            extras: extras("what_matters", vec![]),
        }],
        "value-commitments" => vec![SectionBlockGroup {
            block_type: BlockType::ClaimSummary,
            item_key: "items",
            intent: Some("value"),
            group_key: "built",
            projections: all(),
            salience_value: 0.72,
            salience_band: SalienceBand::Important,
            salience_reason: "value and commitments",
            intelligence: subset(),
            extras: extras("built", vec![]),
        }],
        "their-voice" => {
            let quote_projections = projections
                .iter()
                .copied()
                .filter(|projection| verified_transcript_quote_payload(projection).is_some())
                .collect::<Vec<_>>();
            let quotes = snapshot
                .and_then(|snap| snap.glean_signals.as_ref())
                .and_then(|glean| glean.get("quoteWall"))
                .filter(|quotes| quotes.as_array().is_some_and(|rows| !rows.is_empty()))
                .cloned()
                .or_else(|| quote_wall_from_transcript_claims(&quote_projections));
            vec![SectionBlockGroup {
                block_type: BlockType::EvidenceList,
                item_key: "items",
                intent: None,
                group_key: "their-voice",
                projections: quote_projections,
                salience_value: 0.5,
                salience_band: SalienceBand::Contextual,
                salience_reason: "their voice",
                intelligence: None,
                extras: extras("quote_wall", vec![("quotes", quotes)]),
            }]
        }
        "commercial-shape" => vec![domain_block(
            BlockType::ClaimSummary,
            "commercial-shape",
            "commercial_shape",
            vec![("commercial", domain("commercial"))],
            0.55,
            SalienceBand::Contextual,
            "commercial shape",
        )],
        "technical-shape" => vec![domain_block(
            BlockType::ClaimSummary,
            "technical-shape",
            "technical_footprint",
            vec![("technicalFootprint", domain("technicalFootprint"))],
            0.45,
            SalienceBand::Background,
            "technical shape",
        )],
        "relationship-fabric" => vec![domain_block(
            BlockType::ClaimSummary,
            "relationship-fabric",
            "relationship_fabric",
            vec![("fabric", domain("fabric"))],
            0.45,
            SalienceBand::Background,
            "relationship fabric",
        )],
        "about-dossier" => vec![domain_block(
            BlockType::ClaimSummary,
            "about-dossier",
            "about_dossier",
            vec![(
                "intelligence",
                intel_keys(&["enrichedAt", "sourceFileCount", "sourceManifest"]),
            )],
            0.25,
            SalienceBand::Background,
            "about the dossier",
        )],
        // the-work and any future claim section: one action list.
        _ => vec![SectionBlockGroup {
            block_type: BlockType::ActionList,
            item_key: "items",
            intent: None,
            group_key: "actions",
            projections: all(),
            salience_value: 0.62,
            salience_band: SalienceBand::Important,
            salience_reason: "commitment claims",
            intelligence: subset(),
            extras: serde_json::Map::new(),
        }],
    }
}

fn quote_wall_from_transcript_claims(projections: &[&ClaimProjection]) -> Option<Value> {
    let quotes = projections
        .iter()
        .filter_map(|projection| verified_transcript_quote_payload(projection))
        .collect::<Vec<_>>();
    if quotes.is_empty() {
        None
    } else {
        Some(Value::Array(quotes))
    }
}

fn verified_transcript_quote_payload(projection: &ClaimProjection) -> Option<Value> {
    let metadata: Value = serde_json::from_str(projection.claim.metadata_json.as_deref()?).ok()?;
    if !metadata
        .get("quote_verified")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    let quote = metadata.pointer("/quote/text")?.as_str()?.trim();
    if quote.is_empty() {
        return None;
    }
    let workspace_file_kind = metadata
        .get("workspace_file_kind")
        .and_then(Value::as_str)
        .unwrap_or("transcript");
    let source_label = data_source_for_claim(&projection.claim.data_source).display_name();
    let source_asof = projection.claim.source_asof.as_deref();
    let claim_kind = metadata
        .get("claim_kind")
        .and_then(Value::as_str)
        .unwrap_or(projection.claim.claim_type.as_str());
    let redaction_policy = metadata
        .pointer("/quote/redaction_policy")
        .and_then(Value::as_str)
        .unwrap_or("sensitivity_ceiling");

    Some(json!({
        "claim_id": projection.claim.id,
        "speaker": "Customer",
        "quote": quote,
        "evidence_quote": quote,
        "assertion_text": projection.rendered_text,
        "meetingDate": source_asof,
        "meetingTitle": "Transcript",
        "topicTags": [claim_kind],
        "sentiment": transcript_quote_sentiment(&projection.claim.claim_type),
        "publicSafeConfidence": "high",
        "source_label": source_label,
        "source_asof": source_asof,
        "workspace_file_kind": workspace_file_kind,
        "workspaceFileKind": workspace_file_kind,
        "trust_band": trust_band_label(projection.trust_band),
        "sensitivity": sensitivity_label(&projection.claim.sensitivity),
        "redaction_policy": redaction_policy,
        "redaction_state": "policy_allowed"
    }))
}

fn transcript_quote_sentiment(claim_type: &str) -> &'static str {
    match claim_type {
        "entity_win" | "value_delivered" => "positive",
        "entity_risk" => "negative",
        _ => "neutral",
    }
}

fn sensitivity_label(sensitivity: &ClaimSensitivity) -> &'static str {
    match sensitivity {
        ClaimSensitivity::Public => "public",
        ClaimSensitivity::Internal => "internal",
        ClaimSensitivity::Confidential => "confidential",
        ClaimSensitivity::UserOnly => "user_only",
    }
}

#[allow(clippy::too_many_arguments)]
fn build_aggregate_claim_block(
    input: &NormalizedInput,
    section_id: &str,
    group: SectionBlockGroup<'_>,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    composition_block_path: &str,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let item_key = group.item_key;
    let lead = group.projections.first();

    let mut claim_refs = Vec::with_capacity(group.projections.len());
    let mut source_indexes = Vec::with_capacity(group.projections.len());
    let mut items = Vec::with_capacity(group.projections.len());
    let mut bindings = Vec::new();
    for (index, projection) in group.projections.iter().enumerate() {
        claim_refs.push(claim_ref_for_projection(projection)?);
        source_indexes.push(projection.source_index);
        items.push(json!({
            "claim_id": projection.claim.id,
            "text": projection.rendered_text,
            "claim_type": projection.claim.claim_type,
            "trust_band": trust_band_label(projection.trust_band),
            "source_asof": projection.claim.source_asof,
            "provenance_kind": claim_provenance_kind(&projection.claim),
        }));
        let text_path = format!("/{item_key}/{index}/text");
        bindings.push(binding(&text_path, BindingRole::Source, vec![index])?);
        bindings.push(binding(&text_path, BindingRole::FeedbackTarget, vec![index])?);
        bindings.push(computed_binding_for_indexes(
            &format!("/{item_key}/{index}/trust_band"),
            vec![index],
        )?);
        for field in ["claim_id", "claim_type", "source_asof", "provenance_kind"] {
            bindings.push(display_only_binding(&format!("/{item_key}/{index}/{field}"))?);
        }
    }

    // Block-level metadata mirrors the lead claim so the shell (trust band,
    // freshness, fallback selection) keeps a stable contract for aggregates.
    let mut attributes = json!({ item_key: items });
    if let Some(lead) = lead {
        attributes["claim_type"] = json!(lead.claim.claim_type);
        attributes["trust_band"] = json!(trust_band_label(lead.trust_band));
        attributes["source_asof"] = json!(lead.claim.source_asof);
    }
    if let Some(intent) = group.intent {
        attributes["intent"] = json!(intent);
    }
    for field in ["claim_type", "trust_band", "source_asof", "intent"] {
        if attributes.get(field).is_some() {
            bindings.push(display_only_binding(&format!("/{field}"))?);
        }
    }
    // The chapter's production content contract: the enriched intelligence
    // subset rides the payload at /intelligence/<key>, display-bound so the
    // projection admits it through the per-type field policies.
    if let Some(serde_json::Value::Object(subset)) = group.intelligence {
        for key in subset.keys() {
            bindings.push(display_only_binding(&format!("/intelligence/{key}"))?);
        }
        attributes["intelligence"] = serde_json::Value::Object(subset);
    }
    // Production-block payload: the "block" discriminator + the domain values
    // the main-branch component consumes, all display-bound.
    for (key, value) in group.extras {
        if key == "intelligence" {
            // Domain-scoped intelligence subset (e.g. triage) — bind per key
            // so the per-type /intelligence/<key> policies admit it.
            if let serde_json::Value::Object(ref subset) = value {
                for sub_key in subset.keys() {
                    bindings.push(display_only_binding(&format!("/intelligence/{sub_key}"))?);
                }
            }
            attributes["intelligence"] = value;
            continue;
        }
        bindings.push(display_only_binding(&format!("/{key}"))?);
        attributes[key.as_str()] = value;
    }

    let mut block = Block::new(
        BlockId::new(block_id(
            input,
            section_id,
            group.block_type.type_id(),
            group.group_key,
        )),
        group.block_type,
        attributes,
        claim_refs,
        ProvenanceRef::new(
            invocation_id,
            FieldPath::new(composition_block_path).map_err(field_error)?,
        ),
        None,
    )
    .map_err(block_error)?;
    block.field_bindings = bindings;
    block.salience = salience(group.salience_value, group.salience_band, group.salience_reason);
    attribute_block(
        provenance_builder,
        composition_block_path,
        subject,
        source_indexes,
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
    fields: Vec<&AccountCompositionSnapshotField>,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let source_indexes = snapshot_source_indexes(ctx, input, &fields, provenance_builder)?;
    let items = fields
        .iter()
        .map(|field| snapshot_field_evidence_item(field))
        .collect::<Vec<_>>();
    let composition_block_path = format!("/sections/{section_index}/blocks/{block_index}");
    let mut block = Block::new(
        BlockId::new(block_id(input, section_id, "evidence_list", "snapshot")),
        BlockType::EvidenceList,
        json!({ "items": items }),
        Vec::new(),
        ProvenanceRef::new(
            invocation_id,
            FieldPath::new(&composition_block_path).map_err(field_error)?,
        ),
        None,
    )
    .map_err(block_error)?;
    block.field_bindings = evidence_list_display_bindings(items.len())?;
    block.salience = salience(0.54, SalienceBand::Contextual, "snapshot fields");
    attribute_block(
        provenance_builder,
        &composition_block_path,
        subject,
        source_indexes,
    )?;
    Ok(block)
}

fn snapshot_field_evidence_item(field: &AccountCompositionSnapshotField) -> Value {
    json!({
        "label": format!("{}: {}", field.label, snapshot_value_text(&field.value)),
        "source_label": field.source_label.as_deref().unwrap_or(match field.provenance_kind {
            AccountCompositionProvenanceKind::NonSensitiveIdentity => "identity",
            AccountCompositionProvenanceKind::ManualUser => "user",
            AccountCompositionProvenanceKind::SourceField => "source",
            AccountCompositionProvenanceKind::SystemConfig => "system_config",
            AccountCompositionProvenanceKind::Derived => "derived",
            AccountCompositionProvenanceKind::Unavailable => "unavailable",
        }),
        "source_asof": field.source_asof,
    })
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

fn build_overview_block(
    build_context: &AccountBlockBuildContext<'_, '_>,
    projections: &[ClaimProjection],
    snapshot_outcome: &SnapshotReadOutcome,
    composition_block_path: &str,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let snapshot = snapshot_outcome.snapshot.as_ref();
    let headline_fields = snapshot_fields_for_section(snapshot, "headline");
    let mut source_indexes = projections
        .iter()
        .map(|projection| projection.source_index)
        .collect::<Vec<_>>();
    source_indexes.extend(snapshot_source_indexes(
        build_context.ctx,
        build_context.input,
        &headline_fields,
        provenance_builder,
    )?);
    let overview_claims = projections
        .iter()
        .enumerate()
        .filter(|(_, projection)| projection.placement == ClaimPlacement::Overview)
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
    let account_display_name = snapshot
        .map(|snapshot| snapshot_value_text(&snapshot.display_name.value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| build_context.input.account_id.clone());
    let account_type = snapshot
        .and_then(|snapshot| snapshot.account_type.as_ref())
        .filter(|field| field.sensitivity.is_render_safe())
        .map(|field| snapshot_value_text(&field.value));
    let vitals = headline_fields
        .iter()
        .map(|field| {
            let (display_value, kind) = vital_display(field);
            json!({
                "label": field.label,
                "value": field.value,
                "display_value": display_value,
                "kind": kind,
                "source_label": vital_source_label(field.source_label.as_deref()),
                "source_asof": field.source_asof,
                "trust_band": trust_band_label(field.trust_band),
            })
        })
        .collect::<Vec<_>>();
    let vitals_len = vitals.len();
    let attributes = json!({
        "account_id": build_context.input.account_id,
        "account": {
            "id": build_context.input.account_id,
            "display_name": account_display_name,
            "type": account_type,
        },
        "title": "Account overview",
        "summary": overview_claims
            .first()
            .map(|(_, projection)| projection.rendered_text.as_str())
            .unwrap_or("Account composition is grounded in current renderable claims and source-backed account fields."),
        "claim_count": projections.len(),
        "trust_band": trust_band_label(trust_band),
        "counts_by_trust_band": counts_by_band,
        "context": context,
        "vitals": vitals,
        "snapshot_degraded": snapshot_outcome.degraded_reason.as_deref().unwrap_or(""),
    });
    let mut block = Block::new(
        BlockId::new(block_id(
            build_context.input,
            "headline",
            "account_overview",
            "summary",
        )),
        BlockType::AccountOverview,
        attributes,
        all_refs,
        ProvenanceRef::new(
            build_context.invocation_id,
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
        display_only_binding("/snapshot_degraded")?,
    ];
    display_bindings.extend(vitals_display_bindings(vitals_len)?);

    if projections.is_empty() {
        block.field_bindings = display_bindings;
        attribute_block(
            provenance_builder,
            composition_block_path,
            build_context.subject,
            source_indexes,
        )?;
    } else {
        let mut bindings = vec![computed_binding("/claim_count", 0..projections.len())?];
        bindings.extend(trust_band_count_computed_bindings(projections.len())?);
        bindings.extend(context_computed_bindings(&overview_claim_indexes)?);
        bindings.extend(display_bindings);
        block.field_bindings = bindings;
        attribute_block(
            provenance_builder,
            composition_block_path,
            build_context.subject,
            source_indexes,
        )?;
    }
    Ok(block)
}

fn snapshot_fields_for_section<'a>(
    snapshot: Option<&'a AccountCompositionSnapshot>,
    section_id: &str,
) -> Vec<&'a AccountCompositionSnapshotField> {
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
        "headline" => {
            field_path.starts_with("/vitals/")
                || field_path.starts_with("/identity/")
                || field_path == "/health/band"
        }
        "outlook" => {
            field_path.starts_with("/outlook/")
                || field_path.starts_with("/renewal/")
                || field_path == "/vitals/contract_end"
                || field_path == "/health/band"
        }
        "state-of-play" => field_path.starts_with("/state/"),
        "the-room" => field_path.starts_with("/stakeholders/"),
        "whats-next" => field_path.starts_with("/work/next_steps/"),
        "watch-list" => field_path.starts_with("/watch_list/"),
        "value-commitments" => {
            field_path.starts_with("/value/")
                || field_path.starts_with("/work/commitments/")
                || field_path == "/vitals/arr"
        }
        "strategic-landscape" => {
            field_path.starts_with("/strategy/")
                || field_path.starts_with("/technical/")
                || field_path.starts_with("/company/")
        }
        "the-record" => field_path.starts_with("/record/") || field_path.starts_with("/sources/"),
        "the-work" => field_path.starts_with("/work/"),
        "reports" => field_path.starts_with("/reports/"),
        _ => false,
    }
}

fn snapshot_source_indexes(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    fields: &[&AccountCompositionSnapshotField],
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
    field: &AccountCompositionSnapshotField,
) -> Result<Option<SourceAttribution>, AbilityError> {
    if matches!(
        field.provenance_kind,
        AccountCompositionProvenanceKind::NonSensitiveIdentity
            | AccountCompositionProvenanceKind::Unavailable
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
            entity_id: EntityId::new(input.account_id.clone()),
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

fn data_source_for_snapshot_field(field: &AccountCompositionSnapshotField) -> DataSource {
    match field.provenance_kind {
        AccountCompositionProvenanceKind::ManualUser => DataSource::User,
        AccountCompositionProvenanceKind::SystemConfig => DataSource::LocalEnrichment,
        _ => field
            .source_label
            .as_deref()
            .map(data_source_for_claim)
            .unwrap_or_else(|| DataSource::Other(SourceName::new("account_snapshot"))),
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

/// Format an integer with thousands separators (e.g. `185400` -> `185,400`).
fn format_thousands(value: i64) -> String {
    let negative = value < 0;
    let digits = value.unsigned_abs().to_string();
    let bytes = digits.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + len / 3);
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 && (len - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*byte as char);
    }
    if negative {
        format!("-{out}")
    } else {
        out
    }
}

/// Produce the display-ready string + a `kind` hint for a headline vital
/// (WR-R2). Formatting lives in the producer so every surface — Tauri,
/// WordPress, MCP — renders identically; the typed `value` is preserved
/// separately so agents can still compute on it. Time-relative rendering
/// (countdowns) is deliberately NOT done here — the composition is cached and
/// would freeze a relative value; a surface derives it from the typed `value`
/// using this `kind` hint.
fn vital_display(field: &AccountCompositionSnapshotField) -> (String, &'static str) {
    let key = field.field_path.rsplit('/').next().unwrap_or("");
    match key {
        "arr" => match field.value.as_f64() {
            Some(amount) => (format!("${}", format_thousands(amount.round() as i64)), "currency"),
            None => (snapshot_value_text(&field.value), "currency"),
        },
        "contract_end" => {
            let raw = snapshot_value_text(&field.value);
            let display = NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
                .map(|date| date.format("%b %-d, %Y").to_string())
                .unwrap_or(raw);
            (display, "date")
        }
        "nps" => (snapshot_value_text(&field.value), "number"),
        _ => (snapshot_value_text(&field.value), "text"),
    }
}

/// Human-facing source label for a vital, via `DataSource::display_name()`
/// (ADR-0108) rather than the raw source key. Normalizes the WR-R1
/// `workspace_file:backfilled` transitional label (which classifies as
/// `Other` and would otherwise render the raw key) to a clean "Workspace
/// file".
fn vital_source_label(raw: Option<&str>) -> Option<String> {
    let display = data_source_for_claim(raw?).display_name();
    if display.starts_with("workspace_file") {
        Some("Workspace file".to_string())
    } else {
        Some(display)
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
            "dailyos.account_overview.v1",
            source_indexes
                .into_iter()
                .map(|source_index| SourceRef::Source { source_index })
                .collect(),
            Confidence::computed(1.0).map_err(field_error)?,
        )
        .map_err(field_error)?
    };
    builder
        .attribute(path.clone(), attribution.clone())
        .map_err(provenance_error)?;
    Ok(())
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
    let mut bindings = Vec::with_capacity(item_count * 7);
    for index in 0..item_count {
        for field in [
            "label",
            "value",
            "display_value",
            "kind",
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
    let mut bindings = Vec::with_capacity(item_count * 3);
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
        ClaimPlacement::Health => 1,
        ClaimPlacement::Commitment => 2,
        ClaimPlacement::Value => 3,
        ClaimPlacement::Win => 4,
        ClaimPlacement::Relationship => 5,
        ClaimPlacement::Overview => 6,
        ClaimPlacement::Ignored => 7,
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
    account_id: &str,
    claim: &IntelligenceClaim,
) -> Result<SourceAttribution, AbilityError> {
    let now = ctx.services().clock.now();
    let observed_at = parse_observed_at(claim, now);
    let source_asof = parse_claim_source_asof(claim, now);
    let mut identifiers = vec![SourceIdentifier::Entity {
        entity_id: EntityId::new(account_id.to_string()),
        field: Some(
            claim
                .field_path
                .clone()
                .unwrap_or_else(|| claim.claim_type.clone()),
        ),
    }];
    if let Some(file_id) = claim
        .source_ref
        .as_deref()
        .and_then(workspace_file_id_from_source_ref)
    {
        identifiers.push(SourceIdentifier::Document {
            document_id: DocumentId::new(file_id.to_string()),
            chunk_id: None,
        });
    }
    SourceAttribution::new(
        data_source_for_claim(&claim.data_source),
        identifiers,
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
        "glean" => DataSource::Glean {
            downstream: GleanDownstream::Documents,
        },
        _ => data_source_from_key(value),
    }
}

/// Provenance class for trust rendering: a claim resting on a hard source
/// (has a `source_ref` — workspace docs, CRM, etc.) is `"sourced"`; one with
/// no source_ref (enrichment inference) is `"inferred"`. Surfaces fade
/// `inferred` content; `sourced` reads at full presence. This is the render
/// signal — distinct from the trust score, whose cold-start calibration is
/// handled by scoring policy so render presence stays source-based.
fn claim_provenance_kind(claim: &IntelligenceClaim) -> &'static str {
    match claim.source_ref.as_deref() {
        Some(reference) if !reference.trim().is_empty() => "sourced",
        _ => "inferred",
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
    use crate::abilities::provenance::ProvenanceWarning;
    use crate::abilities::registry::{AbilityRegistry, ActorKind, McpExposure, ScopeSet};
    use crate::abilities::{
        project_composition_for_surface, Actor, FallbackProjectionContext, SurfaceKind,
        NOOP_ABILITY_TRACER,
    };
    use crate::intelligence::provider::{
        Completion, FingerprintMetadata, IntelligenceProvider, ModelName, ModelTier, PromptInput,
        ProviderError, ProviderKind,
    };
    use crate::sensitivity::{ClaimDismissalSurface, ClaimVerificationState};
    use crate::services::context::{
        AccountCompositionSnapshotReadFuture, AccountCompositionSnapshotReadHandle,
        AccountCompositionSnapshotSensitivity, CompositionCommitFuture, CompositionCommitHandle,
        CompositionCommitRequest, EntityContextClaimReadFuture, EntityContextClaimReadHandle,
        ExternalClients, FixedClock, SeedableRng, ServiceContext,
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
                *self.last_surface.lock().expect("surface lock") = Some(surface);
                assert_eq!(entity_type, "account");
                assert_eq!(entity_id, "acct-fixture-1");
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
        result: Mutex<Result<AccountCompositionSnapshot, AccountCompositionSnapshotReadError>>,
        calls: AtomicUsize,
        last_surface: Mutex<Option<ClaimDismissalSurface>>,
    }

    impl SpySnapshotReader {
        fn new(
            result: Result<AccountCompositionSnapshot, AccountCompositionSnapshotReadError>,
        ) -> Self {
            Self {
                result: Mutex::new(result),
                calls: AtomicUsize::new(0),
                last_surface: Mutex::new(None),
            }
        }
    }

    impl AccountCompositionSnapshotReadHandle for SpySnapshotReader {
        fn read_account_composition_snapshot<'a>(
            &'a self,
            account_id: String,
            surface: ClaimDismissalSurface,
        ) -> AccountCompositionSnapshotReadFuture<'a> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                *self.last_surface.lock().expect("snapshot surface lock") = Some(surface);
                assert_eq!(account_id, "acct-fixture-1");
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
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, external)
            .with_actor("surface_client")
            .with_ability_id(ABILITY_NAME)
            .with_entity_context_claim_reader(reader)
            .with_composition_commit_handle(committer)
    }

    fn services_with_snapshot<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        external: &'a ExternalClients,
        reader: Arc<SpyClaimReader>,
        committer: Arc<RecordingCommitter>,
        snapshot_reader: Arc<SpySnapshotReader>,
    ) -> ServiceContext<'a> {
        services(clock, rng, external, reader, committer)
            .with_account_composition_snapshot_reader(snapshot_reader)
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
                instance: crate::abilities::registry::SurfaceClientId::new("sc_fixture"),
                scopes: ScopeSet::new([crate::abilities::registry::SurfaceScope::new(
                    "read.account_overview",
                )])
                .expect("scope set"),
            },
            None,
            ClaimDismissalSurface::LogStructured,
        )
    }

    fn input() -> AccountOverviewInput {
        AccountOverviewInput {
            schema_version: ABILITY_SCHEMA_VERSION,
            account_id: "acct-fixture-1".to_string(),
            entity_type: None,
            entity_id: None,
            expected_composition_version: 0,
            composition_id: Some("acct-overview-fixture".to_string()),
        }
    }

    fn claim(
        id: &str,
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
            subject_ref: json!({"kind": "account", "id": "acct-fixture-1"}).to_string(),
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

    fn snapshot_field(
        field_path: &str,
        label: &str,
        value: Value,
        sensitivity: AccountCompositionSnapshotSensitivity,
        source_label: Option<&str>,
        source_asof: Option<&str>,
    ) -> AccountCompositionSnapshotField {
        AccountCompositionSnapshotField {
            field_path: field_path.to_string(),
            label: label.to_string(),
            value,
            sensitivity,
            source_label: source_label.map(ToString::to_string),
            source_ref: source_label.map(|source| format!("{source}:fixture")),
            source_asof: source_asof.map(ToString::to_string),
            trust_band: TrustBand::UseWithCaution,
            trust_status: "use_with_caution".to_string(),
            provenance_kind: AccountCompositionProvenanceKind::SourceField,
        }
    }

    fn identity_snapshot_field(
        field_path: &str,
        label: &str,
        value: &str,
    ) -> AccountCompositionSnapshotField {
        AccountCompositionSnapshotField {
            field_path: field_path.to_string(),
            label: label.to_string(),
            value: Value::String(value.to_string()),
            sensitivity: AccountCompositionSnapshotSensitivity::NonSensitiveIdentity,
            source_label: None,
            source_ref: None,
            source_asof: None,
            trust_band: TrustBand::LikelyCurrent,
            trust_status: "likely_current".to_string(),
            provenance_kind: AccountCompositionProvenanceKind::NonSensitiveIdentity,
        }
    }

    #[test]
    fn vital_display_formats_by_kind_and_preserves_typed_value() {
        let arr = snapshot_field(
            "/vitals/arr",
            "ARR",
            serde_json::json!(185_400.0),
            AccountCompositionSnapshotSensitivity::Internal,
            Some("workspace_file:entity_doc"),
            None,
        );
        assert_eq!(vital_display(&arr), ("$185,400".to_string(), "currency"));
        assert!(arr.value.is_number(), "typed value preserved for compute/MCP");

        let date = snapshot_field(
            "/vitals/contract_end",
            "Contract end",
            serde_json::json!("2026-11-24"),
            AccountCompositionSnapshotSensitivity::Internal,
            None,
            None,
        );
        assert_eq!(vital_display(&date), ("Nov 24, 2026".to_string(), "date"));

        let nps = snapshot_field(
            "/vitals/nps",
            "NPS",
            serde_json::json!(8),
            AccountCompositionSnapshotSensitivity::Internal,
            None,
            None,
        );
        assert_eq!(vital_display(&nps), ("8".to_string(), "number"));

        let lifecycle = snapshot_field(
            "/vitals/lifecycle",
            "Lifecycle",
            serde_json::json!("nurture"),
            AccountCompositionSnapshotSensitivity::Internal,
            None,
            None,
        );
        assert_eq!(vital_display(&lifecycle), ("nurture".to_string(), "text"));
    }

    #[test]
    fn format_thousands_groups_digits() {
        assert_eq!(format_thousands(8), "8");
        assert_eq!(format_thousands(185_400), "185,400");
        assert_eq!(format_thousands(1_234_567), "1,234,567");
        assert_eq!(format_thousands(-2_000), "-2,000");
    }

    #[test]
    fn vital_source_label_uses_display_name_and_normalizes_backfill() {
        assert_eq!(
            vital_source_label(Some("workspace_file:entity_doc")).as_deref(),
            Some("Workspace file (entity document)")
        );
        assert_eq!(
            vital_source_label(Some("workspace_file:backfilled")).as_deref(),
            Some("Workspace file")
        );
        assert_eq!(vital_source_label(None), None);
    }

    fn snapshot_fixture(
        fields: Vec<AccountCompositionSnapshotField>,
    ) -> AccountCompositionSnapshot {
        AccountCompositionSnapshot {
            account_id: "acct-fixture-1".to_string(),
            display_name: identity_snapshot_field(
                "/identity/display_name",
                "Account",
                "Example Account",
            ),
            account_type: Some(identity_snapshot_field(
                "/identity/account_type",
                "Account type",
                "customer",
            )),
            fields,
            intelligence: None,
            glean_signals: None,
            sentiment: None,
            stakeholders: None,
            technical_footprint: None,
            commercial: None,
            fabric: None,
            record: None,
        }
    }

    fn output_json(output: &crate::abilities::provenance::AbilityOutput<Composition>) -> Value {
        serde_json::to_value(output).expect("output serializes")
    }

    #[test]
    fn registry_declaration_pins_policy() {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let descriptor = registry
            .iter_all()
            .find(|descriptor| descriptor.name == ABILITY_NAME)
            .expect("account overview ability registered");

        assert_eq!(descriptor.name, ABILITY_NAME);
        assert_eq!(descriptor.category, AbilityCategory::Read);
        assert_eq!(
            descriptor.policy.allowed_actors,
            &[ActorKind::User, ActorKind::SurfaceClient]
        );
        assert_eq!(
            descriptor.policy.required_scopes,
            &["read.account_overview"]
        );
        assert_eq!(descriptor.policy.mcp_exposure, McpExposure::Invocable);
        assert!(!descriptor.policy.client_side_executable);
        assert!(descriptor.mutates.is_empty());
        assert!(descriptor.composes.is_empty());
    }

    #[test]
    fn entity_envelope_accepts_matching_account_and_rejects_mismatch() {
        let ok = AccountOverviewInput {
            entity_type: Some("account".to_string()),
            entity_id: Some("acct-fixture-1".to_string()),
            ..input()
        };
        normalize_input(ok).expect("matching envelope is accepted");

        let wrong_type = AccountOverviewInput {
            entity_type: Some("project".to_string()),
            entity_id: Some("acct-fixture-1".to_string()),
            ..input()
        };
        assert_eq!(
            normalize_input(wrong_type)
                .expect_err("type mismatch rejects")
                .kind,
            AbilityErrorKind::Validation
        );

        let wrong_id = AccountOverviewInput {
            entity_type: Some("account".to_string()),
            entity_id: Some("other-account".to_string()),
            ..input()
        };
        assert_eq!(
            normalize_input(wrong_id)
                .expect_err("id mismatch rejects")
                .kind,
            AbilityErrorKind::Validation
        );
    }

    #[tokio::test]
    async fn missing_account_id_rejects_before_claim_read_or_commit() {
        let (clock, rng, external, reader, committer, provider) = fixture_parts(Vec::new());
        let services = services(&clock, &rng, &external, reader.clone(), committer.clone());
        let ctx = ability_ctx(&services, &provider);

        let err = match account_overview(
            &ctx,
            AccountOverviewInput {
                account_id: " ".to_string(),
                ..input()
            },
        )
        .await
        {
            Ok(_) => panic!("missing account id rejects"),
            Err(error) => error,
        };

        assert_eq!(err.kind, AbilityErrorKind::Validation);
        assert_eq!(reader.calls.load(Ordering::SeqCst), 0);
        assert_eq!(committer.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn missing_account_snapshot_rejects_before_composition_commit() {
        let snapshot_reader = Arc::new(SpySnapshotReader::new(Err(
            AccountCompositionSnapshotReadError::AccountNotFound("acct-fixture-1".to_string()),
        )));
        let (clock, rng, external, reader, committer, provider) = fixture_parts(Vec::new());
        let services = services_with_snapshot(
            &clock,
            &rng,
            &external,
            reader.clone(),
            committer.clone(),
            snapshot_reader.clone(),
        );
        let ctx = ability_ctx(&services, &provider);

        let err = match account_overview(&ctx, input()).await {
            Ok(_) => panic!("missing account rejects"),
            Err(error) => error,
        };

        assert_eq!(err.kind, AbilityErrorKind::Validation);
        assert!(err.message.contains("acct-fixture-1"));
        assert_eq!(reader.calls.load(Ordering::SeqCst), 1);
        assert_eq!(snapshot_reader.calls.load(Ordering::SeqCst), 1);
        assert_eq!(committer.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn committed_output_has_field_bindings_provenance_and_degraded_trust() {
        let mut hidden = claim(
            "claim-hidden",
            "entity_risk",
            "/risk/hidden",
            "Hidden risk",
            Some(0.99),
            Some("2026-05-15T09:00:00Z"),
            ClaimSensitivity::Confidential,
        );
        hidden.surfacing_state = SurfacingState::Active;
        let claims = vec![
            claim(
                "claim-risk",
                "entity_risk",
                "/risk/current",
                "Implementation risk is rising",
                Some(0.97),
                None,
                ClaimSensitivity::Internal,
            ),
            claim(
                "claim-win",
                "entity_win",
                "/wins/latest",
                "Renewal path is clearer",
                Some(0.92),
                Some("2026-05-14T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            claim(
                "claim-value",
                "value_delivered",
                "/value/latest",
                "Team shipped adoption milestone",
                None,
                Some("2026-05-14T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            claim(
                "claim-commitment",
                "commitment",
                "/commitments/next",
                "Follow up on launch checklist",
                Some(0.85),
                Some("2026-05-01T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            claim(
                "claim-context",
                "company_context",
                "/company/industry",
                "Fixture Account operates in software",
                Some(0.96),
                Some("2026-03-01T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            hidden,
        ];
        let (clock, rng, external, reader, committer, provider) = fixture_parts(claims);
        let services = services(&clock, &rng, &external, reader.clone(), committer.clone());
        let ctx = ability_ctx(&services, &provider);

        let output = account_overview(&ctx, input())
            .await
            .expect("account overview succeeds");
        let composition = output.data();

        assert_eq!(reader.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            *reader.last_surface.lock().expect("surface lock"),
            Some(ClaimDismissalSurface::LogStructured)
        );
        assert_eq!(committer.calls.load(Ordering::SeqCst), 1);
        assert_eq!(composition.metadata.composition_version.0, 1);
        assert_eq!(composition.generated_by.as_str(), ABILITY_NAME);
        assert_eq!(composition.metadata.generated_by, ABILITY_NAME);

        let serialized = output_json(&output);
        assert!(serialized.to_string().contains("claim-risk"));
        assert!(!serialized.to_string().contains("claim-hidden"));
        assert!(serialized.to_string().contains("needs_verification"));
        assert!(output.provenance().warnings.iter().any(|warning| {
            matches!(warning, ProvenanceWarning::SourceTimestampUnknown { .. })
        }));

        let blocks = composition.blocks().collect::<Vec<_>>();
        assert!(blocks
            .iter()
            .any(|block| block.block_type == BlockType::RiskCallout));
        assert!(blocks
            .iter()
            .any(|block| block.block_type == BlockType::ActionList));
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
    async fn high_cardinality_overview_claims_keep_provenance_under_hard_budget() {
        let claims = (0..160)
            .map(|index| {
                claim(
                    &format!("claim-context-{index:03}"),
                    "company_context",
                    &format!("/company/context/{index}"),
                    &format!(
                        "Context signal {index} remains relevant for the account composition proof."
                    ),
                    Some(0.94),
                    Some("2026-05-14T09:00:00Z"),
                    ClaimSensitivity::Internal,
                )
            })
            .collect::<Vec<_>>();
        let (clock, rng, external, reader, committer, provider) = fixture_parts(claims);
        let services = services(&clock, &rng, &external, reader, committer);
        let ctx = ability_ctx(&services, &provider);

        let output = account_overview(&ctx, input())
            .await
            .expect("large overview composition stays within provenance budget");
        let provenance_bytes = serde_json::to_vec(output.provenance())
            .expect("provenance serializes")
            .len();

        assert!(
            provenance_bytes < crate::abilities::provenance::builder::HARD_PROVENANCE_BUDGET_BYTES,
            "provenance envelope should stay under hard budget, got {provenance_bytes}"
        );
        for block in output.data().blocks() {
            block
                .validate_against(output.provenance())
                .expect("block provenance resolves");
        }
    }

    #[tokio::test]
    async fn empty_claim_set_returns_display_only_empty_state() {
        let (clock, rng, external, reader, committer, provider) = fixture_parts(Vec::new());
        let services = services(&clock, &rng, &external, reader, committer);
        let ctx = ability_ctx(&services, &provider);

        let output = account_overview(&ctx, input())
            .await
            .expect("empty state succeeds");
        let composition = output.data();
        let section_ids = composition
            .sections
            .iter()
            .map(|section| section.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            section_ids,
            VARIANT_D_SECTIONS
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<_>>()
        );
        assert!(!section_ids.contains(&"empty"));
        assert!(!section_ids.contains(&"signals"));

        let degraded_blocks = composition
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
            degraded_blocks.len() >= 9,
            "most non-headline sections render degraded empty blocks"
        );
        for block in degraded_blocks {
            assert!(block.claim_refs.is_empty());
            assert!(block.field_bindings.iter().all(|binding| {
                binding.role == BindingRole::DisplayOnly && binding.claim_refs.is_empty()
            }));
        }
    }

    #[tokio::test]
    async fn snapshot_fields_are_wrapped_sourced_and_sensitivity_filtered() {
        let snapshot_reader = Arc::new(SpySnapshotReader::new(Ok(snapshot_fixture(vec![
            snapshot_field(
                "/vitals/lifecycle",
                "Lifecycle",
                Value::String("active".to_string()),
                AccountCompositionSnapshotSensitivity::Internal,
                Some("user"),
                Some("2026-05-14T09:00:00Z"),
            ),
            snapshot_field(
                "/vitals/arr",
                "ARR",
                Value::String("sensitive-commercial-value".to_string()),
                AccountCompositionSnapshotSensitivity::Confidential,
                Some("salesforce"),
                Some("2026-05-14T09:00:00Z"),
            ),
            snapshot_field(
                "/reports/account_report",
                "Account report",
                Value::String("unavailable".to_string()),
                AccountCompositionSnapshotSensitivity::Internal,
                Some("system_config"),
                Some("2026-05-15T10:00:00Z"),
            ),
        ]))));
        let (clock, rng, external, reader, committer, provider) = fixture_parts(Vec::new());
        let services = services_with_snapshot(
            &clock,
            &rng,
            &external,
            reader,
            committer,
            snapshot_reader.clone(),
        );
        let ctx = ability_ctx(&services, &provider);

        let output = account_overview(&ctx, input())
            .await
            .expect("snapshot-backed account overview succeeds");
        let serialized = output_json(&output);

        assert_eq!(snapshot_reader.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            *snapshot_reader
                .last_surface
                .lock()
                .expect("snapshot surface lock"),
            Some(ClaimDismissalSurface::LogStructured)
        );
        assert!(serialized.to_string().contains("Example Account"));
        let headline_vitals = output.data().sections[0].blocks[0]
            .attributes
            .pointer("/vitals")
            .and_then(Value::as_array)
            .expect("headline vitals array");
        assert!(headline_vitals.iter().any(|value| {
            value.pointer("/label").and_then(Value::as_str) == Some("Lifecycle")
                && value.pointer("/value").and_then(Value::as_str) == Some("active")
        }));
        assert!(!serialized
            .to_string()
            .contains("sensitive-commercial-value"));
        assert!(output.provenance().sources.iter().any(|source| {
            source.identifiers.iter().any(|identifier| {
                matches!(
                    identifier,
                    SourceIdentifier::Entity { field: Some(field), .. }
                        if field == "/vitals/lifecycle"
                )
            })
        }));
    }

    #[tokio::test]
    async fn mixed_claim_and_snapshot_sections_keep_both_evidence_paths() {
        let snapshot_reader = Arc::new(SpySnapshotReader::new(Ok(snapshot_fixture(vec![
            snapshot_field(
                "/value/growth_potential",
                "Growth potential",
                Value::String("expansion motion active".to_string()),
                AccountCompositionSnapshotSensitivity::Internal,
                Some("salesforce"),
                Some("2026-05-14T09:00:00Z"),
            ),
        ]))));
        let claims = vec![claim(
            "claim-value",
            "value_delivered",
            "/value/latest",
            "Adoption milestone shipped",
            Some(0.92),
            Some("2026-05-14T09:00:00Z"),
            ClaimSensitivity::Internal,
        )];
        let (clock, rng, external, reader, committer, provider) = fixture_parts(claims);
        let services =
            services_with_snapshot(&clock, &rng, &external, reader, committer, snapshot_reader);
        let ctx = ability_ctx(&services, &provider);

        let output = account_overview(&ctx, input())
            .await
            .expect("mixed claim and snapshot account overview succeeds");
        let composition = output.data();
        let value_section = composition
            .sections
            .iter()
            .find(|section| section.id.as_str() == "value-commitments")
            .expect("value commitments section");

        assert!(
            value_section
                .blocks
                .iter()
                .any(|block| block.block_type == BlockType::ClaimSummary
                    && block
                        .attributes
                        .pointer("/items/0/text")
                        .and_then(Value::as_str)
                        == Some("Adoption milestone shipped")),
            "claim evidence remains renderable as an aggregate item"
        );
        // Snapshot-field evidence dumps were removed from sections by design
        // (production parity): raw vitals never render as section residue.
        assert!(
            !value_section
                .blocks
                .iter()
                .any(|block| block.attributes.to_string().contains("Growth potential")),
            "snapshot fields must not leak into section blocks"
        );
    }

    #[tokio::test]
    async fn pure_builder_output_is_deterministic_without_commit() {
        let claims = vec![
            claim(
                "claim-b",
                "entity_win",
                "/wins/latest",
                "Stable win",
                Some(0.91),
                Some("2026-05-14T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            claim(
                "claim-a",
                "entity_risk",
                "/risk/current",
                "Stable risk",
                Some(0.91),
                Some("2026-05-14T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
        ];
        let first = prepared_json(claims.clone()).await;
        let second = prepared_json(claims).await;

        assert_eq!(first, second);
        assert!(!first.to_string().contains("Acme"));
    }

    async fn prepared_json(claims: Vec<IntelligenceClaim>) -> Value {
        let (clock, rng, external, reader, committer, provider) = fixture_parts(claims);
        let services = services(&clock, &rng, &external, reader, committer);
        let ctx = ability_ctx(&services, &provider);
        let normalized = normalize_input(input()).expect("input normalizes");
        let prepared = prepare_account_overview(&ctx, &normalized)
            .await
            .expect("proposal builds");
        let output = prepared
            .provenance_builder
            .finalize(prepared.proposal.composition)
            .expect("provenance finalizes");
        output_json(&output)
    }

    // Asserts producer output passes fallback_projection's binding validator
    // without BindingTargetsUnknownField. Covers the producer→projection
    // contract that wasn't exercised by either side's isolated test suite.
    #[tokio::test]
    async fn dos670_producer_output_passes_w4d_projection() {
        let claims = vec![
            claim(
                "claim-risk",
                "entity_risk",
                "/risk/current",
                "Implementation risk is rising",
                Some(0.97),
                Some("2026-05-14T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            claim(
                "claim-win",
                "entity_win",
                "/wins/latest",
                "Renewal path is clearer",
                Some(0.92),
                Some("2026-05-14T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            claim(
                "claim-commitment",
                "commitment",
                "/commitments/next",
                "Follow up on launch checklist",
                Some(0.85),
                Some("2026-05-01T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            claim(
                "claim-context",
                "company_context",
                "/company/industry",
                "Fixture Account operates in software",
                Some(0.96),
                Some("2026-03-01T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            // Exercise the Health (HealthSnapshot) and Relationship
            // (RelationshipMap) placements so the provenance_kind bindings on
            // every claim-block rule are validated by this parity gate.
            claim(
                "claim-health",
                "entity_current_state",
                "/health/current",
                "Renewal posture is steady",
                Some(0.9),
                Some("2026-05-14T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
            claim(
                "claim-room",
                "stakeholder_role",
                "/relationships/champion",
                "Primary champion owns the rollout",
                Some(0.88),
                Some("2026-05-14T09:00:00Z"),
                ClaimSensitivity::Internal,
            ),
        ];
        // Headline vitals must flow through projection: the producer emits
        // display-only bindings for every vital field (including R2's
        // `display_value`/`kind`), so the parity gate has to exercise an
        // account that actually has vitals. A vitals-free fixture silently
        // skips the `/vitals/*/...` bindings and lets a binding↔rule desync
        // ship undetected (the BindingTargetsUnknownField producer_unavailable
        // regression).
        let snapshot_reader = Arc::new(SpySnapshotReader::new(Ok(snapshot_fixture(vec![
            snapshot_field(
                "/vitals/arr",
                "ARR",
                json!(185_400),
                AccountCompositionSnapshotSensitivity::Internal,
                Some("Workspace file (entity document)"),
                Some("2026-05-14T09:00:00Z"),
            ),
            snapshot_field(
                "/vitals/contract_end",
                "Contract end",
                Value::String("2026-11-24".to_string()),
                AccountCompositionSnapshotSensitivity::Internal,
                Some("Workspace file (entity document)"),
                Some("2026-05-14T09:00:00Z"),
            ),
        ]))));
        let (clock, rng, external, reader, committer, provider) = fixture_parts(claims);
        let services =
            services_with_snapshot(&clock, &rng, &external, reader, committer, snapshot_reader);
        let ctx = ability_ctx(&services, &provider);

        let output = account_overview(&ctx, input())
            .await
            .expect("account overview succeeds");
        let composition = output.data();

        let proj_ctx = FallbackProjectionContext::new(
            Actor::SurfaceClient {
                instance: crate::abilities::registry::SurfaceClientId::new("sc_fixture"),
                scopes: ScopeSet::new([crate::abilities::registry::SurfaceScope::new(
                    "read.account_overview",
                )])
                .expect("scope set"),
            },
            SurfaceKind::SurfaceClient,
            3,
        );

        let (projected, _audits) = project_composition_for_surface(composition, &proj_ctx)
            .expect("projection must accept producer output (DOS-670 contract)");

        assert!(
            !projected.blocks.is_empty(),
            "projected composition must contain at least one block"
        );

        let claim_text_rendered = projected.blocks.iter().any(|block| {
            block.payload.pointer("/text").is_some()
                || block.payload.pointer("/items/0/text").is_some()
                || block.payload.pointer("/nodes/0/text").is_some()
        });
        assert!(
            claim_text_rendered,
            "projected payload must surface claim text from producer attributes"
        );

        let trust_band_rendered = projected.blocks.iter().any(|block| {
            block.payload.pointer("/trust_band").is_some()
                || block.payload.pointer("/items/0/trust_band").is_some()
                || block.payload.pointer("/nodes/0/trust_band").is_some()
        });
        assert!(
            trust_band_rendered,
            "projected payload must surface trust_band from producer attributes"
        );

        // Guard the coverage that made this gate meaningful: the headline
        // vitals (with R2's producer-formatted display_value) must survive
        // projection. If this drops to zero the parity check above is vacuous.
        let vital_display_rendered = projected.blocks.iter().any(|block| {
            block.payload.pointer("/vitals/0/display_value").is_some()
        });
        assert!(
            vital_display_rendered,
            "projected payload must surface headline vital display_value from producer attributes"
        );
    }
}

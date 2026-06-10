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
    prompt_input_sensitivity_allowed, subject_ref_from_json, ClaimState, ClaimSubjectRef,
    ClaimSensitivity, IntelligenceClaim, SurfacingState,
};

const ABILITY_NAME: &str = "dailyos/account-overview";
const ABILITY_SCHEMA_VERSION: u32 = 1;
const ACCOUNT_CLAIM_DEPTH: usize = 3;
const MAX_TRANSCRIPT_QUOTE_ITEMS: usize = 5;
const TRANSCRIPT_QUOTE_REDACTION_POLICY: &str = "sensitivity_ceiling";

const VARIANT_D_SECTIONS: [(&str, &str); 11] = [
    ("headline", "Headline"),
    ("outlook", "Outlook"),
    ("state-of-play", "State of play"),
    ("the-room", "The room"),
    ("whats-next", "What's next"),
    ("watch-list", "Watch list"),
    ("value-commitments", "Value commitments"),
    ("strategic-landscape", "Strategic landscape"),
    ("the-record", "The record"),
    ("the-work", "The work"),
    ("reports", "Reports"),
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

struct TranscriptQuoteEvidence<'a> {
    projection: &'a ClaimProjection,
    quote: TranscriptVerifiedQuote,
}

struct TranscriptVerifiedQuote {
    text: String,
    start_char: u64,
    end_char: u64,
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

    let outlook_claims = projections
        .iter()
        .filter(|projection| projection.placement == ClaimPlacement::Health)
        .collect::<Vec<_>>();
    sections.push(variant_section(
        "outlook",
        "Outlook",
        build_claim_or_snapshot_section_blocks(
            ctx,
            input,
            "outlook",
            1,
            outlook_claims,
            snapshot_fields_for_section(snapshot, "outlook"),
            EmptySectionCopy {
                title: "No current outlook signals",
                body: "DailyOS has not found current account-health signals with renderable provenance.",
                status: "source_gap",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.86, SalienceBand::Important, "account outlook"),
    ));

    let state_claims = projections
        .iter()
        .filter(|projection| {
            matches!(
                projection.placement,
                ClaimPlacement::Health
                    | ClaimPlacement::Risk
                    | ClaimPlacement::Win
                    | ClaimPlacement::Value
                    | ClaimPlacement::Overview
            )
        })
        .collect::<Vec<_>>();
    sections.push(variant_section(
        "state-of-play",
        "State of play",
        build_claim_or_snapshot_section_blocks(
            ctx,
            input,
            "state-of-play",
            2,
            state_claims,
            snapshot_fields_for_section(snapshot, "state-of-play"),
            EmptySectionCopy {
                title: "No active state-of-play signals",
                body: "No active account claims are currently eligible for this surface.",
                status: "empty",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.82, SalienceBand::Important, "current account state"),
    ));

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
            3,
            room_claims,
            snapshot_fields_for_section(snapshot, "the-room"),
            EmptySectionCopy {
                title: "No room signals",
                body: "Stakeholder and relationship inputs are not yet grounded for this account.",
                status: "empty",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Grid,
        salience(0.72, SalienceBand::Contextual, "account relationships"),
    ));

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
            4,
            next_claims,
            snapshot_fields_for_section(snapshot, "whats-next"),
            EmptySectionCopy {
                title: "No open next steps",
                body: "There are no renderable commitments or next-step records for this account.",
                status: "empty",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.8, SalienceBand::Important, "next account work"),
    ));

    let watch_claims = projections
        .iter()
        .filter(|projection| projection.placement == ClaimPlacement::Risk)
        .collect::<Vec<_>>();
    sections.push(variant_section(
        "watch-list",
        "Watch list",
        build_claim_or_snapshot_section_blocks(
            ctx,
            input,
            "watch-list",
            5,
            watch_claims,
            snapshot_fields_for_section(snapshot, "watch-list"),
            EmptySectionCopy {
                title: "No active watch-list signals",
                body: "No active risk or watch-list claims are eligible for this account.",
                status: "empty",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.76, SalienceBand::Important, "watch-list signals"),
    ));

    let value_claims = projections
        .iter()
        .filter(|projection| {
            matches!(
                projection.placement,
                ClaimPlacement::Value | ClaimPlacement::Win | ClaimPlacement::Commitment
            )
        })
        .collect::<Vec<_>>();
    sections.push(variant_section(
        "value-commitments",
        "Value commitments",
        build_claim_or_snapshot_section_blocks(
            ctx,
            input,
            "value-commitments",
            6,
            value_claims,
            snapshot_fields_for_section(snapshot, "value-commitments"),
            EmptySectionCopy {
                title: "No commitments with current evidence",
                body: "DailyOS has not found value or commitment claims with current evidence.",
                status: "source_gap",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.72, SalienceBand::Important, "value and commitments"),
    ));

    let strategic_claims = projections
        .iter()
        .filter(|projection| {
            matches!(
                projection.claim_type,
                ClaimType::CompanyContext
                    | ClaimType::AccountFact
                    | ClaimType::EntityIdentity
                    | ClaimType::EntitySummary
                    | ClaimType::UserNote
            )
        })
        .collect::<Vec<_>>();
    sections.push(variant_section(
        "strategic-landscape",
        "Strategic landscape",
        build_claim_or_snapshot_section_blocks(
            ctx,
            input,
            "strategic-landscape",
            7,
            strategic_claims,
            snapshot_fields_for_section(snapshot, "strategic-landscape"),
            EmptySectionCopy {
                title: "Strategic context not yet grounded",
                body: "No source-backed strategic context is available for this account.",
                status: "needs_grounding",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.62, SalienceBand::Contextual, "strategic account context"),
    ));

    sections.push(variant_section(
        "the-record",
        "The record",
        build_record_section_blocks(
            ctx,
            input,
            projections,
            snapshot_fields_for_section(snapshot, "the-record"),
            8,
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.58, SalienceBand::Contextual, "account evidence record"),
    ));

    let work_claims = projections
        .iter()
        .filter(|projection| projection.placement == ClaimPlacement::Commitment)
        .collect::<Vec<_>>();
    sections.push(variant_section(
        "the-work",
        "The work",
        build_claim_or_snapshot_section_blocks(
            ctx,
            input,
            "the-work",
            9,
            work_claims,
            snapshot_fields_for_section(snapshot, "the-work"),
            EmptySectionCopy {
                title: "No active work items",
                body: "No active work records are currently grounded for this account.",
                status: "empty",
            },
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Stacked,
        salience(0.46, SalienceBand::Background, "account work"),
    ));

    sections.push(variant_section(
        "reports",
        "Reports",
        build_reports_section_blocks(
            ctx,
            input,
            snapshot_fields_for_section(snapshot, "reports"),
            10,
            subject,
            invocation_id,
            provenance_builder,
        )?,
        SectionLayout::Grid,
        salience(0.38, SalienceBand::Background, "account reports"),
    ));

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
    snapshot_fields: Vec<&AccountCompositionSnapshotField>,
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
                title: "No account record yet",
                body: "No source-backed account events are available for this record.",
                status: "empty",
            },
            section_index,
            0,
            subject,
            invocation_id,
            provenance_builder,
        )?]);
    }

    let mut blocks = Vec::new();
    let quote_items = transcript_quote_evidence_items(projections);
    if !quote_items.is_empty() {
        let block_index = blocks.len();
        blocks.push(build_transcript_quote_wall_block(
            input,
            quote_items,
            section_index,
            block_index,
            subject,
            invocation_id,
            provenance_builder,
        )?);
    }

    let mut claim_refs = Vec::new();
    let mut source_indexes = Vec::new();
    let mut items = Vec::new();
    for projection in projections {
        claim_refs.push(claim_ref_for_projection(projection)?);
        source_indexes.push(projection.source_index);
        items.push(json!({
            "label": projection.rendered_text,
            "source_label": source_label_for_claim(&projection.claim),
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
        items.push(snapshot_field_evidence_item(field));
    }

    let block_index = blocks.len();
    let composition_block_path = format!("/sections/{section_index}/blocks/{block_index}");
    let mut block = Block::new(
        BlockId::new(block_id(input, "the-record", "evidence_list", "sources")),
        BlockType::EvidenceList,
        json!({ "items": items }),
        claim_refs,
        ProvenanceRef::new(
            invocation_id,
            FieldPath::new(&composition_block_path).map_err(field_error)?,
        ),
        None,
    )
    .map_err(block_error)?;
    block.field_bindings = evidence_list_display_bindings(items.len())?;
    block.salience = salience(0.58, SalienceBand::Contextual, "source record");
    attribute_block(
        provenance_builder,
        &composition_block_path,
        subject,
        source_indexes,
    )?;
    blocks.push(block);
    Ok(blocks)
}

#[allow(clippy::too_many_arguments)]
fn build_transcript_quote_wall_block(
    input: &NormalizedInput,
    quote_items: Vec<TranscriptQuoteEvidence<'_>>,
    section_index: usize,
    block_index: usize,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let claim_refs = quote_items
        .iter()
        .map(|item| claim_ref_for_projection(item.projection))
        .collect::<Result<Vec<_>, _>>()?;
    let source_indexes = quote_items
        .iter()
        .map(|item| item.projection.source_index)
        .collect::<Vec<_>>();
    let items = quote_items
        .iter()
        .map(|item| {
            let workspace_file_kind = match data_source_for_claim(&item.projection.claim.data_source)
            {
                DataSource::WorkspaceFile { kind } => Some(kind.slug()),
                _ => None,
            };
            json!({
                "label": item.quote.text.as_str(),
                "claim_id": item.projection.claim.id.as_str(),
                "claim_type": item.projection.claim.claim_type.as_str(),
                "assertion_text": item.projection.rendered_text.as_str(),
                "evidence_quote": item.quote.text.as_str(),
                "quote_exactness": "exact_match",
                "quote_redaction_policy": TRANSCRIPT_QUOTE_REDACTION_POLICY,
                "source_label": source_label_for_claim(&item.projection.claim),
                "source_asof": item.projection.claim.source_asof,
                "workspace_file_kind": workspace_file_kind,
                "trust_band": trust_band_label(item.projection.trust_band),
                "sensitivity": claim_sensitivity_label(&item.projection.claim.sensitivity),
                "redaction_state": "policy_allowed",
                "source_locator": {
                    "type": "char_range",
                    "start_char": item.quote.start_char,
                    "end_char": item.quote.end_char,
                },
                "feedback_allowed": true,
                "feedback_claim_id": item.projection.claim.id.as_str(),
                "feedback_route": "claim_feedback",
            })
        })
        .collect::<Vec<_>>();
    let composition_block_path = format!("/sections/{section_index}/blocks/{block_index}");
    let mut block = Block::new(
        BlockId::new(block_id(input, "the-record", "evidence_list", "their-voice")),
        BlockType::EvidenceList,
        json!({
            "title": "Their voice",
            "items": items
        }),
        claim_refs,
        ProvenanceRef::new(
            invocation_id,
            FieldPath::new(&composition_block_path).map_err(field_error)?,
        ),
        None,
    )
    .map_err(block_error)?;
    block.field_bindings = transcript_quote_display_bindings(items.len())?;
    block.salience = salience(0.66, SalienceBand::Contextual, "transcript quotes");
    attribute_block(
        provenance_builder,
        &composition_block_path,
        subject,
        source_indexes,
    )?;
    Ok(block)
}

fn transcript_quote_evidence_items(
    projections: &[ClaimProjection],
) -> Vec<TranscriptQuoteEvidence<'_>> {
    projections
        .iter()
        .filter_map(|projection| {
            transcript_verified_quote_text(projection).map(|quote| TranscriptQuoteEvidence {
                projection,
                quote,
            })
        })
        .take(MAX_TRANSCRIPT_QUOTE_ITEMS)
        .collect()
}

fn transcript_verified_quote_text(projection: &ClaimProjection) -> Option<TranscriptVerifiedQuote> {
    match data_source_for_claim(&projection.claim.data_source) {
        DataSource::WorkspaceFile { .. } => {}
        _ => return None,
    }
    let metadata: Value = serde_json::from_str(projection.claim.metadata_json.as_deref()?).ok()?;
    if metadata.get("producer").and_then(Value::as_str) != Some("transcript_claims") {
        return None;
    }
    if metadata.get("quote_verified").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let quote = metadata.get("quote")?;
    if quote.get("verification").and_then(Value::as_str) != Some("exact_match") {
        return None;
    }
    if quote.get("redaction_policy").and_then(Value::as_str)
        != Some(TRANSCRIPT_QUOTE_REDACTION_POLICY)
    {
        return None;
    }
    let start = quote.get("start_char").and_then(Value::as_u64)?;
    let end = quote.get("end_char").and_then(Value::as_u64)?;
    if start >= end {
        return None;
    }
    let text = quote.get("text").and_then(Value::as_str)?.trim();
    (!text.is_empty()).then(|| TranscriptVerifiedQuote {
        text: text.to_string(),
        start_char: start,
        end_char: end,
    })
}

#[allow(clippy::too_many_arguments)]
fn build_reports_section_blocks(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    snapshot_fields: Vec<&AccountCompositionSnapshotField>,
    section_index: usize,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Vec<Block>, AbilityError> {
    if snapshot_fields.is_empty() {
        let composition_block_path = format!("/sections/{section_index}/blocks/0");
        let mut block = Block::new(
            BlockId::new(block_id(input, "reports", "action_list", "system_config")),
            BlockType::ActionList,
            json!({
                "claim_type": "system_config",
                "items": [{
                    "title": "Account report",
                    "status": "unavailable",
                    "text": "No generated account report is currently available."
                }]
            }),
            Vec::new(),
            ProvenanceRef::new(
                invocation_id,
                FieldPath::new(&composition_block_path).map_err(field_error)?,
            ),
            None,
        )
        .map_err(block_error)?;
        block.field_bindings = action_list_display_bindings(1)?;
        block.salience = salience(0.32, SalienceBand::Background, "report availability");
        attribute_block(
            provenance_builder,
            &composition_block_path,
            subject,
            Vec::new(),
        )?;
        return Ok(vec![block]);
    }

    Ok(vec![build_snapshot_fields_block(
        ctx,
        input,
        "reports",
        section_index,
        0,
        snapshot_fields,
        subject,
        invocation_id,
        provenance_builder,
    )?])
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
    let (block_type, mut attributes, mut bindings, salience_value, salience_band, salience_reason) =
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
            ClaimPlacement::Win => (
                BlockType::ClaimSummary,
                json!({
                    "intent": "win",
                    "claim_id": projection.claim.id,
                    "text": projection.rendered_text,
                    "claim_type": projection.claim.claim_type,
                    "trust_band": trust_band,
                    "source_asof": projection.claim.source_asof,
                }),
                source_feedback_computed_bindings("/text", "/trust_band")?,
                0.72,
                SalienceBand::Important,
                "win claim",
            ),
            ClaimPlacement::Value => (
                BlockType::ClaimSummary,
                json!({
                    "intent": "value",
                    "claim_id": projection.claim.id,
                    "text": projection.rendered_text,
                    "claim_type": projection.claim.claim_type,
                    "trust_band": trust_band,
                    "source_asof": projection.claim.source_asof,
                }),
                source_feedback_computed_bindings("/text", "/trust_band")?,
                0.72,
                SalienceBand::Important,
                "value claim",
            ),
            ClaimPlacement::Commitment => (
                BlockType::ActionList,
                json!({
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
            ClaimPlacement::Health => (
                BlockType::HealthSnapshot,
                json!({
                    "claim_id": projection.claim.id,
                    "text": projection.rendered_text,
                    "claim_type": projection.claim.claim_type,
                    "trust_band": trust_band,
                    "source_asof": projection.claim.source_asof,
                }),
                source_feedback_computed_bindings("/text", "/trust_band")?,
                0.82,
                SalienceBand::Important,
                "health claim",
            ),
            ClaimPlacement::Overview => (
                BlockType::ClaimSummary,
                json!({
                    "intent": "context",
                    "claim_id": projection.claim.id,
                    "text": projection.rendered_text,
                    "claim_type": projection.claim.claim_type,
                    "trust_band": trust_band,
                    "source_asof": projection.claim.source_asof,
                }),
                source_feedback_computed_bindings("/text", "/trust_band")?,
                0.58,
                SalienceBand::Contextual,
                "account context claim",
            ),
            ClaimPlacement::Ignored => {
                return Err(validation_error(
                    "unexpected account overview block placement",
                ));
            }
        };

    // Provenance class drives trust rendering (opacity) on every surface:
    // hard-sourced claims read at full presence, enrichment inferences fade.
    // Keyed off source_ref presence, NOT the cold-start trust score (DOS-853).
    let provenance_kind = claim_provenance_kind(&projection.claim);
    let provenance_kind_binding_path = match projection.placement {
        ClaimPlacement::Commitment => {
            attributes["items"][0]["provenance_kind"] = json!(provenance_kind);
            "/items/0/provenance_kind"
        }
        ClaimPlacement::Relationship => {
            attributes["nodes"][0]["provenance_kind"] = json!(provenance_kind);
            "/nodes/0/provenance_kind"
        }
        _ => {
            attributes["provenance_kind"] = json!(provenance_kind);
            "/provenance_kind"
        }
    };
    bindings.push(display_only_binding(provenance_kind_binding_path)?);

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

fn transcript_quote_display_bindings(item_count: usize) -> Result<Vec<FieldBinding>, AbilityError> {
    let mut bindings = Vec::with_capacity(item_count * 19);
    for index in 0..item_count {
        for field in [
            "label",
            "claim_id",
            "claim_type",
            "assertion_text",
            "evidence_quote",
            "quote_exactness",
            "quote_redaction_policy",
            "source_label",
            "source_asof",
            "workspace_file_kind",
            "trust_band",
            "sensitivity",
            "redaction_state",
            "feedback_claim_id",
            "feedback_route",
        ] {
            bindings.push(display_only_binding(&format!("/items/{index}/{field}"))?);
        }
        bindings.push(binding(
            &format!("/items/{index}/evidence_quote"),
            BindingRole::FeedbackTarget,
            vec![index],
        )?);
        bindings.push(display_only_binding(&format!(
            "/items/{index}/feedback_allowed"
        ))?);
        bindings.push(display_only_binding(&format!(
            "/items/{index}/source_locator/type"
        ))?);
        bindings.push(display_only_binding(&format!(
            "/items/{index}/source_locator/start_char"
        ))?);
        bindings.push(display_only_binding(&format!(
            "/items/{index}/source_locator/end_char"
        ))?);
    }
    Ok(bindings)
}

fn action_list_display_bindings(item_count: usize) -> Result<Vec<FieldBinding>, AbilityError> {
    let mut bindings = Vec::with_capacity(item_count * 3);
    for index in 0..item_count {
        bindings.push(display_only_binding(&format!("/items/{index}/title"))?);
        bindings.push(display_only_binding(&format!("/items/{index}/status"))?);
        bindings.push(display_only_binding(&format!("/items/{index}/text"))?);
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
/// tracked separately (DOS-853).
fn claim_provenance_kind(claim: &IntelligenceClaim) -> &'static str {
    match claim.source_ref.as_deref() {
        Some(reference) if !reference.trim().is_empty() => "sourced",
        _ => "inferred",
    }
}

fn source_label_for_claim(claim: &IntelligenceClaim) -> String {
    data_source_for_claim(&claim.data_source).display_name()
}

fn claim_sensitivity_label(sensitivity: &ClaimSensitivity) -> &'static str {
    match sensitivity {
        ClaimSensitivity::Public => "public",
        ClaimSensitivity::Internal => "internal",
        ClaimSensitivity::Confidential => "confidential",
        ClaimSensitivity::UserOnly => "user_only",
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
                    && block.attributes.pointer("/text").and_then(Value::as_str)
                        == Some("Adoption milestone shipped")),
            "claim evidence remains renderable"
        );
        assert!(
            value_section
                .blocks
                .iter()
                .any(|block| block.block_type == BlockType::EvidenceList
                    && block.attributes.to_string().contains("Growth potential")),
            "snapshot evidence remains renderable beside claims"
        );
    }

    #[tokio::test]
    async fn transcript_claim_quotes_render_their_voice_evidence() {
        let mut verified = claim(
            "claim-transcript-quote",
            "entity_win",
            "/transcript/wins",
            "Expansion interest surfaced in the transcript",
            Some(0.91),
            Some("2026-05-14T09:00:00Z"),
            ClaimSensitivity::Internal,
        );
        verified.data_source = "workspace_file:quill_transcript".to_string();
        verified.source_ref = Some("workspace_file:file-quill-1".to_string());
        verified.temporal_scope = TemporalScope::PointInTime;
        verified.metadata_json = Some(
            json!({
                "producer": "transcript_claims",
                "quote_verified": true,
                "quote": {
                    "text": "We would expand if onboarding gets easier.",
                    "start_char": 12,
                    "end_char": 55,
                    "verification": "exact_match",
                    "redaction_policy": "sensitivity_ceiling"
                }
            })
            .to_string(),
        );

        let mut unverified = claim(
            "claim-transcript-unverified",
            "entity_risk",
            "/transcript/risks",
            "Unverified paraphrase should not enter the quote wall",
            Some(0.84),
            Some("2026-05-14T09:00:00Z"),
            ClaimSensitivity::Internal,
        );
        unverified.data_source = "workspace_file:quill_transcript".to_string();
        unverified.source_ref = Some("workspace_file:file-quill-2".to_string());
        unverified.temporal_scope = TemporalScope::PointInTime;
        unverified.metadata_json = Some(
            json!({
                "producer": "transcript_claims",
                "quote_verified": false,
                "quote": {
                    "text": "This is not exact.",
                    "start_char": 4,
                    "end_char": 22,
                    "verification": "paraphrase",
                    "redaction_policy": "sensitivity_ceiling"
                }
            })
            .to_string(),
        );

        let mut missing_policy = claim(
            "claim-transcript-no-policy",
            "entity_win",
            "/transcript/wins",
            "Policyless quote should not enter the quote wall",
            Some(0.82),
            Some("2026-05-14T09:00:00Z"),
            ClaimSensitivity::Internal,
        );
        missing_policy.data_source = "workspace_file:quill_transcript".to_string();
        missing_policy.source_ref = Some("workspace_file:file-quill-3".to_string());
        missing_policy.temporal_scope = TemporalScope::PointInTime;
        missing_policy.metadata_json = Some(
            json!({
                "producer": "transcript_claims",
                "quote_verified": true,
                "quote": {
                    "text": "This lacks a quote redaction policy.",
                    "start_char": 6,
                    "end_char": 43,
                    "verification": "exact_match"
                }
            })
            .to_string(),
        );

        let mut confidential = claim(
            "claim-transcript-confidential",
            "entity_win",
            "/transcript/wins",
            "Confidential transcript claim should not render",
            Some(0.8),
            Some("2026-05-14T09:00:00Z"),
            ClaimSensitivity::Confidential,
        );
        confidential.data_source = "workspace_file:quill_transcript".to_string();
        confidential.source_ref = Some("workspace_file:file-quill-4".to_string());
        confidential.temporal_scope = TemporalScope::PointInTime;
        confidential.metadata_json = Some(
            json!({
                "producer": "transcript_claims",
                "quote_verified": true,
                "quote": {
                    "text": "This confidential quote should not render.",
                    "start_char": 1,
                    "end_char": 44,
                    "verification": "exact_match",
                    "redaction_policy": "sensitivity_ceiling"
                }
            })
            .to_string(),
        );

        let (clock, rng, external, reader, committer, provider) =
            fixture_parts(vec![verified, unverified, missing_policy, confidential]);
        let services = services(&clock, &rng, &external, reader, committer);
        let ctx = ability_ctx(&services, &provider);

        let output = account_overview(&ctx, input())
            .await
            .expect("transcript-backed account overview succeeds");
        let composition = output.data();
        let record_section = composition
            .sections
            .iter()
            .find(|section| section.id.as_str() == "the-record")
            .expect("record section exists");
        let quote_block = record_section
            .blocks
            .iter()
            .find(|block| {
                block.block_type == BlockType::EvidenceList
                    && block.attributes.pointer("/title").and_then(Value::as_str)
                        == Some("Their voice")
            })
            .expect("verified transcript quote block exists");

        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/label")
                .and_then(Value::as_str),
            Some("We would expand if onboarding gets easier.")
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/claim_id")
                .and_then(Value::as_str),
            Some("claim-transcript-quote")
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/claim_type")
                .and_then(Value::as_str),
            Some("entity_win")
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/assertion_text")
                .and_then(Value::as_str),
            Some("Expansion interest surfaced in the transcript")
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/evidence_quote")
                .and_then(Value::as_str),
            Some("We would expand if onboarding gets easier.")
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/quote_exactness")
                .and_then(Value::as_str),
            Some("exact_match")
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/quote_redaction_policy")
                .and_then(Value::as_str),
            Some(TRANSCRIPT_QUOTE_REDACTION_POLICY)
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/source_label")
                .and_then(Value::as_str),
            Some("Workspace file (Quill transcript)")
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/workspace_file_kind")
                .and_then(Value::as_str),
            Some("quill_transcript")
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/trust_band")
                .and_then(Value::as_str),
            Some("likely_current")
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/sensitivity")
                .and_then(Value::as_str),
            Some("internal")
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/redaction_state")
                .and_then(Value::as_str),
            Some("policy_allowed")
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/source_locator/type")
                .and_then(Value::as_str),
            Some("char_range")
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/source_locator/start_char")
                .and_then(Value::as_u64),
            Some(12)
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/source_locator/end_char")
                .and_then(Value::as_u64),
            Some(55)
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/feedback_allowed")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/feedback_claim_id")
                .and_then(Value::as_str),
            Some("claim-transcript-quote")
        );
        assert!(quote_block.field_bindings.iter().any(|binding| {
            binding.role == BindingRole::FeedbackTarget
                && binding.field_path.as_str() == "/items/0/evidence_quote"
                && !binding.claim_refs.is_empty()
        }));
        assert_eq!(
            quote_block
                .attributes
                .pointer("/items/0/source_asof")
                .and_then(Value::as_str),
            Some("2026-05-14T09:00:00Z")
        );
        assert!(!quote_block.attributes.to_string().contains("This is not exact"));
        assert!(!quote_block
            .attributes
            .to_string()
            .contains("This lacks a quote redaction policy."));
        assert!(!quote_block
            .attributes
            .to_string()
            .contains("This confidential quote should not render."));
        assert!(quote_block
            .claim_refs
            .iter()
            .any(|claim_ref| claim_ref.claim_id == "claim-transcript-quote"));
        assert!(output.provenance().sources.iter().any(|source| {
            matches!(&source.data_source, DataSource::WorkspaceFile { .. })
                && source.identifiers.iter().any(|identifier| {
                    matches!(
                        identifier,
                        SourceIdentifier::Document { document_id, .. }
                            if document_id.0.as_str() == "file-quill-1"
                    )
                })
        }));

        let proj_ctx = FallbackProjectionContext::new(
            Actor::SurfaceClient {
                instance: crate::abilities::registry::SurfaceClientId::new("sc_fixture"),
                scopes: ScopeSet::new([
                    crate::abilities::registry::SurfaceScope::new("read.account_overview"),
                    crate::abilities::registry::SurfaceScope::new("submit.feedback"),
                ])
                .expect("scope set"),
            },
            SurfaceKind::SurfaceClient,
            3,
        );
        let (projected, _audits) = project_composition_for_surface(composition, &proj_ctx)
            .expect("projected transcript quote preserves contract fields");
        let projected_record = projected
            .sections
            .iter()
            .find(|section| section.section_id.as_str() == "the-record")
            .expect("projected record section");
        let projected_quote = projected_record
            .block_indexes
            .iter()
            .filter_map(|index| projected.blocks.get(*index as usize))
            .find(|block| block.payload.pointer("/title").and_then(Value::as_str) == Some("Their voice"))
            .expect("projected quote wall block");
        assert_eq!(
            projected_quote
                .payload
                .pointer("/items/0/evidence_quote")
                .and_then(Value::as_str),
            Some("We would expand if onboarding gets easier.")
        );
        assert_eq!(
            projected_quote
                .payload
                .pointer("/items/0/workspace_file_kind")
                .and_then(Value::as_str),
            Some("quill_transcript")
        );
        assert_eq!(
            projected_quote
                .payload
                .pointer("/items/0/trust_band")
                .and_then(Value::as_str),
            Some("likely_current")
        );
        assert_eq!(
            projected_quote
                .payload
                .pointer("/items/0/feedback_allowed")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(projected_quote.edit_routes.iter().any(|route| {
            route.feedback_allowed
                && route.field_path.as_str() == "/items/0/evidence_quote"
                && !route.claim_refs.is_empty()
        }));
    }

    #[tokio::test]
    async fn recommendation_claims_render_as_work_actions() {
        let claims = vec![claim(
            "claim-recommendation",
            "recommendation",
            "/recommendations/review",
            "Review the launch plan with the account owner",
            Some(0.88),
            Some("2026-05-14T09:00:00Z"),
            ClaimSensitivity::Internal,
        )];
        let (clock, rng, external, reader, committer, provider) = fixture_parts(claims);
        let services = services(&clock, &rng, &external, reader, committer);
        let ctx = ability_ctx(&services, &provider);

        let output = account_overview(&ctx, input())
            .await
            .expect("recommendation-backed account overview succeeds");
        let composition = output.data();

        for section_id in ["whats-next", "the-work"] {
            let section = composition
                .sections
                .iter()
                .find(|section| section.id.as_str() == section_id)
                .expect("work section exists");
            let block = section
                .blocks
                .iter()
                .find(|block| {
                    block.block_type == BlockType::ActionList
                        && block
                            .attributes
                            .pointer("/claim_type")
                            .and_then(Value::as_str)
                            == Some("recommendation")
                        && block
                            .attributes
                            .pointer("/items/0/text")
                            .and_then(Value::as_str)
                            == Some("Review the launch plan with the account owner")
                })
                .expect("recommendation is rendered as a work action");

            assert!(block
                .claim_refs
                .iter()
                .any(|claim_ref| claim_ref.claim_id == "claim-recommendation"));
            assert!(block.field_bindings.iter().any(|binding| {
                binding.role == BindingRole::FeedbackTarget
                    && binding.field_path.as_str() == "/items/0/text"
                    && !binding.claim_refs.is_empty()
            }));
            assert_eq!(
                block
                    .attributes
                    .pointer("/trust_band")
                    .and_then(Value::as_str),
                Some("likely_current"),
                "work action blocks expose block-level trust for the shell badge"
            );
            assert_eq!(
                block
                    .attributes
                    .pointer("/source_asof")
                    .and_then(Value::as_str),
                Some("2026-05-14T09:00:00Z"),
                "work action blocks expose block-level freshness for the shell"
            );
        }

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
            .expect("projected recommendation action preserves shell metadata");

        for section_id in ["whats-next", "the-work"] {
            let section = projected
                .sections
                .iter()
                .find(|section| section.section_id.as_str() == section_id)
                .expect("projected work section exists");
            let projected_block = section
                .block_indexes
                .iter()
                .filter_map(|index| projected.blocks.get(*index as usize))
                .find(|block| {
                    block.payload.pointer("/claim_type").and_then(Value::as_str)
                        == Some("recommendation")
                })
                .expect("projected recommendation work action exists");

            assert_eq!(
                projected_block
                    .payload
                    .pointer("/trust_band")
                    .and_then(Value::as_str),
                Some("likely_current"),
                "projection must preserve block-level trust for the renderer shell"
            );
            assert_eq!(
                projected_block
                    .payload
                    .pointer("/source_asof")
                    .and_then(Value::as_str),
                Some("2026-05-14T09:00:00Z"),
                "projection must preserve block-level freshness for the renderer shell"
            );
        }
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

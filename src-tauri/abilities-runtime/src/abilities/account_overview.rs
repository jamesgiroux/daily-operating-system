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
use crate::services::context::{CompositionCommitError, CompositionProposal};
use crate::types::{
    prompt_input_sensitivity_allowed, subject_ref_from_json, ClaimState, ClaimSubjectRef,
    IntelligenceClaim, SurfacingState,
};

const ABILITY_NAME: &str = "dailyos/account-overview";
const ABILITY_SCHEMA_VERSION: u32 = 1;
const ACCOUNT_CLAIM_DEPTH: usize = 3;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AccountOverviewInput {
    pub schema_version: u32,
    pub account_id: String,
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

    let composition = build_composition(
        ctx,
        input,
        &projections,
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
        ClaimSubjectRef::Person { .. }
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
        ClaimType::Commitment | ClaimType::OpenLoop => ClaimPlacement::Commitment,
        ClaimType::StakeholderEngagement
        | ClaimType::StakeholderAssessment
        | ClaimType::StakeholderRole => ClaimPlacement::Relationship,
        ClaimType::EntityCurrentState => ClaimPlacement::Health,
        ClaimType::CompanyContext
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
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Composition, AbilityError> {
    let mut sections = Vec::new();
    let overview_block = build_overview_block(
        input,
        projections,
        subject,
        invocation_id,
        "/sections/0/blocks/0",
        provenance_builder,
    )?;
    let mut overview_section = Section::new(SectionId::new("overview"), vec![overview_block]);
    overview_section.label = Some("Overview".to_string());
    overview_section.salience = salience(0.95, SalienceBand::Critical, "account summary");
    sections.push(overview_section);

    if projections.is_empty() {
        let empty_block = build_empty_state_block(
            input,
            subject,
            invocation_id,
            "/sections/1/blocks/0",
            provenance_builder,
        )?;
        let mut empty_section = Section::new(SectionId::new("empty"), vec![empty_block]);
        empty_section.label = Some("Signals".to_string());
        empty_section.salience = salience(0.4, SalienceBand::Contextual, "no visible claims");
        sections.push(empty_section);
    } else {
        let mut signal_blocks = Vec::new();
        for projection in projections
            .iter()
            .filter(|projection| projection.placement != ClaimPlacement::Overview)
        {
            let block_index = signal_blocks.len();
            signal_blocks.push(build_claim_block(
                input,
                projection,
                subject,
                invocation_id,
                &format!("/sections/1/blocks/{block_index}"),
                provenance_builder,
            )?);
        }
        if !signal_blocks.is_empty() {
            let mut signals_section = Section::new(SectionId::new("signals"), signal_blocks);
            signals_section.label = Some("Signals".to_string());
            signals_section.layout = SectionLayout::Stacked;
            signals_section.salience = salience(0.8, SalienceBand::Important, "visible claims");
            sections.push(signals_section);
        }
    }

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

    attribute_static_composition_fields(provenance_builder, subject)?;
    Ok(composition)
}

fn build_overview_block(
    input: &NormalizedInput,
    projections: &[ClaimProjection],
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    composition_block_path: &str,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let overview_claims = projections
        .iter()
        .filter(|projection| projection.placement == ClaimPlacement::Overview)
        .collect::<Vec<_>>();
    let all_refs = projections
        .iter()
        .map(claim_ref_for_projection)
        .collect::<Result<Vec<_>, _>>()?;
    let context = overview_claims
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
    let trust_band = block_trust_band(projections.iter().map(|projection| projection.trust_band));
    let counts_by_band = trust_band_counts(projections);
    let attributes = json!({
        "account_id": input.account_id,
        "title": "Account overview",
        "claim_count": projections.len(),
        "trust_band": trust_band_label(trust_band),
        "counts_by_trust_band": counts_by_band,
        "context": context,
    });
    let mut block = Block::new(
        BlockId::new(block_id(input, "overview", "account_overview", "summary")),
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

    if projections.is_empty() {
        block.field_bindings = vec![display_only_binding("/attributes/title")?];
        attribute_block(
            provenance_builder,
            composition_block_path,
            subject,
            Vec::new(),
        )?;
    } else {
        block.field_bindings = vec![
            computed_binding("/attributes/claim_count", 0..projections.len())?,
            computed_binding("/attributes/counts_by_trust_band", 0..projections.len())?,
            computed_binding("/attributes/context", 0..projections.len())?,
            display_only_binding("/attributes/title")?,
        ];
        attribute_block(
            provenance_builder,
            composition_block_path,
            subject,
            projections
                .iter()
                .map(|projection| projection.source_index)
                .collect(),
        )?;
    }
    Ok(block)
}

fn build_empty_state_block(
    input: &NormalizedInput,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    composition_block_path: &str,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let attributes = json!({
        "title": "No visible account signals",
        "empty_state": true,
        "trust_band": "needs_verification",
    });
    let mut block = Block::new(
        BlockId::new(block_id(input, "empty", "empty_state", "display")),
        BlockType::ClaimSummary,
        attributes,
        Vec::new(),
        ProvenanceRef::new(
            invocation_id,
            FieldPath::new(composition_block_path).map_err(field_error)?,
        ),
        None,
    )
    .map_err(block_error)?;
    block.field_bindings = vec![
        display_only_binding("/attributes/title")?,
        display_only_binding("/attributes/empty_state")?,
    ];
    block.salience = salience(0.3, SalienceBand::Background, "empty state");
    attribute_block(
        provenance_builder,
        composition_block_path,
        subject,
        Vec::new(),
    )?;
    Ok(block)
}

fn build_claim_block(
    input: &NormalizedInput,
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
                source_feedback_computed_bindings("/attributes/text", "/attributes/trust_band")?,
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
                source_feedback_computed_bindings("/attributes/text", "/attributes/trust_band")?,
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
                source_feedback_computed_bindings("/attributes/text", "/attributes/trust_band")?,
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
                }),
                source_feedback_computed_bindings(
                    "/attributes/items/0/text",
                    "/attributes/items/0/trust_band",
                )?,
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
                source_feedback_computed_bindings(
                    "/attributes/nodes/0/text",
                    "/attributes/nodes/0/trust_band",
                )?,
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
                source_feedback_computed_bindings("/attributes/text", "/attributes/trust_band")?,
                0.82,
                SalienceBand::Important,
                "health claim",
            ),
            ClaimPlacement::Overview | ClaimPlacement::Ignored => {
                return Err(validation_error(
                    "unexpected account overview block placement",
                ));
            }
        };

    let mut block = Block::new(
        BlockId::new(block_id(
            input,
            "signals",
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

fn attribute_static_composition_fields(
    builder: &mut ProvenanceBuilder,
    subject: &SubjectAttribution,
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
        "/sections/0/id",
        "/sections/0/label",
        "/sections/0/layout",
        "/sections/0/salience",
    ] {
        builder
            .attribute_subtree(
                FieldPath::new(path).map_err(field_error)?,
                FieldAttribution::constant(subject.clone()),
            )
            .map_err(provenance_error)?;
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
    builder
        .attribute_subtree(path, attribution)
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

fn display_only_binding(field_path: &str) -> Result<FieldBinding, AbilityError> {
    binding(field_path, BindingRole::DisplayOnly, Vec::new())
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
    SourceAttribution::new(
        data_source_for_claim(&claim.data_source),
        vec![SourceIdentifier::Entity {
            entity_id: EntityId::new(account_id.to_string()),
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
    use crate::abilities::{Actor, NOOP_ABILITY_TRACER};
    use crate::intelligence::provider::{
        Completion, FingerprintMetadata, IntelligenceProvider, ModelName, ModelTier, PromptInput,
        ProviderError, ProviderKind,
    };
    use crate::sensitivity::{ClaimDismissalSurface, ClaimVerificationState};
    use crate::services::context::{
        CompositionCommitFuture, CompositionCommitHandle, CompositionCommitRequest,
        EntityContextClaimReadFuture, EntityContextClaimReadHandle, ExternalClients, FixedClock,
        SeedableRng, ServiceContext,
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
    async fn empty_claim_set_returns_display_only_empty_state() {
        let (clock, rng, external, reader, committer, provider) = fixture_parts(Vec::new());
        let services = services(&clock, &rng, &external, reader, committer);
        let ctx = ability_ctx(&services, &provider);

        let output = account_overview(&ctx, input())
            .await
            .expect("empty state succeeds");
        let empty_block = output
            .data()
            .sections
            .iter()
            .find(|section| section.id.as_str() == "empty")
            .and_then(|section| section.blocks.first())
            .expect("empty block exists");

        assert!(empty_block.claim_refs.is_empty());
        assert!(empty_block.field_bindings.iter().all(|binding| {
            binding.role == BindingRole::DisplayOnly && binding.claim_refs.is_empty()
        }));
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
}

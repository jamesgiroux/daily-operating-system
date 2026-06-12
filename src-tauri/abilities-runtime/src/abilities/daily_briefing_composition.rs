use chrono::{NaiveDate, Utc};
use dailyos_abilities_macro::ability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::abilities::composition::{
    AbilityRef, BindingRole, Block, BlockId, BlockType, Composition, CompositionDocId,
    CompositionKind, CompositionMetadata, CompositionVersion, EntityRef, FieldBinding,
    ProvenanceRef, Salience, SalienceBand, Section, SectionId, SectionLayout,
};
use crate::abilities::get_daily_briefing::{
    producer::build_daily_briefing, BriefingAdvisory, BriefingAvailability, BriefingFreshness,
    BriefingIntegrity, BriefingSection, DailyBriefingInput, DailyBriefingOutput, MeetingBriefRef,
    BRIEFING_SCHEMA_VERSION,
};
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, Confidence, DataSource, FieldAttribution, FieldPath,
    InputsSnapshot, InvocationId, MeetingId, ProvenanceBuilder, ProvenanceBuilderConfig,
    SchemaVersion, SourceAttribution, SourceIdentifier, SourceRef, SubjectAttribution, SubjectRef,
};
use crate::abilities::trust::TrustBand;
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};
use crate::services::context::{CompositionCommitError, CompositionProposal};

const ABILITY_NAME: &str = "dailyos/daily-briefing";
const ABILITY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct DailyBriefingCompositionInput {
    pub schema_version: u32,
    #[schemars(with = "String")]
    pub date: NaiveDate,
    pub workspace_scope: String,
    pub workspace_id: String,
    #[serde(default)]
    pub expected_composition_version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition_id: Option<String>,
}

#[derive(Debug, Clone)]
struct NormalizedInput {
    date: NaiveDate,
    workspace_id: String,
    composition_id: CompositionDocId,
    subject_key: String,
    expected_composition_version: u64,
}

#[ability(
    name = "dailyos/daily-briefing",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.daily_briefing"],
    mcp_exposure = None,
    client_side_executable = false,
    composes = [
        { id = "get_daily_briefing", ability = "get_daily_briefing", optional = false }
    ],
    experimental = false,
    signal_policy = { emits_on_output_change = [
        "daily_readiness.changed",
        "meeting_prep.status_changed",
        "claim.version",
        "claim.lifecycle",
        "claim.dismissal",
        "source.freshness",
        "source.revocation"
    ], coalesce = true }
)]
pub async fn daily_briefing_composition(
    ctx: &AbilityContext<'_>,
    input: DailyBriefingCompositionInput,
) -> AbilityResult<Composition> {
    let input = normalize_input(input)?;
    let briefing_output = build_daily_briefing(
        ctx,
        DailyBriefingInput {
            schema_version: BRIEFING_SCHEMA_VERSION,
            date: input.date,
            workspace_id: input.workspace_id.clone(),
            upcoming_meetings_cursor: None,
            sections: Some(BriefingSection::ALL.to_vec()),
        },
    )
    .await?;
    let briefing = briefing_output.data();

    let subject = SubjectAttribution::direct_confident(SubjectRef::Global);
    let provenance_config = provenance_config(ctx);
    let invocation_id = provenance_config.invocation_id;
    let mut provenance_builder = ProvenanceBuilder::new(provenance_config);
    provenance_builder.set_subject(subject.clone());

    let composition = build_composition(
        ctx,
        &input,
        briefing,
        &subject,
        invocation_id,
        &mut provenance_builder,
    )?;
    let committed = ctx
        .services()
        .commit_composition(CompositionProposal {
            composition_id: input.composition_id,
            expected_composition_version: input.expected_composition_version,
            composition,
        })
        .await
        .map_err(composition_commit_error)?;
    let output = provenance_builder
        .finalize(committed.composition)
        .map_err(provenance_error)?;
    validate_block_provenance(output.data(), output.provenance())?;
    Ok(output)
}

fn normalize_input(input: DailyBriefingCompositionInput) -> Result<NormalizedInput, AbilityError> {
    if input.schema_version != ABILITY_SCHEMA_VERSION {
        return Err(validation_error(format!(
            "unsupported schema_version `{}` for `{ABILITY_NAME}`",
            input.schema_version
        )));
    }
    if input.workspace_scope != "local" {
        return Err(validation_error("workspace_scope must be `local`"));
    }
    if input.workspace_id.trim().is_empty() {
        return Err(validation_error("workspace_id must be non-empty"));
    }
    let subject_key = format!("local~{}", input.date);
    let composition_id = input
        .composition_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{ABILITY_NAME}:briefing:{subject_key}"));
    if composition_id != format!("{ABILITY_NAME}:briefing:{subject_key}") {
        return Err(validation_error(format!(
            "composition_id must be `{ABILITY_NAME}:briefing:{subject_key}`"
        )));
    }
    if composition_id.chars().any(char::is_control) {
        return Err(validation_error(
            "composition_id must not contain control characters",
        ));
    }
    Ok(NormalizedInput {
        date: input.date,
        workspace_id: input.workspace_id,
        composition_id: CompositionDocId::new(composition_id),
        subject_key,
        expected_composition_version: input.expected_composition_version,
    })
}

fn build_composition(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    briefing: &DailyBriefingOutput,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Composition, AbilityError> {
    let local_source = local_source(ctx, &input.subject_key, briefing)?;
    let source_index = provenance_builder.add_source(local_source);
    provenance_builder.set_source_trust_band(
        source_index,
        visible_trust_band(briefing.trust_summary.aggregate_band),
    );

    let sections = vec![
        section(
            "headline",
            "Today",
            vec![block(
                input,
                "headline",
                0,
                0,
                BlockType::AccountOverview,
                state_attributes(briefing),
                source_index,
                subject,
                invocation_id,
                provenance_builder,
                salience(0.96, SalienceBand::Critical, "daily briefing state"),
            )?],
            SectionLayout::Stacked,
            salience(0.96, SalienceBand::Critical, "daily briefing state"),
        ),
        section(
            "schedule",
            "Schedule",
            vec![block(
                input,
                "schedule",
                1,
                0,
                BlockType::ActionList,
                meetings_attributes(briefing),
                source_index,
                subject,
                invocation_id,
                provenance_builder,
                salience(0.84, SalienceBand::Important, "daily meeting sequence"),
            )?],
            SectionLayout::Stacked,
            salience(0.84, SalienceBand::Important, "daily meeting sequence"),
        ),
        section(
            "attention",
            "Attention",
            vec![block(
                input,
                "attention",
                2,
                0,
                BlockType::ClaimSummary,
                attention_attributes(briefing),
                source_index,
                subject,
                invocation_id,
                provenance_builder,
                salience(0.72, SalienceBand::Important, "daily briefing advisories"),
            )?],
            SectionLayout::Stacked,
            salience(0.72, SalienceBand::Important, "daily briefing advisories"),
        ),
        section(
            "readiness",
            "Readiness",
            vec![block(
                input,
                "readiness",
                3,
                0,
                BlockType::ClaimSummary,
                readiness_attributes(briefing),
                source_index,
                subject,
                invocation_id,
                provenance_builder,
                salience(0.68, SalienceBand::Important, "daily readiness posture"),
            )?],
            SectionLayout::Stacked,
            salience(0.68, SalienceBand::Important, "daily readiness posture"),
        ),
        section(
            "follow-through",
            "Follow-through",
            vec![block(
                input,
                "follow-through",
                4,
                0,
                BlockType::ActionList,
                follow_through_attributes(briefing),
                source_index,
                subject,
                invocation_id,
                provenance_builder,
                salience(0.62, SalienceBand::Contextual, "daily follow-through"),
            )?],
            SectionLayout::Stacked,
            salience(0.62, SalienceBand::Contextual, "daily follow-through"),
        ),
        section(
            "trust-and-sources",
            "Trust and sources",
            vec![block(
                input,
                "trust-and-sources",
                5,
                0,
                BlockType::EvidenceList,
                sources_attributes(briefing),
                source_index,
                subject,
                invocation_id,
                provenance_builder,
                salience(0.52, SalienceBand::Contextual, "daily briefing sources"),
            )?],
            SectionLayout::Grid,
            salience(0.52, SalienceBand::Contextual, "daily briefing sources"),
        ),
    ];

    let section_count = sections.len();
    let generated_at = ctx.services().clock.now();
    let composition = Composition::new(
        input.composition_id.clone(),
        CompositionKind::Briefing,
        Some(EntityRef::new(format!("briefing:{}", input.subject_key))),
        sections,
        salience(0.92, SalienceBand::Critical, "daily briefing"),
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

fn state_attributes(briefing: &DailyBriefingOutput) -> Value {
    let trust_band = trust_band_label(briefing.trust_summary.aggregate_band);
    json!({
        "entity_type": "briefing",
        "account": {
            "id": format!("briefing:{}", briefing.date),
            "display_name": "Daily briefing",
            "type": "Briefing",
        },
        "title": "Daily briefing",
        "summary": briefing_state_summary(briefing),
        "date": briefing.date.to_string(),
        "availability": availability_label(&briefing.state.availability),
        "freshness": freshness_label(&briefing.state.freshness),
        "integrity": integrity_label(&briefing.state.integrity),
        "trust_band": trust_band,
        "counts_by_trust_band": {
            "likely_current": briefing.trust_summary.likely_current_count,
            "use_with_caution": briefing.trust_summary.use_with_caution_count,
            "needs_verification": briefing.trust_summary.needs_verification_count,
        },
        "source_asof_count": briefing.source_asof_inputs.len(),
        "source_asof": latest_source_asof(briefing),
    })
}

fn meetings_attributes(briefing: &DailyBriefingOutput) -> Value {
    let mut items = Vec::new();
    if let Some(meeting) = &briefing.current_meeting {
        items.push(meeting_item("Current", meeting));
    }
    if let Some(meeting) = &briefing.next_meeting {
        items.push(meeting_item("Next", meeting));
    }
    for meeting in &briefing.upcoming_meetings.items {
        items.push(meeting_item("Upcoming", meeting));
    }
    json!({
        "title": "Meetings",
        "items": items,
        "trust_band": trust_band_label(briefing.trust_summary.aggregate_band),
        "source_asof": latest_source_asof(briefing),
    })
}

fn meeting_item(label: &str, meeting: &MeetingBriefRef) -> Value {
    json!({
        "text": meeting.title.as_deref().unwrap_or("Untitled meeting"),
        "status": meeting.prep_status.as_str(),
        "label": label,
        "starts_at": meeting.starts_at.as_deref(),
        "ends_at": meeting.ends_at.as_deref(),
        "linked_entity_type": meeting.linked_entity_type.as_deref(),
        "linked_entity_id": meeting.linked_entity_id.as_deref(),
        "source_asof": meeting.last_prepared_at.as_deref(),
    })
}

fn attention_attributes(briefing: &DailyBriefingOutput) -> Value {
    let mut lines = Vec::new();
    for advisory in &briefing.state.advisories {
        lines.push(advisory_label(advisory));
    }
    for proposal in &briefing.watch_proposals {
        lines.push(format!("{}: {}", proposal.subject_kind, proposal.headline));
    }
    let text = if lines.is_empty() {
        "No source-backed advisories need attention for this briefing.".to_string()
    } else {
        lines.join("\n")
    };
    json!({
        "title": "Attention",
        "text": text,
        "body": text,
        "trust_band": trust_band_label(briefing.trust_summary.aggregate_band),
        "advisory_count": briefing.state.advisories.len(),
        "watch_proposal_count": briefing.watch_proposals.len(),
        "source_asof": latest_source_asof(briefing),
    })
}

fn readiness_attributes(briefing: &DailyBriefingOutput) -> Value {
    let lines = [
        availability_label(&briefing.state.availability),
        freshness_label(&briefing.state.freshness),
        integrity_label(&briefing.state.integrity),
    ];
    json!({
        "title": "Readiness",
        "text": lines.join("\n"),
        "body": lines.join("\n"),
        "trust_band": trust_band_label(briefing.trust_summary.aggregate_band),
        "source_asof": latest_source_asof(briefing),
    })
}

fn follow_through_attributes(briefing: &DailyBriefingOutput) -> Value {
    let mut items = Vec::new();
    if let BriefingFreshness::NeedsPreparation { meeting_ids } = &briefing.state.freshness {
        items.push(json!({
            "text": format!("{} meeting(s) need preparation", meeting_ids.len()),
            "status": "prep",
        }));
    }
    for advisory in &briefing.state.advisories {
        items.push(json!({
            "text": advisory_label(advisory),
            "status": "attention",
        }));
    }
    for proposal in &briefing.watch_proposals {
        items.push(json!({
            "text": proposal.headline.as_str(),
            "status": proposal.subject_kind.as_str(),
            "trust_band": trust_band_label(proposal.trust_band),
        }));
    }
    let empty = items.is_empty();
    if empty {
        items.push(json!({
            "text": "No source-backed follow-through items are currently open for this briefing.",
            "status": "clear",
        }));
    }
    json!({
        "title": "Follow-through",
        "items": items,
        "empty_state": empty,
        "trust_band": trust_band_label(briefing.trust_summary.aggregate_band),
        "source_asof": latest_source_asof(briefing),
    })
}

fn sources_attributes(briefing: &DailyBriefingOutput) -> Value {
    let items = briefing
        .source_asof_inputs
        .iter()
        .map(|source| {
            json!({
                "label": source.source.as_str(),
                "source_label": source.source.as_str(),
                "source_asof": source.as_of.as_str(),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "title": "Sources",
        "items": items,
        "trust_band": trust_band_label(briefing.trust_summary.aggregate_band),
        "source_asof": latest_source_asof(briefing),
    })
}

fn latest_source_asof(briefing: &DailyBriefingOutput) -> Option<&str> {
    briefing
        .source_asof_inputs
        .iter()
        .map(|source| source.as_of.as_str())
        .max()
}

fn briefing_state_summary(briefing: &DailyBriefingOutput) -> String {
    let availability = match &briefing.state.availability {
        BriefingAvailability::Available => "available".to_string(),
        BriefingAvailability::Empty { reason } => format!("empty: {reason:?}"),
        BriefingAvailability::AuthLocked => "auth locked".to_string(),
    };
    let freshness = match &briefing.state.freshness {
        BriefingFreshness::Fresh => "fresh".to_string(),
        BriefingFreshness::Stale { reason } => format!("stale: {reason:?}"),
        BriefingFreshness::NeedsPreparation { meeting_ids } => {
            format!("needs preparation for {} meeting(s)", meeting_ids.len())
        }
    };
    let integrity = match &briefing.state.integrity {
        BriefingIntegrity::Clean => "clean".to_string(),
        BriefingIntegrity::HasCorrections {
            superseded_claim_ids,
        } => format!("{} correction(s)", superseded_claim_ids.len()),
        BriefingIntegrity::HasAmbiguity { ambiguous_pairs } => {
            format!("{} ambiguity pair(s)", ambiguous_pairs.len())
        }
    };
    format!("{availability}; {freshness}; {integrity}")
}

fn availability_label(value: &BriefingAvailability) -> String {
    match value {
        BriefingAvailability::Available => "available".to_string(),
        BriefingAvailability::Empty { reason } => format!("empty: {reason:?}"),
        BriefingAvailability::AuthLocked => "auth locked".to_string(),
    }
}

fn freshness_label(value: &BriefingFreshness) -> String {
    match value {
        BriefingFreshness::Fresh => "fresh".to_string(),
        BriefingFreshness::Stale { reason } => format!("stale: {reason:?}"),
        BriefingFreshness::NeedsPreparation { meeting_ids } => {
            format!("needs preparation for {} meeting(s)", meeting_ids.len())
        }
    }
}

fn integrity_label(value: &BriefingIntegrity) -> String {
    match value {
        BriefingIntegrity::Clean => "clean".to_string(),
        BriefingIntegrity::HasCorrections {
            superseded_claim_ids,
        } => format!("{} correction(s)", superseded_claim_ids.len()),
        BriefingIntegrity::HasAmbiguity { ambiguous_pairs } => {
            format!("{} ambiguity pair(s)", ambiguous_pairs.len())
        }
    }
}

fn advisory_label(advisory: &BriefingAdvisory) -> String {
    match advisory {
        BriefingAdvisory::WatchProposal { summary, .. } => summary.clone(),
        BriefingAdvisory::UnlinkedMeetings { meeting_ids } => {
            format!("{} meeting(s) need an entity link", meeting_ids.len())
        }
        BriefingAdvisory::PartialReadFailure { advisory } => advisory.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn block(
    input: &NormalizedInput,
    section_id: &str,
    section_index: usize,
    block_index: usize,
    block_type: BlockType,
    attributes: Value,
    source_index: crate::abilities::provenance::SourceIndex,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
    salience: Salience,
) -> Result<Block, AbilityError> {
    let composition_block_path = format!("/sections/{section_index}/blocks/{block_index}");
    let mut block = Block::new(
        BlockId::new(format!(
            "{}:{}:{}",
            input.composition_id.as_str(),
            section_id,
            block_type.type_id()
        )),
        block_type,
        attributes,
        Vec::new(),
        ProvenanceRef::new(
            invocation_id,
            FieldPath::new(&composition_block_path).map_err(field_error)?,
        ),
        None,
    )
    .map_err(block_error)?;
    block.field_bindings = vec![display_only_binding("/title")?];
    block.salience = salience;
    attribute_block(
        provenance_builder,
        &composition_block_path,
        subject,
        source_index,
    )?;
    Ok(block)
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

fn local_source(
    ctx: &AbilityContext<'_>,
    subject_key: &str,
    briefing: &DailyBriefingOutput,
) -> Result<SourceAttribution, AbilityError> {
    let now = ctx.services().clock.now();
    let source_asof = briefing
        .source_asof_inputs
        .iter()
        .filter_map(|source| chrono::DateTime::parse_from_rfc3339(&source.as_of).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .max();
    SourceAttribution::new(
        DataSource::LocalEnrichment,
        vec![SourceIdentifier::Meeting {
            meeting_id: MeetingId::new(format!("briefing:{subject_key}")),
        }],
        now,
        source_asof,
        1.0,
        None,
    )
    .map_err(|error| validation_error(format!("invalid briefing source attribution: {error}")))
}

fn attribute_static_composition_fields(
    builder: &mut ProvenanceBuilder,
    subject: &SubjectAttribution,
    section_count: usize,
) -> Result<(), AbilityError> {
    for path in [
        "",
        "/id",
        "/kind",
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
    Ok(())
}

fn attribute_block(
    builder: &mut ProvenanceBuilder,
    composition_block_path: &str,
    subject: &SubjectAttribution,
    source_index: crate::abilities::provenance::SourceIndex,
) -> Result<(), AbilityError> {
    builder
        .attribute(
            FieldPath::new(composition_block_path).map_err(field_error)?,
            FieldAttribution::computed(
                subject.clone(),
                "dailyos.daily_briefing.v1",
                vec![SourceRef::Source { source_index }],
                Confidence::computed(1.0).map_err(field_error)?,
            )
            .map_err(field_error)?,
        )
        .map_err(provenance_error)?;
    Ok(())
}

fn display_only_binding(field_path: &str) -> Result<FieldBinding, AbilityError> {
    Ok(FieldBinding {
        field_path: FieldPath::new(field_path).map_err(field_error)?,
        role: BindingRole::DisplayOnly,
        claim_refs: Vec::new(),
    })
}

fn provenance_config(ctx: &AbilityContext<'_>) -> ProvenanceBuilderConfig {
    let mut config = ProvenanceBuilderConfig::new(ABILITY_NAME, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(ABILITY_SCHEMA_VERSION);
    config.invocation_id = InvocationId::new(uuid::Uuid::new_v4());
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

fn salience(weight: f32, band: SalienceBand, reason: &str) -> Salience {
    Salience {
        weight,
        band,
        reason: reason.to_string(),
    }
}

fn visible_trust_band(band: TrustBand) -> TrustBand {
    match band {
        TrustBand::Unscored => TrustBand::NeedsVerification,
        other => other,
    }
}

fn trust_band_label(band: TrustBand) -> &'static str {
    match visible_trust_band(band) {
        TrustBand::LikelyCurrent => "likely_current",
        TrustBand::UseWithCaution => "use_with_caution",
        TrustBand::NeedsVerification | TrustBand::Unscored => "needs_verification",
    }
}

fn composition_commit_error(error: CompositionCommitError) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::HardError("daily_briefing_composition_commit".to_string()),
        message: error.to_string(),
    }
}

fn validation_error(message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: message.into(),
    }
}

fn provenance_error(error: impl std::fmt::Display) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::HardError("daily_briefing_composition_provenance".to_string()),
        message: format!("provenance error: {error}"),
    }
}

fn field_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("field path error: {error}"))
}

fn block_error(error: impl std::fmt::Display) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::HardError("daily_briefing_composition_block".to_string()),
        message: format!("block build error: {error}"),
    }
}

fn validate_block_provenance(
    composition: &Composition,
    provenance: &crate::abilities::provenance::Provenance,
) -> Result<(), AbilityError> {
    for section in &composition.sections {
        for block in &section.blocks {
            block
                .validate_against(provenance)
                .map_err(|error| block_error(format!("{}: {error}", block.id.as_str())))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::composition::RenderHints;
    use crate::abilities::provenance::field::FieldPath;
    use crate::abilities::registry::Actor;
    use crate::abilities::{
        project_composition_for_surface, FallbackProjectionContext, SurfaceKind,
    };

    fn test_invocation_id() -> InvocationId {
        InvocationId::new(uuid::Uuid::from_u128(
            0x2234_5678_90ab_cdef_1122_3344_5566_7788,
        ))
    }

    fn test_provenance_ref(block_index: usize) -> ProvenanceRef {
        ProvenanceRef::new(
            test_invocation_id(),
            FieldPath::new(format!("/sections/0/blocks/{block_index}")).unwrap(),
        )
    }

    fn test_block(
        id: &str,
        block_type: BlockType,
        attributes: Value,
        bindings: Vec<FieldBinding>,
        block_index: usize,
    ) -> Block {
        Block {
            id: BlockId::new(id),
            block_type,
            attributes,
            claim_refs: Vec::new(),
            field_bindings: bindings,
            provenance: test_provenance_ref(block_index),
            salience: Salience::default(),
            render_hints: RenderHints::default(),
        }
    }

    fn test_composition(blocks: Vec<Block>) -> Composition {
        let generated_at =
            chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 6, 3, 0, 0, 0).unwrap();
        Composition::new(
            CompositionDocId::new("dailyos/daily-briefing:briefing:local~2026-06-03"),
            CompositionKind::Briefing,
            Some(EntityRef::new("briefing:local~2026-06-03")),
            vec![Section::new(SectionId::new("briefing"), blocks)],
            Salience::default(),
            generated_at,
            AbilityRef::new(ABILITY_NAME),
            CompositionMetadata {
                schema_version: SchemaVersion(ABILITY_SCHEMA_VERSION),
                generated_at,
                composition_version: CompositionVersion::new(1),
                generated_by: ABILITY_NAME.to_string(),
            },
        )
    }

    #[test]
    fn daily_briefing_bindings_project_for_tauri_surface() {
        let headline = test_block(
            "headline",
            BlockType::AccountOverview,
            json!({
                "account": {
                    "id": "briefing:local~2026-06-03",
                    "display_name": "Daily briefing",
                    "type": "Briefing",
                },
                "title": "Daily briefing",
                "summary": "available; fresh; clean",
                "trust_band": "likely_current",
                "vitals": [{
                    "label": "Sources",
                    "value": "3",
                    "source_label": "briefing",
                    "source_asof": "2026-06-03T00:00:00Z",
                    "trust_band": "likely_current",
                }],
            }),
            vec![
                display_only_binding("/title").unwrap(),
                display_only_binding("/account/id").unwrap(),
                display_only_binding("/account/display_name").unwrap(),
            ],
            0,
        );
        let schedule = test_block(
            "schedule",
            BlockType::ActionList,
            json!({
                "title": "Meetings",
                "items": [{
                    "title": "Customer checkpoint",
                    "status": "ready",
                    "trust_band": "likely_current",
                    "source_asof": "2026-06-03T00:00:00Z",
                }],
                "trust_band": "likely_current",
                "source_asof": "2026-06-03T00:00:00Z",
            }),
            vec![display_only_binding("/title").unwrap()],
            1,
        );
        let attention = test_block(
            "attention",
            BlockType::ClaimSummary,
            json!({
                "title": "Attention",
                "text": "No source-backed advisories need attention for this briefing.",
                "body": "No source-backed advisories need attention for this briefing.",
                "trust_band": "likely_current",
                "source_asof": "2026-06-03T00:00:00Z",
            }),
            vec![display_only_binding("/title").unwrap()],
            2,
        );
        let sources = test_block(
            "sources",
            BlockType::EvidenceList,
            json!({
                "title": "Sources",
                "items": [{
                    "label": "calendar",
                    "source_label": "calendar",
                    "source_asof": "2026-06-03T00:00:00Z",
                }],
            }),
            vec![display_only_binding("/title").unwrap()],
            3,
        );

        let (projection, _audits) = project_composition_for_surface(
            &test_composition(vec![headline, schedule, attention, sources]),
            &FallbackProjectionContext::new(Actor::User, SurfaceKind::TauriApp, 1),
        )
        .expect("daily briefing projection should accept producer bindings");

        assert_eq!(projection.blocks.len(), 4);
        assert!(projection.diagnostics.is_empty());
        assert!(projection
            .blocks
            .iter()
            .all(|block| block.trust_band != TrustBand::Unscored));
    }
}

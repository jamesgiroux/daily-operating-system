use chrono::{DateTime, Utc};
use dailyos_abilities_macro::ability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::abilities::composition::{
    AbilityRef, BindingRole, Block, BlockId, BlockType, Composition, CompositionDocId,
    CompositionKind, CompositionMetadata, CompositionVersion, EntityRef, FieldBinding,
    ProvenanceRef, Salience, SalienceBand, Section, SectionId, SectionLayout,
};
use crate::abilities::provenance::source_time::{parse_source_timestamp, SourceTimestampStatus};
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, Confidence, DataSource, FieldAttribution, FieldPath,
    InputsSnapshot, InvocationId, MeetingId, ProvenanceBuilder, ProvenanceBuilderConfig,
    SchemaVersion, SourceAttribution, SourceIdentifier, SourceName, SourceRef, SubjectAttribution,
    SubjectRef,
};
use crate::abilities::trust::TrustBand;
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};
use crate::services::context::{
    CompositionCommitError, CompositionProposal, MeetingCompositionProvenanceKind,
    MeetingCompositionSnapshot, MeetingCompositionSnapshotField, MeetingCompositionSnapshotQuery,
    MeetingCompositionSnapshotReadError,
};

const ABILITY_NAME: &str = "dailyos/meeting-detail";
const ABILITY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct MeetingDetailCompositionInput {
    pub schema_version: u32,
    pub meeting_token: String,
    pub meeting_id: String,
    #[serde(default)]
    pub expected_composition_version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition_id: Option<String>,
}

#[derive(Debug, Clone)]
struct NormalizedInput {
    meeting_token: String,
    meeting_id: String,
    composition_id: CompositionDocId,
    expected_composition_version: u64,
}

#[ability(
    name = "dailyos/meeting-detail",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.meeting_detail"],
    mcp_exposure = None,
    client_side_executable = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [
        "meeting.field_changed",
        "meeting_prep.status_changed",
        "meeting_transcript.processed",
        "meeting_outcome.changed",
        "meeting_entity_link.changed",
        "claim.version",
        "claim.lifecycle",
        "claim.dismissal",
        "source.freshness",
        "source.revocation"
    ], coalesce = true }
)]
pub async fn meeting_detail_composition(
    ctx: &AbilityContext<'_>,
    input: MeetingDetailCompositionInput,
) -> AbilityResult<Composition> {
    let input = normalize_input(input)?;
    let snapshot = ctx
        .services()
        .read_meeting_composition_snapshot(MeetingCompositionSnapshotQuery {
            meeting_id: input.meeting_id.clone(),
            meeting_token: input.meeting_token.clone(),
            surface: ctx.entity_context_claim_surface(),
        })
        .await
        .map_err(meeting_snapshot_error)?;

    let subject =
        SubjectAttribution::direct_confident(SubjectRef::Meeting(input.meeting_token.clone()));
    let provenance_config = provenance_config(ctx);
    let invocation_id = provenance_config.invocation_id;
    let mut provenance_builder = ProvenanceBuilder::new(provenance_config);
    provenance_builder.set_subject(subject.clone());

    let composition = build_composition(
        ctx,
        &input,
        &snapshot,
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

fn normalize_input(input: MeetingDetailCompositionInput) -> Result<NormalizedInput, AbilityError> {
    if input.schema_version != ABILITY_SCHEMA_VERSION {
        return Err(validation_error(format!(
            "unsupported schema_version `{}` for `{ABILITY_NAME}`",
            input.schema_version
        )));
    }
    if !valid_meeting_token(&input.meeting_token) {
        return Err(validation_error(
            "meeting_token must match mtg_[0-9a-f]{16,32}",
        ));
    }
    if input.meeting_id.trim().is_empty() {
        return Err(validation_error("meeting_id must be non-empty"));
    }
    let composition_id = input
        .composition_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{ABILITY_NAME}:meeting:{}", input.meeting_token));
    if composition_id != format!("{ABILITY_NAME}:meeting:{}", input.meeting_token) {
        return Err(validation_error(format!(
            "composition_id must be `{ABILITY_NAME}:meeting:{}`",
            input.meeting_token
        )));
    }
    if composition_id.chars().any(char::is_control) {
        return Err(validation_error(
            "composition_id must not contain control characters",
        ));
    }
    Ok(NormalizedInput {
        meeting_token: input.meeting_token,
        meeting_id: input.meeting_id,
        composition_id: CompositionDocId::new(composition_id),
        expected_composition_version: input.expected_composition_version,
    })
}

fn valid_meeting_token(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("mtg_") else {
        return false;
    };
    (16..=32).contains(&hex.len())
        && hex
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn build_composition(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    snapshot: &MeetingCompositionSnapshot,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Composition, AbilityError> {
    let generated_at = ctx.services().clock.now();
    let mut sections = Vec::new();
    sections.push(section(
        "headline",
        "Meeting",
        vec![build_overview_block(
            ctx,
            input,
            snapshot,
            0,
            0,
            subject,
            invocation_id,
            provenance_builder,
        )?],
        SectionLayout::Stacked,
        salience(0.96, SalienceBand::Critical, "meeting headline"),
    ));

    for (section_index, (section_id, label, layout, band, reason)) in [
        (
            "prep",
            "Prep",
            SectionLayout::Stacked,
            SalienceBand::Important,
            "meeting prep",
        ),
        (
            "recap",
            "Recap",
            SectionLayout::Stacked,
            SalienceBand::Important,
            "meeting recap",
        ),
        (
            "continuity",
            "Continuity",
            SectionLayout::Grid,
            SalienceBand::Contextual,
            "meeting continuity",
        ),
        (
            "predictions",
            "Prediction scorecard",
            SectionLayout::Stacked,
            SalienceBand::Contextual,
            "meeting predictions",
        ),
        (
            "relationship",
            "Relationship",
            SectionLayout::Grid,
            SalienceBand::Contextual,
            "meeting relationships",
        ),
        (
            "sources",
            "Sources",
            SectionLayout::Grid,
            SalienceBand::Contextual,
            "meeting sources",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let index = section_index + 1;
        let fields = snapshot_fields_for_section(snapshot, section_id);
        let block = if fields.is_empty() {
            build_empty_block(
                input,
                section_id,
                index,
                0,
                empty_title(section_id),
                empty_body(section_id),
                subject,
                invocation_id,
                provenance_builder,
            )?
        } else {
            build_fields_block(
                ctx,
                input,
                section_id,
                index,
                0,
                fields,
                subject,
                invocation_id,
                provenance_builder,
            )?
        };
        sections.push(section(
            section_id,
            label,
            vec![block],
            layout,
            salience(0.72, band, reason),
        ));
    }

    let section_count = sections.len();
    let composition = Composition::new(
        input.composition_id.clone(),
        CompositionKind::Custom {
            type_id: "dailyos/meeting-detail".to_string(),
        },
        Some(EntityRef::new(format!("meeting:{}", input.meeting_token))),
        sections,
        salience(0.9, SalienceBand::Important, "meeting detail"),
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

#[allow(clippy::too_many_arguments)]
fn build_overview_block(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    snapshot: &MeetingCompositionSnapshot,
    section_index: usize,
    block_index: usize,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let fields = snapshot_fields_for_section(snapshot, "headline");
    let source_indexes = snapshot_source_indexes(ctx, input, &fields, provenance_builder)?;
    let title = snapshot_value_text(&snapshot.title.value);
    let starts_at = snapshot
        .starts_at
        .as_ref()
        .map(|field| snapshot_value_text(&field.value));
    let ends_at = snapshot
        .ends_at
        .as_ref()
        .map(|field| snapshot_value_text(&field.value));
    let meeting_type = snapshot
        .meeting_type
        .as_ref()
        .map(|field| snapshot_value_text(&field.value));
    let trust_band = block_trust_band(fields.iter().map(|field| field.trust_band));
    let vitals = fields
        .iter()
        .map(|field| {
            json!({
                "label": field.label,
                "value": snapshot_value_text(&field.value),
                "source_label": field.source_label.as_deref(),
                "source_asof": field.source_asof.as_deref(),
                "trust_band": trust_band_label(field.trust_band),
            })
        })
        .collect::<Vec<_>>();
    let composition_block_path = format!("/sections/{section_index}/blocks/{block_index}");
    let mut block = Block::new(
        BlockId::new(block_id(input, "headline", "account_overview", "summary")),
        BlockType::AccountOverview,
        json!({
            "entity_type": "meeting",
            "account": {
                "id": format!("meeting:{}", input.meeting_token),
                "display_name": title,
                "type": "Meeting",
            },
            "meeting": {
                "token": input.meeting_token,
                "title": title,
                "starts_at": starts_at,
                "ends_at": ends_at,
                "meeting_type": meeting_type,
                "is_past": snapshot.is_past,
                "is_current": snapshot.is_current,
                "has_transcript": snapshot.has_transcript,
            },
            "title": "Meeting detail",
            "summary": "Meeting detail is grounded in prep, transcript, outcomes, and continuity sources.",
            "trust_band": trust_band_label(trust_band),
            "vitals": vitals,
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
        display_only_binding("/account/id")?,
        display_only_binding("/account/display_name")?,
    ];
    block.salience = salience(0.96, SalienceBand::Critical, "meeting summary");
    attribute_block(
        provenance_builder,
        &composition_block_path,
        subject,
        source_indexes,
    )?;
    Ok(block)
}

#[allow(clippy::too_many_arguments)]
fn build_fields_block(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    section_id: &str,
    section_index: usize,
    block_index: usize,
    fields: Vec<&MeetingCompositionSnapshotField>,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let source_indexes = snapshot_source_indexes(ctx, input, &fields, provenance_builder)?;
    let composition_block_path = format!("/sections/{section_index}/blocks/{block_index}");
    let (block_type, attributes, weight, band, reason) = match section_id {
        "recap" => (
            BlockType::ActionList,
            json!({
                "title": "Recap",
                "items": fields
                    .iter()
                    .flat_map(|field| field_action_items(field))
                    .collect::<Vec<_>>(),
                "trust_band": trust_band_label(block_trust_band(fields.iter().map(|field| field.trust_band))),
            }),
            0.78,
            SalienceBand::Important,
            "meeting recap fields",
        ),
        "predictions" => (
            BlockType::ClaimSummary,
            json!({
                "title": "Prediction scorecard",
                "text": fields.iter().map(field_summary_line).collect::<Vec<_>>().join("\n"),
                "trust_band": trust_band_label(block_trust_band(fields.iter().map(|field| field.trust_band))),
            }),
            0.62,
            SalienceBand::Contextual,
            "meeting prediction fields",
        ),
        _ => (
            BlockType::EvidenceList,
            json!({
                "title": section_title(section_id),
                "items": fields
                    .iter()
                    .flat_map(|field| snapshot_field_evidence_items(field))
                    .collect::<Vec<_>>(),
                "trust_band": trust_band_label(block_trust_band(fields.iter().map(|field| field.trust_band))),
            }),
            0.6,
            SalienceBand::Contextual,
            "meeting evidence fields",
        ),
    };
    let mut block = Block::new(
        BlockId::new(block_id(
            input,
            section_id,
            block_type.type_id(),
            "snapshot",
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
    block.salience = salience(weight, band, reason);
    attribute_block(
        provenance_builder,
        &composition_block_path,
        subject,
        source_indexes,
    )?;
    Ok(block)
}

#[allow(clippy::too_many_arguments)]
fn build_empty_block(
    input: &NormalizedInput,
    section_id: &str,
    section_index: usize,
    block_index: usize,
    title: &str,
    body: &str,
    subject: &SubjectAttribution,
    invocation_id: InvocationId,
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Block, AbilityError> {
    let composition_block_path = format!("/sections/{section_index}/blocks/{block_index}");
    let mut block = Block::new(
        BlockId::new(block_id(input, section_id, "empty_state", "empty")),
        BlockType::ClaimSummary,
        json!({
            "title": title,
            "text": body,
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
        display_only_binding("/empty_state")?,
    ];
    block.salience = salience(0.22, SalienceBand::Background, "meeting empty state");
    attribute_block(
        provenance_builder,
        &composition_block_path,
        subject,
        Vec::new(),
    )?;
    Ok(block)
}

fn snapshot_fields_for_section<'a>(
    snapshot: &'a MeetingCompositionSnapshot,
    section_id: &str,
) -> Vec<&'a MeetingCompositionSnapshotField> {
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
        "prep" => field_path.starts_with("/prep/"),
        "recap" => field_path.starts_with("/recap/"),
        "continuity" => field_path.starts_with("/continuity/"),
        "predictions" => field_path.starts_with("/predictions/"),
        "relationship" => field_path.starts_with("/relationship/"),
        "sources" => field_path.starts_with("/sources/"),
        _ => false,
    }
}

fn snapshot_source_indexes(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    fields: &[&MeetingCompositionSnapshotField],
    provenance_builder: &mut ProvenanceBuilder,
) -> Result<Vec<crate::abilities::provenance::SourceIndex>, AbilityError> {
    let mut indexes = Vec::new();
    for field in fields {
        let source = source_for_snapshot_field(ctx, input, field)?;
        let index = provenance_builder.add_source(source);
        provenance_builder.set_source_trust_band(index, visible_trust_band(field.trust_band));
        indexes.push(index);
    }
    Ok(indexes)
}

fn source_for_snapshot_field(
    ctx: &AbilityContext<'_>,
    input: &NormalizedInput,
    field: &MeetingCompositionSnapshotField,
) -> Result<SourceAttribution, AbilityError> {
    let now = ctx.services().clock.now();
    let source_asof = parse_optional_source_asof(field.source_asof.as_deref(), now);
    SourceAttribution::new(
        data_source_for_snapshot_field(field),
        vec![SourceIdentifier::Meeting {
            meeting_id: MeetingId::new(input.meeting_token.clone()),
        }],
        source_asof.unwrap_or(now),
        source_asof,
        1.0,
        None,
    )
    .map_err(|error| validation_error(format!("invalid meeting source attribution: {error}")))
}

fn parse_optional_source_asof(value: Option<&str>, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match parse_source_timestamp(value, now, None) {
        SourceTimestampStatus::Accepted(parsed)
        | SourceTimestampStatus::Implausible { parsed, .. } => Some(parsed),
        SourceTimestampStatus::Malformed(_) | SourceTimestampStatus::Missing => None,
    }
}

fn data_source_for_snapshot_field(field: &MeetingCompositionSnapshotField) -> DataSource {
    match field.provenance_kind {
        MeetingCompositionProvenanceKind::ManualUser => DataSource::User,
        MeetingCompositionProvenanceKind::Unavailable => {
            DataSource::Other(SourceName::new("meeting_source_gap"))
        }
        MeetingCompositionProvenanceKind::NonSensitiveIdentity
        | MeetingCompositionProvenanceKind::SourceField
        | MeetingCompositionProvenanceKind::SystemConfig
        | MeetingCompositionProvenanceKind::Derived => DataSource::LocalEnrichment,
    }
}

fn field_action_items(field: &MeetingCompositionSnapshotField) -> Vec<Value> {
    match &field.value {
        Value::Object(object) => {
            let mut items = Vec::new();
            if let Some(summary) = object.get("summary").and_then(Value::as_str) {
                if !summary.trim().is_empty() {
                    items.push(json!({
                        "text": summary,
                        "status": field.label,
                        "status_label": humanize_token(&field.label),
                        "trust_band": trust_band_label(field.trust_band),
                        "source_asof": field.source_asof.as_deref(),
                    }));
                }
            }
            for key in ["wins", "risks", "decisions", "actions"] {
                if let Some(values) = object.get(key).and_then(Value::as_array) {
                    for value in values.iter().take(10) {
                        items.push(json!({
                            "text": compact_item_label(value),
                            "status": key,
                            "status_label": humanize_token(key),
                            "trust_band": trust_band_label(field.trust_band),
                            "source_asof": field.source_asof.as_deref(),
                        }));
                    }
                }
            }
            items
        }
        Value::Array(items) => items
            .iter()
            .take(12)
            .map(|item| {
                json!({
                    "text": compact_item_label(item),
                    "status": field.label,
                    "status_label": humanize_token(&field.label),
                    "trust_band": trust_band_label(field.trust_band),
                    "source_asof": field.source_asof.as_deref(),
                })
            })
            .collect(),
        _ => vec![json!({
            "text": snapshot_value_text(&field.value),
            "status": field.label,
            "status_label": humanize_token(&field.label),
            "trust_band": trust_band_label(field.trust_band),
            "source_asof": field.source_asof.as_deref(),
        })],
    }
}

fn snapshot_field_evidence_items(field: &MeetingCompositionSnapshotField) -> Vec<Value> {
    let source_label = field.source_label.as_deref().unwrap_or("meeting");
    match &field.value {
        Value::Array(items) => items
            .iter()
            .take(12)
            .map(|item| {
                json!({
                    "label": evidence_label(&field.label, &compact_item_label(item)),
                    "source_label": source_label,
                    "source_asof": field.source_asof.as_deref(),
                    "trust_band": trust_band_label(field.trust_band),
                })
            })
            .collect(),
        Value::Object(_) => vec![json!({
            "label": evidence_label(&field.label, &compact_item_label(&field.value)),
            "source_label": source_label,
            "source_asof": field.source_asof.as_deref(),
            "trust_band": trust_band_label(field.trust_band),
        })],
        _ => vec![json!({
            "label": evidence_label(&field.label, &snapshot_value_text(&field.value)),
            "source_label": source_label,
            "source_asof": field.source_asof.as_deref(),
            "trust_band": trust_band_label(field.trust_band),
        })],
    }
}

/// Compose a human-facing `label: value` evidence line, humanizing each side so
/// neither a machine field name nor an enum value leaks as raw display text.
/// A value that collapses to empty yields just the humanized label.
fn evidence_label(label: &str, value: &str) -> String {
    let label = humanize_token(label);
    let value = humanize_token(value);
    if value.is_empty() {
        label
    } else {
        format!("{label}: {value}")
    }
}

fn field_summary_line(field: &&MeetingCompositionSnapshotField) -> String {
    evidence_label(&field.label, &snapshot_value_text(&field.value))
}

fn compact_item_label(value: &Value) -> String {
    let Some(object) = value.as_object() else {
        return snapshot_value_text(value);
    };
    for key in [
        "title", "text", "content", "name", "label", "status", "value",
    ] {
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

/// Convert a `snake_case` / lowercase machine token into a human-facing label.
/// Strings that already read as prose (containing a space or any uppercase) are
/// returned untouched, so real sentences are never mangled — only bare enum
/// tokens like `prediction_status` or `pending_verification` are reshaped.
fn humanize_token(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let looks_machine = trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !looks_machine {
        return trimmed.to_string();
    }
    let spaced = trimmed.replace('_', " ");
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn section_title(section_id: &str) -> &'static str {
    match section_id {
        "prep" => "Prep",
        "recap" => "Recap",
        "continuity" => "Continuity",
        "predictions" => "Prediction scorecard",
        "relationship" => "Relationship",
        "sources" => "Sources",
        _ => "Meeting detail",
    }
}

fn empty_title(section_id: &str) -> &'static str {
    match section_id {
        "prep" => "No prep context",
        "recap" => "No recap yet",
        "continuity" => "No continuity thread",
        "predictions" => "No prediction scorecard",
        "relationship" => "No linked entities",
        "sources" => "No sources",
        _ => "No meeting detail",
    }
}

fn empty_body(section_id: &str) -> &'static str {
    match section_id {
        "prep" => "No source-backed prep context is currently available for this meeting.",
        "recap" => "No processed transcript outcomes are currently available for this meeting.",
        "continuity" => "No prior meeting context is currently available.",
        "predictions" => "Prep predictions and transcript outcomes are not both available.",
        "relationship" => "No entity links are currently attached to this meeting.",
        "sources" => "No source metadata is currently available.",
        _ => "No renderable meeting fields are currently available.",
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

fn block_id(
    input: &NormalizedInput,
    section_id: &str,
    block_type: &str,
    discriminator: &str,
) -> String {
    format!(
        "{}:{}:{}:{}",
        input.composition_id.as_str(),
        section_id,
        block_type,
        discriminator
    )
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
    source_indexes: Vec<crate::abilities::provenance::SourceIndex>,
) -> Result<(), AbilityError> {
    let attribution = if source_indexes.is_empty() {
        FieldAttribution::constant(subject.clone())
    } else if source_indexes.len() == 1 {
        FieldAttribution::direct(subject.clone(), source_indexes[0])
    } else {
        FieldAttribution::computed(
            subject.clone(),
            "dailyos.meeting_detail.v1",
            source_indexes
                .into_iter()
                .map(|source_index| SourceRef::Source { source_index })
                .collect(),
            Confidence::computed(1.0).map_err(field_error)?,
        )
        .map_err(field_error)?
    };
    builder
        .attribute(
            FieldPath::new(composition_block_path).map_err(field_error)?,
            attribution,
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

fn block_trust_band(bands: impl IntoIterator<Item = TrustBand>) -> TrustBand {
    let mut result = TrustBand::LikelyCurrent;
    let mut seen = false;
    for band in bands.into_iter().map(visible_trust_band) {
        seen = true;
        result = match (result, band) {
            (TrustBand::NeedsVerification, _) | (_, TrustBand::NeedsVerification) => {
                TrustBand::NeedsVerification
            }
            (TrustBand::UseWithCaution, _) | (_, TrustBand::UseWithCaution) => {
                TrustBand::UseWithCaution
            }
            _ => TrustBand::LikelyCurrent,
        };
    }
    if seen {
        result
    } else {
        TrustBand::NeedsVerification
    }
}

fn trust_band_label(band: TrustBand) -> &'static str {
    match visible_trust_band(band) {
        TrustBand::LikelyCurrent => "likely_current",
        TrustBand::UseWithCaution => "use_with_caution",
        TrustBand::NeedsVerification | TrustBand::Unscored => "needs_verification",
    }
}

fn meeting_snapshot_error(error: MeetingCompositionSnapshotReadError) -> AbilityError {
    match error {
        MeetingCompositionSnapshotReadError::MeetingNotFound(meeting_token) => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("meeting not found for token {meeting_token}"),
        },
        MeetingCompositionSnapshotReadError::ReadFailed(message) => AbilityError {
            kind: AbilityErrorKind::HardError("meeting_detail_snapshot_read".to_string()),
            message,
        },
    }
}

fn composition_commit_error(error: CompositionCommitError) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::HardError("meeting_detail_composition_commit".to_string()),
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
        kind: AbilityErrorKind::HardError("meeting_detail_composition_provenance".to_string()),
        message: format!("provenance error: {error}"),
    }
}

fn field_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("field path error: {error}"))
}

fn block_error(error: impl std::fmt::Display) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::HardError("meeting_detail_composition_block".to_string()),
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
    use crate::abilities::trust::TrustBand;
    use crate::abilities::{
        project_composition_for_surface, FallbackProjectionContext, SurfaceKind,
    };

    fn test_invocation_id() -> InvocationId {
        InvocationId::new(uuid::Uuid::from_u128(
            0x1234_5678_90ab_cdef_1122_3344_5566_7788,
        ))
    }

    fn test_provenance_ref(block_index: usize) -> ProvenanceRef {
        ProvenanceRef::new(
            test_invocation_id(),
            FieldPath::new(format!("/sections/0/blocks/{block_index}")).unwrap(),
        )
    }

    #[test]
    fn humanize_token_reshapes_machine_tokens_but_preserves_prose() {
        assert_eq!(humanize_token("prediction_status"), "Prediction status");
        assert_eq!(humanize_token("pending_verification"), "Pending verification");
        assert_eq!(humanize_token("wins"), "Wins");
        // Anything already reading as prose (space or uppercase) is untouched.
        assert_eq!(
            humanize_token("Legal review is starting"),
            "Legal review is starting"
        );
        assert_eq!(humanize_token(""), "");
    }

    #[test]
    fn evidence_label_never_leaks_raw_snake_pairs() {
        assert_eq!(
            evidence_label("prediction_status", "pending_verification"),
            "Prediction status: Pending verification"
        );
        // An empty value collapses to just the humanized label — no trailing colon.
        assert_eq!(evidence_label("risks", ""), "Risks");
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
            CompositionDocId::new("dailyos/meeting-detail:meeting:mtg_0123456789abcdef"),
            CompositionKind::EntityPage,
            Some(EntityRef::new("meeting:mtg_0123456789abcdef")),
            vec![Section::new(SectionId::new("meeting"), blocks)],
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
    fn meeting_detail_bindings_project_for_tauri_surface() {
        let overview = test_block(
            "overview",
            BlockType::AccountOverview,
            json!({
                "entity_type": "meeting",
                "account": {
                    "id": "meeting:mtg_0123456789abcdef",
                    "display_name": "Meeting fixture",
                    "type": "Meeting",
                },
                "meeting": {
                    "token": "mtg_0123456789abcdef",
                    "title": "Meeting fixture",
                },
                "title": "Meeting detail",
                "summary": "Meeting detail fixture",
                "vitals": [{
                    "label": "State",
                    "value": "ready",
                    "source_label": "meeting",
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
        let recap = test_block(
            "recap",
            BlockType::ActionList,
            json!({
                "title": "Recap",
                "items": [{
                    "text": "Follow-up fixture",
                    "status": "actions",
                    "trust_band": "likely_current",
                    "source_asof": "2026-06-03T00:00:00Z",
                }],
                "trust_band": "likely_current",
            }),
            vec![display_only_binding("/title").unwrap()],
            1,
        );
        let empty = test_block(
            "empty",
            BlockType::ClaimSummary,
            json!({
                "title": "No linked entities",
                "body": "No entity links are currently attached.",
                "text": "No entity links are currently attached.",
                "empty_state": true,
                "trust_band": "needs_verification",
            }),
            vec![
                display_only_binding("/title").unwrap(),
                display_only_binding("/body").unwrap(),
                display_only_binding("/empty_state").unwrap(),
            ],
            2,
        );

        let (projection, _audits) = project_composition_for_surface(
            &test_composition(vec![overview, recap, empty]),
            &FallbackProjectionContext::new(Actor::User, SurfaceKind::TauriApp, 1),
        )
        .expect("meeting detail projection should accept producer bindings");

        assert_eq!(projection.blocks.len(), 3);
        assert!(projection
            .blocks
            .iter()
            .all(|block| block.trust_band != TrustBand::Unscored));
    }
}

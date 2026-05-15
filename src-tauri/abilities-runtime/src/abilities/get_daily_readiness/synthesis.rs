use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::prompts;
use crate::abilities::get_entity_context::GetEntityContextOutput;
use crate::abilities::prepare_meeting::{
    AttendeeContext, ChangeMarker, MeetingBrief, MeetingSummary, OpenLoop, SuggestedOutcome, Topic,
};
use crate::abilities::provenance::MaskReason;
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, CompositionId, Confidence, ContextEntryId, DataSource,
    EntityId, FieldAttribution, FieldPath, GleanDownstream, MeetingId, Provenance,
    ProvenanceBuilder, ProvenanceBuilderConfig, ProvenanceWarning, SchemaVersion,
    SourceAttribution, SourceIdentifier, SourceName, SourceRef, SubjectAttribution, SubjectRef,
};
use crate::abilities::{AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind};
use crate::abilities::{AbilityResult, Actor as RegistryActor};
use crate::intelligence::provider::{ModelTier, ProviderError};
use crate::sensitivity::{renderable_claim_text_with_value, RenderActor, RenderSurface};
use crate::services::context::{
    DailyReadinessContextSnapshot, DailyReadinessCoverageWarningSnapshot,
    DailyReadinessMeetingSnapshot, DailyReadinessOpenLoopSnapshot, DailyReadinessRiskSnapshot,
    DailyReadinessSignalSnapshot, DailyReadinessSubjectSnapshot,
};
use crate::types::{
    prompt_input_sensitivity_name_allowed, ClaimSensitivity, ClaimState, EntityContextText,
    IntelligenceClaim, SurfacingState, TemporalScope,
};

const ABILITY_NAME: &str = "get_daily_readiness";
const ABILITY_SCHEMA_VERSION: u32 = 1;
const PREPARE_MEETING_SCHEMA_VERSION: u32 = 1;
const GET_ENTITY_CONTEXT_SCHEMA_VERSION: u32 = 2;
const CHILD_PROMPT_DEFAULT_SENSITIVITY: &str = "internal";
const CHILD_PROMPT_ACTIVE_LIFECYCLE: &str = "active";
pub const JUDGE_MODEL: &str = "claude-sonnet-4-6";

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DailyReadinessInput {
    pub schema_version: SchemaVersion,
    pub workspace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Private Evaluate-mode seam for fixture-driven context building.
    /// This is intentionally omitted from the public ability schema.
    #[serde(default, skip_deserializing, skip_serializing)]
    #[schemars(skip)]
    context: Option<DailyReadinessContext>,
}

impl DailyReadinessInput {
    pub fn public(
        workspace_id: impl Into<String>,
        date: Option<String>,
        schema_version: SchemaVersion,
    ) -> Self {
        Self {
            schema_version,
            workspace_id: workspace_id.into(),
            date,
            context: None,
        }
    }

    #[doc(hidden)]
    pub fn evaluate_with_context(context: DailyReadinessContext, schema_version: SchemaVersion) -> Self {
        Self {
            workspace_id: context.workspace_scope.clone(),
            date: Some(context.date.clone()),
            schema_version,
            context: Some(context),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DailyReadinessContext {
    pub workspace_scope: String,
    pub date: String,
    #[serde(default)]
    pub meetings: Vec<DailyReadinessMeetingSeed>,
    #[serde(default)]
    pub tracked_subjects: Vec<DailyReadinessSubject>,
    #[serde(default)]
    pub prepare_meeting_children: Vec<ComposedPrepareMeetingOutput>,
    #[serde(default)]
    pub entity_context_children: Vec<ComposedEntityContextOutput>,
    #[serde(default)]
    pub overnight_changes: Vec<DailyReadinessOvernightChange>,
    #[serde(default)]
    pub risk_shifts: Vec<DailyReadinessRiskShift>,
    #[serde(default)]
    pub open_loops: Vec<DailyReadinessOpenLoop>,
    #[serde(default)]
    pub coverage_warnings: Vec<DailyReadinessCoverageWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DailyReadinessSubject {
    pub kind: String,
    pub id: String,
    pub display_name: String,
    pub workspace_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DailyReadinessMeetingSeed {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<String>,
    pub workspace_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ComposedPrepareMeetingOutput {
    pub meeting_id: String,
    pub workspace_scope: String,
    pub cache_dedupe_key: String,
    #[serde(
        default = "default_child_prompt_sensitivity",
        skip_serializing_if = "is_child_prompt_default_sensitivity"
    )]
    pub sensitivity: String,
    pub output: MeetingBrief,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComposedEntityContextOutput {
    pub subject: DailyReadinessSubject,
    pub cache_dedupe_key: String,
    pub output: GetEntityContextOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct DailyReadinessOvernightChange {
    pub id: String,
    pub subject: DailyReadinessSubject,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
    pub observed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_asof: Option<String>,
    pub data_source: String,
    pub lifecycle: String,
    pub confidence: f32,
    pub sensitivity: String,
    pub workspace_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct DailyReadinessRiskShift {
    pub id: String,
    pub subject: DailyReadinessSubject,
    pub direction: String,
    pub evidence_summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
    pub observed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_asof: Option<String>,
    pub data_source: String,
    pub lifecycle: String,
    pub confidence: f32,
    pub sensitivity: String,
    pub workspace_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct DailyReadinessOpenLoop {
    pub id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub subject: DailyReadinessSubject,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
    pub observed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_asof: Option<String>,
    pub data_source: String,
    pub lifecycle: String,
    pub confidence: f32,
    pub sensitivity: String,
    pub workspace_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DailyReadinessCoverageWarning {
    pub kind: String,
    pub message: String,
    pub count: u32,
    pub workspace_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct DailyReadinessMeeting {
    pub meeting: MeetingSummary,
    pub workspace_scope: String,
    #[serde(default)]
    pub topics: Vec<Topic>,
    #[serde(default)]
    pub attendee_context: Vec<AttendeeContext>,
    #[serde(default)]
    pub open_loops: Vec<OpenLoop>,
    #[serde(default)]
    pub suggested_outcomes: Vec<SuggestedOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct DailyReadiness {
    pub narrative: String,
    pub meetings_today: Vec<DailyReadinessMeeting>,
    pub overnight_changes: Vec<DailyReadinessOvernightChange>,
    pub risk_shifts: Vec<DailyReadinessRiskShift>,
    pub open_loops: Vec<DailyReadinessOpenLoop>,
    pub coverage_warnings: Vec<DailyReadinessCoverageWarning>,
    pub schema_version: SchemaVersion,
}

pub async fn build_daily_readiness(
    ctx: &AbilityContext<'_>,
    input: DailyReadinessInput,
) -> AbilityResult<DailyReadiness> {
    validate_schema_version(input.schema_version)?;
    let date = input.date.clone().unwrap_or_else(|| today(ctx));
    let mut context = match input.context.clone() {
        Some(context) => context,
        None => DailyReadinessContext::from_workspace_date(ctx, &input.workspace_id, &date).await?,
    };
    context.validate_for_input(&input.workspace_id, &date)?;
    context.retain_parent_prompt_allowed(render_actor_for_context(ctx));
    build_daily_readiness_from_context(ctx, input.schema_version, context).await
}

async fn build_daily_readiness_from_context(
    ctx: &AbilityContext<'_>,
    schema_version: SchemaVersion,
    context: DailyReadinessContext,
) -> AbilityResult<DailyReadiness> {
    let composed_children = ComposedChildren::synthetic(ctx, &context)?;
    let prompt_context = PromptContext::from_context(&context);
    let prompt_sections = prompt_context.sections().map_err(json_error)?;
    let rendered = prompts::render_prompt(&prompt_context, &prompt_sections, schema_version.0)
        .map_err(json_error)?;
    let completion = ctx
        .provider
        .complete(rendered.prompt_input(), ModelTier::Synthesis)
        .await
        .map_err(provider_error)?;
    let raw = parse_completion(&completion.text)?;

    DailyReadinessAssembler::new(ctx, schema_version, context, composed_children).assemble(
        raw,
        prompts::fingerprint_from_completion(&completion, &rendered),
    )
}

#[derive(Debug, Serialize)]
struct PromptContext {
    workspace_scope: String,
    date: String,
    meeting_topics: Vec<PromptMeetingTopic>,
    meeting_attendees: Vec<PromptMeetingAttendee>,
    meeting_open_loops: Vec<PromptMeetingOpenLoop>,
    meeting_outcomes: Vec<PromptMeetingOutcome>,
    entity_contexts: Vec<PromptEntityContext>,
    risk_directions: Vec<PromptRiskDirection>,
    risk_summaries: Vec<PromptRiskSummary>,
    open_loop_texts: Vec<PromptOpenLoopText>,
    overnight_summaries: Vec<PromptOvernightSummary>,
    coverage_warnings: Vec<DailyReadinessCoverageWarning>,
}

#[derive(Debug, Serialize)]
struct PromptMeetingTopic {
    meeting_id: String,
    title: String,
    detail: String,
    subject: Value,
}

#[derive(Debug, Serialize)]
struct PromptMeetingAttendee {
    meeting_id: String,
    attendee: String,
    context: String,
    subject: Value,
}

#[derive(Debug, Serialize)]
struct PromptMeetingOpenLoop {
    meeting_id: String,
    description: String,
    owner: Option<String>,
    subject: Value,
}

#[derive(Debug, Serialize)]
struct PromptMeetingOutcome {
    meeting_id: String,
    outcome: String,
    rationale: String,
    subject: Value,
}

#[derive(Debug, Serialize)]
struct PromptEntityContext {
    subject: DailyReadinessSubject,
    entries: Vec<PromptEntityContextEntry>,
}

#[derive(Debug, Serialize)]
struct PromptEntityContextEntry {
    id: String,
    title: String,
    content: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct PromptRiskDirection {
    id: String,
    subject: DailyReadinessSubject,
    direction: String,
}

#[derive(Debug, Serialize)]
struct PromptRiskSummary {
    id: String,
    subject: DailyReadinessSubject,
    summary: String,
}

#[derive(Debug, Serialize)]
struct PromptOpenLoopText {
    id: String,
    subject: DailyReadinessSubject,
    text: String,
}

#[derive(Debug, Serialize)]
struct PromptOvernightSummary {
    id: String,
    subject: DailyReadinessSubject,
    summary: String,
}

impl PromptContext {
    fn from_context(context: &DailyReadinessContext) -> Self {
        let mut meeting_topics = Vec::new();
        let mut meeting_attendees = Vec::new();
        let mut meeting_open_loops = Vec::new();
        let mut meeting_outcomes = Vec::new();
        for child in &context.prepare_meeting_children {
            for topic in &child.output.topics {
                meeting_topics.push(PromptMeetingTopic {
                    meeting_id: child.meeting_id.clone(),
                    title: topic.title.clone(),
                    detail: topic.detail.clone(),
                    subject: json_subject(&topic.subject),
                });
            }
            for attendee_context in &child.output.attendee_context {
                meeting_attendees.push(PromptMeetingAttendee {
                    meeting_id: child.meeting_id.clone(),
                    attendee: attendee_context.attendee.clone(),
                    context: attendee_context.context.clone(),
                    subject: json_subject(&attendee_context.subject),
                });
            }
            for open_loop in &child.output.open_loops {
                meeting_open_loops.push(PromptMeetingOpenLoop {
                    meeting_id: child.meeting_id.clone(),
                    description: open_loop.description.clone(),
                    owner: open_loop.owner.clone(),
                    subject: json_subject(&open_loop.subject),
                });
            }
            for outcome in &child.output.suggested_outcomes {
                meeting_outcomes.push(PromptMeetingOutcome {
                    meeting_id: child.meeting_id.clone(),
                    outcome: outcome.outcome.clone(),
                    rationale: outcome.rationale.clone(),
                    subject: json_subject(&outcome.subject),
                });
            }
        }

        Self {
            workspace_scope: context.workspace_scope.clone(),
            date: context.date.clone(),
            meeting_topics,
            meeting_attendees,
            meeting_open_loops,
            meeting_outcomes,
            entity_contexts: context
                .entity_context_children
                .iter()
                .map(|child| PromptEntityContext {
                    subject: child.subject.clone(),
                    entries: child
                        .output
                        .entries
                        .iter()
                        .map(|entry| PromptEntityContextEntry {
                            id: entry.id.clone(),
                            title: entry.title.as_str().to_string(),
                            content: entry.content.as_str().to_string(),
                            created_at: entry.created_at.clone(),
                            updated_at: entry.updated_at.clone(),
                        })
                        .collect(),
                })
                .collect(),
            risk_directions: context
                .risk_shifts
                .iter()
                .map(|risk| PromptRiskDirection {
                    id: risk.id.clone(),
                    subject: risk.subject.clone(),
                    direction: risk.direction.clone(),
                })
                .collect(),
            risk_summaries: context
                .risk_shifts
                .iter()
                .map(|risk| PromptRiskSummary {
                    id: risk.id.clone(),
                    subject: risk.subject.clone(),
                    summary: risk.evidence_summary.clone(),
                })
                .collect(),
            open_loop_texts: context
                .open_loops
                .iter()
                .map(|open_loop| PromptOpenLoopText {
                    id: open_loop.id.clone(),
                    subject: open_loop.subject.clone(),
                    text: open_loop.text.clone(),
                })
                .collect(),
            overnight_summaries: context
                .overnight_changes
                .iter()
                .map(|change| PromptOvernightSummary {
                    id: change.id.clone(),
                    subject: change.subject.clone(),
                    summary: change.summary.clone(),
                })
                .collect(),
            coverage_warnings: context.coverage_warnings.clone(),
        }
    }

    fn sections(&self) -> Result<prompts::PromptSections, serde_json::Error> {
        Ok(prompts::PromptSections {
            meeting_topics: serde_json::to_value(&self.meeting_topics)?,
            meeting_attendees: serde_json::to_value(&self.meeting_attendees)?,
            meeting_open_loops: serde_json::to_value(&self.meeting_open_loops)?,
            meeting_outcomes: serde_json::to_value(&self.meeting_outcomes)?,
            entity_contexts: serde_json::to_value(&self.entity_contexts)?,
            risk_directions: serde_json::to_value(&self.risk_directions)?,
            risk_summaries: serde_json::to_value(&self.risk_summaries)?,
            open_loop_texts: serde_json::to_value(&self.open_loop_texts)?,
            overnight_summaries: serde_json::to_value(&self.overnight_summaries)?,
            coverage_warnings: serde_json::to_value(&self.coverage_warnings)?,
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawDailyReadiness {
    narrative: String,
}

fn parse_completion(text: &str) -> Result<RawDailyReadiness, AbilityError> {
    let trimmed = text.trim();
    let json_text = trimmed
        .strip_prefix("```json")
        .and_then(|rest| rest.strip_suffix("```"))
        .or_else(|| {
            trimmed
                .strip_prefix("```")
                .and_then(|rest| rest.strip_suffix("```"))
        })
        .map(str::trim)
        .unwrap_or(trimmed);
    serde_json::from_str(json_text).map_err(|error| AbilityError {
        kind: AbilityErrorKind::Validation,
        message: format!("get_daily_readiness provider response was not valid JSON: {error}"),
    })
}

struct ComposedChildren {
    prepare_meeting: Vec<ComposedChildProvenance>,
    entity_context: Vec<ComposedChildProvenance>,
}

struct ComposedChildProvenance {
    composition_id: CompositionId,
    provenance: Provenance,
}

impl ComposedChildren {
    fn synthetic(
        ctx: &AbilityContext<'_>,
        context: &DailyReadinessContext,
    ) -> Result<Self, AbilityError> {
        let prepare_meeting = context
            .prepare_meeting_children
            .iter()
            .map(|child| {
                let composition_id = CompositionId::new(format!(
                    "prepare_meeting:{}:{}",
                    child.workspace_scope, child.meeting_id
                ));
                let provenance =
                    synthetic_prepare_meeting_provenance(ctx, &child.output, &composition_id)?;
                Ok(ComposedChildProvenance {
                    composition_id,
                    provenance,
                })
            })
            .collect::<Result<Vec<_>, AbilityError>>()?;
        let entity_context = context
            .entity_context_children
            .iter()
            .map(|child| {
                let composition_id = CompositionId::new(format!(
                    "get_entity_context:{}:{}:{}",
                    child.subject.workspace_scope, child.subject.kind, child.subject.id
                ));
                let provenance = synthetic_entity_context_provenance(ctx, child, &composition_id)?;
                Ok(ComposedChildProvenance {
                    composition_id,
                    provenance,
                })
            })
            .collect::<Result<Vec<_>, AbilityError>>()?;
        Ok(Self {
            prepare_meeting,
            entity_context,
        })
    }

    fn source_refs(&self) -> Vec<SourceRef> {
        self.prepare_meeting
            .iter()
            .chain(self.entity_context.iter())
            .map(|child| SourceRef::Child {
                composition_id: child.composition_id.clone(),
                field_path: FieldPath::root(),
            })
            .collect()
    }
}

struct DailyReadinessAssembler<'a> {
    ctx: &'a AbilityContext<'a>,
    schema_version: SchemaVersion,
    context: DailyReadinessContext,
    children: ComposedChildren,
    source_indices: BTreeMap<String, crate::abilities::provenance::SourceIndex>,
}

impl<'a> DailyReadinessAssembler<'a> {
    fn new(
        ctx: &'a AbilityContext<'a>,
        schema_version: SchemaVersion,
        context: DailyReadinessContext,
        children: ComposedChildren,
    ) -> Self {
        Self {
            ctx,
            schema_version,
            context,
            children,
            source_indices: BTreeMap::new(),
        }
    }

    fn assemble(
        mut self,
        raw: RawDailyReadiness,
        fingerprint: crate::abilities::provenance::PromptFingerprint,
    ) -> AbilityResult<DailyReadiness> {
        let mut builder = ProvenanceBuilder::new(config_for(
            self.ctx,
            ABILITY_NAME,
            self.schema_version,
            AbilityCategory::Read,
            AbilityVersion::new(0, 1),
        ));
        builder.set_prompt_fingerprint(fingerprint);
        let subject = SubjectAttribution::direct_confident(SubjectRef::Global);
        builder.set_subject(subject.clone());
        self.add_mask_warnings(&mut builder);

        for child in &self.children.prepare_meeting {
            builder
                .compose(child.composition_id.clone(), child.provenance.clone())
                .map_err(map_provenance_error)?;
        }
        for child in &self.children.entity_context {
            builder
                .compose(child.composition_id.clone(), child.provenance.clone())
                .map_err(map_provenance_error)?;
        }

        let output = DailyReadiness {
            narrative: raw.narrative,
            meetings_today: self
                .context
                .prepare_meeting_children
                .iter()
                .map(DailyReadinessMeeting::from)
                .collect(),
            overnight_changes: self.context.overnight_changes.clone(),
            risk_shifts: self.context.risk_shifts.clone(),
            open_loops: self.context.open_loops.clone(),
            coverage_warnings: self.context.coverage_warnings.clone(),
            schema_version: self.schema_version,
        };

        let mut narrative_refs = self.children.source_refs();
        narrative_refs.extend(self.context_source_refs(&mut builder)?);
        if narrative_refs.is_empty() {
            let source_index = self.workspace_context_source(&mut builder)?;
            narrative_refs.push(SourceRef::Source { source_index });
        }
        builder
            .attribute(
                FieldPath::new("/narrative").map_err(map_field_error)?,
                FieldAttribution::llm_synthesis(
                    subject.clone(),
                    narrative_refs,
                    Confidence::provider_reported(0.8).map_err(map_field_error)?,
                    None,
                )
                .map_err(map_field_error)?,
            )
            .map_err(map_provenance_error)?;
        builder
            .attribute(
                FieldPath::new("/schema_version").map_err(map_field_error)?,
                FieldAttribution::constant(subject.clone()),
            )
            .map_err(map_provenance_error)?;

        self.attribute_meetings(&mut builder, &subject, &output)?;
        self.attribute_snapshot_sections(&mut builder, &subject, &output)?;

        builder.finalize(output).map_err(map_provenance_error)
    }

    fn add_mask_warnings(&self, builder: &mut ProvenanceBuilder) {
        for warning in &self.context.coverage_warnings {
            match warning.kind.as_str() {
                "source_revoked" => {
                    builder.add_warning(ProvenanceWarning::Masked {
                        reason: MaskReason::SourceRevoked,
                    });
                }
                "private_prompt_input_filtered" => {
                    builder.add_warning(ProvenanceWarning::Masked {
                        reason: MaskReason::Sensitive,
                    });
                }
                _ => {}
            }
        }
    }

    fn attribute_meetings(
        &mut self,
        builder: &mut ProvenanceBuilder,
        subject: &SubjectAttribution,
        output: &DailyReadiness,
    ) -> Result<(), AbilityError> {
        if output.meetings_today.is_empty() {
            builder
                .attribute(
                    FieldPath::new("/meetings_today").map_err(map_field_error)?,
                    FieldAttribution::constant(subject.clone()),
                )
                .map_err(map_provenance_error)?;
            return Ok(());
        }

        for (index, child) in self.context.prepare_meeting_children.iter().enumerate() {
            let Some(composed) = self
                .children
                .prepare_meeting
                .iter()
                .find(|composed| composed.composition_id.as_str().contains(&child.meeting_id))
            else {
                continue;
            };
            let attribution = FieldAttribution::composed(
                subject.clone(),
                composed.composition_id.clone(),
                FieldPath::root(),
                Confidence::composed_min(1.0).map_err(map_field_error)?,
            )
            .map_err(map_field_error)?;
            builder
                .attribute_subtree(
                    FieldPath::new(format!("/meetings_today/{index}")).map_err(map_field_error)?,
                    attribution,
                )
                .map_err(map_provenance_error)?;
        }
        Ok(())
    }

    fn attribute_snapshot_sections(
        &mut self,
        builder: &mut ProvenanceBuilder,
        subject: &SubjectAttribution,
        output: &DailyReadiness,
    ) -> Result<(), AbilityError> {
        self.attribute_direct_section(
            builder,
            subject,
            "/overnight_changes",
            &output.overnight_changes,
            SnapshotKind::OvernightChange,
        )?;
        self.attribute_direct_section(
            builder,
            subject,
            "/risk_shifts",
            &output.risk_shifts,
            SnapshotKind::RiskShift,
        )?;
        self.attribute_direct_section(
            builder,
            subject,
            "/open_loops",
            &output.open_loops,
            SnapshotKind::OpenLoop,
        )?;
        if output.coverage_warnings.is_empty() {
            builder
                .attribute(
                    FieldPath::new("/coverage_warnings").map_err(map_field_error)?,
                    FieldAttribution::constant(subject.clone()),
                )
                .map_err(map_provenance_error)?;
        } else {
            let source_index = self.workspace_context_source(builder)?;
            builder
                .attribute_subtree(
                    FieldPath::new("/coverage_warnings").map_err(map_field_error)?,
                    FieldAttribution::direct(subject.clone(), source_index),
                )
                .map_err(map_provenance_error)?;
        }
        Ok(())
    }

    fn attribute_direct_section<T: SnapshotAttribution>(
        &mut self,
        builder: &mut ProvenanceBuilder,
        subject: &SubjectAttribution,
        path: &str,
        items: &[T],
        kind: SnapshotKind,
    ) -> Result<(), AbilityError> {
        if items.is_empty() {
            builder
                .attribute(
                    FieldPath::new(path).map_err(map_field_error)?,
                    FieldAttribution::constant(subject.clone()),
                )
                .map_err(map_provenance_error)?;
            return Ok(());
        }

        for (index, item) in items.iter().enumerate() {
            let source_index = self.ensure_snapshot_source(builder, item, kind)?;
            builder
                .attribute_subtree(
                    FieldPath::new(format!("{path}/{index}")).map_err(map_field_error)?,
                    FieldAttribution::direct(subject.clone(), source_index),
                )
                .map_err(map_provenance_error)?;
        }
        Ok(())
    }

    fn context_source_refs(
        &mut self,
        builder: &mut ProvenanceBuilder,
    ) -> Result<Vec<SourceRef>, AbilityError> {
        let mut refs = Vec::new();
        for change in self.context.overnight_changes.clone() {
            refs.push(SourceRef::Source {
                source_index: self.ensure_snapshot_source(
                    builder,
                    &change,
                    SnapshotKind::OvernightChange,
                )?,
            });
        }
        for risk in self.context.risk_shifts.clone() {
            refs.push(SourceRef::Source {
                source_index: self.ensure_snapshot_source(
                    builder,
                    &risk,
                    SnapshotKind::RiskShift,
                )?,
            });
        }
        for open_loop in self.context.open_loops.clone() {
            refs.push(SourceRef::Source {
                source_index: self.ensure_snapshot_source(
                    builder,
                    &open_loop,
                    SnapshotKind::OpenLoop,
                )?,
            });
        }
        Ok(refs)
    }

    fn workspace_context_source(
        &mut self,
        builder: &mut ProvenanceBuilder,
    ) -> Result<crate::abilities::provenance::SourceIndex, AbilityError> {
        let key = format!(
            "workspace-context:{}:{}",
            self.context.workspace_scope, self.context.date
        );
        if let Some(index) = self.source_indices.get(&key) {
            return Ok(*index);
        }
        let observed_at = self.ctx.services().clock.now();
        let source = SourceAttribution::new(
            DataSource::LocalEnrichment,
            vec![SourceIdentifier::Entity {
                entity_id: EntityId::new(format!("workspace:{}", self.context.workspace_scope)),
                field: Some(format!("daily_readiness:{}", self.context.date)),
            }],
            observed_at,
            Some(observed_at),
            1.0,
            None,
        )
        .map_err(source_error)?;
        let index = builder.add_source(source);
        self.source_indices.insert(key, index);
        Ok(index)
    }

    fn ensure_snapshot_source<T: SnapshotAttribution>(
        &mut self,
        builder: &mut ProvenanceBuilder,
        item: &T,
        kind: SnapshotKind,
    ) -> Result<crate::abilities::provenance::SourceIndex, AbilityError> {
        let key = format!(
            "{}:{}:{}",
            item.workspace_scope(),
            kind.as_str(),
            item.source_id()
        );
        if let Some(index) = self.source_indices.get(&key) {
            return Ok(*index);
        }
        let source = SourceAttribution::new(
            data_source(item.data_source()),
            vec![kind.source_identifier(item.source_id())],
            parse_rfc3339(item.observed_at()).unwrap_or_else(|| self.ctx.services().clock.now()),
            item.source_asof()
                .and_then(parse_rfc3339)
                .or_else(|| parse_rfc3339(item.observed_at())),
            item.confidence().clamp(0.0, 1.0),
            None,
        )
        .map_err(source_error)?;
        let index = builder.add_source(source);
        self.source_indices.insert(key, index);
        Ok(index)
    }
}

#[derive(Clone, Copy)]
enum SnapshotKind {
    OvernightChange,
    RiskShift,
    OpenLoop,
}

impl SnapshotKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::OvernightChange => "overnight_change",
            Self::RiskShift => "risk_shift",
            Self::OpenLoop => "open_loop",
        }
    }

    fn source_identifier(self, id: &str) -> SourceIdentifier {
        match self {
            Self::OvernightChange | Self::RiskShift => SourceIdentifier::Signal {
                signal_id: crate::abilities::provenance::SignalId::new(id.to_string()),
            },
            Self::OpenLoop => SourceIdentifier::UserEntry {
                entry_id: ContextEntryId::new(id.to_string()),
            },
        }
    }
}

trait SnapshotAttribution {
    fn source_id(&self) -> &str;
    fn workspace_scope(&self) -> &str;
    fn data_source(&self) -> &str;
    fn observed_at(&self) -> &str;
    fn source_asof(&self) -> Option<&str>;
    fn confidence(&self) -> f32;
}

impl SnapshotAttribution for DailyReadinessOvernightChange {
    fn source_id(&self) -> &str {
        &self.id
    }

    fn workspace_scope(&self) -> &str {
        &self.workspace_scope
    }

    fn data_source(&self) -> &str {
        &self.data_source
    }

    fn observed_at(&self) -> &str {
        &self.observed_at
    }

    fn source_asof(&self) -> Option<&str> {
        self.source_asof.as_deref()
    }

    fn confidence(&self) -> f32 {
        self.confidence
    }
}

impl SnapshotAttribution for DailyReadinessRiskShift {
    fn source_id(&self) -> &str {
        &self.id
    }

    fn workspace_scope(&self) -> &str {
        &self.workspace_scope
    }

    fn data_source(&self) -> &str {
        &self.data_source
    }

    fn observed_at(&self) -> &str {
        &self.observed_at
    }

    fn source_asof(&self) -> Option<&str> {
        self.source_asof.as_deref()
    }

    fn confidence(&self) -> f32 {
        self.confidence
    }
}

impl SnapshotAttribution for DailyReadinessOpenLoop {
    fn source_id(&self) -> &str {
        &self.id
    }

    fn workspace_scope(&self) -> &str {
        &self.workspace_scope
    }

    fn data_source(&self) -> &str {
        &self.data_source
    }

    fn observed_at(&self) -> &str {
        &self.observed_at
    }

    fn source_asof(&self) -> Option<&str> {
        self.source_asof.as_deref()
    }

    fn confidence(&self) -> f32 {
        self.confidence
    }
}

impl DailyReadinessContext {
    async fn from_workspace_date(
        ctx: &AbilityContext<'_>,
        workspace_id: &str,
        date: &str,
    ) -> Result<Self, AbilityError> {
        let snapshot = ctx
            .services()
            .read_daily_readiness_context(workspace_id.to_string(), date.to_string())
            .await
            .map_err(|error| AbilityError {
                kind: AbilityErrorKind::HardError("daily_readiness_context_read".into()),
                message: error,
            })?;
        Self::from_snapshot(snapshot)
    }

    fn from_snapshot(snapshot: DailyReadinessContextSnapshot) -> Result<Self, AbilityError> {
        let workspace_scope = snapshot.workspace_scope;
        let date = snapshot.date;
        let meetings = snapshot
            .meetings
            .into_iter()
            .map(DailyReadinessMeetingSeed::from)
            .collect::<Vec<_>>();
        let tracked_subjects = snapshot
            .tracked_subjects
            .into_iter()
            .map(DailyReadinessSubject::from)
            .collect::<Vec<_>>();
        let prepare_meeting_children = meetings
            .iter()
            .map(synthetic_prepare_meeting_child)
            .collect::<Vec<_>>();
        let entity_context_children = tracked_subjects
            .iter()
            .map(synthetic_entity_context_child)
            .collect::<Vec<_>>();

        Ok(Self {
            workspace_scope,
            date,
            meetings,
            tracked_subjects,
            prepare_meeting_children,
            entity_context_children,
            overnight_changes: snapshot
                .overnight_changes
                .into_iter()
                .map(DailyReadinessOvernightChange::from)
                .collect(),
            risk_shifts: snapshot
                .risk_shifts
                .into_iter()
                .map(DailyReadinessRiskShift::from)
                .collect(),
            open_loops: snapshot
                .open_loops
                .into_iter()
                .map(DailyReadinessOpenLoop::from)
                .collect(),
            coverage_warnings: snapshot
                .coverage_warnings
                .into_iter()
                .map(DailyReadinessCoverageWarning::from)
                .collect(),
        })
    }

    fn validate_for_input(&self, workspace_id: &str, date: &str) -> Result<(), AbilityError> {
        if self.workspace_scope != workspace_id {
            return Err(subject_not_owned(format!(
                "daily readiness context workspace `{}` does not match input workspace `{workspace_id}`",
                self.workspace_scope
            )));
        }
        if self.date != date {
            return Err(validation_error(format!(
                "daily readiness context date `{}` does not match input date `{date}`",
                self.date
            )));
        }

        for meeting in &self.meetings {
            validate_workspace(
                "meeting",
                &meeting.id,
                &meeting.workspace_scope,
                workspace_id,
            )?;
        }
        for subject in &self.tracked_subjects {
            validate_workspace(
                "subject",
                &subject.id,
                &subject.workspace_scope,
                workspace_id,
            )?;
        }
        for child in &self.prepare_meeting_children {
            validate_workspace(
                "prepare_meeting child",
                &child.meeting_id,
                &child.workspace_scope,
                workspace_id,
            )?;
        }
        for child in &self.entity_context_children {
            validate_workspace(
                "get_entity_context child",
                &child.subject.id,
                &child.subject.workspace_scope,
                workspace_id,
            )?;
        }
        for change in &self.overnight_changes {
            validate_workspace(
                "overnight change",
                &change.id,
                &change.workspace_scope,
                workspace_id,
            )?;
            validate_workspace(
                "overnight change subject",
                &change.subject.id,
                &change.subject.workspace_scope,
                workspace_id,
            )?;
        }
        for risk in &self.risk_shifts {
            validate_workspace("risk shift", &risk.id, &risk.workspace_scope, workspace_id)?;
            validate_workspace(
                "risk shift subject",
                &risk.subject.id,
                &risk.subject.workspace_scope,
                workspace_id,
            )?;
        }
        for open_loop in &self.open_loops {
            validate_workspace(
                "open loop",
                &open_loop.id,
                &open_loop.workspace_scope,
                workspace_id,
            )?;
            validate_workspace(
                "open loop subject",
                &open_loop.subject.id,
                &open_loop.subject.workspace_scope,
                workspace_id,
            )?;
        }
        Ok(())
    }

    fn retain_parent_prompt_allowed(&mut self, render_actor: RenderActor) {
        let workspace_scope = self.workspace_scope.clone();
        let mut filtered_private = 0u32;
        let mut filtered_revoked = 0u32;

        self.overnight_changes.retain_mut(|change| {
            match render_prompt_text(
                &snapshot_claim(
                    &change.id,
                    "daily_readiness_overnight_change",
                    &change.summary,
                    &change.subject,
                    &change.sensitivity,
                    &change.data_source,
                    change.source_ref.as_deref(),
                    change.source_asof.as_deref(),
                    &change.observed_at,
                    change.confidence,
                ),
                &change.summary,
                &change.lifecycle,
                &render_actor,
            ) {
                PromptTextDecision::Allow(rendered) => {
                    change.summary = rendered;
                    true
                }
                PromptTextDecision::Private => {
                    filtered_private += 1;
                    false
                }
                PromptTextDecision::Revoked => {
                    filtered_revoked += 1;
                    false
                }
            }
        });
        self.risk_shifts.retain_mut(|risk| {
            match render_prompt_text(
                &snapshot_claim(
                    &risk.id,
                    "daily_readiness_risk_shift",
                    &risk.evidence_summary,
                    &risk.subject,
                    &risk.sensitivity,
                    &risk.data_source,
                    risk.source_ref.as_deref(),
                    risk.source_asof.as_deref(),
                    &risk.observed_at,
                    risk.confidence,
                ),
                &risk.evidence_summary,
                &risk.lifecycle,
                &render_actor,
            ) {
                PromptTextDecision::Allow(rendered) => {
                    risk.evidence_summary = rendered;
                    true
                }
                PromptTextDecision::Private => {
                    filtered_private += 1;
                    false
                }
                PromptTextDecision::Revoked => {
                    filtered_revoked += 1;
                    false
                }
            }
        });
        self.open_loops.retain_mut(|open_loop| {
            match render_prompt_text(
                &snapshot_claim(
                    &open_loop.id,
                    "daily_readiness_open_loop",
                    &open_loop.text,
                    &open_loop.subject,
                    &open_loop.sensitivity,
                    &open_loop.data_source,
                    open_loop.source_ref.as_deref(),
                    open_loop.source_asof.as_deref(),
                    &open_loop.observed_at,
                    open_loop.confidence,
                ),
                &open_loop.text,
                &open_loop.lifecycle,
                &render_actor,
            ) {
                PromptTextDecision::Allow(rendered) => {
                    open_loop.text = rendered;
                    true
                }
                PromptTextDecision::Private => {
                    filtered_private += 1;
                    false
                }
                PromptTextDecision::Revoked => {
                    filtered_revoked += 1;
                    false
                }
            }
        });

        for child in &mut self.prepare_meeting_children {
            let meeting_id = child.meeting_id.clone();
            let sensitivity = child.sensitivity.clone();

            let mut topic_index = 0usize;
            child.output.topics.retain_mut(|topic| {
                let item_id = format!("prepare_meeting:{meeting_id}:topics:{topic_index}");
                topic_index += 1;
                match render_prepare_topic_for_parent_prompt(
                    &item_id,
                    topic,
                    &sensitivity,
                    &render_actor,
                ) {
                    PromptItemDecision::Allow => true,
                    PromptItemDecision::Private => {
                        filtered_private += 1;
                        false
                    }
                    PromptItemDecision::Revoked => {
                        filtered_revoked += 1;
                        false
                    }
                }
            });

            let mut attendee_index = 0usize;
            child.output.attendee_context.retain_mut(|attendee_context| {
                let item_id =
                    format!("prepare_meeting:{meeting_id}:attendee_context:{attendee_index}");
                attendee_index += 1;
                match render_prepare_attendee_for_parent_prompt(
                    &item_id,
                    attendee_context,
                    &sensitivity,
                    &render_actor,
                ) {
                    PromptItemDecision::Allow => true,
                    PromptItemDecision::Private => {
                        filtered_private += 1;
                        false
                    }
                    PromptItemDecision::Revoked => {
                        filtered_revoked += 1;
                        false
                    }
                }
            });

            let mut open_loop_index = 0usize;
            child.output.open_loops.retain_mut(|open_loop| {
                let item_id = format!("prepare_meeting:{meeting_id}:open_loops:{open_loop_index}");
                open_loop_index += 1;
                match render_prepare_open_loop_for_parent_prompt(
                    &item_id,
                    open_loop,
                    &sensitivity,
                    &render_actor,
                ) {
                    PromptItemDecision::Allow => true,
                    PromptItemDecision::Private => {
                        filtered_private += 1;
                        false
                    }
                    PromptItemDecision::Revoked => {
                        filtered_revoked += 1;
                        false
                    }
                }
            });

            let mut outcome_index = 0usize;
            child.output.suggested_outcomes.retain_mut(|outcome| {
                let item_id =
                    format!("prepare_meeting:{meeting_id}:suggested_outcomes:{outcome_index}");
                outcome_index += 1;
                match render_prepare_outcome_for_parent_prompt(
                    &item_id,
                    outcome,
                    &sensitivity,
                    &render_actor,
                ) {
                    PromptItemDecision::Allow => true,
                    PromptItemDecision::Private => {
                        filtered_private += 1;
                        false
                    }
                    PromptItemDecision::Revoked => {
                        filtered_revoked += 1;
                        false
                    }
                }
            });
        }

        for child in &mut self.entity_context_children {
            child.output.entries.retain_mut(|entry| {
                match render_entity_entry_for_parent_prompt(entry, &render_actor) {
                    PromptItemDecision::Allow => true,
                    PromptItemDecision::Private => {
                        filtered_private += 1;
                        false
                    }
                    PromptItemDecision::Revoked => {
                        filtered_revoked += 1;
                        false
                    }
                }
            });
        }

        if filtered_private > 0 {
            self.coverage_warnings.push(DailyReadinessCoverageWarning {
                kind: "private_prompt_input_filtered".into(),
                message: "Some readiness inputs were omitted from narrative synthesis by the prompt-input sensitivity gate.".into(),
                count: filtered_private,
                workspace_scope: workspace_scope.clone(),
            });
        }
        if filtered_revoked > 0 {
            self.coverage_warnings.push(DailyReadinessCoverageWarning {
                kind: "source_revoked".into(),
                message:
                    "Some readiness inputs were omitted because their source lifecycle is revoked."
                        .into(),
                count: filtered_revoked,
                workspace_scope,
            });
        }
    }
}

impl From<DailyReadinessMeetingSnapshot> for DailyReadinessMeetingSeed {
    fn from(value: DailyReadinessMeetingSnapshot) -> Self {
        Self {
            id: value.id,
            title: value.title,
            starts_at: value.starts_at,
            ends_at: value.ends_at,
            workspace_scope: value.workspace_scope,
        }
    }
}

impl From<DailyReadinessSubjectSnapshot> for DailyReadinessSubject {
    fn from(value: DailyReadinessSubjectSnapshot) -> Self {
        Self {
            kind: value.kind,
            id: value.id,
            display_name: value.display_name,
            workspace_scope: value.workspace_scope,
        }
    }
}

impl From<DailyReadinessSignalSnapshot> for DailyReadinessOvernightChange {
    fn from(value: DailyReadinessSignalSnapshot) -> Self {
        Self {
            id: value.id,
            subject: DailyReadinessSubject::from(value.subject),
            summary: value.summary,
            source_ref: value.source_ref,
            observed_at: value.observed_at,
            source_asof: value.source_asof,
            data_source: value.data_source,
            lifecycle: value.lifecycle,
            confidence: value.confidence,
            sensitivity: value.sensitivity,
            workspace_scope: value.workspace_scope,
        }
    }
}

impl From<DailyReadinessRiskSnapshot> for DailyReadinessRiskShift {
    fn from(value: DailyReadinessRiskSnapshot) -> Self {
        Self {
            id: value.id,
            subject: DailyReadinessSubject::from(value.subject),
            direction: value.direction,
            evidence_summary: value.evidence_summary,
            source_ref: value.source_ref,
            observed_at: value.observed_at,
            source_asof: value.source_asof,
            data_source: value.data_source,
            lifecycle: value.lifecycle,
            confidence: value.confidence,
            sensitivity: value.sensitivity,
            workspace_scope: value.workspace_scope,
        }
    }
}

impl From<DailyReadinessOpenLoopSnapshot> for DailyReadinessOpenLoop {
    fn from(value: DailyReadinessOpenLoopSnapshot) -> Self {
        Self {
            id: value.id,
            text: value.text,
            owner: value.owner,
            subject: DailyReadinessSubject::from(value.subject),
            due_date: value.due_date,
            source_ref: value.source_ref,
            observed_at: value.observed_at,
            source_asof: value.source_asof,
            data_source: value.data_source,
            lifecycle: value.lifecycle,
            confidence: value.confidence,
            sensitivity: value.sensitivity,
            workspace_scope: value.workspace_scope,
        }
    }
}

impl From<DailyReadinessCoverageWarningSnapshot> for DailyReadinessCoverageWarning {
    fn from(value: DailyReadinessCoverageWarningSnapshot) -> Self {
        Self {
            kind: value.kind,
            message: value.message,
            count: value.count,
            workspace_scope: value.workspace_scope,
        }
    }
}

impl From<&ComposedPrepareMeetingOutput> for DailyReadinessMeeting {
    fn from(value: &ComposedPrepareMeetingOutput) -> Self {
        Self {
            meeting: value.output.meeting.clone(),
            workspace_scope: value.workspace_scope.clone(),
            topics: value.output.topics.clone(),
            attendee_context: value.output.attendee_context.clone(),
            open_loops: value.output.open_loops.clone(),
            suggested_outcomes: value.output.suggested_outcomes.clone(),
        }
    }
}

fn synthetic_prepare_meeting_child(
    meeting: &DailyReadinessMeetingSeed,
) -> ComposedPrepareMeetingOutput {
    let output = MeetingBrief {
        meeting: MeetingSummary {
            id: meeting.id.clone(),
            title: meeting.title.clone(),
            starts_at: meeting.starts_at.clone(),
            ends_at: meeting.ends_at.clone(),
            attendees: Vec::new(),
        },
        topics: Vec::new(),
        attendee_context: Vec::new(),
        open_loops: Vec::new(),
        what_changed_since_last: Vec::<ChangeMarker>::new(),
        suggested_outcomes: Vec::new(),
        schema_version: SchemaVersion(PREPARE_MEETING_SCHEMA_VERSION),
    };
    ComposedPrepareMeetingOutput {
        meeting_id: meeting.id.clone(),
        workspace_scope: meeting.workspace_scope.clone(),
        cache_dedupe_key: format!("prepare_meeting:{}:{}", meeting.workspace_scope, meeting.id),
        sensitivity: default_child_prompt_sensitivity(),
        output,
    }
}

fn synthetic_entity_context_child(subject: &DailyReadinessSubject) -> ComposedEntityContextOutput {
    ComposedEntityContextOutput {
        subject: subject.clone(),
        cache_dedupe_key: format!(
            "get_entity_context:{}:{}:{}",
            subject.workspace_scope, subject.kind, subject.id
        ),
        output: GetEntityContextOutput {
            entries: Vec::new(),
            trajectory: None,
        },
    }
}

fn synthetic_prepare_meeting_provenance(
    ctx: &AbilityContext<'_>,
    output: &MeetingBrief,
    _composition_id: &CompositionId,
) -> Result<Provenance, AbilityError> {
    let mut builder = ProvenanceBuilder::new(config_for(
        ctx,
        "prepare_meeting",
        SchemaVersion(PREPARE_MEETING_SCHEMA_VERSION),
        AbilityCategory::Transform,
        AbilityVersion::new(0, 1),
    ));
    let subject =
        SubjectAttribution::direct_confident(SubjectRef::Meeting(output.meeting.id.clone()));
    builder.set_subject(subject.clone());
    let source_index = builder.add_source(meeting_source(ctx, &output.meeting)?);
    builder
        .attribute_subtree(
            FieldPath::new("/meeting").map_err(map_field_error)?,
            FieldAttribution::direct(subject.clone(), source_index),
        )
        .map_err(map_provenance_error)?;
    for path in [
        "/topics",
        "/attendee_context",
        "/open_loops",
        "/what_changed_since_last",
        "/suggested_outcomes",
        "/schema_version",
    ] {
        builder
            .attribute(
                FieldPath::new(path).map_err(map_field_error)?,
                FieldAttribution::constant(subject.clone()),
            )
            .map_err(map_provenance_error)?;
    }
    builder
        .finalize(output.clone())
        .map(|child| child.into_parts().1)
        .map_err(map_provenance_error)
}

fn synthetic_entity_context_provenance(
    ctx: &AbilityContext<'_>,
    child: &ComposedEntityContextOutput,
    _composition_id: &CompositionId,
) -> Result<Provenance, AbilityError> {
    let mut builder = ProvenanceBuilder::new(config_for(
        ctx,
        "get_entity_context",
        SchemaVersion(GET_ENTITY_CONTEXT_SCHEMA_VERSION),
        AbilityCategory::Read,
        AbilityVersion::new(1, 0),
    ));
    let subject = SubjectAttribution::direct_confident(subject_ref(&child.subject));
    builder.set_subject(subject.clone());
    builder
        .attribute(
            FieldPath::new("/entries").map_err(map_field_error)?,
            FieldAttribution::constant(subject),
        )
        .map_err(map_provenance_error)?;
    builder
        .finalize(child.output.clone())
        .map(|child| child.into_parts().1)
        .map_err(map_provenance_error)
}

fn meeting_source(
    ctx: &AbilityContext<'_>,
    meeting: &MeetingSummary,
) -> Result<SourceAttribution, AbilityError> {
    let now = ctx.services().clock.now();
    let source_asof = meeting.starts_at.as_deref().and_then(parse_rfc3339);
    SourceAttribution::new(
        DataSource::Google,
        vec![SourceIdentifier::Meeting {
            meeting_id: MeetingId::new(meeting.id.clone()),
        }],
        source_asof.unwrap_or(now),
        source_asof,
        1.0,
        None,
    )
    .map_err(source_error)
}

enum PromptTextDecision {
    Allow(String),
    Private,
    Revoked,
}

enum PromptItemDecision {
    Allow,
    Private,
    Revoked,
}

struct ChildPromptText<'a> {
    item_id: &'a str,
    claim_type: &'a str,
    field_path: &'a str,
    value: &'a str,
    subject_kind: &'a str,
    subject_id: &'a str,
    sensitivity: &'a str,
}

fn render_prepare_topic_for_parent_prompt(
    item_id: &str,
    topic: &mut Topic,
    sensitivity: &str,
    actor: &RenderActor,
) -> PromptItemDecision {
    match render_prepare_child_text(
        item_id,
        "meeting_topic",
        "/title",
        &topic.title,
        &topic.subject,
        sensitivity,
        actor,
    ) {
        PromptTextDecision::Allow(rendered) => topic.title = rendered,
        PromptTextDecision::Private => return PromptItemDecision::Private,
        PromptTextDecision::Revoked => return PromptItemDecision::Revoked,
    }
    match render_prepare_child_text(
        item_id,
        "meeting_topic",
        "/detail",
        &topic.detail,
        &topic.subject,
        sensitivity,
        actor,
    ) {
        PromptTextDecision::Allow(rendered) => {
            topic.detail = rendered;
            PromptItemDecision::Allow
        }
        PromptTextDecision::Private => PromptItemDecision::Private,
        PromptTextDecision::Revoked => PromptItemDecision::Revoked,
    }
}

fn render_prepare_attendee_for_parent_prompt(
    item_id: &str,
    attendee_context: &mut AttendeeContext,
    sensitivity: &str,
    actor: &RenderActor,
) -> PromptItemDecision {
    match render_prepare_child_text(
        item_id,
        "attendee_context",
        "/attendee",
        &attendee_context.attendee,
        &attendee_context.subject,
        sensitivity,
        actor,
    ) {
        PromptTextDecision::Allow(rendered) => attendee_context.attendee = rendered,
        PromptTextDecision::Private => return PromptItemDecision::Private,
        PromptTextDecision::Revoked => return PromptItemDecision::Revoked,
    }
    match render_prepare_child_text(
        item_id,
        "attendee_context",
        "/context",
        &attendee_context.context,
        &attendee_context.subject,
        sensitivity,
        actor,
    ) {
        PromptTextDecision::Allow(rendered) => {
            attendee_context.context = rendered;
            PromptItemDecision::Allow
        }
        PromptTextDecision::Private => PromptItemDecision::Private,
        PromptTextDecision::Revoked => PromptItemDecision::Revoked,
    }
}

fn render_prepare_open_loop_for_parent_prompt(
    item_id: &str,
    open_loop: &mut OpenLoop,
    sensitivity: &str,
    actor: &RenderActor,
) -> PromptItemDecision {
    match render_prepare_child_text(
        item_id,
        "open_loop",
        "/description",
        &open_loop.description,
        &open_loop.subject,
        sensitivity,
        actor,
    ) {
        PromptTextDecision::Allow(rendered) => open_loop.description = rendered,
        PromptTextDecision::Private => return PromptItemDecision::Private,
        PromptTextDecision::Revoked => return PromptItemDecision::Revoked,
    }
    if let Some(owner) = open_loop.owner.as_mut() {
        match render_prepare_child_text(
            item_id,
            "open_loop",
            "/owner",
            owner,
            &open_loop.subject,
            sensitivity,
            actor,
        ) {
            PromptTextDecision::Allow(rendered) => *owner = rendered,
            PromptTextDecision::Private => return PromptItemDecision::Private,
            PromptTextDecision::Revoked => return PromptItemDecision::Revoked,
        }
    }
    PromptItemDecision::Allow
}

fn render_prepare_outcome_for_parent_prompt(
    item_id: &str,
    outcome: &mut SuggestedOutcome,
    sensitivity: &str,
    actor: &RenderActor,
) -> PromptItemDecision {
    match render_prepare_child_text(
        item_id,
        "suggested_outcome",
        "/outcome",
        &outcome.outcome,
        &outcome.subject,
        sensitivity,
        actor,
    ) {
        PromptTextDecision::Allow(rendered) => outcome.outcome = rendered,
        PromptTextDecision::Private => return PromptItemDecision::Private,
        PromptTextDecision::Revoked => return PromptItemDecision::Revoked,
    }
    match render_prepare_child_text(
        item_id,
        "suggested_outcome",
        "/rationale",
        &outcome.rationale,
        &outcome.subject,
        sensitivity,
        actor,
    ) {
        PromptTextDecision::Allow(rendered) => {
            outcome.rationale = rendered;
            PromptItemDecision::Allow
        }
        PromptTextDecision::Private => PromptItemDecision::Private,
        PromptTextDecision::Revoked => PromptItemDecision::Revoked,
    }
}

fn render_prepare_child_text(
    item_id: &str,
    claim_type: &str,
    field_path: &str,
    value: &str,
    subject: &crate::abilities::prepare_meeting::BriefSubjectRef,
    sensitivity: &str,
    actor: &RenderActor,
) -> PromptTextDecision {
    render_child_prompt_text(
        ChildPromptText {
            item_id,
            claim_type,
            field_path,
            value,
            subject_kind: &subject.kind,
            subject_id: &subject.id,
            sensitivity,
        },
        actor,
    )
}

fn render_entity_entry_for_parent_prompt(
    entry: &mut crate::types::EntityContextEntry,
    actor: &RenderActor,
) -> PromptItemDecision {
    let title_sensitivity = entity_context_text_sensitivity(&entry.title).to_string();
    let title_value = entry.title.as_str().to_string();
    match render_child_prompt_text(
        ChildPromptText {
            item_id: &entry.id,
            claim_type: "entity_context_entry",
            field_path: "/title",
            value: &title_value,
            subject_kind: &entry.entity_type,
            subject_id: &entry.entity_id,
            sensitivity: &title_sensitivity,
        },
        actor,
    ) {
        PromptTextDecision::Allow(rendered) => set_entity_context_text(&mut entry.title, rendered),
        PromptTextDecision::Private => return PromptItemDecision::Private,
        PromptTextDecision::Revoked => return PromptItemDecision::Revoked,
    }

    let content_sensitivity = entity_context_text_sensitivity(&entry.content).to_string();
    let content_value = entry.content.as_str().to_string();
    match render_child_prompt_text(
        ChildPromptText {
            item_id: &entry.id,
            claim_type: "entity_context_entry",
            field_path: "/content",
            value: &content_value,
            subject_kind: &entry.entity_type,
            subject_id: &entry.entity_id,
            sensitivity: &content_sensitivity,
        },
        actor,
    ) {
        PromptTextDecision::Allow(rendered) => {
            set_entity_context_text(&mut entry.content, rendered);
            PromptItemDecision::Allow
        }
        PromptTextDecision::Private => PromptItemDecision::Private,
        PromptTextDecision::Revoked => PromptItemDecision::Revoked,
    }
}

fn entity_context_text_sensitivity(text: &EntityContextText) -> &'static str {
    match text {
        EntityContextText::Claim(rendered) => sensitivity_name(&rendered.policy.sensitivity),
        EntityContextText::Plain(_) => CHILD_PROMPT_DEFAULT_SENSITIVITY,
    }
}

fn set_entity_context_text(text: &mut EntityContextText, rendered: String) {
    match text {
        EntityContextText::Claim(claim_text) => claim_text.text = rendered,
        EntityContextText::Plain(value) => *value = rendered,
    }
}

fn render_child_prompt_text(input: ChildPromptText<'_>, actor: &RenderActor) -> PromptTextDecision {
    let claim = child_prompt_claim(&input);
    render_prompt_text(&claim, input.value, CHILD_PROMPT_ACTIVE_LIFECYCLE, actor)
}

fn child_prompt_claim(input: &ChildPromptText<'_>) -> IntelligenceClaim {
    let sensitivity = if input.sensitivity.trim().is_empty() {
        CHILD_PROMPT_DEFAULT_SENSITIVITY
    } else {
        input.sensitivity
    };
    IntelligenceClaim {
        id: format!("{}:{}", input.item_id, input.field_path),
        subject_ref: json!({
            "kind": input.subject_kind,
            "id": input.subject_id,
        })
        .to_string(),
        claim_type: input.claim_type.to_string(),
        field_path: Some(input.field_path.to_string()),
        topic_key: None,
        text: input.value.to_string(),
        dedup_key: format!("prompt-child:{}:{}", input.item_id, input.field_path),
        item_hash: None,
        actor: "agent:get_daily_readiness".to_string(),
        data_source: "composed_child".to_string(),
        source_ref: None,
        source_asof: None,
        observed_at: "1970-01-01T00:00:00Z".to_string(),
        created_at: "1970-01-01T00:00:00Z".to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: None,
        claim_state: ClaimState::Active,
        surfacing_state: SurfacingState::Active,
        demotion_reason: None,
        reactivated_at: None,
        retraction_reason: None,
        expires_at: None,
        superseded_by: None,
        trust_score: None,
        trust_computed_at: None,
        trust_version: None,
        thread_id: None,
        temporal_scope: TemporalScope::State,
        sensitivity: sensitivity_from_name(sensitivity),
        verification_state: crate::sensitivity::ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    }
}

fn render_prompt_text(
    claim: &IntelligenceClaim,
    value: &str,
    lifecycle: &str,
    actor: &RenderActor,
) -> PromptTextDecision {
    if lifecycle.trim().eq_ignore_ascii_case("revoked") {
        return PromptTextDecision::Revoked;
    }
    if !prompt_input_sensitivity_name_allowed(sensitivity_name(&claim.sensitivity)) {
        return PromptTextDecision::Private;
    }
    renderable_claim_text_with_value(claim, value, RenderSurface::TauriBriefingPrep, actor)
        .map(|rendered| PromptTextDecision::Allow(rendered.text))
        .unwrap_or(PromptTextDecision::Private)
}

#[allow(clippy::too_many_arguments)]
fn snapshot_claim(
    id: &str,
    claim_type: &str,
    text: &str,
    subject: &DailyReadinessSubject,
    sensitivity: &str,
    data_source: &str,
    source_ref: Option<&str>,
    source_asof: Option<&str>,
    observed_at: &str,
    confidence: f32,
) -> IntelligenceClaim {
    IntelligenceClaim {
        id: id.to_string(),
        subject_ref: json!({
            "kind": subject.kind,
            "id": subject.id,
        })
        .to_string(),
        claim_type: claim_type.to_string(),
        field_path: None,
        topic_key: None,
        text: text.to_string(),
        dedup_key: format!(
            "daily_readiness:{}:{}:{}:{}",
            subject.workspace_scope, subject.kind, subject.id, id
        ),
        item_hash: None,
        actor: "agent:get_daily_readiness".to_string(),
        data_source: data_source.to_string(),
        source_ref: source_ref.map(str::to_string),
        source_asof: source_asof.map(str::to_string),
        observed_at: observed_at.to_string(),
        created_at: observed_at.to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: None,
        claim_state: ClaimState::Active,
        surfacing_state: SurfacingState::Active,
        demotion_reason: None,
        reactivated_at: None,
        retraction_reason: None,
        expires_at: None,
        superseded_by: None,
        trust_score: Some(confidence.clamp(0.0, 1.0) as f64),
        trust_computed_at: None,
        trust_version: None,
        thread_id: None,
        temporal_scope: TemporalScope::State,
        sensitivity: sensitivity_from_name(sensitivity),
        verification_state: crate::sensitivity::ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    }
}

fn sensitivity_from_name(value: &str) -> ClaimSensitivity {
    match value.trim().to_ascii_lowercase().as_str() {
        "public" => ClaimSensitivity::Public,
        "internal" => ClaimSensitivity::Internal,
        "confidential" => ClaimSensitivity::Confidential,
        "user_only" | "user-only" => ClaimSensitivity::UserOnly,
        _ => ClaimSensitivity::Confidential,
    }
}

fn default_child_prompt_sensitivity() -> String {
    CHILD_PROMPT_DEFAULT_SENSITIVITY.to_string()
}

fn is_child_prompt_default_sensitivity(value: &str) -> bool {
    value
        .trim()
        .eq_ignore_ascii_case(CHILD_PROMPT_DEFAULT_SENSITIVITY)
}

fn sensitivity_name(value: &ClaimSensitivity) -> &'static str {
    match value {
        ClaimSensitivity::Public => "public",
        ClaimSensitivity::Internal => "internal",
        ClaimSensitivity::Confidential => "confidential",
        ClaimSensitivity::UserOnly => "user_only",
    }
}

fn render_actor_for_context(ctx: &AbilityContext<'_>) -> RenderActor {
    match &ctx.actor {
        RegistryActor::User => RenderActor::user(ctx.services().actor, Some(ctx.services().actor)),
        RegistryActor::Agent => RenderActor::agent("agent:get_daily_readiness"),
        RegistryActor::Admin => RenderActor {
            actor: "admin".into(),
            user_id: None,
        },
        RegistryActor::System => RenderActor {
            actor: "system".into(),
            user_id: None,
        },
        RegistryActor::SurfaceClient { .. } => todo!("W1-B+ wiring for Actor::SurfaceClient"),
    }
}

fn validate_workspace(
    label: &str,
    id: &str,
    actual: &str,
    expected: &str,
) -> Result<(), AbilityError> {
    if actual == expected {
        Ok(())
    } else {
        Err(subject_not_owned(format!(
            "{label} `{id}` belongs to workspace `{actual}`, not `{expected}`"
        )))
    }
}

fn subject_ref(subject: &DailyReadinessSubject) -> SubjectRef {
    match subject.kind.as_str() {
        "account" => SubjectRef::Account(subject.id.clone()),
        "project" => SubjectRef::Project(subject.id.clone()),
        "person" => SubjectRef::Person(subject.id.clone()),
        "meeting" => SubjectRef::Meeting(subject.id.clone()),
        "user" => SubjectRef::User(subject.id.clone()),
        _ => SubjectRef::Global,
    }
}

fn json_subject<T: Serialize>(subject: &T) -> Value {
    serde_json::to_value(subject).unwrap_or_else(|_| json!({"kind": "unknown", "id": "unknown"}))
}

fn data_source(value: &str) -> DataSource {
    match value.trim().to_ascii_lowercase().as_str() {
        "user" | "human" => DataSource::User,
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

fn today(ctx: &AbilityContext<'_>) -> String {
    ctx.services().clock.now().date_naive().to_string()
}

fn validate_schema_version(schema_version: SchemaVersion) -> Result<(), AbilityError> {
    if schema_version.0 == ABILITY_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(validation_error(format!(
            "unsupported schema_version `{}` for `{ABILITY_NAME}`",
            schema_version.0
        )))
    }
}

fn config_for(
    ctx: &AbilityContext<'_>,
    ability_name: &str,
    schema_version: SchemaVersion,
    category: AbilityCategory,
    ability_version: AbilityVersion,
) -> ProvenanceBuilderConfig {
    let mut config = ProvenanceBuilderConfig::new(ability_name, ctx.services().clock.now());
    config.ability_version = ability_version;
    config.ability_schema_version = schema_version;
    config.actor = provenance_actor(ctx.actor.clone());
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = category;
    config
}

fn provenance_actor(actor: RegistryActor) -> crate::abilities::provenance::Actor {
    match actor {
        RegistryActor::User => crate::abilities::provenance::Actor::User,
        RegistryActor::Agent => crate::abilities::provenance::Actor::Agent {
            name: "agent".into(),
            version: "unknown".into(),
        },
        RegistryActor::Admin => crate::abilities::provenance::Actor::Human {
            role: "admin".into(),
            id: "admin".into(),
        },
        RegistryActor::System => crate::abilities::provenance::Actor::System {
            component: "ability-runtime".into(),
        },
        RegistryActor::SurfaceClient { .. } => {
            todo!("W1-B+ wiring for Actor::SurfaceClient")
        }
    }
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|parsed| parsed.with_timezone(&Utc))
        .ok()
}

fn provider_error(error: ProviderError) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Capability,
        message: error.to_string(),
    }
}

fn validation_error(message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: message.into(),
    }
}

fn subject_not_owned(message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::HardError("subject_not_owned".into()),
        message: message.into(),
    }
}

fn json_error(error: serde_json::Error) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: error.to_string(),
    }
}

fn source_error(error: crate::abilities::provenance::SourceAttributionError) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: error.to_string(),
    }
}

fn map_field_error(error: crate::abilities::provenance::FieldAttributionError) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: error.to_string(),
    }
}

fn map_provenance_error(error: crate::abilities::provenance::ProvenanceError) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: error.to_string(),
    }
}

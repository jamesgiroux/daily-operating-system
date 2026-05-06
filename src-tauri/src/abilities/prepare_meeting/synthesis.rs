use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::prompts;
use crate::abilities::get_entity_context::{
    get_entity_context, ContextDepth, GetEntityContextInput,
};
use crate::abilities::provenance::source_time::{parse_source_timestamp, SourceTimestampStatus};
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, CompositionId, Confidence, DataSource, FieldAttribution,
    FieldPath, GleanDownstream, MaskReason, MeetingId, ProvenanceBuilder, ProvenanceBuilderConfig,
    ProvenanceWarning, SchemaVersion, SourceAttribution, SourceIdentifier, SourceRef,
    SubjectAttribution, SubjectBindingKind, SubjectFitAssessment, SubjectRef,
};
use crate::abilities::{metadata_for_claim_type, AbilityCategory};
use crate::abilities::{AbilityContext, AbilityError, AbilityErrorKind, AbilityResult};
use crate::abilities::{Actor as RegistryActor, ClaimType};
use crate::db::claim_invalidation::SubjectRef as ClaimSubjectRef;
use crate::db::claims::{ClaimSensitivity, IntelligenceClaim, TemporalScope};
use crate::intelligence::provider::{ModelTier, ProviderError};
use crate::services::context::PrepareMeetingContextSnapshot;
use crate::types::EntityContextEntry;

const ABILITY_NAME: &str = "prepare_meeting";
const SUBJECT_CONFIDENCE_FLOOR: f32 = 0.65;
const STALE_SOURCE_DAYS: i64 = 180;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct PrepareMeetingInput {
    pub meeting_id: String,
    #[serde(default = "default_depth")]
    pub depth: u8,
    #[serde(default = "default_true")]
    pub include_open_loops: bool,
    #[serde(default = "default_schema_version")]
    pub schema_version: SchemaVersion,
    /// Private Evaluate-mode seam for fixture-driven context building.
    /// This is intentionally omitted from the public ability schema.
    #[serde(default, skip_deserializing, skip_serializing)]
    #[schemars(skip)]
    context: Option<MeetingBriefContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct MeetingBriefContext {
    pub meeting: MeetingSummary,
    #[serde(default)]
    pub evidence: Vec<EvidenceSource>,
    #[serde(default)]
    pub entity_contexts: Vec<EntityContextSeed>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MeetingSummary {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<String>,
    #[serde(default)]
    pub attendees: Vec<MeetingAttendee>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MeetingAttendee {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct BriefSubjectRef {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct EvidenceSource {
    pub id: String,
    pub subject: BriefSubjectRef,
    pub claim_type: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_asof: Option<String>,
    pub observed_at: String,
    #[serde(default = "default_data_source")]
    pub data_source: String,
    #[serde(default = "default_active_lifecycle")]
    pub lifecycle: String,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    #[serde(default = "default_temporal_scope_name")]
    pub temporal_scope: String,
    #[serde(default = "default_sensitivity_name")]
    pub sensitivity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct EntityContextSeed {
    pub subject: BriefSubjectRef,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct MeetingBrief {
    pub meeting: MeetingSummary,
    pub topics: Vec<Topic>,
    pub attendee_context: Vec<AttendeeContext>,
    pub open_loops: Vec<OpenLoop>,
    pub what_changed_since_last: Vec<ChangeMarker>,
    pub suggested_outcomes: Vec<SuggestedOutcome>,
    pub schema_version: SchemaVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Topic {
    pub title: String,
    pub detail: String,
    pub subject: BriefSubjectRef,
    pub temporal_scope: BriefTemporalScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AttendeeContext {
    pub attendee: String,
    pub context: String,
    pub subject: BriefSubjectRef,
    pub temporal_scope: BriefTemporalScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OpenLoop {
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub subject: BriefSubjectRef,
    pub temporal_scope: BriefTemporalScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChangeMarker {
    pub description: String,
    pub subject: BriefSubjectRef,
    pub temporal_scope: BriefTemporalScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SuggestedOutcome {
    pub outcome: String,
    pub rationale: String,
    pub subject: BriefSubjectRef,
    pub temporal_scope: BriefTemporalScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BriefTemporalScope {
    State,
    PointInTime {
        occurred_at: String,
    },
    Trend {
        window_start: String,
        window_end: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ClaimDraft {
    pub claim_type: String,
    pub text: String,
    pub subject: BriefSubjectRef,
    pub temporal_scope: BriefTemporalScope,
    pub sensitivity: String,
}

pub async fn build_meeting_brief(
    ctx: &AbilityContext<'_>,
    input: PrepareMeetingInput,
) -> AbilityResult<MeetingBrief> {
    let context = match input.context.clone() {
        Some(context) => context,
        None => MeetingBriefContext::from_meeting_id(ctx, &input.meeting_id).await?,
    };
    build_meeting_brief_from_context(ctx, input, context).await
}

async fn build_meeting_brief_from_context(
    ctx: &AbilityContext<'_>,
    input: PrepareMeetingInput,
    context: MeetingBriefContext,
) -> AbilityResult<MeetingBrief> {
    if context.meeting.id != input.meeting_id {
        return Err(validation_error(
            "meeting context id does not match prepare_meeting input",
        ));
    }

    let prompt_allowed_subjects = meeting_scope_source_subjects(&context);
    let mut composed_children = Vec::new();
    for entity_context in &context.entity_contexts {
        let depth = context_depth(input.depth);
        let child = get_entity_context(
            ctx,
            GetEntityContextInput {
                schema_version: 1,
                entity_type: entity_context.subject.kind.clone(),
                entity_id: entity_context.subject.id.clone(),
                depth: depth.clone(),
            },
        )
        .await?;
        let (entries, provenance) = child.into_parts();
        let entry_claims = ctx
            .services()
            .read_entity_context_claims(
                entity_context.subject.kind.clone(),
                entity_context.subject.id.clone(),
                context_depth_levels(input.depth),
            )
            .await
            .map_err(|error| AbilityError {
                kind: AbilityErrorKind::HardError("entity_context_claim_read".into()),
                message: error,
            })?;
        let entries =
            filter_prompt_entity_entries(entries, &entry_claims, &prompt_allowed_subjects);
        composed_children.push(ComposedEntityContext {
            subject: entity_context.subject.clone(),
            entries,
            provenance,
        });
    }

    let prompt_context = PromptContext::from_context(&context, &composed_children);
    let prompt_context_json =
        serde_json::to_string_pretty(&prompt_context).map_err(|error| AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("failed to serialize prepare_meeting prompt context: {error}"),
        })?;
    let rendered = prompts::render_prompt(&prompt_context_json, input.schema_version.0);
    let completion = ctx
        .provider
        .complete(rendered.prompt_input(), ModelTier::Synthesis)
        .await
        .map_err(provider_error)?;
    let raw = parse_completion(&completion.text)?;

    BriefAssembler::new(
        ctx,
        input.schema_version,
        input.include_open_loops,
        context,
        composed_children,
    )
    .assemble(
        raw,
        prompts::fingerprint_from_completion(&completion, &rendered),
    )
}

pub fn draft_claims_for_publish(brief: &MeetingBrief) -> Vec<ClaimDraft> {
    let mut drafts = Vec::new();
    drafts.extend(brief.topics.iter().map(|topic| {
        ClaimDraft {
            claim_type: ClaimType::MeetingTopic.as_str().to_string(),
            text: format!("{}: {}", topic.title, topic.detail),
            subject: topic.subject.clone(),
            temporal_scope: topic.temporal_scope.clone(),
            sensitivity: sensitivity_name(
                &metadata_for_claim_type(ClaimType::MeetingTopic).default_sensitivity,
            )
            .to_string(),
        }
    }));
    drafts.extend(brief.attendee_context.iter().map(|context| {
        ClaimDraft {
            claim_type: ClaimType::AttendeeContext.as_str().to_string(),
            text: context.context.clone(),
            subject: context.subject.clone(),
            temporal_scope: context.temporal_scope.clone(),
            sensitivity: sensitivity_name(
                &metadata_for_claim_type(ClaimType::AttendeeContext).default_sensitivity,
            )
            .to_string(),
        }
    }));
    drafts.extend(brief.open_loops.iter().map(|open_loop| {
        ClaimDraft {
            claim_type: ClaimType::OpenLoop.as_str().to_string(),
            text: open_loop.description.clone(),
            subject: open_loop.subject.clone(),
            temporal_scope: open_loop.temporal_scope.clone(),
            sensitivity: sensitivity_name(
                &metadata_for_claim_type(ClaimType::OpenLoop).default_sensitivity,
            )
            .to_string(),
        }
    }));
    drafts.extend(brief.what_changed_since_last.iter().map(|change| {
        ClaimDraft {
            claim_type: ClaimType::MeetingChangeMarker.as_str().to_string(),
            text: change.description.clone(),
            subject: change.subject.clone(),
            temporal_scope: change.temporal_scope.clone(),
            sensitivity: sensitivity_name(
                &metadata_for_claim_type(ClaimType::MeetingChangeMarker).default_sensitivity,
            )
            .to_string(),
        }
    }));
    drafts.extend(brief.suggested_outcomes.iter().map(|outcome| {
        ClaimDraft {
            claim_type: ClaimType::SuggestedOutcome.as_str().to_string(),
            text: format!("{}: {}", outcome.outcome, outcome.rationale),
            subject: outcome.subject.clone(),
            temporal_scope: outcome.temporal_scope.clone(),
            sensitivity: sensitivity_name(
                &metadata_for_claim_type(ClaimType::SuggestedOutcome).default_sensitivity,
            )
            .to_string(),
        }
    }));
    drafts
}

struct ComposedEntityContext {
    subject: BriefSubjectRef,
    entries: Vec<EntityContextEntry>,
    provenance: crate::abilities::provenance::Provenance,
}

#[derive(Debug, Serialize)]
struct PromptContext<'a> {
    meeting: &'a MeetingSummary,
    evidence: &'a [EvidenceSource],
    entity_contexts: Vec<PromptEntityContext<'a>>,
}

#[derive(Debug, Serialize)]
struct PromptEntityContext<'a> {
    subject: &'a BriefSubjectRef,
    entries: &'a [EntityContextEntry],
}

impl<'a> PromptContext<'a> {
    fn from_context(
        context: &'a MeetingBriefContext,
        children: &'a [ComposedEntityContext],
    ) -> Self {
        Self {
            meeting: &context.meeting,
            evidence: &context.evidence,
            entity_contexts: children
                .iter()
                .map(|child| PromptEntityContext {
                    subject: &child.subject,
                    entries: &child.entries,
                })
                .collect(),
        }
    }
}

fn filter_prompt_entity_entries(
    entries: Vec<EntityContextEntry>,
    claims: &[IntelligenceClaim],
    allowed_subjects: &BTreeSet<String>,
) -> Vec<EntityContextEntry> {
    let sensitivity_by_id: BTreeMap<&str, &ClaimSensitivity> = claims
        .iter()
        .map(|claim| (claim.id.as_str(), &claim.sensitivity))
        .collect();

    entries
        .into_iter()
        .filter(|entry| {
            let subject = BriefSubjectRef {
                kind: entry.entity_type.clone(),
                id: entry.entity_id.clone(),
            };
            if !allowed_subjects.contains(&subject.key()) {
                return false;
            }

            sensitivity_by_id
                .get(entry.id.as_str())
                .is_some_and(|sensitivity| prompt_input_sensitivity_allowed(sensitivity))
        })
        .collect()
}

impl MeetingBriefContext {
    async fn from_meeting_id(
        ctx: &AbilityContext<'_>,
        meeting_id: &str,
    ) -> Result<Self, AbilityError> {
        let snapshot = ctx
            .services()
            .read_prepare_meeting_context(meeting_id.to_string())
            .await
            .map_err(|error| AbilityError {
                kind: AbilityErrorKind::HardError("prepare_meeting_context_read".into()),
                message: error,
            })?;
        Self::from_snapshot(snapshot)
    }

    fn from_snapshot(snapshot: PrepareMeetingContextSnapshot) -> Result<Self, AbilityError> {
        let meeting = MeetingSummary {
            id: snapshot.meeting.id,
            title: snapshot.meeting.title,
            starts_at: snapshot.meeting.starts_at,
            ends_at: snapshot.meeting.ends_at,
            attendees: snapshot
                .attendees
                .into_iter()
                .map(|attendee| MeetingAttendee {
                    name: attendee.name,
                    email: attendee.email,
                    person_id: attendee.person_id,
                    account_id: attendee.account_id,
                    domain: attendee.domain,
                })
                .collect(),
        };
        let evidence = snapshot
            .claims
            .iter()
            .map(evidence_from_claim)
            .collect::<Result<Vec<_>, _>>()?;
        let entity_contexts = snapshot
            .subjects
            .into_iter()
            .map(|subject| EntityContextSeed {
                subject: BriefSubjectRef {
                    kind: subject.kind,
                    id: subject.id,
                },
                display_name: subject.display_name,
            })
            .collect();

        Ok(Self {
            meeting,
            evidence,
            entity_contexts,
        })
    }
}

fn evidence_from_claim(claim: &IntelligenceClaim) -> Result<EvidenceSource, AbilityError> {
    let subject = brief_subject_from_claim(claim)?;
    Ok(EvidenceSource {
        id: claim.id.clone(),
        subject,
        claim_type: claim.claim_type.clone(),
        text: claim.text.clone(),
        source_asof: claim.source_asof.clone(),
        observed_at: if claim.observed_at.trim().is_empty() {
            claim.created_at.clone()
        } else {
            claim.observed_at.clone()
        },
        data_source: claim.data_source.clone(),
        lifecycle: "active".into(),
        confidence: claim.trust_score.unwrap_or(0.8).clamp(0.0, 1.0) as f32,
        temporal_scope: temporal_scope_name(&claim.temporal_scope).into(),
        sensitivity: sensitivity_name(&claim.sensitivity).into(),
    })
}

fn brief_subject_from_claim(claim: &IntelligenceClaim) -> Result<BriefSubjectRef, AbilityError> {
    let value: serde_json::Value = serde_json::from_str(&claim.subject_ref).map_err(|error| {
        validation_error(format!(
            "prepare_meeting claim `{}` has invalid subject_ref JSON: {error}",
            claim.id
        ))
    })?;
    match crate::services::claims::subject_ref_from_json(&value).map_err(|error| {
        validation_error(format!(
            "prepare_meeting claim `{}` has invalid subject_ref: {error}",
            claim.id
        ))
    })? {
        ClaimSubjectRef::Account { id } => Ok(BriefSubjectRef::account(&id)),
        ClaimSubjectRef::Meeting { id } => Ok(BriefSubjectRef::meeting(&id)),
        ClaimSubjectRef::Person { id } => Ok(BriefSubjectRef::person(&id)),
        ClaimSubjectRef::Project { id } => Ok(BriefSubjectRef::project(&id)),
        ClaimSubjectRef::Email { .. } | ClaimSubjectRef::Multi(_) | ClaimSubjectRef::Global => {
            Err(validation_error(format!(
                "prepare_meeting claim `{}` has unsupported subject_ref",
                claim.id
            )))
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawMeetingBrief {
    #[serde(default)]
    topics: Vec<RawTopic>,
    #[serde(default)]
    attendee_context: Vec<RawAttendeeContext>,
    #[serde(default)]
    open_loops: Vec<RawOpenLoop>,
    #[serde(default)]
    what_changed_since_last: Vec<RawChangeMarker>,
    #[serde(default)]
    suggested_outcomes: Vec<RawSuggestedOutcome>,
}

#[derive(Debug, Deserialize)]
struct RawTopic {
    title: String,
    #[serde(default)]
    detail: String,
    #[serde(default)]
    subject: Option<BriefSubjectRef>,
    #[serde(default)]
    source_ids: Vec<String>,
    #[serde(default)]
    confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct RawAttendeeContext {
    attendee: String,
    context: String,
    #[serde(default)]
    subject: Option<BriefSubjectRef>,
    #[serde(default)]
    source_ids: Vec<String>,
    #[serde(default)]
    confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct RawOpenLoop {
    description: String,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    subject: Option<BriefSubjectRef>,
    #[serde(default)]
    source_ids: Vec<String>,
    #[serde(default)]
    confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct RawChangeMarker {
    description: String,
    #[serde(default)]
    subject: Option<BriefSubjectRef>,
    #[serde(default)]
    source_ids: Vec<String>,
    #[serde(default)]
    confidence: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct RawSuggestedOutcome {
    outcome: String,
    #[serde(default)]
    rationale: String,
    #[serde(default)]
    subject: Option<BriefSubjectRef>,
    #[serde(default)]
    source_ids: Vec<String>,
    #[serde(default)]
    confidence: Option<f32>,
}

fn parse_completion(text: &str) -> Result<RawMeetingBrief, AbilityError> {
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
        message: format!("prepare_meeting provider response was not valid JSON: {error}"),
    })
}

struct BriefAssembler<'a> {
    ctx: &'a AbilityContext<'a>,
    schema_version: SchemaVersion,
    include_open_loops: bool,
    context: MeetingBriefContext,
    children: Vec<ComposedEntityContext>,
    source_indices: BTreeMap<String, crate::abilities::provenance::SourceIndex>,
    source_by_id: BTreeMap<String, EvidenceSource>,
    child_ref_by_subject: BTreeMap<String, CompositionId>,
    subject_catalog: SubjectCatalog,
    meeting_scope_source_subjects: BTreeSet<String>,
}

impl<'a> BriefAssembler<'a> {
    fn new(
        ctx: &'a AbilityContext<'a>,
        schema_version: SchemaVersion,
        include_open_loops: bool,
        context: MeetingBriefContext,
        children: Vec<ComposedEntityContext>,
    ) -> Self {
        let source_by_id = context
            .evidence
            .iter()
            .cloned()
            .map(|source| (source.id.clone(), source))
            .collect();
        let subject_catalog = SubjectCatalog::new(&context);
        let meeting_scope_source_subjects = meeting_scope_source_subjects(&context);
        Self {
            ctx,
            schema_version,
            include_open_loops,
            context,
            children,
            source_indices: BTreeMap::new(),
            source_by_id,
            child_ref_by_subject: BTreeMap::new(),
            subject_catalog,
            meeting_scope_source_subjects,
        }
    }

    fn assemble(
        mut self,
        raw: RawMeetingBrief,
        fingerprint: crate::abilities::provenance::PromptFingerprint,
    ) -> AbilityResult<MeetingBrief> {
        let mut builder = ProvenanceBuilder::new(config_for(
            self.ctx,
            ABILITY_NAME,
            self.schema_version,
            AbilityCategory::Transform,
        ));
        builder.set_prompt_fingerprint(fingerprint);

        let meeting_subject = SubjectAttribution::direct_confident(SubjectRef::Meeting(
            self.context.meeting.id.clone(),
        ));
        let meeting_source = self.add_meeting_source(&mut builder)?;
        builder
            .attribute_subtree(
                FieldPath::new("/meeting").map_err(map_field_error)?,
                FieldAttribution::direct(meeting_subject.clone(), meeting_source),
            )
            .map_err(map_provenance_error)?;
        builder
            .attribute(
                FieldPath::new("/schema_version").map_err(map_field_error)?,
                FieldAttribution::constant(meeting_subject.clone()),
            )
            .map_err(map_provenance_error)?;

        for child in &self.children {
            let composition_id =
                CompositionId::new(format!("get_entity_context:{}", child.subject.key()));
            self.child_ref_by_subject
                .insert(child.subject.key(), composition_id.clone());
            builder
                .compose(composition_id, child.provenance.clone())
                .map_err(map_provenance_error)?;
        }

        let mut accepted_subjects = vec![SubjectRef::Meeting(self.context.meeting.id.clone())];
        let mut brief = MeetingBrief {
            meeting: self.context.meeting.clone(),
            topics: Vec::new(),
            attendee_context: Vec::new(),
            open_loops: Vec::new(),
            what_changed_since_last: Vec::new(),
            suggested_outcomes: Vec::new(),
            schema_version: self.schema_version,
        };

        for raw_topic in raw.topics {
            let candidate = Candidate::topic(raw_topic, &self.context);
            if let Some((topic, attribution)) = self.accept_llm_candidate(
                &mut builder,
                candidate,
                ClaimType::MeetingTopic,
                "/topics",
            )? {
                push_subject_unique(&mut accepted_subjects, attribution.subject.subject.clone());
                let index = brief.topics.len();
                brief.topics.push(topic);
                self.attribute_item(&mut builder, format!("/topics/{index}"), attribution)?;
            }
        }

        for raw_context in raw.attendee_context {
            let candidate = Candidate::attendee_context(raw_context);
            if let Some((item, attribution)) =
                self.accept_attendee_context(&mut builder, candidate, "/attendee_context")?
            {
                push_subject_unique(&mut accepted_subjects, attribution.subject.subject.clone());
                let index = brief.attendee_context.len();
                brief.attendee_context.push(item);
                self.attribute_item(
                    &mut builder,
                    format!("/attendee_context/{index}"),
                    attribution,
                )?;
            }
        }

        if self.include_open_loops {
            for raw_open_loop in raw.open_loops {
                let candidate = Candidate::open_loop(raw_open_loop, &self.context);
                if let Some((item, attribution)) = self.accept_direct_candidate(
                    &mut builder,
                    candidate,
                    ClaimType::OpenLoop,
                    "/open_loops",
                )? {
                    push_subject_unique(
                        &mut accepted_subjects,
                        attribution.subject.subject.clone(),
                    );
                    let index = brief.open_loops.len();
                    brief.open_loops.push(item);
                    self.attribute_item(&mut builder, format!("/open_loops/{index}"), attribution)?;
                }
            }
        }

        for raw_change in raw.what_changed_since_last {
            let candidate = Candidate::change_marker(raw_change, &self.context);
            if let Some((item, attribution)) = self.accept_direct_candidate(
                &mut builder,
                candidate,
                ClaimType::MeetingChangeMarker,
                "/what_changed_since_last",
            )? {
                push_subject_unique(&mut accepted_subjects, attribution.subject.subject.clone());
                let index = brief.what_changed_since_last.len();
                brief.what_changed_since_last.push(item);
                self.attribute_item(
                    &mut builder,
                    format!("/what_changed_since_last/{index}"),
                    attribution,
                )?;
            }
        }

        for raw_outcome in raw.suggested_outcomes {
            let candidate = Candidate::suggested_outcome(raw_outcome, &self.context);
            if let Some((item, attribution)) = self.accept_llm_candidate(
                &mut builder,
                candidate,
                ClaimType::SuggestedOutcome,
                "/suggested_outcomes",
            )? {
                push_subject_unique(&mut accepted_subjects, attribution.subject.subject.clone());
                let index = brief.suggested_outcomes.len();
                brief.suggested_outcomes.push(item);
                self.attribute_item(
                    &mut builder,
                    format!("/suggested_outcomes/{index}"),
                    attribution,
                )?;
            }
        }

        self.attribute_empty_sections(&mut builder, &meeting_subject, &brief)?;
        builder.set_subject(envelope_subject(accepted_subjects));
        builder.finalize(brief).map_err(map_provenance_error)
    }

    fn add_meeting_source(
        &mut self,
        builder: &mut ProvenanceBuilder,
    ) -> Result<crate::abilities::provenance::SourceIndex, AbilityError> {
        let now = self.ctx.services().clock.now();
        let source_asof = self
            .context
            .meeting
            .starts_at
            .as_deref()
            .and_then(parse_rfc3339);
        SourceAttribution::new(
            DataSource::Google,
            vec![SourceIdentifier::Meeting {
                meeting_id: MeetingId::new(self.context.meeting.id.clone()),
            }],
            source_asof.unwrap_or(now),
            source_asof,
            1.0,
            None,
        )
        .map(|source| builder.add_source(source))
        .map_err(|error| AbilityError {
            kind: AbilityErrorKind::Validation,
            message: error.to_string(),
        })
    }

    fn accept_llm_candidate<T>(
        &mut self,
        builder: &mut ProvenanceBuilder,
        candidate: Candidate<T>,
        claim_type: ClaimType,
        section_path: &str,
    ) -> Result<Option<(T, FieldAttribution)>, AbilityError> {
        self.accept_candidate(
            builder,
            candidate,
            claim_type,
            section_path,
            AttributionMode::Llm,
        )
    }

    fn accept_direct_candidate<T>(
        &mut self,
        builder: &mut ProvenanceBuilder,
        candidate: Candidate<T>,
        claim_type: ClaimType,
        section_path: &str,
    ) -> Result<Option<(T, FieldAttribution)>, AbilityError> {
        self.accept_candidate(
            builder,
            candidate,
            claim_type,
            section_path,
            AttributionMode::Direct,
        )
    }

    fn accept_attendee_context(
        &mut self,
        builder: &mut ProvenanceBuilder,
        candidate: Candidate<AttendeeContext>,
        section_path: &str,
    ) -> Result<Option<(AttendeeContext, FieldAttribution)>, AbilityError> {
        if !self.validate_candidate_sources(builder, &candidate, section_path)? {
            return Ok(None);
        }
        let Some(subject) = self
            .subject_catalog
            .attribution_for(&candidate.subject, candidate.confidence)
        else {
            self.add_subject_ambiguous_warning(builder, section_path)?;
            return Ok(None);
        };
        if !subject.is_confident() {
            self.add_subject_ambiguous_warning(builder, section_path)?;
            return Ok(None);
        }
        let Some(child_id) = self
            .child_ref_by_subject
            .get(&candidate.subject.key())
            .cloned()
        else {
            return self.accept_candidate(
                builder,
                candidate,
                ClaimType::AttendeeContext,
                section_path,
                AttributionMode::Llm,
            );
        };
        let attribution = FieldAttribution::composed(
            subject,
            child_id,
            FieldPath::root(),
            Confidence::composed_min(candidate.confidence).map_err(map_field_error)?,
        )
        .map_err(map_field_error)?;
        Ok(Some((candidate.item, attribution)))
    }

    fn accept_candidate<T>(
        &mut self,
        builder: &mut ProvenanceBuilder,
        candidate: Candidate<T>,
        claim_type: ClaimType,
        section_path: &str,
        mode: AttributionMode,
    ) -> Result<Option<(T, FieldAttribution)>, AbilityError> {
        if candidate.source_ids.is_empty() {
            return Err(validation_error("LLM candidate missing source_ids"));
        }
        let Some(subject) = self
            .subject_catalog
            .attribution_for(&candidate.subject, candidate.confidence)
        else {
            self.add_subject_ambiguous_warning(builder, section_path)?;
            return Ok(None);
        };
        if !subject.is_confident() || candidate.confidence < SUBJECT_CONFIDENCE_FLOOR {
            self.add_subject_ambiguous_warning(builder, section_path)?;
            return Ok(None);
        }

        let mut source_refs = Vec::new();
        for source_id in &candidate.source_ids {
            let Some(source) = self.source_by_id.get(source_id).cloned() else {
                return Err(validation_error(
                    "LLM candidate referenced unknown source_id",
                ));
            };
            if source.lifecycle == "revoked" {
                builder.add_warning(ProvenanceWarning::Masked {
                    reason: MaskReason::SourceRevoked,
                });
                return Ok(None);
            }
            if !source_subject_allowed(
                &candidate.subject,
                &source.subject,
                &self.meeting_scope_source_subjects,
            ) {
                self.add_subject_ambiguous_warning(builder, section_path)?;
                return Ok(None);
            }
            let source_index = self.ensure_source(builder, &source)?;
            source_refs.push(SourceRef::Source { source_index });
        }

        let confidence = match mode {
            AttributionMode::Llm => Confidence::provider_reported(candidate.confidence),
            AttributionMode::Direct => Confidence::computed(candidate.confidence),
        }
        .map_err(map_field_error)?;

        let attribution = match mode {
            AttributionMode::Llm => {
                FieldAttribution::llm_synthesis(subject, source_refs, confidence, None)
            }
            AttributionMode::Direct => FieldAttribution::computed(
                subject,
                format!("claim_type:{}", claim_type.as_str()),
                source_refs,
                confidence,
            ),
        }
        .map_err(map_field_error)?;
        Ok(Some((candidate.item, attribution)))
    }

    fn validate_candidate_sources<T>(
        &self,
        builder: &mut ProvenanceBuilder,
        candidate: &Candidate<T>,
        section_path: &str,
    ) -> Result<bool, AbilityError> {
        if candidate.source_ids.is_empty() {
            return Err(validation_error("LLM candidate missing source_ids"));
        }
        for source_id in &candidate.source_ids {
            let Some(source) = self.source_by_id.get(source_id) else {
                return Err(validation_error(
                    "LLM candidate referenced unknown source_id",
                ));
            };
            if source.lifecycle == "revoked" {
                builder.add_warning(ProvenanceWarning::Masked {
                    reason: MaskReason::SourceRevoked,
                });
                return Ok(false);
            }
            if !source_subject_allowed(
                &candidate.subject,
                &source.subject,
                &self.meeting_scope_source_subjects,
            ) {
                self.add_subject_ambiguous_warning(builder, section_path)?;
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn ensure_source(
        &mut self,
        builder: &mut ProvenanceBuilder,
        evidence: &EvidenceSource,
    ) -> Result<crate::abilities::provenance::SourceIndex, AbilityError> {
        if let Some(index) = self.source_indices.get(&evidence.id) {
            return Ok(*index);
        }
        let now = self.ctx.services().clock.now();
        let source_asof = source_asof(evidence.source_asof.as_deref(), now);
        let observed_at = parse_rfc3339(&evidence.observed_at).unwrap_or(now);
        let age = source_asof.map(|asof| now.signed_duration_since(asof));
        let evidence_weight = if age.is_some_and(|age| age.num_days() > STALE_SOURCE_DAYS) {
            0.35
        } else {
            evidence.confidence.clamp(0.0, 1.0)
        };
        let source = SourceAttribution::new(
            data_source(&evidence.data_source),
            vec![source_identifier(evidence)],
            observed_at,
            source_asof,
            evidence_weight,
            None,
        )
        .map_err(|error| AbilityError {
            kind: AbilityErrorKind::Validation,
            message: error.to_string(),
        })?;
        let index = builder.add_source(source);
        if let Some(age) = age {
            if age.num_days() > STALE_SOURCE_DAYS {
                builder.add_warning(ProvenanceWarning::SourceStale {
                    source_index: index,
                    age_seconds: age.num_seconds(),
                });
            }
        }
        self.source_indices.insert(evidence.id.clone(), index);
        Ok(index)
    }

    fn attribute_item(
        &self,
        builder: &mut ProvenanceBuilder,
        path: String,
        attribution: FieldAttribution,
    ) -> Result<(), AbilityError> {
        builder
            .attribute_subtree(FieldPath::new(path).map_err(map_field_error)?, attribution)
            .map_err(map_provenance_error)?;
        Ok(())
    }

    fn attribute_empty_sections(
        &self,
        builder: &mut ProvenanceBuilder,
        meeting_subject: &SubjectAttribution,
        brief: &MeetingBrief,
    ) -> Result<(), AbilityError> {
        let sections = [
            ("/topics", brief.topics.is_empty()),
            ("/attendee_context", brief.attendee_context.is_empty()),
            ("/open_loops", brief.open_loops.is_empty()),
            (
                "/what_changed_since_last",
                brief.what_changed_since_last.is_empty(),
            ),
            ("/suggested_outcomes", brief.suggested_outcomes.is_empty()),
        ];
        for (path, is_empty) in sections {
            if is_empty {
                builder
                    .attribute(
                        FieldPath::new(path).map_err(map_field_error)?,
                        FieldAttribution::constant(meeting_subject.clone()),
                    )
                    .map_err(map_provenance_error)?;
            }
        }
        Ok(())
    }

    fn add_subject_ambiguous_warning(
        &self,
        builder: &mut ProvenanceBuilder,
        section_path: &str,
    ) -> Result<(), AbilityError> {
        builder.add_warning(ProvenanceWarning::SubjectFitQualified {
            field: Some(FieldPath::new(section_path).map_err(map_field_error)?),
            status: "SubjectAmbiguous".into(),
        });
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum AttributionMode {
    Direct,
    Llm,
}

struct Candidate<T> {
    item: T,
    subject: BriefSubjectRef,
    source_ids: Vec<String>,
    confidence: f32,
}

impl Candidate<Topic> {
    fn topic(raw: RawTopic, context: &MeetingBriefContext) -> Self {
        let subject = raw
            .subject
            .unwrap_or_else(|| BriefSubjectRef::meeting(&context.meeting.id));
        let item = Topic {
            title: raw.title,
            detail: raw.detail,
            subject: subject.clone(),
            temporal_scope: state_scope(),
        };
        Self {
            item,
            subject,
            source_ids: raw.source_ids,
            confidence: raw.confidence.unwrap_or(0.8),
        }
    }
}

impl Candidate<AttendeeContext> {
    fn attendee_context(raw: RawAttendeeContext) -> Self {
        let subject = raw.subject.unwrap_or_else(BriefSubjectRef::unknown);
        let item = AttendeeContext {
            attendee: raw.attendee,
            context: raw.context,
            subject: subject.clone(),
            temporal_scope: state_scope(),
        };
        Self {
            item,
            subject,
            source_ids: raw.source_ids,
            confidence: raw.confidence.unwrap_or(0.8),
        }
    }
}

impl Candidate<OpenLoop> {
    fn open_loop(raw: RawOpenLoop, context: &MeetingBriefContext) -> Self {
        let subject = raw
            .subject
            .unwrap_or_else(|| BriefSubjectRef::meeting(&context.meeting.id));
        let item = OpenLoop {
            description: raw.description,
            owner: raw.owner,
            subject: subject.clone(),
            temporal_scope: state_scope(),
        };
        Self {
            item,
            subject,
            source_ids: raw.source_ids,
            confidence: raw.confidence.unwrap_or(0.85),
        }
    }
}

impl Candidate<ChangeMarker> {
    fn change_marker(raw: RawChangeMarker, context: &MeetingBriefContext) -> Self {
        let subject = raw
            .subject
            .unwrap_or_else(|| BriefSubjectRef::meeting(&context.meeting.id));
        let item = ChangeMarker {
            description: raw.description,
            subject: subject.clone(),
            temporal_scope: point_in_time_scope(context),
        };
        Self {
            item,
            subject,
            source_ids: raw.source_ids,
            confidence: raw.confidence.unwrap_or(0.85),
        }
    }
}

impl Candidate<SuggestedOutcome> {
    fn suggested_outcome(raw: RawSuggestedOutcome, context: &MeetingBriefContext) -> Self {
        let subject = raw
            .subject
            .unwrap_or_else(|| BriefSubjectRef::meeting(&context.meeting.id));
        let item = SuggestedOutcome {
            outcome: raw.outcome,
            rationale: raw.rationale,
            subject: subject.clone(),
            temporal_scope: state_scope(),
        };
        Self {
            item,
            subject,
            source_ids: raw.source_ids,
            confidence: raw.confidence.unwrap_or(0.8),
        }
    }
}

struct SubjectCatalog {
    subjects: BTreeMap<String, SubjectAttribution>,
}

impl SubjectCatalog {
    fn new(context: &MeetingBriefContext) -> Self {
        let mut subjects = BTreeMap::new();
        let meeting = BriefSubjectRef::meeting(&context.meeting.id);
        subjects.insert(
            meeting.key(),
            SubjectAttribution::direct_confident(meeting.to_subject_ref()),
        );

        let mut accounts_by_domain: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for attendee in &context.meeting.attendees {
            if let Some(account_id) = attendee.account_id.as_deref() {
                let subject = BriefSubjectRef::account(account_id);
                subjects.insert(
                    subject.key(),
                    inferred_subject(subject.to_subject_ref(), "attendee_account", 0.9),
                );
                if let Some(domain) = attendee.domain.as_deref().or_else(|| {
                    attendee
                        .email
                        .as_deref()
                        .and_then(|email| email.split_once('@').map(|(_, domain)| domain))
                }) {
                    accounts_by_domain
                        .entry(domain.to_ascii_lowercase())
                        .or_default()
                        .insert(account_id.to_string());
                }
            }
            if let Some(person_id) = attendee.person_id.as_deref() {
                let subject = BriefSubjectRef::person(person_id);
                subjects.insert(
                    subject.key(),
                    inferred_subject(subject.to_subject_ref(), "attendee_person", 0.85),
                );
            }
        }

        for accounts in accounts_by_domain
            .values()
            .filter(|accounts| accounts.len() > 1)
        {
            for account_id in accounts {
                let subject = BriefSubjectRef::account(account_id);
                subjects.insert(
                    subject.key(),
                    ambiguous_subject(subject.to_subject_ref(), "same_domain_multi_account", 0.5),
                );
            }
        }

        for evidence in &context.evidence {
            subjects.entry(evidence.subject.key()).or_insert_with(|| {
                inferred_subject(evidence.subject.to_subject_ref(), "source_matched", 0.75)
            });
        }
        for child in &context.entity_contexts {
            subjects.entry(child.subject.key()).or_insert_with(|| {
                inferred_subject(child.subject.to_subject_ref(), "composed_child", 0.85)
            });
        }
        Self { subjects }
    }

    fn attribution_for(
        &self,
        subject: &BriefSubjectRef,
        candidate_confidence: f32,
    ) -> Option<SubjectAttribution> {
        let mut attribution = self.subjects.get(&subject.key()).cloned()?;
        let confidence = attribution.fit.confidence.min(candidate_confidence);
        if confidence < SUBJECT_CONFIDENCE_FLOOR {
            attribution.fit = SubjectFitAssessment::ambiguous("below_confidence_floor", confidence)
                .expect("valid confidence");
        }
        Some(attribution)
    }
}

impl BriefSubjectRef {
    fn meeting(id: &str) -> Self {
        Self {
            kind: "meeting".into(),
            id: id.into(),
        }
    }

    fn account(id: &str) -> Self {
        Self {
            kind: "account".into(),
            id: id.into(),
        }
    }

    fn person(id: &str) -> Self {
        Self {
            kind: "person".into(),
            id: id.into(),
        }
    }

    fn project(id: &str) -> Self {
        Self {
            kind: "project".into(),
            id: id.into(),
        }
    }

    fn unknown() -> Self {
        Self {
            kind: "unknown".into(),
            id: "unknown".into(),
        }
    }

    fn key(&self) -> String {
        format!("{}:{}", self.kind, self.id)
    }

    fn to_subject_ref(&self) -> SubjectRef {
        match self.kind.as_str() {
            "account" => SubjectRef::Account(self.id.clone()),
            "project" => SubjectRef::Project(self.id.clone()),
            "person" => SubjectRef::Person(self.id.clone()),
            "meeting" => SubjectRef::Meeting(self.id.clone()),
            "user" => SubjectRef::User(self.id.clone()),
            _ => SubjectRef::Unknown,
        }
    }
}

fn source_subject_allowed(
    candidate: &BriefSubjectRef,
    source: &BriefSubjectRef,
    meeting_scope_source_subjects: &BTreeSet<String>,
) -> bool {
    let source_key = source.key();
    if !meeting_scope_source_subjects.contains(&source_key) {
        return false;
    }

    if candidate.key() == source_key {
        return true;
    }

    candidate.kind == "meeting"
}

fn meeting_scope_source_subjects(context: &MeetingBriefContext) -> BTreeSet<String> {
    let mut subjects = BTreeSet::new();
    subjects.insert(BriefSubjectRef::meeting(&context.meeting.id).key());
    for attendee in &context.meeting.attendees {
        if let Some(person_id) = attendee.person_id.as_deref() {
            subjects.insert(BriefSubjectRef::person(person_id).key());
        }
        if let Some(account_id) = attendee.account_id.as_deref() {
            subjects.insert(BriefSubjectRef::account(account_id).key());
        }
    }
    for entity_context in &context.entity_contexts {
        if matches!(
            entity_context.subject.kind.as_str(),
            "account" | "person" | "project"
        ) {
            subjects.insert(entity_context.subject.key());
        }
    }
    subjects
}

fn envelope_subject(subjects: Vec<SubjectRef>) -> SubjectAttribution {
    if subjects.len() == 1 {
        return SubjectAttribution::direct_confident(
            subjects.into_iter().next().expect("one subject"),
        );
    }
    SubjectAttribution::direct_confident(SubjectRef::Multi(subjects.into_iter().collect()))
}

fn push_subject_unique(subjects: &mut Vec<SubjectRef>, subject: SubjectRef) {
    if !subjects.iter().any(|existing| existing == &subject) {
        subjects.push(subject);
    }
}

fn inferred_subject(subject: SubjectRef, method: &str, confidence: f32) -> SubjectAttribution {
    SubjectAttribution::new(
        subject,
        SubjectBindingKind::Inferred,
        Vec::new(),
        Vec::new(),
        SubjectFitAssessment::confident(method, confidence).expect("valid confidence"),
    )
    .expect("valid subject attribution")
}

fn ambiguous_subject(subject: SubjectRef, method: &str, confidence: f32) -> SubjectAttribution {
    SubjectAttribution::new(
        subject,
        SubjectBindingKind::Inferred,
        Vec::new(),
        Vec::new(),
        SubjectFitAssessment::ambiguous(method, confidence).expect("valid confidence"),
    )
    .expect("valid subject attribution")
}

fn state_scope() -> BriefTemporalScope {
    BriefTemporalScope::State
}

fn point_in_time_scope(context: &MeetingBriefContext) -> BriefTemporalScope {
    BriefTemporalScope::PointInTime {
        occurred_at: context
            .meeting
            .starts_at
            .clone()
            .unwrap_or_else(|| "unknown".into()),
    }
}

fn source_asof(input: Option<&str>, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match parse_source_timestamp(input, now, None) {
        SourceTimestampStatus::Accepted(parsed) => Some(parsed),
        SourceTimestampStatus::Implausible { parsed, .. } => Some(parsed),
        SourceTimestampStatus::Malformed(_) | SourceTimestampStatus::Missing => None,
    }
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|parsed| parsed.with_timezone(&Utc))
        .ok()
}

fn data_source(value: &str) -> DataSource {
    match value {
        "user" => DataSource::User,
        "google" => DataSource::Google,
        "glean" => DataSource::Glean {
            downstream: GleanDownstream::Documents,
        },
        "ai" => DataSource::Ai,
        "local_enrichment" => DataSource::LocalEnrichment,
        other => DataSource::Other(crate::abilities::provenance::SourceName::new(other)),
    }
}

fn source_identifier(evidence: &EvidenceSource) -> SourceIdentifier {
    match evidence.data_source.as_str() {
        "google" => SourceIdentifier::Meeting {
            meeting_id: MeetingId::new(evidence.id.clone()),
        },
        "user" => SourceIdentifier::UserEntry {
            entry_id: crate::abilities::provenance::ContextEntryId::new(evidence.id.clone()),
        },
        "glean" => SourceIdentifier::Document {
            document_id: crate::abilities::provenance::DocumentId::new(evidence.id.clone()),
            chunk_id: None,
        },
        _ => SourceIdentifier::Signal {
            signal_id: crate::abilities::provenance::SignalId::new(evidence.id.clone()),
        },
    }
}

fn context_depth(depth: u8) -> ContextDepth {
    match depth {
        0 | 1 => ContextDepth::Shallow,
        2 => ContextDepth::Standard,
        _ => ContextDepth::Deep,
    }
}

fn context_depth_levels(depth: u8) -> usize {
    match depth {
        0 | 1 => 1,
        2 => 2,
        _ => 3,
    }
}

fn prompt_input_sensitivity_allowed(sensitivity: &ClaimSensitivity) -> bool {
    matches!(
        sensitivity,
        ClaimSensitivity::Public | ClaimSensitivity::Internal
    )
}

fn config_for(
    ctx: &AbilityContext<'_>,
    ability_name: &str,
    schema_version: SchemaVersion,
    category: AbilityCategory,
) -> ProvenanceBuilderConfig {
    let mut config = ProvenanceBuilderConfig::new(ability_name, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(0, 1);
    config.ability_schema_version = schema_version;
    config.actor = provenance_actor(ctx.actor);
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
    }
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

fn sensitivity_name(sensitivity: &ClaimSensitivity) -> &'static str {
    match sensitivity {
        ClaimSensitivity::Public => "public",
        ClaimSensitivity::Internal => "internal",
        ClaimSensitivity::Confidential => "confidential",
        ClaimSensitivity::UserOnly => "user_only",
    }
}

fn default_depth() -> u8 {
    2
}

fn default_true() -> bool {
    true
}

fn default_schema_version() -> SchemaVersion {
    SchemaVersion(1)
}

fn default_data_source() -> String {
    "local_enrichment".into()
}

fn default_active_lifecycle() -> String {
    "active".into()
}

fn default_confidence() -> f32 {
    0.8
}

fn default_temporal_scope_name() -> String {
    "state".into()
}

fn default_sensitivity_name() -> String {
    "internal".into()
}

fn temporal_scope_name(scope: &TemporalScope) -> &'static str {
    match scope {
        TemporalScope::State => "state",
        TemporalScope::PointInTime => "point_in_time",
        TemporalScope::Trend => "trend",
        TemporalScope::Closed => "closed",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use chrono::TimeZone;

    use super::*;
    use crate::abilities::feedback::ClaimVerificationState;
    use crate::abilities::{AbilityRegistry, Actor, NOOP_ABILITY_TRACER};
    use crate::db::claims::{ClaimState, SurfacingState};
    use crate::intelligence::provider::{
        Completion, FingerprintMetadata, IntelligenceProvider, ModelName, PromptInput, ProviderKind,
    };
    use crate::services::context::{
        EntityContextClaimReadFuture, EntityContextClaimReadHandle, ExternalClients, FixedClock,
        PrepareMeetingAttendeeSnapshot, PrepareMeetingContextReadFuture,
        PrepareMeetingContextReadHandle, PrepareMeetingContextSnapshot, PrepareMeetingSnapshot,
        PrepareMeetingSubjectSnapshot, SeedableRng, ServiceContext,
    };

    struct StaticProvider {
        completion: String,
        prompt: Mutex<Option<PromptInput>>,
    }

    impl StaticProvider {
        fn new(completion: String) -> Self {
            Self {
                completion,
                prompt: Mutex::new(None),
            }
        }

        fn captured_prompt(&self) -> PromptInput {
            self.prompt
                .lock()
                .expect("prompt capture mutex")
                .clone()
                .expect("provider captured a prompt")
        }
    }

    #[async_trait]
    impl IntelligenceProvider for StaticProvider {
        async fn complete(
            &self,
            prompt: PromptInput,
            _tier: ModelTier,
        ) -> Result<Completion, ProviderError> {
            *self.prompt.lock().expect("prompt capture mutex") = Some(prompt);
            Ok(Completion {
                text: self.completion.clone(),
                fingerprint_metadata: FingerprintMetadata {
                    provider: ProviderKind::ClaudeCode,
                    model: ModelName::new("test-synthesis"),
                    temperature: 1.0,
                    ..FingerprintMetadata::default()
                },
            })
        }

        fn provider_kind(&self) -> ProviderKind {
            ProviderKind::ClaudeCode
        }

        fn current_model(&self, _tier: ModelTier) -> ModelName {
            ModelName::new("test-synthesis")
        }
    }

    #[derive(Clone)]
    struct FixtureEntityContextClaimReader {
        claims: Vec<IntelligenceClaim>,
        related_subjects: BTreeMap<String, Vec<BriefSubjectRef>>,
    }

    impl FixtureEntityContextClaimReader {
        fn new(claims: Vec<IntelligenceClaim>) -> Self {
            Self {
                claims,
                related_subjects: BTreeMap::new(),
            }
        }

        fn with_related_subjects(
            claims: Vec<IntelligenceClaim>,
            related_subjects: BTreeMap<String, Vec<BriefSubjectRef>>,
        ) -> Self {
            Self {
                claims,
                related_subjects,
            }
        }

        fn subjects_within_depth(
            &self,
            entity_type: &str,
            entity_id: &str,
            depth: usize,
        ) -> BTreeSet<String> {
            let mut allowed = BTreeSet::new();
            let mut queue = vec![(
                BriefSubjectRef {
                    kind: entity_type.to_string(),
                    id: entity_id.to_string(),
                },
                1usize,
            )];

            while let Some((subject, level)) = queue.pop() {
                if !allowed.insert(subject.key()) || level >= depth {
                    continue;
                }
                if let Some(related) = self.related_subjects.get(&subject.key()) {
                    for related_subject in related {
                        queue.push((related_subject.clone(), level + 1));
                    }
                }
            }

            allowed
        }
    }

    impl EntityContextClaimReadHandle for FixtureEntityContextClaimReader {
        fn read_entity_context_claims<'a>(
            &'a self,
            entity_type: String,
            entity_id: String,
            depth: usize,
        ) -> EntityContextClaimReadFuture<'a> {
            Box::pin(async move {
                let subjects = self.subjects_within_depth(&entity_type, &entity_id, depth);
                Ok(self
                    .claims
                    .iter()
                    .filter(|claim| {
                        claim.claim_state == ClaimState::Active
                            && claim.surfacing_state == SurfacingState::Active
                            && brief_subject_from_claim(claim)
                                .map(|subject| subjects.contains(&subject.key()))
                                .unwrap_or(false)
                    })
                    .cloned()
                    .collect())
            })
        }
    }

    #[derive(Clone)]
    struct FixturePrepareMeetingContextReader {
        snapshot: PrepareMeetingContextSnapshot,
    }

    impl PrepareMeetingContextReadHandle for FixturePrepareMeetingContextReader {
        fn read_prepare_meeting_context<'a>(
            &'a self,
            meeting_id: String,
        ) -> PrepareMeetingContextReadFuture<'a> {
            Box::pin(async move {
                if self.snapshot.meeting.id == meeting_id {
                    Ok(self.snapshot.clone())
                } else {
                    Err(format!("fixture meeting `{meeting_id}` not seeded"))
                }
            })
        }
    }

    struct Harness {
        clock: FixedClock,
        rng: SeedableRng,
        provider: Arc<StaticProvider>,
        claim_reader: Arc<FixtureEntityContextClaimReader>,
        meeting_context_reader: Arc<FixturePrepareMeetingContextReader>,
    }

    impl Harness {
        fn new(completion: serde_json::Value) -> Self {
            let snapshot =
                fixture_meeting_snapshot("meeting-1", Vec::new(), Vec::new(), Vec::new());
            Self {
                clock: FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap()),
                rng: SeedableRng::new(219),
                provider: Arc::new(StaticProvider::new(completion.to_string())),
                claim_reader: Arc::new(FixtureEntityContextClaimReader::new(Vec::new())),
                meeting_context_reader: Arc::new(FixturePrepareMeetingContextReader { snapshot }),
            }
        }

        fn with_claims(mut self, claims: Vec<IntelligenceClaim>) -> Self {
            self.claim_reader = Arc::new(FixtureEntityContextClaimReader::with_related_subjects(
                claims,
                self.claim_reader.related_subjects.clone(),
            ));
            self
        }

        fn with_related_subjects(
            mut self,
            related_subjects: BTreeMap<String, Vec<BriefSubjectRef>>,
        ) -> Self {
            self.claim_reader = Arc::new(FixtureEntityContextClaimReader::with_related_subjects(
                self.claim_reader.claims.clone(),
                related_subjects,
            ));
            self
        }

        fn with_meeting_context(mut self, snapshot: PrepareMeetingContextSnapshot) -> Self {
            self.meeting_context_reader = Arc::new(FixturePrepareMeetingContextReader { snapshot });
            self
        }

        async fn run(&self, input: PrepareMeetingInput) -> AbilityResult<MeetingBrief> {
            let services = ServiceContext::new_evaluate_default(&self.clock, &self.rng)
                .with_entity_context_claim_reader(self.claim_reader.clone())
                .with_prepare_meeting_context_reader(self.meeting_context_reader.clone());
            let ctx = AbilityContext::new(
                &services,
                self.provider.as_ref(),
                &NOOP_ABILITY_TRACER,
                Actor::User,
                None,
            );
            build_meeting_brief(&ctx, input).await
        }

        fn captured_prompt(&self) -> PromptInput {
            self.provider.captured_prompt()
        }

        async fn run_erased_as_agent(
            &self,
            input_json: serde_json::Value,
        ) -> Result<serde_json::Value, AbilityError> {
            let external = ExternalClients::default();
            let services = ServiceContext::new_live(&self.clock, &self.rng, &external)
                .with_entity_context_claim_reader(self.claim_reader.clone())
                .with_prepare_meeting_context_reader(self.meeting_context_reader.clone());
            let ctx = AbilityContext::new(
                &services,
                self.provider.as_ref(),
                &NOOP_ABILITY_TRACER,
                Actor::Agent,
                None,
            );
            let registry = AbilityRegistry::from_inventory_checked().expect("registry builds");
            registry
                .invoke_by_name_json(&ctx, "prepare_meeting", input_json)
                .await
        }
    }

    #[tokio::test]
    async fn prepare_meeting_build_brief_returns_ability_output_with_provenance() {
        let harness = Harness::new(serde_json::json!({
            "topics": [{
                "title": "Confirm rollout owner",
                "detail": "The meeting should settle the rollout owner.",
                "subject": {"kind": "meeting", "id": "meeting-1"},
                "source_ids": ["src-1"],
                "confidence": 0.9
            }],
            "attendee_context": [],
            "open_loops": [],
            "what_changed_since_last": [],
            "suggested_outcomes": []
        }));
        let output = harness
            .run(input_with_source("meeting-1", "src-1"))
            .await
            .unwrap();

        assert_eq!(output.data().topics.len(), 1);
        assert_eq!(output.provenance().ability_name, "prepare_meeting");
        assert!(output.provenance().prompt_fingerprint.is_some());
    }

    #[tokio::test]
    async fn prepare_meeting_public_path_builds_live_context_from_claims() {
        let claim = fixture_claim(
            "claim-live-riley",
            "person",
            "person-riley",
            "Riley asked to keep the agenda focused on onboarding risks.",
            "2026-05-05T15:30:00Z",
            Some("2026-05-05T15:30:00Z"),
        );
        let snapshot = fixture_meeting_snapshot(
            "meeting-live",
            vec![PrepareMeetingAttendeeSnapshot {
                name: "Riley Rivera".into(),
                email: Some("riley@example.com".into()),
                person_id: Some("person-riley".into()),
                account_id: None,
                domain: Some("example.com".into()),
            }],
            vec![PrepareMeetingSubjectSnapshot {
                kind: "person".into(),
                id: "person-riley".into(),
                display_name: "Riley Rivera".into(),
            }],
            vec![claim.clone()],
        );
        let harness = Harness::new(serde_json::json!({
            "topics": [],
            "attendee_context": [{
                "attendee": "Riley Rivera",
                "context": "Riley asked to keep the agenda focused on onboarding risks.",
                "subject": {"kind": "person", "id": "person-riley"},
                "source_ids": ["claim-live-riley"],
                "confidence": 0.9
            }],
            "open_loops": [],
            "what_changed_since_last": [],
            "suggested_outcomes": []
        }))
        .with_claims(vec![claim])
        .with_meeting_context(snapshot);

        let output = harness.run(public_input("meeting-live")).await.unwrap();

        assert_eq!(output.data().meeting.id, "meeting-live");
        assert_eq!(output.data().meeting.attendees.len(), 1);
        assert_eq!(output.data().attendee_context.len(), 1);
        assert_eq!(output.provenance().children.len(), 1);
    }

    #[tokio::test]
    async fn prepare_meeting_erased_agent_input_drops_injected_context() {
        let live_claim = fixture_claim(
            "claim-live-meeting",
            "meeting",
            "meeting-erased",
            "The live claim-backed context is the only accepted source.",
            "2026-05-05T15:30:00Z",
            Some("2026-05-05T15:30:00Z"),
        );
        let snapshot = fixture_meeting_snapshot(
            "meeting-erased",
            Vec::new(),
            Vec::new(),
            vec![live_claim.clone()],
        );
        let harness = Harness::new(serde_json::json!({
            "topics": [{
                "title": "Live claim topic",
                "detail": "The live claim-backed context is the only accepted source.",
                "subject": {"kind": "meeting", "id": "meeting-erased"},
                "source_ids": ["claim-live-meeting"],
                "confidence": 0.9
            }],
            "attendee_context": [],
            "open_loops": [],
            "what_changed_since_last": [],
            "suggested_outcomes": []
        }))
        .with_meeting_context(snapshot);

        let output_json = harness
            .run_erased_as_agent(serde_json::json!({
                "meeting_id": "meeting-erased",
                "depth": 2,
                "include_open_loops": true,
                "schema_version": 1,
                "context": {
                    "meeting": {
                        "id": "meeting-erased",
                        "title": "Injected fabricated meeting",
                        "starts_at": "2026-05-06T15:00:00Z",
                        "ends_at": "2026-05-06T15:30:00Z",
                        "attendees": []
                    },
                    "evidence": [{
                        "id": "fabricated-src",
                        "subject": {"kind": "meeting", "id": "meeting-erased"},
                        "claim_type": "meeting_topic",
                        "text": "Fabricated meeting evidence from caller JSON.",
                        "source_asof": "2026-05-05T15:30:00Z",
                        "observed_at": "2026-05-05T15:30:00Z",
                        "data_source": "glean",
                        "lifecycle": "active",
                        "confidence": 0.99,
                        "temporal_scope": "state",
                        "sensitivity": "internal"
                    }],
                    "entity_contexts": []
                }
            }))
            .await
            .unwrap();
        let output: crate::abilities::provenance::AbilityOutput<MeetingBrief> =
            serde_json::from_value(output_json).unwrap();

        assert_eq!(output.data().meeting.title, "Synthetic planning meeting");
        assert_eq!(output.data().topics.len(), 1);
        assert_eq!(output.data().topics[0].title, "Live claim topic");
    }

    #[tokio::test]
    async fn prepare_meeting_source_asof_from_child_composition_is_reachable() {
        let harness = Harness::new(serde_json::json!({
            "topics": [],
            "attendee_context": [{
                "attendee": "Alex Example",
                "context": "Alex owns the rollout path.",
                "subject": {"kind": "person", "id": "person-alex"},
                "source_ids": ["src-person"],
                "confidence": 0.88
            }],
            "open_loops": [],
            "what_changed_since_last": [],
            "suggested_outcomes": []
        }))
        .with_claims(vec![fixture_claim(
            "claim-person-alex",
            "person",
            "person-alex",
            "Alex owns the rollout path.",
            "2026-04-28T12:00:00Z",
            Some("2026-04-28T12:00:00Z"),
        )]);
        let mut input = input_with_source("meeting-1", "src-person");
        let context = input.context.as_mut().unwrap();
        context.evidence[0].subject = BriefSubjectRef::person("person-alex");
        context.entity_contexts.push(EntityContextSeed {
            subject: BriefSubjectRef::person("person-alex"),
            display_name: "Alex Example".into(),
        });

        let output = harness.run(input).await.unwrap();
        let child_source_asof = output.provenance().children[0].provenance.sources[0]
            .source_asof
            .unwrap()
            .to_rfc3339();

        assert_eq!(child_source_asof, "2026-04-28T12:00:00+00:00");
    }

    #[tokio::test]
    async fn prepare_meeting_filters_hierarchy_only_adjacent_entity_context_from_prompt_input() {
        let target_text = "Target Example account wants launch readiness by Friday.";
        let adjacent_text = "Adjacent Example account has an unrelated infrastructure escalation.";
        let target_claim = fixture_claim(
            "claim-target-account",
            "account",
            "acct-target",
            target_text,
            "2026-05-05T15:30:00Z",
            Some("2026-05-05T15:30:00Z"),
        );
        let adjacent_claim = fixture_claim(
            "claim-adjacent-account",
            "account",
            "acct-adjacent",
            adjacent_text,
            "2026-05-05T15:31:00Z",
            Some("2026-05-05T15:31:00Z"),
        );
        let snapshot = fixture_meeting_snapshot(
            "meeting-hierarchy",
            vec![PrepareMeetingAttendeeSnapshot {
                name: "Tara Example".into(),
                email: Some("tara@example.com".into()),
                person_id: Some("person-tara".into()),
                account_id: Some("acct-target".into()),
                domain: Some("example.com".into()),
            }],
            vec![PrepareMeetingSubjectSnapshot {
                kind: "account".into(),
                id: "acct-target".into(),
                display_name: "Target Example".into(),
            }],
            vec![target_claim.clone()],
        );
        let mut related_subjects = BTreeMap::new();
        related_subjects.insert(
            BriefSubjectRef::account("acct-target").key(),
            vec![BriefSubjectRef::account("acct-adjacent")],
        );
        let harness = Harness::new(empty_completion())
            .with_claims(vec![target_claim, adjacent_claim])
            .with_related_subjects(related_subjects)
            .with_meeting_context(snapshot);

        harness
            .run(public_input("meeting-hierarchy"))
            .await
            .unwrap();
        let entity_contexts = captured_entity_contexts(&harness);
        let serialized =
            serde_json::to_string(&entity_contexts).expect("entity contexts serialize");

        assert!(
            serialized.contains(target_text),
            "in-scope target context must remain in PromptContext.entity_contexts"
        );
        assert!(
            !serialized.contains(adjacent_text),
            "hierarchy-only adjacent context must not cross the provider boundary"
        );
        assert!(
            !serialized.contains("acct-adjacent"),
            "adjacent account subject must be absent from PromptContext.entity_contexts"
        );
    }

    #[tokio::test]
    async fn prepare_meeting_filters_private_entity_context_sensitivity_from_prompt_input() {
        let internal_text = "Target Example internal launch context is allowed.";
        let confidential_text = "Target Example confidential renewal risk is private.";
        let user_only_text = "Target Example user-only note is private.";
        let internal_claim = fixture_claim_with_sensitivity(
            "claim-target-internal",
            "account",
            "acct-target",
            internal_text,
            ClaimSensitivity::Internal,
        );
        let confidential_claim = fixture_claim_with_sensitivity(
            "claim-target-confidential",
            "account",
            "acct-target",
            confidential_text,
            ClaimSensitivity::Confidential,
        );
        let user_only_claim = fixture_claim_with_sensitivity(
            "claim-target-user-only",
            "account",
            "acct-target",
            user_only_text,
            ClaimSensitivity::UserOnly,
        );
        let snapshot = fixture_meeting_snapshot(
            "meeting-sensitivity",
            vec![PrepareMeetingAttendeeSnapshot {
                name: "Tara Example".into(),
                email: Some("tara@example.com".into()),
                person_id: Some("person-tara".into()),
                account_id: Some("acct-target".into()),
                domain: Some("example.com".into()),
            }],
            vec![PrepareMeetingSubjectSnapshot {
                kind: "account".into(),
                id: "acct-target".into(),
                display_name: "Target Example".into(),
            }],
            vec![internal_claim.clone()],
        );
        let harness = Harness::new(empty_completion())
            .with_claims(vec![internal_claim, confidential_claim, user_only_claim])
            .with_meeting_context(snapshot);

        harness
            .run(public_input("meeting-sensitivity"))
            .await
            .unwrap();
        let entity_contexts = captured_entity_contexts(&harness);
        let serialized =
            serde_json::to_string(&entity_contexts).expect("entity contexts serialize");

        assert!(
            serialized.contains(internal_text),
            "internal context should remain available to the prompt"
        );
        assert!(
            !serialized.contains(confidential_text),
            "confidential context must not cross the provider boundary"
        );
        assert!(
            !serialized.contains(user_only_text),
            "user-only context must not cross the provider boundary"
        );
    }

    #[tokio::test]
    async fn prepare_meeting_subject_bleed_blocks_wrong_account_claim() {
        let harness = Harness::new(serde_json::json!({
            "topics": [{
                "title": "Wrong account topic",
                "detail": "This should not attach to Account A.",
                "subject": {"kind": "account", "id": "acct-a"},
                "source_ids": ["src-b"],
                "confidence": 0.92
            }],
            "attendee_context": [],
            "open_loops": [],
            "what_changed_since_last": [],
            "suggested_outcomes": []
        }));
        let mut input = input_with_source("meeting-1", "src-b");
        let context = input.context.as_mut().unwrap();
        context.meeting.attendees = vec![
            attendee_for_account("A Owner", "owner-a@shared.example.com", "acct-a"),
            attendee_for_account("B Owner", "owner-b@shared.example.com", "acct-b"),
        ];
        context.evidence[0].subject = BriefSubjectRef::account("acct-b");

        let output = harness.run(input).await.unwrap();

        assert!(output.data().topics.is_empty());
        assert!(output.provenance().warnings.iter().any(|warning| {
            matches!(
                warning,
                ProvenanceWarning::SubjectFitQualified { status, .. }
                    if status == "SubjectAmbiguous"
            )
        }));
    }

    #[tokio::test]
    async fn prepare_meeting_meeting_subject_blocks_adjacent_account_source() {
        let harness = Harness::new(serde_json::json!({
            "topics": [{
                "title": "Adjacent account escalation",
                "detail": "This source belongs to the adjacent account.",
                "subject": {"kind": "meeting", "id": "meeting-1"},
                "source_ids": ["src-adjacent"],
                "confidence": 0.91
            }],
            "attendee_context": [],
            "open_loops": [],
            "what_changed_since_last": [],
            "suggested_outcomes": []
        }));
        let mut input = input_with_source("meeting-1", "src-adjacent");
        let context = input.context.as_mut().unwrap();
        context.meeting.attendees = vec![attendee_for_account(
            "Target Owner",
            "owner@target.example.com",
            "acct-target",
        )];
        context.evidence[0].subject = BriefSubjectRef::account("acct-adjacent");

        let output = harness.run(input).await.unwrap();

        assert!(output.data().topics.is_empty());
        assert!(output.provenance().warnings.iter().any(|warning| {
            matches!(
                warning,
                ProvenanceWarning::SubjectFitQualified { status, .. }
                    if status == "SubjectAmbiguous"
            )
        }));
    }

    #[tokio::test]
    async fn prepare_meeting_blocks_direct_adjacent_account_subject_source() {
        let harness = Harness::new(serde_json::json!({
            "topics": [{
                "title": "Adjacent account escalation",
                "detail": "This source and candidate both belong to the adjacent account.",
                "subject": {"kind": "account", "id": "acct-adjacent"},
                "source_ids": ["src-adjacent"],
                "confidence": 0.91
            }],
            "attendee_context": [],
            "open_loops": [],
            "what_changed_since_last": [],
            "suggested_outcomes": []
        }));
        let mut input = input_with_source("meeting-1", "src-adjacent");
        let context = input.context.as_mut().unwrap();
        context.meeting.attendees = vec![attendee_for_account(
            "Target Owner",
            "owner@target.example.com",
            "acct-target",
        )];
        context.evidence[0].subject = BriefSubjectRef::account("acct-adjacent");

        let output = harness.run(input).await.unwrap();

        assert!(output.data().topics.is_empty());
        assert!(output.provenance().warnings.iter().any(|warning| {
            matches!(
                warning,
                ProvenanceWarning::SubjectFitQualified { status, .. }
                    if status == "SubjectAmbiguous"
            )
        }));
    }

    #[tokio::test]
    async fn prepare_meeting_accepts_linked_project_source_subject() {
        let claim = fixture_claim(
            "claim-linked-project",
            "project",
            "proj-linked",
            "The linked project milestone needs launch owner confirmation.",
            "2026-05-05T16:00:00Z",
            Some("2026-05-05T16:00:00Z"),
        );
        let snapshot = fixture_meeting_snapshot(
            "meeting-linked-project",
            vec![PrepareMeetingAttendeeSnapshot {
                name: "Mina Example".into(),
                email: Some("mina@example.com".into()),
                person_id: Some("person-mina".into()),
                account_id: None,
                domain: Some("example.com".into()),
            }],
            vec![PrepareMeetingSubjectSnapshot {
                kind: "project".into(),
                id: "proj-linked".into(),
                display_name: "Launch Project".into(),
            }],
            vec![claim.clone()],
        );
        let harness = Harness::new(serde_json::json!({
            "topics": [{
                "title": "Confirm launch owner",
                "detail": "The linked project milestone needs launch owner confirmation.",
                "subject": {"kind": "project", "id": "proj-linked"},
                "source_ids": ["claim-linked-project"],
                "confidence": 0.91
            }],
            "attendee_context": [],
            "open_loops": [],
            "what_changed_since_last": [],
            "suggested_outcomes": []
        }))
        .with_claims(vec![claim])
        .with_meeting_context(snapshot);

        let output = harness
            .run(public_input("meeting-linked-project"))
            .await
            .unwrap();

        assert_eq!(output.data().topics.len(), 1);
        assert_eq!(
            output.data().topics[0].subject,
            BriefSubjectRef::project("proj-linked")
        );
    }

    #[tokio::test]
    async fn prepare_meeting_blocks_unlinked_adjacent_project_source() {
        let claim = fixture_claim(
            "claim-adjacent-project",
            "project",
            "proj-adjacent",
            "The adjacent project escalation does not belong in this meeting.",
            "2026-05-05T16:00:00Z",
            Some("2026-05-05T16:00:00Z"),
        );
        let snapshot = fixture_meeting_snapshot(
            "meeting-linked-project",
            vec![PrepareMeetingAttendeeSnapshot {
                name: "Mina Example".into(),
                email: Some("mina@example.com".into()),
                person_id: Some("person-mina".into()),
                account_id: None,
                domain: Some("example.com".into()),
            }],
            vec![PrepareMeetingSubjectSnapshot {
                kind: "project".into(),
                id: "proj-linked".into(),
                display_name: "Launch Project".into(),
            }],
            vec![claim.clone()],
        );
        let harness = Harness::new(serde_json::json!({
            "topics": [{
                "title": "Adjacent project escalation",
                "detail": "The adjacent project escalation does not belong in this meeting.",
                "subject": {"kind": "project", "id": "proj-adjacent"},
                "source_ids": ["claim-adjacent-project"],
                "confidence": 0.91
            }],
            "attendee_context": [],
            "open_loops": [],
            "what_changed_since_last": [],
            "suggested_outcomes": []
        }))
        .with_claims(vec![claim])
        .with_meeting_context(snapshot);

        let output = harness
            .run(public_input("meeting-linked-project"))
            .await
            .unwrap();

        assert!(output.data().topics.is_empty());
        assert!(output.provenance().warnings.iter().any(|warning| {
            matches!(
                warning,
                ProvenanceWarning::SubjectFitQualified { status, .. }
                    if status == "SubjectAmbiguous"
            )
        }));
    }

    #[tokio::test]
    async fn prepare_meeting_revoked_source_masks_rendered_fact() {
        let harness = Harness::new(serde_json::json!({
            "topics": [{
                "title": "Revoked evidence",
                "detail": "This should be suppressed.",
                "subject": {"kind": "meeting", "id": "meeting-1"},
                "source_ids": ["src-1"],
                "confidence": 0.9
            }],
            "attendee_context": [],
            "open_loops": [],
            "what_changed_since_last": [],
            "suggested_outcomes": []
        }));
        let mut input = input_with_source("meeting-1", "src-1");
        input.context.as_mut().unwrap().evidence[0].lifecycle = "revoked".into();

        let output = harness.run(input).await.unwrap();

        assert!(output.data().topics.is_empty());
        assert!(output.provenance().warnings.iter().any(|warning| {
            matches!(
                warning,
                ProvenanceWarning::Masked {
                    reason: MaskReason::SourceRevoked
                }
            )
        }));
    }

    #[tokio::test]
    async fn prepare_meeting_change_marker_uses_meeting_point_in_time_scope() {
        let harness = Harness::new(serde_json::json!({
            "topics": [],
            "attendee_context": [],
            "open_loops": [],
            "what_changed_since_last": [{
                "description": "The launch date moved.",
                "subject": {"kind": "meeting", "id": "meeting-1"},
                "source_ids": ["src-1"],
                "confidence": 0.9
            }],
            "suggested_outcomes": []
        }));
        let output = harness
            .run(input_with_source("meeting-1", "src-1"))
            .await
            .unwrap();

        assert_eq!(
            output.data().what_changed_since_last[0].temporal_scope,
            BriefTemporalScope::PointInTime {
                occurred_at: "2026-05-06T15:00:00Z".into()
            }
        );
    }

    fn input_with_source(meeting_id: &str, source_id: &str) -> PrepareMeetingInput {
        PrepareMeetingInput {
            meeting_id: meeting_id.into(),
            depth: 2,
            include_open_loops: true,
            schema_version: SchemaVersion(1),
            context: Some(MeetingBriefContext {
                meeting: MeetingSummary {
                    id: meeting_id.into(),
                    title: "Synthetic planning meeting".into(),
                    starts_at: Some("2026-05-06T15:00:00Z".into()),
                    ends_at: Some("2026-05-06T15:30:00Z".into()),
                    attendees: vec![MeetingAttendee {
                        name: "Alex Example".into(),
                        email: Some("alex@example.com".into()),
                        person_id: Some("person-alex".into()),
                        account_id: None,
                        domain: Some("example.com".into()),
                    }],
                },
                evidence: vec![EvidenceSource {
                    id: source_id.into(),
                    subject: BriefSubjectRef::meeting(meeting_id),
                    claim_type: ClaimType::MeetingTopic.as_str().into(),
                    text: "The rollout owner is unsettled.".into(),
                    source_asof: Some("2026-05-01T12:00:00Z".into()),
                    observed_at: "2026-05-01T12:00:00Z".into(),
                    data_source: "glean".into(),
                    lifecycle: "active".into(),
                    confidence: 0.9,
                    temporal_scope: "state".into(),
                    sensitivity: "internal".into(),
                }],
                entity_contexts: Vec::new(),
            }),
        }
    }

    fn empty_completion() -> serde_json::Value {
        serde_json::json!({
            "topics": [],
            "attendee_context": [],
            "open_loops": [],
            "what_changed_since_last": [],
            "suggested_outcomes": []
        })
    }

    fn captured_entity_contexts(harness: &Harness) -> serde_json::Value {
        harness
            .captured_prompt()
            .canonical_json_inputs
            .expect("prepare_meeting prompt has canonical JSON inputs")
            .pointer("/context/entity_contexts")
            .cloned()
            .expect("canonical PromptContext.entity_contexts exists")
    }

    fn attendee_for_account(name: &str, email: &str, account_id: &str) -> MeetingAttendee {
        MeetingAttendee {
            name: name.into(),
            email: Some(email.into()),
            person_id: None,
            account_id: Some(account_id.into()),
            domain: email.split_once('@').map(|(_, domain)| domain.to_string()),
        }
    }

    fn public_input(meeting_id: &str) -> PrepareMeetingInput {
        PrepareMeetingInput {
            meeting_id: meeting_id.into(),
            depth: 2,
            include_open_loops: true,
            schema_version: SchemaVersion(1),
            context: None,
        }
    }

    fn fixture_meeting_snapshot(
        meeting_id: &str,
        attendees: Vec<PrepareMeetingAttendeeSnapshot>,
        subjects: Vec<PrepareMeetingSubjectSnapshot>,
        claims: Vec<IntelligenceClaim>,
    ) -> PrepareMeetingContextSnapshot {
        PrepareMeetingContextSnapshot {
            meeting: PrepareMeetingSnapshot {
                id: meeting_id.into(),
                title: "Synthetic planning meeting".into(),
                starts_at: Some("2026-05-06T15:00:00Z".into()),
                ends_at: Some("2026-05-06T15:30:00Z".into()),
                attendees_raw: None,
            },
            attendees,
            subjects,
            claims,
        }
    }

    fn fixture_claim(
        id: &str,
        entity_type: &str,
        entity_id: &str,
        text: &str,
        created_at: &str,
        source_asof: Option<&str>,
    ) -> IntelligenceClaim {
        IntelligenceClaim {
            id: id.to_string(),
            subject_ref: serde_json::json!({
                "kind": entity_type,
                "id": entity_id,
            })
            .to_string(),
            claim_type: "entity_summary".to_string(),
            field_path: Some("summary".to_string()),
            topic_key: None,
            text: text.to_string(),
            dedup_key: format!("dedup-{id}"),
            item_hash: Some(format!("hash-{id}")),
            actor: "agent:test".to_string(),
            data_source: "user".to_string(),
            source_ref: Some(format!("source-{id}")),
            source_asof: source_asof.map(str::to_string),
            observed_at: created_at.to_string(),
            created_at: created_at.to_string(),
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
            sensitivity: ClaimSensitivity::Internal,
            verification_state: ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        }
    }

    fn fixture_claim_with_sensitivity(
        id: &str,
        entity_type: &str,
        entity_id: &str,
        text: &str,
        sensitivity: ClaimSensitivity,
    ) -> IntelligenceClaim {
        let mut claim = fixture_claim(
            id,
            entity_type,
            entity_id,
            text,
            "2026-05-05T15:30:00Z",
            Some("2026-05-05T15:30:00Z"),
        );
        claim.sensitivity = sensitivity;
        claim
    }
}

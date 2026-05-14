use dailyos_abilities_macro::ability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::abilities::provenance::source_time::{parse_source_timestamp, SourceTimestampStatus};
use crate::abilities::provenance::trust::claim_trust_band_from_score;
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, ChunkId, ContextEntryId, DataSource, DocumentId,
    EntityId, FieldAttribution, FieldPath, GleanDownstream, MeetingId, ProvenanceBuilder,
    ProvenanceBuilderConfig, ProvenanceWarning, SchemaVersion, SourceAttribution,
    SourceIdentifier, SourceName, SubjectAttribution, SubjectRef,
};
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
    ClaimType,
};
use crate::sensitivity::{
    renderable_claim_text_with_value, ClaimDismissalSurface, RenderActor, RenderSurface,
};
use crate::services::context::{ListOpenLoopsQuery, ListOpenLoopsReadError};
use crate::types::{
    prompt_input_sensitivity_allowed, subject_ref_from_json, ClaimSubjectRef, IntelligenceClaim,
};

const ABILITY_NAME: &str = "list_open_loops";
const ABILITY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ListOpenLoopsInput {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct OpenLoopsResult {
    pub loops: Vec<OpenLoop>,
    pub schema_version: SchemaVersion,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct OpenLoop {
    pub id: String,
    pub subject: OpenLoopSubject,
    pub loop_kind: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_asof: Option<String>,
    pub claim_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct OpenLoopSubject {
    pub entity_type: String,
    pub entity_id: String,
}

#[ability(
    name = "list_open_loops",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, Agent, System],
    allowed_modes = [Live, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.open_loops"],
    mcp_exposure = Invocable,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn list_open_loops(
    ctx: &AbilityContext<'_>,
    input: ListOpenLoopsInput,
) -> AbilityResult<OpenLoopsResult> {
    validate_schema_version(input.schema_version)?;
    let entity_filter = normalize_entity_filter(&input)?;
    let subject_ref = subject_ref_for_filter(entity_filter.as_ref());

    let snapshot = ctx
        .services()
        .read_list_open_loops(ListOpenLoopsQuery {
            entity_type: entity_filter
                .as_ref()
                .map(|(entity_type, _)| entity_type.clone()),
            entity_id: entity_filter
                .as_ref()
                .map(|(_, entity_id)| entity_id.clone()),
            surface: ctx.entity_context_claim_surface(),
        })
        .await
        .map_err(read_error)?;

    let mut eligible_claims = snapshot
        .claims
        .into_iter()
        .filter(loop_claim_allowed)
        .collect::<Vec<_>>();
    eligible_claims.sort_by(|left, right| {
        (
            left.source_asof.as_deref(),
            left.observed_at.as_str(),
            left.created_at.as_str(),
            left.id.as_str(),
        )
            .cmp(&(
                right.source_asof.as_deref(),
                right.observed_at.as_str(),
                right.created_at.as_str(),
                right.id.as_str(),
            ))
    });

    let mut builder = ProvenanceBuilder::new(provenance_config(ctx, input.schema_version));
    let render_surface = render_surface_for_context(ctx);
    let render_actor = render_actor_for_context(ctx);
    let mut produced = Vec::<(IntelligenceClaim, OpenLoop)>::new();

    for claim in eligible_claims {
        if primary_source_revoked(&claim) {
            builder.add_warning(ProvenanceWarning::SourceRevoked);
            continue;
        }
        if !prompt_input_sensitivity_allowed(&claim.sensitivity) {
            continue;
        }
        let Some(open_loop) = open_loop_for_claim(&claim, render_surface, &render_actor)? else {
            continue;
        };
        produced.push((claim, open_loop));
    }

    let loops = produced
        .iter()
        .map(|(_, open_loop)| open_loop.clone())
        .collect::<Vec<_>>();
    let subject = envelope_subject(subject_ref, &loops)?;
    builder.set_subject(subject.clone());

    if loops.is_empty() {
        builder
            .attribute(
                FieldPath::new("/loops").map_err(field_error)?,
                FieldAttribution::constant(subject.clone()),
            )
            .map_err(provenance_error)?;
    }

    for (index, (claim, open_loop)) in produced.iter().enumerate() {
        let loop_subject = SubjectAttribution::direct_confident(subject_ref_for(
            &open_loop.subject.entity_type,
            &open_loop.subject.entity_id,
        )?);
        let source_index = builder.add_source(source_for_claim(ctx, claim, open_loop)?);
        builder.set_source_trust_band(source_index, claim_trust_band_from_score(claim.trust_score));
        builder
            .attribute_subtree(
                FieldPath::new(format!("/loops/{index}")).map_err(field_error)?,
                FieldAttribution::direct(loop_subject, source_index),
            )
            .map_err(provenance_error)?;
    }

    builder
        .attribute(
            FieldPath::new("/schema_version").map_err(field_error)?,
            FieldAttribution::constant(subject),
        )
        .map_err(provenance_error)?;

    let output = OpenLoopsResult {
        loops,
        schema_version: SchemaVersion(ABILITY_SCHEMA_VERSION),
    };
    builder.finalize(output).map_err(provenance_error)
}

fn validate_schema_version(schema_version: u32) -> Result<(), AbilityError> {
    if schema_version == ABILITY_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(validation_error(format!(
            "unsupported schema_version `{schema_version}` for `{ABILITY_NAME}`"
        )))
    }
}

fn normalize_entity_filter(
    input: &ListOpenLoopsInput,
) -> Result<Option<(String, String)>, AbilityError> {
    match (input.entity_type.as_deref(), input.entity_id.as_deref()) {
        (None, None) => Ok(None),
        (Some(entity_type), Some(entity_id)) => {
            let entity_type = entity_type.trim();
            let entity_id = entity_id.trim();
            if entity_type.is_empty() {
                return Err(validation_error("entity_type must be non-empty"));
            }
            if entity_id.is_empty() {
                return Err(validation_error("entity_id must be non-empty"));
            }
            subject_ref_for(entity_type, entity_id)?;
            Ok(Some((entity_type.to_string(), entity_id.to_string())))
        }
        (Some(_), None) => Err(validation_error(
            "entity_id is required when entity_type is provided",
        )),
        (None, Some(_)) => Err(validation_error(
            "entity_type is required when entity_id is provided",
        )),
    }
}

fn subject_ref_for_filter(filter: Option<&(String, String)>) -> SubjectRef {
    match filter {
        Some((entity_type, entity_id)) => {
            subject_ref_for(entity_type, entity_id).expect("entity filter was validated")
        }
        None => SubjectRef::Global,
    }
}

fn subject_ref_for(entity_type: &str, entity_id: &str) -> Result<SubjectRef, AbilityError> {
    if entity_id.trim().is_empty() {
        return Err(validation_error("entity_id must be non-empty"));
    }

    match entity_type {
        "account" => Ok(SubjectRef::Account(entity_id.to_string())),
        "project" => Ok(SubjectRef::Project(entity_id.to_string())),
        "person" => Ok(SubjectRef::Person(entity_id.to_string())),
        "meeting" => Ok(SubjectRef::Meeting(entity_id.to_string())),
        other => Err(validation_error(format!(
            "unsupported entity_type `{other}` for `{ABILITY_NAME}`"
        ))),
    }
}

fn loop_claim_allowed(claim: &IntelligenceClaim) -> bool {
    [ClaimType::OpenLoop, ClaimType::Commitment]
        .iter()
        .any(|claim_type| claim.claim_type == claim_type.as_str())
}

fn envelope_subject(
    root: SubjectRef,
    loops: &[OpenLoop],
) -> Result<SubjectAttribution, AbilityError> {
    let mut subjects = Vec::new();
    if !matches!(root, SubjectRef::Global) {
        push_subject(&mut subjects, root);
    }
    for open_loop in loops {
        push_subject(
            &mut subjects,
            subject_ref_for(&open_loop.subject.entity_type, &open_loop.subject.entity_id)?,
        );
    }

    let subject = match subjects.len() {
        0 => SubjectRef::Global,
        1 => subjects.into_iter().next().expect("one subject"),
        _ => SubjectRef::Multi(subjects),
    };
    Ok(SubjectAttribution::direct_confident(subject))
}

fn push_subject(subjects: &mut Vec<SubjectRef>, subject: SubjectRef) {
    if !subjects.iter().any(|existing| existing == &subject) {
        subjects.push(subject);
    }
}

fn render_surface_for_context(ctx: &AbilityContext<'_>) -> RenderSurface {
    match ctx.entity_context_claim_surface() {
        ClaimDismissalSurface::TauriEntityDetail => RenderSurface::TauriEntityDetail,
        ClaimDismissalSurface::Briefing => RenderSurface::TauriBriefingPrep,
        ClaimDismissalSurface::TauriMeetingDetail => RenderSurface::TauriMeetingDetail,
        ClaimDismissalSurface::TauriEmailSummary => RenderSurface::TauriEmailSummary,
        ClaimDismissalSurface::Action => RenderSurface::Action,
        ClaimDismissalSurface::TauriProvenance => RenderSurface::TauriProvenance,
        ClaimDismissalSurface::TauriReport => RenderSurface::TauriReport,
        ClaimDismissalSurface::TauriChat => RenderSurface::TauriChat,
        ClaimDismissalSurface::McpTool => RenderSurface::McpTool,
        ClaimDismissalSurface::McpToolDetail => RenderSurface::McpToolDetail,
        ClaimDismissalSurface::P2Publication => RenderSurface::P2Publication,
        ClaimDismissalSurface::LogStructured => RenderSurface::LogStructured,
        ClaimDismissalSurface::PushNotification => RenderSurface::PushNotification,
        ClaimDismissalSurface::Worker | ClaimDismissalSurface::Eval => {
            if matches!(&ctx.actor, Actor::Agent) {
                RenderSurface::McpTool
            } else {
                RenderSurface::TauriEntityDetail
            }
        }
    }
}

fn render_actor_for_context(ctx: &AbilityContext<'_>) -> RenderActor {
    match &ctx.actor {
        Actor::User => RenderActor::user(ctx.services().actor, Some(ctx.services().actor)),
        Actor::Agent => RenderActor::agent("agent:list_open_loops"),
        Actor::Admin => RenderActor {
            actor: "admin".to_string(),
            user_id: None,
        },
        Actor::System => RenderActor {
            actor: "system".to_string(),
            user_id: None,
        },
        Actor::SurfaceClient { .. } => todo!("W1-B+ wiring for Actor::SurfaceClient"),
    }
}

fn open_loop_for_claim(
    claim: &IntelligenceClaim,
    render_surface: RenderSurface,
    render_actor: &RenderActor,
) -> Result<Option<OpenLoop>, AbilityError> {
    let Some(rendered_description) =
        renderable_claim_text_with_value(claim, &claim.text, render_surface, render_actor)
    else {
        return Ok(None);
    };
    let (entity_type, entity_id) = claim_subject_identity(claim)?;
    let metadata = claim_metadata(claim);

    Ok(Some(OpenLoop {
        id: claim.id.clone(),
        subject: OpenLoopSubject {
            entity_type,
            entity_id,
        },
        loop_kind: metadata_string(metadata.as_ref(), &["loop_kind", "loopKind"])
            .unwrap_or_else(|| claim.claim_type.clone()),
        description: rendered_description.text,
        owner: metadata_string(
            metadata.as_ref(),
            &["owner", "owner_raw", "ownerRaw", "assignee"],
        ),
        due_date: metadata_string(
            metadata.as_ref(),
            &["due_date", "dueDate", "due_normalized", "dueNormalized"],
        ),
        status: metadata_string(metadata.as_ref(), &["status", "state"]),
        source_asof: claim.source_asof.clone(),
        claim_type: claim.claim_type.clone(),
    }))
}

fn claim_subject_identity(claim: &IntelligenceClaim) -> Result<(String, String), AbilityError> {
    let value: Value = serde_json::from_str(&claim.subject_ref)
        .map_err(|error| validation_error(format!("invalid claim subject_ref JSON: {error}")))?;
    match subject_ref_from_json(&value)
        .map_err(|error| validation_error(format!("invalid claim subject_ref: {error}")))?
    {
        ClaimSubjectRef::Account { id } => Ok(("account".to_string(), id)),
        ClaimSubjectRef::Meeting { id } => Ok(("meeting".to_string(), id)),
        ClaimSubjectRef::Person { id } => Ok(("person".to_string(), id)),
        ClaimSubjectRef::Project { id } => Ok(("project".to_string(), id)),
        ClaimSubjectRef::Email { .. } | ClaimSubjectRef::Multi(_) | ClaimSubjectRef::Global => {
            Err(validation_error(format!(
                "claim `{}` has unsupported open loop subject",
                claim.id
            )))
        }
    }
}

fn claim_metadata(claim: &IntelligenceClaim) -> Option<Value> {
    claim
        .metadata_json
        .as_deref()
        .and_then(|raw| serde_json::from_str(raw).ok())
}

fn metadata_string(metadata: Option<&Value>, keys: &[&str]) -> Option<String> {
    let object = metadata?.as_object()?;
    keys.iter().find_map(|key| {
        object
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    })
}

fn primary_source_revoked(claim: &IntelligenceClaim) -> bool {
    source_json_marks_revoked(claim.source_ref.as_deref())
        || source_json_marks_revoked(claim.metadata_json.as_deref())
}

fn source_json_marks_revoked(raw: Option<&str>) -> bool {
    let Some(raw) = raw else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return false;
    };
    value_marks_revoked_source(&value)
}

fn value_marks_revoked_source(value: &Value) -> bool {
    if bool_field(value, &["source_revoked", "sourceRevoked", "primary_source_revoked"]) {
        return true;
    }
    if lifecycle_field_revoked(
        value,
        &[
            "source_lifecycle_state",
            "sourceLifecycleState",
            "source_lifecycle",
            "sourceLifecycle",
            "lifecycle_state",
            "lifecycleState",
            "lifecycle",
        ],
    ) {
        return true;
    }

    [
        "primary_source",
        "primarySource",
        "source",
        "source_ref",
        "sourceRef",
        "item_source",
        "itemSource",
    ]
    .iter()
    .any(|key| {
        value
            .get(*key)
            .is_some_and(value_marks_revoked_source)
    })
}

fn bool_field(value: &Value, keys: &[&str]) -> bool {
    keys.iter()
        .any(|key| value.get(*key).and_then(Value::as_bool).unwrap_or(false))
}

fn lifecycle_field_revoked(value: &Value, keys: &[&str]) -> bool {
    keys.iter().any(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .is_some_and(|state| state.trim().eq_ignore_ascii_case("revoked"))
    })
}

fn source_for_claim(
    ctx: &AbilityContext<'_>,
    claim: &IntelligenceClaim,
    open_loop: &OpenLoop,
) -> Result<SourceAttribution, AbilityError> {
    let now = ctx.services().clock.now();
    let (observed_at, source_asof) = parsed_claim_timestamp(claim, now);
    SourceAttribution::new(
        data_source_for_claim(&claim.data_source),
        vec![source_identifier_for_claim(claim, open_loop)],
        observed_at,
        source_asof,
        1.0,
        None,
    )
    .map_err(|error| validation_error(format!("invalid source attribution: {error}")))
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

fn source_identifier_for_claim(
    claim: &IntelligenceClaim,
    open_loop: &OpenLoop,
) -> SourceIdentifier {
    let source_ref = claim
        .source_ref
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    match claim.data_source.trim().to_ascii_lowercase().as_str() {
        "google" => SourceIdentifier::Meeting {
            meeting_id: MeetingId::new(source_ref.unwrap_or(&claim.id).to_string()),
        },
        "glean" => SourceIdentifier::Document {
            document_id: DocumentId::new(source_ref.unwrap_or(&claim.id).to_string()),
            chunk_id: claim
                .thread_id
                .as_ref()
                .map(|thread_id| ChunkId::new(thread_id.clone())),
        },
        "user" | "human" | "manual" => SourceIdentifier::UserEntry {
            entry_id: ContextEntryId::new(source_ref.unwrap_or(&claim.id).to_string()),
        },
        _ => SourceIdentifier::Entity {
            entity_id: EntityId::new(open_loop.subject.entity_id.clone()),
            field: Some(
                claim
                    .field_path
                    .clone()
                    .unwrap_or_else(|| claim.claim_type.clone()),
            ),
        },
    }
}

fn parsed_claim_timestamp(
    claim: &IntelligenceClaim,
    now: chrono::DateTime<chrono::Utc>,
) -> (
    chrono::DateTime<chrono::Utc>,
    Option<chrono::DateTime<chrono::Utc>>,
) {
    let source_asof = parse_claim_timestamp(claim.source_asof.as_deref(), now);
    for candidate in [claim.observed_at.as_str(), claim.created_at.as_str()] {
        match parse_source_timestamp(Some(candidate), now, None) {
            SourceTimestampStatus::Accepted(parsed)
            | SourceTimestampStatus::Implausible { parsed, .. } => {
                return (parsed, source_asof);
            }
            SourceTimestampStatus::Malformed(_) | SourceTimestampStatus::Missing => {}
        }
    }

    (now, source_asof)
}

fn parse_claim_timestamp(
    candidate: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    match parse_source_timestamp(candidate, now, None) {
        SourceTimestampStatus::Accepted(parsed)
        | SourceTimestampStatus::Implausible { parsed, .. } => Some(parsed),
        SourceTimestampStatus::Malformed(_) | SourceTimestampStatus::Missing => None,
    }
}

fn provenance_config(ctx: &AbilityContext<'_>, schema_version: u32) -> ProvenanceBuilderConfig {
    let mut config = ProvenanceBuilderConfig::new(ABILITY_NAME, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(schema_version);
    config.actor = provenance_actor(ctx.actor.clone());
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Read;
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
        Actor::SurfaceClient { .. } => todo!("W1-B+ wiring for Actor::SurfaceClient"),
    }
}

fn read_error(error: ListOpenLoopsReadError) -> AbilityError {
    match error {
        ListOpenLoopsReadError::SubjectNotOwned {
            entity_type,
            entity_id,
        } => AbilityError {
            kind: AbilityErrorKind::SubjectNotOwned,
            message: format!("subject is not owned by this workspace: {entity_type}:{entity_id}"),
        },
        ListOpenLoopsReadError::ReadFailed(message) => {
            hard_error("list_open_loops_read_failed", message)
        }
    }
}

fn validation_error(message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: message.into(),
    }
}

fn hard_error(code: impl Into<String>, message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::HardError(code.into()),
        message: message.into(),
    }
}

fn provenance_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("provenance construction failed: {error}"))
}

fn field_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("field attribution path failed: {error}"))
}

use std::collections::BTreeMap;

use dailyos_abilities_macro::ability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::provenance::source_time::{parse_source_timestamp, SourceTimestampStatus};
use crate::abilities::provenance::trust::claim_trust_band_from_score;
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, ChunkId, Confidence, ContextEntryId, DataSource,
    DocumentId, EntityId, FieldAttribution, FieldPath, GleanDownstream, MeetingId,
    ProvenanceBuilder, ProvenanceBuilderConfig, SchemaVersion, SourceAttribution, SourceIdentifier,
    SourceIndex, SourceName, SourceRef, SubjectAttribution, SubjectRef,
};
use crate::abilities::temporal::{TrajectoryBundle, TrajectoryQueryDepth, DEEP_LIMIT_WEEKS};
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};
use crate::sensitivity::{renderable_claim_text_with_value, RenderActor, RenderSurface};
use crate::types::{
    claim_allowed_for_prompt_input, subject_ref_from_json, ClaimSubjectRef, EntityContextEntry,
    EntityContextText, IntelligenceClaim,
};

const ABILITY_NAME: &str = "get_entity_context";
const ABILITY_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextDepth {
    Shallow,
    Standard,
    Deep,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct GetEntityContextInput {
    pub schema_version: u32,
    pub entity_type: String,
    pub entity_id: String,
    pub depth: ContextDepth,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetEntityContextOutput {
    pub entries: Vec<EntityContextEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trajectory: Option<TrajectoryBundle>,
}

#[ability(
    name = "get_entity_context",
    category = Read,
    version = "1.0.0",
    schema_version = 2,
    allowed_actors = [User, Agent, System],
    allowed_modes = [Live, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn get_entity_context(
    ctx: &AbilityContext<'_>,
    input: GetEntityContextInput,
) -> AbilityResult<GetEntityContextOutput> {
    validate_schema_version(input.schema_version)?;
    let subject_ref = subject_ref_for(&input.entity_type, &input.entity_id)?;
    let subject = SubjectAttribution::direct_confident(subject_ref.clone());
    let claims = ctx
        .services()
        .read_entity_context_claims(
            input.entity_type.clone(),
            input.entity_id.clone(),
            ctx.entity_context_claim_surface(),
            input.depth.claim_levels(),
        )
        .await
        .map_err(|error| hard_error("entity context claim read failed", error))?;
    let claims = filter_claims_for_actor(ctx.actor.clone(), claims);
    let render_actor = render_actor_for_context(ctx);
    let entries = claims
        .iter()
        .map(|claim| entry_for_claim(claim, &render_actor))
        .collect::<Result<Vec<_>, _>>()?;
    let mut trajectory = trajectory_for_depth(ctx, &input).await?;

    let mut builder = ProvenanceBuilder::new(provenance_config(ctx, input.schema_version));
    builder.set_subject(envelope_subject(subject_ref, &entries)?);

    if entries.is_empty() {
        builder
            .attribute(
                FieldPath::new("/entries").map_err(field_error)?,
                FieldAttribution::constant(subject.clone()),
            )
            .map_err(provenance_error)?;
    }

    for (index, (claim, entry)) in claims.iter().zip(entries.iter()).enumerate() {
        let entry_subject = SubjectAttribution::direct_confident(subject_ref_for(
            &entry.entity_type,
            &entry.entity_id,
        )?);
        let source_index = builder.add_source(source_for_claim(ctx, claim, entry)?);
        builder.set_source_trust_band(source_index, claim_trust_band_from_score(claim.trust_score));
        builder
            .attribute_subtree(
                FieldPath::new(format!("/entries/{index}")).map_err(field_error)?,
                FieldAttribution::direct(entry_subject, source_index),
            )
            .map_err(provenance_error)?;
    }

    if let Some(bundle) = trajectory.as_mut() {
        if bundle.is_empty() {
            builder
                .attribute(
                    FieldPath::new("/trajectory").map_err(field_error)?,
                    FieldAttribution::constant(subject),
                )
                .map_err(provenance_error)?;
        } else {
            attribute_trajectory(&mut builder, bundle, subject, ctx.services().clock.now())?;
        }
    }

    let output = GetEntityContextOutput {
        entries,
        trajectory,
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

impl ContextDepth {
    fn claim_levels(&self) -> usize {
        match self {
            Self::Shallow => 1,
            Self::Standard => 2,
            Self::Deep => 3,
        }
    }

    fn trajectory_depth(&self) -> TrajectoryQueryDepth {
        match self {
            Self::Shallow => TrajectoryQueryDepth::None,
            Self::Standard => TrajectoryQueryDepth::Latest,
            Self::Deep => TrajectoryQueryDepth::Weeks(DEEP_LIMIT_WEEKS),
        }
    }
}

async fn trajectory_for_depth(
    ctx: &AbilityContext<'_>,
    input: &GetEntityContextInput,
) -> Result<Option<TrajectoryBundle>, AbilityError> {
    let depth = input.depth.trajectory_depth();
    if matches!(depth, TrajectoryQueryDepth::None) {
        return Ok(None);
    }

    ctx.services()
        .read_trajectory_bundle(input.entity_type.clone(), input.entity_id.clone(), depth)
        .await
        .map(Some)
        .map_err(|error| hard_error("trajectory read failed", error))
}

fn attribute_trajectory(
    builder: &mut ProvenanceBuilder,
    bundle: &TrajectoryBundle,
    subject: SubjectAttribution,
    legacy_observed_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), AbilityError> {
    let mut source_indexes = BTreeMap::<String, SourceIndex>::new();

    if let Some(snapshot) = bundle.engagement_curve.as_ref() {
        let refs = snapshot
            .series
            .iter()
            .flat_map(|point| point.source_refs.iter())
            .collect::<Vec<_>>();
        let attribution = attribution_for_trajectory_refs(
            builder,
            &mut source_indexes,
            &subject,
            &refs,
            "temporal_engagement_curve",
            legacy_observed_at,
        )?;
        builder
            .attribute_subtree(
                FieldPath::new("/trajectory/engagement_curve").map_err(field_error)?,
                attribution,
            )
            .map_err(provenance_error)?;

        for (index, point) in snapshot.series.iter().enumerate() {
            let refs = point.source_refs.iter().collect::<Vec<_>>();
            let attribution = attribution_for_trajectory_refs(
                builder,
                &mut source_indexes,
                &subject,
                &refs,
                "temporal_engagement_curve_point",
                legacy_observed_at,
            )?;
            builder
                .attribute_subtree(
                    FieldPath::new(format!("/trajectory/engagement_curve/series/{index}"))
                        .map_err(field_error)?,
                    attribution,
                )
                .map_err(provenance_error)?;
        }
    }

    if let Some(snapshot) = bundle.role_progression.as_ref() {
        let refs = snapshot
            .series
            .iter()
            .flat_map(|point| point.source_refs.iter())
            .collect::<Vec<_>>();
        let attribution = attribution_for_trajectory_refs(
            builder,
            &mut source_indexes,
            &subject,
            &refs,
            "temporal_role_progression",
            legacy_observed_at,
        )?;
        builder
            .attribute_subtree(
                FieldPath::new("/trajectory/role_progression").map_err(field_error)?,
                attribution,
            )
            .map_err(provenance_error)?;

        for (index, point) in snapshot.series.iter().enumerate() {
            let refs = point.source_refs.iter().collect::<Vec<_>>();
            let attribution = attribution_for_trajectory_refs(
                builder,
                &mut source_indexes,
                &subject,
                &refs,
                "temporal_role_progression_point",
                legacy_observed_at,
            )?;
            builder
                .attribute_subtree(
                    FieldPath::new(format!("/trajectory/role_progression/series/{index}"))
                        .map_err(field_error)?,
                    attribution,
                )
                .map_err(provenance_error)?;
        }
    }

    Ok(())
}

fn attribution_for_trajectory_refs(
    builder: &mut ProvenanceBuilder,
    source_indexes: &mut BTreeMap<String, SourceIndex>,
    subject: &SubjectAttribution,
    refs: &[&SourceRef],
    algorithm: &'static str,
    legacy_observed_at: chrono::DateTime<chrono::Utc>,
) -> Result<FieldAttribution, AbilityError> {
    let mut indexes = Vec::new();
    for source_ref in refs {
        let source_index = builder_source_index_for_trajectory_ref(
            builder,
            source_indexes,
            source_ref,
            legacy_observed_at,
        )?;
        if !indexes.contains(&source_index) {
            indexes.push(source_index);
        }
    }

    match indexes.as_slice() {
        [] => Ok(FieldAttribution::constant(subject.clone())),
        [source_index] => Ok(FieldAttribution::direct(subject.clone(), *source_index)),
        _ => FieldAttribution::computed(
            subject.clone(),
            algorithm,
            indexes
                .into_iter()
                .map(|source_index| SourceRef::Source { source_index })
                .collect(),
            Confidence::computed(1.0).map_err(provenance_error)?,
        )
        .map_err(provenance_error),
    }
}

fn builder_source_index_for_trajectory_ref(
    builder: &mut ProvenanceBuilder,
    source_indexes: &mut BTreeMap<String, SourceIndex>,
    source_ref: &SourceRef,
    legacy_observed_at: chrono::DateTime<chrono::Utc>,
) -> Result<SourceIndex, AbilityError> {
    let key = serde_json::to_string(source_ref)
        .map_err(|error| validation_error(format!("serialize trajectory source ref: {error}")))?;
    if let Some(index) = source_indexes.get(&key) {
        return Ok(*index);
    }

    let source = source_attribution_for_trajectory_ref(source_ref, legacy_observed_at)?;
    let source_index = builder.add_source(source);
    source_indexes.insert(key, source_index);
    Ok(source_index)
}

fn source_attribution_for_trajectory_ref(
    source_ref: &SourceRef,
    legacy_observed_at: chrono::DateTime<chrono::Utc>,
) -> Result<SourceAttribution, AbilityError> {
    match source_ref {
        SourceRef::Direct {
            data_source,
            identifier,
            observed_at,
            source_asof,
        } => SourceAttribution::new(
            data_source.clone(),
            vec![identifier.clone()],
            *observed_at,
            *source_asof,
            1.0,
            None,
        )
        .map_err(|error| validation_error(format!("invalid trajectory source: {error}"))),
        SourceRef::Source { .. } | SourceRef::Child { .. } => {
            SourceAttribution::legacy_unattributed(legacy_observed_at).map_err(|error| {
                validation_error(format!("invalid legacy trajectory source: {error}"))
            })
        }
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

fn envelope_subject(
    root: SubjectRef,
    entries: &[EntityContextEntry],
) -> Result<SubjectAttribution, AbilityError> {
    let mut subjects = vec![root];
    for entry in entries {
        let subject = subject_ref_for(&entry.entity_type, &entry.entity_id)?;
        if !subjects.iter().any(|existing| existing == &subject) {
            subjects.push(subject);
        }
    }

    let subject = if subjects.len() == 1 {
        subjects.into_iter().next().expect("one subject")
    } else {
        SubjectRef::Multi(subjects)
    };
    Ok(SubjectAttribution::direct_confident(subject))
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

fn filter_claims_for_actor(actor: Actor, claims: Vec<IntelligenceClaim>) -> Vec<IntelligenceClaim> {
    if actor == Actor::Agent {
        claims
            .into_iter()
            .filter(agent_can_read_claim)
            .collect::<Vec<_>>()
    } else {
        claims
    }
}

fn agent_can_read_claim(claim: &IntelligenceClaim) -> bool {
    claim_allowed_for_prompt_input(claim)
}

pub(crate) fn provenance_actor(actor: Actor) -> crate::abilities::provenance::Actor {
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
        // TODO: W1-B+ wiring — SurfaceClient provenance attribution lands in
        // W1-A0 / W1-B (audit-log helper + AbilityPolicy.required_scopes). The
        // stage-1a landing only ships the actor variant; no current invocation
        // path constructs Actor::SurfaceClient.
        Actor::SurfaceClient(_) => todo!("W1-B+ wiring for Actor::SurfaceClient"),
    }
}

fn entry_for_claim(
    claim: &IntelligenceClaim,
    render_actor: &RenderActor,
) -> Result<EntityContextEntry, AbilityError> {
    let (entity_type, entity_id) = claim_subject_identity(claim)?;
    let title = title_for_claim(claim);
    Ok(EntityContextEntry {
        id: claim.id.clone(),
        entity_type,
        entity_id,
        title: renderable_entity_context_text(claim, &title, render_actor)?,
        content: renderable_entity_context_text(claim, &claim.text, render_actor)?,
        created_at: claim.created_at.clone(),
        updated_at: claim
            .reactivated_at
            .clone()
            .unwrap_or_else(|| claim.created_at.clone()),
    })
}

fn render_actor_for_context(ctx: &AbilityContext<'_>) -> RenderActor {
    match &ctx.actor {
        Actor::User => RenderActor::user(ctx.services().actor, Some(ctx.services().actor)),
        Actor::Agent => RenderActor::agent("agent:get_entity_context"),
        Actor::Admin => RenderActor {
            actor: "admin".to_string(),
            user_id: None,
        },
        Actor::System => RenderActor {
            actor: "system".to_string(),
            user_id: None,
        },
        // TODO: W1-B+ wiring — SurfaceClient render actor mapping (per ADR-0108
        // sensitivity rules) lands with the SurfaceClientBridge plumbing.
        Actor::SurfaceClient(_) => todo!("W1-B+ wiring for Actor::SurfaceClient"),
    }
}

fn renderable_entity_context_text(
    claim: &IntelligenceClaim,
    value: &str,
    render_actor: &RenderActor,
) -> Result<EntityContextText, AbilityError> {
    renderable_claim_text_with_value(claim, value, RenderSurface::TauriEntityDetail, render_actor)
        .map(EntityContextText::Claim)
        .ok_or_else(|| {
            validation_error(format!(
                "claim `{}` cannot render for entity context",
                claim.id
            ))
        })
}

fn claim_subject_identity(claim: &IntelligenceClaim) -> Result<(String, String), AbilityError> {
    let value: serde_json::Value = serde_json::from_str(&claim.subject_ref)
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
                "claim `{}` has unsupported entity context subject",
                claim.id
            )))
        }
    }
}

fn title_for_claim(claim: &IntelligenceClaim) -> String {
    match claim
        .field_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(field_path) => format!("{}: {field_path}", claim.claim_type),
        None => claim.claim_type.clone(),
    }
}

fn source_for_claim(
    ctx: &AbilityContext<'_>,
    claim: &IntelligenceClaim,
    entry: &EntityContextEntry,
) -> Result<SourceAttribution, AbilityError> {
    let now = ctx.services().clock.now();
    let (observed_at, source_asof) = parsed_claim_timestamp(claim, now);
    SourceAttribution::new(
        data_source_for_claim(&claim.data_source),
        vec![source_identifier_for_claim(claim, entry)],
        observed_at,
        source_asof,
        1.0,
        None,
    )
    .map_err(|error| validation_error(format!("invalid source attribution: {error}")))
}

fn data_source_for_claim(value: &str) -> DataSource {
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

fn source_identifier_for_claim(
    claim: &IntelligenceClaim,
    entry: &EntityContextEntry,
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
        "user" | "human" => SourceIdentifier::UserEntry {
            entry_id: ContextEntryId::new(source_ref.unwrap_or(&claim.id).to_string()),
        },
        _ => SourceIdentifier::Entity {
            entity_id: EntityId::new(entry.entity_id.clone()),
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

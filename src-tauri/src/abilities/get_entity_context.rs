use dailyos_abilities_macro::ability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::provenance::source_time::{parse_source_timestamp, SourceTimestampStatus};
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, ChunkId, ContextEntryId, DataSource, DocumentId,
    EntityId, FieldAttribution, FieldPath, GleanDownstream, MeetingId, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SourceAttribution, SourceIdentifier, SourceName,
    SubjectAttribution, SubjectRef,
};
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};
use crate::db::claim_invalidation::SubjectRef as ClaimSubjectRef;
use crate::db::claims::IntelligenceClaim;
use crate::types::EntityContextEntry;

const ABILITY_NAME: &str = "get_entity_context";
const ABILITY_SCHEMA_VERSION: u32 = 1;

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

#[ability(
    name = "get_entity_context",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
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
) -> AbilityResult<Vec<EntityContextEntry>> {
    validate_schema_version(input.schema_version)?;
    let subject_ref = subject_ref_for(&input.entity_type, &input.entity_id)?;
    let subject = SubjectAttribution::direct_confident(subject_ref.clone());
    let claims = ctx
        .services()
        .read_entity_context_claims(
            input.entity_type.clone(),
            input.entity_id.clone(),
            input.depth.levels(),
        )
        .await
        .map_err(|error| hard_error("entity context claim read failed", error))?;
    let claims = filter_claims_for_actor(ctx.actor, claims);
    let entries = claims
        .iter()
        .map(entry_for_claim)
        .collect::<Result<Vec<_>, _>>()?;

    let mut builder = ProvenanceBuilder::new(provenance_config(ctx, input.schema_version));
    builder.set_subject(envelope_subject(subject_ref, &entries)?);

    if claims.is_empty() {
        builder
            .attribute(
                FieldPath::root(),
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
        builder
            .attribute_subtree(
                FieldPath::new(format!("/{index}")).map_err(field_error)?,
                FieldAttribution::direct(entry_subject, source_index),
            )
            .map_err(provenance_error)?;
    }

    builder.finalize(entries).map_err(provenance_error)
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
    fn levels(&self) -> usize {
        match self {
            Self::Shallow => 1,
            Self::Standard => 2,
            Self::Deep => 3,
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
    config.actor = provenance_actor(ctx.actor);
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
    crate::services::claims::claim_allowed_for_prompt_input(claim)
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
    }
}

fn entry_for_claim(claim: &IntelligenceClaim) -> Result<EntityContextEntry, AbilityError> {
    let (entity_type, entity_id) = claim_subject_identity(claim)?;
    Ok(EntityContextEntry {
        id: claim.id.clone(),
        entity_type,
        entity_id,
        title: title_for_claim(claim),
        content: claim.text.clone(),
        created_at: claim.created_at.clone(),
        updated_at: claim
            .reactivated_at
            .clone()
            .unwrap_or_else(|| claim.created_at.clone()),
    })
}

fn claim_subject_identity(claim: &IntelligenceClaim) -> Result<(String, String), AbilityError> {
    let value: serde_json::Value = serde_json::from_str(&claim.subject_ref)
        .map_err(|error| validation_error(format!("invalid claim subject_ref JSON: {error}")))?;
    match crate::services::claims::subject_ref_from_json(&value)
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

#[cfg(test)]
#[path = "get_entity_context/tests/mod.rs"]
mod tests;

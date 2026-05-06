use dailyos_abilities_macro::ability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::provenance::source_time::{parse_source_timestamp, SourceTimestampStatus};
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, ContextEntryId, DataSource, FieldAttribution, FieldPath,
    ProvenanceBuilder, ProvenanceBuilderConfig, SchemaVersion, SourceAttribution, SourceIdentifier,
    SubjectAttribution, SubjectRef,
};
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult,
};
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
    allowed_actors = [User, System],
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
    let subject = SubjectAttribution::direct_confident(subject_ref);
    let mut entries = ctx
        .services()
        .read_entity_context_entries(input.entity_type.clone(), input.entity_id.clone())
        .await
        .map_err(|error| hard_error("entity context read failed", error))?;
    entries.sort_by(|left, right| right.created_at.cmp(&left.created_at));

    for entry in &entries {
        if entry.entity_type != input.entity_type || entry.entity_id != input.entity_id {
            return Err(validation_error(format!(
                "entity context entry `{}` does not belong to requested subject",
                entry.id
            )));
        }
    }

    let mut builder = ProvenanceBuilder::new(provenance_config(ctx, input.schema_version));
    builder.set_subject(subject.clone());

    if entries.is_empty() {
        builder
            .attribute(
                FieldPath::root(),
                FieldAttribution::constant(subject.clone()),
            )
            .map_err(provenance_error)?;
    }

    for (index, entry) in entries.iter().enumerate() {
        let source_index = builder.add_source(source_for_entry(ctx, entry)?);
        builder
            .attribute_subtree(
                FieldPath::new(format!("/{index}")).map_err(field_error)?,
                FieldAttribution::direct(subject.clone(), source_index),
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

fn provenance_config(ctx: &AbilityContext<'_>, schema_version: u32) -> ProvenanceBuilderConfig {
    let mut config = ProvenanceBuilderConfig::new(ABILITY_NAME, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(schema_version);
    config.actor = provenance_actor(ctx.actor);
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Read;
    config
}

fn provenance_actor(actor: crate::abilities::Actor) -> crate::abilities::provenance::Actor {
    match actor {
        crate::abilities::Actor::User => crate::abilities::provenance::Actor::User,
        crate::abilities::Actor::Agent => crate::abilities::provenance::Actor::Agent {
            name: "agent".to_string(),
            version: "unknown".to_string(),
        },
        crate::abilities::Actor::Admin => crate::abilities::provenance::Actor::Human {
            role: "admin".to_string(),
            id: "admin".to_string(),
        },
        crate::abilities::Actor::System => crate::abilities::provenance::Actor::System {
            component: "dailyos".to_string(),
        },
    }
}

fn source_for_entry(
    ctx: &AbilityContext<'_>,
    entry: &EntityContextEntry,
) -> Result<SourceAttribution, AbilityError> {
    let now = ctx.services().clock.now();
    let (observed_at, source_asof) = parsed_entry_timestamp(entry, now);
    SourceAttribution::new(
        DataSource::User,
        vec![SourceIdentifier::UserEntry {
            entry_id: ContextEntryId::new(entry.id.clone()),
        }],
        observed_at,
        source_asof,
        1.0,
        None,
    )
    .map_err(|error| validation_error(format!("invalid source attribution: {error}")))
}

fn parsed_entry_timestamp(
    entry: &EntityContextEntry,
    now: chrono::DateTime<chrono::Utc>,
) -> (
    chrono::DateTime<chrono::Utc>,
    Option<chrono::DateTime<chrono::Utc>>,
) {
    for candidate in [entry.updated_at.as_str(), entry.created_at.as_str()] {
        match parse_source_timestamp(Some(candidate), now, None) {
            SourceTimestampStatus::Accepted(parsed)
            | SourceTimestampStatus::Implausible { parsed, .. } => {
                return (parsed, Some(parsed));
            }
            SourceTimestampStatus::Malformed(_) | SourceTimestampStatus::Missing => {}
        }
    }

    (now, None)
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

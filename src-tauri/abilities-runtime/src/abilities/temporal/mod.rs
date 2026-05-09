use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use dailyos_abilities_macro::ability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, EntityId, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SourceRef, SubjectAttribution, SubjectRef,
};
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult,
};

const SCHEMA_VERSION: u32 = 2;
const REFRESH_ABILITY_NAME: &str = "refresh_engagement_curve";
const ROLE_ABILITY_NAME: &str = "detect_role_change";
pub const RETENTION_DAYS: u16 = 730;
pub const DEEP_LIMIT_WEEKS: u16 = 52;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrajectoryKind {
    EngagementCurve,
    RoleProgression,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct DataPoint<T> {
    #[schemars(with = "String")]
    pub at: DateTime<Utc>,
    pub value: T,
    pub source_refs: Vec<SourceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TrajectorySnapshot<T> {
    pub kind: TrajectoryKind,
    pub entity_type: String,
    pub entity_id: EntityId,
    pub series: Vec<DataPoint<T>>,
    #[schemars(with = "String")]
    pub computed_at: DateTime<Utc>,
    pub confidence: f32,
}

impl<T> TrajectorySnapshot<T> {
    pub fn new(
        kind: TrajectoryKind,
        entity_type: String,
        entity_id: EntityId,
        series: Vec<DataPoint<T>>,
        computed_at: DateTime<Utc>,
        confidence: f32,
    ) -> Result<Self, TrajectoryError> {
        if !(0.0..=1.0).contains(&confidence) || !confidence.is_finite() {
            return Err(TrajectoryError::InvalidConfidence);
        }

        Ok(Self {
            kind,
            entity_type,
            entity_id,
            series,
            computed_at,
            confidence,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct EngagementWindow {
    pub meetings_count: u32,
    pub emails_count: u32,
    pub bidirectional_ratio: f32,
}

impl EngagementWindow {
    pub fn new(
        meetings_count: u32,
        emails_count: u32,
        bidirectional_ratio: f32,
    ) -> Result<Self, TrajectoryError> {
        if !(0.0..=1.0).contains(&bidirectional_ratio) || !bidirectional_ratio.is_finite() {
            return Err(TrajectoryError::InvalidRatio);
        }

        Ok(Self {
            meetings_count,
            emails_count,
            bidirectional_ratio,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RoleEntry {
    #[schemars(with = "String")]
    pub started_at: DateTime<Utc>,
    #[schemars(with = "Option<String>")]
    pub ended_at: Option<DateTime<Utc>>,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seniority: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TrajectoryBundle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engagement_curve: Option<TrajectorySnapshot<EngagementWindow>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_progression: Option<TrajectorySnapshot<RoleEntry>>,
}

impl TrajectoryBundle {
    pub fn is_empty(&self) -> bool {
        self.engagement_curve.is_none() && self.role_progression.is_none()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrajectoryQueryDepth {
    None,
    Latest,
    Weeks(u16),
}

impl TrajectoryQueryDepth {
    pub fn limit(self) -> usize {
        match self {
            Self::None => 0,
            Self::Latest => 1,
            Self::Weeks(weeks) => usize::from(weeks.min(DEEP_LIMIT_WEEKS)),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct RefreshEngagementCurveInput {
    pub schema_version: u32,
    pub entity_type: String,
    pub entity_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RefreshEngagementCurveResult {
    pub entity_type: String,
    pub entity_id: String,
    #[schemars(with = "String")]
    pub week_start: DateTime<Utc>,
    pub rows_written: u32,
    pub retained_weeks: u16,
    #[schemars(with = "String")]
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct DetectRoleChangeInput {
    pub schema_version: u32,
    pub entity_type: String,
    pub entity_id: String,
    #[schemars(with = "Option<String>")]
    pub observed_at: Option<DateTime<Utc>>,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seniority: Option<String>,
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DetectRoleChangeResult {
    pub entity_type: String,
    pub entity_id: String,
    pub appended: bool,
    #[schemars(with = "String")]
    pub current_started_at: DateTime<Utc>,
    #[schemars(with = "Option<String>")]
    pub prior_ended_at: Option<DateTime<Utc>>,
    #[schemars(with = "String")]
    pub computed_at: DateTime<Utc>,
}

pub type TrajectoryReadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<TrajectoryBundle, String>> + Send + 'a>>;

pub trait TrajectoryReadHandle: Send + Sync {
    fn read_trajectory_bundle<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        depth: TrajectoryQueryDepth,
        computed_at: DateTime<Utc>,
    ) -> TrajectoryReadFuture<'a>;
}

pub type TemporalMaintenanceFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

pub trait TemporalMaintenanceHandle: Send + Sync {
    fn refresh_engagement_curve<'a>(
        &'a self,
        input: RefreshEngagementCurveInput,
        computed_at: DateTime<Utc>,
    ) -> TemporalMaintenanceFuture<'a, RefreshEngagementCurveResult>;

    fn detect_role_change<'a>(
        &'a self,
        input: DetectRoleChangeInput,
        computed_at: DateTime<Utc>,
    ) -> TemporalMaintenanceFuture<'a, DetectRoleChangeResult>;
}

#[ability(
    name = "refresh_engagement_curve",
    category = Maintenance,
    version = "1.0.0",
    schema_version = 2,
    allowed_actors = [System],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = true }
)]
pub async fn refresh_engagement_curve(
    ctx: &AbilityContext<'_>,
    input: RefreshEngagementCurveInput,
) -> AbilityResult<RefreshEngagementCurveResult> {
    validate_schema_version(input.schema_version, REFRESH_ABILITY_NAME)?;
    validate_entity_type(&input.entity_type)?;
    validate_entity_id(&input.entity_id)?;
    ctx.services()
        .check_mutation_allowed()
        .map_err(service_error)?;

    let computed_at = ctx.services().clock.now();
    let output = ctx
        .services()
        .refresh_engagement_curve(input, computed_at)
        .await
        .map_err(|error| hard_error("refresh_engagement_curve_failed", error))?;

    finalize_output(ctx, REFRESH_ABILITY_NAME, output)
}

#[ability(
    name = "detect_role_change",
    category = Maintenance,
    version = "1.0.0",
    schema_version = 2,
    allowed_actors = [System],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = true }
)]
pub async fn detect_role_change(
    ctx: &AbilityContext<'_>,
    input: DetectRoleChangeInput,
) -> AbilityResult<DetectRoleChangeResult> {
    validate_schema_version(input.schema_version, ROLE_ABILITY_NAME)?;
    validate_entity_type(&input.entity_type)?;
    validate_entity_id(&input.entity_id)?;
    if input.title.trim().is_empty() {
        return Err(validation_error("title must be non-empty"));
    }
    ctx.services()
        .check_mutation_allowed()
        .map_err(service_error)?;

    let computed_at = ctx.services().clock.now();
    let output = ctx
        .services()
        .detect_role_change(input, computed_at)
        .await
        .map_err(|error| hard_error("detect_role_change_failed", error))?;

    finalize_output(ctx, ROLE_ABILITY_NAME, output)
}

fn validate_schema_version(schema_version: u32, ability_name: &str) -> Result<(), AbilityError> {
    if schema_version == SCHEMA_VERSION {
        Ok(())
    } else {
        Err(validation_error(format!(
            "unsupported schema_version `{schema_version}` for `{ability_name}`"
        )))
    }
}

fn validate_entity_id(entity_id: &str) -> Result<(), AbilityError> {
    if entity_id.trim().is_empty() {
        Err(validation_error("entity_id must be non-empty"))
    } else {
        Ok(())
    }
}

fn validate_entity_type(entity_type: &str) -> Result<(), AbilityError> {
    match entity_type.trim() {
        "account" | "project" | "person" => Ok(()),
        "" => Err(validation_error("entity_type must be non-empty")),
        other => Err(validation_error(format!(
            "unsupported entity_type `{other}` for temporal maintenance"
        ))),
    }
}

fn finalize_output<T: Serialize>(
    ctx: &AbilityContext<'_>,
    ability_name: &'static str,
    output: T,
) -> AbilityResult<T> {
    let mut builder = ProvenanceBuilder::new(provenance_config(ctx, ability_name));
    builder.set_subject(SubjectAttribution::direct_confident(SubjectRef::Unknown));
    builder
        .attribute(
            FieldPath::root(),
            FieldAttribution::constant(SubjectAttribution::direct_confident(SubjectRef::Unknown)),
        )
        .map_err(provenance_error)?;
    builder.finalize(output).map_err(provenance_error)
}

fn provenance_config(
    ctx: &AbilityContext<'_>,
    ability_name: &'static str,
) -> ProvenanceBuilderConfig {
    let mut config = ProvenanceBuilderConfig::new(ability_name, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(SCHEMA_VERSION);
    config.actor = crate::abilities::get_entity_context::provenance_actor(ctx.actor);
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Maintenance;
    config
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

fn service_error(error: impl std::fmt::Display) -> AbilityError {
    hard_error("temporal_mutation_blocked", error.to_string())
}

fn provenance_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("provenance construction failed: {error}"))
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TrajectoryError {
    #[error("confidence must be finite and within [0.0, 1.0]")]
    InvalidConfidence,
    #[error("ratio must be finite and within [0.0, 1.0]")]
    InvalidRatio,
}

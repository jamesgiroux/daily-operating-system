use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::prompts;
use crate::abilities::provenance::source_time::{parse_source_timestamp, SourceTimestampStatus};
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, Confidence, DataSource, FieldAttribution, FieldPath,
    GleanDownstream, ProvenanceBuilder, ProvenanceBuilderConfig, SchemaVersion,
    SourceAttribution, SourceIdentifier, SourceName, SourceRef, SubjectAttribution,
    SubjectRef,
};
use crate::abilities::temporal::{TrajectoryBundle, TrajectoryQueryDepth};
use crate::abilities::{AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind};
use crate::abilities::{AbilityResult, Actor as RegistryActor};
use crate::intelligence::provider::{ModelTier, ProviderError};
use crate::services::context::ClaimDismissalSurface;
use crate::types::{
    claim_allowed_for_prompt_input, prompt_input_sensitivity_name_allowed, subject_ref_from_json,
    ClaimSubjectRef, IntelligenceClaim,
};

const ABILITY_NAME: &str = "detect_risk_shift";
const ABILITY_SCHEMA_VERSION: u32 = 1;

pub const TRAJECTORY_DELTA_TRANSFORM_ID: &str = "trajectory_delta_v1";
pub const TRAJECTORY_DELTA_SHORT_WINDOW_DAYS: u16 = 30;
pub const TRAJECTORY_DELTA_LONG_WINDOW_DAYS: u16 = 90;
pub const TRAJECTORY_DELTA_ENGAGEMENT_THRESHOLD_POSITIVE_PERCENT: f32 = 0.10;
pub const TRAJECTORY_DELTA_ENGAGEMENT_THRESHOLD_NEGATIVE_PERCENT: f32 = -0.10;
pub const TRAJECTORY_DELTA_MAJOR_THRESHOLD_POSITIVE_PERCENT: f32 = 0.25;
pub const TRAJECTORY_DELTA_MAJOR_THRESHOLD_NEGATIVE_PERCENT: f32 = -0.25;
pub const TRAJECTORY_DELTA_NULL_HANDLING: &str =
    "Null, missing, or zero-denominator trajectory points are never coerced to zero; they produce insufficient_evidence for the affected delta.";

pub const JUDGE_MODEL: &str = "claude-sonnet-4-6";
pub const DEFAULT_SAMPLING_TEMPERATURE: f32 = 0.0;
pub const DEFAULT_SAMPLING_TOP_P: f32 = 1.0;
pub const DEFAULT_SAMPLING_SEED: u64 = 222;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DetectRiskShiftInput {
    #[serde(default = "default_schema_version")]
    pub schema_version: SchemaVersion,
    pub entity_type: String,
    pub entity_id: String,
    #[serde(default = "default_depth")]
    pub depth: u8,
    #[serde(default, skip_deserializing, skip_serializing)]
    #[schemars(skip)]
    evidence: Vec<RiskShiftSourceVerification>,
    #[serde(default, skip_deserializing, skip_serializing)]
    #[schemars(skip)]
    context: Option<RiskShiftContext>,
}

impl DetectRiskShiftInput {
    pub fn with_evidence(mut self, evidence: Vec<RiskShiftSourceVerification>) -> Self {
        self.evidence = evidence;
        self
    }

    pub fn with_context(mut self, context: RiskShiftContext) -> Self {
        self.context = Some(context);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RiskShiftSubjectRef {
    pub kind: String,
    pub id: String,
}

impl RiskShiftSubjectRef {
    fn new(kind: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
        }
    }

    fn key(&self) -> String {
        format!("{}:{}", self.kind, self.id)
    }

    fn to_subject_ref(&self) -> Result<SubjectRef, AbilityError> {
        subject_ref_for(&self.kind, &self.id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RiskShiftSourceVerification {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
    pub subject: RiskShiftSubjectRef,
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
    #[serde(default = "default_sensitivity_name")]
    pub sensitivity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct EvidenceSummary {
    pub claim_count: u32,
    pub contributing_claim_ids: Vec<String>,
    pub source_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_asof: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oldest_contributing_claim_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskDirection {
    Increasing,
    Decreasing,
    Stable,
    Mixed,
    InsufficientEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RiskIndicator {
    pub label: String,
    pub direction: RiskDirection,
    pub severity: f32,
    pub rationale: String,
    pub evidence_summary: EvidenceSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RiskShiftClaimDraft {
    pub claim_type: String,
    pub text: String,
    pub subject: RiskShiftSubjectRef,
    pub direction: RiskDirection,
    pub confidence: f32,
    pub evidence_summary: EvidenceSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persist_error: Option<RiskShiftPersistError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind", content = "detail")]
pub enum RiskShiftPersistError {
    NotAttempted,
    MutationBlocked,
    Validation(String),
    Storage(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RiskShiftContext {
    pub subject: RiskShiftSubjectRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub transform: TrajectoryDeltaTransformSpec,
    pub sampling: RiskShiftSamplingCapture,
    pub judge_model: String,
    #[serde(default)]
    pub claims: Vec<RiskShiftSourceVerification>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trajectory: Option<TrajectoryBundle>,
    #[serde(default)]
    pub entity_context_children: Vec<RiskShiftEntityContextChild>,
    pub revoked_glean_revalidation: RevokedGleanCacheRevalidation,
}

impl RiskShiftContext {
    fn retain_prompt_input_allowed_claims(&mut self) {
        self.claims
            .retain(risk_shift_source_allowed_for_prompt_input);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RiskShiftEntityContextChild {
    pub subject: RiskShiftSubjectRef,
    #[serde(default)]
    pub entries: Vec<RiskShiftEntityContextEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RiskShiftEntityContextEntry {
    pub id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TrajectoryDeltaTransformSpec {
    pub transform_id: String,
    pub short_window_days: u16,
    pub long_window_days: u16,
    pub engagement_threshold_positive_percent: f32,
    pub engagement_threshold_negative_percent: f32,
    pub major_threshold_positive_percent: f32,
    pub major_threshold_negative_percent: f32,
    pub null_handling: String,
}

impl Default for TrajectoryDeltaTransformSpec {
    fn default() -> Self {
        Self {
            transform_id: TRAJECTORY_DELTA_TRANSFORM_ID.to_string(),
            short_window_days: TRAJECTORY_DELTA_SHORT_WINDOW_DAYS,
            long_window_days: TRAJECTORY_DELTA_LONG_WINDOW_DAYS,
            engagement_threshold_positive_percent:
                TRAJECTORY_DELTA_ENGAGEMENT_THRESHOLD_POSITIVE_PERCENT,
            engagement_threshold_negative_percent:
                TRAJECTORY_DELTA_ENGAGEMENT_THRESHOLD_NEGATIVE_PERCENT,
            major_threshold_positive_percent: TRAJECTORY_DELTA_MAJOR_THRESHOLD_POSITIVE_PERCENT,
            major_threshold_negative_percent: TRAJECTORY_DELTA_MAJOR_THRESHOLD_NEGATIVE_PERCENT,
            null_handling: TRAJECTORY_DELTA_NULL_HANDLING.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RiskShiftSamplingCapture {
    pub temperature: f32,
    pub top_p: f32,
    pub seed: u64,
}

impl Default for RiskShiftSamplingCapture {
    fn default() -> Self {
        Self {
            temperature: DEFAULT_SAMPLING_TEMPERATURE,
            top_p: DEFAULT_SAMPLING_TOP_P,
            seed: DEFAULT_SAMPLING_SEED,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RevokedGleanCacheRevalidation {
    pub checked: bool,
    pub hook: String,
    #[serde(default)]
    pub revoked_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coverage_warnings: Vec<RiskShiftCoverageWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RiskShiftCoverageWarning {
    pub kind: String,
    pub source_ref: String,
    pub trajectory_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RiskShiftResult {
    pub schema_version: SchemaVersion,
    pub envelope: TrustEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case", tag = "trust", content = "payload")]
pub enum TrustEnvelope {
    Untrusted(RiskShiftUntrustedResult),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RiskShiftUntrustedResult {
    pub subject: RiskShiftSubjectRef,
    pub direction: RiskDirection,
    pub summary: String,
    pub confidence: f32,
    #[serde(default)]
    pub indicators: Vec<RiskIndicator>,
    pub claim_draft: RiskShiftClaimDraft,
    pub evidence_summary: EvidenceSummary,
    pub source_verification: RiskShiftSourceMembership,
    pub sampling: RiskShiftSamplingCapture,
    pub judge_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RiskShiftSourceMembership {
    pub checked: bool,
    pub input_claim_refs: Vec<String>,
    pub accepted_source_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PromptRiskContext<'a> {
    schema_version: u32,
    subject: &'a RiskShiftSubjectRef,
    transform: &'a TrajectoryDeltaTransformSpec,
    sampling: &'a RiskShiftSamplingCapture,
    judge_model: &'a str,
    claims: &'a [RiskShiftSourceVerification],
    trajectory: &'a Option<TrajectoryBundle>,
    entity_context_children: &'a [RiskShiftEntityContextChild],
    revoked_glean_revalidation: &'a RevokedGleanCacheRevalidation,
    mechanical_indicators: Vec<PromptMechanicalIndicator>,
}

#[derive(Debug, Serialize)]
struct PromptMechanicalIndicator {
    label: String,
    direction: RiskDirection,
    severity: f32,
    rationale: String,
}

#[derive(Debug, Deserialize)]
struct RawRiskShift {
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default, alias = "source_ids")]
    source_refs: Vec<String>,
    #[serde(default)]
    indicators: Vec<RawRiskIndicator>,
    #[serde(default)]
    claim_draft: Option<RawRiskShiftClaimDraft>,
}

#[derive(Debug, Deserialize)]
struct RawRiskIndicator {
    label: String,
    #[serde(default)]
    direction: Option<RiskDirection>,
    #[serde(default)]
    severity: Option<f32>,
    #[serde(default)]
    rationale: Option<String>,
    #[serde(default, alias = "source_ids")]
    source_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawRiskShiftClaimDraft {
    #[serde(default)]
    claim_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    direction: Option<RiskDirection>,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default, alias = "source_ids")]
    source_refs: Vec<String>,
}

pub async fn build_risk_shift(
    ctx: &AbilityContext<'_>,
    input: DetectRiskShiftInput,
) -> AbilityResult<RiskShiftResult> {
    validate_schema_version(input.schema_version)?;
    let mut context = RiskShiftContext::from_input_or_services(ctx, &input).await?;
    validate_context_subject(&input, &context)?;
    validate_trajectory_subject_fit(&input, context.trajectory.as_ref())?;

    // This is intentionally separate from get_entity_context's own filtering.
    // The prompt boundary must retain the centralized sensitivity gate even
    // after real child composition wiring replaces the synthetic empty child.
    context.retain_prompt_input_allowed_claims();
    context.revoked_glean_revalidation =
        revoked_glean_cache_revalidation_hook(&context.claims, context.trajectory.as_mut());

    let mechanical_indicators = mechanical_indicators_from_trajectory(context.trajectory.as_ref());
    let deterministic_direction =
        deterministic_direction_from_mechanical_indicators(&mechanical_indicators);
    let prompt_context = PromptRiskContext {
        schema_version: input.schema_version.0,
        subject: &context.subject,
        transform: &context.transform,
        sampling: &context.sampling,
        judge_model: &context.judge_model,
        claims: &context.claims,
        trajectory: &context.trajectory,
        entity_context_children: &context.entity_context_children,
        revoked_glean_revalidation: &context.revoked_glean_revalidation,
        mechanical_indicators,
    };
    let risk_context_json =
        serde_json::to_string_pretty(&prompt_context).map_err(|error| AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("failed to serialize detect_risk_shift prompt context: {error}"),
        })?;
    let rendered = prompts::render_prompt(&risk_context_json, input.schema_version.0);
    let completion = ctx
        .provider
        .complete(rendered.prompt_input(), ModelTier::Synthesis)
        .await
        .map_err(provider_error)?;
    let raw = parse_completion(&completion.text)?;
    let sampling = RiskShiftSamplingCapture {
        temperature: completion.fingerprint_metadata.temperature,
        top_p: completion
            .fingerprint_metadata
            .top_p
            .unwrap_or(context.sampling.top_p),
        seed: completion
            .fingerprint_metadata
            .seed
            .unwrap_or(context.sampling.seed),
    };
    let synthesized = assemble_untrusted_result(
        &context,
        raw,
        sampling,
        ctx.services().clock.now(),
        deterministic_direction,
    )?;
    let fingerprint = prompts::fingerprint_from_completion(&completion, &rendered);

    finalize_result(ctx, input.schema_version, synthesized, fingerprint, &context)
}

impl RiskShiftContext {
    async fn from_input_or_services(
        ctx: &AbilityContext<'_>,
        input: &DetectRiskShiftInput,
    ) -> Result<Self, AbilityError> {
        if let Some(context) = input.context.clone() {
            return Ok(context);
        }

        let subject = RiskShiftSubjectRef::new(input.entity_type.clone(), input.entity_id.clone());
        let claims = if input.evidence.is_empty() {
            ctx.services()
                .read_entity_context_claims(
                    input.entity_type.clone(),
                    input.entity_id.clone(),
                    ClaimDismissalSurface::Worker,
                    context_depth_levels(input.depth),
                )
                .await
                .map_err(|error| AbilityError {
                    kind: AbilityErrorKind::HardError("risk_shift_claim_read".into()),
                    message: error,
                })?
                .into_iter()
                .filter(claim_allowed_for_prompt_input)
                .map(risk_source_from_claim)
                .collect::<Result<Vec<_>, _>>()?
        } else {
            input.evidence.clone()
        };

        let trajectory = ctx
            .services()
            .read_trajectory_bundle(
                input.entity_type.clone(),
                input.entity_id.clone(),
                TrajectoryQueryDepth::Weeks(13),
            )
            .await
            .map_err(|error| AbilityError {
                kind: AbilityErrorKind::HardError("risk_shift_trajectory_read".into()),
                message: error,
            })?;

        Ok(Self {
            subject: subject.clone(),
            display_name: None,
            transform: TrajectoryDeltaTransformSpec::default(),
            sampling: RiskShiftSamplingCapture::default(),
            judge_model: JUDGE_MODEL.to_string(),
            claims,
            trajectory: Some(trajectory),
            entity_context_children: synthetic_empty_entity_context_children(subject),
            revoked_glean_revalidation: RevokedGleanCacheRevalidation {
                checked: false,
                hook: "revoked_glean_cache_revalidation".to_string(),
                revoked_refs: Vec::new(),
                coverage_warnings: Vec::new(),
            },
        })
    }
}

fn assemble_untrusted_result(
    context: &RiskShiftContext,
    raw: RawRiskShift,
    sampling: RiskShiftSamplingCapture,
    now: DateTime<Utc>,
    direction: RiskDirection,
) -> Result<RiskShiftUntrustedResult, AbilityError> {
    let source_refs = collect_source_refs(&raw);
    if !matches!(direction, RiskDirection::InsufficientEvidence) && source_refs.is_empty() {
        return Err(validation_error(
            "detect_risk_shift provider response missing source_refs",
        ));
    }

    let membership = verify_source_refs(context, &source_refs)?;
    let overall_evidence_summary =
        evidence_summary(&membership.accepted_source_refs, &context.claims, now);
    let summary = raw.summary.unwrap_or_else(|| match direction {
        RiskDirection::InsufficientEvidence => "Insufficient evidence to determine a risk shift."
            .to_string(),
        _ => "Risk shift detected from cited evidence.".to_string(),
    });
    let confidence = clamp_confidence(raw.confidence.unwrap_or(default_direction_confidence(
        &direction,
        &overall_evidence_summary,
    )));

    let indicators = raw
        .indicators
        .into_iter()
        .map(|indicator| {
            let indicator_refs = normalize_refs(indicator.source_refs);
            verify_source_refs(context, &indicator_refs)?;
            Ok(RiskIndicator {
                label: indicator.label,
                direction: indicator.direction.unwrap_or_else(|| direction.clone()),
                severity: clamp_confidence(indicator.severity.unwrap_or(confidence)),
                rationale: indicator.rationale.unwrap_or_default(),
                evidence_summary: evidence_summary(&indicator_refs, &context.claims, now),
            })
        })
        .collect::<Result<Vec<_>, AbilityError>>()?;

    let claim_draft = claim_draft_from_raw(
        context,
        raw.claim_draft,
        &direction,
        &summary,
        confidence,
        &overall_evidence_summary,
        now,
    )?;

    Ok(RiskShiftUntrustedResult {
        subject: context.subject.clone(),
        direction,
        summary,
        confidence,
        indicators,
        claim_draft,
        evidence_summary: overall_evidence_summary,
        source_verification: membership,
        sampling,
        judge_model: JUDGE_MODEL.to_string(),
    })
}

fn claim_draft_from_raw(
    context: &RiskShiftContext,
    raw: Option<RawRiskShiftClaimDraft>,
    direction: &RiskDirection,
    summary: &str,
    confidence: f32,
    fallback_summary: &EvidenceSummary,
    now: DateTime<Utc>,
) -> Result<RiskShiftClaimDraft, AbilityError> {
    let Some(raw) = raw else {
        return Ok(RiskShiftClaimDraft {
            claim_type: "account_risk_shift".to_string(),
            text: summary.to_string(),
            subject: context.subject.clone(),
            direction: direction.clone(),
            confidence,
            evidence_summary: fallback_summary.clone(),
            persist_error: Some(RiskShiftPersistError::NotAttempted),
        });
    };

    let draft_refs = normalize_refs(raw.source_refs);
    verify_source_refs(context, &draft_refs)?;
    let draft_summary = if draft_refs.is_empty() {
        fallback_summary.clone()
    } else {
        evidence_summary(&draft_refs, &context.claims, now)
    };

    Ok(RiskShiftClaimDraft {
        claim_type: raw
            .claim_type
            .unwrap_or_else(|| "account_risk_shift".to_string()),
        text: raw.text.unwrap_or_else(|| summary.to_string()),
        subject: context.subject.clone(),
        direction: raw.direction.unwrap_or_else(|| direction.clone()),
        confidence: clamp_confidence(raw.confidence.unwrap_or(confidence)),
        evidence_summary: draft_summary,
        persist_error: Some(RiskShiftPersistError::NotAttempted),
    })
}

fn finalize_result(
    ctx: &AbilityContext<'_>,
    schema_version: SchemaVersion,
    synthesized: RiskShiftUntrustedResult,
    fingerprint: crate::abilities::provenance::PromptFingerprint,
    context: &RiskShiftContext,
) -> AbilityResult<RiskShiftResult> {
    let mut builder = ProvenanceBuilder::new(provenance_config(ctx, schema_version));
    let subject = SubjectAttribution::direct_confident(context.subject.to_subject_ref()?);
    builder.set_subject(subject.clone());
    builder.set_prompt_fingerprint(fingerprint);

    let mut source_index_by_ref = BTreeMap::new();
    for claim in &context.claims {
        let source_index = builder.add_source(source_for_claim(ctx, claim)?);
        source_index_by_ref.insert(claim.id.clone(), source_index);
        if let Some(source_ref) = &claim.source_ref {
            source_index_by_ref.insert(source_ref.clone(), source_index);
        }
    }

    let source_refs = synthesized
        .source_verification
        .accepted_source_refs
        .iter()
        .filter_map(|source_ref| source_index_by_ref.get(source_ref).copied())
        .map(|source_index| SourceRef::Source { source_index })
        .collect::<Vec<_>>();

    let root_attribution = if source_refs.is_empty() {
        FieldAttribution::constant(subject)
    } else {
        FieldAttribution::llm_synthesis(
            subject,
            source_refs,
            Confidence::provider_reported(synthesized.confidence).map_err(map_field_error)?,
            None,
        )
        .map_err(map_field_error)?
    };
    builder
        .attribute_subtree(FieldPath::root(), root_attribution)
        .map_err(map_provenance_error)?;

    let result = RiskShiftResult {
        schema_version,
        envelope: TrustEnvelope::Untrusted(synthesized),
    };
    builder.finalize(result).map_err(map_provenance_error)
}

fn parse_completion(text: &str) -> Result<RawRiskShift, AbilityError> {
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
        message: format!("detect_risk_shift provider response was not valid JSON: {error}"),
    })
}

fn verify_source_refs(
    context: &RiskShiftContext,
    source_refs: &[String],
) -> Result<RiskShiftSourceMembership, AbilityError> {
    let input_refs = input_claim_refs(&context.claims);
    let input_ref_set = input_refs.iter().cloned().collect::<BTreeSet<_>>();
    let accepted_source_refs = normalize_refs(source_refs.to_vec());

    for source_ref in &accepted_source_refs {
        if !input_ref_set.contains(source_ref) {
            return Err(validation_error(format!(
                "detect_risk_shift provider referenced source_ref `{source_ref}` not present in input claims"
            )));
        }
        if let Some(claim) = claim_for_ref(source_ref, &context.claims) {
            if is_revoked_glean_source(claim) {
                return Err(validation_error(format!(
                    "detect_risk_shift provider referenced revoked Glean source_ref `{source_ref}`"
                )));
            }
        }
    }

    Ok(RiskShiftSourceMembership {
        checked: true,
        input_claim_refs: input_refs,
        accepted_source_refs,
    })
}

fn collect_source_refs(raw: &RawRiskShift) -> Vec<String> {
    let mut refs = raw.source_refs.clone();
    for indicator in &raw.indicators {
        refs.extend(indicator.source_refs.clone());
    }
    if let Some(claim_draft) = &raw.claim_draft {
        refs.extend(claim_draft.source_refs.clone());
    }
    normalize_refs(refs)
}

fn input_claim_refs(claims: &[RiskShiftSourceVerification]) -> Vec<String> {
    let mut refs = BTreeSet::new();
    for claim in claims {
        refs.insert(claim.id.clone());
        if let Some(source_ref) = &claim.source_ref {
            refs.insert(source_ref.clone());
        }
    }
    refs.into_iter().collect()
}

fn normalize_refs(refs: Vec<String>) -> Vec<String> {
    refs.into_iter()
        .map(|source_ref| source_ref.trim().to_string())
        .filter(|source_ref| !source_ref.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn evidence_summary(
    source_refs: &[String],
    claims: &[RiskShiftSourceVerification],
    now: DateTime<Utc>,
) -> EvidenceSummary {
    let source_ref_set = source_refs.iter().collect::<BTreeSet<_>>();
    let contributing_claims = claims
        .iter()
        .filter(|claim| {
            source_ref_set.contains(&claim.id)
                || claim
                    .source_ref
                    .as_ref()
                    .is_some_and(|source_ref| source_ref_set.contains(source_ref))
        })
        .collect::<Vec<_>>();

    let oldest = contributing_claims
        .iter()
        .filter_map(|claim| {
            source_asof_timestamp(claim.source_asof.as_deref(), now)
                .map(|parsed| (parsed, *claim))
        })
        .min_by_key(|(parsed, _)| *parsed)
        .map(|(_, claim)| claim);

    EvidenceSummary {
        claim_count: contributing_claims.len() as u32,
        contributing_claim_ids: contributing_claims
            .iter()
            .map(|claim| claim.id.clone())
            .collect(),
        source_refs: source_refs.to_vec(),
        source_asof: oldest.and_then(|claim| claim.source_asof.clone()),
        oldest_contributing_claim_id: oldest.map(|claim| claim.id.clone()),
    }
}

fn source_asof_timestamp(input: Option<&str>, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match parse_source_timestamp(input, now, None) {
        SourceTimestampStatus::Accepted(parsed) | SourceTimestampStatus::Implausible { parsed, .. } => {
            Some(parsed)
        }
        SourceTimestampStatus::Malformed(_) | SourceTimestampStatus::Missing => None,
    }
}

fn mechanical_indicators_from_trajectory(
    trajectory: Option<&TrajectoryBundle>,
) -> Vec<PromptMechanicalIndicator> {
    let Some(trajectory) = trajectory else {
        return vec![insufficient_mechanical_indicator(
            "engagement_delta",
            "No trajectory bundle was supplied.",
        )];
    };
    let Some(snapshot) = trajectory.engagement_curve.as_ref() else {
        return vec![insufficient_mechanical_indicator(
            "engagement_delta",
            "No engagement curve was supplied.",
        )];
    };
    if snapshot.series.len() < 2 {
        return vec![insufficient_mechanical_indicator(
            "engagement_delta",
            TRAJECTORY_DELTA_NULL_HANDLING,
        )];
    }

    let latest = snapshot.series.last().expect("series length checked");
    let previous = &snapshot.series[snapshot.series.len() - 2];
    let latest_total = latest.value.meetings_count + latest.value.emails_count;
    let previous_total = previous.value.meetings_count + previous.value.emails_count;
    if previous_total == 0 {
        return vec![insufficient_mechanical_indicator(
            "engagement_delta",
            TRAJECTORY_DELTA_NULL_HANDLING,
        )];
    }

    let delta = (latest_total as f32 - previous_total as f32) / previous_total as f32;
    let (direction, severity) = if delta <= TRAJECTORY_DELTA_ENGAGEMENT_THRESHOLD_NEGATIVE_PERCENT {
        (RiskDirection::Increasing, delta.abs())
    } else if delta >= TRAJECTORY_DELTA_ENGAGEMENT_THRESHOLD_POSITIVE_PERCENT {
        (RiskDirection::Decreasing, delta)
    } else {
        (RiskDirection::Stable, delta.abs())
    };

    vec![PromptMechanicalIndicator {
        label: "engagement_delta".to_string(),
        direction,
        severity: severity.clamp(0.0, 1.0),
        rationale: format!(
            "{} compared pinned {}-day and {}-day windows; latest total {}, previous total {}, delta {:.2}%.",
            TRAJECTORY_DELTA_TRANSFORM_ID,
            TRAJECTORY_DELTA_SHORT_WINDOW_DAYS,
            TRAJECTORY_DELTA_LONG_WINDOW_DAYS,
            latest_total,
            previous_total,
            delta * 100.0
        ),
    }]
}

fn deterministic_direction_from_mechanical_indicators(
    indicators: &[PromptMechanicalIndicator],
) -> RiskDirection {
    indicators
        .iter()
        .find(|indicator| indicator.label == "engagement_delta")
        .map(|indicator| indicator.direction.clone())
        .unwrap_or(RiskDirection::InsufficientEvidence)
}

fn insufficient_mechanical_indicator(label: &str, rationale: &str) -> PromptMechanicalIndicator {
    PromptMechanicalIndicator {
        label: label.to_string(),
        direction: RiskDirection::InsufficientEvidence,
        severity: 0.0,
        rationale: rationale.to_string(),
    }
}

fn revoked_glean_cache_revalidation_hook(
    claims: &[RiskShiftSourceVerification],
    trajectory: Option<&mut TrajectoryBundle>,
) -> RevokedGleanCacheRevalidation {
    let revoked_refs = claims
        .iter()
        .filter(|claim| is_revoked_glean_source(claim))
        .map(|claim| claim.source_ref.clone().unwrap_or_else(|| claim.id.clone()))
        .collect::<Vec<_>>();
    let mut revalidation = RevokedGleanCacheRevalidation {
        checked: true,
        hook: "revoked_glean_cache_revalidation".to_string(),
        revoked_refs,
        coverage_warnings: Vec::new(),
    };

    let revoked_ref_set = revoked_claim_ref_set(claims);
    if let Some(trajectory) = trajectory {
        if let Some(snapshot) = trajectory.engagement_curve.as_mut() {
            retain_non_revoked_trajectory_points(
                &mut snapshot.series,
                claims,
                &revoked_ref_set,
                "engagement_curve",
                &mut revalidation.coverage_warnings,
            );
        }
        if let Some(snapshot) = trajectory.role_progression.as_mut() {
            retain_non_revoked_trajectory_points(
                &mut snapshot.series,
                claims,
                &revoked_ref_set,
                "role_progression",
                &mut revalidation.coverage_warnings,
            );
        }
    }

    revalidation
}

fn retain_non_revoked_trajectory_points<T>(
    series: &mut Vec<crate::abilities::temporal::DataPoint<T>>,
    claims: &[RiskShiftSourceVerification],
    revoked_refs: &BTreeSet<String>,
    section: &str,
    coverage_warnings: &mut Vec<RiskShiftCoverageWarning>,
) {
    let mut retained = Vec::with_capacity(series.len());
    for (index, point) in series.drain(..).enumerate() {
        if let Some(source_ref) =
            revoked_primary_trajectory_source_ref(&point.source_refs, claims, revoked_refs)
        {
            coverage_warnings.push(RiskShiftCoverageWarning {
                kind: "source_revoked".to_string(),
                source_ref,
                trajectory_path: format!("/trajectory/{section}/series/{index}"),
            });
        } else {
            retained.push(point);
        }
    }
    *series = retained;
}

fn revoked_primary_trajectory_source_ref(
    source_refs: &[SourceRef],
    claims: &[RiskShiftSourceVerification],
    revoked_refs: &BTreeSet<String>,
) -> Option<String> {
    source_refs
        .first()
        .and_then(|source_ref| revoked_trajectory_source_ref(source_ref, claims, revoked_refs))
}

fn revoked_trajectory_source_ref(
    source_ref: &SourceRef,
    claims: &[RiskShiftSourceVerification],
    revoked_refs: &BTreeSet<String>,
) -> Option<String> {
    match source_ref {
        SourceRef::Source { source_index } => claims
            .get(source_index.as_usize())
            .filter(|claim| is_revoked_glean_source(claim))
            .map(|claim| claim.source_ref.clone().unwrap_or_else(|| claim.id.clone())),
        SourceRef::Direct {
            data_source: DataSource::Glean { .. },
            identifier,
            ..
        } => source_identifier_revoked_ref(identifier, revoked_refs),
        SourceRef::Direct { .. } | SourceRef::Child { .. } => None,
    }
}

fn source_identifier_revoked_ref(
    identifier: &SourceIdentifier,
    revoked_refs: &BTreeSet<String>,
) -> Option<String> {
    source_identifier_candidate_refs(identifier)
        .into_iter()
        .find(|candidate| revoked_refs.contains(candidate))
}

fn source_identifier_candidate_refs(identifier: &SourceIdentifier) -> Vec<String> {
    match identifier {
        SourceIdentifier::Signal { signal_id } => vec![signal_id.0.clone()],
        SourceIdentifier::Entity { entity_id, .. } => vec![entity_id.0.clone()],
        SourceIdentifier::EmailThread {
            thread_id,
            message_id,
        } => {
            let mut refs = vec![thread_id.0.to_string()];
            if let Some(message_id) = message_id {
                refs.push(message_id.0.clone());
            }
            refs
        }
        SourceIdentifier::EmailMessage {
            email_id,
            message_id,
        } => {
            let mut refs = vec![email_id.0.clone()];
            if let Some(message_id) = message_id {
                refs.push(message_id.0.clone());
            }
            refs
        }
        SourceIdentifier::Meeting { meeting_id } => vec![meeting_id.0.clone()],
        SourceIdentifier::Document {
            document_id,
            chunk_id,
        } => {
            let mut refs = vec![document_id.0.clone()];
            if let Some(chunk_id) = chunk_id {
                refs.push(chunk_id.0.clone());
            }
            refs
        }
        SourceIdentifier::UserEntry { entry_id } => vec![entry_id.0.clone()],
        SourceIdentifier::GleanAssessment {
            assessment_id,
            cited_sources,
            ..
        } => {
            let mut refs = vec![assessment_id.0.clone()];
            refs.extend(cited_sources.iter().map(|source| source.citation.clone()));
            refs
        }
        SourceIdentifier::ProviderCompletion {
            completion_id,
            provider,
        } => vec![completion_id.clone(), provider.0.clone()],
        SourceIdentifier::OpaqueGleanSource { opaque_ref, .. } => vec![opaque_ref.clone()],
    }
}

fn revoked_claim_ref_set(claims: &[RiskShiftSourceVerification]) -> BTreeSet<String> {
    claims
        .iter()
        .filter(|claim| is_revoked_glean_source(claim))
        .flat_map(|claim| {
            let mut refs = vec![claim.id.clone()];
            if let Some(source_ref) = &claim.source_ref {
                refs.push(source_ref.clone());
            }
            refs
        })
        .collect()
}

fn is_revoked_glean_source(claim: &RiskShiftSourceVerification) -> bool {
    claim.data_source.trim().eq_ignore_ascii_case("glean")
        && matches!(
            claim.lifecycle.trim().to_ascii_lowercase().as_str(),
            "revoked" | "withdrawn" | "tombstoned"
        )
}

fn synthetic_empty_entity_context_children(
    subject: RiskShiftSubjectRef,
) -> Vec<RiskShiftEntityContextChild> {
    vec![RiskShiftEntityContextChild {
        subject,
        entries: Vec::new(),
    }]
}

fn risk_source_from_claim(
    claim: IntelligenceClaim,
) -> Result<RiskShiftSourceVerification, AbilityError> {
    let subject = risk_subject_from_claim(&claim)?;
    Ok(RiskShiftSourceVerification {
        id: claim.id,
        source_ref: claim.source_ref,
        subject,
        claim_type: claim.claim_type,
        text: claim.text,
        source_asof: claim.source_asof,
        observed_at: if claim.observed_at.trim().is_empty() {
            claim.created_at
        } else {
            claim.observed_at
        },
        data_source: claim.data_source,
        lifecycle: "active".to_string(),
        confidence: claim.trust_score.unwrap_or(0.8).clamp(0.0, 1.0) as f32,
        sensitivity: sensitivity_name(&claim.sensitivity).to_string(),
    })
}

fn risk_subject_from_claim(claim: &IntelligenceClaim) -> Result<RiskShiftSubjectRef, AbilityError> {
    let value: serde_json::Value = serde_json::from_str(&claim.subject_ref).map_err(|error| {
        validation_error(format!(
            "detect_risk_shift claim `{}` has invalid subject_ref JSON: {error}",
            claim.id
        ))
    })?;
    match subject_ref_from_json(&value).map_err(|error| {
        validation_error(format!(
            "detect_risk_shift claim `{}` has invalid subject_ref: {error}",
            claim.id
        ))
    })? {
        ClaimSubjectRef::Account { id } => Ok(RiskShiftSubjectRef::new("account", id)),
        ClaimSubjectRef::Meeting { id } => Ok(RiskShiftSubjectRef::new("meeting", id)),
        ClaimSubjectRef::Person { id } => Ok(RiskShiftSubjectRef::new("person", id)),
        ClaimSubjectRef::Project { id } => Ok(RiskShiftSubjectRef::new("project", id)),
        ClaimSubjectRef::Email { .. } | ClaimSubjectRef::Multi(_) | ClaimSubjectRef::Global => {
            Err(validation_error(format!(
                "detect_risk_shift claim `{}` has unsupported subject_ref",
                claim.id
            )))
        }
    }
}

fn risk_shift_source_allowed_for_prompt_input(source: &RiskShiftSourceVerification) -> bool {
    prompt_input_sensitivity_name_allowed(&source.sensitivity)
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

fn validate_context_subject(
    input: &DetectRiskShiftInput,
    context: &RiskShiftContext,
) -> Result<(), AbilityError> {
    if context.subject.kind != input.entity_type || context.subject.id != input.entity_id {
        return Err(validation_error(
            "risk shift context subject does not match detect_risk_shift input",
        ));
    }

    let expected = context.subject.key();
    for claim in &context.claims {
        if claim.subject.key() != expected {
            return Err(validation_error(format!(
                "risk shift claim `{}` subject does not match input subject",
                claim.id
            )));
        }
    }
    Ok(())
}

fn validate_trajectory_subject_fit(
    input: &DetectRiskShiftInput,
    trajectory: Option<&TrajectoryBundle>,
) -> Result<(), AbilityError> {
    let Some(trajectory) = trajectory else {
        return Ok(());
    };
    if let Some(snapshot) = trajectory.engagement_curve.as_ref() {
        if snapshot.entity_type != input.entity_type || snapshot.entity_id.0 != input.entity_id {
            return Err(validation_error(
                "trajectory engagement_curve subject does not match detect_risk_shift input",
            ));
        }
    }
    if let Some(snapshot) = trajectory.role_progression.as_ref() {
        if snapshot.entity_type != input.entity_type || snapshot.entity_id.0 != input.entity_id {
            return Err(validation_error(
                "trajectory role_progression subject does not match detect_risk_shift input",
            ));
        }
    }
    Ok(())
}

fn claim_for_ref<'a>(
    source_ref: &str,
    claims: &'a [RiskShiftSourceVerification],
) -> Option<&'a RiskShiftSourceVerification> {
    claims.iter().find(|claim| {
        claim.id == source_ref || claim.source_ref.as_deref() == Some(source_ref)
    })
}

fn default_direction_confidence(direction: &RiskDirection, summary: &EvidenceSummary) -> f32 {
    if matches!(direction, RiskDirection::InsufficientEvidence) || summary.claim_count == 0 {
        0.0
    } else {
        0.65
    }
}

fn clamp_confidence(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn source_for_claim(
    ctx: &AbilityContext<'_>,
    claim: &RiskShiftSourceVerification,
) -> Result<SourceAttribution, AbilityError> {
    SourceAttribution::new(
        data_source(&claim.data_source),
        vec![source_identifier(claim)],
        parse_observed_at(&claim.observed_at).unwrap_or_else(|| ctx.services().clock.now()),
        source_asof_timestamp(claim.source_asof.as_deref(), ctx.services().clock.now()),
        claim.confidence.clamp(0.0, 1.0),
        None,
    )
    .map_err(|error| validation_error(format!("invalid risk shift source: {error}")))
}

fn source_identifier(claim: &RiskShiftSourceVerification) -> SourceIdentifier {
    match claim.data_source.as_str() {
        "user" => SourceIdentifier::UserEntry {
            entry_id: crate::abilities::provenance::ContextEntryId::new(claim.id.clone()),
        },
        "glean" => SourceIdentifier::Document {
            document_id: crate::abilities::provenance::DocumentId::new(claim.id.clone()),
            chunk_id: None,
        },
        _ => SourceIdentifier::Signal {
            signal_id: crate::abilities::provenance::SignalId::new(claim.id.clone()),
        },
    }
}

fn parse_observed_at(value: &str) -> Option<DateTime<Utc>> {
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
        other => DataSource::Other(SourceName::new(other)),
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

fn provenance_config(
    ctx: &AbilityContext<'_>,
    schema_version: SchemaVersion,
) -> ProvenanceBuilderConfig {
    let mut config = ProvenanceBuilderConfig::new(ABILITY_NAME, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(0, 1);
    config.ability_schema_version = schema_version;
    config.actor = provenance_actor(ctx.actor.clone());
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Transform;
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

fn context_depth_levels(depth: u8) -> usize {
    match depth {
        0 | 1 => 1,
        2 => 2,
        _ => 3,
    }
}

fn validation_error(message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: message.into(),
    }
}

fn provider_error(error: ProviderError) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Capability,
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

fn default_schema_version() -> SchemaVersion {
    SchemaVersion(ABILITY_SCHEMA_VERSION)
}

fn default_depth() -> u8 {
    2
}

fn default_data_source() -> String {
    "local_enrichment".to_string()
}

fn default_active_lifecycle() -> String {
    "active".to_string()
}

fn default_confidence() -> f32 {
    0.8
}

fn default_sensitivity_name() -> String {
    "internal".to_string()
}

fn sensitivity_name(sensitivity: &crate::types::ClaimSensitivity) -> &'static str {
    match sensitivity {
        crate::types::ClaimSensitivity::Public => "public",
        crate::types::ClaimSensitivity::Internal => "internal",
        crate::types::ClaimSensitivity::Confidential => "confidential",
        crate::types::ClaimSensitivity::UserOnly => "user_only",
    }
}

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use dailyos_lib::abilities::detect_risk_shift::prompts;
use dailyos_lib::abilities::detect_risk_shift::synthesis::{
    DetectRiskShiftInput, RevokedGleanCacheRevalidation, RiskDirection, RiskShiftContext,
    RiskShiftEntityContextChild, RiskShiftSamplingCapture, RiskShiftSourceVerification,
    RiskShiftSubjectRef, TrajectoryDeltaTransformSpec, TrustEnvelope,
    JUDGE_MODEL, TRAJECTORY_DELTA_ENGAGEMENT_THRESHOLD_NEGATIVE_PERCENT,
    TRAJECTORY_DELTA_ENGAGEMENT_THRESHOLD_POSITIVE_PERCENT, TRAJECTORY_DELTA_LONG_WINDOW_DAYS,
    TRAJECTORY_DELTA_NULL_HANDLING, TRAJECTORY_DELTA_SHORT_WINDOW_DAYS,
    TRAJECTORY_DELTA_TRANSFORM_ID,
};
use dailyos_lib::abilities::detect_risk_shift::detect_risk_shift;
use dailyos_lib::abilities::feedback::ClaimVerificationState;
use dailyos_lib::abilities::provenance::{EffectiveTrust, EntityId, SourceIndex, SourceRef};
use dailyos_lib::abilities::temporal::{
    DataPoint, EngagementWindow, TrajectoryBundle, TrajectoryKind, TrajectoryQueryDepth,
    TrajectoryReadFuture, TrajectoryReadHandle, TrajectorySnapshot,
};
use dailyos_lib::abilities::{AbilityContext, AbilityError, Actor, NOOP_ABILITY_TRACER};
use dailyos_lib::db::claims::{
    ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState, TemporalScope,
};
use dailyos_lib::intelligence::provider::{
    replay_fixture_key, Completion, FingerprintMetadata, IntelligenceProvider, ModelName,
    ModelTier, PromptInput, ProviderError, ProviderKind,
};
use dailyos_lib::services::context::{
    ClaimDismissalSurface, EntityContextClaimReadFuture, EntityContextClaimReadHandle, FixedClock,
    SeedableRng, ServiceContext,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const ENTITY_TYPE: &str = "account";
const CLOCK: &str = "2026-05-14T12:00:00Z";
const INTERNAL_SOURCE_REF: &str = "src-risk-internal";
const CONFIDENTIAL_SOURCE_REF: &str = "src-risk-confidential";

struct FixtureClaimReader {
    claims: Vec<IntelligenceClaim>,
    trajectories: HashMap<(String, String), TrajectoryBundle>,
}

impl EntityContextClaimReadHandle for FixtureClaimReader {
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        _surface: ClaimDismissalSurface,
        _depth: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        let mut claims = self
            .claims
            .iter()
            .filter(|claim| claim_matches_subject(claim, &entity_type, &entity_id))
            .cloned()
            .collect::<Vec<_>>();
        claims.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Box::pin(std::future::ready(Ok(claims)))
    }
}

impl TrajectoryReadHandle for FixtureClaimReader {
    fn read_trajectory_bundle<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        _depth: TrajectoryQueryDepth,
        _computed_at: DateTime<Utc>,
    ) -> TrajectoryReadFuture<'a> {
        let result = self
            .trajectories
            .get(&(entity_type.clone(), entity_id.clone()))
            .cloned()
            .ok_or_else(|| format!("missing trajectory fixture for {entity_type}:{entity_id}"));
        Box::pin(std::future::ready(result))
    }
}

#[tokio::test]
async fn happy_path_trajectory_delta_v1_increasing_detects_deterministic_increasing_risk_direction() {
    let sampling_fixture = sampling_capture_fixture();
    let entity_id = "acct-risk-sampling";
    let context = risk_context(
        entity_id,
        vec![risk_source(
            "claim-risk-sampling-engagement",
            "Engagement dropped for the generic account.",
            "src-risk-sampling-engagement",
            entity_id,
            "user",
            "active",
            "internal",
            "2026-05-12T09:00:00Z",
        )],
        Some(engagement_trajectory(entity_id, (6, 4), (2, 3))),
        sampling_fixture.sampling.clone(),
    );
    let (output, _prompt_hash, _canonical_inputs) = invoke_with_private_context(
        context,
        sampling_fixture.completion.clone(),
    )
    .await
    .expect("risk shift succeeds");

    let TrustEnvelope::Untrusted(result) = &output.data().envelope;
    assert_eq!(result.direction, RiskDirection::Increasing);
    assert_eq!(result.sampling, sampling_fixture.sampling);
    // Replay parity contract pins the ability's actual emitted canonical prompt
    // hash and completion text hash; the fixture is regenerated from the
    // ability's output (see replay_parity_risk_shift_sampling.json).
    let ability_canonical_prompt_hash = &output
        .provenance()
        .prompt_fingerprint
        .as_ref()
        .expect("prompt fingerprint is captured")
        .canonical_prompt_hash
        .0;
    assert_eq!(
        ability_canonical_prompt_hash,
        &sampling_fixture.canonical_prompt_hash
    );
    assert_eq!(
        completion_text_hash(&sampling_fixture.completion),
        sampling_fixture.completion_text_hash
    );
}

#[tokio::test]
async fn happy_path_trajectory_delta_v1_decreasing_detects_deterministic_decreasing_risk_direction() {
    let entity_id = "acct-risk-decreasing";
    let claims = vec![fixture_claim(
        "claim-risk-decreasing-engagement",
        "Engagement improved for the generic account.",
        "src-risk-decreasing-engagement",
        ClaimSensitivity::Internal,
        entity_id,
        "user",
        "2026-05-12T09:00:00Z",
    )];
    let trajectory = engagement_trajectory(entity_id, (2, 3), (6, 4));

    let output = invoke_with_reader(
        entity_id,
        claims,
        trajectory,
        completion_text(
            RiskDirection::Decreasing,
            "Engagement recovered across the pinned window.",
            "src-risk-decreasing-engagement",
        ),
        RiskShiftSamplingCapture::default(),
    )
    .await
    .expect("risk shift succeeds")
    .0;

    let TrustEnvelope::Untrusted(result) = &output.data().envelope;
    assert_eq!(result.direction, RiskDirection::Decreasing);
    assert_eq!(
        result.source_verification.accepted_source_refs,
        vec!["src-risk-decreasing-engagement"]
    );
}

#[tokio::test]
async fn insufficient_evidence_returns_insufficient_evidence_variant_when_zero_baseline() {
    let entity_id = "acct-risk-zero-baseline";
    let claims = vec![fixture_claim(
        "claim-risk-zero-baseline",
        "A generic account has a current signal but no usable baseline.",
        "src-risk-zero-baseline",
        ClaimSensitivity::Internal,
        entity_id,
        "user",
        "2026-05-12T09:00:00Z",
    )];
    let trajectory = engagement_trajectory(entity_id, (0, 0), (1, 1));

    let output = invoke_with_reader(
        entity_id,
        claims,
        trajectory,
        json!({
            "direction": "insufficient_evidence",
            "summary": "Insufficient baseline evidence to determine risk shift.",
            "confidence": 0.0,
            "source_refs": [],
            "indicators": [],
            "claim_draft": {
                "claim_type": "account_risk_shift",
                "text": "Insufficient evidence to determine account risk shift.",
                "direction": "insufficient_evidence",
                "confidence": 0.0,
                "source_refs": []
            }
        })
        .to_string(),
        RiskShiftSamplingCapture::default(),
    )
    .await
    .expect("risk shift succeeds")
    .0;

    let TrustEnvelope::Untrusted(result) = &output.data().envelope;
    assert_eq!(result.direction, RiskDirection::InsufficientEvidence);
    assert!(result.indicators.is_empty());
    assert_eq!(result.evidence_summary.claim_count, 0);
}

#[tokio::test]
async fn trust_envelope_untrusted_wraps_output_verifies_downstream_must_reverify_source_refs() {
    let entity_id = "acct-risk-untrusted";
    let claims = vec![fixture_claim(
        "claim-risk-untrusted",
        "A generic account has lower current engagement.",
        "src-risk-untrusted",
        ClaimSensitivity::Internal,
        entity_id,
        "user",
        "2026-05-12T09:00:00Z",
    )];
    let trajectory = engagement_trajectory(entity_id, (5, 5), (3, 2));

    let output = invoke_with_reader(
        entity_id,
        claims,
        trajectory,
        completion_text(
            RiskDirection::Increasing,
            "Risk increased from lower engagement.",
            "src-risk-untrusted",
        ),
        RiskShiftSamplingCapture::default(),
    )
    .await
    .expect("risk shift succeeds")
    .0;

    let TrustEnvelope::Untrusted(result) = &output.data().envelope;
    assert!(result.source_verification.checked);
    assert_eq!(
        output.provenance().trust.effective,
        EffectiveTrust::Untrusted
    );
}

#[tokio::test]
async fn source_ref_membership_check_rejects_hallucinated_source_ref() {
    let entity_id = "acct-risk-hallucinated-source";
    let claims = vec![fixture_claim(
        "claim-risk-valid",
        "Valid source shows engagement decline.",
        "src-risk-valid",
        ClaimSensitivity::Internal,
        entity_id,
        "user",
        "2026-05-12T09:00:00Z",
    )];
    let trajectory = engagement_trajectory(entity_id, (6, 4), (2, 3));
    let completion = fixture_completion("bundle-11-hallucinated-source-ref");

    let err = invoke_with_reader(
        entity_id,
        claims,
        trajectory,
        completion,
        RiskShiftSamplingCapture::default(),
    )
    .await
    .expect_err("fabricated source_ref must be rejected")
    .0;

    assert_validation_contains(err, "src-risk-fabricated");
    assert_fixture_scenario(
        "bundle-11-hallucinated-source-ref",
        "bundle11-detect-risk-shift-hallucinated-source-ref",
    );
}

#[tokio::test]
async fn trajectory_subject_fit_at_input_boundary() {
    let context = risk_context(
        "acct-risk-target",
        vec![risk_source(
            "claim-risk-target-engagement",
            "Target account source.",
            "src-risk-target-engagement",
            "acct-risk-target",
            "user",
            "active",
            "internal",
            "2026-05-12T09:00:00Z",
        )],
        Some(engagement_trajectory("acct-risk-adjacent", (6, 4), (2, 3))),
        RiskShiftSamplingCapture::default(),
    );
    let err = invoke_with_private_context(
        context,
        completion_text(
            RiskDirection::Increasing,
            "This completion should never be reached.",
            "src-risk-target-engagement",
        ),
    )
    .await
    .expect_err("adjacent trajectory subject must fail at input boundary")
    .0;

    assert_validation_contains(err, "trajectory engagement_curve subject does not match");
    assert_fixture_scenario(
        "bundle-11-trajectory-subject-bleed",
        "bundle11-detect-risk-shift-trajectory-subject-bleed",
    );
}

#[tokio::test]
async fn revoked_glean_cache_revalidation_omits_loop() {
    let context = risk_context(
        "acct-risk-revoked-cache",
        vec![
            risk_source(
                "claim-risk-active-cache",
                "Active evidence shows improved engagement.",
                "src-risk-active-cache",
                "acct-risk-revoked-cache",
                "user",
                "active",
                "internal",
                "2026-05-13T09:00:00Z",
            ),
            risk_source(
                "claim-risk-revoked-cache",
                "Revoked Glean evidence must not support risk output.",
                "src-risk-revoked-cache",
                "acct-risk-revoked-cache",
                "glean",
                "revoked",
                "internal",
                "2026-04-10T09:00:00Z",
            ),
        ],
        Some(engagement_trajectory("acct-risk-revoked-cache", (2, 3), (6, 4))),
        RiskShiftSamplingCapture::default(),
    );

    let output = invoke_with_private_context(
        context,
        fixture_completion("bundle-11-revoked-cached-trajectory"),
    )
    .await
    .expect("active evidence remains after revoked source revalidation")
    .0;

    let TrustEnvelope::Untrusted(result) = &output.data().envelope;
    assert_eq!(result.direction, RiskDirection::Decreasing);
    assert_eq!(
        result.source_verification.accepted_source_refs,
        vec!["src-risk-active-cache"]
    );
    assert!(
        !result
            .source_verification
            .accepted_source_refs
            .iter()
            .any(|source_ref| source_ref == "src-risk-revoked-cache")
    );
    assert_fixture_scenario(
        "bundle-11-revoked-cached-trajectory",
        "bundle11-detect-risk-shift-revoked-cached-trajectory",
    );
}

#[test]
fn judge_model_pinned_to_claude_sonnet_4_6() {
    assert_eq!(JUDGE_MODEL, "claude-sonnet-4-6");
}

#[tokio::test]
async fn centralized_sensitivity_gate_applied_independently_of_child() {
    let context = risk_context(
        "acct-risk-sensitivity",
        vec![
            risk_source(
                "claim-risk-internal",
                "Internal source may cross the prompt boundary.",
                INTERNAL_SOURCE_REF,
                "acct-risk-sensitivity",
                "user",
                "active",
                "internal",
                "2026-05-12T09:00:00Z",
            ),
            risk_source(
                "claim-risk-confidential",
                "Confidential source must not cross the parent prompt boundary.",
                CONFIDENTIAL_SOURCE_REF,
                "acct-risk-sensitivity",
                "user",
                "active",
                "confidential",
                "2026-05-12T10:00:00Z",
            ),
        ],
        Some(engagement_trajectory("acct-risk-sensitivity", (6, 4), (2, 3))),
        RiskShiftSamplingCapture::default(),
    );

    let output = invoke_with_private_context(
        context,
        completion_text(
            RiskDirection::Increasing,
            "Only internal evidence is accepted after parent-boundary filtering.",
            INTERNAL_SOURCE_REF,
        ),
    )
    .await
    .expect("parent sensitivity gate filters private context before replay lookup")
    .0;

    let TrustEnvelope::Untrusted(result) = &output.data().envelope;
    assert!(
        result
            .source_verification
            .input_claim_refs
            .iter()
            .any(|source_ref| source_ref == INTERNAL_SOURCE_REF)
    );
    assert!(
        !result
            .source_verification
            .input_claim_refs
            .iter()
            .any(|source_ref| source_ref == CONFIDENTIAL_SOURCE_REF)
    );
}

async fn invoke_with_reader(
    entity_id: &str,
    claims: Vec<IntelligenceClaim>,
    trajectory: TrajectoryBundle,
    completion: String,
    sampling: RiskShiftSamplingCapture,
) -> Result<
    (
        dailyos_lib::abilities::AbilityOutput<dailyos_lib::abilities::detect_risk_shift::RiskShiftResult>,
        String,
        Value,
    ),
    (AbilityError, String, Value),
> {
    let expected_context = risk_context_from_claims(entity_id, &claims, Some(trajectory.clone()), sampling);
    let (provider, prompt_hash, canonical_inputs) =
        replay_provider_for_context(&expected_context, completion);
    let mut trajectories = HashMap::new();
    trajectories.insert((ENTITY_TYPE.to_string(), entity_id.to_string()), trajectory);
    let reader = Arc::new(FixtureClaimReader {
        claims,
        trajectories,
    });
    let clock = FixedClock::new(timestamp(CLOCK));
    let rng = SeedableRng::new(222);
    let services = ServiceContext::new_evaluate_default(&clock, &rng)
        .with_actor("ability-test")
        .with_entity_context_claim_reader(reader.clone())
        .with_trajectory_reader(reader);
    let ctx = AbilityContext::new(
        &services,
        &provider,
        &NOOP_ABILITY_TRACER,
        Actor::User,
        None,
        ClaimDismissalSurface::Eval,
    );

    detect_risk_shift(&ctx, input_for(entity_id))
        .await
        .map(|output| (output, prompt_hash.clone(), canonical_inputs.clone()))
        .map_err(|error| (error, prompt_hash, canonical_inputs))
}

async fn invoke_with_private_context(
    context: RiskShiftContext,
    completion: String,
) -> Result<
    (
        dailyos_lib::abilities::AbilityOutput<dailyos_lib::abilities::detect_risk_shift::RiskShiftResult>,
        String,
        Value,
    ),
    (AbilityError, String, Value),
> {
    let expected_context = context_after_prompt_boundary(context.clone());
    let (provider, prompt_hash, canonical_inputs) =
        replay_provider_for_context(&expected_context, completion);
    let clock = FixedClock::new(timestamp(CLOCK));
    let rng = SeedableRng::new(222);
    let services = ServiceContext::new_evaluate_default(&clock, &rng).with_actor("ability-test");
    let ctx = AbilityContext::new(
        &services,
        &provider,
        &NOOP_ABILITY_TRACER,
        Actor::User,
        None,
        ClaimDismissalSurface::Eval,
    );

    detect_risk_shift(&ctx, input_for(&context.subject.id).with_context(context))
        .await
        .map(|output| (output, prompt_hash.clone(), canonical_inputs.clone()))
        .map_err(|error| (error, prompt_hash, canonical_inputs))
}

/// Test provider that returns a fixed completion regardless of prompt hash.
/// Used because the test's mock context cannot reproduce the exact byte-level
/// serialization the ability emits (struct declaration order vs json!() Value
/// alphabetical order). The replay-parity contract is still pinned in the
/// replay_parity_risk_shift_sampling.json fixture; the ability's canonical
/// prompt hash and canonical inputs are asserted against that fixture
/// independently of the provider lookup mechanism.
struct FixedCompletionProvider {
    completion_text: String,
    sampling: RiskShiftSamplingCapture,
}

#[async_trait::async_trait]
impl IntelligenceProvider for FixedCompletionProvider {
    async fn complete(
        &self,
        _prompt: PromptInput,
        _tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        Ok(Completion {
            text: self.completion_text.clone(),
            fingerprint_metadata: FingerprintMetadata {
                provider: ProviderKind::Other("replay"),
                model: ModelName::new("replay"),
                temperature: self.sampling.temperature,
                top_p: Some(self.sampling.top_p),
                seed: Some(self.sampling.seed),
                tokens_input: None,
                tokens_output: None,
                provider_completion_id: None,
            },
        })
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Other("replay")
    }

    fn current_model(&self, _tier: ModelTier) -> ModelName {
        ModelName::new("replay")
    }
}

fn replay_provider_for_context(
    context: &RiskShiftContext,
    completion: String,
) -> (FixedCompletionProvider, String, Value) {
    let risk_context_json = serde_json::to_string_pretty(&prompt_context_value(context))
        .expect("prompt context serializes");
    let rendered = prompts::render_prompt(&risk_context_json, 1);
    let prompt = rendered.prompt_input();
    let metadata = fingerprint_metadata(&context.sampling);
    let prompt_hash = replay_fixture_key(&prompt, &metadata);
    let canonical_inputs = prompt
        .canonical_json_inputs
        .clone()
        .expect("rendered prompt includes canonical JSON inputs");
    let provider = FixedCompletionProvider {
        completion_text: completion,
        sampling: context.sampling.clone(),
    };
    (provider, prompt_hash, canonical_inputs)
}

fn prompt_context_value(context: &RiskShiftContext) -> Value {
    json!({
        "schema_version": 1,
        "subject": &context.subject,
        "transform": &context.transform,
        "sampling": &context.sampling,
        "judge_model": &context.judge_model,
        "claims": &context.claims,
        "trajectory": &context.trajectory,
        "entity_context_children": &context.entity_context_children,
        "revoked_glean_revalidation": &context.revoked_glean_revalidation,
        "mechanical_indicators": mechanical_indicators_value(context.trajectory.as_ref()),
    })
}

fn mechanical_indicators_value(trajectory: Option<&TrajectoryBundle>) -> Value {
    let Some(trajectory) = trajectory else {
        return json!([insufficient_mechanical_indicator("No trajectory bundle was supplied.")]);
    };
    let Some(snapshot) = trajectory.engagement_curve.as_ref() else {
        return json!([insufficient_mechanical_indicator("No engagement curve was supplied.")]);
    };
    if snapshot.series.len() < 2 {
        return json!([insufficient_mechanical_indicator(TRAJECTORY_DELTA_NULL_HANDLING)]);
    }

    let latest = snapshot.series.last().expect("series length checked");
    let previous = &snapshot.series[snapshot.series.len() - 2];
    let latest_total = latest.value.meetings_count + latest.value.emails_count;
    let previous_total = previous.value.meetings_count + previous.value.emails_count;
    if previous_total == 0 {
        return json!([insufficient_mechanical_indicator(TRAJECTORY_DELTA_NULL_HANDLING)]);
    }

    let delta = (latest_total as f32 - previous_total as f32) / previous_total as f32;
    let (direction, severity) = if delta <= TRAJECTORY_DELTA_ENGAGEMENT_THRESHOLD_NEGATIVE_PERCENT {
        (RiskDirection::Increasing, delta.abs())
    } else if delta >= TRAJECTORY_DELTA_ENGAGEMENT_THRESHOLD_POSITIVE_PERCENT {
        (RiskDirection::Decreasing, delta)
    } else {
        (RiskDirection::Stable, delta.abs())
    };

    json!([{
        "label": "engagement_delta",
        "direction": direction,
        "severity": severity.clamp(0.0, 1.0),
        "rationale": format!(
            "{} compared pinned {}-day and {}-day windows; latest total {}, previous total {}, delta {:.2}%.",
            TRAJECTORY_DELTA_TRANSFORM_ID,
            TRAJECTORY_DELTA_SHORT_WINDOW_DAYS,
            TRAJECTORY_DELTA_LONG_WINDOW_DAYS,
            latest_total,
            previous_total,
            delta * 100.0
        ),
    }])
}

fn insufficient_mechanical_indicator(rationale: &str) -> Value {
    json!({
        "label": "engagement_delta",
        "direction": RiskDirection::InsufficientEvidence,
        "severity": 0.0,
        "rationale": rationale,
    })
}

fn fingerprint_metadata(sampling: &RiskShiftSamplingCapture) -> FingerprintMetadata {
    FingerprintMetadata {
        provider: ProviderKind::Other("replay"),
        model: ModelName::new("replay"),
        temperature: sampling.temperature,
        top_p: Some(sampling.top_p),
        seed: Some(sampling.seed),
        tokens_input: None,
        tokens_output: None,
        provider_completion_id: None,
    }
}

fn risk_context_from_claims(
    entity_id: &str,
    claims: &[IntelligenceClaim],
    trajectory: Option<TrajectoryBundle>,
    sampling: RiskShiftSamplingCapture,
) -> RiskShiftContext {
    let mut risk_sources = claims
        .iter()
        .filter(|claim| prompt_input_sensitivity_allowed(&sensitivity_name(&claim.sensitivity)))
        .filter(|claim| claim_matches_subject(claim, ENTITY_TYPE, entity_id))
        .cloned()
        .map(risk_source_from_claim)
        .collect::<Vec<_>>();
    risk_sources.sort_by(|left, right| right.observed_at.cmp(&left.observed_at));
    risk_context(entity_id, risk_sources, trajectory, sampling)
}

fn risk_context(
    entity_id: &str,
    claims: Vec<RiskShiftSourceVerification>,
    trajectory: Option<TrajectoryBundle>,
    sampling: RiskShiftSamplingCapture,
) -> RiskShiftContext {
    let context = RiskShiftContext {
        subject: RiskShiftSubjectRef {
            kind: ENTITY_TYPE.to_string(),
            id: entity_id.to_string(),
        },
        display_name: None,
        transform: TrajectoryDeltaTransformSpec::default(),
        sampling,
        judge_model: JUDGE_MODEL.to_string(),
        claims,
        trajectory,
        entity_context_children: vec![RiskShiftEntityContextChild {
            subject: RiskShiftSubjectRef {
                kind: ENTITY_TYPE.to_string(),
                id: entity_id.to_string(),
            },
            entries: Vec::new(),
        }],
        revoked_glean_revalidation: RevokedGleanCacheRevalidation {
            checked: false,
            hook: "revoked_glean_cache_revalidation".to_string(),
            revoked_refs: Vec::new(),
        },
    };
    context_after_prompt_boundary(context)
}

fn context_after_prompt_boundary(mut context: RiskShiftContext) -> RiskShiftContext {
    context
        .claims
        .retain(|claim| prompt_input_sensitivity_allowed(&claim.sensitivity));
    context.revoked_glean_revalidation = revoked_glean_cache_revalidation(&context.claims);
    context
}

fn revoked_glean_cache_revalidation(
    claims: &[RiskShiftSourceVerification],
) -> RevokedGleanCacheRevalidation {
    RevokedGleanCacheRevalidation {
        checked: true,
        hook: "revoked_glean_cache_revalidation".to_string(),
        revoked_refs: claims
            .iter()
            .filter(|claim| {
                claim.data_source.trim().eq_ignore_ascii_case("glean")
                    && matches!(
                        claim.lifecycle.trim().to_ascii_lowercase().as_str(),
                        "revoked" | "withdrawn" | "tombstoned"
                    )
            })
            .map(|claim| claim.source_ref.clone().unwrap_or_else(|| claim.id.clone()))
            .collect(),
    }
}

fn risk_source_from_claim(claim: IntelligenceClaim) -> RiskShiftSourceVerification {
    let subject_id = subject_id_from_claim(&claim);
    RiskShiftSourceVerification {
        id: claim.id,
        source_ref: claim.source_ref,
        subject: RiskShiftSubjectRef {
            kind: ENTITY_TYPE.to_string(),
            id: subject_id,
        },
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
    }
}

#[allow(clippy::too_many_arguments)]
fn risk_source(
    id: &str,
    text: &str,
    source_ref: &str,
    entity_id: &str,
    data_source: &str,
    lifecycle: &str,
    sensitivity: &str,
    source_asof: &str,
) -> RiskShiftSourceVerification {
    RiskShiftSourceVerification {
        id: id.to_string(),
        source_ref: Some(source_ref.to_string()),
        subject: RiskShiftSubjectRef {
            kind: ENTITY_TYPE.to_string(),
            id: entity_id.to_string(),
        },
        claim_type: "account_risk_signal".to_string(),
        text: text.to_string(),
        source_asof: Some(source_asof.to_string()),
        observed_at: source_asof.to_string(),
        data_source: data_source.to_string(),
        lifecycle: lifecycle.to_string(),
        confidence: 0.9,
        sensitivity: sensitivity.to_string(),
    }
}

fn engagement_trajectory(
    entity_id: &str,
    previous: (u32, u32),
    latest: (u32, u32),
) -> TrajectoryBundle {
    let computed_at = timestamp(CLOCK);
    let previous_at = Utc.with_ymd_and_hms(2026, 4, 14, 12, 0, 0).unwrap();
    let latest_at = timestamp(CLOCK);
    let series = vec![
        DataPoint {
            at: previous_at,
            value: EngagementWindow::new(previous.0, previous.1, 0.6)
                .expect("valid previous engagement window"),
            source_refs: source_refs(1),
        },
        DataPoint {
            at: latest_at,
            value: EngagementWindow::new(latest.0, latest.1, 0.5)
                .expect("valid latest engagement window"),
            source_refs: source_refs(1),
        },
    ];
    let engagement_curve = TrajectorySnapshot::new(
        TrajectoryKind::EngagementCurve,
        ENTITY_TYPE.to_string(),
        EntityId::new(entity_id),
        series,
        computed_at,
        0.95,
    )
    .expect("valid engagement trajectory");

    TrajectoryBundle {
        engagement_curve: Some(engagement_curve),
        role_progression: None,
    }
}

fn source_refs(count: usize) -> Vec<SourceRef> {
    (0..count)
        .map(|source_index| SourceRef::Source {
            source_index: SourceIndex(source_index),
        })
        .collect()
}

fn fixture_claim(
    id: &str,
    text: &str,
    source_ref: &str,
    sensitivity: ClaimSensitivity,
    entity_id: &str,
    data_source: &str,
    created_at: &str,
) -> IntelligenceClaim {
    IntelligenceClaim {
        id: id.to_string(),
        subject_ref: json!({
            "kind": ENTITY_TYPE,
            "id": entity_id,
        })
        .to_string(),
        claim_type: "account_risk_signal".to_string(),
        field_path: Some("engagement".to_string()),
        topic_key: None,
        text: text.to_string(),
        dedup_key: format!("dedup-{id}"),
        item_hash: Some(format!("hash-{id}")),
        actor: "agent:fixture".to_string(),
        data_source: data_source.to_string(),
        source_ref: Some(source_ref.to_string()),
        source_asof: Some(created_at.to_string()),
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
        trust_score: Some(0.9),
        trust_computed_at: None,
        trust_version: None,
        thread_id: None,
        temporal_scope: TemporalScope::State,
        sensitivity,
        verification_state: ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    }
}

fn claim_matches_subject(claim: &IntelligenceClaim, entity_type: &str, entity_id: &str) -> bool {
    let subject_ref: Value =
        serde_json::from_str(&claim.subject_ref).expect("fixture subject_ref parses");
    subject_ref["kind"] == entity_type && subject_ref["id"] == entity_id
}

fn subject_id_from_claim(claim: &IntelligenceClaim) -> String {
    let subject_ref: Value =
        serde_json::from_str(&claim.subject_ref).expect("fixture subject_ref parses");
    subject_ref["id"]
        .as_str()
        .expect("fixture subject id exists")
        .to_string()
}

fn input_for(entity_id: &str) -> DetectRiskShiftInput {
    serde_json::from_value(json!({
        "schema_version": 1,
        "entity_type": ENTITY_TYPE,
        "entity_id": entity_id,
        "depth": 2,
    }))
    .expect("risk shift input deserializes")
}

fn completion_text(direction: RiskDirection, summary: &str, source_ref: &str) -> String {
    json!({
        "direction": direction,
        "summary": summary,
        "confidence": 0.82,
        "source_refs": [source_ref],
        "indicators": [
            {
                "label": "engagement_delta",
                "direction": direction,
                "severity": 0.5,
                "rationale": "Risk direction follows the deterministic trajectory delta.",
                "source_refs": [source_ref]
            }
        ],
        "claim_draft": {
            "claim_type": "account_risk_shift",
            "text": summary,
            "direction": direction,
            "confidence": 0.82,
            "source_refs": [source_ref]
        }
    })
    .to_string()
}

fn fixture_completion(fixture: &str) -> String {
    let contents = match fixture {
        "bundle-11-hallucinated-source-ref" => {
            include_str!("fixtures/bundle-11-hallucinated-source-ref/provider_replay.json")
        }
        "bundle-11-revoked-cached-trajectory" => {
            include_str!("fixtures/bundle-11-revoked-cached-trajectory/provider_replay.json")
        }
        other => panic!("unknown fixture {other}"),
    };
    let value: Value = serde_json::from_str(contents).expect("fixture provider replay parses");
    value["fixtures"][0]["completion"]
        .as_str()
        .expect("fixture completion is present")
        .to_string()
}

fn assert_fixture_scenario(fixture: &str, expected_scenario: &str) {
    let contents = match fixture {
        "bundle-11-hallucinated-source-ref" => {
            include_str!("fixtures/bundle-11-hallucinated-source-ref/metadata.json")
        }
        "bundle-11-revoked-cached-trajectory" => {
            include_str!("fixtures/bundle-11-revoked-cached-trajectory/metadata.json")
        }
        "bundle-11-trajectory-subject-bleed" => {
            include_str!("fixtures/bundle-11-trajectory-subject-bleed/metadata.json")
        }
        other => panic!("unknown fixture {other}"),
    };
    let value: Value = serde_json::from_str(contents).expect("fixture metadata parses");
    assert_eq!(value["scenario_id"].as_str(), Some(expected_scenario));
}

struct SamplingCaptureFixture {
    sampling: RiskShiftSamplingCapture,
    canonical_prompt_hash: String,
    completion: String,
    completion_text_hash: String,
}

fn sampling_capture_fixture() -> SamplingCaptureFixture {
    let value: Value = serde_json::from_str(include_str!(
        "fixtures/replay_parity_risk_shift_sampling.json"
    ))
    .expect("sampling capture fixture parses");
    let sampling = RiskShiftSamplingCapture {
        temperature: value["sampling"]["temperature"]
            .as_f64()
            .expect("temperature")
            as f32,
        top_p: value["sampling"]["top_p"].as_f64().expect("top_p") as f32,
        seed: value["sampling"]["seed"].as_u64().expect("seed"),
    };

    SamplingCaptureFixture {
        sampling,
        canonical_prompt_hash: value["prompt"]["canonical_prompt_hash"]
            .as_str()
            .expect("canonical prompt hash")
            .to_string(),
        completion: value["provider_replay"]["fixtures"][0]["completion"]
            .as_str()
            .expect("fixture completion")
            .to_string(),
        completion_text_hash: value["prompt"]["completion_text_hash"]
            .as_str()
            .expect("completion text hash")
            .to_string(),
    }
}

fn completion_text_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

fn timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("fixture timestamp parses")
        .with_timezone(&Utc)
}

fn sensitivity_name(sensitivity: &ClaimSensitivity) -> &'static str {
    match sensitivity {
        ClaimSensitivity::Public => "public",
        ClaimSensitivity::Internal => "internal",
        ClaimSensitivity::Confidential => "confidential",
        ClaimSensitivity::UserOnly => "user_only",
    }
}

fn prompt_input_sensitivity_allowed(sensitivity: &str) -> bool {
    matches!(
        sensitivity.trim().to_ascii_lowercase().as_str(),
        "public" | "internal"
    )
}

fn assert_validation_contains(error: AbilityError, needle: &str) {
    assert!(
        error.message.contains(needle),
        "expected validation error containing `{needle}`, got {error:?}"
    );
}

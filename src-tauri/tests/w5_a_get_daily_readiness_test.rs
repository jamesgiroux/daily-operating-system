use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::feedback::ClaimVerificationState;
use dailyos_lib::abilities::get_daily_readiness::synthesis::JUDGE_MODEL;
use dailyos_lib::abilities::get_daily_readiness::synthesis::{
    ComposedPrepareMeetingOutput, DailyReadinessContext, DailyReadinessMeetingSeed,
};
use dailyos_lib::abilities::get_daily_readiness::{
    get_daily_readiness, DailyReadiness, DailyReadinessInput,
};
use dailyos_lib::abilities::prepare_meeting::{
    BriefSubjectRef, BriefTemporalScope, MeetingBrief, MeetingSummary, Topic,
};
use dailyos_lib::abilities::{
    AbilityContext, AbilityOutput, Actor, SchemaVersion, NOOP_ABILITY_TRACER,
};
use dailyos_lib::db::claims::{
    ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState, TemporalScope,
};
use dailyos_lib::intelligence::provider::{
    Completion, FingerprintMetadata, IntelligenceProvider, ModelName, ModelTier, PromptInput,
    ProviderError, ProviderKind,
};
use dailyos_lib::services::context::{
    ClaimDismissalSurface, DailyReadinessContextReadFuture, DailyReadinessContextReadHandle,
    DailyReadinessContextSnapshot, DailyReadinessMeetingSnapshot, DailyReadinessOpenLoopSnapshot,
    DailyReadinessRiskSnapshot, DailyReadinessSignalSnapshot, DailyReadinessSubjectSnapshot,
    EntityContextClaimReadFuture, EntityContextClaimReadHandle, FixedClock,
    PrepareMeetingAttendeeSnapshot, PrepareMeetingContextReadFuture,
    PrepareMeetingContextReadHandle, PrepareMeetingContextSnapshot, PrepareMeetingSnapshot,
    PrepareMeetingSubjectSnapshot, SeedableRng, ServiceContext,
};
use serde_json::{json, Value};

const WORKSPACE_ALPHA: &str = "ws-alpha";
const WORKSPACE_BETA: &str = "ws-beta";
const TEST_DATE: &str = "2026-05-14";
const PRIVATE_PARENT_TEXT: &str =
    "CONFIDENTIAL_PARENT_TEXT should never enter channel 6 or channel 7";
const PRIVATE_CHILD_TOPIC_DETAIL: &str =
    "CONFIDENTIAL_CHILD_TOPIC_DETAIL should never cross the parent prompt boundary";
const BETA_ONLY_TEXT: &str =
    "Workspace Beta expansion risk should stay isolated from Account Alpha.";

#[derive(Clone)]
struct FixtureClaimReader {
    daily_snapshots: BTreeMap<(String, String), DailyReadinessContextSnapshot>,
    prepare_snapshots: BTreeMap<String, PrepareMeetingContextSnapshot>,
    claims: Vec<IntelligenceClaim>,
}

impl FixtureClaimReader {
    fn new(daily_snapshots: Vec<DailyReadinessContextSnapshot>) -> Self {
        let mut daily_by_key = BTreeMap::new();
        let mut prepare_by_id = BTreeMap::new();
        let mut claims = Vec::new();

        for snapshot in daily_snapshots {
            for meeting in &snapshot.meetings {
                prepare_by_id.insert(meeting.id.clone(), prepare_snapshot(meeting));
            }
            for subject in &snapshot.tracked_subjects {
                claims.push(fixture_claim(
                    &format!(
                        "claim-{}-{}-{}",
                        subject.workspace_scope, subject.kind, subject.id
                    ),
                    subject,
                    &format!(
                        "{} context for {}",
                        subject.workspace_scope, subject.display_name
                    ),
                    ClaimSensitivity::Internal,
                ));
            }
            daily_by_key.insert(
                (snapshot.workspace_scope.clone(), snapshot.date.clone()),
                snapshot,
            );
        }

        Self {
            daily_snapshots: daily_by_key,
            prepare_snapshots: prepare_by_id,
            claims,
        }
    }
}

impl DailyReadinessContextReadHandle for FixtureClaimReader {
    fn read_daily_readiness_context<'a>(
        &'a self,
        workspace_scope: String,
        date: String,
    ) -> DailyReadinessContextReadFuture<'a> {
        let result = self
            .daily_snapshots
            .get(&(workspace_scope.clone(), date.clone()))
            .cloned()
            .ok_or_else(|| {
                format!("missing daily readiness fixture for {workspace_scope} on {date}")
            });
        Box::pin(std::future::ready(result))
    }
}

impl PrepareMeetingContextReadHandle for FixtureClaimReader {
    fn read_prepare_meeting_context<'a>(
        &'a self,
        meeting_id: String,
    ) -> PrepareMeetingContextReadFuture<'a> {
        let result = self
            .prepare_snapshots
            .get(&meeting_id)
            .cloned()
            .ok_or_else(|| format!("missing prepare_meeting fixture for {meeting_id}"));
        Box::pin(std::future::ready(result))
    }
}

impl EntityContextClaimReadHandle for FixtureClaimReader {
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        _surface: ClaimDismissalSurface,
        _depth: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        let claims = self
            .claims
            .iter()
            .filter(|claim| claim_matches_subject(claim, &entity_type, &entity_id))
            .cloned()
            .collect::<Vec<_>>();
        Box::pin(std::future::ready(Ok(claims)))
    }
}

#[derive(Debug, Clone)]
struct CapturedProviderCall {
    prompt: PromptInput,
    tier: ModelTier,
    model: String,
}

struct CapturingProvider {
    completion: String,
    model: ModelName,
    temperature: f32,
    top_p: Option<f32>,
    seed: Option<u64>,
    calls: Mutex<Vec<CapturedProviderCall>>,
}

impl CapturingProvider {
    fn new(narrative: &str) -> Self {
        Self {
            completion: json!({ "narrative": narrative }).to_string(),
            model: ModelName::new(JUDGE_MODEL),
            temperature: 0.0,
            top_p: None,
            seed: None,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<CapturedProviderCall> {
        self.calls.lock().expect("provider call mutex").clone()
    }
}

#[async_trait]
impl IntelligenceProvider for CapturingProvider {
    async fn complete(
        &self,
        prompt: PromptInput,
        tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        let metadata = FingerprintMetadata {
            provider: ProviderKind::ClaudeCode,
            model: self.current_model(tier),
            temperature: self.temperature,
            top_p: self.top_p,
            seed: self.seed,
            tokens_input: None,
            tokens_output: None,
            provider_completion_id: None,
        };
        self.calls
            .lock()
            .expect("provider call mutex")
            .push(CapturedProviderCall {
                prompt,
                tier,
                model: metadata.model.as_str().to_string(),
            });
        Ok(Completion {
            text: self.completion.clone(),
            fingerprint_metadata: metadata,
        })
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::ClaudeCode
    }

    fn current_model(&self, _tier: ModelTier) -> ModelName {
        self.model.clone()
    }
}

#[tokio::test]
async fn happy_path_composed_read_llm_narrative() {
    let reader = Arc::new(FixtureClaimReader::new(vec![happy_snapshot()]));
    let provider = CapturingProvider::new(
        "Account Alpha needs security-review follow-through before today's readiness review.",
    );

    let output = invoke_daily_readiness(reader, &provider, Actor::User, WORKSPACE_ALPHA).await;

    assert_eq!(
        output.data().narrative,
        "Account Alpha needs security-review follow-through before today's readiness review."
    );
    assert_eq!(output.data().schema_version.0, 1);
    assert_eq!(output.data().meetings_today.len(), 1);
    assert_eq!(output.data().risk_shifts.len(), 1);
    assert_eq!(output.data().open_loops.len(), 1);

    let child_ids = child_composition_ids(&output);
    assert!(child_ids.contains(&"prepare_meeting:ws-alpha:meeting-alpha-readiness".to_string()));
    assert!(child_ids.contains(&"get_entity_context:ws-alpha:account:acct-alpha".to_string()));

    let call = only_call(&provider);
    assert_eq!(call.tier, ModelTier::Synthesis);
    assert_eq!(call.prompt.template_id.as_deref(), Some("daily_readiness"));
    assert_eq!(call.prompt.template_version.as_deref(), Some("1.0.0"));
}

#[tokio::test]
async fn per_child_sensitivity_double_gate_at_parent_boundary() {
    let reader = Arc::new(FixtureClaimReader::new(vec![sensitivity_gate_snapshot()]));
    let provider = CapturingProvider::new("Only the public Alpha risk is ready for synthesis.");

    let output = invoke_daily_readiness(reader, &provider, Actor::Agent, WORKSPACE_ALPHA).await;

    assert_eq!(output.data().risk_shifts.len(), 1);
    assert_eq!(output.data().risk_shifts[0].id, "risk-alpha-public");
    assert_eq!(output.data().coverage_warnings.len(), 1);
    assert_eq!(
        output.data().coverage_warnings[0].kind,
        "private_prompt_input_filtered"
    );
    assert_eq!(output.data().coverage_warnings[0].count, 1);

    let call = only_call(&provider);
    assert!(
        !call.prompt.text.contains(PRIVATE_PARENT_TEXT),
        "channel 7 rendered prompt text must not contain parent-filtered private text"
    );
    let canonical_inputs = call
        .prompt
        .canonical_json_inputs
        .expect("daily_readiness prompt has canonical JSON inputs");
    let canonical_json =
        serde_json::to_string(&canonical_inputs).expect("canonical JSON serializes");
    assert!(
        !canonical_json.contains(PRIVATE_PARENT_TEXT),
        "channel 6 canonical inputs must not contain parent-filtered private text"
    );
}

#[tokio::test]
async fn prepare_meeting_child_private_topic_detail_filtered_at_parent_boundary() {
    let reader = Arc::new(FixtureClaimReader::new(Vec::new()));
    let provider = CapturingProvider::new("Only parent-allowed child context was synthesized.");
    let input = DailyReadinessInput::evaluate_with_context(
        private_prepare_child_context(),
        SchemaVersion(1),
    );

    let output = invoke_daily_readiness_with_input(reader, &provider, Actor::Agent, input).await;

    assert!(output.data().meetings_today[0].topics.is_empty());
    assert!(output
        .data()
        .coverage_warnings
        .iter()
        .any(|warning| warning.kind == "private_prompt_input_filtered"));

    let call = only_call(&provider);
    assert!(
        !call.prompt.text.contains(PRIVATE_CHILD_TOPIC_DETAIL),
        "channel 7 rendered prompt text must not contain parent-filtered child topic detail"
    );
    let canonical_inputs = call
        .prompt
        .canonical_json_inputs
        .expect("daily_readiness prompt has canonical JSON inputs");
    let canonical_json =
        serde_json::to_string(&canonical_inputs).expect("canonical JSON serializes");
    assert!(
        !canonical_json.contains(PRIVATE_CHILD_TOPIC_DETAIL),
        "channel 6 canonical inputs must not contain parent-filtered child topic detail"
    );
}

#[tokio::test]
async fn workspace_scope_partition_rejects_cross_workspace_bleed() {
    let expected = fixture_json("bundle-9-subject-partition/expected_output.json");
    assert_eq!(expected["workspace_scope"], "ws-alpha");

    let reader = Arc::new(FixtureClaimReader::new(vec![
        partition_alpha_snapshot(),
        partition_beta_snapshot(),
    ]));
    let provider = CapturingProvider::new(
        "Account Alpha needs security-review follow-through; Workspace Beta is out of scope.",
    );

    let output = invoke_daily_readiness(reader, &provider, Actor::User, WORKSPACE_ALPHA).await;

    assert_workspace_scope(output.data(), WORKSPACE_ALPHA);
    let output_json = serde_json::to_string(output.data()).expect("output serializes");
    assert!(!output_json.contains(BETA_ONLY_TEXT));
    assert!(!output_json.contains("acct-beta"));
    assert!(child_composition_ids(&output)
        .iter()
        .all(|composition_id| !composition_id.contains(WORKSPACE_BETA)));
}

#[tokio::test]
async fn concurrent_composition_cache_key_includes_workspace_scope() {
    let expected = fixture_json("bundle-9-concurrent-composition/expected_output.json");
    assert_eq!(expected["workspace_scope"], "ws-alpha");

    let reader = Arc::new(FixtureClaimReader::new(vec![
        concurrent_snapshot(WORKSPACE_ALPHA),
        concurrent_snapshot(WORKSPACE_BETA),
    ]));
    let provider = CapturingProvider::new(
        "Account Alpha has stable readiness composition across concurrent invocations.",
    );

    let (refresh, retry, scheduled) = tokio::join!(
        invoke_daily_readiness(reader.clone(), &provider, Actor::User, WORKSPACE_ALPHA),
        invoke_daily_readiness(reader.clone(), &provider, Actor::User, WORKSPACE_ALPHA),
        invoke_daily_readiness(reader.clone(), &provider, Actor::User, WORKSPACE_ALPHA),
    );
    let beta = invoke_daily_readiness(reader, &provider, Actor::User, WORKSPACE_BETA).await;

    let refresh_ids = child_composition_ids(&refresh);
    assert_eq!(refresh_ids, child_composition_ids(&retry));
    assert_eq!(refresh_ids, child_composition_ids(&scheduled));
    assert!(refresh_ids
        .iter()
        .all(|composition_id| composition_id.contains(WORKSPACE_ALPHA)));

    let beta_ids = child_composition_ids(&beta);
    assert!(beta_ids
        .iter()
        .all(|composition_id| composition_id.contains(WORKSPACE_BETA)));
    assert_ne!(refresh_ids, beta_ids);

    let calls = provider.calls();
    assert_eq!(calls.len(), 4);
    assert!(calls.iter().all(|call| call.tier == ModelTier::Synthesis));
}

#[tokio::test]
async fn judge_model_pinned_to_claude_sonnet_4_6() {
    assert_eq!(JUDGE_MODEL, "claude-sonnet-4-6");

    let reader = Arc::new(FixtureClaimReader::new(vec![happy_snapshot()]));
    let provider = CapturingProvider::new("Judge-model metadata is pinned.");
    let output = invoke_daily_readiness(reader, &provider, Actor::System, WORKSPACE_ALPHA).await;

    let call = only_call(&provider);
    assert_eq!(call.model, JUDGE_MODEL);

    let fingerprint = output
        .provenance()
        .prompt_fingerprint
        .as_ref()
        .expect("daily_readiness emits prompt fingerprint");
    assert_eq!(fingerprint.model.0, JUDGE_MODEL);
    assert_eq!(fingerprint.provider, "claude_code");
}

async fn invoke_daily_readiness(
    reader: Arc<FixtureClaimReader>,
    provider: &CapturingProvider,
    actor: Actor,
    workspace_id: &str,
) -> AbilityOutput<DailyReadiness> {
    invoke_daily_readiness_with_input(
        reader,
        provider,
        actor,
        DailyReadinessInput::public(
            workspace_id.to_string(),
            Some(TEST_DATE.to_string()),
            SchemaVersion(1),
        ),
    )
    .await
}

async fn invoke_daily_readiness_with_input(
    reader: Arc<FixtureClaimReader>,
    provider: &CapturingProvider,
    actor: Actor,
    input: DailyReadinessInput,
) -> AbilityOutput<DailyReadiness> {
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 14, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(220);
    let daily_reader: Arc<dyn DailyReadinessContextReadHandle> = reader.clone();
    let prepare_reader: Arc<dyn PrepareMeetingContextReadHandle> = reader.clone();
    let claim_reader: Arc<dyn EntityContextClaimReadHandle> = reader;
    let services = ServiceContext::new_evaluate_default(&clock, &rng)
        .with_actor("ability-test")
        .with_daily_readiness_context_reader(daily_reader)
        .with_prepare_meeting_context_reader(prepare_reader)
        .with_entity_context_claim_reader(claim_reader);
    let ctx = AbilityContext::new(
        &services,
        provider,
        &NOOP_ABILITY_TRACER,
        actor,
        None,
        ClaimDismissalSurface::Eval,
    );

    get_daily_readiness(&ctx, input)
        .await
        .expect("get_daily_readiness succeeds")
}

fn only_call(provider: &CapturingProvider) -> CapturedProviderCall {
    let calls = provider.calls();
    assert_eq!(calls.len(), 1);
    calls.into_iter().next().expect("one provider call")
}

fn child_composition_ids(output: &AbilityOutput<DailyReadiness>) -> Vec<String> {
    output
        .provenance()
        .children
        .iter()
        .map(|child| child.composition_id.as_str().to_string())
        .collect()
}

fn assert_workspace_scope(data: &DailyReadiness, workspace_scope: &str) {
    assert!(data
        .meetings_today
        .iter()
        .all(|meeting| meeting.workspace_scope == workspace_scope));
    assert!(data
        .overnight_changes
        .iter()
        .all(|change| change.workspace_scope == workspace_scope));
    assert!(data
        .risk_shifts
        .iter()
        .all(|risk| risk.workspace_scope == workspace_scope));
    assert!(data
        .open_loops
        .iter()
        .all(|open_loop| open_loop.workspace_scope == workspace_scope));
    assert!(data
        .coverage_warnings
        .iter()
        .all(|warning| warning.workspace_scope == workspace_scope));
}

fn fixture_json(relative_path: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative_path);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

fn happy_snapshot() -> DailyReadinessContextSnapshot {
    let subject = subject(WORKSPACE_ALPHA, "acct-alpha", "Account Alpha");
    DailyReadinessContextSnapshot {
        workspace_scope: WORKSPACE_ALPHA.to_string(),
        date: TEST_DATE.to_string(),
        meetings: vec![meeting(
            WORKSPACE_ALPHA,
            "meeting-alpha-readiness",
            "Account Alpha readiness review",
        )],
        tracked_subjects: vec![subject.clone()],
        overnight_changes: vec![signal(
            "change-alpha-sponsor-delay",
            subject.clone(),
            "Account Alpha sponsor delayed the security review overnight.",
            WORKSPACE_ALPHA,
        )],
        risk_shifts: vec![risk(
            "risk-alpha-renewal",
            subject.clone(),
            "up",
            "Account Alpha renewal risk increased after the sponsor missed yesterday's security review.",
            "internal",
            WORKSPACE_ALPHA,
        )],
        open_loops: vec![open_loop_snapshot(
            "loop-alpha-security-review",
            subject,
            "Confirm Account Alpha security reviewer before the 14:00 readiness call.",
            WORKSPACE_ALPHA,
        )],
        coverage_warnings: Vec::new(),
    }
}

fn sensitivity_gate_snapshot() -> DailyReadinessContextSnapshot {
    let subject = subject(WORKSPACE_ALPHA, "acct-alpha", "Account Alpha");
    DailyReadinessContextSnapshot {
        workspace_scope: WORKSPACE_ALPHA.to_string(),
        date: TEST_DATE.to_string(),
        meetings: Vec::new(),
        tracked_subjects: vec![subject.clone()],
        overnight_changes: Vec::new(),
        risk_shifts: vec![
            risk(
                "risk-alpha-public",
                subject.clone(),
                "up",
                "Account Alpha public risk increased after support queue growth.",
                "internal",
                WORKSPACE_ALPHA,
            ),
            risk(
                "risk-alpha-private",
                subject,
                "up",
                PRIVATE_PARENT_TEXT,
                "confidential",
                WORKSPACE_ALPHA,
            ),
        ],
        open_loops: Vec::new(),
        coverage_warnings: Vec::new(),
    }
}

fn private_prepare_child_context() -> DailyReadinessContext {
    let meeting = DailyReadinessMeetingSeed {
        id: "meeting-alpha-private-topic".to_string(),
        title: "Account Alpha sensitive prep".to_string(),
        starts_at: Some("2026-05-14T14:00:00Z".to_string()),
        ends_at: Some("2026-05-14T14:30:00Z".to_string()),
        workspace_scope: WORKSPACE_ALPHA.to_string(),
    };
    let topic = Topic {
        title: "Sensitive renewal blocker".to_string(),
        detail: PRIVATE_CHILD_TOPIC_DETAIL.to_string(),
        subject: BriefSubjectRef {
            kind: "meeting".to_string(),
            id: meeting.id.clone(),
        },
        temporal_scope: BriefTemporalScope::State,
    };
    let child = ComposedPrepareMeetingOutput {
        meeting_id: meeting.id.clone(),
        workspace_scope: WORKSPACE_ALPHA.to_string(),
        cache_dedupe_key: format!("prepare_meeting:{}:{}", WORKSPACE_ALPHA, meeting.id),
        sensitivity: "confidential".to_string(),
        output: MeetingBrief {
            meeting: MeetingSummary {
                id: meeting.id.clone(),
                title: meeting.title.clone(),
                starts_at: meeting.starts_at.clone(),
                ends_at: meeting.ends_at.clone(),
                attendees: Vec::new(),
            },
            topics: vec![topic],
            attendee_context: Vec::new(),
            open_loops: Vec::new(),
            what_changed_since_last: Vec::new(),
            suggested_outcomes: Vec::new(),
            schema_version: SchemaVersion(1),
        },
    };

    DailyReadinessContext {
        workspace_scope: WORKSPACE_ALPHA.to_string(),
        date: TEST_DATE.to_string(),
        meetings: vec![meeting],
        tracked_subjects: Vec::new(),
        prepare_meeting_children: vec![child],
        entity_context_children: Vec::new(),
        overnight_changes: Vec::new(),
        risk_shifts: Vec::new(),
        open_loops: Vec::new(),
        coverage_warnings: Vec::new(),
    }
}

fn partition_alpha_snapshot() -> DailyReadinessContextSnapshot {
    happy_snapshot()
}

fn partition_beta_snapshot() -> DailyReadinessContextSnapshot {
    let subject = subject(WORKSPACE_BETA, "acct-beta", "Account Beta");
    DailyReadinessContextSnapshot {
        workspace_scope: WORKSPACE_BETA.to_string(),
        date: TEST_DATE.to_string(),
        meetings: vec![meeting(
            WORKSPACE_BETA,
            "meeting-beta-expansion",
            "Account Beta expansion review",
        )],
        tracked_subjects: vec![subject.clone()],
        overnight_changes: Vec::new(),
        risk_shifts: vec![risk(
            "risk-beta-expansion",
            subject,
            "up",
            BETA_ONLY_TEXT,
            "internal",
            WORKSPACE_BETA,
        )],
        open_loops: Vec::new(),
        coverage_warnings: Vec::new(),
    }
}

fn concurrent_snapshot(workspace_scope: &str) -> DailyReadinessContextSnapshot {
    let meeting_id = if workspace_scope == WORKSPACE_ALPHA {
        "meeting-alpha-readiness"
    } else {
        "meeting-beta-readiness"
    };
    let display_name = if workspace_scope == WORKSPACE_ALPHA {
        "Shared Alpha Account"
    } else {
        "Shared Beta Account"
    };
    let subject = subject(workspace_scope, "acct-shared", display_name);
    DailyReadinessContextSnapshot {
        workspace_scope: workspace_scope.to_string(),
        date: TEST_DATE.to_string(),
        meetings: vec![meeting(workspace_scope, meeting_id, "Readiness review")],
        tracked_subjects: vec![subject.clone()],
        overnight_changes: Vec::new(),
        risk_shifts: Vec::new(),
        open_loops: vec![open_loop_snapshot(
            &format!("loop-{workspace_scope}-follow-up"),
            subject,
            &format!("{workspace_scope} follow-up remains open."),
            workspace_scope,
        )],
        coverage_warnings: Vec::new(),
    }
}

fn subject(workspace_scope: &str, id: &str, display_name: &str) -> DailyReadinessSubjectSnapshot {
    DailyReadinessSubjectSnapshot {
        kind: "account".to_string(),
        id: id.to_string(),
        display_name: display_name.to_string(),
        workspace_scope: workspace_scope.to_string(),
    }
}

fn meeting(workspace_scope: &str, id: &str, title: &str) -> DailyReadinessMeetingSnapshot {
    DailyReadinessMeetingSnapshot {
        id: id.to_string(),
        title: title.to_string(),
        starts_at: Some("2026-05-14T14:00:00Z".to_string()),
        ends_at: Some("2026-05-14T14:30:00Z".to_string()),
        workspace_scope: workspace_scope.to_string(),
    }
}

fn signal(
    id: &str,
    subject: DailyReadinessSubjectSnapshot,
    summary: &str,
    workspace_scope: &str,
) -> DailyReadinessSignalSnapshot {
    DailyReadinessSignalSnapshot {
        id: id.to_string(),
        subject,
        summary: summary.to_string(),
        source_ref: Some(format!("src-{id}")),
        observed_at: "2026-05-14T09:00:00Z".to_string(),
        source_asof: Some("2026-05-14T09:00:00Z".to_string()),
        data_source: "user".to_string(),
        lifecycle: "active".to_string(),
        confidence: 0.91,
        sensitivity: "internal".to_string(),
        workspace_scope: workspace_scope.to_string(),
    }
}

fn risk(
    id: &str,
    subject: DailyReadinessSubjectSnapshot,
    direction: &str,
    evidence_summary: &str,
    sensitivity: &str,
    workspace_scope: &str,
) -> DailyReadinessRiskSnapshot {
    DailyReadinessRiskSnapshot {
        id: id.to_string(),
        subject,
        direction: direction.to_string(),
        evidence_summary: evidence_summary.to_string(),
        source_ref: Some(format!("src-{id}")),
        observed_at: "2026-05-14T10:00:00Z".to_string(),
        source_asof: Some("2026-05-14T10:00:00Z".to_string()),
        data_source: "user".to_string(),
        lifecycle: "active".to_string(),
        confidence: 0.87,
        sensitivity: sensitivity.to_string(),
        workspace_scope: workspace_scope.to_string(),
    }
}

fn open_loop_snapshot(
    id: &str,
    subject: DailyReadinessSubjectSnapshot,
    text: &str,
    workspace_scope: &str,
) -> DailyReadinessOpenLoopSnapshot {
    DailyReadinessOpenLoopSnapshot {
        id: id.to_string(),
        text: text.to_string(),
        owner: Some("owner@example.invalid".to_string()),
        subject,
        due_date: Some(TEST_DATE.to_string()),
        source_ref: Some(format!("src-{id}")),
        observed_at: "2026-05-14T11:00:00Z".to_string(),
        source_asof: Some("2026-05-14T11:00:00Z".to_string()),
        data_source: "user".to_string(),
        lifecycle: "active".to_string(),
        confidence: 0.9,
        sensitivity: "internal".to_string(),
        workspace_scope: workspace_scope.to_string(),
    }
}

fn prepare_snapshot(meeting: &DailyReadinessMeetingSnapshot) -> PrepareMeetingContextSnapshot {
    PrepareMeetingContextSnapshot {
        meeting: PrepareMeetingSnapshot {
            id: meeting.id.clone(),
            title: meeting.title.clone(),
            starts_at: meeting.starts_at.clone(),
            ends_at: meeting.ends_at.clone(),
            attendees_raw: None,
        },
        attendees: vec![PrepareMeetingAttendeeSnapshot {
            name: "Example Attendee".to_string(),
            email: Some("attendee@example.invalid".to_string()),
            person_id: Some(format!("person-{}", meeting.id)),
            account_id: None,
            domain: Some("example.invalid".to_string()),
        }],
        subjects: vec![PrepareMeetingSubjectSnapshot {
            kind: "meeting".to_string(),
            id: meeting.id.clone(),
            display_name: meeting.title.clone(),
        }],
        claims: Vec::new(),
    }
}

fn fixture_claim(
    id: &str,
    subject: &DailyReadinessSubjectSnapshot,
    text: &str,
    sensitivity: ClaimSensitivity,
) -> IntelligenceClaim {
    IntelligenceClaim {
        id: id.to_string(),
        claim_version: 1,
        subject_ref: json!({
            "kind": subject.kind,
            "id": subject.id,
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
        source_asof: Some("2026-05-14T09:00:00Z".to_string()),
        observed_at: "2026-05-14T09:00:00Z".to_string(),
        created_at: "2026-05-14T09:00:00Z".to_string(),
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

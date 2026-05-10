#![cfg(feature = "release-gate")]

#[path = "harness/mod.rs"]
mod harness;

use std::sync::Mutex;

use async_trait::async_trait;
use dailyos_lib::abilities::prepare_meeting::{prepare_meeting, MeetingBrief, PrepareMeetingInput};
use dailyos_lib::abilities::{AbilityContext, AbilityOutput, Actor, NOOP_ABILITY_TRACER};
use dailyos_lib::intelligence::provider::{
    Completion, FingerprintMetadata, IntelligenceProvider, ModelName, ModelTier, PromptInput,
    ProviderError, ProviderKind,
};
use dailyos_lib::services::context::ClaimDismissalSurface;
use rusqlite::Connection;
use serde_json::Value;

use harness::{
    bundle_helpers::{bundle_fixture_path, refresh_prepare_meeting_context_from_db},
    load_fixture, prepare_fixture_for_run,
};

const BUNDLE: u32 = 5;
const MEETING_ID: &str = "meeting-b5-first-person";
const EXPIRED_DORMANT_OPEN_LOOP: &str = "src-b5-expired-dormant-open-loop";
const WRONG_SUBJECT_TOMBSTONED: &str = "src-b5-wrong-attendee-original";
const SUPERSEDED_ORIGINAL: &str = "src-b5-original-preference";
const USER_EDITED_SUPERSESSION: &str = "src-b5-user-edited-preference";
const DUPLICATE_CANONICAL: &str = "src-b5-agenda-dup-canonical";
const DUPLICATE_CORROBORATION: &str = "corro-b5-agenda-paraphrase";

const EXPIRED_DORMANT_TEXT: &str = "Send Riley the old onboarding checklist before the call.";
const WRONG_SUBJECT_TEXT: &str = "Riley Rivera owns the renewal approval for Example Portfolio.";
const SUPERSEDED_TEXT: &str = "Riley prefers a broad discovery agenda.";
const USER_EDITED_TEXT: &str =
    "Riley Rivera asked to start with a written agenda and confirm next ownership.";

#[test]
fn bundle5_double_refresh_does_not_resurrect_dormant_or_corrected_claims() {
    let fixture = load_fixture(&bundle_fixture_path(BUNDLE)).expect("bundle-5 fixture loads");
    let mut prepared = prepare_fixture_for_run(&fixture).expect("bundle-5 fixture prepares");
    let input: PrepareMeetingInput =
        serde_json::from_value(fixture.inputs_json["input_json"].clone())
            .expect("bundle-5 input parses");
    let completion = fixture.provider_replay["fixtures"][0]["completion"]
        .as_str()
        .expect("bundle-5 completion text")
        .to_string();
    let provider = PromptCaptureProvider::new(completion);
    let expected_prompt_hash = fixture.provider_replay["fixtures"][0]["canonical_prompt_hash"]
        .as_str()
        .expect("bundle-5 canonical prompt hash");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    refresh_prepare_meeting_context_from_db(&mut prepared, MEETING_ID)
        .expect("first prepare_meeting context refresh succeeds");
    let first = {
        let services = prepared.service_context();
        let ctx = AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::User,
            None,
            ClaimDismissalSurface::Eval,
        );
        let output = runtime
            .block_on(prepare_meeting(&ctx, input.clone()))
            .expect("first prepare_meeting refresh succeeds");
        refresh_run_from_output(output, &provider)
    };
    assert_refresh_contract(
        &prepared.conn,
        &first,
        &fixture.expected.output,
        expected_prompt_hash,
        "first refresh",
    );

    refresh_prepare_meeting_context_from_db(&mut prepared, MEETING_ID)
        .expect("second prepare_meeting context refresh succeeds");
    let second = {
        let services = prepared.service_context();
        let ctx = AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::User,
            None,
            ClaimDismissalSurface::Eval,
        );
        let output = runtime
            .block_on(prepare_meeting(&ctx, input))
            .expect("second prepare_meeting refresh succeeds");
        refresh_run_from_output(output, &provider)
    };
    assert_refresh_contract(
        &prepared.conn,
        &second,
        &fixture.expected.output,
        expected_prompt_hash,
        "second refresh",
    );

    assert_eq!(
        second.output, first.output,
        "second refresh output must stay byte-equivalent at the JSON layer"
    );
    assert_eq!(
        second.evidence_ids, first.evidence_ids,
        "second refresh prompt evidence ids must not drift"
    );

    let captured = provider.captured_prompts();
    assert_eq!(
        captured.len(),
        2,
        "test must exercise two prepare_meeting provider calls"
    );
}

struct RefreshRun {
    output: Value,
    prompt_hash: String,
    evidence_ids: Vec<String>,
    prompt_inputs: Value,
}

fn refresh_run_from_output(
    output: AbilityOutput<MeetingBrief>,
    provider: &PromptCaptureProvider,
) -> RefreshRun {
    let output_value = serde_json::to_value(output.data()).expect("MeetingBrief serializes");
    let prompt_hash = output
        .provenance()
        .prompt_fingerprint
        .as_ref()
        .expect("prepare_meeting emits prompt fingerprint")
        .canonical_prompt_hash
        .0
        .clone();
    let prompt_inputs = provider
        .last_prompt()
        .canonical_json_inputs
        .expect("prepare_meeting prompt has canonical JSON inputs");
    let evidence_ids = evidence(&prompt_inputs)
        .iter()
        .map(|source| {
            source["id"]
                .as_str()
                .expect("prompt evidence source id")
                .to_string()
        })
        .collect();

    RefreshRun {
        output: output_value,
        prompt_hash,
        evidence_ids,
        prompt_inputs,
    }
}

fn assert_refresh_contract(
    conn: &Connection,
    run: &RefreshRun,
    expected_output: &Value,
    expected_prompt_hash: &str,
    label: &str,
) {
    assert_eq!(
        &run.output, expected_output,
        "{label}: rendered MeetingBrief must match bundle-5 expected output"
    );
    assert_eq!(
        run.prompt_hash, expected_prompt_hash,
        "{label}: prompt hash must remain pinned to the bundle-5 replay key"
    );

    assert_lifecycle(
        conn,
        EXPIRED_DORMANT_OPEN_LOOP,
        "dormant",
        "dormant",
        None,
        label,
    );
    assert_lifecycle(
        conn,
        WRONG_SUBJECT_TOMBSTONED,
        "tombstoned",
        "dormant",
        None,
        label,
    );
    assert_lifecycle(
        conn,
        SUPERSEDED_ORIGINAL,
        "dormant",
        "dormant",
        Some(USER_EDITED_SUPERSESSION),
        label,
    );
    assert_lifecycle(
        conn,
        USER_EDITED_SUPERSESSION,
        "active",
        "active",
        None,
        label,
    );

    let prompt_evidence = evidence(&run.prompt_inputs);
    assert_source_present_once(prompt_evidence, USER_EDITED_SUPERSESSION, label);
    assert_source_present_once(prompt_evidence, DUPLICATE_CANONICAL, label);
    assert_source_absent(prompt_evidence, EXPIRED_DORMANT_OPEN_LOOP, label);
    assert_source_absent(prompt_evidence, WRONG_SUBJECT_TOMBSTONED, label);
    assert_source_absent(prompt_evidence, SUPERSEDED_ORIGINAL, label);
    assert_source_absent(prompt_evidence, DUPLICATE_CORROBORATION, label);

    let output_text = serde_json::to_string(&run.output).expect("output serializes");
    let prompt_text = serde_json::to_string(&run.prompt_inputs).expect("prompt inputs serialize");
    assert!(
        output_text.contains(USER_EDITED_TEXT),
        "{label}: user-edited supersession must render"
    );
    assert!(
        prompt_text.contains(USER_EDITED_TEXT),
        "{label}: user-edited supersession must enter prompt evidence"
    );
    for forbidden in [
        EXPIRED_DORMANT_OPEN_LOOP,
        EXPIRED_DORMANT_TEXT,
        WRONG_SUBJECT_TOMBSTONED,
        WRONG_SUBJECT_TEXT,
        SUPERSEDED_ORIGINAL,
        SUPERSEDED_TEXT,
        DUPLICATE_CORROBORATION,
    ] {
        assert!(
            !prompt_text.contains(forbidden),
            "{label}: `{forbidden}` must be absent from prompt evidence"
        );
        assert!(
            !output_text.contains(forbidden),
            "{label}: `{forbidden}` must be absent from rendered output"
        );
    }

    let corroboration_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM claim_corroborations
             WHERE id = ?1
               AND claim_id = ?2
               AND source_mechanism = 'paraphrase_duplicate'",
            [DUPLICATE_CORROBORATION, DUPLICATE_CANONICAL],
            |row| row.get(0),
        )
        .expect("read duplicate paraphrase corroboration");
    assert_eq!(
        corroboration_count, 1,
        "{label}: duplicate paraphrase pair remains collapsed through claim_corroborations"
    );
}

fn evidence(prompt_inputs: &Value) -> &[Value] {
    prompt_inputs
        .pointer("/context/evidence")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .expect("canonical prompt context evidence array")
}

fn assert_source_present_once(evidence: &[Value], source_id: &str, label: &str) {
    let count = evidence
        .iter()
        .filter(|source| source["id"].as_str() == Some(source_id))
        .count();
    assert_eq!(
        count, 1,
        "{label}: `{source_id}` must appear exactly once in prompt evidence"
    );
}

fn assert_source_absent(evidence: &[Value], source_id: &str, label: &str) {
    assert!(
        evidence
            .iter()
            .all(|source| source["id"].as_str() != Some(source_id)),
        "{label}: `{source_id}` must be absent from prompt evidence"
    );
}

fn assert_lifecycle(
    conn: &Connection,
    claim_id: &str,
    expected_claim_state: &str,
    expected_surfacing_state: &str,
    expected_superseded_by: Option<&str>,
    label: &str,
) {
    let (claim_state, surfacing_state, superseded_by): (String, String, Option<String>) = conn
        .query_row(
            "SELECT claim_state, surfacing_state, superseded_by
             FROM intelligence_claims
             WHERE id = ?1",
            [claim_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap_or_else(|error| panic!("{label}: read lifecycle for `{claim_id}`: {error}"));

    assert_eq!(
        claim_state, expected_claim_state,
        "{label}: `{claim_id}` claim_state"
    );
    assert_eq!(
        surfacing_state, expected_surfacing_state,
        "{label}: `{claim_id}` surfacing_state"
    );
    assert_eq!(
        superseded_by.as_deref(),
        expected_superseded_by,
        "{label}: `{claim_id}` superseded_by"
    );
}

struct PromptCaptureProvider {
    completion: String,
    prompts: Mutex<Vec<PromptInput>>,
}

impl PromptCaptureProvider {
    fn new(completion: String) -> Self {
        Self {
            completion,
            prompts: Mutex::new(Vec::new()),
        }
    }

    fn last_prompt(&self) -> PromptInput {
        self.prompts
            .lock()
            .expect("prompt capture mutex")
            .last()
            .cloned()
            .expect("provider captured a prompt")
    }

    fn captured_prompts(&self) -> Vec<PromptInput> {
        self.prompts.lock().expect("prompt capture mutex").clone()
    }
}

#[async_trait]
impl IntelligenceProvider for PromptCaptureProvider {
    async fn complete(
        &self,
        prompt: PromptInput,
        _tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        self.prompts
            .lock()
            .expect("prompt capture mutex")
            .push(prompt);
        Ok(Completion {
            text: self.completion.clone(),
            fingerprint_metadata: FingerprintMetadata::default(),
        })
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Other("prompt_capture")
    }

    fn current_model(&self, _tier: ModelTier) -> ModelName {
        ModelName::new("prompt-capture")
    }
}

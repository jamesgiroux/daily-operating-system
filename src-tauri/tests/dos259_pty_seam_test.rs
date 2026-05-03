//!  exercise `PtyClaudeCode::complete_blocking`
//! and `complete()` end-to-end through the `PtySpawnAdapter` test seam.
//!
//! The original parity tests used `ReplayProvider` only and never invoked
//! the migrated PTY path, so a broken prompt/workspace/tier propagation
//! would still pass. These tests inject a `FakePtySpawnAdapter` that
//! captures the args `PtyClaudeCode` actually feeds to the spawn step
//! and asserts they match the expected propagation contract.

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;

use dailyos_lib::intelligence::provider::{
    Completion, IntelligenceProvider, ModelTier, ProviderKind, PromptInput,
};
use dailyos_lib::intelligence::pty_provider::{
    PtyClaudeCode, PtySpawnAdapter, PtySpawnRequest,
};
use dailyos_lib::pty::{AiUsageContext, ClaudeOutput};
use dailyos_lib::types::AiModelConfig;

#[derive(Debug, Clone)]
struct CapturedSpawn {
    workspace: PathBuf,
    prompt: String,
    tier: ModelTier,
    model_for_tier: String,
    timeout_secs: u64,
    nice_priority: i32,
    usage_subsystem: String,
    usage_tier: String,
}

struct FakePtySpawnAdapter {
    captured: Mutex<Vec<CapturedSpawn>>,
    fixture_stdout: String,
}

impl FakePtySpawnAdapter {
    fn new(fixture_stdout: impl Into<String>) -> Self {
        Self {
            captured: Mutex::new(Vec::new()),
            fixture_stdout: fixture_stdout.into(),
        }
    }
    fn captured(&self) -> Vec<CapturedSpawn> {
        self.captured.lock().clone()
    }
}

impl PtySpawnAdapter for FakePtySpawnAdapter {
    fn spawn_claude(&self, req: PtySpawnRequest<'_>) -> Result<ClaudeOutput, String> {
        let model = match req.tier {
            ModelTier::Synthesis => &req.ai_config.synthesis,
            ModelTier::Extraction => &req.ai_config.extraction,
            ModelTier::Background => &req.ai_config.background,
            ModelTier::Mechanical => &req.ai_config.mechanical,
        };
        self.captured.lock().push(CapturedSpawn {
            workspace: req.workspace.to_path_buf(),
            prompt: req.prompt.to_string(),
            tier: req.tier,
            model_for_tier: model.clone(),
            timeout_secs: req.timeout_secs,
            nice_priority: req.nice_priority,
            usage_subsystem: req.usage_context.subsystem.clone(),
            usage_tier: req.usage_context.tier.clone(),
        });
        Ok(ClaudeOutput {
            stdout: self.fixture_stdout.clone(),
            exit_code: 0,
        })
    }
}

fn fixture_ai_config() -> AiModelConfig {
    AiModelConfig {
        synthesis: "syn-test".into(),
        extraction: "ext-test".into(),
        background: "bg-test".into(),
        mechanical: "mech-test".into(),
    }
}

#[test]
fn complete_blocking_propagates_prompt_workspace_tier_and_model() {
    let fake = Arc::new(FakePtySpawnAdapter::new("FIXTURE-STDOUT"));
    let provider = PtyClaudeCode::with_spawn_adapter(
        Arc::new(fixture_ai_config()),
        std::env::temp_dir(),
        AiUsageContext::new("dos259_seam_test", "complete_blocking"),
        fake.clone() as Arc<dyn PtySpawnAdapter>,
    );

    let workspace = std::env::temp_dir().join("dos259-seam-workspace");
    let prompt = PromptInput::new("inject-this-prompt").with_workspace(workspace.clone());

    let completion = provider
        .complete_blocking(prompt, ModelTier::Extraction)
        .expect("fake adapter returns canned stdout");

    // Stdout flows through to Completion.text byte-identically.
    assert_eq!(completion.text, "FIXTURE-STDOUT");

    // Fingerprint metadata reflects the requested tier's model.
    assert_eq!(completion.fingerprint_metadata.provider, ProviderKind::ClaudeCode);
    assert_eq!(completion.fingerprint_metadata.model.as_str(), "ext-test");
    assert_eq!(completion.fingerprint_metadata.temperature, 1.0);

    // The fake captured exactly one call with the expected propagation:
    let calls = fake.captured();
    assert_eq!(calls.len(), 1, "complete_blocking should call spawn once");
    let c = &calls[0];
    assert_eq!(c.prompt, "inject-this-prompt");
    assert_eq!(c.workspace, workspace);
    assert_eq!(c.tier, ModelTier::Extraction);
    assert_eq!(c.model_for_tier, "ext-test");
    assert_eq!(
        c.timeout_secs, 240,
        "Extraction tier must use the documented 240s timeout"
    );
    assert_eq!(c.nice_priority, 10);
    assert_eq!(c.usage_subsystem, "dos259_seam_test");
    assert_eq!(c.usage_tier, "extraction", "usage_context.tier overlaid by tier param");
}

#[tokio::test]
async fn complete_async_uses_same_adapter_path() {
    // The async `complete()` (trait surface) must route through the same
    // adapter as `complete_blocking` — so injecting a fake exercises both
    // call shapes consistently.
    let fake = Arc::new(FakePtySpawnAdapter::new("ASYNC-FIXTURE"));
    let provider = PtyClaudeCode::with_spawn_adapter(
        Arc::new(fixture_ai_config()),
        std::env::temp_dir(),
        AiUsageContext::new("dos259_seam_test", "complete_async"),
        fake.clone() as Arc<dyn PtySpawnAdapter>,
    );

    let prompt = PromptInput::new("async-prompt");
    let completion: Completion = provider
        .complete(prompt, ModelTier::Synthesis)
        .await
        .expect("async complete returns canned stdout");
    assert_eq!(completion.text, "ASYNC-FIXTURE");
    assert_eq!(
        completion.fingerprint_metadata.model.as_str(),
        "syn-test"
    );

    let calls = fake.captured();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].tier, ModelTier::Synthesis);
    assert_eq!(calls[0].timeout_secs, 240);
}

#[test]
fn complete_blocking_maps_auth_failures_to_permanent_error() {
    struct ErrorAdapter(String);
    impl PtySpawnAdapter for ErrorAdapter {
        fn spawn_claude(&self, _: PtySpawnRequest<'_>) -> Result<ClaudeOutput, String> {
            Err(self.0.clone())
        }
    }

    let provider = PtyClaudeCode::with_spawn_adapter(
        Arc::new(fixture_ai_config()),
        std::env::temp_dir(),
        AiUsageContext::new("test", "auth_mapping"),
        Arc::new(ErrorAdapter(
            "ClaudeCodeNotFound: claude binary missing".to_string(),
        )),
    );
    let prompt = PromptInput::new("p");
    let err = provider
        .complete_blocking(prompt, ModelTier::Synthesis)
        .expect_err("auth failure must be Permanent");
    match err {
        dailyos_lib::intelligence::provider::ProviderError::Permanent(_) => (),
        other => panic!("expected Permanent, got {other:?}"),
    }
}

#[test]
fn complete_blocking_maps_other_failures_to_transient_error() {
    struct ErrorAdapter;
    impl PtySpawnAdapter for ErrorAdapter {
        fn spawn_claude(&self, _: PtySpawnRequest<'_>) -> Result<ClaudeOutput, String> {
            Err("network unreachable".to_string())
        }
    }

    let provider = PtyClaudeCode::with_spawn_adapter(
        Arc::new(fixture_ai_config()),
        std::env::temp_dir(),
        AiUsageContext::new("test", "transient_mapping"),
        Arc::new(ErrorAdapter),
    );
    let prompt = PromptInput::new("p");
    let err = provider
        .complete_blocking(prompt, ModelTier::Synthesis)
        .expect_err("network error must be Transient");
    match err {
        dailyos_lib::intelligence::provider::ProviderError::Transient(_) => (),
        other => panic!("expected Transient, got {other:?}"),
    }
}

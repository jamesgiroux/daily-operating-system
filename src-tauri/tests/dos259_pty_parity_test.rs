//! PTY provider byte-identical parity and call-configuration parity integration test.
//!
//! The provider refactor relocates provider construction from inline
//! `PtyManager::for_tier(...)` to `PtyClaudeCode::new(...)` + the
//! `IntelligenceProvider` trait. Two parity invariants must hold:
//!
//! 1. **Parsed-output byte-identity** — `parse_intelligence_response`
//!    must produce the same `IntelligenceJson` whether the completion
//!    text comes through the trait surface or the legacy direct stdout.
//!
//! 2. **Call-configuration parity** — `complete_blocking` and
//!    `complete()` must invoke the underlying PTY with identical
//!    timeout / `usage_context` / tier / nice_priority values, and
//!    those values must match the pre-refactor inline calls.
//!
//! When a real fixture corpus lands, this test gains captured PTY output
//! as the canned text; until then the synthetic JSON-shaped fixture covers
//! the parser's happy-path through the provider trait surface.

use std::sync::{Arc, Mutex};

use dailyos_lib::intelligence::io::SourceManifestEntry;
use dailyos_lib::intelligence::prompts::parse_intelligence_response;
use dailyos_lib::intelligence::provider::{
    IntelligenceProvider, PromptInput, ReplayProvider,
};
use dailyos_lib::intelligence::pty_provider::{
    PtyClaudeCode, PtySpawnAdapter, PtySpawnRequest,
};
use dailyos_lib::pty::{AiUsageContext, ClaudeOutput, ModelTier};
use dailyos_lib::types::AiModelConfig;

const FIXTURE_PROMPT: &str = "synthesize intelligence for entity-1";

const FIXTURE_PTY_STDOUT: &str = r#"{
  "executiveAssessment": "stable account",
  "risks": [],
  "recentWins": [],
  "stakeholderInsights": [],
  "valueDelivered": [],
  "competitiveContext": [],
  "marketContext": [],
  "organizationalChanges": [],
  "expansionSignals": [],
  "strategicPriorities": [],
  "internalTeam": [],
  "blockers": [],
  "gongCallSummaries": [],
  "userEdits": [],
  "dismissedItems": [],
  "domains": [],
  "sourceManifest": []
}"#;

/// Captured invocation config — what the production `PtyManager`
/// builder chain would have been called with for one
/// `complete()` / `complete_blocking()` call.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CapturedCall {
    tier: ModelTier,
    timeout_secs: u64,
    nice_priority: i32,
    usage_label: String,
}

/// `PtySpawnAdapter` that records the spawn config and returns a
/// canned stdout, so parity tests can assert on the actual values
/// `complete_blocking` / `complete` would have passed to `PtyManager`.
struct FakePtySpawnAdapter {
    canned_stdout: String,
    captured: Arc<Mutex<Vec<CapturedCall>>>,
}

impl FakePtySpawnAdapter {
    fn new(canned_stdout: &str) -> (Self, Arc<Mutex<Vec<CapturedCall>>>) {
        let captured = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                canned_stdout: canned_stdout.to_string(),
                captured: Arc::clone(&captured),
            },
            captured,
        )
    }
}

impl PtySpawnAdapter for FakePtySpawnAdapter {
    fn spawn_claude(&self, req: PtySpawnRequest<'_>) -> Result<ClaudeOutput, String> {
        self.captured.lock().unwrap().push(CapturedCall {
            tier: req.tier,
            timeout_secs: req.timeout_secs,
            nice_priority: req.nice_priority,
            usage_label: format!("{:?}", req.usage_context),
        });
        Ok(ClaudeOutput {
            stdout: self.canned_stdout.clone(),
            exit_code: 0,
        })
    }
}

#[tokio::test]
async fn pty_provider_parity_fixture_intelligence_json_byte_identical() {
    // Original parity invariant: the trait surface does not perturb
    // the parsed output byte-shape vs. the direct-stdout path.
    let provider =
        ReplayProvider::from_prompt_pairs([(FIXTURE_PROMPT, FIXTURE_PTY_STDOUT)]);
    let prompt = PromptInput::new(FIXTURE_PROMPT);
    let completion = provider
        .complete(prompt, ModelTier::Synthesis)
        .await
        .expect("replay returns canned text");

    let manifest: Vec<SourceManifestEntry> = vec![];
    let mut parsed_via_provider = parse_intelligence_response(
        &completion.text,
        "entity-1",
        "account",
        0,
        manifest.clone(),
    )
    .expect("parse via provider path");
    let mut parsed_direct = parse_intelligence_response(
        FIXTURE_PTY_STDOUT,
        "entity-1",
        "account",
        0,
        manifest,
    )
    .expect("parse via direct path");

    let pin = "2026-04-29T00:00:00+00:00".to_string();
    parsed_via_provider.enriched_at = pin.clone();
    parsed_direct.enriched_at = pin;

    let bytes_via_provider =
        serde_json::to_vec(&parsed_via_provider).expect("serialize provider parse");
    let bytes_direct = serde_json::to_vec(&parsed_direct).expect("serialize direct parse");
    assert_eq!(
        bytes_via_provider, bytes_direct,
        "PTY trait abstraction must produce byte-identical IntelligenceJson \
         output vs. the legacy direct-stdout path for the same fixture"
    );
}

/// `complete_blocking` invokes the PTY
/// adapter with the documented per-tier timeout, the configured
/// `usage_context` (with tier appended), the requested tier, and
/// `nice_priority=10` — the same values the pre-refactor inline
/// `PtyManager::for_tier(...).with_usage_context(...).with_timeout(...)
/// .with_nice_priority(10)` builder chain produced.
#[test]
fn pty_provider_complete_blocking_propagates_call_config() {
    let (adapter, captured) = FakePtySpawnAdapter::new(FIXTURE_PTY_STDOUT);
    let ai_config = Arc::new(AiModelConfig {
        synthesis: "syn-model".into(),
        extraction: "ext-model".into(),
        background: "bg-model".into(),
        mechanical: "mech-model".into(),
    });
    let provider = PtyClaudeCode::with_spawn_adapter(
        ai_config,
        std::env::temp_dir(),
        AiUsageContext::new("test", "complete_blocking_parity"),
        Arc::new(adapter),
    );

    // Synthesis tier — 240s timeout, nice=10, usage label includes
    // the configured context AND the resolved tier.
    let _ = provider
        .complete_blocking(PromptInput::new("p"), ModelTier::Synthesis)
        .expect("complete_blocking returns");

    let calls = captured.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].tier, ModelTier::Synthesis);
    assert_eq!(calls[0].timeout_secs, 240, "Synthesis tier must use 240s timeout");
    assert_eq!(calls[0].nice_priority, 10, "nice_priority must be 10 (legacy value)");
    assert!(
        calls[0].usage_label.contains("complete_blocking_parity"),
        "usage_context must propagate the caller-supplied label, got: {}",
        calls[0].usage_label
    );
}

/// Per-tier timeout matrix: assert
/// each tier maps to its documented timeout. Regression catches a
/// drift where someone changes one tier's timeout without updating
/// the constant.
#[tokio::test]
async fn pty_provider_complete_async_propagates_per_tier_timeouts() {
    let (adapter, captured) = FakePtySpawnAdapter::new(FIXTURE_PTY_STDOUT);
    let ai_config = Arc::new(AiModelConfig::default());
    let provider = PtyClaudeCode::with_spawn_adapter(
        ai_config,
        std::env::temp_dir(),
        AiUsageContext::new("test", "complete_async_parity"),
        Arc::new(adapter),
    );

    // Run all four tiers through the async `complete()` path.
    for tier in [
        ModelTier::Synthesis,
        ModelTier::Extraction,
        ModelTier::Background,
        ModelTier::Mechanical,
    ] {
        let _ = provider
            .complete(PromptInput::new("p"), tier)
            .await
            .expect("complete returns");
    }

    let calls = captured.lock().unwrap();
    assert_eq!(calls.len(), 4);

    // Tier-to-timeout matrix must match the legacy builder chain.
    assert_eq!(calls[0].tier, ModelTier::Synthesis);
    assert_eq!(calls[0].timeout_secs, 240);
    assert_eq!(calls[1].tier, ModelTier::Extraction);
    assert_eq!(calls[1].timeout_secs, 240);
    assert_eq!(calls[2].tier, ModelTier::Background);
    assert_eq!(calls[2].timeout_secs, 240);
    assert_eq!(calls[3].tier, ModelTier::Mechanical);
    assert_eq!(calls[3].timeout_secs, 90);

    // nice_priority is invariant across tiers.
    for call in calls.iter() {
        assert_eq!(call.nice_priority, 10);
    }
}

/// `complete_blocking` and `complete()`
/// produce identical PTY invocations for the same inputs — the two
/// methods exist as sync/async siblings and must be substitutable.
/// Without this guard the async path could silently drift (different
/// timeout, different usage_context format) from the sync path that
/// legacy `intel_queue` callers still use.
#[tokio::test]
async fn pty_provider_complete_blocking_and_async_produce_identical_calls() {
    let (sync_adapter, sync_captured) = FakePtySpawnAdapter::new(FIXTURE_PTY_STDOUT);
    let (async_adapter, async_captured) = FakePtySpawnAdapter::new(FIXTURE_PTY_STDOUT);
    let ai_config = Arc::new(AiModelConfig {
        synthesis: "shared-syn".into(),
        extraction: "shared-ext".into(),
        background: "shared-bg".into(),
        mechanical: "shared-mech".into(),
    });
    let usage_context = AiUsageContext::new("parity", "sync_vs_async");

    let sync_provider = PtyClaudeCode::with_spawn_adapter(
        Arc::clone(&ai_config),
        std::env::temp_dir(),
        usage_context.clone(),
        Arc::new(sync_adapter),
    );
    let async_provider = PtyClaudeCode::with_spawn_adapter(
        ai_config,
        std::env::temp_dir(),
        usage_context,
        Arc::new(async_adapter),
    );

    sync_provider
        .complete_blocking(PromptInput::new("p"), ModelTier::Synthesis)
        .expect("sync ok");
    async_provider
        .complete(PromptInput::new("p"), ModelTier::Synthesis)
        .await
        .expect("async ok");

    let sync_calls = sync_captured.lock().unwrap();
    let async_calls = async_captured.lock().unwrap();
    assert_eq!(sync_calls.len(), 1);
    assert_eq!(async_calls.len(), 1);
    assert_eq!(
        *sync_calls, *async_calls,
        "complete_blocking and complete() must invoke the PTY with identical \
         tier / timeout / nice_priority / usage_context"
    );
}

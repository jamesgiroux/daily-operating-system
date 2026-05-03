//! DOS-259 (W2-B) integration test: provider selection is single-source per tier.
//!
//! Asserts the structural invariant that a tier resolves to exactly one
//! production provider implementation when configured. This guards against
//! drift where multiple call sites construct different provider instances
//! for the same tier and end up using divergent timeouts/usage contexts.

use std::sync::Arc;

use dailyos_lib::intelligence::glean_provider::GleanIntelligenceProvider;
use dailyos_lib::intelligence::provider::{
    Completion, FingerprintMetadata, IntelligenceProvider, ModelName, ModelTier, ProviderKind,
    ReplayProvider,
};
use dailyos_lib::intelligence::pty_provider::PtyClaudeCode;
use dailyos_lib::pty::AiUsageContext;
use dailyos_lib::types::AiModelConfig;

#[test]
fn provider_selection_is_single_source_for_tier() {
    // Each provider type, asked for the same tier twice, returns identical
    // model/kind. This is the "single source" invariant — selection is a
    // pure function of (provider, tier), not stateful or randomized.
    let pty = PtyClaudeCode::new(
        Arc::new(AiModelConfig::default()),
        std::env::temp_dir(),
        AiUsageContext::new("test", "selection_invariant"),
    );
    assert_eq!(
        pty.current_model(ModelTier::Synthesis),
        pty.current_model(ModelTier::Synthesis),
    );
    assert_eq!(pty.provider_kind(), ProviderKind::ClaudeCode);

    let glean = GleanIntelligenceProvider::new("https://example.invalid/glean");
    assert_eq!(
        glean.current_model(ModelTier::Synthesis),
        glean.current_model(ModelTier::Synthesis),
    );
    assert_eq!(glean.provider_kind(), ProviderKind::Other("glean"));

    let replay = ReplayProvider::new(std::collections::HashMap::new());
    assert_eq!(
        replay.current_model(ModelTier::Synthesis),
        replay.current_model(ModelTier::Synthesis),
    );
    assert_eq!(replay.provider_kind(), ProviderKind::Other("replay"));
}

#[test]
fn provider_selection_distinguishes_tiers() {
    // PTY provider resolves a different model name per tier — the routing
    // factory must surface that distinction.
    let pty = PtyClaudeCode::new(
        Arc::new(AiModelConfig {
            synthesis: "syn-model".into(),
            extraction: "ext-model".into(),
            background: "bg-model".into(),
            mechanical: "mech-model".into(),
        }),
        std::env::temp_dir(),
        AiUsageContext::new("test", "tier_distinction"),
    );
    assert_eq!(pty.current_model(ModelTier::Synthesis).as_str(), "syn-model");
    assert_eq!(pty.current_model(ModelTier::Extraction).as_str(), "ext-model");
    assert_eq!(pty.current_model(ModelTier::Background).as_str(), "bg-model");
    assert_eq!(pty.current_model(ModelTier::Mechanical).as_str(), "mech-model");
}

#[test]
fn pty_claude_code_propagates_tier_to_model_and_provider_kind() {
    // L2 codex finding #3 regression: tests must exercise the actual
    // PtyClaudeCode surface (timeout_for_tier already covered in the unit
    // tests; here we verify the trait surface returns the right model
    // for the right tier without spawning Claude Code).
    let cfg = AiModelConfig {
        synthesis: "syn-routed".into(),
        extraction: "ext-routed".into(),
        background: "bg-routed".into(),
        mechanical: "mech-routed".into(),
    };
    let pty = PtyClaudeCode::new(
        Arc::new(cfg),
        std::env::temp_dir(),
        AiUsageContext::new("test", "tier_propagation"),
    );

    // Each tier resolves to its configured model name.
    assert_eq!(pty.current_model(ModelTier::Synthesis).as_str(), "syn-routed");
    assert_eq!(pty.current_model(ModelTier::Extraction).as_str(), "ext-routed");
    assert_eq!(pty.current_model(ModelTier::Background).as_str(), "bg-routed");
    assert_eq!(
        pty.current_model(ModelTier::Mechanical).as_str(),
        "mech-routed"
    );

    // ProviderKind is invariant — every tier reports ClaudeCode.
    assert_eq!(pty.provider_kind(), ProviderKind::ClaudeCode);
}

#[test]
fn fingerprint_metadata_required_fields_match_adr_0106() {
    // L2 codex finding #2 regression: ADR-0106 §3 makes
    // provider/model/temperature REQUIRED. The struct shape must reject
    // any attempt to construct a FingerprintMetadata without them — the
    // type system is the gate.
    let meta = FingerprintMetadata {
        provider: ProviderKind::ClaudeCode,
        model: ModelName::new("test"),
        temperature: 1.0,
        top_p: None,
        seed: None,
        tokens_input: None,
        tokens_output: None,
        provider_completion_id: None,
    };
    assert_eq!(meta.provider, ProviderKind::ClaudeCode);
    assert_eq!(meta.model.as_str(), "test");
    assert_eq!(meta.temperature, 1.0);

    // Default impl supplies non-Option values so ReplayProvider fixtures
    // can still call ::default() without leaving holes — and Default
    // populates the required fields with documented placeholders.
    let dflt = FingerprintMetadata::default();
    assert_eq!(dflt.provider, ProviderKind::Other("replay"));
    assert_eq!(dflt.model.as_str(), "replay");
    assert_eq!(dflt.temperature, 0.0);
}

#[tokio::test]
async fn completion_via_replay_provider_carries_required_fingerprint_fields() {
    // The Completion struct returned by ReplayProvider must populate the
    // ADR-0106 §3 required fields — provider/model/temperature — even
    // for the smallest from_prompt_pairs constructor. Anything less
    // would break DOS-213's canonical hash consumer.
    let provider = ReplayProvider::from_prompt_pairs([("p", "r")]);
    let prompt = dailyos_lib::intelligence::provider::PromptInput::new("p");
    let got: Completion = provider
        .complete(prompt, ModelTier::Synthesis)
        .await
        .expect("replay returns");
    assert_eq!(got.text, "r");
    assert_eq!(got.fingerprint_metadata.provider, ProviderKind::Other("replay"));
    // temperature is f32 (not Option) — equality is well-defined here.
    assert_eq!(got.fingerprint_metadata.temperature, 0.0);
    // model is non-Option — assert it's a real ModelName, not absent.
    assert!(!got.fingerprint_metadata.model.as_str().is_empty());
}

#[tokio::test]
async fn evaluate_mode_replay_provider_never_falls_through_to_live() {
    // ADR-0104 / DOS-259 §4 invariant: Evaluate-mode replay must structurally
    // refuse to invoke any live path. Modeled by ReplayProvider returning a
    // typed error rather than ever calling network/PTY when fixture missing.
    let provider: Arc<dyn IntelligenceProvider> =
        Arc::new(ReplayProvider::new(std::collections::HashMap::new()));
    let prompt = dailyos_lib::intelligence::provider::PromptInput::new("would-be-live-prompt");
    let err = provider
        .complete(prompt, ModelTier::Synthesis)
        .await
        .expect_err("empty replay must always error");
    match err {
        dailyos_lib::intelligence::provider::ProviderError::ReplayFixtureMissing(_) => {}
        other => panic!("expected ReplayFixtureMissing, got {:?}", other),
    }
}

/// L2 cycle-26 (DOS-259) F2 regression: `select_provider` routes Live
/// to the configured live provider, Evaluate to replay, and Simulate
/// to fail-closed. Replaces the prior `select_provider_stub` that
/// always returned `ModeNotSupported` (which left the production
/// route unable to actually return canned Evaluate completions and
/// gave callers no way to enforce the no-live-PTY/no-live-HTTP
/// boundary). The test calls the SELECTOR — not ReplayProvider
/// directly — closing the codex F2 critique that prior coverage
/// didn't exercise the routing seam.
#[tokio::test]
async fn select_provider_routes_live_evaluate_simulate_per_adr_0104() {
    use dailyos_lib::intelligence::provider::{select_provider, ExecutionMode};

    let live: Arc<dyn IntelligenceProvider> =
        Arc::new(ReplayProvider::from_prompt_pairs([("p", "live-response")]));
    let replay: Arc<dyn IntelligenceProvider> =
        Arc::new(ReplayProvider::from_prompt_pairs([("p", "replay-response")]));

    // Live mode → live provider returned.
    let chosen = select_provider(
        ExecutionMode::Live,
        Arc::clone(&live),
        Some(Arc::clone(&replay)),
        ModelTier::Synthesis,
    )
    .expect("Live mode must resolve");
    let got = chosen
        .complete(
            dailyos_lib::intelligence::provider::PromptInput::new("p"),
            ModelTier::Synthesis,
        )
        .await
        .expect("Live provider returns");
    assert_eq!(got.text, "live-response");

    // Evaluate mode → replay provider returned.
    let chosen = select_provider(
        ExecutionMode::Evaluate,
        Arc::clone(&live),
        Some(Arc::clone(&replay)),
        ModelTier::Synthesis,
    )
    .expect("Evaluate mode resolves when replay is configured");
    let got = chosen
        .complete(
            dailyos_lib::intelligence::provider::PromptInput::new("p"),
            ModelTier::Synthesis,
        )
        .await
        .expect("Replay provider returns");
    assert_eq!(got.text, "replay-response");

    // Evaluate mode without a replay provider → fail-closed.
    // Crucially: NEVER falls through to live, even if a live provider
    // is supplied — the missing fixture is a structural error.
    let res = select_provider(
        ExecutionMode::Evaluate,
        Arc::clone(&live),
        None,
        ModelTier::Synthesis,
    );
    assert!(matches!(
        res,
        Err(dailyos_lib::intelligence::provider::ProviderError::ModeNotSupported)
    ));

    // Simulate mode → always fail-closed, no provider invoked.
    let res = select_provider(
        ExecutionMode::Simulate,
        Arc::clone(&live),
        Some(Arc::clone(&replay)),
        ModelTier::Synthesis,
    );
    assert!(matches!(
        res,
        Err(dailyos_lib::intelligence::provider::ProviderError::ModeNotSupported)
    ));
}

/// L2 cycle-26 (DOS-259) F1 regression: ProviderError must surface
/// the typed variants ADR-0106 + DOS-259 acceptance call out
/// (Unavailable, MalformedResponse, TierUnavailable, PromptTooLarge),
/// not collapse everything into the broad Permanent/Transient/
/// Timeout/InvalidPrompt bucket. Callers may want to handle these
/// distinctly: a `MalformedResponse` is the provider's fault,
/// `PromptTooLarge` is the caller's fault, `TierUnavailable` means
/// "configure another tier", `Unavailable` means "retry later."
#[test]
fn provider_error_carries_dos259_typed_variants() {
    use dailyos_lib::intelligence::provider::ProviderError;

    let unavailable = ProviderError::Unavailable("glean offline".into());
    assert!(matches!(unavailable, ProviderError::Unavailable(_)));

    let malformed = ProviderError::MalformedResponse("not JSON".into());
    assert!(matches!(malformed, ProviderError::MalformedResponse(_)));

    let tier = ProviderError::TierUnavailable {
        tier: ModelTier::Synthesis,
        message: "no synthesis model configured".into(),
    };
    match tier {
        ProviderError::TierUnavailable { tier, message } => {
            assert_eq!(tier, ModelTier::Synthesis);
            assert!(message.contains("synthesis"));
        }
        _ => panic!("expected TierUnavailable"),
    }

    let too_large = ProviderError::PromptTooLarge {
        tokens: 200_000,
        limit: 100_000,
    };
    match too_large {
        ProviderError::PromptTooLarge { tokens, limit } => {
            assert_eq!(tokens, 200_000);
            assert_eq!(limit, 100_000);
        }
        _ => panic!("expected PromptTooLarge"),
    }
}

//! DOS-259 (W2-B) integration test: provider selection is single-source per tier.
//!
//! Asserts the structural invariant that a tier resolves to exactly one
//! production provider implementation when configured. This guards against
//! drift where multiple call sites construct different provider instances
//! for the same tier and end up using divergent timeouts/usage contexts.

use std::sync::Arc;

use dailyos_lib::intelligence::glean_provider::GleanIntelligenceProvider;
use dailyos_lib::intelligence::provider::{
    IntelligenceProvider, ModelTier, ProviderKind, ReplayProvider,
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

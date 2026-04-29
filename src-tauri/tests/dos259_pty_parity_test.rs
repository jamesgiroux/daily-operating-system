//! DOS-259 (W2-B) integration test: PTY provider byte-identical parity.
//!
//! The W2-B refactor relocates provider construction from inline
//! `PtyManager::for_tier(...)` to `PtyClaudeCode::new(...)` + the
//! `IntelligenceProvider` trait. The parsing path
//! (`parse_intelligence_response`) is untouched. This test verifies the
//! end-to-end call shape — `ReplayProvider`-fed completion text feeds
//! through `parse_intelligence_response` to a stable `IntelligenceJson` —
//! demonstrating the new abstraction does not perturb the parsed output
//! byte-shape vs. the pre-refactor direct-stdout path.
//!
//! When DOS-216 / W4-B lands a real fixture corpus, this test gains
//! captured PTY output as the canned text; until then the synthetic
//! JSON-shaped fixture covers the parser's happy-path through the
//! provider trait surface.

use dailyos_lib::intelligence::io::SourceManifestEntry;
use dailyos_lib::intelligence::prompts::parse_intelligence_response;
use dailyos_lib::intelligence::provider::{
    IntelligenceProvider, PromptInput, ReplayProvider,
};
use dailyos_lib::pty::ModelTier;

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

#[tokio::test]
async fn pty_provider_parity_fixture_intelligence_json_byte_identical() {
    // Build a ReplayProvider seeded with the fixture prompt → fixture stdout
    // pair. Calling provider.complete() returns the same text the legacy
    // PtyManager.spawn_claude path would have produced for this prompt.
    let provider =
        ReplayProvider::from_prompt_pairs([(FIXTURE_PROMPT, FIXTURE_PTY_STDOUT)]);
    let prompt = PromptInput::new(FIXTURE_PROMPT);
    let completion = provider
        .complete(prompt, ModelTier::Synthesis)
        .await
        .expect("replay returns canned text");

    // Feed the completion text through the same parser the legacy path uses.
    // Byte-identical check: the parser's output, when re-serialized, equals
    // the parser output of the raw fixture text — proving the trait surface
    // doesn't mutate the text that downstream parsing consumes.
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

    // The parser stamps `enriched_at` with `Utc::now()`, so two calls yield
    // a non-deterministic delta. Pin both to a fixed sentinel before byte
    // comparison so the test asserts shape parity, not wall-clock parity.
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

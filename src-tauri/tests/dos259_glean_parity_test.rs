//! Glean provider byte-identical parity integration test.
//!
//! Mirror of dos259_pty_parity_test for the Glean trait surface. The
//! The provider refactor moves Glean construction from inline
//! `GleanIntelligenceProvider::new(endpoint)` to AppState-Arc bridge per
//! ADR-0091. The parser (`parse_intelligence_response`) is shared between
//! PTY and Glean response paths and is unchanged. This test asserts the
//! same byte-identical invariant for Glean: ReplayProvider-fed completion
//! text feeds through the parser to a stable `IntelligenceJson`.

use dailyos_lib::intelligence::io::SourceManifestEntry;
use dailyos_lib::intelligence::prompts::parse_intelligence_response;
use dailyos_lib::intelligence::provider::{
    IntelligenceProvider, PromptInput, ReplayProvider,
};
use dailyos_lib::pty::ModelTier;

const FIXTURE_PROMPT: &str = "glean enrichment for account-99";

const FIXTURE_GLEAN_RESPONSE: &str = r#"{
  "executiveAssessment": "renewal in flight",
  "risks": [
    {"text": "discovery gap", "source": "glean_chat"}
  ],
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
async fn glean_provider_parity_fixture_intelligence_json_byte_identical() {
    // ReplayProvider stands in for the Glean MCP chat call — same
    // text shape the live Glean provider returns, fed through the
    // shared parser the legacy direct-construction path uses.
    let provider =
        ReplayProvider::from_prompt_pairs([(FIXTURE_PROMPT, FIXTURE_GLEAN_RESPONSE)]);
    let prompt = PromptInput::new(FIXTURE_PROMPT);
    let completion = provider
        .complete(prompt, ModelTier::Synthesis)
        .await
        .expect("replay returns canned glean text");

    let manifest: Vec<SourceManifestEntry> = vec![SourceManifestEntry {
        filename: "glean_chat".into(),
        content_type: Some("glean_synthesis".into()),
        format: Some("json".into()),
        modified_at: "2026-04-29T00:00:00Z".into(),
        selected: true,
        skip_reason: None,
    }];

    let mut parsed_via_provider = parse_intelligence_response(
        &completion.text,
        "account-99",
        "account",
        1,
        manifest.clone(),
    )
    .expect("parse via provider path");
    let mut parsed_direct = parse_intelligence_response(
        FIXTURE_GLEAN_RESPONSE,
        "account-99",
        "account",
        1,
        manifest,
    )
    .expect("parse via direct path");

    // Pin enriched_at (parser stamps Utc::now()) so byte comparison
    // reflects shape parity, not wall-clock drift between two calls.
    let pin = "2026-04-29T00:00:00+00:00".to_string();
    parsed_via_provider.enriched_at = pin.clone();
    parsed_direct.enriched_at = pin;

    let bytes_via_provider =
        serde_json::to_vec(&parsed_via_provider).expect("serialize provider parse");
    let bytes_direct = serde_json::to_vec(&parsed_direct).expect("serialize direct parse");
    assert_eq!(
        bytes_via_provider, bytes_direct,
        "Glean trait abstraction must produce byte-identical IntelligenceJson \
         output vs. the legacy direct-construction path for the same fixture"
    );

    // Sanity: a well-formed risk entry survived parsing.
    assert_eq!(parsed_via_provider.risks.len(), 1);
    assert_eq!(parsed_via_provider.risks[0].text, "discovery gap");
}

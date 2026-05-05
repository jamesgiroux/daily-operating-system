#[path = "harness/mod.rs"]
mod harness;

use harness::{
    bundle_helpers::{
        actual_post_action_state, assert_eval_bridge_stub_invoked, assert_fixture_metadata,
        assert_state_string_array_contains, assert_string_array_contains_all, bundle_fixture_path,
        assert_trust_score_at_least, assert_trust_score_below, assert_warning_present,
        claim_by_id, expected_post_action_state, run_with_synthetic_enrich_stub,
    },
    load_fixture,
};

#[test]
fn provider_hallucination_fixture_flags_low_trust_and_preserves_ground_truth() {
    let fixture = load_fixture(&bundle_fixture_path(2)).expect("bundle-2 fixture loads");
    assert_fixture_metadata(
        &fixture,
        2,
        &["source_reliability", "internal_consistency"],
        "trust_score < 0.5",
    );

    let result =
        run_with_synthetic_enrich_stub(&fixture).expect("fixture invokes through eval bridge");
    assert_eval_bridge_stub_invoked(&result);

    let expected_state = expected_post_action_state(&fixture);
    let hallucinated = claim_by_id(
        expected_state,
        "claim-test-provider-hallucinated-expansion-cancelled",
    );
    assert_trust_score_below(hallucinated, 0.5);
    assert_eq!(hallucinated["trust_band"], "needs_verification");
    assert_eq!(hallucinated["corroboration_count"], 0);
    assert_string_array_contains_all(
        &hallucinated["dominant_penalties"],
        &["source_reliability", "internal_consistency"],
    );
    assert_eq!(hallucinated["metadata"]["low_internal_consistency"], true);
    assert_eq!(
        hallucinated["metadata"]["contradicts_claim_id"],
        "claim-test-ground-truth-eu-expansion"
    );
    assert_warning_present(&fixture, "attribution_incomplete");

    let actual_state = actual_post_action_state(&result);
    let ground_truth = claim_by_id(actual_state, "claim-test-ground-truth-eu-expansion");
    assert_trust_score_at_least(ground_truth, 0.9);
    assert_eq!(
        ground_truth["text"],
        "acme.example.com has confirmed plan to expand to EU in Q3 2026."
    );
    assert_eq!(ground_truth["trust_score"], 0.92);
    assert_eq!(ground_truth["trust_version"], 1);
    assert_state_string_array_contains(
        actual_state,
        "preserved_claims",
        "claim-test-ground-truth-eu-expansion",
    );
}

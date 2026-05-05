#[path = "harness/mod.rs"]
mod harness;

use harness::{
    bundle_helpers::{
        actual_post_action_state, assert_eval_bridge_stub_invoked, assert_fixture_metadata,
        assert_state_string_array_contains, assert_string_array_contains_all,
        assert_trust_score_below, assert_warning_present, bundle_fixture_path, claim_by_id,
        expected_post_action_state, run_with_synthetic_enrich_stub,
    },
    load_fixture,
};

#[test]
fn stale_source_resurrection_fixture_preserves_dismissal_and_scores_low() {
    let fixture = load_fixture(&bundle_fixture_path(3)).expect("bundle-3 fixture loads");
    assert_fixture_metadata(
        &fixture,
        3,
        &["user_feedback_weight", "source_lifecycle_weight"],
        "trust_score < 0.4",
    );

    let result =
        run_with_synthetic_enrich_stub(&fixture).expect("fixture invokes through eval bridge");
    assert_eval_bridge_stub_invoked(&result);

    let expected_state = expected_post_action_state(&fixture);
    let resurrected = claim_by_id(expected_state, "claim-test-resurrected-inbox-vendor");
    assert_trust_score_below(resurrected, 0.4);
    assert_eq!(resurrected["trust_band"], "needs_verification");
    assert_eq!(resurrected["verification_state"], "contested");
    assert_eq!(
        resurrected["verification_reason"],
        "source_withdrawn_and_prior_user_dismissal"
    );
    assert_eq!(resurrected["metadata"]["source_withdrawn"], true);
    assert_eq!(resurrected["metadata"]["prior_user_dismissal"], true);
    assert_eq!(
        resurrected["metadata"]["prior_feedback_id"],
        "feedback-test-dismissed-inbox-vendor"
    );
    assert_warning_present(&fixture, "source_withdrawn");
    assert_warning_present(&fixture, "prior_user_dismissal");

    let expected_feedback = &expected_state["claim_feedback"][0];
    assert_eq!(
        expected_feedback["id"],
        "feedback-test-dismissed-inbox-vendor"
    );
    assert_eq!(expected_feedback["feedback_label"], "Dismissed");
    assert_eq!(
        expected_feedback["payload_json"]["reason"],
        "user_dismissed_resurrected_topic"
    );

    let actual_state = actual_post_action_state(&result);
    let dismissed = claim_by_id(actual_state, "claim-test-dismissed-inbox-vendor");
    assert_eq!(dismissed["claim_state"], "tombstoned");
    assert_eq!(dismissed["surfacing_state"], "dormant");
    assert_eq!(dismissed["data_source"], "provider_completion");
    assert_eq!(dismissed["source_ref"]["lifecycle_state"], "withdrawn");
    assert_state_string_array_contains(
        actual_state,
        "preserved_claims",
        "claim-test-dismissed-inbox-vendor",
    );

    let actual_feedback = actual_state["claim_feedback"]
        .as_array()
        .expect("claim_feedback captured");
    let preserved_feedback = actual_feedback
        .iter()
        .find(|feedback| feedback["id"] == "feedback-test-dismissed-inbox-vendor")
        .expect("dismissal feedback preserved in actual state");
    assert_eq!(preserved_feedback["feedback_type"], "mark_false");
    assert_eq!(
        preserved_feedback["payload_json"]["reason"],
        "user_dismissed_resurrected_topic"
    );
    assert_string_array_contains_all(
        &fixture.expected.output["trustAnnotations"]["executiveAssessment"]["warnings"],
        &[
            "source_withdrawn",
            "prior_user_dismissal",
            "stale_source_resurrection",
        ],
    );
}

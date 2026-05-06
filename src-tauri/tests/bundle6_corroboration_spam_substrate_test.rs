#[path = "harness/mod.rs"]
mod harness;

use harness::{
    bundle_helpers::{
        actual_post_action_state, assert_eval_bridge_stub_invoked, assert_fixture_metadata,
        assert_string_array_contains_all, assert_trust_score_at_least, assert_trust_score_below,
        assert_warning_present, bundle_fixture_path, claim_by_id, claims, confidence_evidence_for,
        expected_post_action_state, run_with_synthetic_enrich_stub,
    },
    load_fixture,
};

#[test]
fn corroboration_spam_fixture_keeps_weak_cluster_below_trust_boundary() {
    let fixture = load_fixture(&bundle_fixture_path(6)).expect("bundle-6 fixture loads");
    assert_fixture_metadata(
        &fixture,
        6,
        &["source_reliability", "contradiction_penalty"],
        "scores below 0.5 trust",
    );

    let result =
        run_with_synthetic_enrich_stub(&fixture).expect("fixture invokes through eval bridge");
    assert_eval_bridge_stub_invoked(&result);

    let expected_state = expected_post_action_state(&fixture);
    let false_provider_claim = claim_by_id(
        expected_state,
        "claim-test-provider-primary-contact-unknown",
    );
    assert_trust_score_below(false_provider_claim, 0.5);
    assert_eq!(false_provider_claim["trust_band"], "needs_verification");
    assert_eq!(false_provider_claim["verification_state"], "contested");
    assert_eq!(
        false_provider_claim["verification_reason"],
        "weighted_source_reliability_and_authoritative_contradiction"
    );
    assert_eq!(
        false_provider_claim["metadata"]["contradicts_claim_id"],
        "claim-test-strong-primary-contact-jane"
    );
    assert_eq!(
        false_provider_claim["metadata"]["raw_corroborator_count"],
        5
    );
    assert_eq!(
        false_provider_claim["metadata"]["naive_count_rejected"],
        true
    );
    assert_string_array_contains_all(
        &false_provider_claim["dominant_penalties"],
        &["source_reliability", "contradiction_penalty"],
    );

    let source_reliability = confidence_evidence_for(
        expected_state,
        "claim-test-provider-primary-contact-unknown",
        "source_reliability",
    );
    assert_eq!(
        source_reliability["factor"]["aggregation"],
        "weighted_by_evidence_weight"
    );
    assert_eq!(
        source_reliability["factor"]["raw_value"]["supporting_source_count"],
        5
    );
    assert_eq!(
        source_reliability["factor"]["raw_value"]["supporting_weight_total"],
        1
    );
    assert_eq!(
        source_reliability["factor"]["raw_value"]["contradicting_source_count"],
        1
    );
    assert_eq!(
        source_reliability["factor"]["raw_value"]["contradicting_weight_total"],
        1
    );
    assert_eq!(source_reliability["factor"]["naive_count_rejected"], true);
    assert!(
        source_reliability["factor"]["value"]
            .as_f64()
            .expect("source reliability value")
            < 0.5
            || source_reliability["factor"]["raw_value"]["supporting_weight_total"]
                == source_reliability["factor"]["raw_value"]["contradicting_weight_total"],
        "source reliability should be sub-0.5 or neutralized at equal weight with a contradiction"
    );

    let contradiction = confidence_evidence_for(
        expected_state,
        "claim-test-provider-primary-contact-unknown",
        "contradiction_penalty",
    );
    assert_eq!(
        contradiction["contradiction"]["authoritative_claim_id"],
        "claim-test-strong-primary-contact-jane"
    );
    assert_eq!(
        contradiction["contradiction"]["authoritative_source_id"],
        "src-test-strong"
    );
    assert_warning_present(&fixture, "corroboration_spam_detected");
    assert_warning_present(&fixture, "contradiction_detected");

    let strong_claim = claim_by_id(expected_state, "claim-test-strong-primary-contact-jane");
    assert_trust_score_at_least(strong_claim, 0.7);
    assert_eq!(strong_claim["trust_band"], "likely_current");

    let actual_state = actual_post_action_state(&result);
    let actual_strong = claim_by_id(actual_state, "claim-test-strong-primary-contact-jane");
    assert_trust_score_at_least(actual_strong, 0.7);

    let weak_claims = claims(actual_state)
        .iter()
        .filter(|claim| {
            claim["claim_id"]
                .as_str()
                .is_some_and(|id| id.starts_with("claim-test-weak-primary-contact-unknown-"))
        })
        .collect::<Vec<_>>();
    assert_eq!(weak_claims.len(), 5);
    assert!(weak_claims
        .iter()
        .all(|claim| claim["verification_state"] == "contested"));
}

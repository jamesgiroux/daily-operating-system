#[path = "harness/mod.rs"]
mod harness;

use harness::{
    bundle_helpers::{
        actual_post_action_state, assert_eval_bridge_stub_invoked, assert_fixture_metadata,
        assert_trust_score_at_least, assert_warning_present, bundle_fixture_path, claim_by_id,
        confidence_evidence_for, expected_post_action_state, run_with_synthetic_enrich_stub,
        trust_score,
    },
    load_fixture,
};

#[test]
fn temporal_scope_violation_fixture_preserves_closed_scope_baseline_score() {
    let fixture = load_fixture(&bundle_fixture_path(7)).expect("bundle-7 fixture loads");
    assert_fixture_metadata(
        &fixture,
        7,
        &["freshness_weight", "temporal_scope_validation"],
        "freshness_weight",
    );

    let result =
        run_with_synthetic_enrich_stub(&fixture).expect("fixture invokes through eval bridge");
    assert_eval_bridge_stub_invoked(&result);

    let expected_state = expected_post_action_state(&fixture);
    let expected_claim = claim_by_id(expected_state, "claim-test-closed-renewal-checklist");
    assert_eq!(expected_claim["temporal_scope"], "closed");
    assert_eq!(expected_claim["trust_score_before"], 0.76);
    assert_eq!(expected_claim["trust_score_changed"], false);
    assert_eq!(
        expected_claim["trust_score"],
        expected_claim["trust_score_before"]
    );
    assert_trust_score_at_least(expected_claim, 0.7);
    assert_eq!(
        expected_claim["metadata"]["post_closure_refresh_rejected"],
        true
    );
    assert_eq!(
        expected_claim["metadata"]["rejected_source_id"],
        "src-test-source-stale-postclosure"
    );

    let evidence = confidence_evidence_for(
        expected_state,
        "claim-test-closed-renewal-checklist",
        "freshness_weight",
    );
    assert_eq!(evidence["raw_value"], 0);
    assert_eq!(evidence["effective_value"], 0);
    assert_eq!(
        evidence["reason"],
        "post_closure_evidence_cannot_refresh_closed_scope_claim"
    );
    assert_eq!(
        evidence["temporal_scope_validation"]["claim_temporal_scope"],
        "closed"
    );
    assert_eq!(evidence["temporal_scope_validation"]["accepted"], false);
    assert_eq!(
        evidence["temporal_scope_validation"]["warning"],
        "corroboration_outside_temporal_scope"
    );
    assert_warning_present(&fixture, "corroboration_outside_temporal_scope");

    let rejected = &expected_state["rejected_corroborations"][0];
    assert_eq!(
        rejected["rejection_reason"],
        "corroboration_outside_temporal_scope"
    );
    assert_eq!(rejected["affected_trust_score"], false);
    assert_eq!(rejected["days_after_window_end"], 150);

    let actual_state = actual_post_action_state(&result);
    let actual_claim = claim_by_id(actual_state, "claim-test-closed-renewal-checklist");
    assert_eq!(actual_claim["temporal_scope"], "closed");
    assert_eq!(actual_claim["metadata"]["original_trust_score"], 0.76);
    assert_eq!(trust_score(actual_claim), trust_score(expected_claim));
    assert_eq!(actual_claim["trust_score"], 0.76);
    assert_eq!(actual_claim["trust_version"], 1);
}

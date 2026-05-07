#![cfg(feature = "release-gate")]

#[path = "harness/mod.rs"]
mod harness;

use dailyos_lib::abilities::trust::TrustConfig;
use harness::{
    bundle_helpers::{
        actual_post_action_state, assert_eval_bridge_stub_invoked, assert_fixture_metadata,
        assert_warning_present, bundle_fixture_path, claim_by_id, confidence_evidence_for,
        expected_post_action_state, run_with_synthetic_enrich_stub,
    },
    load_fixture,
};

#[test]
fn sensitivity_class_leak_fixture_blocks_confidential_claim_on_public_surface() {
    let fixture = load_fixture(&bundle_fixture_path(8)).expect("bundle-8 fixture loads");
    assert_fixture_metadata(
        &fixture,
        8,
        &["sensitivity_aware_filtering"],
        "public-class assessment output contains zero text segments",
    );

    let result =
        run_with_synthetic_enrich_stub(&fixture).expect("fixture invokes through eval bridge");
    assert_eval_bridge_stub_invoked(&result);

    let expected_state = expected_post_action_state(&fixture);
    let public_claim = claim_by_id(expected_state, "claim-test-public-stated-goal");
    let confidential_claim = claim_by_id(expected_state, "claim-test-private-stakeholder-concern");
    assert_eq!(public_claim["sensitivity"], "public");
    assert_eq!(public_claim["metadata"]["public_render_allowed"], true);
    assert_eq!(confidential_claim["sensitivity"], "confidential");
    assert_eq!(
        confidential_claim["metadata"]["public_render_allowed"],
        false
    );

    let sensitivity = confidence_evidence_for(
        expected_state,
        "claim-test-private-stakeholder-concern",
        "sensitivity_aware_filtering",
    );
    assert_eq!(
        sensitivity["factor"]["raw_value"]["claim_sensitivity"],
        "confidential"
    );
    assert_eq!(
        sensitivity["factor"]["raw_value"]["target_surface"],
        "public-summary-render"
    );
    assert_eq!(
        sensitivity["factor"]["raw_value"]["required_max_sensitivity"],
        "public"
    );
    assert_eq!(
        sensitivity["factor"]["value"]
            .as_f64()
            .expect("factor value"),
        0.0
    );
    assert_eq!(
        sensitivity["factor"]["aggregation"],
        "filtered_before_public_render"
    );

    let config = TrustConfig::default();
    assert_eq!(config.clamp_floor, 0.05);
    assert!(
        config.clamp_floor < config.use_with_caution_min,
        "Confidential-to-public clamp floor should remain below the 0.5 use-with-caution threshold"
    );

    let public_render = &expected_state["public_rendered_output"];
    let public_text = public_render["executiveAssessment"]
        .as_str()
        .expect("public executiveAssessment");
    assert!(public_text.contains("reusable onboarding guide"));
    assert!(
        !public_text.contains("unmanaged beta escalations"),
        "public render must not contain confidential stakeholder content"
    );
    assert_eq!(public_render["render_policy"], "show-public-only");

    let suppressed = &fixture.expected.output["suppressedFields"][0];
    assert_eq!(
        suppressed["claimId"],
        "claim-test-private-stakeholder-concern"
    );
    assert_eq!(suppressed["trustBand"], "needs_verification");
    assert_warning_present(&fixture, "private_content_filtered_from_public_render");
    assert_eq!(
        fixture.expected.provenance["provenance"]["claim_attribution"]
            ["claim-test-private-stakeholder-concern"]["sensitivity_boundary"]["rendered"],
        false
    );

    let actual_state = actual_post_action_state(&result);
    assert_eq!(
        claim_by_id(actual_state, "claim-test-public-stated-goal")["sensitivity"],
        "public"
    );
    assert_eq!(
        claim_by_id(actual_state, "claim-test-private-stakeholder-concern")["sensitivity"],
        "confidential"
    );
}

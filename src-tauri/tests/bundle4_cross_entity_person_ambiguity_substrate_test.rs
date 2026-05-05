#[path = "harness/mod.rs"]
mod harness;

use harness::{
    bundle_helpers::{
        actual_post_action_state, assert_eval_bridge_stub_invoked, assert_fixture_metadata,
        assert_string_array_contains_all, assert_trust_score_below, assert_warning_present,
        bundle_fixture_path, claim_by_id, claims, confidence_evidence_for,
        expected_post_action_state, run_with_synthetic_enrich_stub,
    },
    load_fixture,
};

#[test]
fn cross_entity_person_ambiguity_fixture_rejects_account_b_project_bleed() {
    let fixture = load_fixture(&bundle_fixture_path(4)).expect("bundle-4 fixture loads");
    assert_fixture_metadata(
        &fixture,
        4,
        &["cross_entity_coherence", "subject_fit_confidence"],
        "cross_entity_coherence factor scores below 0.3",
    );

    let result =
        run_with_synthetic_enrich_stub(&fixture).expect("fixture invokes through eval bridge");
    assert_eval_bridge_stub_invoked(&result);

    let expected_state = expected_post_action_state(&fixture);
    let not_created = expected_state["not_created_claim_subjects"]
        .as_array()
        .expect("not_created_claim_subjects array");
    let rejected_project_ids = not_created
        .iter()
        .map(|entry| {
            assert_eq!(entry["target_account_id"], "acct-test-1");
            assert_eq!(entry["created"], false);
            assert_eq!(entry["reason"], "cross_entity_subject_ambiguity");
            entry["subject_ref"]["id"].as_str().expect("project id")
        })
        .collect::<Vec<_>>();
    assert_eq!(rejected_project_ids, ["proj-test-b-1", "proj-test-b-2"]);

    let rejected = &expected_state["rejected_claims"][0];
    assert_eq!(rejected["claim_id"], "claim-test-provider-person-context-bleed");
    assert_eq!(rejected["created"], false);
    assert_eq!(rejected["requested_entity_ref"]["id"], "acct-test-1");
    assert_eq!(rejected["origin_entity_ref"]["id"], "acct-test-2");
    assert_trust_score_below(rejected, 0.3);
    assert_string_array_contains_all(
        &rejected["dominant_penalties"],
        &["cross_entity_coherence", "subject_fit_confidence"],
    );

    let evidence = confidence_evidence_for(
        expected_state,
        "claim-test-provider-person-context-bleed",
        "cross_entity_coherence",
    );
    assert!(
        evidence["factor"]["raw_value"].as_f64().expect("raw value") < 0.3,
        "cross_entity_coherence raw value should be below 0.3"
    );
    assert_string_array_contains_all(&evidence["hits"], &["proj-test-b-1", "proj-test-b-2"]);
    assert_warning_present(&fixture, "cross_entity_subject_ambiguity");

    let actual_state = actual_post_action_state(&result);
    let leaked_project_claims = claims(actual_state)
        .iter()
        .filter(|claim| {
            let subject_id = claim["subject_ref"]["id"].as_str();
            let topic_key = claim["topic_key"].as_str();
            matches!(subject_id, Some("proj-test-b-1" | "proj-test-b-2"))
                && topic_key.is_some_and(|topic| topic.starts_with("acct-test-1:"))
        })
        .collect::<Vec<_>>();
    assert!(
        leaked_project_claims.is_empty(),
        "Account A should have zero claims pointing at Account B project subjects: {leaked_project_claims:?}"
    );

    for (claim_id, expected_score) in [
        ("claim-test-b-proj-1", 0.92),
        ("claim-test-b-proj-2", 0.91),
    ] {
        let account_b_claim = claim_by_id(actual_state, claim_id);
        assert_eq!(account_b_claim["subject_ref"]["kind"], "project");
        assert_eq!(account_b_claim["metadata"]["account_id"], "acct-test-2");
        assert_eq!(account_b_claim["trust_score"], expected_score);
    }
}

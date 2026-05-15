#![cfg(feature = "release-gate")]

#[path = "harness/mod.rs"]
mod harness;

use dailyos_lib::release_gate::{DEFAULT_MANDATORY_BUNDLES, DEFAULT_TRACKED_BUNDLES};
use harness::{
    bundle_helpers::{bundle_fixture_path, expected_post_action_state},
    load_fixture, prepare_fixture_for_run, EvalFixture,
};
use serde_json::Value;

const BUNDLE: u32 = 14;
const SUBJECT_ID: &str = "account-stale-example";
const CURRENT_CLAIM_ID: &str = "claim-b14-current-resolved-risk";
const SOURCE_ASOF_STALE_CLAIM_ID: &str = "claim-b14-source-asof-independent";
const SUPERSEDED_CLAIM_ID: &str = "claim-b14-superseded-open-risk";
const FEEDBACK_ONLY_CLAIM_ID: &str = "claim-b14-feedback-only-old-title";

#[test]
fn fixture_metadata_matches_bundle14_contract() {
    let fixture = bundle14();

    assert_eq!(fixture.metadata.bundle, Some(BUNDLE));
    assert_eq!(fixture.metadata.scenario_id, "stale-current-contradiction");
    assert_eq!(fixture.metadata.anonymization_cert, "synthetic");
    assert_eq!(
        fixture.metadata.trust_factors_dominant,
        [
            "freshness",
            "contradiction",
            "source_semantics",
            "user_feedback"
        ]
    );
    assert!(fixture
        .metadata
        .surfaces_exercised
        .iter()
        .any(|surface| surface == "prepare_meeting"));
    assert!(fixture
        .metadata
        .surfaces_exercised
        .iter()
        .any(|surface| surface == "get_entity_context"));
    assert!(fixture
        .metadata
        .surfaces_exercised
        .iter()
        .any(|surface| surface == "get_daily_readiness"));
    assert!(fixture
        .metadata
        .pass_fail_definition
        .contains("stale current advice renders as current state"));
}

#[test]
fn state_sql_prepares_required_substrate_groups() {
    let fixture = bundle14();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-14 fixture prepares");
    let claim_ids = prepared
        .entity_context_claims
        .iter()
        .map(|claim| claim.id.as_str())
        .collect::<Vec<_>>();

    for expected in [
        SOURCE_ASOF_STALE_CLAIM_ID,
        "claim-b14-contradiction-old-happy",
        "claim-b14-contradiction-fresh-risk",
        SUPERSEDED_CLAIM_ID,
        CURRENT_CLAIM_ID,
        FEEDBACK_ONLY_CLAIM_ID,
    ] {
        assert!(
            claim_ids.contains(&expected),
            "state.sql should seed substrate claim {expected}"
        );
    }

    let meeting_context = prepared
        .prepare_meeting_context
        .as_ref()
        .expect("prepare meeting context is captured");
    let prompt_claim_ids = meeting_context
        .claims
        .iter()
        .map(|claim| claim.id.as_str())
        .collect::<Vec<_>>();
    assert!(prompt_claim_ids.contains(&CURRENT_CLAIM_ID));
    assert!(!prompt_claim_ids.contains(&SUPERSEDED_CLAIM_ID));
    assert!(!prompt_claim_ids.contains(&FEEDBACK_ONLY_CLAIM_ID));
}

#[test]
fn source_asof_is_independent_from_observed_at() {
    let fixture = bundle14();
    let state = expected_post_action_state(&fixture);
    let claim = claim_by_id(state, SOURCE_ASOF_STALE_CLAIM_ID);
    let source_semantics = &fixture.external_replay["downstream_source_semantics"][0];

    assert_eq!(claim["source_asof"], "2025-11-15T10:00:00Z");
    assert_eq!(claim["observed_at"], "2026-05-14T09:00:00Z");
    assert_eq!(source_semantics["source_asof"], claim["source_asof"]);
    assert_eq!(source_semantics["observed_at"], claim["observed_at"]);
    assert_eq!(claim["metadata"]["freshness_age_days"], 181);
    assert_ne!(claim["trust_band"], "likely_current");
}

#[test]
fn prepare_meeting_suppresses_attempted_stale_current_talking_point() {
    let fixture = bundle14();
    let output = &fixture.expected.output;
    let prepare = &output["surfaces"]["prepare_meeting"];
    let rejected = &prepare["rejected_provider_candidates"][0];

    assert_eq!(rejected["source_claim_id"], SUPERSEDED_CLAIM_ID);
    assert_eq!(
        fixture.provider_replay["attempted_stale_current_talking_point"]["source_claim_id"],
        SUPERSEDED_CLAIM_ID
    );
    assert_eq!(
        prepare["topics"][0]["source_claim_ids"][0],
        CURRENT_CLAIM_ID
    );
    assert!(!array_contains_str(
        &prepare["topics"][0]["source_claim_ids"],
        SUPERSEDED_CLAIM_ID
    ));
    assert!(output["current_rendering"]["stale_current_advice_absent"]
        .as_bool()
        .expect("stale_current_advice_absent bool"));
}

#[test]
fn get_entity_context_splits_current_from_historical_or_qualified_context() {
    let fixture = bundle14();
    let entity = &fixture.expected.output["surfaces"]["get_entity_context"];

    assert_eq!(entity["subject"]["entity_id"], SUBJECT_ID);
    assert_eq!(entity["current_state_claim_id"], CURRENT_CLAIM_ID);
    assert!(array_contains_str(
        &entity["historical_or_qualified_claim_ids"],
        SOURCE_ASOF_STALE_CLAIM_ID
    ));
    assert!(array_contains_str(
        &entity["historical_or_qualified_claim_ids"],
        "claim-b14-historical-case-study"
    ));
    assert!(!array_contains_str(
        &entity["current_claim_ids"],
        SUPERSEDED_CLAIM_ID
    ));
    assert!(!array_contains_str(
        &entity["current_claim_ids"],
        FEEDBACK_ONLY_CLAIM_ID
    ));
}

#[test]
fn contradiction_and_supersession_paths_are_both_exercised() {
    let fixture = bundle14();
    let state = expected_post_action_state(&fixture);
    let edges = state["claim_contradictions"]
        .as_array()
        .expect("claim_contradictions array");

    assert!(edges
        .iter()
        .any(|edge| edge["branch_kind"] == "contradiction"));
    assert!(edges
        .iter()
        .any(|edge| edge["branch_kind"] == "supersession"));

    let superseded = claim_by_id(state, SUPERSEDED_CLAIM_ID);
    assert_eq!(superseded["claim_state"], "dormant");
    assert_eq!(superseded["surfacing_state"], "dormant");
    assert_eq!(superseded["demotion_reason"], "superseded");
    assert_eq!(superseded["superseded_by"], CURRENT_CLAIM_ID);
}

#[test]
fn trust_assessment_uses_band_enum_not_only_caveat_text() {
    let fixture = bundle14();
    let state = expected_post_action_state(&fixture);

    for claim_id in [
        SOURCE_ASOF_STALE_CLAIM_ID,
        "claim-b14-contradiction-old-happy",
        "claim-b14-contradiction-fresh-risk",
    ] {
        let claim = claim_by_id(state, claim_id);
        assert!(
            matches!(
                claim["trust_band"].as_str(),
                Some("use_with_caution" | "needs_verification")
            ),
            "{claim_id} should be downgraded on the trust_band enum"
        );
        assert_ne!(claim["trust_band"], "likely_current");
    }

    let warning_claim_ids = fixture.expected.provenance["provenance"]["warnings"]
        .as_array()
        .expect("warnings array")
        .iter()
        .map(|warning| warning.to_string())
        .collect::<Vec<_>>();
    assert!(warning_claim_ids
        .iter()
        .any(|warning| warning.contains(SOURCE_ASOF_STALE_CLAIM_ID)));
}

#[test]
fn lint_oracle_flags_unsuppressed_stale_current_contradiction() {
    let fixture = bundle14();
    let lint = &fixture.expected.output["validation_lint"];

    assert_eq!(
        lint["rule"],
        "stale_current_contradiction_requires_suppression_or_timestamped_disagreement"
    );
    assert_eq!(lint["status"], "would_fail");
    assert_eq!(
        lint["failure_case"],
        "old_open_and_fresh_resolved_both_current_without_qualification"
    );
}

#[test]
fn three_surfaces_agree_via_normalized_field_level_matrix() {
    let fixture = bundle14();
    let rows = fixture.expected.output["normalized_field_level_diff_matrix"]
        .as_array()
        .expect("normalized matrix array");

    for row in rows {
        assert_eq!(row["equal"], true, "matrix row should be equal: {row:?}");
        assert_eq!(row["get_entity_context"], row["prepare_meeting"]);
        assert_eq!(row["prepare_meeting"], row["get_daily_readiness"]);
    }
    assert!(fixture.expected.output["field_level_diffs"]
        .as_array()
        .expect("field_level_diffs array")
        .is_empty());
}

#[test]
fn mark_outdated_feedback_prevents_post_feedback_current_reuse() {
    let fixture = bundle14();
    let state = expected_post_action_state(&fixture);
    let claim = claim_by_id(state, FEEDBACK_ONLY_CLAIM_ID);
    let feedback = &state["claim_feedback"][0];
    let render_attempt = &state["post_feedback_render_attempt"];

    assert_eq!(claim["claim_state"], "dormant");
    assert_eq!(claim["surfacing_state"], "dormant");
    assert_eq!(claim["demotion_reason"], "mark_outdated");
    assert!(claim["superseded_by"].is_null());
    assert_eq!(feedback["feedback_type"], "mark_outdated");
    assert_eq!(
        feedback["payload_json"]["render_policy"],
        "HiddenFromCurrent"
    );
    assert_eq!(render_attempt["claim_id"], FEEDBACK_ONLY_CLAIM_ID);
    assert_eq!(render_attempt["rendered_as_current"], false);
    assert_eq!(render_attempt["no_resurrection"], true);
}

#[test]
fn historical_case_study_remains_historical_not_current_state() {
    let fixture = bundle14();
    let state = expected_post_action_state(&fixture);
    let claim = claim_by_id(state, "claim-b14-historical-case-study");
    let historical =
        &fixture.expected.output["surfaces"]["get_entity_context"]["historical_case_study"];

    assert_eq!(claim["temporal_scope"], "point_in_time");
    assert_eq!(claim["metadata"]["render_class"], "historical");
    assert_eq!(claim["metadata"]["cannot_drive_current_state"], true);
    assert_eq!(historical["render_class"], "historical");
    assert_eq!(historical["can_drive_current_state"], false);
}

#[test]
fn bundle14_is_mandatory_in_release_gate_defaults() {
    assert!(DEFAULT_MANDATORY_BUNDLES.contains(&"bundle-14"));
    assert!(!DEFAULT_TRACKED_BUNDLES.contains(&"bundle-14"));
}

fn bundle14() -> EvalFixture {
    load_fixture(&bundle_fixture_path(BUNDLE)).expect("bundle-14 fixture loads")
}

fn claim_by_id<'a>(state: &'a Value, claim_id: &str) -> &'a Value {
    state["intelligence_claims"]
        .as_array()
        .expect("intelligence_claims array")
        .iter()
        .find(|claim| claim["claim_id"] == claim_id)
        .unwrap_or_else(|| panic!("missing claim {claim_id}"))
}

fn array_contains_str(array: &Value, expected: &str) -> bool {
    array
        .as_array()
        .expect("string array")
        .iter()
        .any(|value| value.as_str() == Some(expected))
}

#![cfg(feature = "release-gate")]

#[path = "harness/mod.rs"]
mod harness;

use std::collections::BTreeSet;

use dailyos_lib::release_gate::{DEFAULT_MANDATORY_BUNDLES, DEFAULT_TRACKED_BUNDLES};
use harness::{
    bundle_helpers::{bundle_fixture_path, expected_post_action_state},
    load_fixture, prepare_fixture_for_run, EvalFixture,
};
use serde_json::Value;

const BUNDLE: u32 = 15;
const ACCOUNT_ID: &str = "account-b15-example";
const MEETING_ID: &str = "meeting-b15-example";
const PROJECT_ID: &str = "project-b15-example";
const PERSON_ID: &str = "person-b15-example";
const ACCOUNT_HEALTH_CLAIM_ID: &str = "claim-b15-account-health-current";
const PROJECT_STATUS_CLAIM_ID: &str = "claim-b15-project-status-current";
const PERSON_ACTION_CLAIM_ID: &str = "claim-b15-person-action-current";
const LINT_BLOCKED_CLAIM_ID: &str = "claim-b15-lint-blocked-bleed";
const PRE_REFRESH_CLAIM_ID: &str = "claim-b15-pre-refresh-healthy";
const POST_REFRESH_CLAIM_ID: &str = "claim-b15-post-refresh-risk";

const MATRIX_SURFACE_KEYS: [&str; 5] = [
    "get_entity_context",
    "prepare_meeting",
    "get_daily_readiness",
    "dashboard_render",
    "mcp_bridge_response",
];

#[test]
fn cross_surface_fixture_comparison_prepares_shared_substrate() {
    let fixture = bundle15();

    assert_eq!(fixture.metadata.bundle, Some(BUNDLE));
    assert_eq!(fixture.metadata.scenario_id, "cross-surface-consistency");
    assert_eq!(fixture.metadata.anonymization_cert, "synthetic");
    assert_eq!(
        fixture.metadata.trust_factors_dominant,
        [
            "cross_surface_consistency",
            "source_freshness",
            "subject_ownership",
            "render_policy",
            "bridge_parity"
        ]
    );
    assert!(fixture
        .metadata
        .pass_fail_definition
        .contains("normalized field-level values"));

    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-15 fixture prepares");
    let claim_ids = prepared
        .entity_context_claims
        .iter()
        .map(|claim| claim.id.as_str())
        .collect::<Vec<_>>();

    for expected in [
        ACCOUNT_HEALTH_CLAIM_ID,
        PROJECT_STATUS_CLAIM_ID,
        PERSON_ACTION_CLAIM_ID,
        LINT_BLOCKED_CLAIM_ID,
        PRE_REFRESH_CLAIM_ID,
        POST_REFRESH_CLAIM_ID,
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
    assert_eq!(meeting_context.meeting.id, MEETING_ID);

    let prompt_claim_ids = meeting_context
        .claims
        .iter()
        .map(|claim| claim.id.as_str())
        .collect::<Vec<_>>();
    for expected in [
        ACCOUNT_HEALTH_CLAIM_ID,
        PROJECT_STATUS_CLAIM_ID,
        PERSON_ACTION_CLAIM_ID,
    ] {
        assert!(
            prompt_claim_ids.contains(&expected),
            "prepare_meeting should compose claim {expected}"
        );
    }
    assert!(!prompt_claim_ids.contains(&LINT_BLOCKED_CLAIM_ID));
}

#[test]
fn surface_matrix_covers_daily_meeting_entity_dashboard_and_mcp_outputs() {
    let fixture = bundle15();
    let output = &fixture.expected.output;
    let surfaces = &output["surfaces"];

    for surface in [
        "get_entity_context",
        "get_entity_context_project",
        "get_entity_context_person",
        "prepare_meeting",
        "get_daily_readiness",
        "dashboard",
        "mcp",
    ] {
        assert!(
            surfaces[surface].is_object(),
            "expected output should include surface {surface}"
        );
    }

    for required_surface in [
        "get_entity_context",
        "prepare_meeting",
        "get_daily_readiness",
        "dashboard",
        "mcp",
    ] {
        assert!(fixture
            .metadata
            .surfaces_exercised
            .iter()
            .any(|surface| surface == required_surface));
        assert!(
            surface_invocations(&fixture)
                .iter()
                .any(|surface| *surface == required_surface),
            "inputs.json should capture invocation for {required_surface}"
        );
    }

    for field in [
        "primary_account_id",
        "eligible_meeting_count",
        "account_health_risk",
        "project_status",
        "current_state_claim_id",
        "source_asof",
        "produced_at_generated_at",
        "trust_band",
    ] {
        assert!(matrix_row(output, field)["equal"]
            .as_bool()
            .unwrap_or(false));
    }
}

#[test]
fn meeting_counts_use_the_same_eligibility_set() {
    let fixture = bundle15();
    let output = &fixture.expected.output;
    let daily = &output["surfaces"]["get_daily_readiness"]["readiness_checks"];
    let dashboard = &output["surfaces"]["dashboard"];
    let oracle = &output["meeting_count_oracle"];

    assert_eq!(daily["eligible_meeting_count"], dashboard["meeting_count"]);
    assert_eq!(
        daily["ready_meeting_count"],
        dashboard["ready_meeting_count"]
    );
    assert_eq!(daily["eligible_meeting_count"], 4);
    assert_eq!(oracle["same_eligibility_set"], true);
    assert_eq!(
        oracle["counterexample_daily_page_5_readiness_4"]["status"],
        "would_fail"
    );
    assert_normalized_row_equal(matrix_row(output, "eligible_meeting_count"));
}

#[test]
fn primary_entity_disagreement_is_a_release_gate_failure() {
    let fixture = bundle15();
    let output = &fixture.expected.output;
    let oracle = &output["primary_entity_oracle"];

    assert_normalized_row_equal(matrix_row(output, "primary_account_id"));
    assert_eq!(oracle["primary_account_id"], ACCOUNT_ID);
    assert_eq!(
        oracle["disagreement_counterexample"]["status"],
        "release_gate_failure"
    );
    assert_eq!(
        oracle["disagreement_counterexample"]["mandatory_bundle"],
        "bundle-15"
    );
    assert_eq!(output["validation_lint"]["release_gate_failure"], true);
}

#[test]
fn health_risk_state_agrees_across_surfaces_or_degrades() {
    let fixture = bundle15();
    let output = &fixture.expected.output;
    let row = matrix_row(output, "account_health_risk");

    assert_normalized_row_equal(row);
    for key in MATRIX_SURFACE_KEYS {
        assert_ne!(
            row[key], "healthy",
            "{key} must not silently diverge healthy"
        );
    }
    assert_eq!(
        output["surfaces"]["get_entity_context"]["entries"][0]["current_state"]["health"],
        "at_risk"
    );
    assert_eq!(
        output["surfaces"]["prepare_meeting"]["topics"][0]["health_posture"],
        "at_risk"
    );
}

#[test]
fn project_detail_matches_account_project_section() {
    let fixture = bundle15();
    let output = &fixture.expected.output;
    let pinned = &output["project_page_surface_pinned"];

    assert_normalized_row_equal(matrix_row(output, "project_status"));
    assert_eq!(pinned["equal"], true);
    assert_eq!(pinned["project_page"]["entity_id"], PROJECT_ID);
    assert_eq!(
        pinned["project_page"]["status"],
        pinned["account_page_project_section"]["status"]
    );
    assert_eq!(
        pinned["project_page"]["claim_id"],
        pinned["account_page_project_section"]["claim_id"]
    );
}

#[test]
fn person_detail_keeps_actions_and_meetings_used_by_meeting_prep() {
    let fixture = bundle15();
    let output = &fixture.expected.output;
    let person_surface = &output["surfaces"]["get_entity_context_person"];
    let meeting_surface = &output["surfaces"]["prepare_meeting"];
    let oracle = &output["person_action_oracle"];

    assert_eq!(person_surface["subject"]["entity_id"], PERSON_ID);
    assert!(array_contains_str(&person_surface["meetings"], MEETING_ID));
    assert_eq!(
        person_surface["actions"][0]["claim_id"],
        PERSON_ACTION_CLAIM_ID
    );
    assert!(array_contains_str(
        &meeting_surface["open_loops"][0]["source_claim_ids"],
        PERSON_ACTION_CLAIM_ID
    ));
    assert_eq!(oracle["person_detail_has_action"], true);
    assert_eq!(oracle["person_detail_has_meeting"], true);
    assert_eq!(oracle["prepare_meeting_uses_person_context"], true);
    assert_eq!(
        oracle["empty_person_detail_counterexample"]["status"],
        "would_fail"
    );
}

#[test]
fn stale_or_degraded_state_has_visible_timestamps_on_every_surface() {
    let fixture = bundle15();
    let output = &fixture.expected.output;
    let provenance = &fixture.expected.provenance["provenance"];

    assert_normalized_row_equal(matrix_row(output, "source_asof"));
    assert_normalized_row_equal(matrix_row(output, "produced_at_generated_at"));

    for surface in [
        "get_entity_context",
        "prepare_meeting",
        "get_daily_readiness",
        "dashboard",
        "mcp",
    ] {
        let visibility = &provenance["timestamp_visibility"][surface];
        assert_eq!(
            visibility["source_asof_visible"], true,
            "{surface} should expose source_asof"
        );
        assert!(
            visibility["produced_at_visible"].as_bool().unwrap_or(false)
                || visibility["generated_at_visible"]
                    .as_bool()
                    .unwrap_or(false),
            "{surface} should expose produced_at or generated_at"
        );
    }

    assert!(warning_present(
        &fixture.expected.provenance,
        "lint_blocked_claim"
    ));
    assert!(warning_present(
        &fixture.expected.provenance,
        "refresh_completed_post_state"
    ));
}

#[test]
fn lint_blocked_claims_never_render_confidently_elsewhere() {
    let fixture = bundle15();
    let output = &fixture.expected.output;
    let state = expected_post_action_state(&fixture);
    let claim = claim_by_id(state, LINT_BLOCKED_CLAIM_ID);

    assert_eq!(claim["claim_state"], "dormant");
    assert_eq!(claim["surfacing_state"], "dormant");
    assert_eq!(claim["demotion_reason"], "lint_blocked_subject_bleed");
    assert_eq!(claim["trust_band"], "needs_verification");
    assert_eq!(claim["metadata"]["rendered_confidently"], false);
    assert_eq!(
        output["stale_degraded_visibility"]["blocked_claim_visible_as_confident_anywhere"],
        false
    );

    for surface in [
        "get_entity_context",
        "prepare_meeting",
        "get_daily_readiness",
        "dashboard",
        "mcp",
    ] {
        let blocked = &output["surfaces"][surface]["blocked_claims"][0];
        assert_eq!(blocked["claim_id"], LINT_BLOCKED_CLAIM_ID);
        assert_eq!(blocked["render_policy"], "blocked");
        assert_eq!(blocked["rendered_confidently"], false);
    }

    let lint = &state["lint_blocking"];
    assert_eq!(lint["claim_id"], LINT_BLOCKED_CLAIM_ID);
    assert_eq!(lint["no_confident_rendering"], true);
}

#[test]
fn mcp_tauri_parity_preserves_actor_filtered_policy() {
    let fixture = bundle15();
    let output = &fixture.expected.output;
    let policy = &output["mcp_redaction_policy"];

    assert_eq!(policy["actor"], "seeded-tauri-user");
    assert_eq!(
        policy["unexpected_divergences"].as_array().unwrap().len(),
        0
    );
    assert_normalized_row_equal(matrix_row(output, "primary_account_id"));
    assert_normalized_row_equal(matrix_row(output, "account_health_risk"));
    assert_normalized_row_equal(matrix_row(output, "project_status"));
    assert_normalized_row_equal(matrix_row(output, "current_state_claim_id"));

    for field_policy in policy["fields"].as_array().expect("fields array") {
        match field_policy["policy"].as_str() {
            Some("same_value_required") => {
                assert_eq!(
                    field_policy["tauri_value"], field_policy["mcp_value"],
                    "MCP must match Tauri for {}",
                    field_policy["field"]
                );
            }
            Some("diagnostics_warnings_dropped_for_mcp") => {
                assert!(
                    field_policy["mcp_value"].is_null(),
                    "diagnostic warning redaction should be explicit"
                );
            }
            other => panic!("unexpected MCP redaction policy {other:?}"),
        }
    }
}

#[test]
fn activity_log_refresh_completed_reads_post_refresh_state() {
    let fixture = bundle15();
    let state = expected_post_action_state(&fixture);
    let refresh = &state["refresh_completion"];

    assert_eq!(state["activity_log"][0]["event_type"], "refresh_completed");
    assert_eq!(refresh["pre_refresh_claim_id"], PRE_REFRESH_CLAIM_ID);
    assert_eq!(refresh["post_refresh_claim_id"], POST_REFRESH_CLAIM_ID);
    assert_eq!(
        refresh["pre_refresh_old_state_visible_after_refresh"],
        false
    );

    for surface in ["get_entity_context", "prepare_meeting", "dashboard", "mcp"] {
        assert_eq!(
            refresh["post_refresh_reads"][surface]["rendered_claim_id"], POST_REFRESH_CLAIM_ID,
            "{surface} should render post-refresh claim"
        );
        assert_eq!(refresh["post_refresh_reads"][surface]["state"], "at_risk");
    }

    let pre_refresh_claim = claim_by_id(state, PRE_REFRESH_CLAIM_ID);
    assert_eq!(pre_refresh_claim["claim_state"], "dormant");
    assert_eq!(
        pre_refresh_claim["metadata"]["rendered_after_refresh"],
        false
    );
}

#[test]
fn bundle15_is_mandatory_in_release_gate_defaults() {
    assert!(DEFAULT_MANDATORY_BUNDLES.contains(&"bundle-15"));
    assert!(!DEFAULT_TRACKED_BUNDLES.contains(&"bundle-15"));
}

fn bundle15() -> EvalFixture {
    load_fixture(&bundle_fixture_path(BUNDLE)).expect("bundle-15 fixture loads")
}

fn surface_invocations(fixture: &EvalFixture) -> Vec<&str> {
    fixture.inputs_json["surface_invocations"]
        .as_array()
        .expect("surface_invocations array")
        .iter()
        .filter_map(|invocation| invocation["surface"].as_str())
        .collect()
}

fn matrix_row<'a>(output: &'a Value, field: &str) -> &'a Value {
    output["normalized_field_level_diff_matrix"]
        .as_array()
        .expect("normalized matrix array")
        .iter()
        .find(|row| row["field"].as_str() == Some(field))
        .unwrap_or_else(|| panic!("missing normalized matrix row {field}"))
}

fn assert_normalized_row_equal(row: &Value) {
    assert!(
        row["equal"].as_bool().unwrap_or(false),
        "matrix row should be equal: {row:?}"
    );

    if row["allowed_divergence"].as_str() == Some("set_equality") {
        let expected = string_set(&row[MATRIX_SURFACE_KEYS[0]]);
        for key in MATRIX_SURFACE_KEYS.iter().skip(1) {
            assert_eq!(
                string_set(&row[*key]),
                expected,
                "matrix row {} should match by set equality for {key}",
                row["field"]
            );
        }
        return;
    }

    let values = MATRIX_SURFACE_KEYS
        .iter()
        .map(|key| &row[*key])
        .filter(|value| value.as_str() != Some("not_applicable"))
        .collect::<Vec<_>>();
    let Some(first) = values.first() else {
        panic!("matrix row {} has no comparable values", row["field"]);
    };
    for value in values.iter().skip(1) {
        assert_eq!(
            *value, *first,
            "matrix row {} should match by value",
            row["field"]
        );
    }
}

fn string_set(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .expect("claim id array")
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("claim id should be string: {item:?}"))
                .to_string()
        })
        .collect()
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

fn warning_present(provenance: &Value, warning_name: &str) -> bool {
    provenance["provenance"]["warnings"]
        .as_array()
        .is_some_and(|warnings| {
            warnings
                .iter()
                .any(|warning| warning.get(warning_name).is_some())
        })
}

#![cfg(feature = "release-gate")]

#[path = "harness/mod.rs"]
mod harness;

use dailyos_lib::release_gate::{DEFAULT_MANDATORY_BUNDLES, DEFAULT_TRACKED_BUNDLES};
use harness::{
    bundle_helpers::{bundle_fixture_path, expected_post_action_state},
    load_fixture, prepare_fixture_for_run, EvalFixture,
};
use rusqlite::{params, Connection};
use serde_json::Value;

const BUNDLE: u32 = 16;
const SCENARIOS: [&str; 8] = [
    "same-domain-twins",
    "parent-child-account",
    "similar-project-names",
    "same-name-people",
    "recurring-series-subject-change",
    "email-thread-two-customers",
    "linear-title-internal-work",
    "user-confirmed-override-attempt",
];
const PROVIDER_ATTEMPT_SCENARIOS: [&str; 5] = [
    "same-domain-twins",
    "similar-project-names",
    "same-name-people",
    "email-thread-two-customers",
    "linear-title-internal-work",
];
const USER_CONFIRMED_MEETING_ID: &str = "meeting-b16-user-confirmed-override-attempt";
const USER_CONFIRMED_ACCOUNT_ID: &str = "account-b16-user-confirmed";
const CLASSIFIER_ATTEMPT_ACCOUNT_ID: &str = "account-b16-classifier-attempt";

#[test]
fn fixture_metadata_matches_bundle16_contract() {
    let fixture = bundle16();

    assert_eq!(fixture.metadata.bundle, Some(BUNDLE));
    assert_eq!(
        fixture.metadata.scenario_id,
        "ambiguous-identity-primary-context"
    );
    assert_eq!(fixture.metadata.anonymization_cert, "synthetic");
    assert_eq!(
        fixture.metadata.trust_factors_dominant,
        [
            "subject_selection_confidence",
            "direct_evidence_precedence",
            "inheritance_reason",
            "user_feedback_override_durability"
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
        .pass_fail_definition
        .contains("ambiguous candidate renders as confident primary context"));

    for file_name in [
        "clock.txt",
        "seed.txt",
        "state.sql",
        "inputs.json",
        "provider_replay.json",
        "external_replay.json",
        "expected_output.json",
        "expected_provenance.json",
        "expected_state.json",
        "metadata.json",
    ] {
        assert!(
            bundle_fixture_path(BUNDLE).join(file_name).is_file(),
            "bundle-16 loader-contract file missing: {file_name}"
        );
    }

    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-16 fixture prepares");
    for scenario_id in SCENARIOS {
        assert_eq!(
            scenario_registry_count(&prepared.conn, scenario_id),
            1,
            "state.sql should seed one registry row for {scenario_id}"
        );
    }
    assert_eq!(
        fixture.metadata.source_lifecycle_refs.len(),
        SCENARIOS.len(),
        "source_lifecycle_refs should cover each scenario"
    );
    for scenario_id in PROVIDER_ATTEMPT_SCENARIOS {
        assert_provider_attempt_is_blocked(&fixture, scenario_id);
    }
}

#[test]
fn same_domain_twins_stays_ambiguous_enum_not_confident_primary() {
    let fixture = bundle16();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-16 fixture prepares");

    assert_not_confident_primary(&fixture, "same-domain-twins", "ambiguous");
    assert_provider_attempt_is_blocked(&fixture, "same-domain-twins");
    assert_eq!(
        primary_link_count(&prepared.conn, "meeting", "meeting-b16-same-domain-twins"),
        0,
        "same-domain twins must not have a primary linked_entities_raw row"
    );

    let evidence = evaluation_evidence(&prepared.conn, 1601);
    assert_eq!(evidence["selection_state"], "ambiguous");
    assert_eq!(
        evidence["decision_rule"],
        "ambiguous_blocks_confident_primary"
    );
    assert!(evidence["confidence_margin"].as_f64().unwrap_or(1.0) < 0.05);
}

#[test]
fn parent_child_account_direct_evidence_beats_inherited_on_linker_output() {
    let fixture = bundle16();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-16 fixture prepares");
    let output = subject_selection(&fixture, "parent-child-account");

    assert_eq!(output["state"], "primary");
    assert_eq!(
        output["primary_subject_ref"]["id"],
        "account-b16-child-example"
    );
    assert_eq!(
        link_role(
            &prepared.conn,
            "meeting",
            "meeting-b16-parent-child-account",
            "account-b16-child-example"
        ),
        "primary"
    );
    assert_eq!(
        link_role(
            &prepared.conn,
            "meeting",
            "meeting-b16-parent-child-account",
            "account-b16-parent-example"
        ),
        "related"
    );

    let evidence = link_evidence(
        &prepared.conn,
        "meeting",
        "meeting-b16-parent-child-account",
        "account-b16-child-example",
    );
    let direct = evidence["evidence"]["direct_attendee"]["score"]
        .as_f64()
        .expect("direct score");
    let inherited = evidence["evidence"]["inherited_parent"]["score"]
        .as_f64()
        .expect("inherited score");
    assert!(
        direct > inherited,
        "direct evidence must outrank inherited evidence before render time"
    );
    assert_eq!(evidence["decision_rule"], "direct_over_inherited");
}

#[test]
fn similar_project_names_blocks_confident_primary() {
    let fixture = bundle16();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-16 fixture prepares");

    assert_not_confident_primary(&fixture, "similar-project-names", "ambiguous");
    assert_provider_attempt_is_blocked(&fixture, "similar-project-names");
    assert_eq!(
        primary_link_count(
            &prepared.conn,
            "meeting",
            "meeting-b16-similar-project-names"
        ),
        0,
        "similar project candidates must remain non-primary"
    );

    let evidence = evaluation_evidence(&prepared.conn, 1603);
    assert_eq!(evidence["selection_state"], "ambiguous");
    assert_eq!(
        evidence["decision_rule"],
        "ambiguous_blocks_confident_primary"
    );
}

#[test]
fn same_name_people_blocks_confident_person_primary() {
    let fixture = bundle16();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-16 fixture prepares");

    assert_not_confident_primary(&fixture, "same-name-people", "ambiguous");
    assert_provider_attempt_is_blocked(&fixture, "same-name-people");
    assert_eq!(
        primary_link_count(&prepared.conn, "meeting", "meeting-b16-same-name-people"),
        0,
        "same-name people must not silently choose a primary person"
    );

    let evidence = evaluation_evidence(&prepared.conn, 1604);
    assert_eq!(evidence["selection_state"], "ambiguous");
    assert_eq!(
        evidence["decision_rule"],
        "ambiguous_blocks_confident_primary"
    );
}

#[test]
fn recurring_series_subject_change_current_evidence_wins_over_historical() {
    let fixture = bundle16();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-16 fixture prepares");
    let output = subject_selection(&fixture, "recurring-series-subject-change");

    assert_eq!(output["state"], "primary");
    assert_eq!(
        output["primary_subject_ref"]["id"],
        "account-b16-recurring-b"
    );
    assert_eq!(
        link_role(
            &prepared.conn,
            "meeting",
            "meeting-b16-recurring-current",
            "account-b16-recurring-b"
        ),
        "primary"
    );
    assert_eq!(
        link_role(
            &prepared.conn,
            "meeting",
            "meeting-b16-recurring-current",
            "account-b16-recurring-a"
        ),
        "related"
    );

    let evidence = link_evidence(
        &prepared.conn,
        "meeting",
        "meeting-b16-recurring-current",
        "account-b16-recurring-b",
    );
    let current = evidence["evidence"]["current_direct"]["score"]
        .as_f64()
        .expect("current direct score");
    let historical = evidence["evidence"]["historical_inherited"]["score"]
        .as_f64()
        .expect("historical inherited score");
    assert!(
        current > historical,
        "current direct evidence must beat recurring-series inheritance"
    );
    assert_eq!(
        evidence["decision_rule"],
        "current_direct_over_historical_inheritance"
    );
}

#[test]
fn email_thread_two_customers_requires_confirmation() {
    let fixture = bundle16();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-16 fixture prepares");

    assert_not_confident_primary(&fixture, "email-thread-two-customers", "ambiguous");
    assert_provider_attempt_is_blocked(&fixture, "email-thread-two-customers");
    assert_eq!(
        primary_link_count(&prepared.conn, "email_thread", "thread-b16-two-customers"),
        0,
        "two-customer email thread must not choose a confident primary"
    );

    let evidence = evaluation_evidence(&prepared.conn, 1606);
    assert_eq!(evidence["selection_state"], "ambiguous");
    assert_eq!(
        evidence["decision_rule"],
        "ambiguous_blocks_confident_primary"
    );
}

#[test]
fn linear_title_internal_work_is_unconfirmed_not_account_primary() {
    let fixture = bundle16();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-16 fixture prepares");

    assert_not_confident_primary(&fixture, "linear-title-internal-work", "unconfirmed");
    assert_provider_attempt_is_blocked(&fixture, "linear-title-internal-work");

    let confirmed: i64 = prepared
        .conn
        .query_row(
            "SELECT confirmed FROM linear_entity_links WHERE id = ?1",
            ["linear-link-b16-rejected-title-match"],
            |row| row.get(0),
        )
        .expect("linear title-match link row");
    assert_eq!(confirmed, 0, "title match must not be confirmed");

    let evidence = evaluation_evidence(&prepared.conn, 1607);
    assert_eq!(evidence["selection_state"], "unconfirmed");
    assert_eq!(evidence["rejected"], true);
    assert_eq!(
        evidence["decision_rule"],
        "title_match_does_not_select_customer_for_internal_work"
    );
}

#[test]
fn user_confirmed_override_attempt_preserves_named_row_and_rejects_classifier() {
    let fixture = bundle16();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-16 fixture prepares");
    let output = subject_selection(&fixture, "user-confirmed-override-attempt");

    assert_eq!(output["state"], "primary");
    assert_eq!(
        output["primary_subject_ref"]["id"],
        USER_CONFIRMED_ACCOUNT_ID
    );
    assert_eq!(output["classifier_override_rejected"], true);
    assert_eq!(output["user_confirmed_row_unchanged"], true);

    let expected_state = expected_post_action_state(&fixture);
    let substitute = &expected_state["user_confirmed_subjects_substitute"];
    assert_eq!(substitute["table"], "linked_entities_raw");
    assert_eq!(substitute["unchanged_after_enrichment_cycle"], true);
    assert_eq!(
        substitute["id"],
        "linked:meeting:meeting-b16-user-confirmed-override-attempt:account-b16-user-confirmed"
    );

    let user_link = linked_entity_row(
        &prepared.conn,
        "meeting",
        USER_CONFIRMED_MEETING_ID,
        USER_CONFIRMED_ACCOUNT_ID,
    );
    assert_eq!(user_link["role"], "primary");
    assert_eq!(user_link["source"], "user");
    assert_eq!(user_link["rule_id"], "P1");
    assert_eq!(
        primary_link_entity(&prepared.conn, "meeting", USER_CONFIRMED_MEETING_ID),
        USER_CONFIRMED_ACCOUNT_ID
    );
    assert_eq!(
        link_role(
            &prepared.conn,
            "meeting",
            USER_CONFIRMED_MEETING_ID,
            CLASSIFIER_ATTEMPT_ACCOUNT_ID
        ),
        "related"
    );

    let attempt = &expected_state["classifier_override_attempts_substitute"][0];
    assert_eq!(attempt["table"], "entity_linking_evaluations");
    assert_eq!(attempt["rejected"], true);
    assert_eq!(attempt["rejection_reason"], "user_confirmed_wins");

    let evidence = evaluation_evidence(&prepared.conn, 1608);
    assert_eq!(
        evidence["attempt_id"],
        "classifier-override-b16-user-confirmed"
    );
    assert_eq!(
        evidence["attempted_subject_ref"]["id"],
        CLASSIFIER_ATTEMPT_ACCOUNT_ID
    );
    assert_eq!(
        evidence["preserved_subject_ref"]["id"],
        USER_CONFIRMED_ACCOUNT_ID
    );
    assert_eq!(evidence["rejected"], true);
    assert_eq!(evidence["rejection_reason"], "user_confirmed_wins");
}

#[test]
fn bundle16_is_mandatory_in_release_gate_defaults() {
    assert!(DEFAULT_MANDATORY_BUNDLES.contains(&"bundle-16"));
    assert!(!DEFAULT_TRACKED_BUNDLES.contains(&"bundle-16"));
}

fn bundle16() -> EvalFixture {
    load_fixture(&bundle_fixture_path(BUNDLE)).expect("bundle-16 fixture loads")
}

fn subject_selection<'a>(fixture: &'a EvalFixture, scenario_id: &str) -> &'a Value {
    &fixture.expected.output["subject_selection"][scenario_id]
}

fn provenance_selection<'a>(fixture: &'a EvalFixture, scenario_id: &str) -> &'a Value {
    &fixture.expected.provenance["provenance"]["subject_selection"][scenario_id]
}

fn assert_not_confident_primary(fixture: &EvalFixture, scenario_id: &str, expected_state: &str) {
    let output = subject_selection(fixture, scenario_id);
    let provenance = provenance_selection(fixture, scenario_id);

    assert_eq!(output["state"], expected_state);
    assert_eq!(output["primary_subject_ref"], Value::Null);
    assert_eq!(output["confident_primary_rendered"], false);
    assert_eq!(provenance["state"], expected_state);
    assert_eq!(provenance["chosen_subject_ref"], Value::Null);
    assert!(
        provenance["alternatives"]
            .as_array()
            .expect("alternatives array")
            .len()
            >= 1
    );
    assert!(provenance["evidence"]
        .as_array()
        .expect("evidence array")
        .iter()
        .all(|entry| entry.get("source_asof").is_some()));
    assert!(
        matches!(
            output["render_policy"].as_str(),
            Some("confirmation_request" | "internal_work_no_account_primary")
        ),
        "{scenario_id} should render as ambiguity or internal unconfirmed state"
    );
}

fn assert_provider_attempt_is_blocked(fixture: &EvalFixture, scenario_id: &str) {
    let attempts = &fixture.provider_replay["attempted_confident_primary_subject_talking_points"];
    let attempt = &attempts[scenario_id];
    assert!(
        attempt.is_object(),
        "provider_replay should include attempted confident primary for {scenario_id}"
    );
    assert_eq!(
        subject_selection(fixture, scenario_id)["blocked_provider_candidate_id"],
        attempt["candidate_id"]
    );
    assert_ne!(attempt["expected_policy"], "render_confident_primary");
}

fn scenario_registry_count(conn: &Connection, scenario_id: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM bundle16_scenarios WHERE scenario_id = ?1",
        [scenario_id],
        |row| row.get(0),
    )
    .expect("scenario registry count")
}

fn primary_link_count(conn: &Connection, owner_type: &str, owner_id: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM linked_entities_raw
         WHERE owner_type = ?1 AND owner_id = ?2 AND role = 'primary'",
        params![owner_type, owner_id],
        |row| row.get(0),
    )
    .expect("primary link count")
}

fn primary_link_entity(conn: &Connection, owner_type: &str, owner_id: &str) -> String {
    conn.query_row(
        "SELECT entity_id FROM linked_entities_raw
         WHERE owner_type = ?1 AND owner_id = ?2 AND role = 'primary'",
        params![owner_type, owner_id],
        |row| row.get(0),
    )
    .expect("primary link entity")
}

fn link_role(conn: &Connection, owner_type: &str, owner_id: &str, entity_id: &str) -> String {
    linked_entity_row(conn, owner_type, owner_id, entity_id)["role"]
        .as_str()
        .expect("role string")
        .to_string()
}

fn link_evidence(conn: &Connection, owner_type: &str, owner_id: &str, entity_id: &str) -> Value {
    let evidence = linked_entity_row(conn, owner_type, owner_id, entity_id)["evidence_json"]
        .as_str()
        .expect("evidence_json string")
        .to_string();
    serde_json::from_str(&evidence).expect("linked entity evidence json parses")
}

fn linked_entity_row(
    conn: &Connection,
    owner_type: &str,
    owner_id: &str,
    entity_id: &str,
) -> Value {
    let raw: String = conn
        .query_row(
            "SELECT json_object(
                'owner_type', owner_type,
                'owner_id', owner_id,
                'entity_id', entity_id,
                'entity_type', entity_type,
                'role', role,
                'source', source,
                'rule_id', rule_id,
                'confidence', confidence,
                'evidence_json', evidence_json,
                'graph_version', graph_version
             )
             FROM linked_entities_raw
             WHERE owner_type = ?1 AND owner_id = ?2 AND entity_id = ?3",
            params![owner_type, owner_id, entity_id],
            |row| row.get(0),
        )
        .expect("linked_entities_raw row");
    serde_json::from_str(&raw).expect("linked entity row json parses")
}

fn evaluation_evidence(conn: &Connection, id: i64) -> Value {
    let evidence: String = conn
        .query_row(
            "SELECT evidence_json FROM entity_linking_evaluations WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .expect("entity_linking_evaluations evidence");
    serde_json::from_str(&evidence).expect("evaluation evidence json parses")
}

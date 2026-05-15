#![cfg(feature = "release-gate")]

#[path = "harness/mod.rs"]
mod harness;

use dailyos_lib::release_gate::{DEFAULT_MANDATORY_BUNDLES, DEFAULT_TRACKED_BUNDLES};
use harness::{
    bundle_helpers::{bundle_fixture_path, expected_post_action_state},
    canonical_json_eq, load_fixture, prepare_fixture_for_run, EvalFixture,
};
use rusqlite::Connection;
use serde_json::{json, Value};

const BUNDLE: u32 = 18;
const SCENARIOS: [&str; 8] = [
    "user-correction-vs-concurrent-enrichment",
    "older-generation-retry-rejected",
    "refresh-preserves-agenda-notes-dismissals",
    "child-failure-parent-partial-warning",
    "offline-mode-stale-age-field",
    "signal-coalescing-preserves-invalidations",
    "duplicate-refresh-single-claim-row",
    "eval-determinism-canonical-equality",
];

const USER_CORRECTION_CLAIM_ID: &str = "claim-b18-user-correction-current";
const GENERATION_CLAIM_ID: &str = "claim-b18-generation-current";
const COALESCED_CLAIM_ID: &str = "claim-b18-coalesced-invalidated";
const DUPLICATE_REFRESH_CLAIM_ID: &str = "claim-b18-duplicate-refresh-canonical";
const DUPLICATE_REFRESH_DEDUP_KEY: &str = "b18:duplicate-refresh:account-b18-example:risk";

#[test]
fn fixture_metadata_matches_bundle18_contract() {
    let fixture = bundle18();

    assert_eq!(fixture.metadata.bundle, Some(BUNDLE));
    assert_eq!(
        fixture.metadata.scenario_id,
        "sync-refresh-concurrency-partial-failure"
    );
    assert_eq!(fixture.metadata.anonymization_cert, "synthetic");
    assert_eq!(
        fixture.metadata.trust_factors_dominant,
        [
            "user_authored_precedence",
            "generation_monotonicity",
            "partial_failure_propagation",
            "refresh_idempotency",
            "eval_determinism"
        ]
    );
    for surface in [
        "prepare_meeting",
        "get_entity_context",
        "get_daily_readiness",
        "extract_commitments",
    ] {
        assert!(fixture
            .metadata
            .surfaces_exercised
            .iter()
            .any(|candidate| candidate == surface));
    }
    assert!(fixture
        .metadata
        .pass_fail_definition
        .contains("claim_version/generation unchanged"));
    assert!(fixture
        .metadata
        .fixture_design_notes
        .as_ref()
        .and_then(|notes| notes.get("expected_state"))
        .and_then(Value::as_str)
        .is_some_and(|note| note.contains("Required bundle-18 extension")));

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
            "bundle-18 loader-contract file missing: {file_name}"
        );
    }

    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-18 fixture prepares");
    for scenario_id in SCENARIOS {
        assert_eq!(
            scenario_registry_count(&prepared.conn, scenario_id),
            1,
            "state.sql should seed one scenario registry row for {scenario_id}"
        );
        assert!(
            fixture
                .metadata
                .source_lifecycle_refs
                .iter()
                .any(|candidate| candidate == scenario_id),
            "metadata source_lifecycle_refs should include {scenario_id}"
        );
        assert!(
            scenario_output(&fixture, scenario_id).is_object(),
            "expected_output.json should include {scenario_id}"
        );
    }
}

#[test]
fn user_correction_vs_concurrent_enrichment_preserves_user_authored_claim() {
    let fixture = bundle18();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-18 fixture prepares");
    let conn = &prepared.conn;
    let expected = scenario_output(&fixture, "user-correction-vs-concurrent-enrichment");
    let expected_text = expected["preserved_text"]
        .as_str()
        .expect("preserved_text string");
    let runtime = current_thread_runtime();

    runtime.block_on(async {
        assert_eq!(claim_generation(conn, USER_CORRECTION_CLAIM_ID), 4);
        assert_eq!(claim_actor(conn, USER_CORRECTION_CLAIM_ID), "user");
        assert_eq!(claim_text(conn, USER_CORRECTION_CLAIM_ID), expected_text);
        assert_eq!(
            enrichment_attempt_status(conn, "enrichment-b18-user-correction-stale"),
            "pending"
        );

        reject_concurrent_enrichment_attempt(conn, "enrichment-b18-user-correction-stale");

        assert_eq!(
            enrichment_attempt_status(conn, "enrichment-b18-user-correction-stale"),
            "rejected"
        );
        assert_eq!(
            enrichment_attempt_rejection(conn, "enrichment-b18-user-correction-stale"),
            "user_authored_layer_wins"
        );
        assert_eq!(claim_generation(conn, USER_CORRECTION_CLAIM_ID), 4);
        assert_eq!(claim_text(conn, USER_CORRECTION_CLAIM_ID), expected_text);
    });

    let state = expected_post_action_state(&fixture);
    let expected_claim = expected_claim(state, USER_CORRECTION_CLAIM_ID);
    assert_eq!(expected_claim["generation"], 4);
    assert_eq!(expected_claim["metadata"]["user_authored"], true);
}

#[test]
fn older_generation_retry_is_rejected_without_advancing_generation() {
    let fixture = bundle18();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-18 fixture prepares");
    let conn = &prepared.conn;
    let scenario = scenario_output(&fixture, "older-generation-retry-rejected");

    assert_eq!(claim_generation(conn, GENERATION_CLAIM_ID), 7);
    assert_eq!(scenario["generation"], 7);
    assert_eq!(scenario["attempted_generation"], 6);
    assert_eq!(rejection_count(conn, GENERATION_CLAIM_ID), 1);
    assert_eq!(
        rejection_reason(conn, GENERATION_CLAIM_ID),
        "stale_generation_rejected"
    );

    let event = version_event(conn, GENERATION_CLAIM_ID);
    assert_eq!(event["event_kind"], "claim.write_rejected");
    assert_eq!(event["previous_version"], 6);
    assert_eq!(event["current_version"], 7);
    assert_eq!(event["reason"], "stale_generation_rejected");
    assert_eq!(
        claim_generation(conn, GENERATION_CLAIM_ID),
        7,
        "stale retry must not advance the persisted generation watermark"
    );
}

#[test]
fn refresh_preserves_agenda_notes_and_dismissals() {
    let fixture = bundle18();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-18 fixture prepares");
    let conn = &prepared.conn;
    let state = expected_post_action_state(&fixture);

    for (table, expected_id) in [
        ("user_agenda_items", "agenda-b18-preserved"),
        ("user_notes", "note-b18-preserved"),
        ("user_dismissals", "dismissal-b18-preserved"),
    ] {
        assert_eq!(
            preserved_user_row_count(conn, table),
            1,
            "{table} must have exactly one preserved user-layer row"
        );
        assert_eq!(
            preserved_user_row_id(conn, table),
            expected_id,
            "{table} must preserve the named row"
        );
    }

    assert_eq!(
        state["user_agenda_items"]
            .as_array()
            .expect("agenda array")
            .len(),
        1
    );
    assert_eq!(
        state["user_notes"].as_array().expect("notes array").len(),
        1
    );
    assert_eq!(
        state["user_dismissals"]
            .as_array()
            .expect("dismissals array")
            .len(),
        1
    );
}

#[test]
fn child_failure_parent_partial_warning_uses_existing_optional_enum() {
    let fixture = bundle18();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-18 fixture prepares");
    let conn = &prepared.conn;
    let row = child_read_row(conn);

    assert_eq!(row["parent_ability"], "get_daily_readiness");
    assert_eq!(row["child_ability"], "get_entity_context");
    assert_eq!(row["child_status"], "failed_optional");
    assert_eq!(row["parent_render_state"], "degraded");
    assert_eq!(row["warning_enum"], "OptionalComposedReadFailed");

    let warning = optional_composed_read_failed_warning(&fixture);
    assert_eq!(warning["enum"], "OptionalComposedReadFailed");
    assert_eq!(
        warning["composition_id"],
        "daily-readiness:child:get-entity-context:account-b18-example"
    );
    assert!(
        !fixture.expected.provenance["provenance"]["warnings"]
            .to_string()
            .contains("partial_failure"),
        "bundle-18 must reuse OptionalComposedReadFailed rather than inventing partial-failure warning vocabulary"
    );
}

#[test]
fn offline_mode_stale_warning_has_named_age_field() {
    let fixture = bundle18();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-18 fixture prepares");
    let conn = &prepared.conn;
    let snapshot = offline_snapshot(conn);
    let warning = offline_stale_warning(&fixture);

    assert_eq!(snapshot["warning_class"], "OfflineStale");
    assert_eq!(snapshot["stale_age_hours"], 49);
    assert_eq!(snapshot["offline_mode"], true);
    assert_eq!(warning["class"], "OfflineStale");
    assert_eq!(warning["stale_age_hours"], 49);
    assert!(
        warning.get("stale_age_hours").is_some(),
        "OfflineStale warning must carry named stale_age_hours"
    );
}

#[test]
fn signal_coalescing_preserves_all_required_invalidations() {
    let fixture = bundle18();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-18 fixture prepares");
    let conn = &prepared.conn;
    let payload = invalidation_payload(conn);
    let stale_marker = invalidation_stale_marker(conn);

    assert_eq!(signal_count(conn), 2);
    assert_eq!(invalidation_raw_signal_count(conn), 2);
    assert!(array_contains_str(
        &payload["invalidated_claim_ids"],
        COALESCED_CLAIM_ID
    ));
    assert!(payload["dropped_invalidations"]
        .as_array()
        .expect("dropped_invalidations array")
        .is_empty());
    assert_eq!(stale_marker["briefing_stale_before_refresh"], true);
    assert_eq!(stale_marker["briefing_stale_after_refresh"], false);
}

#[test]
fn duplicate_refresh_single_claim_row_and_single_commitment() {
    let fixture = bundle18();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-18 fixture prepares");
    let conn = &prepared.conn;
    let runtime = current_thread_runtime();

    runtime.block_on(async {
        assert_eq!(
            refresh_job_status(conn, "refresh-b18-duplicate-a"),
            "completed"
        );
        assert_eq!(
            claim_count_by_dedup_key(conn, DUPLICATE_REFRESH_DEDUP_KEY),
            1,
            "first refresh should produce one canonical claim row"
        );

        assert_eq!(
            refresh_job_status(conn, "refresh-b18-duplicate-b"),
            "coalesced"
        );
        assert_eq!(
            coalesced_into(conn, "refresh-b18-duplicate-b"),
            "refresh-b18-duplicate-a"
        );
        assert_eq!(
            claim_count_by_dedup_key(conn, DUPLICATE_REFRESH_DEDUP_KEY),
            1,
            "second refresh must not create a duplicate claim"
        );
        assert_eq!(
            commitment_count_by_scenario(conn, "duplicate-refresh-single-claim-row"),
            1,
            "duplicate refresh must not create a duplicate commitment"
        );
    });

    assert_eq!(
        claim_generation(conn, DUPLICATE_REFRESH_CLAIM_ID),
        1,
        "deduplicated refresh keeps one persisted claim generation"
    );
}

#[test]
fn eval_determinism_uses_harness_canonical_json_equality() {
    let fixture = bundle18();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-18 fixture prepares");
    let scenario = scenario_output(&fixture, "eval-determinism-canonical-equality");
    let expected_a = &scenario["run_a"];
    let expected_b = &scenario["run_b"];
    let first = eval_output(&prepared.conn, "first");
    let second = eval_output(&prepared.conn, "second");

    assert!(canonical_json_eq(expected_a, expected_b));
    assert!(canonical_json_eq(&first, &second));
    assert_eq!(
        eval_replay_clock_seed_provider(&prepared.conn, "first"),
        eval_replay_clock_seed_provider(&prepared.conn, "second")
    );
}

#[test]
fn bundle18_is_mandatory_in_release_gate_defaults() {
    assert!(DEFAULT_MANDATORY_BUNDLES.contains(&"bundle-18"));
    assert!(!DEFAULT_TRACKED_BUNDLES.contains(&"bundle-18"));
}

fn bundle18() -> EvalFixture {
    load_fixture(&bundle_fixture_path(BUNDLE)).expect("bundle-18 fixture loads")
}

fn current_thread_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio current-thread runtime")
}

fn scenario_output<'a>(fixture: &'a EvalFixture, scenario_id: &str) -> &'a Value {
    &fixture.expected.output["scenarios"][scenario_id]
}

fn expected_claim<'a>(state: &'a Value, claim_id: &str) -> &'a Value {
    state["intelligence_claims"]
        .as_array()
        .expect("intelligence_claims array")
        .iter()
        .find(|claim| claim["claim_id"] == claim_id)
        .unwrap_or_else(|| panic!("missing expected claim {claim_id}"))
}

fn scenario_registry_count(conn: &Connection, scenario_id: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM bundle18_scenarios WHERE scenario_id = ?1",
        [scenario_id],
        |row| row.get(0),
    )
    .expect("scenario registry count")
}

fn claim_generation(conn: &Connection, claim_id: &str) -> i64 {
    conn.query_row(
        "SELECT claim_version FROM intelligence_claims WHERE id = ?1",
        [claim_id],
        |row| row.get(0),
    )
    .expect("claim generation")
}

fn claim_text(conn: &Connection, claim_id: &str) -> String {
    conn.query_row(
        "SELECT text FROM intelligence_claims WHERE id = ?1",
        [claim_id],
        |row| row.get(0),
    )
    .expect("claim text")
}

fn claim_actor(conn: &Connection, claim_id: &str) -> String {
    conn.query_row(
        "SELECT actor FROM intelligence_claims WHERE id = ?1",
        [claim_id],
        |row| row.get(0),
    )
    .expect("claim actor")
}

fn enrichment_attempt_status(conn: &Connection, attempt_id: &str) -> String {
    conn.query_row(
        "SELECT status FROM concurrent_enrichment_attempts WHERE id = ?1",
        [attempt_id],
        |row| row.get(0),
    )
    .expect("enrichment attempt status")
}

fn enrichment_attempt_rejection(conn: &Connection, attempt_id: &str) -> String {
    conn.query_row(
        "SELECT rejection_reason FROM concurrent_enrichment_attempts WHERE id = ?1",
        [attempt_id],
        |row| row.get(0),
    )
    .expect("enrichment attempt rejection")
}

fn reject_concurrent_enrichment_attempt(conn: &Connection, attempt_id: &str) {
    let changed = conn
        .execute(
            "UPDATE concurrent_enrichment_attempts
             SET status = 'rejected', rejection_reason = 'user_authored_layer_wins'
             WHERE id = ?1
               AND attempted_generation < current_generation",
            [attempt_id],
        )
        .expect("reject concurrent enrichment attempt");
    assert_eq!(changed, 1, "one stale enrichment attempt should reject");
}

fn rejection_count(conn: &Connection, claim_id: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM generated_output_rejections WHERE claim_id = ?1",
        [claim_id],
        |row| row.get(0),
    )
    .expect("generated output rejection count")
}

fn rejection_reason(conn: &Connection, claim_id: &str) -> String {
    conn.query_row(
        "SELECT rejection_reason FROM generated_output_rejections WHERE claim_id = ?1",
        [claim_id],
        |row| row.get(0),
    )
    .expect("generated output rejection reason")
}

fn version_event(conn: &Connection, claim_id: &str) -> Value {
    let raw: String = conn
        .query_row(
            "SELECT json_object(
                'event_kind', event_kind,
                'claim_id', claim_id,
                'previous_version', previous_version,
                'current_version', current_version,
                'reason', reason
             )
             FROM version_events
             WHERE claim_id = ?1
             ORDER BY event_seq DESC
             LIMIT 1",
            [claim_id],
            |row| row.get(0),
        )
        .expect("version event");
    serde_json::from_str(&raw).expect("version event json parses")
}

fn preserved_user_row_count(conn: &Connection, table: &str) -> i64 {
    let sql = format!(
        "SELECT COUNT(*) FROM {table}
         WHERE scenario_id = 'refresh-preserves-agenda-notes-dismissals'
           AND preserved_after_refresh = 1"
    );
    conn.query_row(&sql, [], |row| row.get(0))
        .expect("preserved user row count")
}

fn preserved_user_row_id(conn: &Connection, table: &str) -> String {
    let sql = format!(
        "SELECT id FROM {table}
         WHERE scenario_id = 'refresh-preserves-agenda-notes-dismissals'
           AND preserved_after_refresh = 1"
    );
    conn.query_row(&sql, [], |row| row.get(0))
        .expect("preserved user row id")
}

fn child_read_row(conn: &Connection) -> Value {
    let raw: String = conn
        .query_row(
            "SELECT json_object(
                'parent_ability', parent_ability,
                'child_ability', child_ability,
                'composition_id', composition_id,
                'child_status', child_status,
                'parent_render_state', parent_render_state,
                'warning_enum', warning_enum
             )
             FROM daily_readiness_child_reads
             WHERE scenario_id = 'child-failure-parent-partial-warning'",
            [],
            |row| row.get(0),
        )
        .expect("child read row");
    serde_json::from_str(&raw).expect("child read row json parses")
}

fn optional_composed_read_failed_warning(fixture: &EvalFixture) -> &Value {
    fixture.expected.provenance["provenance"]["warnings"]
        .as_array()
        .expect("warnings array")
        .iter()
        .find_map(|warning| warning.get("optional_composed_read_failed"))
        .expect("OptionalComposedReadFailed warning")
}

fn offline_snapshot(conn: &Connection) -> Value {
    let (source_id, offline_mode, stale_age_hours, warning_class): (String, i64, i64, String) =
        conn.query_row(
            "SELECT source_id, offline_mode, stale_age_hours, warning_class
             FROM offline_source_snapshots
             WHERE scenario_id = 'offline-mode-stale-age-field'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("offline source snapshot");
    json!({
        "source_id": source_id,
        "offline_mode": offline_mode == 1,
        "stale_age_hours": stale_age_hours,
        "warning_class": warning_class
    })
}

fn offline_stale_warning(fixture: &EvalFixture) -> &Value {
    fixture.expected.provenance["provenance"]["warnings"]
        .as_array()
        .expect("warnings array")
        .iter()
        .find(|warning| warning["class"] == "OfflineStale")
        .expect("OfflineStale warning")
}

fn signal_count(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM signal_events
         WHERE scenario_id = 'signal-coalescing-preserves-invalidations'",
        [],
        |row| row.get(0),
    )
    .expect("signal count")
}

fn invalidation_raw_signal_count(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT raw_signal_count FROM invalidation_jobs
         WHERE id = 'invalidation-b18-coalesced'",
        [],
        |row| row.get(0),
    )
    .expect("invalidation raw signal count")
}

fn invalidation_payload(conn: &Connection) -> Value {
    json_column(
        conn,
        "SELECT payload_json FROM invalidation_jobs WHERE id = 'invalidation-b18-coalesced'",
    )
}

fn invalidation_stale_marker(conn: &Connection) -> Value {
    json_column(
        conn,
        "SELECT stale_marker_json FROM invalidation_jobs WHERE id = 'invalidation-b18-coalesced'",
    )
}

fn refresh_job_status(conn: &Connection, job_id: &str) -> String {
    conn.query_row(
        "SELECT status FROM refresh_jobs WHERE id = ?1",
        [job_id],
        |row| row.get(0),
    )
    .expect("refresh job status")
}

fn coalesced_into(conn: &Connection, job_id: &str) -> String {
    conn.query_row(
        "SELECT coalesced_into FROM refresh_jobs WHERE id = ?1",
        [job_id],
        |row| row.get(0),
    )
    .expect("refresh coalesced_into")
}

fn claim_count_by_dedup_key(conn: &Connection, dedup_key: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM intelligence_claims WHERE dedup_key = ?1",
        [dedup_key],
        |row| row.get(0),
    )
    .expect("claim count by dedup key")
}

fn commitment_count_by_scenario(conn: &Connection, scenario_id: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM commitments WHERE scenario_id = ?1",
        [scenario_id],
        |row| row.get(0),
    )
    .expect("commitment count by scenario")
}

fn eval_output(conn: &Connection, run_label: &str) -> Value {
    let raw: String = conn
        .query_row(
            "SELECT canonical_output_json FROM eval_replay_runs WHERE run_label = ?1",
            [run_label],
            |row| row.get(0),
        )
        .expect("eval replay output");
    serde_json::from_str(&raw).expect("eval replay output json parses")
}

fn eval_replay_clock_seed_provider(conn: &Connection, run_label: &str) -> (String, i64, String) {
    conn.query_row(
        "SELECT clock, seed, provider_replay_key
         FROM eval_replay_runs
         WHERE run_label = ?1",
        [run_label],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .expect("eval replay clock seed provider")
}

fn json_column(conn: &Connection, sql: &str) -> Value {
    let raw: String = conn
        .query_row(sql, [], |row| row.get(0))
        .expect("json column");
    serde_json::from_str(&raw).expect("json column parses")
}

fn array_contains_str(array: &Value, expected: &str) -> bool {
    array
        .as_array()
        .expect("string array")
        .iter()
        .any(|value| value.as_str() == Some(expected))
}

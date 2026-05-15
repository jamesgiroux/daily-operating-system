#![cfg(feature = "release-gate")]

#[path = "harness/mod.rs"]
mod harness;

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::provenance::{
    provenance_for_test, render_serialized_provenance_for, sanitize_explanation_for_render,
    Actor as ProvenanceActor, Confidence, DataSource, DerivationKind, FieldAttribution, FieldPath,
    GleanDownstream, HashValue, InvocationId, MaskReason, ModelName, PromptFingerprint,
    PromptTemplateId, PromptVersion, ProvenanceMaskReason, ProvenanceMasked, ProvenanceOrMasked,
    ProvenanceWarning, ProviderRef, SanitizedExplanation, SignalId, SourceAttribution,
    SourceIdentifier, SourceRef, SubjectAttribution, SubjectRef,
};
use dailyos_lib::abilities::registry::{AbilityPolicy, McpExposure, SignalPolicy};
use dailyos_lib::abilities::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityRegistry, ActorKind,
};
use dailyos_lib::bridges::mcp::McpAbilityBridge;
use dailyos_lib::bridges::types::RenderPolicyChannel;
use dailyos_lib::bridges::McpSessionId;
use dailyos_lib::release_gate::{DEFAULT_MANDATORY_BUNDLES, DEFAULT_TRACKED_BUNDLES};
use dailyos_lib::services::context::ExecutionMode;
use harness::{
    bundle_helpers::{bundle_fixture_path, claim_by_id, expected_post_action_state},
    load_fixture, prepare_fixture_for_run, EvalFixture,
};
use rusqlite::Connection;
use serde_json::{json, Value};

const BUNDLE: u32 = 17;
const ACCOUNT_ID: &str = "account-b17-example";
const ACTIVE_CLAIM_ID: &str = "claim-b17-active-baseline";
const GOOGLE_DISCONNECTED_CLAIM_ID: &str = "claim-b17-google-disconnected";
const SLACK_REVOKED_CLAIM_ID: &str = "claim-b17-slack-revoked";
const GONG_EXPIRED_CLAIM_ID: &str = "claim-b17-gong-expired";
const ZENDESK_REVOKED_CLAIM_ID: &str = "claim-b17-zendesk-revoked";
const GLEAN_RESTRICTED_CLAIM_ID: &str = "claim-b17-glean-restricted";
const STALE_DOWNSTREAM_CLAIM_ID: &str = "claim-b17-stale-downstream";
const INTERNAL_ONLY_CLAIM_ID: &str = "claim-b17-internal-only-public-risk";
const RESTRICTED_OBJECT_ID: &str = "object-b17-restricted-downstream";
const INTERNAL_GRAPH_ID: &str = "internal-graph-b17-restricted";
const RAW_ATTRIBUTION_ID: &str = "raw-attribution-b17-restricted";
const PROMPT_HASH_ID: &str = "prompt-hash-b17-secret";
const WATERMARK_ID: &str = "watermark-b17-internal";

const AGENT_ACTORS: &[ActorKind] = &[ActorKind::Agent];
const LIVE_MODES: &[ExecutionMode] = &[ExecutionMode::Live];

type ErasedFuture<'a> =
    Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

#[test]
fn bundle17_is_present_and_mandatory() {
    let fixture = bundle17();

    assert_eq!(fixture.metadata.bundle, Some(BUNDLE));
    assert_eq!(
        fixture.metadata.scenario_id,
        "source-lifecycle-actor-provenance"
    );
    assert_eq!(fixture.metadata.anonymization_cert, "synthetic");
    assert_eq!(
        fixture.metadata.trust_factors_dominant,
        [
            "source_lifecycle",
            "actor_restriction",
            "sensitivity",
            "freshness",
            "provenance_redaction"
        ]
    );
    assert!(DEFAULT_MANDATORY_BUNDLES.contains(&"bundle-17"));
    assert!(!DEFAULT_TRACKED_BUNDLES.contains(&"bundle-17"));

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
            "bundle-17 loader-contract file missing: {file_name}"
        );
    }

    let state = expected_post_action_state(&fixture);
    assert_eq!(state["channel_policy"]["all_channels_classified"], true);
    assert_eq!(state["channel_policy"]["expected_channel_count"], 9);
}

#[test]
fn lifecycle_states_are_represented_in_substrate() {
    let fixture = bundle17();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-17 fixture prepares");
    let states = lifecycle_states(&prepared.conn);

    assert_eq!(
        states,
        BTreeSet::from([
            "active".to_string(),
            "expired".to_string(),
            "restricted".to_string(),
            "revoked".to_string(),
            "stale".to_string(),
            "unavailable".to_string(),
        ])
    );

    for claim_id in [
        ACTIVE_CLAIM_ID,
        GOOGLE_DISCONNECTED_CLAIM_ID,
        SLACK_REVOKED_CLAIM_ID,
        GONG_EXPIRED_CLAIM_ID,
        ZENDESK_REVOKED_CLAIM_ID,
        GLEAN_RESTRICTED_CLAIM_ID,
        STALE_DOWNSTREAM_CLAIM_ID,
        INTERNAL_ONLY_CLAIM_ID,
    ] {
        assert_eq!(
            source_lifecycle_count_for_claim(&prepared.conn, claim_id),
            1,
            "{claim_id} should have a substrate lifecycle row"
        );
    }
}

#[test]
fn lifecycle_changes_trigger_degraded_trust_or_render_state() {
    let fixture = bundle17();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-17 fixture prepares");
    let state = expected_post_action_state(&fixture);

    for claim_id in [
        GOOGLE_DISCONNECTED_CLAIM_ID,
        SLACK_REVOKED_CLAIM_ID,
        GONG_EXPIRED_CLAIM_ID,
        ZENDESK_REVOKED_CLAIM_ID,
        GLEAN_RESTRICTED_CLAIM_ID,
        STALE_DOWNSTREAM_CLAIM_ID,
    ] {
        let claim = claim_by_id(state, claim_id);
        assert_ne!(claim["trust_band"], "likely_current");
        assert!(
            matches!(
                claim["metadata"]["render_policy"].as_str(),
                Some(
                    "degraded_safe_summary"
                        | "masked"
                        | "actor_filtered_summary"
                        | "stale_qualified"
                )
            ),
            "{claim_id} should carry a degraded render policy"
        );
    }

    assert_eq!(
        lifecycle_render_policy(&prepared.conn, GOOGLE_DISCONNECTED_CLAIM_ID),
        "degraded_safe_summary"
    );
    assert_eq!(
        lifecycle_invalidation_signal(&prepared.conn, STALE_DOWNSTREAM_CLAIM_ID),
        Some("source_freshness_invalidated".to_string())
    );
}

#[test]
fn source_time_beats_fetch_time_for_stale_downstream_object() {
    let fixture = bundle17();
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-17 fixture prepares");
    let state = expected_post_action_state(&fixture);
    let claim = claim_by_id(state, STALE_DOWNSTREAM_CLAIM_ID);
    let source_asof = &fixture.expected.provenance["provenance"]["source_asof_reachable"];
    let semantics = &fixture.external_replay["downstream_source_semantics"][0];

    assert_eq!(claim["source_asof"], "2025-10-01T09:00:00Z");
    assert_eq!(claim["observed_at"], "2026-05-15T11:45:00Z");
    assert_eq!(source_asof["trust_path_input"], "downstream_source_asof");
    assert_eq!(
        source_asof["downstream_source_asof"],
        semantics["downstream_source_asof"]
    );
    assert_eq!(
        source_asof["wrapper_fetch_at"],
        semantics["wrapper_fetch_at"]
    );
    assert_ne!(claim["trust_band"], "likely_current");

    let (downstream_source_asof, wrapper_fetch_at) =
        stale_downstream_timestamps(&prepared.conn, STALE_DOWNSTREAM_CLAIM_ID);
    assert_eq!(downstream_source_asof, "2025-10-01T09:00:00Z");
    assert_eq!(wrapper_fetch_at, "2026-05-15T11:45:00Z");
}

#[test]
fn restricted_provenance_does_not_leak_sensitive_detail() {
    let provenance = provenance_fixture();
    let mcp_detail = render_serialized_provenance_for(
        serde_json::to_value(&provenance).expect("provenance serializes"),
        ProvenanceActor::Agent {
            name: "dailyos-mcp".to_string(),
            version: "fixture".to_string(),
        },
        dailyos_lib::abilities::provenance::Surface::McpToolDetail,
    );
    let serialized = serde_json::to_string(&mcp_detail).expect("rendered mcp serializes");

    for forbidden in forbidden_detail_tokens() {
        assert!(
            !serialized.contains(forbidden),
            "MCP detail leaked restricted token {forbidden}: {serialized}"
        );
    }
    assert!(serialized.contains("source_id_redacted"));
    assert!(!serialized.contains("canonical_prompt_hash"));
}

#[test]
fn redacted_provenance_retains_safe_summary() {
    let fixture = bundle17();
    let summary = &fixture.expected.provenance["provenance"]["safe_redaction_summary"];

    assert_eq!(summary["source_class"], "glean");
    assert_eq!(summary["lifecycle_state"], "restricted");
    assert_eq!(summary["timestamp_posture"], "source_asof_known");
    assert_eq!(summary["source_asof"], "2026-05-10T10:00:00Z");
    assert_eq!(summary["redaction_reason"], "actor_not_authorized");
    assert_eq!(summary["safe_source_count"], 1);
}

#[tokio::test]
async fn mcp_wrapper_actor_detail_does_not_bypass_actor_filter() {
    let registry = AbilityRegistry::from_descriptors_checked(vec![synthetic_descriptor()])
        .expect("synthetic registry builds");
    let bridge = McpAbilityBridge::new(&registry);
    let session = McpSessionId::from_uuid(uuid::Uuid::from_u128(17));
    let response = bridge
        .invoke_ability(session, "bundle17_actor_provenance", json!({}), false, None)
        .await
        .expect("MCP bridge invocation succeeds");
    let cached_detail = bridge
        .get_provenance(session, response.invocation_id)
        .expect("MCP wrapper cached detail provenance");

    let provenance_json = provenance_value_for_mcp_wrapper();
    assert_eq!(provenance_json["actor"]["kind"], "agent");

    let expected_detail = render_serialized_provenance_for(
        provenance_json,
        ProvenanceActor::Agent {
            name: "dailyos-mcp".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        dailyos_lib::abilities::provenance::Surface::McpToolDetail,
    );
    assert_eq!(cached_detail.value, expected_detail.value);
    assert_eq!(
        response.rendered_provenance.value["render_level"],
        json!("summary")
    );
    assert_eq!(cached_detail.value["render_level"], json!("detail"));

    let summary_serialized =
        serde_json::to_string(&response.rendered_provenance).expect("summary serializes");
    let detail_serialized = serde_json::to_string(&cached_detail).expect("detail serializes");
    for forbidden in forbidden_detail_tokens() {
        assert!(!summary_serialized.contains(forbidden));
        assert!(!detail_serialized.contains(forbidden));
    }
}

#[test]
fn tauri_and_mcp_share_actor_surface_policy() {
    let fixture = bundle17();
    let provenance = tauri_visible_provenance_fixture();
    let serialized = serde_json::to_value(&provenance).expect("provenance serializes");
    let tauri = render_serialized_provenance_for(
        serialized.clone(),
        ProvenanceActor::User,
        dailyos_lib::abilities::provenance::Surface::TauriApp,
    );
    let mcp = render_serialized_provenance_for(
        serialized,
        ProvenanceActor::Agent {
            name: "dailyos-mcp".to_string(),
            version: "fixture".to_string(),
        },
        dailyos_lib::abilities::provenance::Surface::McpToolDetail,
    );

    let tauri_serialized = serde_json::to_string(&tauri).expect("tauri serializes");
    let mcp_serialized = serde_json::to_string(&mcp).expect("mcp serializes");

    assert!(
        tauri_serialized.contains(RESTRICTED_OBJECT_ID)
            || tauri.value["about_this"]["details_available"] == true,
        "Tauri user render should either inline permitted detail or advertise detail availability"
    );
    assert_eq!(
        fixture.expected.output["surfaces"]["tauri_renders"]
            ["restricted_source_detail_visible_when_authorized"],
        true
    );
    assert!(!mcp_serialized.contains(RESTRICTED_OBJECT_ID));
    assert!(mcp_serialized.contains("source_id_redacted"));
    assert_eq!(tauri.value["ability_name"], mcp.value["ability_name"]);
    assert_eq!(
        tauri.value["about_this"]["summary"]["trust"]["effective"],
        mcp.value["trust"]["effective"]
    );
}

#[test]
fn internal_only_content_cannot_become_customer_facing() {
    let fixture = bundle17();
    let public_output = &fixture.expected.output["customer_facing_output"];
    let output_serialized =
        serde_json::to_string(&fixture.expected.output).expect("expected output serializes");
    let provider_attempts = &fixture.provider_replay["attempted_unsafe_public_suggestions"];

    assert_eq!(public_output["contains_internal_only_note"], false);
    assert_eq!(public_output["contains_revoked_source_detail"], false);
    assert!(array_contains_str(
        &public_output["blocked_source_claim_ids"],
        INTERNAL_ONLY_CLAIM_ID
    ));
    assert!(provider_attempts
        .as_array()
        .expect("provider attempts array")
        .iter()
        .any(|attempt| attempt["source_claim_id"] == INTERNAL_ONLY_CLAIM_ID));
    assert!(!output_serialized.contains("Unsafe public wording from the internal-only note."));
}

#[test]
fn all_nine_adr0108_channels_are_enumerated() {
    let fixture = bundle17();
    let expected = channel_names_from_enum();
    let metadata = fixture
        .metadata
        .surfaces_exercised
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let matrix = matrix_channel_names(&fixture);

    assert_eq!(RenderPolicyChannel::all().len(), 9);
    assert_eq!(metadata, expected);
    assert_eq!(matrix, expected);
}

#[test]
fn nine_channel_gate_is_parameterized_by_render_policy_channel_all() {
    let fixture = bundle17();
    let expected = channel_names_from_enum();
    let rows = channel_matrix(&fixture);
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-17 fixture prepares");
    let db_channels = channel_names_from_db(&prepared.conn);

    assert_eq!(rows.len(), RenderPolicyChannel::all().len());
    assert_eq!(db_channels, expected);
    for row in rows {
        let channel = row["channel"].as_str().expect("channel string");
        assert!(
            expected.contains(channel),
            "matrix channel {channel} is not registered in RenderPolicyChannel::all()"
        );
    }
}

#[test]
fn revoked_restricted_rejection_is_green_for_each_channel() {
    let fixture = bundle17();

    for row in channel_matrix(&fixture) {
        let channel = row["channel"].as_str().expect("channel string");
        assert_eq!(
            row["revoked_source_rejected"], true,
            "{channel} must reject revoked source detail"
        );
        assert_eq!(
            row["restricted_source_rejected"], true,
            "{channel} must reject restricted source detail"
        );
        assert_eq!(
            row["internal_only_rejected"], true,
            "{channel} must reject internal-only customer-facing output"
        );
        assert_eq!(
            row["safe_summary_retained"], true,
            "{channel} must keep safe redaction summary"
        );
    }
}

#[test]
fn provenance_masking_shape_is_exercised() {
    let fixture = bundle17();
    let masked_oracle = &fixture.expected.provenance["provenance"]["masked_provenance"];
    let masked = ProvenanceMasked {
        original_invocation_id: InvocationId::new(
            uuid::Uuid::parse_str("17171717-aaaa-4717-87ab-bbbbbbbbbbbb")
                .expect("static uuid parses"),
        ),
        original_ability_name: "prepare_meeting".to_string(),
        original_produced_at: Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap(),
        masked_at: Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap(),
        mask_reason: ProvenanceMaskReason::SourceRevoked {
            data_source: DataSource::Glean {
                downstream: GleanDownstream::Zendesk,
            },
        },
        sources_masked: vec![DataSource::Glean {
            downstream: GleanDownstream::Zendesk,
        }],
    };
    let rendered = render_serialized_provenance_for(
        serde_json::to_value(ProvenanceOrMasked::Masked(masked))
            .expect("masked envelope serializes"),
        ProvenanceActor::Agent {
            name: "dailyos-mcp".to_string(),
            version: "fixture".to_string(),
        },
        dailyos_lib::abilities::provenance::Surface::McpToolDetail,
    );

    assert_eq!(rendered.value["kind"], masked_oracle["kind"]);
    assert_eq!(rendered.value["status"], masked_oracle["status"]);
    assert_eq!(rendered.value["mask_reason"]["kind"], "source_revoked");
    assert_eq!(rendered.value["mask_reason"]["source_class"], "glean");
    assert_eq!(rendered.value["sources_masked"][0]["source_class"], "glean");
}

#[test]
fn sanitization_is_shared_across_render_channels() {
    let field = FieldPath::new("/summary").expect("field path");
    let (sanitized, warning) = sanitize_explanation_for_render(
        &field,
        "# Review [source](https://example.com/detail) and `code` before use",
    );
    assert_eq!(warning, None);
    assert!(sanitized.contains("[url removed]"));
    assert!(!sanitized.contains("https://example.com"));
    assert!(!sanitized.contains("]("));
    assert!(!sanitized.contains('`'));
    assert!(!sanitized.contains('#'));

    let provenance = provenance_fixture();
    for surface in [
        dailyos_lib::abilities::provenance::Surface::TauriApp,
        dailyos_lib::abilities::provenance::Surface::McpToolDetail,
        dailyos_lib::abilities::provenance::Surface::P2Publication,
    ] {
        let actor = match surface {
            dailyos_lib::abilities::provenance::Surface::McpToolDetail => ProvenanceActor::Agent {
                name: "dailyos-mcp".to_string(),
                version: "fixture".to_string(),
            },
            _ => ProvenanceActor::User,
        };
        let rendered = render_serialized_provenance_for(
            serde_json::to_value(&provenance).expect("provenance serializes"),
            actor,
            surface,
        );
        let serialized = serde_json::to_string(&rendered).expect("rendered serializes");
        assert!(!serialized.contains("https://example.com"));
        assert!(!serialized.contains("]("));
    }
}

#[test]
fn logs_and_telemetry_are_low_detail() {
    let fixture = bundle17();

    for channel in ["telemetry", "error_logs", "signal_payloads", "replay"] {
        let value = &fixture.expected.output["surfaces"][channel];
        let serialized = serde_json::to_string(value).expect("channel output serializes");
        assert!(
            value["raw_content_visible"] == false
                || value["raw_restricted_detail_visible"] == false
                || value["safe_fields_only"] == true
                || value["provider_replay_redacted"] == true,
            "{channel} should declare low-detail policy"
        );
        assert!(!serialized.contains(RESTRICTED_OBJECT_ID));
        assert!(!serialized.contains(INTERNAL_GRAPH_ID));
        assert!(!serialized.contains(RAW_ATTRIBUTION_ID));
    }

    let log_render = render_serialized_provenance_for(
        serde_json::to_value(provenance_fixture()).expect("provenance serializes"),
        ProvenanceActor::Agent {
            name: "dailyos-mcp".to_string(),
            version: "fixture".to_string(),
        },
        dailyos_lib::abilities::provenance::Surface::LogStructured,
    );
    let serialized = serde_json::to_string(&log_render).expect("log render serializes");
    assert!(serialized.contains("invocation_id"));
    for forbidden in forbidden_detail_tokens() {
        assert!(!serialized.contains(forbidden));
    }
}

#[test]
fn fixture_data_stays_synthetic() {
    let fixture_dir = bundle_fixture_path(BUNDLE);
    let entries = std::fs::read_dir(&fixture_dir).expect("bundle fixture dir readable");

    for entry in entries {
        let path = entry.expect("fixture entry").path();
        if !path.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("fixture file name");
        assert!(
            !file_name.contains("identity"),
            "bundle-17 must not add identity-map artifacts"
        );
        let content = std::fs::read_to_string(&path).expect("fixture file reads");
        let lowered = content.to_ascii_lowercase();
        for forbidden in [
            "james",
            "gmail.com",
            "slack.com/",
            "salesforce.com/",
            "zendesk.com/",
            "gong.io/",
        ] {
            assert!(
                !lowered.contains(forbidden),
                "{} contains non-synthetic token {forbidden}",
                path.display()
            );
        }
        for token in content
            .split(|ch: char| ch.is_whitespace() || matches!(ch, '"' | '\'' | ',' | '[' | ']'))
        {
            if token.contains('@') {
                assert!(
                    token.contains("example.com"),
                    "{} contains non-example email-like token {token}",
                    path.display()
                );
            }
        }
    }
}

fn bundle17() -> EvalFixture {
    load_fixture(&bundle_fixture_path(BUNDLE)).expect("bundle-17 fixture loads")
}

fn lifecycle_states(conn: &Connection) -> BTreeSet<String> {
    let mut statement = conn
        .prepare("SELECT DISTINCT lifecycle_state FROM source_lifecycle_states")
        .expect("source lifecycle query prepares");
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("source lifecycle query runs")
        .collect::<Result<BTreeSet<_>, _>>()
        .expect("source lifecycle rows map")
}

fn source_lifecycle_count_for_claim(conn: &Connection, claim_id: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM source_lifecycle_states WHERE claim_id = ?1",
        [claim_id],
        |row| row.get(0),
    )
    .expect("source lifecycle count")
}

fn lifecycle_render_policy(conn: &Connection, claim_id: &str) -> String {
    conn.query_row(
        "SELECT render_policy FROM source_lifecycle_states WHERE claim_id = ?1",
        [claim_id],
        |row| row.get(0),
    )
    .expect("source lifecycle render policy")
}

fn lifecycle_invalidation_signal(conn: &Connection, claim_id: &str) -> Option<String> {
    conn.query_row(
        "SELECT invalidation_signal FROM source_lifecycle_states WHERE claim_id = ?1",
        [claim_id],
        |row| row.get(0),
    )
    .expect("source lifecycle invalidation signal")
}

fn stale_downstream_timestamps(conn: &Connection, claim_id: &str) -> (String, String) {
    conn.query_row(
        "SELECT downstream_source_asof, wrapper_fetch_at
         FROM source_lifecycle_states
         WHERE claim_id = ?1",
        [claim_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .expect("stale downstream timestamps")
}

fn channel_names_from_db(conn: &Connection) -> BTreeSet<&'static str> {
    let mut statement = conn
        .prepare("SELECT channel FROM bundle17_channel_policy")
        .expect("channel policy query prepares");
    let channels = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("channel policy query runs")
        .collect::<Result<BTreeSet<_>, _>>()
        .expect("channel policy rows map");
    channel_names_from_owned(channels)
}

fn channel_names_from_enum() -> BTreeSet<&'static str> {
    RenderPolicyChannel::all()
        .iter()
        .copied()
        .map(RenderPolicyChannel::as_str)
        .collect()
}

fn matrix_channel_names(fixture: &EvalFixture) -> BTreeSet<&str> {
    channel_matrix(fixture)
        .iter()
        .map(|row| row["channel"].as_str().expect("channel string"))
        .collect()
}

fn channel_names_from_owned(channels: BTreeSet<String>) -> BTreeSet<&'static str> {
    let expected = channel_names_from_enum();
    channels
        .into_iter()
        .map(|channel| {
            expected
                .iter()
                .copied()
                .find(|expected| *expected == channel)
                .unwrap_or_else(|| panic!("unknown channel {channel}"))
        })
        .collect()
}

fn channel_matrix(fixture: &EvalFixture) -> &Vec<Value> {
    fixture.expected.output["render_policy_channel_matrix"]
        .as_array()
        .expect("render policy channel matrix array")
}

fn array_contains_str(array: &Value, expected: &str) -> bool {
    array
        .as_array()
        .expect("string array")
        .iter()
        .any(|value| value.as_str() == Some(expected))
}

fn forbidden_detail_tokens() -> [&'static str; 5] {
    [
        RESTRICTED_OBJECT_ID,
        INTERNAL_GRAPH_ID,
        RAW_ATTRIBUTION_ID,
        PROMPT_HASH_ID,
        WATERMARK_ID,
    ]
}

fn synthetic_provenance_erased<'a>(
    _ctx: &'a AbilityContext<'a>,
    _input: serde_json::Value,
) -> ErasedFuture<'a> {
    Box::pin(async move {
        Ok(json!({
            "data": {
                "claim_id": GLEAN_RESTRICTED_CLAIM_ID,
                "render_policy": "actor_filtered_summary"
            },
            "ability_version": { "major": 1, "minor": 0 },
            "diagnostics": { "warnings": [] },
            "provenance": provenance_value_for_mcp_wrapper()
        }))
    })
}

fn synthetic_descriptor() -> AbilityDescriptor {
    AbilityDescriptor {
        name: "bundle17_actor_provenance",
        version: "1.0.0",
        schema_version: 1,
        category: AbilityCategory::Read,
        policy: AbilityPolicy {
            allowed_actors: AGENT_ACTORS,
            allowed_modes: LIVE_MODES,
            requires_confirmation: false,
            may_publish: false,
            required_scopes: &[],
            mcp_exposure: McpExposure::None,
            client_side_executable: false,
            rate_limit: None,
        },
        composes: &[],
        mutates: &[],
        experimental: false,
        registered_at: None,
        signal_policy: SignalPolicy::default(),
        invoke_erased: synthetic_provenance_erased,
        input_schema: closed_object_schema,
        output_schema: closed_object_schema,
    }
}

fn closed_object_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {}
    })
}

fn provenance_value_for_mcp_wrapper() -> Value {
    serde_json::to_value(provenance_fixture()).expect("provenance serializes")
}

fn provenance_fixture() -> dailyos_lib::abilities::provenance::Provenance {
    let produced_at = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();
    let source_asof = Utc.with_ymd_and_hms(2026, 5, 10, 10, 0, 0).unwrap();
    let observed_at = Utc.with_ymd_and_hms(2026, 5, 15, 10, 0, 0).unwrap();
    let subject = SubjectAttribution::direct_confident(SubjectRef::Account(ACCOUNT_ID.to_string()));
    let explanation = SanitizedExplanation::new(
        "Based on restricted source https://example.com/detail and actor-filtered summary.",
    )
    .expect("fixture explanation is accepted");
    let source = SourceAttribution::new(
        DataSource::Glean {
            downstream: GleanDownstream::Zendesk,
        },
        vec![
            SourceIdentifier::Signal {
                signal_id: SignalId::new(INTERNAL_GRAPH_ID),
            },
            SourceIdentifier::OpaqueGleanSource {
                downstream: GleanDownstream::Zendesk,
                opaque_ref: RESTRICTED_OBJECT_ID.to_string(),
                cited_as_of: source_asof,
            },
            SourceIdentifier::ProviderCompletion {
                completion_id: WATERMARK_ID.to_string(),
                provider: ProviderRef::new("provider-b17-restricted"),
            },
        ],
        observed_at,
        Some(source_asof),
        0.55,
        None,
    )
    .expect("source attribution builds");
    let field_attribution = FieldAttribution::new(
        subject.clone(),
        DerivationKind::Direct,
        vec![SourceRef::Source {
            source_index: dailyos_lib::abilities::provenance::SourceIndex(0),
        }],
        Confidence {
            value: 0.55,
            kind: dailyos_lib::abilities::provenance::ConfidenceKind::Declared,
        },
        Some(explanation.clone()),
    )
    .expect("field attribution builds");
    let mut provenance = provenance_for_test(
        "bundle17_actor_provenance",
        produced_at,
        subject,
        vec![source],
        Vec::new(),
        BTreeMap::from([(
            FieldPath::new("/summary").expect("field path"),
            field_attribution,
        )]),
        Some(PromptFingerprint {
            provider: "replay".to_string(),
            model: ModelName("bundle17-model".to_string()),
            prompt_template_id: PromptTemplateId("bundle17-template".to_string()),
            prompt_template_version: PromptVersion("1".to_string()),
            canonical_prompt_hash: HashValue::new(PROMPT_HASH_ID),
            temperature: 0.0,
            top_p: None,
            seed: Some(170292),
            tokens_input: None,
            tokens_output: None,
            provider_completion_id: Some(WATERMARK_ID.to_string()),
        }),
        vec![
            ProvenanceWarning::Masked {
                reason: MaskReason::ActorNotAuthorized,
            },
            ProvenanceWarning::SourceUnresolvable {
                source_index: dailyos_lib::abilities::provenance::SourceIndex(0),
                reason: RAW_ATTRIBUTION_ID.to_string(),
            },
        ],
    );
    provenance.invocation_id = InvocationId::new(
        uuid::Uuid::parse_str("17171717-aaaa-4717-87ab-bbbbbbbbbbbb").expect("static uuid parses"),
    );
    provenance.actor = ProvenanceActor::Agent {
        name: "dailyos-mcp".to_string(),
        version: "fixture".to_string(),
    };
    provenance
}

fn tauri_visible_provenance_fixture() -> dailyos_lib::abilities::provenance::Provenance {
    let produced_at = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();
    let source_asof = Utc.with_ymd_and_hms(2026, 5, 10, 10, 0, 0).unwrap();
    let observed_at = Utc.with_ymd_and_hms(2026, 5, 15, 10, 0, 0).unwrap();
    let subject = SubjectAttribution::direct_confident(SubjectRef::Account(ACCOUNT_ID.to_string()));
    let source = SourceAttribution::new(
        DataSource::Glean {
            downstream: GleanDownstream::Zendesk,
        },
        vec![SourceIdentifier::OpaqueGleanSource {
            downstream: GleanDownstream::Zendesk,
            opaque_ref: RESTRICTED_OBJECT_ID.to_string(),
            cited_as_of: source_asof,
        }],
        observed_at,
        Some(source_asof),
        0.55,
        None,
    )
    .expect("source attribution builds");
    let mut provenance = provenance_for_test(
        "bundle17_actor_provenance",
        produced_at,
        subject,
        vec![source],
        Vec::new(),
        BTreeMap::new(),
        None,
        Vec::new(),
    );
    provenance.actor = ProvenanceActor::Agent {
        name: "dailyos-mcp".to_string(),
        version: "fixture".to_string(),
    };
    provenance
}

use std::future::Future;
use std::pin::Pin;

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::registry::{AbilityPolicy, McpExposure, SignalPolicy};
use dailyos_lib::abilities::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityRegistry, Actor,
};
use dailyos_lib::bridges::mcp::McpAbilityBridge;
use dailyos_lib::bridges::tauri::TauriAbilityBridge;
use dailyos_lib::bridges::McpSessionId;
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::claims::{commit_claim, withdraw_claim, ClaimProposal, CommittedClaim};
use dailyos_lib::services::context::{ClaimDismissalSurface, ExecutionMode};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use dailyos_lib::services::sensitivity::{
    render_mcp_ability_data_for_surface, render_mcp_ability_data_for_surface_with_provenance,
};
use dailyos_lib::state::AppState;
use rusqlite::Connection;
use serde_json::{json, Value};

const PUBLIC_TEXT: &str = "public launch readiness is green.";
const INTERNAL_TEXT: &str = "internal rollout dependency is tracked.";
const CONFIDENTIAL_TEXT: &str = "confidential renewal risk must stay hidden.";
const USER_ONLY_TEXT: &str = "user-only negotiation note must stay hidden.";
const UNTAGGED_PRIVATE_TEXT: &str = "Untagged private note must not cross MCP.";
const CYCLE4_OWNER_TEXT: &str = "Private owner text from source content.";
const CYCLE4_ATTENDEE_TEXT: &str = "Private attendee context name from source content.";
const DIAGNOSTIC_PRIVATE_TEXT: &str = "Diagnostic warning with confidential customer text.";
const CLAIMS_SCHEMA_SQL: &str = include_str!("../src/migrations/129_dos_7_claims_schema.sql");
const TYPED_FEEDBACK_SQL: &str =
    include_str!("../src/migrations/135_dos_294_typed_feedback_schema.sql");
const PROJECTION_STATUS_SQL: &str =
    include_str!("../src/migrations/134_dos_301_claim_projection_status.sql");
const MINIMAL_ENTITY_SCHEMA_SQL: &str = r#"
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    claim_version INTEGER NOT NULL DEFAULT 0
);
"#;
const SUBJECT_ACCOUNT_ID: &str = "acct-dos412-mcp-ability";
const TS: &str = "2026-05-06T12:00:00Z";

const USER_AGENT_ACTORS: &[Actor] = &[Actor::User, Actor::Agent];
const LIVE_MODES: &[ExecutionMode] = &[ExecutionMode::Live];

type ErasedFuture<'a> =
    Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

fn synthetic_claim_text_erased<'a>(
    ctx: &'a AbilityContext<'a>,
    _input: serde_json::Value,
) -> ErasedFuture<'a> {
    Box::pin(async move {
        Ok(json!({
            "data": synthetic_ability_data(),
            "ability_version": { "major": 1, "minor": 0 },
                "diagnostics": { "warnings": [DIAGNOSTIC_PRIVATE_TEXT] },
            "provenance": {
                "invocation_id": "41241241-4124-4124-8124-412412412412",
                "ability_name": "dos412_synthetic_claim_text",
                "ability_version": { "major": 1, "minor": 0 },
                "ability_schema_version": 1,
                "actor": format!("{:?}", ctx.actor),
                "mode": ctx.mode().as_str(),
                "warnings": []
            }
        }))
    })
}

#[tokio::test]
async fn mcp_ability_data_redacts_tagged_private_claim_text_while_tauri_stays_raw() {
    let conn = fresh_claims_conn();
    let ctx = live_claim_ctx();
    seed_standard_claims(&ctx, &conn);
    let registry = AbilityRegistry::from_descriptors_checked(vec![synthetic_descriptor()]).unwrap();

    let mcp_data_value =
        render_mcp_ability_data_for_surface(ActionDb::from_conn(&conn), synthetic_ability_data());
    let mcp_data = serde_json::to_string(&mcp_data_value).unwrap();

    assert!(mcp_data.contains(PUBLIC_TEXT));
    assert!(mcp_data.contains(INTERNAL_TEXT));
    assert!(!mcp_data.contains(CONFIDENTIAL_TEXT));
    assert!(!mcp_data.contains(USER_ONLY_TEXT));
    assert!(!mcp_data.contains(UNTAGGED_PRIVATE_TEXT));
    assert!(!mcp_data.contains(CYCLE4_OWNER_TEXT));
    assert!(!mcp_data.contains(CYCLE4_ATTENDEE_TEXT));
    assert!(mcp_data_value["claims"].get("confidential").is_none());
    assert!(mcp_data_value["claims"].get("user_only").is_none());
    assert!(mcp_data_value["untagged"].get("summary").is_none());
    assert!(mcp_data_value["top_level_secret"].is_null());
    assert!(mcp_data_value["untagged"]["nested"].get("detail").is_none());
    assert!(mcp_data_value["untagged"]["nested"].get("id").is_none());
    assert!(mcp_data_value["untagged"]["nested"].get("status").is_none());
    assert!(mcp_data_value["untagged"]["nested"]
        .get("created_at")
        .is_none());
    assert_eq!(
        mcp_data_value["meeting"]["title"],
        "Metadata planning meeting"
    );
    assert_eq!(
        mcp_data_value["meeting"]["attendees"][0]["name"],
        "Riley Rivera"
    );
    assert!(mcp_data_value["meeting"]["attendees"][0]
        .get("email")
        .is_none());
    assert_eq!(
        mcp_data_value["meeting"]["attendees"][0]["person_id"],
        "person-riley"
    );
    assert!(mcp_data_value["open_loops"][0].get("owner").is_none());
    assert!(mcp_data_value["attendee_context"][0]
        .get("attendee")
        .is_none());

    let public_claim = mcp_data_value["claims"]["public"]
        .as_object()
        .expect("public claim renders as an object");
    assert_eq!(public_claim.len(), 2);
    assert_eq!(public_claim["text"], PUBLIC_TEXT);
    assert!(public_claim.get("policy").is_some());

    let state = AppState::new();
    let tauri_bridge = TauriAbilityBridge::new(&registry);
    let tauri_response = tauri_bridge
        .invoke_tauri_app(
            &state,
            "dos412_synthetic_claim_text",
            json!({}),
            ClaimDismissalSurface::TauriEntityDetail,
            false,
            None,
        )
        .await
        .unwrap();
    let tauri_data = serde_json::to_string(&tauri_response.data).unwrap();

    for text in [
        PUBLIC_TEXT,
        INTERNAL_TEXT,
        CONFIDENTIAL_TEXT,
        USER_ONLY_TEXT,
        UNTAGGED_PRIVATE_TEXT,
        CYCLE4_OWNER_TEXT,
        CYCLE4_ATTENDEE_TEXT,
    ] {
        assert!(
            tauri_data.contains(text),
            "Tauri data should include {text}"
        );
    }
    let tauri_diagnostics = serde_json::to_string(&tauri_response.diagnostics).unwrap();
    assert!(tauri_diagnostics.contains(DIAGNOSTIC_PRIVATE_TEXT));
}

#[tokio::test]
async fn mcp_ability_response_drops_diagnostics_warnings_while_tauri_keeps_them() {
    let registry = AbilityRegistry::from_descriptors_checked(vec![synthetic_descriptor()]).unwrap();
    let mcp_bridge = McpAbilityBridge::new(&registry);
    let mcp_response = mcp_bridge
        .invoke_ability(
            McpSessionId::from_uuid(uuid::Uuid::from_u128(412)),
            "dos412_synthetic_claim_text",
            json!({}),
            false,
            None,
        )
        .await
        .unwrap();

    let mcp_diagnostics = serde_json::to_string(&mcp_response.diagnostics).unwrap();
    assert!(!mcp_diagnostics.contains(DIAGNOSTIC_PRIVATE_TEXT));
    assert_eq!(mcp_response.diagnostics, json!({ "warnings": [] }));
    let serialized_mcp_response = serde_json::to_value(&mcp_response).unwrap();
    assert!(
        serialized_mcp_response.get("diagnostics").is_none(),
        "serialized MCP response must omit diagnostics entirely"
    );

    let state = AppState::new();
    let tauri_bridge = TauriAbilityBridge::new(&registry);
    let tauri_response = tauri_bridge
        .invoke_tauri_app(
            &state,
            "dos412_synthetic_claim_text",
            json!({}),
            ClaimDismissalSurface::TauriEntityDetail,
            false,
            None,
        )
        .await
        .unwrap();
    let tauri_diagnostics = serde_json::to_string(&tauri_response.diagnostics).unwrap();
    assert!(tauri_diagnostics.contains(DIAGNOSTIC_PRIVATE_TEXT));
    let serialized_tauri_response = serde_json::to_value(&tauri_response).unwrap();
    assert!(serialized_tauri_response.get("diagnostics").is_some());
}

#[test]
fn mcp_ability_data_drops_dto_sensitivity_downgrade_from_stored_confidential() {
    let conn = fresh_claims_conn();
    let ctx = live_claim_ctx();
    seed_claim(
        &ctx,
        &conn,
        "claim-confidential-downgrade",
        ClaimSensitivity::Confidential,
        CONFIDENTIAL_TEXT,
    );

    let rendered = render_mcp_ability_data_for_surface(
        ActionDb::from_conn(&conn),
        json!({
            "claim": tagged_claim("claim-confidential-downgrade", "internal", CONFIDENTIAL_TEXT)
        }),
    );

    let serialized = serde_json::to_string(&rendered).unwrap();
    assert!(!serialized.contains(CONFIDENTIAL_TEXT));
    assert!(rendered.as_object().unwrap().get("claim").is_none());
}

#[test]
fn mcp_ability_data_drops_tagged_claim_text_mismatch_against_stored_text() {
    let conn = fresh_claims_conn();
    let ctx = live_claim_ctx();
    seed_claim(
        &ctx,
        &conn,
        "claim-internal-text-mismatch",
        ClaimSensitivity::Internal,
        INTERNAL_TEXT,
    );

    let rendered = render_mcp_ability_data_for_surface(
        ActionDb::from_conn(&conn),
        json!({
            "claim": tagged_claim(
                "claim-internal-text-mismatch",
                "internal",
                "Forged DTO text must not render."
            )
        }),
    );

    let serialized = serde_json::to_string(&rendered).unwrap();
    assert!(!serialized.contains("Forged DTO text must not render."));
    assert!(!serialized.contains(INTERNAL_TEXT));
    assert!(rendered.as_object().unwrap().get("claim").is_none());
}

#[test]
fn mcp_ability_data_drops_withdrawn_stored_claim_text() {
    let conn = fresh_claims_conn();
    let ctx = live_claim_ctx();
    seed_claim(
        &ctx,
        &conn,
        "claim-internal-withdrawn",
        ClaimSensitivity::Internal,
        INTERNAL_TEXT,
    );
    withdraw_claim(
        &ctx,
        ActionDb::from_conn(&conn),
        "claim-internal-withdrawn",
        "unit test withdrawal",
    )
    .expect("withdraw claim fixture");

    let rendered = render_mcp_ability_data_for_surface(
        ActionDb::from_conn(&conn),
        json!({
            "claim": tagged_claim("claim-internal-withdrawn", "internal", INTERNAL_TEXT)
        }),
    );

    assert!(rendered.as_object().unwrap().get("claim").is_none());
    assert!(!serde_json::to_string(&rendered)
        .unwrap()
        .contains(INTERNAL_TEXT));
}

#[test]
fn mcp_ability_data_renders_tagged_claim_from_stored_text_only() {
    let conn = fresh_claims_conn();
    let ctx = live_claim_ctx();
    seed_claim(
        &ctx,
        &conn,
        "claim-internal-stored-render",
        ClaimSensitivity::Internal,
        INTERNAL_TEXT,
    );

    let rendered = render_mcp_ability_data_for_surface(
        ActionDb::from_conn(&conn),
        json!({
            "claim": tagged_claim("claim-internal-stored-render", "internal", INTERNAL_TEXT)
        }),
    );

    let claim = rendered["claim"]
        .as_object()
        .expect("matching internal claim renders");
    assert_eq!(claim.len(), 2);
    assert_eq!(claim["text"], INTERNAL_TEXT);
    assert!(claim.get("policy").is_some());
    assert!(claim.get("claim_id").is_none());
    assert!(claim.get("sensitivity").is_none());
}

#[test]
fn mcp_ability_data_renders_only_provenance_attested_raw_claim_text() {
    let conn = fresh_claims_conn();
    let ctx = live_claim_ctx();
    seed_claim(
        &ctx,
        &conn,
        "claim-internal-provenance",
        ClaimSensitivity::Internal,
        INTERNAL_TEXT,
    );
    seed_claim(
        &ctx,
        &conn,
        "claim-confidential-provenance",
        ClaimSensitivity::Confidential,
        CONFIDENTIAL_TEXT,
    );

    let rendered = render_mcp_ability_data_for_surface_with_provenance(
        ActionDb::from_conn(&conn),
        json!({
            "summary": INTERNAL_TEXT,
            "private_summary": CONFIDENTIAL_TEXT,
            "unattributed_summary": UNTAGGED_PRIVATE_TEXT
        }),
        &json!({
            "sources": [
                {
                    "identifiers": [
                        { "signal": { "signal_id": "claim-internal-provenance" } }
                    ]
                },
                {
                    "identifiers": [
                        { "signal": { "signal_id": "claim-confidential-provenance" } }
                    ]
                }
            ],
            "field_attributions": {
                "/summary": {
                    "source_refs": [{ "source": { "source_index": 0 } }]
                },
                "/private_summary": {
                    "source_refs": [{ "source": { "source_index": 1 } }]
                }
            }
        }),
    );

    assert_eq!(rendered["summary"]["text"], INTERNAL_TEXT);
    assert!(rendered["summary"].get("policy").is_some());
    assert!(rendered
        .as_object()
        .unwrap()
        .get("private_summary")
        .is_none());
    assert!(rendered
        .as_object()
        .unwrap()
        .get("unattributed_summary")
        .is_none());
    let serialized = serde_json::to_string(&rendered).unwrap();
    assert!(!serialized.contains(CONFIDENTIAL_TEXT));
    assert!(!serialized.contains(UNTAGGED_PRIVATE_TEXT));
}

#[test]
fn mcp_ability_data_drops_provenance_attested_raw_leaf_text_mismatch() {
    let conn = fresh_claims_conn();
    let ctx = live_claim_ctx();
    seed_claim(
        &ctx,
        &conn,
        "claim-internal-provenance-mismatch",
        ClaimSensitivity::Internal,
        INTERNAL_TEXT,
    );

    let rendered = render_mcp_ability_data_for_surface_with_provenance(
        ActionDb::from_conn(&conn),
        json!({
            "summary": "Forged provenance leaf must not render."
        }),
        &json!({
            "sources": [{
                "identifiers": [
                    { "signal": { "signal_id": "claim-internal-provenance-mismatch" } }
                ]
            }],
            "field_attributions": {
                "/summary": {
                    "source_refs": [{ "source": { "source_index": 0 } }]
                }
            }
        }),
    );

    assert!(rendered.as_object().unwrap().get("summary").is_none());
    let serialized = serde_json::to_string(&rendered).unwrap();
    assert!(!serialized.contains("Forged provenance leaf must not render."));
    assert!(!serialized.contains(INTERNAL_TEXT));
}

#[test]
fn mcp_metadata_key_names_at_non_allowlisted_paths_drop() {
    let rendered = render_mcp_ability_data_for_surface(
        ActionDb::from_conn(&fresh_claims_conn()),
        json!({
            "workflow": {
                "status": "active",
                "kind": "account"
            }
        }),
    );

    assert!(rendered["workflow"].get("status").is_none());
    assert!(rendered["workflow"].get("kind").is_none());
}

#[test]
fn mcp_metadata_allowlisted_enum_path_passes() {
    let rendered = render_mcp_ability_data_for_surface(
        ActionDb::from_conn(&fresh_claims_conn()),
        json!([{ "entityType": "account" }]),
    );

    assert_eq!(rendered[0]["entityType"], "account");
}

#[test]
fn mcp_metadata_timestamp_validator_catches_malformed_iso() {
    let rendered = render_mcp_ability_data_for_surface(
        ActionDb::from_conn(&fresh_claims_conn()),
        json!([
            { "createdAt": "2026-05-06T12:00:00Z" },
            { "createdAt": "not-a-timestamp" }
        ]),
    );

    assert_eq!(rendered[0]["createdAt"], "2026-05-06T12:00:00Z");
    assert!(rendered[1].get("createdAt").is_none());
}

#[test]
fn mcp_metadata_identifier_validator_catches_non_id_strings() {
    let rendered = render_mcp_ability_data_for_surface(
        ActionDb::from_conn(&fresh_claims_conn()),
        json!([
            { "id": "claim-internal" },
            { "id": "not an id" }
        ]),
    );

    assert_eq!(rendered[0]["id"], "claim-internal");
    assert!(rendered[1].get("id").is_none());
}

macro_rules! tagged_sibling_regression {
    ($test_name:ident, $field:literal) => {
        #[test]
        fn $test_name() {
            assert_tagged_claim_sibling_is_stripped($field);
        }
    };
}

tagged_sibling_regression!(
    tagged_claim_carrier_strips_source_text_sibling,
    "source_text"
);
tagged_sibling_regression!(
    tagged_claim_carrier_strips_source_summary_sibling,
    "sourceSummary"
);
tagged_sibling_regression!(
    tagged_claim_carrier_strips_evidence_text_sibling,
    "evidenceText"
);
tagged_sibling_regression!(tagged_claim_carrier_strips_raw_text_sibling, "rawText");
tagged_sibling_regression!(tagged_claim_carrier_strips_quote_sibling, "quote");

fn assert_tagged_claim_sibling_is_stripped(field: &str) {
    let conn = fresh_claims_conn();
    let ctx = live_claim_ctx();
    seed_claim(
        &ctx,
        &conn,
        "claim-internal-sibling",
        ClaimSensitivity::Internal,
        INTERNAL_TEXT,
    );

    let mut tagged = tagged_claim("claim-internal-sibling", "internal", INTERNAL_TEXT);
    tagged[field] = json!(CONFIDENTIAL_TEXT);
    let rendered =
        render_mcp_ability_data_for_surface(ActionDb::from_conn(&conn), json!({ "claim": tagged }));

    let claim = rendered["claim"]
        .as_object()
        .expect("internal claim should render");
    assert_eq!(claim["text"], INTERNAL_TEXT);
    assert!(
        claim.get(field).is_none(),
        "{field} sibling must be stripped"
    );
    assert!(claim.get("claim_id").is_none());
    assert!(claim.get("sensitivity").is_none());
    assert!(claim.get("originating_actor").is_none());
    assert!(!serde_json::to_string(&rendered)
        .unwrap()
        .contains(CONFIDENTIAL_TEXT));
}

fn synthetic_ability_data() -> Value {
    json!({
        "top_level_secret": UNTAGGED_PRIVATE_TEXT,
        "claims": {
            "public": tagged_claim("claim-public", "public", PUBLIC_TEXT),
            "internal": tagged_claim("claim-internal", "internal", INTERNAL_TEXT),
            "confidential": tagged_claim(
                "claim-confidential",
                "confidential",
                CONFIDENTIAL_TEXT
            ),
            "user_only": tagged_claim("claim-user-only", "user_only", USER_ONLY_TEXT)
        },
        "meeting": {
            "id": "meeting-dos412",
            "title": "Metadata planning meeting",
            "starts_at": "2026-05-06T12:00:00Z",
            "attendees": [{
                "name": "Riley Rivera",
                "email": "riley@example.com",
                "person_id": "person-riley"
            }]
        },
        "untagged": {
            "summary": CONFIDENTIAL_TEXT,
            "nested": {
                "id": "metadata-row-1",
                "status": "active",
                "created_at": "2026-05-06T12:00:00Z",
                "detail": UNTAGGED_PRIVATE_TEXT,
                "items": [
                    UNTAGGED_PRIVATE_TEXT,
                    { "description": UNTAGGED_PRIVATE_TEXT }
                ]
            }
        },
        "open_loops": [{
            "description": UNTAGGED_PRIVATE_TEXT,
            "owner": CYCLE4_OWNER_TEXT,
            "subject": { "kind": "meeting", "id": "meeting-dos412" },
            "temporal_scope": "state"
        }],
        "attendee_context": [{
            "attendee": CYCLE4_ATTENDEE_TEXT,
            "context": UNTAGGED_PRIVATE_TEXT,
            "subject": { "kind": "person", "id": "person-riley" },
            "temporal_scope": "state"
        }]
    })
}

fn tagged_claim(claim_id: &str, sensitivity: &str, text: &str) -> Value {
    json!({
        "text": text,
        "claim_id": claim_id,
        "sensitivity": sensitivity,
        "originating_actor": "user"
    })
}

fn fresh_claims_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(MINIMAL_ENTITY_SCHEMA_SQL)
        .expect("apply minimal entity schema");
    conn.execute(
        "INSERT INTO accounts (id, claim_version) VALUES (?1, 0)",
        [SUBJECT_ACCOUNT_ID],
    )
    .expect("seed subject account");
    conn.execute_batch(CLAIMS_SCHEMA_SQL)
        .expect("apply claims schema");
    conn.execute_batch(TYPED_FEEDBACK_SQL)
        .expect("apply typed feedback schema");
    conn.execute_batch(PROJECTION_STATUS_SQL)
        .expect("apply projection status schema");
    conn
}

fn live_claim_ctx() -> ServiceContext<'static> {
    let clock = Box::leak(Box::new(FixedClock::new(
        Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap(),
    )));
    let rng = Box::leak(Box::new(SeedableRng::new(412)));
    let external = Box::leak(Box::new(ExternalClients::default()));
    ServiceContext::new_live(clock, rng, external)
}

fn seed_standard_claims(ctx: &ServiceContext<'_>, conn: &Connection) {
    seed_claim(
        ctx,
        conn,
        "claim-public",
        ClaimSensitivity::Public,
        PUBLIC_TEXT,
    );
    seed_claim(
        ctx,
        conn,
        "claim-internal",
        ClaimSensitivity::Internal,
        INTERNAL_TEXT,
    );
    seed_claim(
        ctx,
        conn,
        "claim-confidential",
        ClaimSensitivity::Confidential,
        CONFIDENTIAL_TEXT,
    );
    seed_claim(
        ctx,
        conn,
        "claim-user-only",
        ClaimSensitivity::UserOnly,
        USER_ONLY_TEXT,
    );
}

fn seed_claim(
    ctx: &ServiceContext<'_>,
    conn: &Connection,
    id: &str,
    sensitivity: ClaimSensitivity,
    text: &str,
) {
    let committed = commit_claim(
        ctx,
        ActionDb::from_conn(conn),
        ClaimProposal {
            id: Some(id.to_string()),
            subject_ref: json!({ "kind": "account", "id": SUBJECT_ACCOUNT_ID }).to_string(),
            claim_type: "risk".to_string(),
            field_path: Some(format!("dos412.{id}")),
            topic_key: None,
            text: text.to_string(),
            actor: "agent:test".to_string(),
            data_source: "unit_test".to_string(),
            source_ref: None,
            source_asof: Some(TS.to_string()),
            observed_at: TS.to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(sensitivity),
            supersedes: None,
            tombstone: None,
        },
    )
    .expect("commit claim fixture");

    match committed {
        CommittedClaim::Inserted { claim } => assert_eq!(claim.id, id),
        other => panic!("expected inserted claim fixture, got {other:?}"),
    }
}

fn synthetic_descriptor() -> AbilityDescriptor {
    AbilityDescriptor {
        name: "dos412_synthetic_claim_text",
        version: "1.0.0",
        schema_version: 1,
        category: AbilityCategory::Read,
        policy: AbilityPolicy {
            allowed_actors: USER_AGENT_ACTORS,
            allowed_modes: LIVE_MODES,
            requires_confirmation: false,
            may_publish: false,
            required_scopes: &[],
            mcp_exposure: McpExposure::None,
            client_side_executable: false,
        },
        composes: &[],
        mutates: &[],
        experimental: false,
        registered_at: None,
        signal_policy: SignalPolicy::default(),
        invoke_erased: synthetic_claim_text_erased,
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

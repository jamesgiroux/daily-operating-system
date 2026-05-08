#![cfg(feature = "test-harness")]

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::AbilityRegistry;
use dailyos_lib::bridges::mcp::McpAbilityBridge;
use dailyos_lib::bridges::McpSessionId;
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::claims::{commit_claim, ClaimProposal, CommittedClaim};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use parking_lot::Mutex;
use rusqlite::Connection;
use serde_json::json;

const CLAIMS_SCHEMA_SQL: &str = include_str!("../src/migrations/129_dos_7_claims_schema.sql");
const PROJECTION_STATUS_SQL: &str =
    include_str!("../src/migrations/134_dos_301_claim_projection_status.sql");
const TYPED_FEEDBACK_SQL: &str =
    include_str!("../src/migrations/135_dos_294_typed_feedback_schema.sql");
const MINIMAL_ENTITY_SCHEMA_SQL: &str = r#"
CREATE TABLE people (
    id TEXT PRIMARY KEY,
    claim_version INTEGER NOT NULL DEFAULT 0
);
"#;
const PERSON_ID: &str = "person-w05-a-mcp-reader";
const CLAIM_TEXT: &str = "MCP dynamic get_entity_context reads this claim through the bridge.";
const TS: &str = "2026-05-06T12:00:00Z";

#[tokio::test]
async fn mcp_dynamic_get_entity_context_uses_readonly_actiondb_readers() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let db_path = temp_dir.path().join("mcp-dynamic-readers.db");
    let write_conn = Connection::open(&db_path).expect("open writable fixture DB");
    write_conn
        .execute_batch(MINIMAL_ENTITY_SCHEMA_SQL)
        .expect("apply minimal entity schema");
    write_conn
        .execute(
            "INSERT INTO people (id, claim_version) VALUES (?1, 0)",
            [PERSON_ID],
        )
        .expect("seed person");
    write_conn
        .execute_batch(CLAIMS_SCHEMA_SQL)
        .expect("apply claims schema");
    write_conn
        .execute_batch(PROJECTION_STATUS_SQL)
        .expect("apply projection status schema");
    write_conn
        .execute_batch(TYPED_FEEDBACK_SQL)
        .expect("apply typed feedback schema");

    let write_db = ActionDb::from_connection_for_tests(write_conn);
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(349);
    let external = ExternalClients::default();
    let seed_ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("agent:test");
    let claim_id = seed_person_claim(&seed_ctx, &write_db);
    drop(write_db);

    let readonly_db =
        ActionDb::open_unencrypted_readonly_at_for_tests(&db_path).expect("open readonly DB");
    let query_only: i64 = readonly_db
        .conn_ref()
        .query_row("PRAGMA query_only", [], |row| row.get(0))
        .expect("read query_only pragma");
    assert_eq!(query_only, 1, "fixture reader DB must be query_only");

    let registry = AbilityRegistry::from_inventory_checked().expect("ability registry builds");
    let bridge =
        McpAbilityBridge::new_with_action_db_readers(&registry, Arc::new(Mutex::new(readonly_db)));

    let response = bridge
        .invoke_ability(
            McpSessionId::from_uuid(uuid::Uuid::from_u128(349)),
            "get_entity_context",
            json!({
                "schema_version": 1,
                "entity_type": "person",
                "entity_id": PERSON_ID,
                "depth": "standard",
            }),
            false,
            None,
        )
        .await
        .expect("MCP bridge should invoke get_entity_context with read-only readers");

    assert_eq!(response.ability_name, "get_entity_context");
    let entries = response
        .data
        .as_array()
        .expect("get_entity_context data remains an array after MCP rendering");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["id"], claim_id);
    assert_eq!(entries[0]["entityType"], "person");
    assert_eq!(entries[0]["entityId"], PERSON_ID);
}

fn seed_person_claim(ctx: &ServiceContext<'_>, db: &ActionDb) -> String {
    let committed = commit_claim(
        ctx,
        db,
        ClaimProposal {
            id: None,
            subject_ref: json!({
                "kind": "person",
                "id": PERSON_ID,
            })
            .to_string(),
            claim_type: "attendee_context".to_string(),
            field_path: Some("context.mcp_dynamic".to_string()),
            topic_key: None,
            text: CLAIM_TEXT.to_string(),
            actor: "agent:test".to_string(),
            data_source: "user".to_string(),
            source_ref: None,
            source_asof: Some(TS.to_string()),
            observed_at: TS.to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(ClaimSensitivity::Internal),
            supersedes: None,
            tombstone: None,
        },
    )
    .expect("commit fixture claim");

    match committed {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected inserted claim, got {other:?}"),
    }
}

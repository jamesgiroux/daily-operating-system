#![cfg(feature = "test-harness")]

use std::sync::{Arc, Mutex};

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::{AbilityRegistry, Actor};
use dailyos_lib::bridges::tauri::TauriAbilityBridge;
use dailyos_lib::bridges::BridgeSurface;
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::intelligence::provider::ReplayProvider;
use dailyos_lib::services::claims::{
    commit_claim, load_claims_active, ClaimProposal, CommittedClaim,
};
use dailyos_lib::services::context::{
    EntityContextClaimReadFuture, EntityContextClaimReadHandle, ExternalClients, FixedClock,
    SeedableRng, ServiceContext,
};
use dailyos_lib::services::sensitivity::{
    reveal_claim_text_for_tauri, RenderActor, RenderPolicyKind, RenderSurface,
};
use rusqlite::Connection;
use serde_json::json;

const CLAIMS_SCHEMA_SQL: &str = include_str!("../src/migrations/129_dos_7_claims_schema.sql");
const PROJECTION_STATUS_SQL: &str =
    include_str!("../src/migrations/134_dos_301_claim_projection_status.sql");
const TYPED_FEEDBACK_SQL: &str =
    include_str!("../src/migrations/135_dos_294_typed_feedback_schema.sql");
const REVEAL_AUDIT_ACTION_TOKEN_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS sensitivity_reveal_audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    claim_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    revealed_at TEXT NOT NULL,
    reveal_action_id TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (claim_id) REFERENCES intelligence_claims(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_claim
    ON sensitivity_reveal_audit(claim_id, revealed_at);
CREATE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_user
    ON sensitivity_reveal_audit(user_id, revealed_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_action_token
    ON sensitivity_reveal_audit(claim_id, user_id, reveal_action_id)
    WHERE reveal_action_id != '';
"#;
const MINIMAL_ENTITY_SCHEMA_SQL: &str = r#"
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    claim_version INTEGER NOT NULL DEFAULT 0
);
"#;

const ACCOUNT_ID: &str = "acct-dos412-kk-example";
const CONFIDENTIAL_TEXT: &str = "confidential entity context risk for example.com.";
const INTERNAL_TEXT: &str = "internal entity context note for example.com.";
const USER_ONLY_TEXT: &str = "user-only entity context note for example.com.";

struct SqliteClaimReader {
    conn: Arc<Mutex<Connection>>,
}

impl EntityContextClaimReadHandle for SqliteClaimReader {
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        depth: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        let result = {
            let conn = self.conn.lock().expect("claim reader DB lock");
            let _ = depth;
            let subject_ref = json!({
                "kind": entity_type,
                "id": entity_id,
            })
            .to_string();
            load_claims_active(ActionDb::from_conn(&conn), &subject_ref, None)
                .map_err(|error| format!("claim read failed: {error}"))
        };
        Box::pin(std::future::ready(result))
    }
}

#[tokio::test]
async fn get_entity_context_tauri_bridge_wraps_confidential_claim_text_with_reveal_policy() {
    let conn = Arc::new(Mutex::new(fresh_claims_conn()));
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(412);
    let external = ExternalClients::default();
    let seed_ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("agent:test");
    let claim_id = {
        let conn = conn.lock().expect("seed DB lock");
        seed_confidential_entity_context_claim(&seed_ctx, &conn)
    };

    let services = ServiceContext::new_live(&clock, &rng, &external)
        .with_actor("user")
        .with_entity_context_claim_reader(Arc::new(SqliteClaimReader {
            conn: Arc::clone(&conn),
        }));
    let provider = ReplayProvider::new(std::collections::HashMap::new());
    let registry = AbilityRegistry::from_inventory_checked().expect("ability registry builds");
    let bridge = TauriAbilityBridge::new(&registry);

    let response = bridge
        .invoke_with_service_context_for_tests(
            &services,
            &provider,
            "get_entity_context",
            json!({
                "schema_version": 1,
                "entity_type": "account",
                "entity_id": ACCOUNT_ID,
                "depth": "standard",
            }),
        )
        .await
        .expect("Tauri ability bridge invocation succeeds");

    let entry = response.data[0]
        .as_object()
        .expect("entity context entry is an object");
    let content = entry
        .get("content")
        .and_then(serde_json::Value::as_object)
        .expect("claim-derived content is a carrier object, not a plain string");
    let title = entry
        .get("title")
        .and_then(serde_json::Value::as_object)
        .expect("claim-derived title is a carrier object, not a plain string");

    assert_confidential_click_to_reveal_carrier(content, &claim_id);
    assert_confidential_click_to_reveal_carrier(title, &claim_id);
    assert!(
        !serde_json::to_string(&response.data)
            .expect("response data serializes")
            .contains(CONFIDENTIAL_TEXT),
        "redacted Tauri carrier must not embed source text before reveal"
    );

    {
        let conn = conn.lock().expect("audit DB lock");
        assert_eq!(reveal_audit_count(&conn), 0);
        let rendered = reveal_claim_text_for_tauri(
            ActionDb::from_conn(&conn),
            &claim_id,
            RenderSurface::TauriEntityDetail,
            &RenderActor::user("user", Some("user")),
            "44444444-4444-4444-8444-444444444444".to_string(),
        )
        .expect("confidential claim reveals through audited Tauri path");
        assert_eq!(rendered.text, CONFIDENTIAL_TEXT);
        assert_eq!(rendered.policy.kind, RenderPolicyKind::Render);
        assert_eq!(reveal_audit_count(&conn), 1);
    }
}

#[tokio::test]
async fn get_entity_context_agent_mcp_bridge_filters_user_only_claims() {
    let conn = Arc::new(Mutex::new(fresh_claims_conn()));
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(264);
    let external = ExternalClients::default();
    let seed_ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("agent:test");
    {
        let conn = conn.lock().expect("seed DB lock");
        seed_entity_context_claim(
            &seed_ctx,
            &conn,
            "claim-dos264-agent-readable",
            INTERNAL_TEXT,
            ClaimSensitivity::Internal,
            "agent:test",
        );
        seed_entity_context_claim(
            &seed_ctx,
            &conn,
            "claim-dos264-user-only",
            USER_ONLY_TEXT,
            ClaimSensitivity::UserOnly,
            "user",
        );
    }

    let services = ServiceContext::new_live(&clock, &rng, &external)
        .with_actor("user")
        .with_entity_context_claim_reader(Arc::new(SqliteClaimReader {
            conn: Arc::clone(&conn),
        }));
    let provider = ReplayProvider::new(std::collections::HashMap::new());
    let registry = AbilityRegistry::from_inventory_checked().expect("ability registry builds");
    let bridge = TauriAbilityBridge::new(&registry);
    let input = json!({
        "schema_version": 1,
        "entity_type": "account",
        "entity_id": ACCOUNT_ID,
        "depth": "standard",
    });

    let user_response = bridge
        .invoke_with_service_context_for_tests(
            &services,
            &provider,
            "get_entity_context",
            input.clone(),
        )
        .await
        .expect("User/Tauri ability bridge invocation succeeds");
    let agent_response = bridge
        .invoke_with_service_context_for_tests_as(
            &services,
            &provider,
            Actor::Agent,
            BridgeSurface::McpTool,
            "get_entity_context",
            input,
        )
        .await
        .expect("Agent/MCP ability bridge invocation succeeds");

    assert_eq!(
        response_entry_ids(&user_response.data),
        vec!["claim-dos264-agent-readable", "claim-dos264-user-only"]
    );
    assert_eq!(
        response_entry_ids(&agent_response.data),
        vec!["claim-dos264-agent-readable"],
        "Agent/MCP context must not receive the full User view"
    );
    assert_eq!(
        agent_response.rendered_provenance.surface,
        BridgeSurface::McpTool
    );
}

fn fresh_claims_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory DB");
    conn.execute_batch(MINIMAL_ENTITY_SCHEMA_SQL)
        .expect("apply minimal entity schema");
    conn.execute(
        "INSERT INTO accounts (id, claim_version) VALUES (?1, 0)",
        [ACCOUNT_ID],
    )
    .expect("seed account");
    conn.execute_batch(CLAIMS_SCHEMA_SQL)
        .expect("apply claims schema");
    conn.execute_batch(TYPED_FEEDBACK_SQL)
        .expect("apply typed feedback schema");
    conn.execute_batch(PROJECTION_STATUS_SQL)
        .expect("apply projection status schema");
    conn.execute_batch(REVEAL_AUDIT_ACTION_TOKEN_SCHEMA_SQL)
        .expect("apply reveal audit action token schema");
    conn
}

fn seed_confidential_entity_context_claim(ctx: &ServiceContext<'_>, conn: &Connection) -> String {
    let committed = commit_claim(
        ctx,
        ActionDb::from_conn(conn),
        ClaimProposal {
            id: Some("claim-dos412-kk-confidential".to_string()),
            subject_ref: json!({
                "kind": "account",
                "id": ACCOUNT_ID,
            })
            .to_string(),
            claim_type: "entity_summary".to_string(),
            field_path: Some("context.risk".to_string()),
            topic_key: None,
            text: CONFIDENTIAL_TEXT.to_string(),
            actor: "agent:test".to_string(),
            data_source: "user".to_string(),
            source_ref: Some("fixture:example.com/entity-context".to_string()),
            source_asof: Some("2026-05-06T12:00:00Z".to_string()),
            observed_at: "2026-05-06T12:00:00Z".to_string(),
            provenance_json: json!({
                "source": "dos412-cycle8-regression",
                "domain": "example.com"
            })
            .to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(ClaimSensitivity::Confidential),
            supersedes: None,
            tombstone: None,
        },
    )
    .expect("commit confidential entity context claim");

    match committed {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected inserted claim, got {other:?}"),
    }
}

fn seed_entity_context_claim(
    ctx: &ServiceContext<'_>,
    conn: &Connection,
    id: &str,
    text: &str,
    sensitivity: ClaimSensitivity,
    actor: &str,
) -> String {
    let claim_type = if sensitivity == ClaimSensitivity::UserOnly {
        "user_note"
    } else {
        "entity_summary"
    };
    let field_path = if sensitivity == ClaimSensitivity::UserOnly {
        None
    } else {
        Some(format!("context.{id}"))
    };
    let committed = commit_claim(
        ctx,
        ActionDb::from_conn(conn),
        ClaimProposal {
            id: Some(id.to_string()),
            subject_ref: json!({
                "kind": "account",
                "id": ACCOUNT_ID,
            })
            .to_string(),
            claim_type: claim_type.to_string(),
            field_path,
            topic_key: None,
            text: text.to_string(),
            actor: actor.to_string(),
            data_source: "user".to_string(),
            source_ref: Some(format!("fixture:example.com/{id}")),
            source_asof: Some("2026-05-06T12:00:00Z".to_string()),
            observed_at: "2026-05-06T12:00:00Z".to_string(),
            provenance_json: json!({
                "source": "dos264-cycle2-regression",
                "domain": "example.com"
            })
            .to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(sensitivity),
            supersedes: None,
            tombstone: None,
        },
    )
    .expect("commit entity context claim");

    match committed {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected inserted claim, got {other:?}"),
    }
}

fn response_entry_ids(data: &serde_json::Value) -> Vec<&str> {
    let mut ids = data
        .as_array()
        .expect("response data is an array")
        .iter()
        .map(|entry| {
            entry
                .get("id")
                .and_then(serde_json::Value::as_str)
                .expect("entry has id")
        })
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

fn assert_confidential_click_to_reveal_carrier(
    carrier: &serde_json::Map<String, serde_json::Value>,
    claim_id: &str,
) {
    assert_eq!(
        carrier.get("text").and_then(serde_json::Value::as_str),
        Some("Confidential claim hidden")
    );
    let policy = carrier
        .get("policy")
        .and_then(serde_json::Value::as_object)
        .expect("carrier includes render policy");
    assert_eq!(
        policy.get("kind").and_then(serde_json::Value::as_str),
        Some("redacted")
    );
    assert_eq!(
        policy
            .get("sensitivity")
            .and_then(serde_json::Value::as_str),
        Some("confidential")
    );
    assert_eq!(
        policy.get("claimId").and_then(serde_json::Value::as_str),
        Some(claim_id)
    );
    let affordance = policy
        .get("affordance")
        .and_then(serde_json::Value::as_object)
        .expect("confidential carrier includes reveal affordance");
    assert_eq!(
        affordance.get("kind").and_then(serde_json::Value::as_str),
        Some("confidential_click_to_reveal")
    );
    assert_eq!(
        affordance
            .get("claimId")
            .or_else(|| affordance.get("claim_id"))
            .and_then(serde_json::Value::as_str),
        Some(claim_id)
    );
}

fn reveal_audit_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM sensitivity_reveal_audit", [], |row| {
        row.get::<_, i64>(0)
    })
    .expect("count reveal audit rows")
}

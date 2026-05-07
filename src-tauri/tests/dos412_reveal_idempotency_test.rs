use chrono::{TimeZone, Utc};
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::claims::{commit_claim, ClaimProposal, CommittedClaim};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
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
const REVEAL_AUDIT_SQL: &str = include_str!("../src/migrations/142_sensitivity_reveal_audit.sql");
const REVEAL_AUDIT_IDEMPOTENCY_SQL: &str =
    include_str!("../src/migrations/143_sensitivity_reveal_audit_idempotency.sql");

const ACCOUNT_ID: &str = "acct-dos412-idempotency-example";
const CONFIDENTIAL_TEXT: &str = "confidential renewal blocker for example.com.";
const REVEAL_SESSION_ID: &str = "dos412-cycle10-reveal-session";

#[test]
fn reveal_sensitive_claim_text_is_idempotent_for_same_reveal_session() {
    let conn = setup_conn();
    let db = ActionDb::from_conn(&conn);
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(41210);
    let external = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("agent:test");
    let claim_id = inserted_id(
        commit_claim(&ctx, db, confidential_claim_proposal())
            .expect("commit confidential claim fixture"),
    );
    let actor = RenderActor::user("user", Some("user"));

    let first = reveal_claim_text_for_tauri(
        db,
        &claim_id,
        RenderSurface::TauriEntityDetail,
        &actor,
        Some(REVEAL_SESSION_ID),
    )
    .expect("first reveal succeeds");
    let second = reveal_claim_text_for_tauri(
        db,
        &claim_id,
        RenderSurface::TauriEntityDetail,
        &actor,
        Some(REVEAL_SESSION_ID),
    )
    .expect("second reveal with same session succeeds");

    assert_eq!(first.text, CONFIDENTIAL_TEXT);
    assert_eq!(second.text, CONFIDENTIAL_TEXT);
    assert_eq!(first.policy.kind, RenderPolicyKind::Render);
    assert_eq!(second.policy.kind, RenderPolicyKind::Render);
    assert_eq!(reveal_audit_count(&conn), 1);
    assert_eq!(reveal_audit_count_for_session(&conn, REVEAL_SESSION_ID), 1);
}

fn setup_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(
        "CREATE TABLE accounts (
            id TEXT PRIMARY KEY,
            claim_version INTEGER NOT NULL DEFAULT 0
        );",
    )
    .expect("create account table");
    conn.execute(
        "INSERT INTO accounts (id, claim_version) VALUES (?1, 0)",
        [ACCOUNT_ID],
    )
    .expect("seed account");
    conn.execute_batch(CLAIMS_SCHEMA_SQL)
        .expect("apply claims schema");
    conn.execute_batch(PROJECTION_STATUS_SQL)
        .expect("apply projection status schema");
    conn.execute_batch(TYPED_FEEDBACK_SQL)
        .expect("apply typed feedback schema");
    conn.execute_batch(REVEAL_AUDIT_SQL)
        .expect("apply reveal audit schema");
    conn.execute_batch(REVEAL_AUDIT_IDEMPOTENCY_SQL)
        .expect("apply reveal audit idempotency schema");
    conn
}

fn confidential_claim_proposal() -> ClaimProposal {
    ClaimProposal {
        id: Some("claim-dos412-idempotent-reveal".to_string()),
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
        source_ref: Some("fixture:example.com/reveal-idempotency".to_string()),
        source_asof: Some("2026-05-07T12:00:00Z".to_string()),
        observed_at: "2026-05-07T12:00:00Z".to_string(),
        provenance_json: json!({
            "source": "dos412-cycle10-regression",
            "domain": "example.com"
        })
        .to_string(),
        metadata_json: None,
        thread_id: None,
        temporal_scope: Some(TemporalScope::State),
        sensitivity: Some(ClaimSensitivity::Confidential),
        supersedes: None,
        tombstone: None,
    }
}

fn inserted_id(result: CommittedClaim) -> String {
    match result {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected inserted claim, got {other:?}"),
    }
}

fn reveal_audit_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM sensitivity_reveal_audit", [], |row| {
        row.get(0)
    })
    .expect("count reveal audit rows")
}

fn reveal_audit_count_for_session(conn: &Connection, reveal_session_id: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*)
         FROM sensitivity_reveal_audit
         WHERE reveal_session_id = ?1",
        [reveal_session_id],
        |row| row.get(0),
    )
    .expect("count reveal audit rows for session")
}

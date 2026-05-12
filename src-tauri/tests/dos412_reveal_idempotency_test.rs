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
const STRUCTURED_CLAIM_CANONICALIZATION_COLUMNS_SQL: &str = r#"
ALTER TABLE intelligence_claims ADD COLUMN structured_claim_json TEXT;
ALTER TABLE intelligence_claims ADD COLUMN predicate_ref TEXT;
ALTER TABLE intelligence_claims ADD COLUMN polarity TEXT;
ALTER TABLE intelligence_claims ADD COLUMN object_value JSON;
ALTER TABLE intelligence_claims ADD COLUMN qualifiers JSON;
ALTER TABLE intelligence_claims ADD COLUMN structural_canonical_id TEXT;
ALTER TABLE intelligence_claims ADD COLUMN canonical_status TEXT NOT NULL DEFAULT 'pending_backfill'
    CHECK (canonical_status IN ('pending_backfill','legacy_unmigrated','live'));
ALTER TABLE intelligence_claims ADD COLUMN non_semantic_mergeable BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE intelligence_claims ADD COLUMN structural_field_content_hash TEXT;
ALTER TABLE intelligence_claims ADD COLUMN backfill_epoch INTEGER NOT NULL DEFAULT 0;
"#;
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

const ACCOUNT_ID: &str = "acct-dos412-idempotency-example";
const CONFIDENTIAL_TEXT: &str = "confidential renewal blocker for example.com.";
const ACTION_ID_ONE: &str = "11111111-1111-4111-8111-111111111111";
const ACTION_ID_TWO: &str = "22222222-2222-4222-8222-222222222222";
const ACTION_ID_WITH_ALPHA: &str = "abcdefab-cdef-4abc-8def-abcdefabcdef";

#[test]
fn reveal_sensitive_claim_text_is_idempotent_for_same_action_id() {
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
        ACTION_ID_ONE.to_string(),
    )
    .expect("first reveal succeeds");
    let second = reveal_claim_text_for_tauri(
        db,
        &claim_id,
        RenderSurface::TauriEntityDetail,
        &actor,
        ACTION_ID_ONE.to_string(),
    )
    .expect("second reveal with same action id succeeds");

    assert_eq!(first.text, CONFIDENTIAL_TEXT);
    assert_eq!(second.text, CONFIDENTIAL_TEXT);
    assert_eq!(first.policy.kind, RenderPolicyKind::Render);
    assert_eq!(second.policy.kind, RenderPolicyKind::Render);
    assert_eq!(reveal_audit_count(&conn), 1);
    assert_eq!(
        reveal_audit_action_ids(&conn),
        vec![ACTION_ID_ONE.to_string()]
    );
}

#[test]
fn reveal_sensitive_claim_text_records_new_audit_for_different_action_ids() {
    let conn = setup_conn();
    let db = ActionDb::from_conn(&conn);
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(41211);
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
        ACTION_ID_ONE.to_string(),
    )
    .expect("first reveal succeeds");
    let second = reveal_claim_text_for_tauri(
        db,
        &claim_id,
        RenderSurface::TauriEntityDetail,
        &actor,
        ACTION_ID_TWO.to_string(),
    )
    .expect("second reveal with different action id succeeds");

    assert_eq!(first.text, CONFIDENTIAL_TEXT);
    assert_eq!(second.text, CONFIDENTIAL_TEXT);
    assert_eq!(reveal_audit_count(&conn), 2);
    assert_eq!(
        reveal_audit_action_ids(&conn),
        vec![ACTION_ID_ONE.to_string(), ACTION_ID_TWO.to_string()]
    );
}

#[test]
fn reveal_sensitive_claim_text_is_idempotent_for_same_logical_uuid_in_different_text_forms() {
    let conn = setup_conn();
    let db = ActionDb::from_conn(&conn);
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(41214);
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
        ACTION_ID_WITH_ALPHA.to_string(),
    )
    .expect("first reveal succeeds");
    let second = reveal_claim_text_for_tauri(
        db,
        &claim_id,
        RenderSurface::TauriEntityDetail,
        &actor,
        ACTION_ID_WITH_ALPHA.to_ascii_uppercase(),
    )
    .expect("second reveal with same logical UUID succeeds");

    assert_eq!(first.text, CONFIDENTIAL_TEXT);
    assert_eq!(second.text, CONFIDENTIAL_TEXT);
    assert_eq!(reveal_audit_count(&conn), 1);
    assert_eq!(
        reveal_audit_action_ids(&conn),
        vec![ACTION_ID_WITH_ALPHA.to_string()]
    );
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
    conn.execute_batch(STRUCTURED_CLAIM_CANONICALIZATION_COLUMNS_SQL)
        .expect("apply structured claim canonicalization columns");
    conn.execute_batch(REVEAL_AUDIT_ACTION_TOKEN_SCHEMA_SQL)
        .expect("apply reveal audit action token schema");
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

fn reveal_audit_action_ids(conn: &Connection) -> Vec<String> {
    let mut stmt = conn
        .prepare("SELECT reveal_action_id FROM sensitivity_reveal_audit ORDER BY id")
        .expect("query reveal audit action ids");
    stmt.query_map([], |row| row.get::<_, String>(0))
        .expect("read reveal audit action ids")
        .map(|row| row.expect("action id row"))
        .collect()
}

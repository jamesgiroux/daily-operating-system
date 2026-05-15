#[cfg(feature = "test-harness")]
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::claims::{
    commit_claim, ClaimProposal, CommittedClaim, DeterministicInsertProposal,
};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use dailyos_lib::services::sensitivity::{
    canonicalize_reveal_action_id, reveal_claim_text_for_tauri,
    validate_canonical_reveal_action_id, RenderActor, RenderPolicyKind, RenderSurface,
};
#[cfg(feature = "test-harness")]
use dailyos_lib::state::AppState;
use rusqlite::Connection;
use serde_json::json;
#[cfg(feature = "test-harness")]
use tauri::Manager;

const CLAIMS_SCHEMA_SQL: &str = include_str!("../src/migrations/129_dos_7_claims_schema.sql");
const V172_SUBSTRATE_CONCURRENCY_SQL: &str =
    include_str!("./shared_schemas/v172_substrate_concurrency.sql");
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

const ACCOUNT_ID: &str = "acct-dos412-action-id-validation";
const CLAIM_ID: &str = "claim-dos412-action-id-validation";
const CONFIDENTIAL_TEXT: &str = "confidential action id validation payload.";
const CANONICAL_ACTION_ID: &str = "abcdefab-cdef-4abc-8def-abcdefabcdef";
const SIMPLE_ACTION_ID: &str = "abcdefabcdef4abc8defabcdefabcdef";

#[test]
fn empty_action_id_is_rejected() {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    let db = ActionDb::from_conn(&conn);
    let error = reveal_claim_text_for_tauri(
        db,
        "claim-any",
        RenderSurface::TauriEntityDetail,
        &RenderActor::user("user", Some("user")),
        String::new(),
    )
    .expect_err("empty action id must fail before DB access");

    assert!(error.contains("UUID v4"));
}

#[test]
fn malformed_action_id_is_rejected() {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    let db = ActionDb::from_conn(&conn);
    let error = reveal_claim_text_for_tauri(
        db,
        "claim-any",
        RenderSurface::TauriEntityDetail,
        &RenderActor::user("user", Some("user")),
        "not-a-uuid".to_string(),
    )
    .expect_err("malformed action id must fail before DB access");

    assert!(error.contains("UUID v4"));
}

#[test]
fn service_boundary_canonicalizes_supported_uuid_text_forms() {
    let cases = vec![
        (
            CANONICAL_ACTION_ID.to_ascii_uppercase(),
            CANONICAL_ACTION_ID,
        ),
        (SIMPLE_ACTION_ID.to_string(), CANONICAL_ACTION_ID),
        (format!("{{{CANONICAL_ACTION_ID}}}"), CANONICAL_ACTION_ID),
        (
            format!("urn:uuid:{CANONICAL_ACTION_ID}"),
            CANONICAL_ACTION_ID,
        ),
        (
            "ABCDEFAB-cDeF-4aBc-8DeF-aBcDeFaBcDeF".to_string(),
            CANONICAL_ACTION_ID,
        ),
    ];

    for (input, expected) in cases {
        assert_eq!(
            canonicalize_reveal_action_id(&input).expect("UUID v4 form canonicalizes"),
            expected
        );
    }
}

#[test]
fn command_boundary_validator_rejects_non_canonical_uuid_forms() {
    let rejected = vec![
        SIMPLE_ACTION_ID.to_string(),
        format!("{{{CANONICAL_ACTION_ID}}}"),
        format!("urn:uuid:{CANONICAL_ACTION_ID}"),
        CANONICAL_ACTION_ID.to_ascii_uppercase(),
        "ABCDEFAB-cDeF-4aBc-8DeF-aBcDeFaBcDeF".to_string(),
    ];

    for input in rejected {
        let error = validate_canonical_reveal_action_id(&input)
            .expect_err("command boundary must reject non-canonical UUID text");
        assert!(error.contains("canonical lowercase hyphenated UUID v4"));
    }
    validate_canonical_reveal_action_id(CANONICAL_ACTION_ID)
        .expect("canonical lowercase hyphenated UUID v4 is accepted");
}

#[cfg(feature = "test-harness")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn command_boundary_rejects_simple_uuid_before_service() {
    let temp_dir = tempfile::tempdir().expect("create isolated DB dir");
    let db_service = dailyos_lib::db_service::DbService::open_at_unencrypted_for_tests(
        temp_dir.path().join("action-id-command-boundary.db"),
    )
    .await
    .expect("open isolated DB service");
    let state = Arc::new(AppState::test_with_db_service(db_service));
    let app = tauri::test::mock_builder()
        .manage(Arc::clone(&state))
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .expect("build mock Tauri app");

    let error = dailyos_lib::command_test_api::reveal_sensitive_claim_text(
        "claim-any".to_string(),
        SIMPLE_ACTION_ID.to_string(),
        None,
        app.state::<Arc<AppState>>(),
    )
    .await
    .expect_err("simple UUID must fail at command boundary");

    assert!(error.contains("canonical lowercase hyphenated UUID v4"));
}

#[test]
fn valid_uuid_v4_action_id_is_accepted() {
    let conn = setup_conn("valid-uuid");
    let db = ActionDb::from_conn(&conn);
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(41212);
    let external = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("agent:test");
    let claim_id = inserted_id(
        commit_claim(&ctx, db, confidential_claim_proposal("valid-uuid"))
            .expect("commit confidential claim fixture"),
    );

    let rendered = reveal_claim_text_for_tauri(
        db,
        &claim_id,
        RenderSurface::TauriEntityDetail,
        &RenderActor::user("user", Some("user")),
        "33333333-3333-4333-8333-333333333333".to_string(),
    )
    .expect("valid UUID v4 action id reveals");

    assert_eq!(rendered.text, CONFIDENTIAL_TEXT);
    assert_eq!(rendered.policy.kind, RenderPolicyKind::Render);
}

#[test]
fn service_boundary_stores_canonical_action_id_and_dedupes_equivalent_text_forms() {
    let conn = setup_conn("canonical-dedupe");
    let db = ActionDb::from_conn(&conn);
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(41213);
    let external = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("agent:test");
    let claim_id = inserted_id(
        commit_claim(&ctx, db, confidential_claim_proposal("canonical-dedupe"))
            .expect("commit confidential claim fixture"),
    );
    let actor = RenderActor::user("user", Some("user"));

    let first = reveal_claim_text_for_tauri(
        db,
        &claim_id,
        RenderSurface::TauriEntityDetail,
        &actor,
        CANONICAL_ACTION_ID.to_ascii_uppercase(),
    )
    .expect("uppercase UUID v4 reveals through service boundary");
    let second = reveal_claim_text_for_tauri(
        db,
        &claim_id,
        RenderSurface::TauriEntityDetail,
        &actor,
        CANONICAL_ACTION_ID.to_string(),
    )
    .expect("canonical UUID v4 reveals through service boundary");

    assert_eq!(first.text, CONFIDENTIAL_TEXT);
    assert_eq!(second.text, CONFIDENTIAL_TEXT);
    assert_eq!(reveal_audit_count(&conn), 1);
    assert_eq!(
        reveal_audit_action_ids(&conn),
        vec![CANONICAL_ACTION_ID.to_string()]
    );
}

fn setup_conn(discriminator: &str) -> Connection {
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
        [format!("{ACCOUNT_ID}-{discriminator}").as_str()],
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
    conn.execute_batch(V172_SUBSTRATE_CONCURRENCY_SQL)
        .expect("apply v172 substrate concurrency schema");
    conn.execute_batch(REVEAL_AUDIT_ACTION_TOKEN_SCHEMA_SQL)
        .expect("apply reveal audit action token schema");
    conn
}

// Per-test discriminator keeps the substrate's process-global commit lock
// (keyed by subject_ref + claim_type + field_path) from serializing parallel
// tests that exercise the same logical fixture in separate in-memory
// connections.
fn confidential_claim_proposal(discriminator: &str) -> DeterministicInsertProposal {
    DeterministicInsertProposal::new(
        format!("{CLAIM_ID}-{discriminator}"),
        confidential_claim_inner(discriminator),
    )
}

fn confidential_claim_inner(discriminator: &str) -> ClaimProposal {
    ClaimProposal {
        id: None,
        expected_claim_version: None,
        subject_ref: json!({
            "kind": "account",
            "id": format!("{ACCOUNT_ID}-{discriminator}"),
        })
        .to_string(),
        claim_type: "entity_summary".to_string(),
        field_path: Some("context.risk".to_string()),
        topic_key: None,
        text: CONFIDENTIAL_TEXT.to_string(),
        actor: "agent:test".to_string(),
        data_source: "user".to_string(),
        source_ref: Some("fixture:example.com/action-id-validation".to_string()),
        source_asof: Some("2026-05-07T12:00:00Z".to_string()),
        observed_at: "2026-05-07T12:00:00Z".to_string(),
        provenance_json: json!({
            "source": "dos412-action-id-validation",
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

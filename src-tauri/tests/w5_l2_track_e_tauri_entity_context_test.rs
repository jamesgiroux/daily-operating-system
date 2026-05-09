#![cfg(feature = "test-harness")]

use std::sync::{Arc, Mutex};

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::feedback::FeedbackAction;
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::claims::{
    commit_claim, load_entity_context_claims_active_for_surface, record_claim_feedback,
    ClaimFeedbackInput, ClaimProposal, CommittedClaim,
};
use dailyos_lib::services::context::{
    EntityContextClaimReadFuture, EntityContextClaimReadHandle, ExternalClients, FixedClock,
    SeedableRng, ServiceContext,
};
use dailyos_lib::state::AppState;
use rusqlite::Connection;
use tauri::Manager;

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
struct SqliteClaimReader {
    conn: Mutex<Connection>,
}

impl EntityContextClaimReadHandle for SqliteClaimReader {
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        depth: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        let result = {
            let conn = self.conn.lock().expect("claim reader db lock");
            load_entity_context_claims_active_for_surface(
                ActionDb::from_conn(&conn),
                &entity_type,
                &entity_id,
                depth,
                "tauri_entity_detail",
            )
            .map_err(|error| format!("claim read failed: {error}"))
        };
        Box::pin(std::future::ready(result))
    }
}

fn fresh_claims_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(MINIMAL_ENTITY_SCHEMA_SQL)
        .expect("apply minimal entity schema");
    conn.execute_batch(CLAIMS_SCHEMA_SQL)
        .expect("apply claims schema");
    conn.execute_batch(PROJECTION_STATUS_SQL)
        .expect("apply projection status schema");
    conn.execute_batch(TYPED_FEEDBACK_SQL)
        .expect("apply typed feedback schema");
    conn
}

fn seed_claim(
    ctx: &ServiceContext<'_>,
    conn: &Connection,
    subject_ref: &str,
    text: &str,
    field_path: &str,
) -> String {
    let committed = commit_claim(
        ctx,
        ActionDb::from_conn(conn),
        ClaimProposal {
            id: None,
            subject_ref: subject_ref.to_string(),
            claim_type: "attendee_context".to_string(),
            field_path: Some(field_path.to_string()),
            topic_key: None,
            text: text.to_string(),
            actor: "agent:test".to_string(),
            data_source: "user".to_string(),
            source_ref: None,
            source_asof: Some("2026-05-06T12:00:00Z".to_string()),
            observed_at: "2026-05-06T12:00:00Z".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: None,
            sensitivity: None,
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

fn apply_feedback(
    ctx: &ServiceContext<'_>,
    conn: &Connection,
    claim_id: &str,
    action: FeedbackAction,
) {
    record_claim_feedback(
        ctx,
        ActionDb::from_conn(conn),
        ClaimFeedbackInput {
            claim_id: claim_id.to_string(),
            action,
            actor: "user:test".to_string(),
            actor_id: Some("user-test".to_string()),
            payload_json: None,
        },
    )
    .expect("record claim feedback");
}

#[tokio::test]
async fn workspace_entity_context_handler_filters_inactive_claim_rows() {
    let conn = fresh_claims_conn();
    let subject_ref = serde_json::json!({
        "kind": "person",
        "id": "person-track-e",
    })
    .to_string();

    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(218);
    let external = ExternalClients::default();
    let seed_ctx = ServiceContext::new_live(&clock, &rng, &external);

    let active_id = seed_claim(
        &seed_ctx,
        &conn,
        &subject_ref,
        "active surfaced context",
        "context.active",
    );
    let surfacing_dormant_id = seed_claim(
        &seed_ctx,
        &conn,
        &subject_ref,
        "surfacing dormant context must stay hidden",
        "context.surfacing_dormant",
    );
    apply_feedback(
        &seed_ctx,
        &conn,
        &surfacing_dormant_id,
        FeedbackAction::MarkOutdated,
    );
    let tombstoned_id = seed_claim(
        &seed_ctx,
        &conn,
        &subject_ref,
        "tombstoned context must stay hidden",
        "context.tombstoned",
    );
    apply_feedback(
        &seed_ctx,
        &conn,
        &tombstoned_id,
        FeedbackAction::WrongSubject,
    );

    let reader = Arc::new(SqliteClaimReader {
        conn: Mutex::new(conn),
    });
    let services = ServiceContext::new_live(&clock, &rng, &external)
        .with_actor("user")
        .with_entity_context_claim_reader(reader);

    let entries = services
        .read_entity_context_claim_entries("person".to_string(), "person-track-e".to_string(), 1)
        .await
        .expect("workspace entity context handler read succeeds");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, active_id);
    assert_eq!(entries[0].content, "active surfaced context");
    assert_eq!(entries[0].entity_type, "person");
    assert_eq!(entries[0].entity_id, "person-track-e");
}

#[test]
fn entity_context_service_no_longer_writes_legacy_table() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/entity_context.rs"),
    )
    .expect("read entity context service source");

    assert!(
        source.contains("commit_claim("),
        "entity context writes must route through commit_claim"
    );
    assert!(
        source.contains("withdraw_claim("),
        "entity context deletes must route through withdraw_claim"
    );
    assert!(
        !source.contains("INSERT INTO entity_context_entries")
            && !source.contains("UPDATE entity_context_entries")
            && !source.contains("DELETE FROM entity_context_entries"),
        "entity context service must not write the legacy table"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_tauri_create_edit_delete_read_roundtrip_uses_user_note_claims() {
    let temp_dir = tempfile::tempdir().expect("create isolated DB dir");
    let db_service = dailyos_lib::db_service::DbService::open_at_unencrypted_for_tests(
        temp_dir.path().join("entity-context-command.db"),
    )
    .await
    .expect("open isolated DB service");
    let state = Arc::new(AppState::test_with_db_service(db_service));
    let app = tauri::test::mock_builder()
        .manage(Arc::clone(&state))
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .expect("build mock Tauri app");

    let created = dailyos_lib::command_test_api::create_entity_context_entry(
        "account".to_string(),
        "account-track-h".to_string(),
        "Renewal note".to_string(),
        "Customer asked for renewal risk follow-up.".to_string(),
        app.state::<Arc<AppState>>(),
    )
    .await
    .expect("create entity context entry through Tauri command");

    let entries = dailyos_lib::command_test_api::get_entity_context_entries(
        "account".to_string(),
        "account-track-h".to_string(),
        app.state::<Arc<AppState>>(),
    )
    .await
    .expect("read entity context entries through Tauri command");

    let read_back = entries
        .iter()
        .find(|entry| entry.id == created.id)
        .expect("created entry is visible to read command");

    assert_eq!(read_back.entity_type, "account");
    assert_eq!(read_back.entity_id, "account-track-h");
    assert_eq!(read_back.title, "Renewal note");
    assert_eq!(
        read_back.content,
        "Customer asked for renewal risk follow-up."
    );

    dailyos_lib::command_test_api::update_entity_context_entry(
        created.id.clone(),
        "Renewal note edited".to_string(),
        "Customer asked for renewal risk follow-up before Friday.".to_string(),
        app.state::<Arc<AppState>>(),
    )
    .await
    .expect("update entity context entry through Tauri command");

    let entries_after_edit = dailyos_lib::command_test_api::get_entity_context_entries(
        "account".to_string(),
        "account-track-h".to_string(),
        app.state::<Arc<AppState>>(),
    )
    .await
    .expect("read entity context entries after edit");

    assert!(
        !entries_after_edit
            .iter()
            .any(|entry| entry.id == created.id),
        "superseded note claim should be hidden from active reads"
    );
    let edited = entries_after_edit
        .iter()
        .find(|entry| entry.title == "Renewal note edited")
        .expect("edited entry is visible after supersession");
    assert_eq!(
        edited.content,
        "Customer asked for renewal risk follow-up before Friday."
    );

    dailyos_lib::command_test_api::delete_entity_context_entry(
        edited.id.clone(),
        app.state::<Arc<AppState>>(),
    )
    .await
    .expect("delete entity context entry through Tauri command");

    let entries_after_delete = dailyos_lib::command_test_api::get_entity_context_entries(
        "account".to_string(),
        "account-track-h".to_string(),
        app.state::<Arc<AppState>>(),
    )
    .await
    .expect("read entity context entries after delete");

    assert!(
        entries_after_delete
            .iter()
            .all(|entry| entry.id != edited.id && entry.id != created.id),
        "withdrawn and superseded note claims should be hidden"
    );
}

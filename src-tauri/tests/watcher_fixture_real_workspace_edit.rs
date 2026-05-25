use std::path::{Path, PathBuf};

use abilities_runtime::abilities::provenance::source::{EntityId, WorkspaceFileKind};
use chrono::{DateTime, Utc};
use dailyos_lib::db::{ActionDb, DbAccount};
use dailyos_lib::entity::EntityType;
use dailyos_lib::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use dailyos_lib::services::workspace_ingestion::contracts::RejectionReason;
use dailyos_lib::services::workspace_ingestion::lifecycle::LifecycleState;
use dailyos_lib::services::workspace_ingestion::pipeline::{
    file_id_from_identity, EntityRef, IngestError, IngestPipeline, IngestRequest,
};
use dailyos_lib::services::workspace_ingestion::registry::WorkspaceSourceRegistry;
use dailyos_lib::services::workspace_ingestion::runs::IngestionMode;
use dailyos_lib::services::workspace_ingestion::wiring;
use rusqlite::Connection;

#[test]
fn real_workspace_edit_creates_lifecycle_run_preserves_emit_and_markdown_shape() {
    let conn = Connection::open_in_memory().expect("sqlite");
    dailyos_lib::migration_test_api::run_migrations(&conn).expect("migrations");
    let db = ActionDb::from_conn(&conn);
    let workspace = tempfile::tempdir().expect("workspace");
    let workspace_root = workspace
        .path()
        .canonicalize()
        .expect("canonical workspace");
    let account_dir = workspace_root.join("Accounts").join("Acme");
    std::fs::create_dir_all(&account_dir).expect("account dir");

    let account = DbAccount {
        id: "acme".to_string(),
        name: "Acme".to_string(),
        tracker_path: Some("Accounts/Acme".to_string()),
        updated_at: Utc::now().to_rfc3339(),
        ..Default::default()
    };
    db.upsert_account(&account).expect("account upsert");
    dailyos_lib::accounts::write_account_markdown(&workspace_root, &account, None, db)
        .expect("markdown");

    let file_path = account_dir.join("meeting-notes.md");
    std::fs::write(&file_path, "# Meeting\n\nFollow up next week.").expect("notes");
    let pipeline = wiring::build_pipeline(workspace_root.clone());
    let file_id = run_pipeline(
        &pipeline,
        &db,
        &workspace_root,
        &file_path,
        WorkspaceFileKind::EntityDoc,
        Some(entity_ref(EntityType::Account, "acme", Some("Acme"))),
    )
    .expect("ingest succeeds");

    let lifecycle =
        dailyos_lib::services::workspace_ingestion::lifecycle::LifecycleRepo::get(&conn, &file_id)
            .expect("lifecycle get")
            .expect("lifecycle row");
    assert_eq!(lifecycle.lifecycle_state, LifecycleState::Ingested);
    let run_count: i64 = conn
        .query_row("SELECT count(*) FROM document_ingestion_runs", [], |row| {
            row.get(0)
        })
        .expect("run count");
    assert_eq!(run_count, 1);
    assert!(account_dir.join("dashboard.md").exists());

    let watcher =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/watcher.rs"))
            .expect("watcher source");
    assert!(watcher.contains("app_handle.emit(\"inbox-updated\""));
}

fn run_pipeline(
    pipeline: &IngestPipeline,
    db: &ActionDb,
    workspace_root: &Path,
    path: &Path,
    source_type: WorkspaceFileKind,
    entity: Option<EntityRef>,
) -> Result<String, IngestError> {
    let (file, identity) = WorkspaceSourceRegistry::open_validated(workspace_root, path)
        .map_err(IngestError::Rejected)?;
    let source_asof: DateTime<Utc> = identity
        .canonical_path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .map_err(IngestError::Io)?
        .into();
    let file_id = file_id_from_identity(&identity, workspace_root)
        .map_err(|_| IngestError::Rejected(RejectionReason::OutsideWorkspace))?;

    let request = IngestRequest {
        file,
        identity,
        file_id: file_id.clone(),
        source_asof,
        source_type,
        entity,
        mode: IngestionMode::Realtime,
        category_hint: None,
        invocation_actor: "system:test".to_string(),
        validated_content: None,
    };
    let clock = SystemClock;
    let rng = SystemRng;
    let external = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("system:test");
    let signal_engine = dailyos_lib::signals::propagation::default_engine();
    pipeline
        .run_with_signal_engine(&ctx, db, &signal_engine, request)
        .map(|_| file_id)
}

fn entity_ref(entity_type: EntityType, id: &str, name: Option<&str>) -> EntityRef {
    EntityRef {
        entity_type,
        entity_id: EntityId::new(id.to_string()),
        entity_name: name.map(str::to_string),
    }
}

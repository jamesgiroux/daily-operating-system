use std::path::Path;

use abilities_runtime::abilities::provenance::source::{EntityId, WorkspaceFileKind};
use chrono::{DateTime, Utc};
use dailyos_lib::db::ActionDb;
use dailyos_lib::entity::EntityType;
use dailyos_lib::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use dailyos_lib::services::workspace_ingestion::contracts::RejectionReason;
use dailyos_lib::services::workspace_ingestion::pipeline::{
    file_id_from_identity, EntityRef, IngestError, IngestPipeline, IngestRequest,
};
use dailyos_lib::services::workspace_ingestion::registry::WorkspaceSourceRegistry;
use dailyos_lib::services::workspace_ingestion::runs::IngestionMode;
use dailyos_lib::services::workspace_ingestion::wiring;
use rusqlite::Connection;

#[test]
fn duplicate_watcher_events_are_idempotent_against_w2_a_pipeline_receipt() {
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
    let file_path = account_dir.join("notes.md");
    std::fs::write(&file_path, "# Notes\n\nUseful context.").expect("notes");

    let pipeline = wiring::build_pipeline(workspace_root.clone());
    let entity = Some(entity_ref(EntityType::Account, "acme", Some("Acme")));
    run_pipeline(
        &pipeline,
        &db,
        &workspace_root,
        &file_path,
        WorkspaceFileKind::EntityDoc,
        entity.clone(),
    )
    .expect("first ingest succeeds");
    let duplicate = run_pipeline(
        &pipeline,
        &db,
        &workspace_root,
        &file_path,
        WorkspaceFileKind::EntityDoc,
        entity,
    );

    assert!(duplicate.is_err(), "duplicate event does not create a run");
    let run_count: i64 = conn
        .query_row("SELECT count(*) FROM document_ingestion_runs", [], |row| {
            row.get(0)
        })
        .expect("run count");
    assert_eq!(run_count, 1);
}

fn run_pipeline(
    pipeline: &IngestPipeline,
    db: &ActionDb,
    workspace_root: &Path,
    path: &Path,
    source_type: WorkspaceFileKind,
    entity: Option<EntityRef>,
) -> Result<(), IngestError> {
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
        file_id,
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
        .map(|_| ())
}

fn entity_ref(entity_type: EntityType, id: &str, name: Option<&str>) -> EntityRef {
    EntityRef {
        entity_type,
        entity_id: EntityId::new(id.to_string()),
        entity_name: name.map(str::to_string),
    }
}

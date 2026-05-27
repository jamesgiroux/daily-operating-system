use std::path::{Path, PathBuf};

use abilities_runtime::abilities::provenance::source::{EntityId, WorkspaceFileKind};
use chrono::{DateTime, Utc};
use dailyos_lib::db::{ActionDb, DbAccount};
use dailyos_lib::entity::EntityType;
use dailyos_lib::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use dailyos_lib::services::workspace_ingestion::contracts::{
    RejectionReason, SignalEmitContext, SignalEmitError, SignalEmitter,
};
use dailyos_lib::services::workspace_ingestion::extract::WorkspaceExtractor;
use dailyos_lib::services::workspace_ingestion::lifecycle::{LifecycleRepo, LifecycleState};
use dailyos_lib::services::workspace_ingestion::link::{LinkAttributionSource, LinkRepo};
use dailyos_lib::services::workspace_ingestion::pipeline::{
    file_id_from_identity, EntityRef, IngestError, IngestPipeline, IngestReceipt, IngestRequest,
    DEFAULT_MAX_FILE_BYTES,
};
use dailyos_lib::services::workspace_ingestion::registry::WorkspaceSourceRegistry;
use dailyos_lib::services::workspace_ingestion::runs::IngestionMode;
use dailyos_lib::services::workspace_ingestion::wiring;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

#[test]
fn verified_entity_note_commits_user_only_user_note_claim() {
    let fixture = Fixture::new();
    fixture.seed_account("acme", "Acme");
    let note = fixture.write_account_file("Acme", "notes.md", "Important local context.");

    let receipt = ingest_file(
        ActionDb::from_conn(&fixture.conn),
        &fixture.workspace_root,
        &note,
        Some(entity_ref(EntityType::Account, "acme", Some("Acme"))),
    )
    .expect("ingest succeeds");

    assert_eq!(receipt.claim_proposals.len(), 1);
    let row = fixture
        .conn
        .query_row(
            "SELECT subject_ref, claim_type, text, data_source, source_ref, \
                    source_asof, provenance_json, metadata_json, sensitivity \
             FROM intelligence_claims",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )
        .expect("claim row");

    let subject_ref: serde_json::Value = serde_json::from_str(&row.0).expect("subject_ref json");
    assert_eq!(subject_ref["kind"], "account");
    assert_eq!(subject_ref["id"], "acme");
    assert_eq!(row.1, "user_note");
    assert_eq!(row.2, "Important local context.");
    assert_eq!(row.3, "workspace_file:entity_doc");
    assert!(row.4.starts_with("workspace_file:"));
    assert!(!row.4.contains('/'));
    assert!(!row.5.is_empty());
    assert!(row.6.contains("\"workspace_file\""));
    assert!(row.7.contains("\"producer\":\"workspace_ingestion\""));
    assert!(row.7.contains("\"sensitivity_floor\":\"user_only\""));
    assert_eq!(row.8, "user_only");

    let (claim_count, extractor_version, error_log): (i64, String, Option<String>) = fixture
        .conn
        .query_row(
            "SELECT claim_count_produced, extractor_version, error_log \
             FROM document_ingestion_runs WHERE run_id = ?1",
            [&receipt.ingestion_run_id.0],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("run error log");
    assert_eq!(claim_count, 1);
    assert_eq!(extractor_version, "workspace-extractor-v1");
    assert!(error_log.is_none());
}

#[test]
fn prevalidated_content_snapshot_drives_claim_after_file_mutates() {
    let fixture = Fixture::new();
    fixture.seed_account("acme", "Acme");
    let note = fixture.write_account_file("Acme", "notes.md", "Original local context.");
    let (file, identity) = WorkspaceSourceRegistry::open_validated(&fixture.workspace_root, &note)
        .map_err(IngestError::Rejected)
        .expect("open validated");
    let source_asof: DateTime<Utc> = identity
        .canonical_path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .map_err(IngestError::Io)
        .expect("source asof")
        .into();
    let file_id = file_id_from_identity(&identity, &fixture.workspace_root).expect("file id");

    std::fs::write(&note, "Mutated local context.").expect("mutate file");

    let request = IngestRequest {
        file,
        identity,
        file_id,
        source_asof,
        source_type: WorkspaceFileKind::EntityDoc,
        entity: Some(entity_ref(EntityType::Account, "acme", Some("Acme"))),
        mode: IngestionMode::Realtime,
        category_hint: None,
        invocation_actor: "user".to_string(),
        validated_content: Some("Original local context.".to_string()),
    };
    let pipeline = wiring::build_pipeline(fixture.workspace_root.clone());
    let receipt = run_pipeline(&ActionDb::from_conn(&fixture.conn), &pipeline, request)
        .expect("ingest succeeds");

    let text: String = fixture
        .conn
        .query_row("SELECT text FROM intelligence_claims", [], |row| row.get(0))
        .expect("claim text");
    assert_eq!(text, "Original local context.");

    let expected_hash = hex::encode(Sha256::digest(b"Original local context."));
    assert_eq!(receipt.content_sha256, expected_hash);
}

#[test]
fn mismatched_entity_hint_does_not_commit_claim() {
    let fixture = Fixture::new();
    fixture.seed_account("acme", "Acme");
    let note = fixture.write_account_file("Acme", "notes.md", "Important local context.");

    let receipt = ingest_file(
        ActionDb::from_conn(&fixture.conn),
        &fixture.workspace_root,
        &note,
        Some(entity_ref(EntityType::Account, "acme", Some("wrong-name"))),
    )
    .expect("ingest succeeds without claim");

    assert!(receipt.claim_proposals.is_empty());
    assert_claim_count(&fixture.conn, 0);
    let error_log = run_error_log(&fixture.conn, &receipt.ingestion_run_id.0);
    assert!(error_log.to_string().contains("entity_name_mismatch"));
    assert!(error_log.to_string().contains("unlinked_subject"));
}

#[test]
fn rejected_existing_link_fails_closed_without_resurrection() {
    let fixture = Fixture::new();
    fixture.seed_account("acme", "Acme");
    let note = fixture.write_account_file("Acme", "notes.md", "Important local context.");
    let file_id = seed_rejected_link(
        &fixture.conn,
        &fixture.workspace_root,
        &note,
        EntityType::Account,
        "acme",
    );

    let receipt = ingest_file(
        ActionDb::from_conn(&fixture.conn),
        &fixture.workspace_root,
        &note,
        Some(entity_ref(EntityType::Account, "acme", Some("Acme"))),
    )
    .expect("ingest succeeds without claim");

    assert!(receipt.claim_proposals.is_empty());
    assert_claim_count(&fixture.conn, 0);
    let active_links: i64 = fixture
        .conn
        .query_row(
            "SELECT count(*) FROM document_entity_links WHERE file_id = ?1 AND rejected = 0",
            [file_id],
            |row| row.get(0),
        )
        .expect("active link count");
    assert_eq!(active_links, 0);
    let error_log = run_error_log(&fixture.conn, &receipt.ingestion_run_id.0);
    assert!(error_log.to_string().contains("rejected_link"));
}

#[test]
fn ingested_signal_failure_rolls_back_claim_commit() {
    let fixture = Fixture::new();
    fixture.seed_account("acme", "Acme");
    let note = fixture.write_account_file("Acme", "notes.md", "Important local context.");
    let (file, identity) = WorkspaceSourceRegistry::open_validated(&fixture.workspace_root, &note)
        .map_err(IngestError::Rejected)
        .expect("open validated");
    let source_asof: DateTime<Utc> = identity
        .canonical_path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .map_err(IngestError::Io)
        .expect("source asof")
        .into();
    let file_id = file_id_from_identity(&identity, &fixture.workspace_root).expect("file id");
    let request = IngestRequest {
        file,
        identity,
        file_id: file_id.clone(),
        source_asof,
        source_type: WorkspaceFileKind::EntityDoc,
        entity: Some(entity_ref(EntityType::Account, "acme", Some("Acme"))),
        mode: IngestionMode::Realtime,
        category_hint: None,
        invocation_actor: "user".to_string(),
        validated_content: None,
    };
    let pipeline = IngestPipeline::new(
        Box::new(WorkspaceExtractor),
        Box::new(FailingIngestedSignalEmitter),
        DEFAULT_MAX_FILE_BYTES,
        "workspace-extractor-v1",
        fixture.workspace_root.clone(),
    );

    let error = run_pipeline(&ActionDb::from_conn(&fixture.conn), &pipeline, request)
        .expect_err("required ingested signal failure fails ingestion");

    match error {
        IngestError::DbError(message) => {
            assert!(message.contains("workspace_file_ingested"));
        }
        other => panic!("expected DB finalization error, got {other:?}"),
    }
    assert_claim_count(&fixture.conn, 0);

    let (status, claim_count, error_log): (String, i64, String) = fixture
        .conn
        .query_row(
            "SELECT status, claim_count_produced, error_log \
             FROM document_ingestion_runs WHERE file_id = ?1",
            [&file_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("failed run");
    assert_eq!(status, "failed");
    assert_eq!(claim_count, 0);
    let error_log: serde_json::Value = serde_json::from_str(&error_log).expect("error log json");
    assert_eq!(
        error_log["error"],
        "workspace_ingestion_finalization_failed"
    );
    assert!(error_log["message"]
        .as_str()
        .expect("error message")
        .contains("workspace_file_ingested"));

    let lifecycle = LifecycleRepo::get(&fixture.conn, &file_id)
        .expect("lifecycle lookup")
        .expect("lifecycle row");
    assert_eq!(lifecycle.lifecycle_state, LifecycleState::Rejected);
}

struct FailingIngestedSignalEmitter;

impl SignalEmitter for FailingIngestedSignalEmitter {
    fn emit_file_ingested(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: &str,
        _ingestion_run_id: &str,
        _entity_type: &str,
        _entity_id: &str,
    ) -> Result<(), SignalEmitError> {
        Err(SignalEmitError::Emit {
            signal_type: "workspace_file_ingested",
            message: "test failure".to_string(),
        })
    }

    fn emit_file_rejected(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: Option<&str>,
        _reason: RejectionReason,
    ) -> Result<(), SignalEmitError> {
        Ok(())
    }

    fn emit_file_pending_entity_assignment(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: &str,
        _ingestion_run_id: &str,
    ) -> Result<(), SignalEmitError> {
        Ok(())
    }

    fn emit_file_quarantined(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: &str,
        _reason: &str,
        _actor: &str,
        _entity_type: Option<&str>,
        _entity_id: Option<&str>,
    ) -> Result<(), SignalEmitError> {
        Ok(())
    }

    fn emit_link_changed(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: &str,
        _entity_type: &str,
        _entity_id: &str,
        _actor: &str,
    ) -> Result<(), SignalEmitError> {
        Ok(())
    }
}

struct Fixture {
    conn: Connection,
    _workspace: tempfile::TempDir,
    workspace_root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let conn = Connection::open_in_memory().expect("sqlite");
        dailyos_lib::migration_test_api::run_migrations(&conn).expect("migrations");
        let workspace = tempfile::tempdir().expect("workspace");
        let workspace_root = workspace
            .path()
            .canonicalize()
            .expect("canonical workspace");
        Self {
            conn,
            _workspace: workspace,
            workspace_root,
        }
    }

    fn seed_account(&self, id: &str, name: &str) {
        let account = DbAccount {
            id: id.to_string(),
            name: name.to_string(),
            tracker_path: Some(format!("Accounts/{name}")),
            updated_at: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        ActionDb::from_conn(&self.conn)
            .upsert_account(&account)
            .expect("account upsert");
    }

    fn write_account_file(&self, account_dir: &str, filename: &str, content: &str) -> PathBuf {
        let dir = self.workspace_root.join("Accounts").join(account_dir);
        std::fs::create_dir_all(&dir).expect("account dir");
        let path = dir.join(filename);
        std::fs::write(&path, content).expect("write note");
        path
    }
}

fn ingest_file(
    db: &ActionDb,
    workspace_root: &Path,
    path: &Path,
    entity: Option<EntityRef>,
) -> Result<IngestReceipt, IngestError> {
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
        source_type: WorkspaceFileKind::EntityDoc,
        entity,
        mode: IngestionMode::Realtime,
        category_hint: None,
        invocation_actor: "user".to_string(),
        validated_content: None,
    };
    let pipeline = wiring::build_pipeline(workspace_root.to_path_buf());
    run_pipeline(db, &pipeline, request)
}

fn run_pipeline(
    db: &ActionDb,
    pipeline: &IngestPipeline,
    request: IngestRequest,
) -> Result<IngestReceipt, IngestError> {
    let clock = SystemClock;
    let rng = SystemRng;
    let external = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("system:test");
    let signal_engine = dailyos_lib::signals::propagation::default_engine();
    pipeline.run_with_signal_engine(&ctx, db, &signal_engine, request)
}

fn seed_rejected_link(
    conn: &Connection,
    workspace_root: &Path,
    path: &Path,
    entity_type: EntityType,
    entity_id: &str,
) -> String {
    let (file, identity) = WorkspaceSourceRegistry::open_validated(workspace_root, path)
        .map_err(IngestError::Rejected)
        .expect("open validated");
    let source_asof: DateTime<Utc> = file
        .metadata()
        .and_then(|metadata| metadata.modified())
        .expect("source asof")
        .into();
    let file_id = file_id_from_identity(&identity, workspace_root).expect("file id");
    LifecycleRepo::insert_pending(
        conn,
        &file_id,
        &identity,
        &WorkspaceFileKind::EntityDoc,
        source_asof,
        Some(&entity_ref(entity_type, entity_id, Some("Acme"))),
    )
    .expect("seed lifecycle");
    LinkRepo::add_link(
        conn,
        &file_id,
        entity_type,
        entity_id,
        LinkAttributionSource::EntityIntake,
        1.0,
        Some("test link"),
        "system:test",
    )
    .expect("add link");
    LinkRepo::reject_link(
        conn,
        &file_id,
        entity_type,
        entity_id,
        "user",
        "wrong entity",
    )
    .expect("reject link");
    file_id
}

fn entity_ref(entity_type: EntityType, id: &str, name: Option<&str>) -> EntityRef {
    EntityRef {
        entity_type,
        entity_id: EntityId::new(id.to_string()),
        entity_name: name.map(str::to_string),
    }
}

fn assert_claim_count(conn: &Connection, expected: i64) {
    let count: i64 = conn
        .query_row("SELECT count(*) FROM intelligence_claims", [], |row| {
            row.get(0)
        })
        .expect("claim count");
    assert_eq!(count, expected);
}

fn run_error_log(conn: &Connection, run_id: &str) -> serde_json::Value {
    let raw: String = conn
        .query_row(
            "SELECT error_log FROM document_ingestion_runs WHERE run_id = ?1",
            [run_id],
            |row| row.get(0),
        )
        .expect("run error log");
    serde_json::from_str(&raw).expect("error_log json")
}

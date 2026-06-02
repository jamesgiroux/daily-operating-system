use std::ffi::CString;
use std::fs::{self, File};
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use abilities_runtime::abilities::provenance::source::EntityId;
use abilities_runtime::abilities::registry::Actor;
use abilities_runtime::services::workspace_intake::{
    EntityRefDto, PlacementError, PlacementErrorCode, PlacementInvocationContext,
    WorkspaceIntakeError, WorkspaceIntakeReceipt, WorkspaceIntakeRequest, WorkspaceIntakeService,
    WorkspacePlaceDocumentReceipt, WorkspacePlaceDocumentRequest, WorkspacePlacementMutationCursor,
    WORKSPACE_PLACE_DOCUMENT_DECODED_MAX_BYTES, WORKSPACE_PLACE_DOCUMENT_SCHEMA_VERSION,
};
use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::db::{ActionDb, LocalKeychain};
use crate::entity::EntityType;
use crate::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};

use super::contracts::{RejectionReason, SignalEmitContext, WorkspaceCategory, WorkspaceFileKind};
use super::lifecycle::{lifecycle_state_slug, workspace_file_kind_from_slug};
use super::pipeline::{file_id_from_identity, EntityRef, FileIdError, IngestError, IngestRequest};
use super::registry::{ResolvePathError, WorkspaceCategoryRegistry};
use super::signals::emit_pre_pipeline_rejection;
use super::wiring::build_pipeline;
use super::{registry::WorkspaceSourceRegistry, runs::IngestionMode};

const PLACEMENT_RATE_LIMIT_MAX: i64 = 200;
const PLACEMENT_RATE_LIMIT_WINDOW_SECONDS: i64 = 60 * 60;

pub struct IngestPipelineWorkspaceIntake {
    workspace_root: PathBuf,
    signal_engine: Option<Arc<crate::signals::propagation::PropagationEngine>>,
}

impl IngestPipelineWorkspaceIntake {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            signal_engine: None,
        }
    }

    pub fn with_signal_engine(
        workspace_root: PathBuf,
        signal_engine: Arc<crate::signals::propagation::PropagationEngine>,
    ) -> Self {
        Self {
            workspace_root,
            signal_engine: Some(signal_engine),
        }
    }

    pub fn from_config_or_empty() -> Self {
        Self::from_config_or_empty_with_signal_engine(None)
    }

    pub fn from_config_or_empty_with_signal_engine(
        signal_engine: Option<Arc<crate::signals::propagation::PropagationEngine>>,
    ) -> Self {
        let workspace_root = crate::state::load_config()
            .map(|config| PathBuf::from(config.workspace_path))
            .unwrap_or_default();
        Self {
            workspace_root,
            signal_engine,
        }
    }
}

#[async_trait]
impl WorkspaceIntakeService for IngestPipelineWorkspaceIntake {
    async fn ingest(
        &self,
        ctx: &abilities_runtime::abilities::registry::AbilityContext<'_>,
        req: WorkspaceIntakeRequest,
    ) -> Result<WorkspaceIntakeReceipt, WorkspaceIntakeError> {
        ctx.services()
            .check_mutation_allowed()
            .map_err(|e| WorkspaceIntakeError::DbError(e.to_string()))?;
        let workspace_root = self.workspace_root.clone();
        let signal_engine = self.signal_engine.clone();
        let original_actor = ability_actor_label(&ctx.actor).to_string();
        let ability_id = ctx.services().ability_id.map(str::to_string);
        tokio::task::spawn_blocking(move || {
            let clock = SystemClock;
            let rng = SystemRng;
            let external = ExternalClients::default();
            let mut service_ctx = ServiceContext::new_live(&clock, &rng, &external)
                .with_actor("system:workspace_ingestion");
            if let Some(ability_id) = ability_id.as_deref() {
                service_ctx = service_ctx.with_ability_id(ability_id);
            }
            ingest_sync(
                &service_ctx,
                workspace_root,
                signal_engine,
                req,
                original_actor,
            )
        })
        .await
        .map_err(|e| WorkspaceIntakeError::Io(format!("workspace intake task failed: {e}")))?
    }

    async fn place_document(
        &self,
        ctx: &abilities_runtime::abilities::registry::AbilityContext<'_>,
        invocation: PlacementInvocationContext,
        req: WorkspacePlaceDocumentRequest,
    ) -> Result<WorkspacePlaceDocumentReceipt, PlacementError> {
        ctx.services()
            .check_mutation_allowed()
            .map_err(|e| PlacementError::internal(e.to_string()))?;
        let workspace_root = self.workspace_root.clone();
        let signal_engine = self.signal_engine.clone();
        let ability_id = ctx.services().ability_id.map(str::to_string);
        tokio::task::spawn_blocking(move || {
            let clock = SystemClock;
            let rng = SystemRng;
            let external = ExternalClients::default();
            let mut service_ctx = ServiceContext::new_live(&clock, &rng, &external)
                .with_actor("system:workspace_placement");
            if let Some(ability_id) = ability_id.as_deref() {
                service_ctx = service_ctx.with_ability_id(ability_id);
            }
            place_document_sync(&service_ctx, workspace_root, signal_engine, invocation, req)
        })
        .await
        .map_err(|e| PlacementError::internal(format!("workspace placement task failed: {e}")))?
    }
}

pub(crate) fn ingest_sync(
    ctx: &ServiceContext<'_>,
    workspace_root: PathBuf,
    signal_engine: Option<Arc<crate::signals::propagation::PropagationEngine>>,
    req: WorkspaceIntakeRequest,
    original_actor: String,
) -> Result<WorkspaceIntakeReceipt, WorkspaceIntakeError> {
    let source_type = parse_workspace_file_kind(&req.source_type_slug)
        .ok_or_else(|| WorkspaceIntakeError::InvalidSourceTypeSlug(req.source_type_slug.clone()))?;
    let mode = parse_ingestion_mode(&req.mode_slug)
        .ok_or_else(|| WorkspaceIntakeError::InvalidModeSlug(req.mode_slug.clone()))?;
    let entity = req.entity.map(parse_entity).transpose()?;
    let category_hint = req
        .category_slug
        .as_deref()
        .map(parse_category)
        .transpose()?;

    // mcp-self-open-allowed: handler-reachable intake adapter fallback; indirect workspace write routing is outside this direct MCP handler pass.
    let db = ActionDb::open(Arc::new(LocalKeychain::new())) // mcp-self-open-allowed: handler-reachable intake adapter fallback
        .map_err(|e| WorkspaceIntakeError::DbError(e.to_string()))?;
    let conn = db.conn_ref();

    if let Some(category) = category_hint.as_ref() {
        let Some(entity_type) = entity.as_ref().map(|e| e.entity_type) else {
            return Err(WorkspaceIntakeError::CategoryNotAllowed { allowed: vec![] });
        };
        WorkspaceCategoryRegistry::validate(conn, category, entity_type)
            .map_err(|e| WorkspaceIntakeError::CategoryNotAllowed { allowed: e.allowed })?;
    }

    let (file, identity) =
        match WorkspaceSourceRegistry::open_validated(&workspace_root, Path::new(&req.file_ref)) {
            Ok(opened) => opened,
            Err(reason) => {
                let signal_ctx = SignalEmitContext::new(ctx, &db, None);
                emit_pre_pipeline_rejection(&signal_ctx, reason.clone())
                    .map_err(|e| WorkspaceIntakeError::DbError(e.to_string()))?;
                return Err(intake_error_from_rejection(reason));
            }
        };
    let source_asof = file
        .metadata()
        .and_then(|m| m.modified())
        .map(chrono::DateTime::<chrono::Utc>::from)
        .map_err(|e| WorkspaceIntakeError::Io(io_error_redacted(e)))?;
    let file_id = match file_id_from_identity(&identity, &workspace_root) {
        Ok(file_id) => file_id,
        Err(error) => {
            let signal_ctx = SignalEmitContext::new(ctx, &db, None);
            emit_pre_pipeline_rejection(&signal_ctx, RejectionReason::OutsideWorkspace)
                .map_err(|e| WorkspaceIntakeError::DbError(e.to_string()))?;
            return Err(intake_file_id_error(error));
        }
    };

    let request = IngestRequest {
        file,
        identity,
        file_id,
        source_asof,
        source_type,
        entity,
        mode,
        category_hint,
        invocation_actor: original_actor,
        validated_content: None,
    };
    let pipeline = build_pipeline(workspace_root);
    let propagation = signal_engine.as_deref().ok_or_else(|| {
        WorkspaceIntakeError::DbError(
            "workspace intake requires a live signal propagation engine".to_string(),
        )
    })?;
    let receipt = pipeline
        .run_with_signal_engine(ctx, &db, propagation, request)
        .map_err(intake_error_from_ingest)?;
    Ok(WorkspaceIntakeReceipt {
        run_id: receipt.ingestion_run_id.0,
        file_id: receipt.file_id,
        content_sha256: receipt.content_sha256,
        lifecycle_state_after_slug: lifecycle_state_slug(receipt.lifecycle_state_after).to_string(),
        resolved_path: receipt.resolved_path,
    })
}

fn ability_actor_label(actor: &Actor) -> &'static str {
    match actor {
        Actor::Agent => "agent",
        Actor::User => "user",
        Actor::Admin => "admin",
        Actor::System => "system",
        Actor::SurfaceClient { .. } => "surface_client",
        Actor::McpClient { .. } => "mcp_client",
    }
}

fn parse_workspace_file_kind(slug: &str) -> Option<WorkspaceFileKind> {
    workspace_file_kind_from_slug(slug)
}

fn parse_ingestion_mode(slug: &str) -> Option<IngestionMode> {
    match slug {
        "initial" => Some(IngestionMode::Initial),
        "incremental" => Some(IngestionMode::Incremental),
        "forced" => Some(IngestionMode::Forced),
        "backfill" => Some(IngestionMode::Backfill),
        "entity_seeded" => Some(IngestionMode::EntitySeeded),
        "realtime" => Some(IngestionMode::Realtime),
        _ => None,
    }
}

fn parse_category(slug: &str) -> Result<WorkspaceCategory, WorkspaceIntakeError> {
    WorkspaceCategory::from_slug(slug)
        .ok_or_else(|| WorkspaceIntakeError::InvalidCategorySlug(slug.to_string()))
}

fn parse_entity(dto: EntityRefDto) -> Result<EntityRef, WorkspaceIntakeError> {
    if !matches!(
        dto.entity_type_slug.as_str(),
        "account" | "person" | "project" | "other"
    ) {
        return Err(WorkspaceIntakeError::InvalidEntityTypeSlug(
            dto.entity_type_slug,
        ));
    }
    if dto.entity_id.trim().is_empty() {
        return Err(WorkspaceIntakeError::InvalidEntityId);
    }
    let entity_name = dto
        .entity_name
        .ok_or_else(|| WorkspaceIntakeError::InvalidEntityName(String::new()))?;
    if !is_valid_path_segment_slug(&entity_name) {
        return Err(WorkspaceIntakeError::InvalidEntityName(entity_name));
    }
    Ok(EntityRef {
        entity_type: EntityType::from_str_lossy(&dto.entity_type_slug),
        entity_id: EntityId::new(dto.entity_id),
        entity_name: Some(entity_name),
    })
}

fn is_valid_path_segment_slug(slug: &str) -> bool {
    !slug.is_empty() && slug.len() <= 32 && {
        let mut chars = slug.chars();
        let first = chars.next().expect("non-empty checked above");
        first.is_ascii_lowercase()
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    }
}

fn intake_error_from_rejection(reason: RejectionReason) -> WorkspaceIntakeError {
    match reason {
        RejectionReason::PathTraversalAttempt => WorkspaceIntakeError::PathTraversalAttempt,
        RejectionReason::OutsideWorkspace => WorkspaceIntakeError::OutsideWorkspace,
        RejectionReason::SymlinkRaced | RejectionReason::SymlinkRefused => {
            WorkspaceIntakeError::SymlinkRaced
        }
        RejectionReason::FileTooLarge => WorkspaceIntakeError::FileTooLarge,
        RejectionReason::UnsupportedFormat => WorkspaceIntakeError::UnsupportedFormat,
    }
}

fn intake_file_id_error(error: FileIdError) -> WorkspaceIntakeError {
    match error {
        FileIdError::OutsideWorkspace => WorkspaceIntakeError::OutsideWorkspace,
    }
}

fn intake_error_from_ingest(error: IngestError) -> WorkspaceIntakeError {
    match error {
        IngestError::Rejected(reason) => intake_error_from_rejection(reason),
        IngestError::AlreadyProcessed { existing_run_id } => {
            WorkspaceIntakeError::AlreadyProcessed {
                existing_run_id: existing_run_id.0,
            }
        }
        IngestError::Io(error) => WorkspaceIntakeError::Io(io_error_redacted(error)),
        IngestError::DbError(message) => WorkspaceIntakeError::DbError(message),
        IngestError::FileIdMismatch { .. } => {
            WorkspaceIntakeError::DbError("file_id mismatch after derivation".to_string())
        }
    }
}

fn io_error_redacted(error: std::io::Error) -> String {
    format!("kind={:?} os={:?}", error.kind(), error.raw_os_error())
}

fn place_document_sync(
    ctx: &ServiceContext<'_>,
    workspace_root: PathBuf,
    signal_engine: Option<Arc<crate::signals::propagation::PropagationEngine>>,
    invocation: PlacementInvocationContext,
    req: WorkspacePlaceDocumentRequest,
) -> Result<WorkspacePlaceDocumentReceipt, PlacementError> {
    if workspace_root.as_os_str().is_empty() {
        return Err(PlacementError::internal("workspace root is not configured"));
    }
    if req.schema_version != WORKSPACE_PLACE_DOCUMENT_SCHEMA_VERSION {
        return Err(PlacementError::new(
            PlacementErrorCode::UnsupportedSchemaVersion,
            "schema_version must be 1",
        ));
    }

    // mcp-self-open-allowed: handler-reachable placement adapter fallback; indirect workspace write routing is outside this direct MCP handler pass.
    let db = ActionDb::open(Arc::new(LocalKeychain::new())) // mcp-self-open-allowed: handler-reachable placement adapter fallback
        .map_err(|e| PlacementError::internal(e.to_string()))?;
    let conn = db.conn_ref();
    let target_key = crate::db::local_db_keyed_audit_tag(
        "target",
        "workspace-placement-target-v1",
        &[&req.entity.entity_type, &req.entity.entity_id],
    )
    .map_err(PlacementError::internal)?;
    let category_audit_slug = category_slug_for_audit(&req);
    if let Err(error) = reserve_placement_rate(conn, &invocation, ctx.clock.now()) {
        write_attempt_audit(
            conn,
            &invocation,
            Some(target_key.as_str()),
            category_audit_slug.as_deref(),
            req.dry_run,
            "failed",
            Some(error.code.as_str()),
        )?;
        return Err(error);
    }
    let outcome = place_document_after_rate(
        ctx,
        &db,
        &workspace_root,
        signal_engine.as_deref(),
        &invocation,
        &req,
        target_key.as_str(),
    );
    match outcome {
        Ok(receipt) => {
            write_attempt_audit(
                conn,
                &invocation,
                Some(target_key.as_str()),
                category_audit_slug.as_deref(),
                req.dry_run,
                "succeeded",
                None,
            )?;
            Ok(receipt)
        }
        Err(error) => {
            write_attempt_audit(
                conn,
                &invocation,
                Some(target_key.as_str()),
                category_audit_slug.as_deref(),
                req.dry_run,
                "failed",
                Some(error.code.as_str()),
            )?;
            Err(error)
        }
    }
}

fn place_document_after_rate(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    workspace_root: &Path,
    signal_engine: Option<&crate::signals::propagation::PropagationEngine>,
    invocation: &PlacementInvocationContext,
    req: &WorkspacePlaceDocumentRequest,
    _target_audit_key: &str,
) -> Result<WorkspacePlaceDocumentReceipt, PlacementError> {
    let entity_type = parse_placeable_entity_type(&req.entity.entity_type)?;
    validate_entity_id(&req.entity.entity_id)?;
    let normalized_client_key = normalize_client_dedup_key(req.client_dedup_key.as_deref())?;
    let filename_hint = normalize_filename_hint(req.filename_hint.as_deref())?;
    let content_type = normalize_content_type(&req.content_type)?;
    let content = decode_placement_content(&req.content_b64)?;
    let content_text = placement_content_text(content_type, content.clone())?;
    let content_sha256 = sha256_hex(&content);
    let conn = db.conn_ref();
    let canonical_name = authorize_target(conn, entity_type, &req.entity.entity_id)?;
    let entity_slug = entity_path_slug(&canonical_name)?;
    let category = parse_and_validate_category(conn, entity_type, &req.category)?;

    let preview_filename = filename_hint
        .clone()
        .unwrap_or_else(|| preview_filename_for_content(content_type, &content_sha256));
    let preview_relative_path = resolve_placement_path(
        conn,
        entity_type,
        &entity_slug,
        &category,
        &preview_filename,
    )?;
    preview_handle_relative_path(workspace_root, &preview_relative_path)?;

    if req.dry_run {
        return Ok(dry_run_receipt(req, &category));
    }

    let fence = reserve_or_load_idempotency(
        conn,
        invocation,
        req,
        PlacementIdempotencyInput {
            content_type,
            category_slug: category.as_slug(),
            content_sha256: &content_sha256,
            client_dedup_key: &normalized_client_key,
            filename_hint: filename_hint.as_deref(),
        },
    )?;

    match fence {
        PlacementFence::Replay(row) => {
            let receipt_path = row
                .chosen_filename
                .as_deref()
                .map(|filename| {
                    resolve_placement_path(conn, entity_type, &entity_slug, &category, filename)
                })
                .transpose()?;
            render_live_receipt(
                invocation,
                req,
                &category,
                &row,
                receipt_path.as_ref(),
                true,
            )
        }
        PlacementFence::StaleFailed(row) => {
            if let Some(relative_path) =
                stale_generated_placement_path(conn, entity_type, &entity_slug, &category, &row)
            {
                best_effort_remove_placement_file(
                    workspace_root,
                    &relative_path,
                    "stale placement cleanup",
                );
            }
            Err(PlacementError::new(
                PlacementErrorCode::PreviousAttemptFailed,
                "previous placement attempt could not be completed",
            ))
        }
        PlacementFence::Reserved(row) => {
            let chosen_filename = row
                .chosen_filename
                .clone()
                .ok_or_else(|| PlacementError::internal("missing reserved filename"))?;
            let relative_path = resolve_placement_path(
                conn,
                entity_type,
                &entity_slug,
                &category,
                &chosen_filename,
            )?;
            if let Err(error) = write_file_handle_relative(workspace_root, &relative_path, &content)
            {
                if error.created_file {
                    best_effort_remove_placement_file(
                        workspace_root,
                        &relative_path,
                        "failed write cleanup",
                    );
                }
                log::warn!(
                    "workspace placement file write rejected: {}",
                    io_error_redacted(error.error)
                );
                mark_placement_failed(
                    conn,
                    &row.idempotency_id,
                    PlacementErrorCode::PlacementPathRejected,
                )?;
                return Err(PlacementError::new(
                    PlacementErrorCode::PlacementPathRejected,
                    "placement path rejected",
                ));
            }

            let (file, identity) =
                WorkspaceSourceRegistry::open_validated(workspace_root, &relative_path).map_err(
                    |reason| {
                        best_effort_remove_placement_file(
                            workspace_root,
                            &relative_path,
                            "post-write validation cleanup",
                        );
                        log::warn!("workspace placement post-write validation failed: {reason:?}");
                        if let Err(mark_error) = mark_placement_failed(
                            conn,
                            &row.idempotency_id,
                            PlacementErrorCode::PlacementPathRejected,
                        ) {
                            log::warn!(
                        "workspace placement failed-row marker after validation failure failed: {}",
                        mark_error.code
                    );
                        }
                        PlacementError::new(
                            PlacementErrorCode::PlacementPathRejected,
                            "placement path rejected",
                        )
                    },
                )?;
            let source_asof = file
                .metadata()
                .and_then(|m| m.modified())
                .map(chrono::DateTime::<chrono::Utc>::from)
                .map_err(|e| PlacementError::internal(io_error_redacted(e)))?;
            let file_id = file_id_from_identity(&identity, workspace_root)
                .map_err(|_| PlacementError::internal("file id derivation failed"))?;
            let source_handle = format!("source_{}", uuid::Uuid::new_v4());
            if let Err(error) = record_placement_commit_started(
                conn,
                PlacementCommitStarted {
                    idempotency_id: &row.idempotency_id,
                    source_handle: &source_handle,
                    file_id: &file_id,
                    source_asof,
                },
            ) {
                best_effort_remove_placement_file(
                    workspace_root,
                    &relative_path,
                    "commit-start failure cleanup",
                );
                if let Err(mark_error) = mark_placement_failed(
                    conn,
                    &row.idempotency_id,
                    PlacementErrorCode::PlacementInternal,
                ) {
                    log::warn!(
                        "workspace placement failed-row marker after commit-start failure failed: {}",
                        mark_error.code
                    );
                }
                return Err(error);
            }
            let request = IngestRequest {
                file,
                identity,
                file_id,
                source_asof,
                source_type: WorkspaceFileKind::McpPlacement,
                entity: Some(EntityRef {
                    entity_type,
                    entity_id: EntityId::new(req.entity.entity_id.clone()),
                    entity_name: Some(canonical_name),
                }),
                mode: IngestionMode::EntitySeeded,
                category_hint: Some(category.clone()),
                invocation_actor: "mcp_client".to_string(),
                validated_content: Some(content_text),
            };
            let pipeline = build_pipeline(workspace_root.to_path_buf());
            let Some(signal_engine) = signal_engine else {
                mark_placement_failed(
                    conn,
                    &row.idempotency_id,
                    PlacementErrorCode::PlacementInternal,
                )?;
                return Err(PlacementError::internal(
                    "workspace placement requires a live signal propagation engine",
                ));
            };
            let receipt = match pipeline.run_with_signal_engine(ctx, db, signal_engine, request) {
                Ok(receipt) => receipt,
                Err(error) => {
                    log::warn!("workspace placement ingestion failed: {error}");
                    mark_placement_failed(
                        conn,
                        &row.idempotency_id,
                        PlacementErrorCode::IngestionFailed,
                    )?;
                    return Err(PlacementError::new(
                        PlacementErrorCode::IngestionFailed,
                        "workspace placement ingestion failed",
                    ));
                }
            };
            let claim_count = receipt.claim_proposals.len() as u64;
            update_placement_success(
                conn,
                PlacementSuccessUpdate {
                    idempotency_id: &row.idempotency_id,
                    source_handle: &source_handle,
                    file_id: &receipt.file_id,
                    run_id: &receipt.ingestion_run_id.0,
                    source_asof,
                    lifecycle_state: lifecycle_state_slug(receipt.lifecycle_state_after),
                    claim_count,
                },
            )?;
            let completed = PlacementRow {
                source_handle: Some(source_handle),
                file_id: Some(receipt.file_id),
                source_asof: Some(source_asof.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
                lifecycle_state: Some(
                    lifecycle_state_slug(receipt.lifecycle_state_after).to_string(),
                ),
                claim_count_produced: claim_count,
                status: "succeeded".to_string(),
                ..row
            };
            render_live_receipt(
                invocation,
                req,
                &category,
                &completed,
                Some(&relative_path),
                false,
            )
        }
    }
}

#[cfg(test)]
pub(crate) fn place_document_after_rate_for_tests(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    workspace_root: &Path,
    signal_engine: Option<&crate::signals::propagation::PropagationEngine>,
    invocation: &PlacementInvocationContext,
    req: &WorkspacePlaceDocumentRequest,
    target_audit_key: &str,
) -> Result<WorkspacePlaceDocumentReceipt, PlacementError> {
    place_document_after_rate(
        ctx,
        db,
        workspace_root,
        signal_engine,
        invocation,
        req,
        target_audit_key,
    )
}

fn reserve_placement_rate(
    conn: &Connection,
    invocation: &PlacementInvocationContext,
    now: DateTime<Utc>,
) -> Result<(), PlacementError> {
    let called_at = now.timestamp();
    let cutoff = called_at - PLACEMENT_RATE_LIMIT_WINDOW_SECONDS;
    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| PlacementError::internal(e.to_string()))?;
    let result = (|| {
        conn.execute(
            "DELETE FROM workspace_placement_rate_ledger \
             WHERE actor_id = ?1 AND tool_name = ?2 AND called_at < ?3",
            params![invocation.actor_id, invocation.tool_name, cutoff],
        )
        .map_err(|e| PlacementError::internal(e.to_string()))?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workspace_placement_rate_ledger \
                 WHERE actor_id = ?1 AND tool_name = ?2 AND called_at >= ?3",
                params![invocation.actor_id, invocation.tool_name, cutoff],
                |row| row.get(0),
            )
            .map_err(|e| PlacementError::internal(e.to_string()))?;
        if count >= PLACEMENT_RATE_LIMIT_MAX {
            return Err(PlacementError::new(
                PlacementErrorCode::RateLimited,
                "workspace placement rate limit exceeded",
            )
            .with_retry_after(PLACEMENT_RATE_LIMIT_WINDOW_SECONDS as u64));
        }
        conn.execute(
            "INSERT INTO workspace_placement_rate_ledger (actor_id, tool_name, called_at) \
             VALUES (?1, ?2, ?3)",
            params![invocation.actor_id, invocation.tool_name, called_at],
        )
        .map_err(|e| PlacementError::internal(e.to_string()))?;
        Ok(())
    })();
    finish_tx(conn, result)
}

fn write_attempt_audit(
    conn: &Connection,
    invocation: &PlacementInvocationContext,
    target_audit_key: Option<&str>,
    category_slug: Option<&str>,
    dry_run: bool,
    outcome: &str,
    error_code: Option<&str>,
) -> Result<(), PlacementError> {
    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| PlacementError::internal(e.to_string()))?;
    let result = conn
        .execute(
            "INSERT INTO workspace_placement_attempt_audit \
             (actor_id, tool_name, target_audit_key, category_slug, dry_run, outcome, error_code) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                invocation.actor_id,
                invocation.tool_name,
                target_audit_key,
                category_slug,
                if dry_run { 1_i64 } else { 0_i64 },
                outcome,
                error_code,
            ],
        )
        .map_err(|e| PlacementError::internal(e.to_string()))
        .and_then(|rows| {
            if rows == 1 {
                Ok(())
            } else {
                Err(PlacementError::internal(
                    "placement success row was not in progress",
                ))
            }
        });
    finish_tx(conn, result)
}

fn finish_tx<T>(conn: &Connection, result: Result<T, PlacementError>) -> Result<T, PlacementError> {
    match result {
        Ok(value) => {
            conn.execute("COMMIT", [])
                .map_err(|e| PlacementError::internal(e.to_string()))?;
            Ok(value)
        }
        Err(error) => {
            drop(conn.execute("ROLLBACK", []));
            Err(error)
        }
    }
}

fn parse_placeable_entity_type(value: &str) -> Result<EntityType, PlacementError> {
    match value {
        "account" => Ok(EntityType::Account),
        "person" => Ok(EntityType::Person),
        "project" => Ok(EntityType::Project),
        _ => Err(PlacementError::new(
            PlacementErrorCode::InvalidEntityType,
            "entity_type must be account, person, or project",
        )),
    }
}

fn validate_entity_id(value: &str) -> Result<(), PlacementError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-'))
    {
        return Err(PlacementError::new(
            PlacementErrorCode::InvalidEntityId,
            "entity_id is invalid",
        ));
    }
    Ok(())
}

fn normalize_content_type(value: &str) -> Result<&'static str, PlacementError> {
    match value {
        "text/markdown" => Ok("text/markdown"),
        "text/plain" => Ok("text/plain"),
        "application/json" => Ok("application/json"),
        _ => Err(PlacementError::new(
            PlacementErrorCode::InvalidContentType,
            "content_type is not supported",
        )),
    }
}

fn normalize_filename_hint(value: Option<&str>) -> Result<Option<String>, PlacementError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > 128
        || trimmed.starts_with('.')
        || trimmed.contains("..")
        || !trimmed.chars().enumerate().all(|(index, c)| {
            if index == 0 {
                c.is_ascii_alphanumeric()
            } else {
                c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')
            }
        })
    {
        return Err(PlacementError::new(
            PlacementErrorCode::InvalidFilenameHint,
            "filename_hint is invalid",
        ));
    }
    Ok(Some(trimmed.to_string()))
}

fn normalize_client_dedup_key(value: Option<&str>) -> Result<String, PlacementError> {
    let Some(value) = value else {
        return Ok(String::new());
    };
    let trimmed = value.trim();
    if trimmed.len() > 128
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-'))
    {
        return Err(PlacementError::new(
            PlacementErrorCode::InvalidClientDedupKey,
            "client_dedup_key is invalid",
        ));
    }
    Ok(trimmed.to_string())
}

fn decode_placement_content(value: &str) -> Result<Vec<u8>, PlacementError> {
    if !value.len().is_multiple_of(4)
        || value
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=')))
    {
        return Err(PlacementError::new(
            PlacementErrorCode::InvalidContentEncoding,
            "content_b64 must use standard padded base64",
        ));
    }
    if let Some(first_padding) = value.find('=') {
        if !value[first_padding..].chars().all(|c| c == '=') || value.len() - first_padding > 2 {
            return Err(PlacementError::new(
                PlacementErrorCode::InvalidContentEncoding,
                "content_b64 padding is invalid",
            ));
        }
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| {
            PlacementError::new(
                PlacementErrorCode::InvalidContentEncoding,
                "content_b64 is invalid",
            )
        })?;
    if bytes.len() > WORKSPACE_PLACE_DOCUMENT_DECODED_MAX_BYTES {
        return Err(PlacementError::new(
            PlacementErrorCode::ContentTooLarge,
            "decoded content exceeds the placement limit",
        ));
    }
    Ok(bytes)
}

fn placement_content_text(content_type: &str, content: Vec<u8>) -> Result<String, PlacementError> {
    // W4-C v1 only accepts textual document types. Keep the UTF-8 gate explicit
    // so JSON is treated as untrusted text bytes, not trusted structured data.
    String::from_utf8(content).map_err(|_| {
        PlacementError::new(
            PlacementErrorCode::InvalidContentEncoding,
            format!("{content_type} placement content must be valid UTF-8"),
        )
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn authorize_target(
    conn: &Connection,
    entity_type: EntityType,
    entity_id: &str,
) -> Result<String, PlacementError> {
    // W4-C's public entity_id is a service-owned opaque handle. In the local
    // v1 store, authorized entity readers already expose the canonical row key
    // as that handle; placement treats it as opaque and never derives names or
    // paths from caller input.
    let sql = match entity_type {
        EntityType::Account => "SELECT COALESCE(name, '') FROM accounts WHERE id = ?1 AND archived = 0",
        EntityType::Project => "SELECT COALESCE(name, '') FROM projects WHERE id = ?1 AND archived = 0",
        EntityType::Person => {
            "SELECT COALESCE(NULLIF(trim(name), ''), email, '') FROM people WHERE id = ?1 AND archived = 0"
        }
        EntityType::Other => {
            return Err(PlacementError::new(
                PlacementErrorCode::EntityNotRoutable,
                "entity type cannot receive workspace placement",
            ))
        }
    };
    conn.query_row(sql, params![entity_id], |row| row.get::<_, String>(0))
        .optional()
        .map_err(|e| PlacementError::internal(e.to_string()))?
        .ok_or_else(|| {
            PlacementError::new(
                PlacementErrorCode::TargetNotFoundOrUnauthorized,
                "target not found or unauthorized",
            )
        })
}

fn entity_path_slug(canonical_name: &str) -> Result<String, PlacementError> {
    let slug = crate::util::slugify(canonical_name);
    if is_valid_path_segment_slug(&slug) {
        Ok(slug)
    } else {
        Err(PlacementError::new(
            PlacementErrorCode::EntityNotRoutable,
            "entity path segment is not routable",
        ))
    }
}

fn parse_and_validate_category(
    conn: &Connection,
    entity_type: EntityType,
    value: &str,
) -> Result<WorkspaceCategory, PlacementError> {
    let category = WorkspaceCategory::from_slug(value).ok_or_else(|| {
        PlacementError::new(
            PlacementErrorCode::InvalidCategory,
            "category is not a valid slug",
        )
    })?;
    WorkspaceCategoryRegistry::validate(conn, &category, entity_type).map_err(|error| {
        PlacementError::new(
            PlacementErrorCode::CategoryNotAllowed,
            "category is not allowed for this entity type",
        )
        .with_allowed(error.allowed)
    })?;
    Ok(category)
}

fn category_slug_for_audit(req: &WorkspacePlaceDocumentRequest) -> Option<String> {
    let category = WorkspaceCategory::from_slug(&req.category)?;
    Some(category.as_slug().to_string())
}

fn preview_filename_for_content(content_type: &str, content_sha256: &str) -> String {
    let ext = extension_for_content_type(content_type);
    format!("placement-preview-{}.{}", &content_sha256[..12], ext)
}

fn generated_filename_for_idempotency(idempotency_id: &str, content_type: &str) -> String {
    format!(
        "{idempotency_id}.{}",
        extension_for_content_type(content_type)
    )
}

fn chosen_filename_for_idempotency(
    idempotency_id: &str,
    content_type: &str,
    filename_hint: Option<&str>,
) -> Result<String, PlacementError> {
    let Some(filename_hint) = filename_hint else {
        return Ok(generated_filename_for_idempotency(
            idempotency_id,
            content_type,
        ));
    };
    let ext = extension_for_content_type(content_type);
    let stem = filename_hint
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .filter(|stem| !stem.is_empty())
        .unwrap_or(filename_hint);
    let suffix = format!("-{idempotency_id}.{ext}");
    let max_stem_len = 128_usize.saturating_sub(suffix.len()).max(1);
    let truncated_stem = &stem[..stem.len().min(max_stem_len)];
    let filename = format!("{truncated_stem}{suffix}");
    normalize_filename_hint(Some(&filename))?
        .ok_or_else(|| PlacementError::internal("generated placement filename was empty"))
}

fn filename_belongs_to_idempotency(filename: &str, idempotency_id: &str) -> bool {
    filename.starts_with(&format!("{idempotency_id}."))
        || filename.contains(&format!("-{idempotency_id}."))
}

fn extension_for_content_type(content_type: &str) -> &'static str {
    match content_type {
        "text/plain" => "txt",
        "application/json" => "json",
        _ => "md",
    }
}

fn resolve_placement_path(
    conn: &Connection,
    entity_type: EntityType,
    entity_slug: &str,
    category: &WorkspaceCategory,
    filename: &str,
) -> Result<PathBuf, PlacementError> {
    WorkspaceCategoryRegistry::resolve_path(
        conn,
        entity_type,
        entity_slug,
        Some(category),
        filename,
        WorkspaceFileKind::McpPlacement,
    )
    .map_err(placement_path_error)
}

fn placement_path_error(error: ResolvePathError) -> PlacementError {
    match error {
        ResolvePathError::CategoryNotAllowed(error) => PlacementError::new(
            PlacementErrorCode::CategoryNotAllowed,
            "category is not allowed for this entity type",
        )
        .with_allowed(error.allowed),
        ResolvePathError::EntityTypeNotRoutable => PlacementError::new(
            PlacementErrorCode::EntityNotRoutable,
            "entity type cannot receive workspace placement",
        ),
        ResolvePathError::DbError(message) => PlacementError::internal(message),
    }
}

#[derive(Debug, Clone)]
struct PlacementRow {
    idempotency_id: String,
    status: String,
    document_handle: Option<String>,
    source_handle: Option<String>,
    chosen_filename: Option<String>,
    file_id: Option<String>,
    source_asof: Option<String>,
    lifecycle_state: Option<String>,
    claim_count_produced: u64,
    stale_after: String,
}

enum PlacementFence {
    Reserved(PlacementRow),
    Replay(PlacementRow),
    StaleFailed(PlacementRow),
}

struct PlacementIdempotencyInput<'a> {
    content_type: &'a str,
    category_slug: &'a str,
    content_sha256: &'a str,
    client_dedup_key: &'a str,
    filename_hint: Option<&'a str>,
}

fn reserve_or_load_idempotency(
    conn: &Connection,
    invocation: &PlacementInvocationContext,
    req: &WorkspacePlaceDocumentRequest,
    input: PlacementIdempotencyInput<'_>,
) -> Result<PlacementFence, PlacementError> {
    let idempotency_id = format!("placement_{}", uuid::Uuid::new_v4());
    let document_handle = format!("placement_{}", uuid::Uuid::new_v4());
    let chosen_filename =
        chosen_filename_for_idempotency(&idempotency_id, input.content_type, input.filename_hint)?;

    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| PlacementError::internal(e.to_string()))?;
    let result = (|| {
        let inserted: Option<String> = conn
            .query_row(
                "INSERT INTO workspace_placement_idempotency \
                 (idempotency_id, actor_id, entity_type, entity_id, content_sha256, \
                  content_type, category_slug, client_dedup_key, status, document_handle, \
                  chosen_filename) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'in_progress', ?9, ?10) \
                 ON CONFLICT(actor_id, entity_type, entity_id, content_sha256, content_type, category_slug, client_dedup_key) \
                 DO NOTHING RETURNING idempotency_id",
                params![
                    idempotency_id,
                    invocation.actor_id,
                    req.entity.entity_type,
                    req.entity.entity_id,
                    input.content_sha256,
                    input.content_type,
                    input.category_slug,
                    input.client_dedup_key,
                    document_handle,
                    chosen_filename,
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| PlacementError::internal(e.to_string()))?;

        let row = if let Some(inserted_id) = inserted {
            load_placement_row_by_id(conn, &inserted_id)?
        } else {
            load_placement_row_by_key(
                conn,
                invocation,
                req,
                input.content_type,
                input.category_slug,
                input.content_sha256,
                input.client_dedup_key,
            )?
        };

        if row.status == "in_progress" && row.idempotency_id == idempotency_id {
            return Ok(PlacementFence::Reserved(row));
        }
        match row.status.as_str() {
            "succeeded" => Ok(PlacementFence::Replay(row)),
            "failed" => Err(PlacementError::new(
                PlacementErrorCode::PreviousAttemptFailed,
                "previous placement attempt failed; use a new client_dedup_key to retry",
            )),
            "in_progress" => {
                if placement_row_is_stale(&row) {
                    if let Some(updated) = reconcile_stale_placement_success_in_tx(conn, &row)? {
                        Ok(PlacementFence::Replay(updated))
                    } else {
                        mark_placement_failed_in_tx(
                            conn,
                            &row.idempotency_id,
                            PlacementErrorCode::PreviousAttemptFailed,
                        )?;
                        Ok(PlacementFence::StaleFailed(row))
                    }
                } else {
                    Err(PlacementError::new(
                        PlacementErrorCode::IdempotencyInProgress,
                        "placement is already in progress",
                    )
                    .with_retry_after(retry_after_seconds(&row)))
                }
            }
            _ => Err(PlacementError::internal("unknown placement status")),
        }
    })();
    finish_tx(conn, result)
}

fn load_placement_row_by_id(conn: &Connection, id: &str) -> Result<PlacementRow, PlacementError> {
    conn.query_row(
        "SELECT idempotency_id, status, document_handle, source_handle, chosen_filename, \
         file_id, source_asof, lifecycle_state, claim_count_produced, stale_after \
         FROM workspace_placement_idempotency WHERE idempotency_id = ?1",
        params![id],
        row_to_placement,
    )
    .map_err(|e| PlacementError::internal(e.to_string()))
}

fn load_placement_row_by_key(
    conn: &Connection,
    invocation: &PlacementInvocationContext,
    req: &WorkspacePlaceDocumentRequest,
    content_type: &str,
    category_slug: &str,
    content_sha256: &str,
    client_dedup_key: &str,
) -> Result<PlacementRow, PlacementError> {
    conn.query_row(
        "SELECT idempotency_id, status, document_handle, source_handle, chosen_filename, \
         file_id, source_asof, lifecycle_state, claim_count_produced, stale_after \
         FROM workspace_placement_idempotency \
         WHERE actor_id = ?1 AND entity_type = ?2 AND entity_id = ?3 AND content_sha256 = ?4 \
           AND content_type = ?5 AND category_slug = ?6 AND client_dedup_key = ?7",
        params![
            invocation.actor_id,
            req.entity.entity_type,
            req.entity.entity_id,
            content_sha256,
            content_type,
            category_slug,
            client_dedup_key,
        ],
        row_to_placement,
    )
    .map_err(|e| PlacementError::internal(e.to_string()))
}

fn row_to_placement(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlacementRow> {
    let claim_count: i64 = row.get(8)?;
    Ok(PlacementRow {
        idempotency_id: row.get(0)?,
        status: row.get(1)?,
        document_handle: row.get(2)?,
        source_handle: row.get(3)?,
        chosen_filename: row.get(4)?,
        file_id: row.get(5)?,
        source_asof: row.get(6)?,
        lifecycle_state: row.get(7)?,
        claim_count_produced: claim_count.max(0) as u64,
        stale_after: row.get(9)?,
    })
}

fn placement_row_is_stale(row: &PlacementRow) -> bool {
    DateTime::parse_from_rfc3339(&row.stale_after)
        .map(|stale_after| Utc::now() >= stale_after.with_timezone(&Utc))
        .unwrap_or(true)
}

fn retry_after_seconds(row: &PlacementRow) -> u64 {
    DateTime::parse_from_rfc3339(&row.stale_after)
        .map(|stale_after| {
            stale_after
                .with_timezone(&Utc)
                .signed_duration_since(Utc::now())
                .num_seconds()
                .max(1) as u64
        })
        .unwrap_or(60)
}

fn mark_placement_failed(
    conn: &Connection,
    idempotency_id: &str,
    error_code: PlacementErrorCode,
) -> Result<(), PlacementError> {
    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| PlacementError::internal(e.to_string()))?;
    let result = mark_placement_failed_in_tx(conn, idempotency_id, error_code);
    finish_tx(conn, result)
}

fn mark_placement_failed_in_tx(
    conn: &Connection,
    idempotency_id: &str,
    error_code: PlacementErrorCode,
) -> Result<(), PlacementError> {
    conn.execute(
        "UPDATE workspace_placement_idempotency SET status = 'failed', error_code = ?1, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE idempotency_id = ?2",
        params![error_code.as_str(), idempotency_id],
    )
    .map(|_| ())
    .map_err(|e| PlacementError::internal(e.to_string()))
}

fn reconcile_stale_placement_success_in_tx(
    conn: &Connection,
    row: &PlacementRow,
) -> Result<Option<PlacementRow>, PlacementError> {
    let Some(file_id) = row.file_id.as_deref() else {
        return Ok(None);
    };
    if row.document_handle.is_none() || row.source_handle.is_none() || row.source_asof.is_none() {
        return Ok(None);
    }

    let recovered: Option<(String, i64, Option<String>)> = conn
        .query_row(
            "SELECT r.run_id, r.claim_count_produced, l.lifecycle_state \
             FROM document_ingestion_runs r \
             LEFT JOIN workspace_file_lifecycle l ON l.file_id = r.file_id \
             WHERE r.file_id = ?1 AND r.status = 'success' \
             ORDER BY r.completed_at DESC, r.id DESC LIMIT 1",
            params![file_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|e| PlacementError::internal(e.to_string()))?;

    let Some((run_id, claim_count, lifecycle_state)) = recovered else {
        return Ok(None);
    };
    let lifecycle_state = lifecycle_state.unwrap_or_else(|| "ingested".to_string());
    conn.execute(
        "UPDATE workspace_placement_idempotency SET status = 'succeeded', run_id = ?1, \
         lifecycle_state = ?2, claim_count_produced = ?3, error_code = NULL, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
         WHERE idempotency_id = ?4 AND document_handle IS NOT NULL \
           AND source_handle IS NOT NULL AND source_asof IS NOT NULL AND file_id IS NOT NULL",
        params![
            run_id,
            lifecycle_state,
            claim_count.max(0),
            row.idempotency_id.as_str(),
        ],
    )
    .map_err(|e| PlacementError::internal(e.to_string()))?;

    load_placement_row_by_id(conn, &row.idempotency_id).map(Some)
}

struct PlacementCommitStarted<'a> {
    idempotency_id: &'a str,
    source_handle: &'a str,
    file_id: &'a str,
    source_asof: DateTime<Utc>,
}

fn record_placement_commit_started(
    conn: &Connection,
    update: PlacementCommitStarted<'_>,
) -> Result<(), PlacementError> {
    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| PlacementError::internal(e.to_string()))?;
    let result = conn
        .execute(
            "UPDATE workspace_placement_idempotency SET source_handle = ?1, file_id = ?2, \
             source_asof = ?3, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
             WHERE idempotency_id = ?4 AND status = 'in_progress'",
            params![
                update.source_handle,
                update.file_id,
                update
                    .source_asof
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                update.idempotency_id,
            ],
        )
        .map_err(|e| PlacementError::internal(e.to_string()))
        .and_then(|rows| {
            if rows == 1 {
                Ok(())
            } else {
                Err(PlacementError::internal(
                    "placement commit-start row was not in progress",
                ))
            }
        });
    finish_tx(conn, result)
}

struct PlacementSuccessUpdate<'a> {
    idempotency_id: &'a str,
    source_handle: &'a str,
    file_id: &'a str,
    run_id: &'a str,
    source_asof: DateTime<Utc>,
    lifecycle_state: &'a str,
    claim_count: u64,
}

fn update_placement_success(
    conn: &Connection,
    update: PlacementSuccessUpdate<'_>,
) -> Result<(), PlacementError> {
    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| PlacementError::internal(e.to_string()))?;
    let result = conn
        .execute(
            "UPDATE workspace_placement_idempotency SET status = 'succeeded', source_handle = ?1, \
             file_id = ?2, run_id = ?3, source_asof = ?4, lifecycle_state = ?5, \
             claim_count_produced = ?6, error_code = NULL, \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
             WHERE idempotency_id = ?7 AND status = 'in_progress'",
            params![
                update.source_handle,
                update.file_id,
                update.run_id,
                update
                    .source_asof
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                update.lifecycle_state,
                update.claim_count as i64,
                update.idempotency_id,
            ],
        )
        .map_err(|e| PlacementError::internal(e.to_string()))
        .and_then(|rows| {
            if rows == 1 {
                Ok(())
            } else {
                Err(PlacementError::internal(
                    "placement success row was not in progress",
                ))
            }
        });
    finish_tx(conn, result)
}

fn dry_run_receipt(
    req: &WorkspacePlaceDocumentRequest,
    category: &WorkspaceCategory,
) -> WorkspacePlaceDocumentReceipt {
    WorkspacePlaceDocumentReceipt {
        schema_version: WORKSPACE_PLACE_DOCUMENT_SCHEMA_VERSION,
        document_handle: None,
        source_handle: None,
        entity_type: req.entity.entity_type.clone(),
        entity_id: req.entity.entity_id.clone(),
        category: category.as_slug().to_string(),
        workspace_file_kind: "mcp_placement".to_string(),
        source_asof: None,
        lifecycle_state: "not_written".to_string(),
        claim_count_produced: 0,
        idempotent_replay: false,
        dry_run: true,
        resolved_path: None,
        mutation_cursor: WorkspacePlacementMutationCursor::WorkspacePlacementPreview {
            dry_run: true,
        },
    }
}

fn render_live_receipt(
    invocation: &PlacementInvocationContext,
    req: &WorkspacePlaceDocumentRequest,
    category: &WorkspaceCategory,
    row: &PlacementRow,
    relative_path: Option<&PathBuf>,
    idempotent_replay: bool,
) -> Result<WorkspacePlaceDocumentReceipt, PlacementError> {
    let document_handle = row
        .document_handle
        .clone()
        .ok_or_else(|| PlacementError::internal("missing document handle"))?;
    let source_handle = row
        .source_handle
        .clone()
        .ok_or_else(|| PlacementError::internal("missing source handle"))?;
    let resolved_path = if invocation.can_read_entity_names {
        relative_path.map(|path| path.to_string_lossy().to_string())
    } else {
        None
    };
    Ok(WorkspacePlaceDocumentReceipt {
        schema_version: WORKSPACE_PLACE_DOCUMENT_SCHEMA_VERSION,
        document_handle: Some(document_handle.clone()),
        source_handle: Some(source_handle.clone()),
        entity_type: req.entity.entity_type.clone(),
        entity_id: req.entity.entity_id.clone(),
        category: category.as_slug().to_string(),
        workspace_file_kind: "mcp_placement".to_string(),
        source_asof: row.source_asof.clone(),
        lifecycle_state: row
            .lifecycle_state
            .clone()
            .unwrap_or_else(|| "ingested".to_string()),
        claim_count_produced: row.claim_count_produced,
        idempotent_replay,
        dry_run: false,
        resolved_path,
        mutation_cursor: WorkspacePlacementMutationCursor::WorkspacePlacement {
            document_handle,
            source_handle,
            idempotency_id: row.idempotency_id.clone(),
        },
    })
}

fn stale_generated_placement_path(
    conn: &Connection,
    entity_type: EntityType,
    entity_slug: &str,
    category: &WorkspaceCategory,
    row: &PlacementRow,
) -> Option<PathBuf> {
    let filename = row.chosen_filename.as_deref()?;
    if !filename_belongs_to_idempotency(filename, &row.idempotency_id) {
        return None;
    }
    resolve_placement_path(conn, entity_type, entity_slug, category, filename)
        .inspect_err(|error| {
            log::warn!(
                "workspace placement stale cleanup path resolution failed: {}",
                error.code
            );
        })
        .ok()
}

fn preview_handle_relative_path(
    workspace_root: &Path,
    relative_path: &Path,
) -> Result<(), PlacementError> {
    validate_relative_components(relative_path)?;
    #[cfg(not(unix))]
    {
        let _ = workspace_root;
        return Err(PlacementError::new(
            PlacementErrorCode::PlacementPathRejected,
            "platform lacks handle-relative placement support",
        ));
    }
    #[cfg(unix)]
    {
        let root_dir = open_workspace_root_no_follow(workspace_root).map_err(|error| {
            log::warn!(
                "workspace placement root preview failed: {}",
                io_error_redacted(error)
            );
            PlacementError::new(
                PlacementErrorCode::PlacementPathRejected,
                "placement path rejected",
            )
        })?;
        let root_dev = root_dir
            .metadata()
            .map_err(|e| PlacementError::internal(io_error_redacted(e)))?
            .dev();
        let mut current = root_dir;
        let mut components = relative_path.components().peekable();
        while let Some(component) = components.next() {
            let Component::Normal(name) = component else {
                return Err(path_rejected("invalid path component"));
            };
            if components.peek().is_none() {
                return Ok(());
            }
            match open_dir_no_follow(current.as_raw_fd(), name) {
                Ok(next) => {
                    ensure_same_device(&next, root_dev)?;
                    current = next;
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(error) => {
                    log::warn!(
                        "workspace placement directory preview failed: {}",
                        io_error_redacted(error)
                    );
                    return Err(PlacementError::new(
                        PlacementErrorCode::PlacementPathRejected,
                        "placement path rejected",
                    ));
                }
            }
        }
        Ok(())
    }
}

struct PlacementWriteFailure {
    error: io::Error,
    created_file: bool,
}

impl PlacementWriteFailure {
    fn before_create(error: io::Error) -> Self {
        Self {
            error,
            created_file: false,
        }
    }

    fn after_create(error: io::Error) -> Self {
        Self {
            error,
            created_file: true,
        }
    }
}

fn write_file_handle_relative(
    workspace_root: &Path,
    relative_path: &Path,
    bytes: &[u8],
) -> Result<(), PlacementWriteFailure> {
    validate_relative_components_io(relative_path).map_err(PlacementWriteFailure::before_create)?;
    #[cfg(not(unix))]
    {
        let _ = (workspace_root, relative_path, bytes);
        return Err(PlacementWriteFailure::before_create(io::Error::new(
            io::ErrorKind::Unsupported,
            "platform lacks handle-relative placement support",
        )));
    }
    #[cfg(unix)]
    {
        let root_dir = open_workspace_root_no_follow(workspace_root)
            .map_err(PlacementWriteFailure::before_create)?;
        let root_dev = root_dir
            .metadata()
            .map_err(PlacementWriteFailure::before_create)?
            .dev();
        let mut current = root_dir;
        let mut components = relative_path.components().peekable();
        while let Some(component) = components.next() {
            let Component::Normal(name) = component else {
                return Err(PlacementWriteFailure::before_create(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "bad component",
                )));
            };
            if components.peek().is_none() {
                let mut file = create_file_no_follow(current.as_raw_fd(), name)
                    .map_err(PlacementWriteFailure::before_create)?;
                file.write_all(bytes)
                    .map_err(PlacementWriteFailure::after_create)?;
                file.sync_all()
                    .map_err(PlacementWriteFailure::after_create)?;
                let metadata = file
                    .metadata()
                    .map_err(PlacementWriteFailure::after_create)?;
                if metadata.dev() != root_dev || metadata.nlink() > 1 {
                    return Err(PlacementWriteFailure::after_create(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "placement target escaped workspace",
                    )));
                }
                return Ok(());
            }
            mkdirat_if_missing(current.as_raw_fd(), name)
                .map_err(PlacementWriteFailure::before_create)?;
            let next = open_dir_no_follow(current.as_raw_fd(), name)
                .map_err(PlacementWriteFailure::before_create)?;
            ensure_same_device_io(&next, root_dev).map_err(PlacementWriteFailure::before_create)?;
            current = next;
        }
        Ok(())
    }
}

fn best_effort_remove_placement_file(workspace_root: &Path, relative_path: &Path, context: &str) {
    if let Err(error) = remove_file_handle_relative(workspace_root, relative_path) {
        log::warn!(
            "workspace placement file cleanup failed after {context}: {}",
            io_error_redacted(error)
        );
    }
}

fn remove_file_handle_relative(workspace_root: &Path, relative_path: &Path) -> io::Result<()> {
    validate_relative_components_io(relative_path)?;
    #[cfg(not(unix))]
    {
        let _ = (workspace_root, relative_path);
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "platform lacks handle-relative placement cleanup support",
        ));
    }
    #[cfg(unix)]
    {
        let root_dir = open_workspace_root_no_follow(workspace_root)?;
        let root_dev = root_dir.metadata()?.dev();
        let mut current = root_dir;
        let mut components = relative_path.components().peekable();
        while let Some(component) = components.next() {
            let Component::Normal(name) = component else {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "bad component"));
            };
            if components.peek().is_none() {
                let c_name = cstring_component(name)?;
                let result = unsafe { libc::unlinkat(current.as_raw_fd(), c_name.as_ptr(), 0) };
                if result == 0 {
                    return Ok(());
                }
                let error = io::Error::last_os_error();
                return if error.kind() == io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(error)
                };
            }
            match open_dir_no_follow(current.as_raw_fd(), name) {
                Ok(next) => {
                    ensure_same_device_io(&next, root_dev)?;
                    current = next;
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

fn validate_relative_components(path: &Path) -> Result<(), PlacementError> {
    validate_relative_components_io(path).map_err(|_| {
        PlacementError::new(
            PlacementErrorCode::PlacementPathRejected,
            "placement path rejected",
        )
    })
}

fn validate_relative_components_io(path: &Path) -> io::Result<()> {
    if path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "absolute path rejected",
        ));
    }
    for component in path.components() {
        let Component::Normal(name) = component else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "non-normal path component rejected",
            ));
        };
        let text = name.to_string_lossy();
        if text.is_empty()
            || text == "."
            || text == ".."
            || text.starts_with('.')
            || text.contains('/')
            || text.contains('\\')
            || text.contains('\0')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsafe path component rejected",
            ));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn open_workspace_root_no_follow(workspace_root: &Path) -> io::Result<File> {
    let root = workspace_root.canonicalize()?;
    let expected = fs::metadata(&root)?;
    let root_dir = open_canonical_dir_no_follow(&root)?;
    let actual = root_dir.metadata()?;
    if expected.dev() == actual.dev() && expected.ino() == actual.ino() {
        Ok(root_dir)
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "workspace root changed during placement open",
        ))
    }
}

#[cfg(unix)]
fn open_canonical_dir_no_follow(canonical_path: &Path) -> io::Result<File> {
    let mut components = canonical_path.components();
    let Some(Component::RootDir) = components.next() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "canonical workspace root is not absolute",
        ));
    };
    let mut current = File::open(Path::new("/"))?;
    for component in components {
        let Component::Normal(name) = component else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "canonical workspace root contains non-normal component",
            ));
        };
        current = open_dir_no_follow(current.as_raw_fd(), name)?;
    }
    Ok(current)
}

#[cfg(unix)]
fn cstring_component(name: &std::ffi::OsStr) -> io::Result<CString> {
    CString::new(name.as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path component contains NUL"))
}

#[cfg(unix)]
fn mkdirat_if_missing(parent: RawFd, name: &std::ffi::OsStr) -> io::Result<()> {
    let c_name = cstring_component(name)?;
    let result = unsafe { libc::mkdirat(parent, c_name.as_ptr(), 0o700) };
    if result == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::AlreadyExists {
        Ok(())
    } else {
        Err(error)
    }
}

#[cfg(unix)]
fn open_dir_no_follow(parent: RawFd, name: &std::ffi::OsStr) -> io::Result<File> {
    let c_name = cstring_component(name)?;
    let fd = unsafe {
        libc::openat(
            parent,
            c_name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(unix)]
fn create_file_no_follow(parent: RawFd, name: &std::ffi::OsStr) -> io::Result<File> {
    let c_name = cstring_component(name)?;
    let fd = unsafe {
        libc::openat(
            parent,
            c_name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(unix)]
fn ensure_same_device(file: &File, root_dev: u64) -> Result<(), PlacementError> {
    ensure_same_device_io(file, root_dev).map_err(|error| {
        log::warn!(
            "workspace placement cross-device check failed: {}",
            io_error_redacted(error)
        );
        PlacementError::new(
            PlacementErrorCode::PlacementPathRejected,
            "placement path rejected",
        )
    })
}

#[cfg(unix)]
fn ensure_same_device_io(file: &File, root_dev: u64) -> io::Result<()> {
    let metadata = file.metadata()?;
    if metadata.dev() == root_dev {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "cross-device placement rejected",
        ))
    }
}

fn path_rejected(message: impl Into<String>) -> PlacementError {
    drop(message.into());
    PlacementError::new(
        PlacementErrorCode::PlacementPathRejected,
        "placement path rejected",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::workspace_graph::contracts::{
        WorkspaceGraphInput, WorkspaceGraphPrivacyProfile, WorkspaceGraphReadRequest,
        WorkspaceGraphResponse,
    };
    use crate::db::DbAccount;
    use crate::services::workspace_ingestion::graph::{
        diagnostic_key_for_tests, read_workspace_graph,
    };
    use abilities_runtime::services::workspace_intake::{
        WorkspacePlacementEntity, WORKSPACE_PLACE_DOCUMENT_TOOL_NAME,
    };

    fn placement_request(dry_run: bool) -> WorkspacePlaceDocumentRequest {
        WorkspacePlaceDocumentRequest {
            schema_version: WORKSPACE_PLACE_DOCUMENT_SCHEMA_VERSION,
            entity: WorkspacePlacementEntity {
                entity_type: "account".to_string(),
                entity_id: "acct_123".to_string(),
            },
            content_b64: "aGVsbG8=".to_string(),
            content_type: "text/markdown".to_string(),
            filename_hint: None,
            category: "notes".to_string(),
            client_dedup_key: None,
            dry_run,
        }
    }

    fn count<P>(conn: &Connection, sql: &str, params: P) -> i64
    where
        P: rusqlite::Params,
    {
        conn.query_row(sql, params, |row| row.get(0))
            .expect("count query")
    }

    #[cfg(unix)]
    #[test]
    fn workspace_placement_success_commits_claim_and_graph_without_path_leak() {
        let conn = Connection::open_in_memory().expect("in-memory db");
        crate::migrations::run_migrations(&conn).expect("migrations");
        let db = ActionDb::from_conn(&conn);
        db.upsert_account(&DbAccount {
            id: "acct_123".to_string(),
            name: "Placement Account".to_string(),
            tracker_path: Some("Accounts/Placement Account".to_string()),
            updated_at: Utc::now().to_rfc3339(),
            ..Default::default()
        })
        .expect("account seed");

        let workspace = tempfile::tempdir().expect("workspace");
        let workspace_root = workspace.path().canonicalize().expect("workspace root");
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let ctx = ServiceContext::new_live(&clock, &rng, &external)
            .with_actor("system:workspace_placement_test");
        let signal_engine = crate::signals::propagation::default_engine();
        let mut request = placement_request(false);
        request.content_b64 = "UGxhY2VtZW50IGdyYXBoIHZhbGlkYXRpb24gbm90ZS4=".to_string();
        request.client_dedup_key = Some("placement-claim-fixture".to_string());
        let invocation = PlacementInvocationContext {
            actor_id: "mcp_client_validation".to_string(),
            tool_name: WORKSPACE_PLACE_DOCUMENT_TOOL_NAME.to_string(),
            can_read_entity_names: false,
        };

        let receipt = place_document_after_rate(
            &ctx,
            db,
            &workspace_root,
            Some(&signal_engine),
            &invocation,
            &request,
            "target_opaque",
        )
        .expect("placement succeeds");

        assert_eq!(receipt.lifecycle_state, "ingested");
        assert_eq!(receipt.workspace_file_kind, "mcp_placement");
        assert_eq!(receipt.claim_count_produced, 1);
        assert_eq!(receipt.entity_type, "account");
        assert_eq!(receipt.entity_id, "acct_123");
        assert_eq!(receipt.category, "notes");
        assert!(receipt.document_handle.is_some());
        assert!(receipt.source_handle.is_some());
        assert_eq!(
            receipt.resolved_path, None,
            "MCP placement receipt must not expose a path when entity-name reads are denied"
        );
        let WorkspacePlacementMutationCursor::WorkspacePlacement {
            document_handle,
            source_handle,
            idempotency_id,
        } = &receipt.mutation_cursor
        else {
            panic!("successful placement must return a mutation cursor");
        };
        assert_eq!(Some(document_handle.clone()), receipt.document_handle);
        assert_eq!(Some(source_handle.clone()), receipt.source_handle);
        assert!(idempotency_id.starts_with("placement_"));

        let file_id = conn
            .query_row(
                "SELECT file_id
                 FROM workspace_placement_idempotency
                 WHERE idempotency_id = ?1
                   AND status = 'succeeded'
                   AND run_id IS NOT NULL
                   AND claim_count_produced = 1",
                params![idempotency_id],
                |row| row.get::<_, String>(0),
            )
            .expect("placement idempotency success row");
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*)
                 FROM workspace_file_lifecycle
                 WHERE file_id = ?1
                   AND lifecycle_state = 'ingested'
                   AND source_type = 'mcp_placement'",
                params![&file_id],
            ),
            1
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*)
                 FROM document_ingestion_runs
                 WHERE file_id = ?1
                   AND mode = 'entity_seeded'
                   AND status = 'success'
                   AND claim_count_produced = 1",
                params![&file_id],
            ),
            1
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*)
                 FROM document_entity_links
                 WHERE file_id = ?1
                   AND entity_type = 'account'
                   AND entity_id = 'acct_123'
                   AND attribution_source = 'mcp_placement'
                   AND rejected = 0",
                params![&file_id],
            ),
            1
        );

        let source_ref = format!("workspace_file:{file_id}");
        let (data_source, metadata_json, provenance_json, sensitivity): (
            String,
            String,
            String,
            String,
        ) = conn
            .query_row(
                "SELECT data_source, metadata_json, provenance_json, sensitivity
                 FROM intelligence_claims
                 WHERE source_ref = ?1",
                params![&source_ref],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("workspace placement claim");
        assert_eq!(data_source, "workspace_file:mcp_placement");
        assert_eq!(sensitivity, "user_only");
        let metadata: serde_json::Value =
            serde_json::from_str(&metadata_json).expect("metadata json");
        assert_eq!(metadata["producer"], "workspace_ingestion");
        assert_eq!(metadata["workspace_file_id"], file_id);
        assert_eq!(metadata["workspace_file_kind"], "mcp_placement");
        assert_eq!(metadata["resolved_category"], "notes");
        assert!(metadata["ingestion_run_id"]
            .as_str()
            .is_some_and(|value| !value.is_empty()));
        assert!(metadata["document_entity_link_id"]
            .as_str()
            .is_some_and(|value| !value.is_empty()));

        for forbidden in [
            "Placement Account",
            "Placement graph validation note",
            workspace_root.to_string_lossy().as_ref(),
        ] {
            assert!(
                !metadata_json.contains(forbidden),
                "claim metadata leaked raw fixture detail `{forbidden}`"
            );
            assert!(
                !provenance_json.contains(forbidden),
                "claim provenance leaked raw fixture detail `{forbidden}`"
            );
        }

        let graph = read_workspace_graph(
            &conn,
            WorkspaceGraphReadRequest {
                input: WorkspaceGraphInput {
                    schema_version: 1,
                    entity_filter: None,
                    category_filter: None,
                    cursor: None,
                    if_none_match: None,
                    include_entity_names: false,
                    page_size: 50,
                },
                privacy_profile: WorkspaceGraphPrivacyProfile::FirstParty,
            },
            &diagnostic_key_for_tests("workspace-placement-success"),
        )
        .expect("workspace graph read");
        let WorkspaceGraphResponse::Projection(projection) = graph else {
            panic!("expected workspace graph projection");
        };
        assert!(
            projection.audit.gaps.is_empty(),
            "workspace graph audit gaps: {:?}",
            projection.audit.gaps
        );
        let entity = projection
            .projection
            .entities
            .iter()
            .find(|entity| entity.entity_type == "account" && entity.entity_id == "acct_123")
            .expect("placement graph entity");
        assert_eq!(entity.file_links.len(), 1);
        assert_eq!(entity.claim_summary.total, 1);

        let serialized = serde_json::to_string(&projection).expect("projection json");
        for forbidden in [
            "Placement Account",
            "Placement graph validation note",
            workspace_root.to_string_lossy().as_ref(),
        ] {
            assert!(
                !serialized.contains(forbidden),
                "workspace graph leaked raw fixture detail `{forbidden}`"
            );
        }
    }

    #[test]
    fn workspace_placement_content_requires_standard_padded_base64() {
        assert_eq!(
            decode_placement_content("aGVsbG8=").expect("standard padded base64 decodes"),
            b"hello"
        );

        for value in ["aGVsbG8", "aGVsbG8_", "aGVs bG8=", "a=GVsbG8"] {
            let error =
                decode_placement_content(value).expect_err("non-standard base64 must be rejected");
            assert_eq!(
                error.code,
                PlacementErrorCode::InvalidContentEncoding.as_str()
            );
        }
    }

    #[test]
    fn workspace_placement_textual_content_requires_utf8() {
        assert_eq!(
            placement_content_text("application/json", br#"{"ok":true}"#.to_vec())
                .expect("valid UTF-8 JSON text is accepted"),
            r#"{"ok":true}"#
        );

        let error = placement_content_text("application/json", vec![0xff])
            .expect_err("non-UTF-8 textual placement content is rejected");
        assert_eq!(
            error.code,
            PlacementErrorCode::InvalidContentEncoding.as_str()
        );
    }

    #[test]
    fn workspace_placement_target_audit_key_is_stable_hmac_without_raw_target() {
        let key = crate::db::local_db_keyed_audit_tag_for_tests(
            "secret",
            "target",
            "workspace-placement-target-v1",
            &["account", "acct_123"],
        );

        assert_eq!(
            key,
            crate::db::local_db_keyed_audit_tag_for_tests(
                "secret",
                "target",
                "workspace-placement-target-v1",
                &["account", "acct_123"],
            )
        );
        assert_ne!(
            key,
            crate::db::local_db_keyed_audit_tag_for_tests(
                "secret",
                "target",
                "workspace-placement-target-v1",
                &["account", "acct_124"],
            )
        );
        assert!(key.starts_with("target_"));
        assert_eq!(key.len(), "target_".len() + 32);
        assert!(!key.contains("account"));
        assert!(!key.contains("acct_123"));
    }

    #[test]
    fn workspace_placement_dry_run_receipt_does_not_return_handles_or_paths() {
        let req = placement_request(true);
        let receipt = dry_run_receipt(&req, &WorkspaceCategory::Notes);

        assert_eq!(
            receipt.schema_version,
            WORKSPACE_PLACE_DOCUMENT_SCHEMA_VERSION
        );
        assert_eq!(receipt.document_handle, None);
        assert_eq!(receipt.source_handle, None);
        assert_eq!(receipt.resolved_path, None);
        assert_eq!(receipt.lifecycle_state, "not_written");
        assert_eq!(receipt.workspace_file_kind, "mcp_placement");
        assert_eq!(receipt.claim_count_produced, 0);
        assert_eq!(
            receipt.mutation_cursor,
            WorkspacePlacementMutationCursor::WorkspacePlacementPreview { dry_run: true }
        );
    }

    #[test]
    fn workspace_placement_filename_hint_is_uniquified_by_idempotency_id() {
        let filename = chosen_filename_for_idempotency(
            "placement_1234567890abcdef",
            "text/markdown",
            Some("renewal-notes.md"),
        )
        .expect("hinted filename should remain valid after uniquification");

        assert_ne!(filename, "renewal-notes.md");
        assert_eq!(filename, "renewal-notes-placement_1234567890abcdef.md");
        assert!(filename_belongs_to_idempotency(
            &filename,
            "placement_1234567890abcdef"
        ));
        assert!(filename.len() <= 128);
    }

    #[test]
    fn workspace_placement_relative_component_validation_rejects_escape_and_hidden_segments() {
        assert!(validate_relative_components(Path::new("accounts/example/notes/file.md")).is_ok());
        assert!(validate_relative_components(Path::new("../file.md")).is_err());
        assert!(validate_relative_components(Path::new("/tmp/file.md")).is_err());
        assert!(validate_relative_components(Path::new("accounts/.hidden/file.md")).is_err());
    }

    #[test]
    fn workspace_placement_success_requires_in_progress_status() {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(include_str!(
            "../../migrations/262_workspace_placement_idempotency.sql"
        ))
        .expect("placement migration installs");
        conn.execute(
            "INSERT INTO workspace_placement_idempotency \
             (idempotency_id, actor_id, entity_type, entity_id, content_sha256, content_type, \
              category_slug, client_dedup_key, status) \
             VALUES (?1, 'actor', 'account', 'acct_123', 'hash', 'text/markdown', 'notes', '', 'failed')",
            rusqlite::params!["placement_test"],
        )
        .expect("failed placement row inserted");

        let error = update_placement_success(
            &conn,
            PlacementSuccessUpdate {
                idempotency_id: "placement_test",
                source_handle: "source_handle",
                file_id: "file_id",
                run_id: "run_id",
                source_asof: Utc::now(),
                lifecycle_state: "ingested",
                claim_count: 3,
            },
        )
        .expect_err("non-in-progress row cannot be marked succeeded");

        assert_eq!(error.code, PlacementErrorCode::PlacementInternal.as_str());
        let row: (String, Option<String>) = conn
            .query_row(
                "SELECT status, run_id FROM workspace_placement_idempotency WHERE idempotency_id = 'placement_test'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("placement row remains queryable");
        assert_eq!(row, ("failed".to_string(), None));
    }

    #[cfg(unix)]
    #[test]
    fn workspace_placement_root_handle_matches_canonical_root_inode() {
        let workspace = tempfile::tempdir().expect("workspace");
        let nested = workspace.path().join("root");
        fs::create_dir(&nested).expect("workspace root created");

        let expected = fs::metadata(&nested).expect("root metadata");
        let root =
            open_workspace_root_no_follow(&nested).expect("root opens through no-follow walk");
        let actual = root.metadata().expect("opened root metadata");

        assert_eq!(actual.dev(), expected.dev());
        assert_eq!(actual.ino(), expected.ino());
    }
}

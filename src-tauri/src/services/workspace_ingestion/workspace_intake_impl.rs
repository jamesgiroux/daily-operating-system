use std::path::{Path, PathBuf};
use std::sync::Arc;

use abilities_runtime::abilities::provenance::source::EntityId;
use abilities_runtime::services::workspace_intake::{
    EntityRefDto, WorkspaceIntakeError, WorkspaceIntakeReceipt, WorkspaceIntakeRequest,
    WorkspaceIntakeService,
};
use async_trait::async_trait;

use crate::db::{ActionDb, LocalKeychain};
use crate::entity::EntityType;

use super::contracts::{RejectionReason, WorkspaceCategory, WorkspaceFileKind};
use super::lifecycle::{lifecycle_state_slug, workspace_file_kind_from_slug};
use super::pipeline::{file_id_from_identity, EntityRef, FileIdError, IngestError, IngestRequest};
use super::registry::WorkspaceCategoryRegistry;
use super::wiring::build_pipeline;
use super::{registry::WorkspaceSourceRegistry, runs::IngestionMode};

pub struct IngestPipelineWorkspaceIntake {
    workspace_root: PathBuf,
}

impl IngestPipelineWorkspaceIntake {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    pub fn from_config_or_empty() -> Self {
        let workspace_root = crate::state::load_config()
            .map(|config| PathBuf::from(config.workspace_path))
            .unwrap_or_default();
        Self::new(workspace_root)
    }
}

#[async_trait]
impl WorkspaceIntakeService for IngestPipelineWorkspaceIntake {
    async fn ingest(
        &self,
        _ctx: &abilities_runtime::abilities::registry::AbilityContext<'_>,
        req: WorkspaceIntakeRequest,
    ) -> Result<WorkspaceIntakeReceipt, WorkspaceIntakeError> {
        let workspace_root = self.workspace_root.clone();
        tokio::task::spawn_blocking(move || ingest_sync(workspace_root, req))
            .await
            .map_err(|e| WorkspaceIntakeError::Io(format!("workspace intake task failed: {e}")))?
    }
}

fn ingest_sync(
    workspace_root: PathBuf,
    req: WorkspaceIntakeRequest,
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

    let db = ActionDb::open(Arc::new(LocalKeychain::new()))
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
        WorkspaceSourceRegistry::open_validated(&workspace_root, Path::new(&req.file_ref))
            .map_err(intake_error_from_rejection)?;
    let source_asof = file
        .metadata()
        .and_then(|m| m.modified())
        .map(chrono::DateTime::<chrono::Utc>::from)
        .map_err(|e| WorkspaceIntakeError::Io(io_error_redacted(e)))?;
    let file_id =
        file_id_from_identity(&identity, &workspace_root).map_err(intake_file_id_error)?;

    let request = IngestRequest {
        file,
        identity,
        file_id,
        source_asof,
        source_type,
        entity,
        mode,
        category_hint,
    };
    let pipeline = build_pipeline(workspace_root);
    let receipt = pipeline
        .run(conn, request)
        .map_err(intake_error_from_ingest)?;
    Ok(WorkspaceIntakeReceipt {
        run_id: receipt.ingestion_run_id.0,
        file_id: receipt.file_id,
        content_sha256: receipt.content_sha256,
        lifecycle_state_after_slug: lifecycle_state_slug(receipt.lifecycle_state_after).to_string(),
        resolved_path: receipt.resolved_path,
    })
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

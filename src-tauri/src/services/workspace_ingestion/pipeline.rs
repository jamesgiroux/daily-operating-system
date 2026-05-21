//! Staged workspace ingestion pipeline.
//!
//! W2-A owns the shell: validated file handle in, lifecycle/run bookkeeping,
//! category sniffing, quarantine mutation, and zero claim production.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use abilities_runtime::abilities::provenance::source::{EntityId, WorkspaceFileKind};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::entity::EntityType;

use super::contracts::{
    Extractor, FileIdentity, RejectionReason, SignalEmitter, WorkspaceCategory,
    WorkspaceClaimProposal,
};
use super::lifecycle::{LifecycleError, LifecycleRepo, LifecycleState};
use super::registry::{ResolvePathError, WorkspaceCategoryRegistry};
use super::runs::{
    IngestionMode, IngestionRunId, IngestionRunStatus, RunsError, RunsRepo, StartRunSeed,
};

const DEFAULT_MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
const CONTENT_HEAD_BYTES: usize = 4 * 1024;

pub struct IngestPipeline {
    pub extractor: Box<dyn Extractor>,
    pub signal_emitter: Box<dyn SignalEmitter>,
    pub max_file_bytes: u64,
    pub extractor_version: String,
    pub workspace_root: PathBuf,
}

pub struct IngestRequest {
    pub file: File,
    pub identity: FileIdentity,
    pub file_id: String,
    pub source_asof: DateTime<Utc>,
    pub source_type: WorkspaceFileKind,
    pub entity: Option<EntityRef>,
    pub mode: IngestionMode,
    pub category_hint: Option<WorkspaceCategory>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityRef {
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub entity_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IngestReceipt {
    pub ingestion_run_id: IngestionRunId,
    pub file_id: String,
    pub content_sha256: String,
    pub lifecycle_state_after: LifecycleState,
    pub claim_proposals: Vec<WorkspaceClaimProposal>,
    pub resolved_category: Option<WorkspaceCategory>,
    pub resolved_path: Option<String>,
}

#[derive(Debug)]
pub enum IngestError {
    FileIdMismatch { expected: String, found: String },
    Rejected(RejectionReason),
    AlreadyProcessed { existing_run_id: IngestionRunId },
    Io(std::io::Error),
    DbError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectionMetadata {
    pub limit_bytes: Option<u64>,
    pub found_bytes: Option<u64>,
    pub detected_format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileIdError {
    OutsideWorkspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuarantineActor {
    User { user_id: String },
}

impl IngestPipeline {
    pub fn new(
        extractor: Box<dyn Extractor>,
        signal_emitter: Box<dyn SignalEmitter>,
        max_file_bytes: u64,
        extractor_version: impl Into<String>,
        workspace_root: PathBuf,
    ) -> Self {
        Self {
            extractor,
            signal_emitter,
            max_file_bytes,
            extractor_version: extractor_version.into(),
            workspace_root,
        }
    }

    pub fn default_with(
        extractor: Box<dyn Extractor>,
        signal_emitter: Box<dyn SignalEmitter>,
        workspace_root: PathBuf,
    ) -> Self {
        Self::new(
            extractor,
            signal_emitter,
            DEFAULT_MAX_FILE_BYTES,
            "workspace-null-extractor-v1",
            workspace_root,
        )
    }

    pub fn run(
        &self,
        conn: &Connection,
        mut request: IngestRequest,
    ) -> Result<IngestReceipt, IngestError> {
        let expected = file_id_from_identity(&request.identity, &self.workspace_root)
            .map_err(|e| IngestError::DbError(format!("file id derivation failed: {e:?}")))?;
        if expected != request.file_id {
            return Err(IngestError::FileIdMismatch {
                expected,
                found: request.file_id,
            });
        }

        LifecycleRepo::insert_pending(
            conn,
            &request.file_id,
            &request.identity,
            &request.source_type,
            request.source_asof,
            request.entity.as_ref(),
        )?;

        let file_size = request.file.metadata().map_err(IngestError::Io)?.len();
        if file_size > self.max_file_bytes {
            self.reject_before_run(
                conn,
                &request.file_id,
                RejectionReason::FileTooLarge,
                LifecycleState::Pending,
            )?;
            return Err(IngestError::Rejected(RejectionReason::FileTooLarge));
        }

        let mut bytes = Vec::with_capacity(file_size as usize);
        request
            .file
            .read_to_end(&mut bytes)
            .map_err(IngestError::Io)?;
        if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
            self.reject_before_run(
                conn,
                &request.file_id,
                RejectionReason::UnsupportedFormat,
                LifecycleState::Pending,
            )?;
            return Err(IngestError::Rejected(RejectionReason::UnsupportedFormat));
        }

        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let content_sha256 = hex::encode(hasher.finalize());

        let content = std::str::from_utf8(&bytes)
            .map_err(|_| IngestError::Rejected(RejectionReason::UnsupportedFormat))?;
        let head_end = if content.len() <= CONTENT_HEAD_BYTES {
            content.len()
        } else {
            content
                .char_indices()
                .map(|(idx, _)| idx)
                .take_while(|idx| *idx <= CONTENT_HEAD_BYTES)
                .last()
                .unwrap_or(0)
        };
        let content_head = &content[..head_end];

        let filename = request
            .identity
            .canonical_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("workspace-file");

        let resolved_category = if let Some(category) = request.category_hint.clone() {
            Some(category)
        } else if let Some(entity) = request.entity.as_ref() {
            Self::validate_detected_category(
                conn,
                Self::auto_detect_category_pure(filename, content_head),
                entity.entity_type,
            )?
        } else {
            None
        };
        LifecycleRepo::update_category(conn, &request.file_id, resolved_category.as_ref())?;
        LifecycleRepo::update_content_sha256(conn, &request.file_id, &content_sha256)?;

        let run_id = RunsRepo::start_run(
            conn,
            StartRunSeed {
                file_id: request.file_id.clone(),
                mode: request.mode,
                content_sha256: content_sha256.clone(),
                file_size_bytes: file_size,
                extractor_version: self.extractor_version.clone(),
                retry_of_run_id: None,
            },
        )?;

        LifecycleRepo::transition(
            conn,
            &request.file_id,
            LifecycleState::Pending,
            LifecycleState::Ingesting,
        )?;

        request
            .file
            .seek(SeekFrom::Start(0))
            .map_err(IngestError::Io)?;
        let _discarded_proposals = self.extractor.extract(
            &request.file,
            &request.identity,
            request.source_type.clone(),
        );
        let claim_proposals = Vec::new();

        RunsRepo::complete_run(conn, &run_id, IngestionRunStatus::Success, 0, None)?;

        let lifecycle_state_after = if request.entity.is_some() {
            LifecycleState::Ingested
        } else {
            LifecycleState::PendingEntityAssignment
        };
        LifecycleRepo::transition(
            conn,
            &request.file_id,
            LifecycleState::Ingesting,
            lifecycle_state_after,
        )?;

        let entity_id_for_signal = request.entity.as_ref().map(|e| e.entity_id.0.as_str());
        match lifecycle_state_after {
            LifecycleState::Ingested => self.signal_emitter.emit_file_ingested(
                &request.file_id,
                &content_sha256,
                &run_id.0,
                entity_id_for_signal,
            ),
            LifecycleState::PendingEntityAssignment => self
                .signal_emitter
                .emit_file_pending_entity_assignment(&request.file_id, &run_id.0),
            _ => {}
        }

        let resolved_path =
            resolve_receipt_path(conn, &request, resolved_category.as_ref(), filename)?;

        Ok(IngestReceipt {
            ingestion_run_id: run_id,
            file_id: request.file_id,
            content_sha256,
            lifecycle_state_after,
            claim_proposals,
            resolved_category,
            resolved_path,
        })
    }

    pub fn validate_detected_category(
        conn: &Connection,
        candidate: Option<WorkspaceCategory>,
        entity_type: EntityType,
    ) -> Result<Option<WorkspaceCategory>, IngestError> {
        let Some(category) = candidate else {
            return Ok(None);
        };
        match WorkspaceCategoryRegistry::validate(conn, &category, entity_type) {
            Ok(()) => Ok(Some(category)),
            Err(_) => Ok(None),
        }
    }

    pub fn auto_detect_category_pure(
        filename: &str,
        content_head: &str,
    ) -> Option<WorkspaceCategory> {
        if let Some(category) = category_from_frontmatter(content_head) {
            return Some(category);
        }
        if content_head.trim_start().starts_with("---") && frontmatter_block(content_head).is_none()
        {
            return None;
        }

        let lower = filename.to_ascii_lowercase();
        if lower.contains("-transcript-") {
            return Some(WorkspaceCategory::Transcripts);
        }
        if lower.contains("-deck-")
            || lower.ends_with(".pptx")
            || lower.ends_with(".key")
            || lower.contains("-slides-")
        {
            return Some(WorkspaceCategory::Presentations);
        }
        if lower.contains("meeting") || lower.contains("-1on1-") {
            return Some(WorkspaceCategory::Meetings);
        }
        if lower.contains("contract") || lower.contains("msa") || lower.contains("sow") {
            return Some(WorkspaceCategory::Contracts);
        }
        if lower.ends_with(".pdf") {
            return Some(WorkspaceCategory::Attachments);
        }
        if lower.ends_with(".md") || lower.ends_with(".txt") {
            return Some(WorkspaceCategory::Notes);
        }
        None
    }

    fn reject_before_run(
        &self,
        conn: &Connection,
        file_id: &str,
        reason: RejectionReason,
        from: LifecycleState,
    ) -> Result<(), IngestError> {
        let _metadata = RejectionMetadata {
            limit_bytes: Some(self.max_file_bytes),
            found_bytes: None,
            detected_format: None,
        };
        LifecycleRepo::transition(conn, file_id, from, LifecycleState::Rejected)?;
        self.signal_emitter
            .emit_file_rejected(Some(file_id), reason.clone());
        Ok(())
    }
}

fn resolve_receipt_path(
    conn: &Connection,
    request: &IngestRequest,
    category: Option<&WorkspaceCategory>,
    filename: &str,
) -> Result<Option<String>, IngestError> {
    if let Some(entity) = request.entity.as_ref() {
        let entity_name = entity
            .entity_name
            .as_deref()
            .unwrap_or(entity.entity_id.0.as_str());
        let path = WorkspaceCategoryRegistry::resolve_path(
            conn,
            entity.entity_type,
            entity_name,
            category,
            filename,
            request.source_type.clone(),
        )?;
        return Ok(Some(path.to_string_lossy().to_string()));
    }

    let relative = request
        .identity
        .canonical_path
        .strip_prefix(PathBuf::from("."))
        .ok()
        .map(|p| p.to_string_lossy().to_string());
    Ok(relative.or_else(|| Some(filename.to_string())))
}

fn frontmatter_block(content_head: &str) -> Option<&str> {
    let trimmed = content_head
        .strip_prefix('\u{feff}')
        .unwrap_or(content_head);
    let rest = trimmed.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

fn category_from_frontmatter(content_head: &str) -> Option<WorkspaceCategory> {
    let block = frontmatter_block(content_head)?;
    for line in block.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim() != "doc_type" {
            continue;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if !is_valid_doc_type(value) {
            return None;
        }
        return match value {
            "transcript" => Some(WorkspaceCategory::Transcripts),
            "presentation" | "deck" | "slides" => Some(WorkspaceCategory::Presentations),
            "meeting" | "1on1" => Some(WorkspaceCategory::Meetings),
            "note" | "notes" => Some(WorkspaceCategory::Notes),
            "contract" | "msa" | "sow" => Some(WorkspaceCategory::Contracts),
            other => WorkspaceCategory::from_slug(other),
        };
    }
    None
}

fn is_valid_doc_type(value: &str) -> bool {
    !value.is_empty() && value.len() <= 32 && {
        let mut chars = value.chars();
        let first = chars.next().expect("non-empty checked above");
        first.is_ascii_lowercase()
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    }
}

pub fn file_id_from_identity(
    identity: &FileIdentity,
    workspace_root: &Path,
) -> Result<String, FileIdError> {
    let relative = identity
        .canonical_path
        .strip_prefix(workspace_root)
        .map_err(|_| FileIdError::OutsideWorkspace)?;
    let mut hasher = Sha256::new();
    hasher.update(relative.as_os_str().as_encoded_bytes());
    let hash = hasher.finalize();
    Ok(hex::encode(hash)[..16].to_string())
}

pub fn quarantine_source(
    conn: &Connection,
    file_id: &str,
    reason: &str,
    actor: QuarantineActor,
) -> Result<(), IngestError> {
    let actor_id = match actor {
        QuarantineActor::User { user_id } => user_id,
    };
    LifecycleRepo::record_user_override(conn, file_id, &actor_id)?;
    let lifecycle = LifecycleRepo::get(conn, file_id)?.ok_or(LifecycleError::FileNotFound)?;
    if lifecycle.lifecycle_state != LifecycleState::Quarantined {
        LifecycleRepo::transition(
            conn,
            file_id,
            lifecycle.lifecycle_state,
            LifecycleState::Quarantined,
        )?;
    }
    let _redacted_reason = reason;
    Ok(())
}

impl From<LifecycleError> for IngestError {
    fn from(value: LifecycleError) -> Self {
        IngestError::DbError(value.to_string())
    }
}

impl From<RunsError> for IngestError {
    fn from(value: RunsError) -> Self {
        match value {
            RunsError::AlreadyCompleted { existing } => IngestError::AlreadyProcessed {
                existing_run_id: existing.run_id,
            },
            RunsError::AlreadyInProgress { existing_run_id } => {
                IngestError::AlreadyProcessed { existing_run_id }
            }
            other => IngestError::DbError(other.to_string()),
        }
    }
}

impl From<ResolvePathError> for IngestError {
    fn from(value: ResolvePathError) -> Self {
        IngestError::DbError(value.to_string())
    }
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileIdMismatch { expected, found } => {
                write!(f, "file_id mismatch: expected {expected}, found {found}")
            }
            Self::Rejected(reason) => write!(f, "workspace file rejected: {reason:?}"),
            Self::AlreadyProcessed { existing_run_id } => {
                write!(f, "workspace file already processed: {}", existing_run_id.0)
            }
            Self::Io(error) => write!(
                f,
                "workspace ingestion I/O error: kind={:?} os={:?}",
                error.kind(),
                error.raw_os_error()
            ),
            Self::DbError(message) => write!(f, "workspace ingestion db error: {message}"),
        }
    }
}

impl std::error::Error for IngestError {}

//! Staged workspace ingestion pipeline.
//!
//! W2-A owns the shell: validated file handle in, lifecycle/run bookkeeping,
//! category sniffing, quarantine mutation, and zero claim production.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use abilities_runtime::abilities::provenance::source::{EntityId, WorkspaceFileKind};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::db::ActionDb;
use crate::entity::EntityType;
use crate::services::claims::{commit_claim, ClaimProposal};
use crate::services::context::ServiceContext;
use crate::signals::propagation::PropagationEngine;

use super::contracts::{
    DroppedFact, DroppedFactSource, ExtractionContext, ExtractionReport, Extractor, FileIdentity,
    RejectionReason, ResolvedLinkedSubject, SignalEmitContext, SignalEmitError, SignalEmitter,
    WorkspaceCategory, WorkspaceClaimProposal,
};
use super::lifecycle::{workspace_file_kind_slug, LifecycleError, LifecycleRepo, LifecycleState};
use super::link::{LinkAttributionSource, LinkError, LinkRepo};
use super::registry::{ResolvePathError, WorkspaceCategoryRegistry};
use super::runs::{
    IngestionMode, IngestionRunId, IngestionRunStatus, RunsError, RunsRepo, StartRunSeed,
};

pub const DEFAULT_MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
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
    pub invocation_actor: String,
    pub validated_content: Option<String>,
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
        ctx: &ServiceContext<'_>,
        db: &ActionDb,
        request: IngestRequest,
    ) -> Result<IngestReceipt, IngestError> {
        self.run_inner(ctx, db, None, request)
    }

    pub fn run_with_signal_engine(
        &self,
        ctx: &ServiceContext<'_>,
        db: &ActionDb,
        propagation: &PropagationEngine,
        request: IngestRequest,
    ) -> Result<IngestReceipt, IngestError> {
        self.run_inner(ctx, db, Some(propagation), request)
    }

    fn run_inner(
        &self,
        ctx: &ServiceContext<'_>,
        db: &ActionDb,
        propagation: Option<&PropagationEngine>,
        mut request: IngestRequest,
    ) -> Result<IngestReceipt, IngestError> {
        ctx.check_mutation_allowed()
            .map_err(|e| IngestError::DbError(format!("workspace ingestion blocked: {e}")))?;
        let conn = db.conn_ref();
        let signal_ctx = SignalEmitContext::new(ctx, db, propagation);
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

        let content = if let Some(content) = request.validated_content.take() {
            content
        } else {
            let file_size = request.file.metadata().map_err(IngestError::Io)?.len();
            if file_size > self.max_file_bytes {
                self.reject_before_run(
                    &signal_ctx,
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
            match String::from_utf8(bytes) {
                Ok(content) => content,
                Err(_) => {
                    self.reject_before_run(
                        &signal_ctx,
                        &request.file_id,
                        RejectionReason::UnsupportedFormat,
                        LifecycleState::Pending,
                    )?;
                    return Err(IngestError::Rejected(RejectionReason::UnsupportedFormat));
                }
            }
        };
        let file_size = content.len() as u64;
        if file_size > self.max_file_bytes {
            self.reject_before_run(
                &signal_ctx,
                &request.file_id,
                RejectionReason::FileTooLarge,
                LifecycleState::Pending,
            )?;
            return Err(IngestError::Rejected(RejectionReason::FileTooLarge));
        }
        if content.as_bytes().contains(&0) {
            self.reject_before_run(
                &signal_ctx,
                &request.file_id,
                RejectionReason::UnsupportedFormat,
                LifecycleState::Pending,
            )?;
            return Err(IngestError::Rejected(RejectionReason::UnsupportedFormat));
        }

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let content_sha256 = hex::encode(hasher.finalize());

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

        let observed_at = ctx.clock.now();
        let mut subject_drops = Vec::new();
        let linked_subject = resolve_linked_subject(conn, &request, &mut subject_drops)?;

        request
            .file
            .seek(SeekFrom::Start(0))
            .map_err(IngestError::Io)?;
        let extraction_context = ExtractionContext {
            file_id: &request.file_id,
            identity: &request.identity,
            content: &content,
            source_type: request.source_type.clone(),
            source_asof: request.source_asof,
            resolved_category: resolved_category.as_ref(),
            linked_subject: linked_subject.as_ref(),
            ingestion_run_id: &run_id.0,
            observed_at,
            invocation_actor: &request.invocation_actor,
        };
        let mut extraction_report = match self
            .extractor
            .extract(&mut request.file, &extraction_context)
        {
            Ok(report) => report,
            Err(error) => {
                let error_log = failed_extraction_log(&error);
                RunsRepo::complete_run(
                    conn,
                    &run_id,
                    IngestionRunStatus::Failed,
                    0,
                    Some(error_log),
                )?;
                LifecycleRepo::transition(
                    conn,
                    &request.file_id,
                    LifecycleState::Ingesting,
                    LifecycleState::Rejected,
                )?;
                return Err(IngestError::DbError(error.to_string()));
            }
        };
        extraction_report.dropped_facts.extend(subject_drops);

        let mut commit_ready = Vec::with_capacity(extraction_report.proposals.len());
        for proposal in &extraction_report.proposals {
            match claim_proposal_from_workspace(
                proposal,
                resolved_category.as_ref(),
                &request.invocation_actor,
            ) {
                Ok(claim) => commit_ready.push(claim),
                Err(error) => {
                    let commit_errors = vec![CommitErrorReport {
                        claim_type: proposal.claim_type.as_str().to_string(),
                        reason: error.clone(),
                    }];
                    let error_log = run_error_log(&extraction_report, 0, &commit_errors, false);
                    RunsRepo::complete_run(
                        conn,
                        &run_id,
                        IngestionRunStatus::Failed,
                        0,
                        Some(error_log),
                    )?;
                    LifecycleRepo::transition(
                        conn,
                        &request.file_id,
                        LifecycleState::Ingesting,
                        LifecycleState::Rejected,
                    )?;
                    return Err(IngestError::DbError(error));
                }
            }
        }

        let lifecycle_state_after = if linked_subject.is_some() {
            LifecycleState::Ingested
        } else {
            LifecycleState::PendingEntityAssignment
        };

        let finalization_result = db.with_transaction(|tx_db| {
            let tx_conn = tx_db.conn_ref();
            let mut committed_count = 0_u64;
            for proposal in commit_ready {
                let claim_type = proposal.claim_type.clone();
                if let Err(error) = commit_claim(ctx, tx_db, proposal) {
                    return Err(format!("claim commit failed for {claim_type}: {error}"));
                }
                committed_count += 1;
            }

            let error_log = if extraction_report.dropped_facts.is_empty()
                && extraction_report.warnings.is_empty()
            {
                None
            } else {
                Some(run_error_log(
                    &extraction_report,
                    committed_count,
                    &[],
                    false,
                ))
            };

            RunsRepo::complete_run(
                tx_conn,
                &run_id,
                IngestionRunStatus::Success,
                committed_count,
                error_log,
            )
            .map_err(|e| e.to_string())?;

            LifecycleRepo::transition(
                tx_conn,
                &request.file_id,
                LifecycleState::Ingesting,
                lifecycle_state_after,
            )
            .map_err(|e| e.to_string())?;

            let tx_signal_ctx = SignalEmitContext::new(ctx, tx_db, propagation);
            let entity_type_for_signal = linked_subject.as_ref().map(|e| e.subject_kind_slug());
            let entity_id_for_signal = linked_subject.as_ref().map(|e| e.entity_id.as_str());
            match lifecycle_state_after {
                LifecycleState::Ingested => self
                    .signal_emitter
                    .emit_file_ingested(
                        &tx_signal_ctx,
                        &request.file_id,
                        &run_id.0,
                        entity_type_for_signal
                            .ok_or(SignalEmitError::MissingEntityTarget(
                                "workspace_file_ingested",
                            ))
                            .map_err(|e| e.to_string())?,
                        entity_id_for_signal
                            .ok_or(SignalEmitError::MissingEntityTarget(
                                "workspace_file_ingested",
                            ))
                            .map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?,
                LifecycleState::PendingEntityAssignment => self
                    .signal_emitter
                    .emit_file_pending_entity_assignment(
                        &tx_signal_ctx,
                        &request.file_id,
                        &run_id.0,
                    )
                    .map_err(|e| e.to_string())?,
                _ => {}
            }
            Ok(committed_count)
        });
        if let Err(message) = finalization_result {
            if let Err(cleanup) = fail_in_progress_run_after_finalization_error(
                conn,
                &run_id,
                &request.file_id,
                &message,
            ) {
                log::warn!(
                    "workspace ingestion signal finalization cleanup failed for {}: {cleanup}",
                    request.file_id
                );
            }
            return Err(IngestError::DbError(message));
        }

        let resolved_path =
            resolve_receipt_path(conn, &request, resolved_category.as_ref(), filename)?;

        Ok(IngestReceipt {
            ingestion_run_id: run_id,
            file_id: request.file_id,
            content_sha256,
            lifecycle_state_after,
            claim_proposals: extraction_report.proposals,
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
        // Priority 1 — frontmatter doc_type. Per packet §3 frozen detection
        // table, priority 1 is terminal whenever it fires: a present `doc_type`
        // key (even if invalid or unregistered) resolves the lane at this
        // priority and lower priorities do not run. Only when frontmatter is
        // absent / malformed / has no `doc_type` key do we fall through.
        if let Some(result) = frontmatter_doctype_detection(content_head) {
            return result;
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
        signal_ctx: &SignalEmitContext<'_, '_>,
        file_id: &str,
        reason: RejectionReason,
        from: LifecycleState,
    ) -> Result<(), IngestError> {
        let _metadata = RejectionMetadata {
            limit_bytes: Some(self.max_file_bytes),
            found_bytes: None,
            detected_format: None,
        };
        signal_ctx
            .db
            .with_transaction(|tx_db| {
                LifecycleRepo::transition(
                    tx_db.conn_ref(),
                    file_id,
                    from,
                    LifecycleState::Rejected,
                )
                .map_err(|e| e.to_string())?;
                let tx_signal_ctx =
                    SignalEmitContext::new(signal_ctx.services, tx_db, signal_ctx.propagation);
                self.signal_emitter
                    .emit_file_rejected(&tx_signal_ctx, Some(file_id), reason.clone())
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
            .map_err(IngestError::DbError)
    }
}

#[derive(Debug, Clone)]
struct CommitErrorReport {
    claim_type: String,
    reason: String,
}

fn resolve_linked_subject(
    conn: &Connection,
    request: &IngestRequest,
    dropped_facts: &mut Vec<DroppedFact>,
) -> Result<Option<ResolvedLinkedSubject>, IngestError> {
    if let Some(entity) = request.entity.as_ref() {
        return resolve_entity_seeded_subject(conn, request, entity, dropped_facts);
    }

    let active_links = LinkRepo::list_links_for_file(conn, &request.file_id, false)
        .map_err(link_error_to_ingest)?;
    match active_links.as_slice() {
        [] => Ok(None),
        [link] => subject_from_link(conn, link, dropped_facts),
        _ => {
            dropped_facts.push(DroppedFact::new(
                "ambiguous_active_links",
                DroppedFactSource::Structured,
            ));
            Ok(None)
        }
    }
}

fn resolve_entity_seeded_subject(
    conn: &Connection,
    request: &IngestRequest,
    entity: &EntityRef,
    dropped_facts: &mut Vec<DroppedFact>,
) -> Result<Option<ResolvedLinkedSubject>, IngestError> {
    if entity.entity_type == EntityType::Other {
        dropped_facts.push(DroppedFact::new(
            "unsupported_subject_kind",
            DroppedFactSource::Structured,
        ));
        return Ok(None);
    }

    let Some(canonical_name) =
        canonical_entity_name(conn, entity.entity_type, &entity.entity_id.0)?
    else {
        dropped_facts.push(DroppedFact::new(
            "entity_not_found",
            DroppedFactSource::Structured,
        ));
        return Ok(None);
    };

    if let Some(provided_name) = entity.entity_name.as_deref().map(str::trim) {
        if !provided_name.is_empty() && !entity_name_matches(provided_name, &canonical_name) {
            dropped_facts.push(
                DroppedFact::new("entity_name_mismatch", DroppedFactSource::Structured)
                    .with_field("entity_name"),
            );
            return Ok(None);
        }
    }

    let all_links = LinkRepo::list_links_for_file(conn, &request.file_id, true)
        .map_err(link_error_to_ingest)?;
    if all_links.iter().any(|link| {
        link.rejected
            && link.entity_type == entity.entity_type
            && link.entity_id == entity.entity_id.0
    }) {
        dropped_facts.push(DroppedFact::new(
            "rejected_link",
            DroppedFactSource::Structured,
        ));
        return Ok(None);
    }

    let active_links = all_links
        .iter()
        .filter(|link| !link.rejected)
        .collect::<Vec<_>>();
    if active_links
        .iter()
        .any(|link| link.entity_type != entity.entity_type || link.entity_id != entity.entity_id.0)
    {
        dropped_facts.push(DroppedFact::new(
            "active_link_mismatch",
            DroppedFactSource::Structured,
        ));
        return Ok(None);
    }

    let attribution_source = link_attribution_source_for_kind(&request.source_type);
    let link_id = if let Some(existing) = active_links.first() {
        existing.link_id.0.clone()
    } else {
        LinkRepo::add_link(
            conn,
            &request.file_id,
            entity.entity_type,
            &entity.entity_id.0,
            attribution_source,
            1.0,
            Some(link_rationale(attribution_source)),
            "system:workspace_ingestion",
        )
        .map_err(link_error_to_ingest)?
        .0
    };

    Ok(Some(ResolvedLinkedSubject {
        entity_type: entity.entity_type,
        entity_id: entity.entity_id.0.clone(),
        entity_name: Some(canonical_name),
        link_id,
    }))
}

fn link_attribution_source_for_kind(source_type: &WorkspaceFileKind) -> LinkAttributionSource {
    if matches!(source_type, WorkspaceFileKind::McpPlacement) {
        LinkAttributionSource::McpPlacement
    } else {
        LinkAttributionSource::EntityIntake
    }
}

fn link_rationale(source: LinkAttributionSource) -> &'static str {
    match source {
        LinkAttributionSource::McpPlacement => "MCP workspace placement",
        _ => "entity-seeded workspace intake",
    }
}

fn subject_from_link(
    conn: &Connection,
    link: &super::link::DocumentEntityLink,
    dropped_facts: &mut Vec<DroppedFact>,
) -> Result<Option<ResolvedLinkedSubject>, IngestError> {
    if link.entity_type == EntityType::Other {
        dropped_facts.push(DroppedFact::new(
            "unsupported_subject_kind",
            DroppedFactSource::Structured,
        ));
        return Ok(None);
    }
    let Some(canonical_name) = canonical_entity_name(conn, link.entity_type, &link.entity_id)?
    else {
        dropped_facts.push(DroppedFact::new(
            "entity_not_found",
            DroppedFactSource::Structured,
        ));
        return Ok(None);
    };

    Ok(Some(ResolvedLinkedSubject {
        entity_type: link.entity_type,
        entity_id: link.entity_id.clone(),
        entity_name: Some(canonical_name),
        link_id: link.link_id.0.clone(),
    }))
}

fn canonical_entity_name(
    conn: &Connection,
    entity_type: EntityType,
    entity_id: &str,
) -> Result<Option<String>, IngestError> {
    let sql = match entity_type {
        EntityType::Account => "SELECT name FROM accounts WHERE id = ?1 AND archived = 0",
        EntityType::Project => "SELECT name FROM projects WHERE id = ?1 AND archived = 0",
        EntityType::Person => {
            "SELECT CASE WHEN trim(name) = '' THEN email ELSE name END FROM people WHERE id = ?1 AND archived = 0"
        }
        EntityType::Other => return Ok(None),
    };
    conn.query_row(sql, params![entity_id], |row| row.get::<_, String>(0))
        .optional()
        .map_err(|e| IngestError::DbError(e.to_string()))
}

fn entity_name_matches(provided_name: &str, canonical_name: &str) -> bool {
    provided_name == canonical_name || provided_name == crate::util::slugify(canonical_name)
}

fn link_error_to_ingest(error: LinkError) -> IngestError {
    IngestError::DbError(error.to_string())
}

fn claim_proposal_from_workspace(
    proposal: &WorkspaceClaimProposal,
    resolved_category: Option<&WorkspaceCategory>,
    original_ability_actor: &str,
) -> Result<ClaimProposal, String> {
    if !proposal.subject.is_claim_supported() {
        return Err("unsupported subject kind for workspace claim".to_string());
    }
    if !proposal.source_ref.starts_with("workspace_file:")
        || proposal.source_ref.contains('/')
        || proposal.source_ref.contains('\\')
    {
        return Err("workspace source_ref must be an opaque workspace_file id".to_string());
    }

    let subject_ref = json!({
        "kind": proposal.subject.subject_kind_slug(),
        "id": proposal.subject.entity_id.as_str(),
    })
    .to_string();
    let workspace_file_kind = match &proposal.data_source {
        super::contracts::DataSource::WorkspaceFile { kind } => workspace_file_kind_slug(kind),
        _ => return Err("workspace proposal carried non-workspace data source".to_string()),
    };
    let provenance_json = serde_json::to_string(&proposal.source_attribution)
        .map_err(|e| format!("serialize source attribution: {e}"))?;
    let metadata_json = json!({
        "producer": "workspace_ingestion",
        "ingestion_run_id": proposal.ingestion_run_id.as_str(),
        "workspace_file_id": proposal.source_ref.trim_start_matches("workspace_file:"),
        "workspace_file_kind": workspace_file_kind,
        "resolved_category": resolved_category.map(WorkspaceCategory::as_slug),
        "original_ability_actor": original_ability_actor,
        "sensitivity_floor": "user_only",
        "document_entity_link_id": proposal.subject.link_id.as_str(),
        "schema_version": 1
    })
    .to_string();

    Ok(ClaimProposal {
        id: None,
        expected_claim_version: None,
        subject_ref,
        claim_type: proposal.claim_type.as_str().to_string(),
        field_path: proposal.field_path.clone(),
        topic_key: proposal.topic_key.clone(),
        text: proposal.text.clone(),
        actor: "system:workspace_ingestion".to_string(),
        data_source: format!("workspace_file:{workspace_file_kind}"),
        source_ref: Some(proposal.source_ref.clone()),
        source_asof: Some(proposal.source_asof.to_rfc3339()),
        observed_at: proposal.observed_at.to_rfc3339(),
        provenance_json,
        metadata_json: Some(metadata_json),
        thread_id: None,
        temporal_scope: None,
        sensitivity: Some(proposal.sensitivity.clone()),
        supersedes: None,
        tombstone: None,
    })
}

fn failed_extraction_log(error: &super::contracts::ExtractionError) -> serde_json::Value {
    json!({
        "schema_version": 1,
        "producer": "workspace_ingestion",
        "proposal_count": 0,
        "committed_count": 0,
        "dropped_count": 0,
        "partial_commit": false,
        "dropped_facts": [],
        "warnings": [],
        "commit_errors": [],
        "extraction_error": error.to_string()
    })
}

fn run_error_log(
    report: &ExtractionReport,
    committed_count: u64,
    commit_errors: &[CommitErrorReport],
    partial_commit: bool,
) -> serde_json::Value {
    let commit_errors = commit_errors
        .iter()
        .map(|error| {
            json!({
                "claim_type": error.claim_type,
                "reason": error.reason,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "schema_version": 1,
        "producer": "workspace_ingestion",
        "proposal_count": report.proposals.len(),
        "committed_count": committed_count,
        "dropped_count": report.dropped_facts.len(),
        "partial_commit": partial_commit,
        "dropped_facts": &report.dropped_facts,
        "warnings": &report.warnings,
        "commit_errors": commit_errors
    })
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
    let rest = trimmed
        .strip_prefix("---\n")
        .or_else(|| trimmed.strip_prefix("---\r\n"))?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

/// Outer `Option` distinguishes "priority 1 fired" from "priority 1 didn't
/// fire":
/// - `None` → no frontmatter block, or frontmatter has no `doc_type` key →
///   priority 1 did NOT fire, caller should fall through.
/// - `Some(Some(cat))` → `doc_type` present, shape-valid, mapped to a known
///   variant OR a shape-valid `Other(slug)` → terminal Some. Registry
///   validation of `Other(slug)` happens later at `validate_detected_category`.
/// - `Some(None)` → `doc_type` present but FAILS the shape check
///   (`^[a-z][a-z0-9_-]{0,31}$`) → priority 1 FIRED with no match → terminal
///   None per packet §3 frozen detection table row 1.
///
/// This split closes the L2 cycle-1 codex BLOCK on §7: a shape-invalid
/// `doc_type` (user-supplied invalid intent) must not allow filename-glob
/// fallback to override.
fn frontmatter_doctype_detection(content_head: &str) -> Option<Option<WorkspaceCategory>> {
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
            return Some(None);
        }
        let category = match value {
            "transcript" => Some(WorkspaceCategory::Transcripts),
            "presentation" | "deck" | "slides" => Some(WorkspaceCategory::Presentations),
            "meeting" | "1on1" => Some(WorkspaceCategory::Meetings),
            "note" | "notes" => Some(WorkspaceCategory::Notes),
            "contract" | "msa" | "sow" => Some(WorkspaceCategory::Contracts),
            other => WorkspaceCategory::from_slug(other),
        };
        return Some(category);
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

fn fail_in_progress_run_after_finalization_error(
    conn: &Connection,
    run_id: &IngestionRunId,
    file_id: &str,
    message: &str,
) -> Result<(), IngestError> {
    let error_log = serde_json::json!({
        "error": "workspace_ingestion_finalization_failed",
        "message": message,
    });
    RunsRepo::complete_run(conn, run_id, IngestionRunStatus::Failed, 0, Some(error_log))?;
    if let Some(row) = LifecycleRepo::get(conn, file_id)? {
        if row.lifecycle_state == LifecycleState::Ingesting {
            LifecycleRepo::transition(
                conn,
                file_id,
                LifecycleState::Ingesting,
                LifecycleState::Rejected,
            )?;
        }
    }
    Ok(())
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
    signal_ctx: &SignalEmitContext<'_, '_>,
    emitter: &dyn SignalEmitter,
    file_id: &str,
    reason: &str,
    actor: QuarantineActor,
) -> Result<(), IngestError> {
    let actor_id = actor_id_for_quarantine(actor);
    signal_ctx
        .db
        .with_transaction(|tx_db| {
            let tx_signal_ctx =
                SignalEmitContext::new(signal_ctx.services, tx_db, signal_ctx.propagation);
            let lifecycle = transition_source_to_quarantined(tx_db.conn_ref(), file_id, &actor_id)
                .map_err(|e| e.to_string())?;
            let (entity_type, entity_id) =
                quarantine_signal_target(tx_db.conn_ref(), &lifecycle, file_id)
                    .map_err(|e| e.to_string())?;
            emitter
                .emit_file_quarantined(
                    &tx_signal_ctx,
                    file_id,
                    reason,
                    &actor_id,
                    entity_type.as_deref(),
                    entity_id.as_deref(),
                )
                .map_err(|e| e.to_string())?;
            Ok(())
        })
        .map_err(IngestError::DbError)
}

fn actor_id_for_quarantine(actor: QuarantineActor) -> String {
    match actor {
        QuarantineActor::User { user_id } => user_id,
    }
}

fn transition_source_to_quarantined(
    conn: &Connection,
    file_id: &str,
    actor_id: &str,
) -> Result<super::lifecycle::WorkspaceFileLifecycle, IngestError> {
    LifecycleRepo::record_user_override(conn, file_id, actor_id)?;
    let mut lifecycle = LifecycleRepo::get(conn, file_id)?.ok_or(LifecycleError::FileNotFound)?;
    if lifecycle.lifecycle_state != LifecycleState::Quarantined {
        LifecycleRepo::transition(
            conn,
            file_id,
            lifecycle.lifecycle_state,
            LifecycleState::Quarantined,
        )?;
        lifecycle = LifecycleRepo::get(conn, file_id)?.ok_or(LifecycleError::FileNotFound)?;
    }
    Ok(lifecycle)
}

fn quarantine_signal_target(
    conn: &Connection,
    lifecycle: &super::lifecycle::WorkspaceFileLifecycle,
    file_id: &str,
) -> Result<(Option<String>, Option<String>), IngestError> {
    if let (Some(entity_type), Some(entity_id)) =
        (lifecycle.entity_type.as_ref(), lifecycle.entity_id.as_ref())
    {
        return Ok((Some(entity_type.clone()), Some(entity_id.clone())));
    }

    let active_links =
        LinkRepo::list_links_for_file(conn, file_id, false).map_err(link_error_to_ingest)?;
    let [link] = active_links.as_slice() else {
        return Ok((None, None));
    };
    if link.entity_type == EntityType::Other {
        return Ok((None, None));
    }
    Ok((
        Some(link.entity_type.as_str().to_string()),
        Some(link.entity_id.clone()),
    ))
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

impl From<SignalEmitError> for IngestError {
    fn from(value: SignalEmitError) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_placement_uses_dedicated_link_attribution() {
        assert_eq!(
            link_attribution_source_for_kind(&WorkspaceFileKind::McpPlacement),
            LinkAttributionSource::McpPlacement
        );
        assert_eq!(
            link_attribution_source_for_kind(&WorkspaceFileKind::EntityDoc),
            LinkAttributionSource::EntityIntake
        );
    }
}

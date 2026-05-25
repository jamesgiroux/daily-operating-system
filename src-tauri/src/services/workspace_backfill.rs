//! Workspace source backfill service for v1.4.5 W5-A.
//!
//! Registers existing workspace files as conservative pending-review sources.
//! This service does not run the ingestion pipeline and does not create claims.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use abilities_runtime::abilities::provenance::source::{EntityId, WorkspaceFileKind};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::db::{ActionDb, LocalKeychain};
use crate::entity::EntityType;
use crate::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use crate::services::workspace_ingestion::contracts::{
    FileIdentity, SignalEmitContext, SignalEmitter, WorkspaceCategory,
};
use crate::services::workspace_ingestion::lifecycle::{
    workspace_file_kind_slug, LifecycleRepo, LifecycleState,
};
use crate::services::workspace_ingestion::link::{LinkAttributionSource, LinkError, LinkRepo};
use crate::services::workspace_ingestion::pipeline::{
    file_id_from_identity, EntityRef, DEFAULT_MAX_FILE_BYTES,
};
use crate::services::workspace_ingestion::registry::{
    WorkspaceCategoryRegistry, WorkspaceSourceRegistry,
};
use crate::services::workspace_ingestion::signals::WorkspaceSignalEmitter;
use crate::signals::propagation::{default_engine, PropagationEngine};

type HmacSha256 = Hmac<Sha256>;

const ACTOR: &str = "system:workspace_backfill:v1";
const EXPOSURE_PENDING_REVIEW: &str = "pending_review";
const SOURCE_TIME_BASIS: &str = "filesystem_mtime";
const SOURCE_TIME_CONFIDENCE: &str = "filesystem_unverified";
const MAX_SUMMARY_HANDLES: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackfillMode {
    DryRun,
    Apply,
}

impl BackfillMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry_run",
            Self::Apply => "apply",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceBackfillOptions {
    pub workspace_root: Option<PathBuf>,
    pub mode: BackfillMode,
    pub resume_run_id: Option<String>,
    pub max_file_bytes: u64,
}

impl WorkspaceBackfillOptions {
    pub fn dry_run(workspace_root: Option<PathBuf>) -> Self {
        Self {
            workspace_root,
            mode: BackfillMode::DryRun,
            resume_run_id: None,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }

    pub fn apply(workspace_root: Option<PathBuf>, resume_run_id: Option<String>) -> Self {
        Self {
            workspace_root,
            mode: BackfillMode::Apply,
            resume_run_id,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateContentGroupSummary {
    pub duplicate_group_handle: String,
    pub source_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillSummary {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub status: String,
    pub scanned_count: u64,
    pub eligible_count: u64,
    pub applied_count: u64,
    pub skipped_count: u64,
    pub failed_count: u64,
    pub reason_counts: BTreeMap<String, u64>,
    pub source_class_counts: BTreeMap<String, u64>,
    pub divergence_counts: BTreeMap<String, u64>,
    pub duplicate_groups: Vec<DuplicateContentGroupSummary>,
    pub source_handles: Vec<String>,
    pub source_handle_count: u64,
    pub truncated_source_handles: bool,
}

#[derive(Debug, Clone)]
pub struct BackfillHandleKey {
    bytes: [u8; 32],
}

impl BackfillHandleKey {
    pub fn local() -> Result<Self, String> {
        let bytes = crate::db::local_db_workspace_graph_diagnostic_key_bytes()
            .map_err(|_| "handle_key_unavailable".to_string())?;
        Ok(Self { bytes })
    }

    #[cfg(test)]
    pub(crate) fn for_tests(label: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"dailyos.workspace_backfill.test_handle_key.v1");
        hasher.update([0]);
        hasher.update(label.as_bytes());
        Self {
            bytes: hasher.finalize().into(),
        }
    }

    fn handle(&self, prefix: &str, domain: &str, components: &[&str]) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.bytes).expect("HMAC key length is valid");
        mac.update(domain.as_bytes());
        for component in components {
            mac.update(&[0]);
            mac.update(component.as_bytes());
        }
        let bytes = mac.finalize().into_bytes();
        format!("{prefix}:v1:{}", hex::encode(&bytes[..16]))
    }
}

pub struct WorkspaceBackfillSignalRuntime<'a, 'svc> {
    pub services: &'a ServiceContext<'svc>,
    pub propagation: &'a PropagationEngine,
    pub emitter: &'a dyn SignalEmitter,
}

#[derive(Debug, Clone)]
struct EntityPath {
    entity_type: EntityType,
    entity_id: String,
    entity_name: String,
    canonical_path: PathBuf,
    relative_path: PathBuf,
}

#[derive(Debug, Clone)]
struct Candidate {
    relative_path: PathBuf,
    identity: FileIdentity,
    file_id: String,
    source_handle: String,
    item_handle: String,
    duplicate_group_handle: String,
    link_handle: Option<String>,
    source_type: WorkspaceFileKind,
    entity_type: Option<EntityType>,
    entity_id: Option<String>,
    entity_name: Option<String>,
    category: Option<WorkspaceCategory>,
    source_asof: DateTime<Utc>,
    observed_at: DateTime<Utc>,
    content_sha256: String,
}

impl Candidate {
    fn candidate_kind(&self) -> &'static str {
        workspace_file_kind_slug(&self.source_type)
    }

    fn entity_ref(&self) -> Option<EntityRef> {
        Some(EntityRef {
            entity_type: self.entity_type?,
            entity_id: EntityId::new(self.entity_id.clone()?),
            entity_name: self.entity_name.clone(),
        })
    }
}

#[derive(Debug)]
struct ScanOutput {
    scanned_count: u64,
    candidates: Vec<Candidate>,
    reason_counts: BTreeMap<String, u64>,
    source_class_counts: BTreeMap<String, u64>,
    duplicate_groups: Vec<DuplicateContentGroupSummary>,
}

#[derive(Debug, Default)]
struct ApplyOutcome {
    applied: bool,
    skipped_reason: Option<String>,
}

pub fn run_workspace_backfill_from_local_db(
    options: WorkspaceBackfillOptions,
) -> Result<BackfillSummary, String> {
    let key = BackfillHandleKey::local()?;
    if options.mode == BackfillMode::DryRun {
        let db = ActionDb::open_readonly(Arc::new(LocalKeychain::new()))
            .map_err(|_| "database_unavailable".to_string())?;
        return run_workspace_backfill(&db, options, &key, None);
    }

    let db = ActionDb::open(Arc::new(LocalKeychain::new()))
        .map_err(|_| "database_unavailable".to_string())?;
    let clock = SystemClock;
    let rng = SystemRng;
    let external = ExternalClients::default();
    let services = ServiceContext::new_live(&clock, &rng, &external).with_actor(ACTOR);
    let propagation = default_engine();
    let emitter = WorkspaceSignalEmitter;
    let runtime = WorkspaceBackfillSignalRuntime {
        services: &services,
        propagation: &propagation,
        emitter: &emitter,
    };
    run_workspace_backfill(&db, options, &key, Some(runtime))
}

pub fn run_workspace_backfill(
    db: &ActionDb,
    options: WorkspaceBackfillOptions,
    key: &BackfillHandleKey,
    signal_runtime: Option<WorkspaceBackfillSignalRuntime<'_, '_>>,
) -> Result<BackfillSummary, String> {
    let workspace_root = resolve_workspace_root(options.workspace_root.as_deref())?;
    let root_fingerprint = key.handle(
        "root",
        "dailyos.workspace_backfill.workspace_root_fingerprint.v1",
        &[&workspace_root.to_string_lossy()],
    );
    let run_id = match options.mode {
        BackfillMode::DryRun => uuid::Uuid::new_v4().to_string(),
        BackfillMode::Apply => options
            .resume_run_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
    };

    let scan = scan_workspace(
        db.conn_ref(),
        &workspace_root,
        &run_id,
        &root_fingerprint,
        key,
        options.max_file_bytes,
    )?;

    let mut reason_counts = scan.reason_counts.clone();
    let mut divergence_counts = divergence_counts(db.conn_ref(), &workspace_root);
    let filesystem_without_lifecycle = scan
        .candidates
        .iter()
        .filter(|candidate| {
            LifecycleRepo::get(db.conn_ref(), &candidate.file_id)
                .ok()
                .flatten()
                .is_none()
        })
        .count() as u64;
    if filesystem_without_lifecycle > 0 {
        divergence_counts.insert(
            "filesystem_candidate_without_lifecycle".to_string(),
            filesystem_without_lifecycle,
        );
    }
    if !scan.duplicate_groups.is_empty() {
        divergence_counts.insert(
            "duplicate_content_hash".to_string(),
            scan.duplicate_groups.len() as u64,
        );
    }

    let mut applied_count = 0_u64;
    let mut skipped_count = 0_u64;
    let mut failed_count = 0_u64;

    if options.mode == BackfillMode::Apply {
        let runtime = signal_runtime.ok_or_else(|| "signal_runtime_unavailable".to_string())?;
        create_or_resume_run(
            db.conn_ref(),
            &run_id,
            options.mode,
            &root_fingerprint,
            &reason_counts,
            &scan.source_class_counts,
            &divergence_counts,
        )?;
        for candidate in &scan.candidates {
            match apply_candidate(db, &workspace_root, &run_id, candidate, &runtime) {
                Ok(outcome) => {
                    if outcome.applied {
                        applied_count += 1;
                    } else {
                        skipped_count += 1;
                        if let Some(reason) = outcome.skipped_reason {
                            increment(&mut reason_counts, &reason);
                        }
                    }
                }
                Err(reason) => {
                    failed_count += 1;
                    increment(&mut reason_counts, &reason);
                    record_failed_item(db, &run_id, candidate, &reason)?;
                }
            }
        }
        complete_run(
            db.conn_ref(),
            &run_id,
            if failed_count > 0 {
                "failed"
            } else {
                "completed"
            },
            &reason_counts,
            &scan.source_class_counts,
            &divergence_counts,
        )?;
    }

    let source_handles = scan
        .candidates
        .iter()
        .map(|candidate| candidate.source_handle.clone())
        .take(MAX_SUMMARY_HANDLES)
        .collect::<Vec<_>>();
    Ok(BackfillSummary {
        mode: options.mode.as_str().to_string(),
        run_id: (options.mode == BackfillMode::Apply).then_some(run_id),
        status: if failed_count > 0 {
            "failed"
        } else {
            "completed"
        }
        .to_string(),
        scanned_count: scan.scanned_count,
        eligible_count: scan.candidates.len() as u64,
        applied_count,
        skipped_count,
        failed_count,
        reason_counts,
        source_class_counts: scan.source_class_counts,
        divergence_counts,
        duplicate_groups: scan.duplicate_groups,
        source_handle_count: scan.candidates.len() as u64,
        truncated_source_handles: scan.candidates.len() > MAX_SUMMARY_HANDLES,
        source_handles,
    })
}

fn resolve_workspace_root(explicit: Option<&Path>) -> Result<PathBuf, String> {
    let root = match explicit {
        Some(root) => root.to_path_buf(),
        None => {
            let config_path = crate::state::config_path()
                .map_err(|_| "workspace_root_unavailable".to_string())?;
            let content = std::fs::read_to_string(config_path)
                .map_err(|_| "workspace_root_unavailable".to_string())?;
            let mut config: crate::types::Config = serde_json::from_str(&content)
                .map_err(|_| "workspace_root_unavailable".to_string())?;
            config.normalize();
            PathBuf::from(config.workspace_path)
        }
    };
    root.canonicalize()
        .map_err(|_| "workspace_root_unavailable".to_string())
}

fn scan_workspace(
    conn: &Connection,
    workspace_root: &Path,
    run_id: &str,
    root_fingerprint: &str,
    key: &BackfillHandleKey,
    max_file_bytes: u64,
) -> Result<ScanOutput, String> {
    let entities = load_entity_paths(conn, workspace_root)?;
    let mut scanned_count = 0_u64;
    let mut candidates = Vec::new();
    let mut reason_counts = BTreeMap::new();
    let mut source_class_counts = BTreeMap::new();

    for entry in WalkDir::new(workspace_root).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                increment(&mut reason_counts, "walk_error");
                continue;
            }
        };
        if entry.depth() == 0 {
            continue;
        }
        if entry.file_type().is_dir() {
            continue;
        }
        scanned_count += 1;
        if !entry.file_type().is_file() {
            increment(&mut reason_counts, "non_regular_file");
            continue;
        }
        let relative_path = match entry.path().strip_prefix(workspace_root) {
            Ok(path) => path.to_path_buf(),
            Err(_) => {
                increment(&mut reason_counts, "outside_workspace");
                continue;
            }
        };
        if let Some(reason) = eligibility_skip_reason(&relative_path) {
            increment(&mut reason_counts, reason);
            continue;
        }
        match candidate_from_relative_path(
            conn,
            workspace_root,
            &relative_path,
            run_id,
            root_fingerprint,
            key,
            max_file_bytes,
            &entities,
        ) {
            Ok(candidate) => {
                increment(&mut source_class_counts, candidate.candidate_kind());
                candidates.push(candidate);
            }
            Err(reason) => increment(&mut reason_counts, &reason),
        }
    }

    candidates.sort_by(|a, b| a.source_handle.cmp(&b.source_handle));
    let duplicate_groups = duplicate_groups(&candidates);

    Ok(ScanOutput {
        scanned_count,
        candidates,
        reason_counts,
        source_class_counts,
        duplicate_groups,
    })
}

fn eligibility_skip_reason(relative_path: &Path) -> Option<&'static str> {
    let components = normal_components(relative_path);
    if components.is_empty() {
        return Some("path_unavailable");
    }
    if components
        .iter()
        .any(|component| component.starts_with('.'))
    {
        return Some("hidden_path");
    }
    let root = components.first()?.as_str();
    if root == "Internal" {
        return Some("internal_path");
    }
    if root.starts_with('_') && root != "_inbox" {
        return Some("managed_root");
    }
    let filename = components.last()?.as_str();
    if filename == ".DS_Store" {
        return Some("generated_file");
    }
    if components.len() == 1 && filename == "CLAUDE.md" {
        return Some("managed_file");
    }
    if matches!(
        filename,
        "dashboard.json" | "dashboard.md" | "intelligence.json"
    ) {
        return Some("generated_file");
    }
    if !supported_extension(filename) {
        return Some("unsupported_format");
    }
    None
}

fn supported_extension(filename: &str) -> bool {
    let Some((_, ext)) = filename.rsplit_once('.') else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "md" | "markdown" | "txt" | "json" | "csv" | "yaml" | "yml"
    )
}

#[allow(clippy::too_many_arguments)]
fn candidate_from_relative_path(
    conn: &Connection,
    workspace_root: &Path,
    relative_path: &Path,
    run_id: &str,
    root_fingerprint: &str,
    key: &BackfillHandleKey,
    max_file_bytes: u64,
    entities: &[EntityPath],
) -> Result<Candidate, String> {
    let (mut file, identity) = WorkspaceSourceRegistry::open_validated(
        workspace_root,
        relative_path,
    )
    .map_err(|reason| match reason {
        crate::services::workspace_ingestion::contracts::RejectionReason::PathTraversalAttempt => {
            "path_traversal_attempt".to_string()
        }
        crate::services::workspace_ingestion::contracts::RejectionReason::SymlinkRefused => {
            "symlink_refused".to_string()
        }
        crate::services::workspace_ingestion::contracts::RejectionReason::SymlinkRaced => {
            "symlink_raced".to_string()
        }
        crate::services::workspace_ingestion::contracts::RejectionReason::OutsideWorkspace => {
            "outside_workspace".to_string()
        }
        crate::services::workspace_ingestion::contracts::RejectionReason::FileTooLarge => {
            "file_too_large".to_string()
        }
        crate::services::workspace_ingestion::contracts::RejectionReason::UnsupportedFormat => {
            "unsupported_format".to_string()
        }
    })?;
    let observed_at = Utc::now();
    let source_asof = source_asof_from_file(&file, observed_at)?;
    let file_size = file
        .metadata()
        .map_err(|_| "file_metadata_unavailable".to_string())?
        .len();
    if file_size > max_file_bytes {
        return Err("file_too_large".to_string());
    }
    let mut bytes = Vec::with_capacity(file_size as usize);
    file.read_to_end(&mut bytes)
        .map_err(|_| "file_read_failed".to_string())?;
    if std::str::from_utf8(&bytes).is_err() {
        return Err("non_utf8_file".to_string());
    }
    let content_sha256 = hex::encode(Sha256::digest(&bytes));
    let file_id = file_id_from_identity(&identity, workspace_root)
        .map_err(|_| "outside_workspace".to_string())?;
    let source_handle = key.handle(
        "source",
        "dailyos.workspace_backfill.source_handle.v1",
        &[root_fingerprint, &file_id],
    );
    let item_handle = key.handle(
        "item",
        "dailyos.workspace_backfill.item_handle.v1",
        &[run_id, &source_handle],
    );
    let duplicate_group_handle = key.handle(
        "dup",
        "dailyos.workspace_backfill.duplicate_group.v1",
        &[root_fingerprint, &content_sha256],
    );
    let classification =
        classify_candidate(conn, workspace_root, &identity, relative_path, entities);
    let link_handle = match (
        classification.entity_type,
        classification.entity_id.as_deref(),
    ) {
        (Some(entity_type), Some(entity_id)) => Some(key.handle(
            "link",
            "dailyos.workspace_backfill.link_handle.v1",
            &[root_fingerprint, &file_id, entity_type.as_str(), entity_id],
        )),
        _ => None,
    };

    Ok(Candidate {
        relative_path: relative_path.to_path_buf(),
        identity,
        file_id,
        source_handle,
        item_handle,
        duplicate_group_handle,
        link_handle,
        source_type: classification.source_type,
        entity_type: classification.entity_type,
        entity_id: classification.entity_id,
        entity_name: classification.entity_name,
        category: classification.category,
        source_asof,
        observed_at,
        content_sha256,
    })
}

fn source_asof_from_file(file: &File, observed_at: DateTime<Utc>) -> Result<DateTime<Utc>, String> {
    let modified = file
        .metadata()
        .and_then(|metadata| metadata.modified())
        .map_err(|_| "source_time_untrusted".to_string())?;
    modified
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "source_time_untrusted".to_string())?;
    let modified_at: DateTime<Utc> = modified.into();
    if modified_at > observed_at + Duration::minutes(5) {
        return Err("source_time_untrusted".to_string());
    }
    if modified_at > observed_at {
        return Ok(observed_at);
    }
    Ok(modified_at)
}

#[derive(Debug)]
struct Classification {
    source_type: WorkspaceFileKind,
    entity_type: Option<EntityType>,
    entity_id: Option<String>,
    entity_name: Option<String>,
    category: Option<WorkspaceCategory>,
}

fn classify_candidate(
    conn: &Connection,
    workspace_root: &Path,
    identity: &FileIdentity,
    relative_path: &Path,
    entities: &[EntityPath],
) -> Classification {
    let components = normal_components(relative_path);
    if components
        .first()
        .is_some_and(|component| component == "_inbox")
    {
        return Classification {
            source_type: WorkspaceFileKind::Inbox,
            entity_type: None,
            entity_id: None,
            entity_name: None,
            category: None,
        };
    }

    let matched = entities
        .iter()
        .filter(|entity| identity.canonical_path.starts_with(&entity.canonical_path))
        .max_by_key(|entity| entity.relative_path.components().count());

    if let Some(entity) = matched {
        let category = category_after_entity_prefix(
            conn,
            relative_path,
            &entity.relative_path,
            entity.entity_type,
        );
        return Classification {
            source_type: WorkspaceFileKind::EntityDoc,
            entity_type: Some(entity.entity_type),
            entity_id: Some(entity.entity_id.clone()),
            entity_name: Some(entity.entity_name.clone()),
            category,
        };
    }

    let conventional = conventional_root_match(conn, workspace_root, identity, &components);
    if let Some(entity) = conventional {
        let category = category_after_entity_prefix(
            conn,
            relative_path,
            &entity.relative_path,
            entity.entity_type,
        );
        return Classification {
            source_type: WorkspaceFileKind::EntityDoc,
            entity_type: Some(entity.entity_type),
            entity_id: Some(entity.entity_id),
            entity_name: Some(entity.entity_name),
            category,
        };
    }

    Classification {
        source_type: if matches!(
            components.first().map(String::as_str),
            Some("Accounts" | "People" | "Projects")
        ) {
            WorkspaceFileKind::EntityDoc
        } else {
            WorkspaceFileKind::UserAttachment
        },
        entity_type: None,
        entity_id: None,
        entity_name: None,
        category: None,
    }
}

fn category_after_entity_prefix(
    conn: &Connection,
    relative_path: &Path,
    entity_prefix: &Path,
    entity_type: EntityType,
) -> Option<WorkspaceCategory> {
    let remainder = relative_path.strip_prefix(entity_prefix).ok()?;
    let components = normal_components(remainder);
    if components.len() < 2 {
        return None;
    }
    let category = WorkspaceCategory::from_slug(&components[0])?;
    WorkspaceCategoryRegistry::validate(conn, &category, entity_type).ok()?;
    Some(category)
}

fn load_entity_paths(conn: &Connection, workspace_root: &Path) -> Result<Vec<EntityPath>, String> {
    let mut entities = Vec::new();
    load_entity_paths_from_table(
        conn,
        workspace_root,
        "accounts",
        EntityType::Account,
        "archived = 0 AND COALESCE(is_internal, 0) = 0",
        &mut entities,
    )?;
    load_entity_paths_from_table(
        conn,
        workspace_root,
        "people",
        EntityType::Person,
        "archived = 0",
        &mut entities,
    )?;
    load_entity_paths_from_table(
        conn,
        workspace_root,
        "projects",
        EntityType::Project,
        "archived = 0",
        &mut entities,
    )?;
    Ok(entities)
}

fn load_entity_paths_from_table(
    conn: &Connection,
    workspace_root: &Path,
    table: &str,
    entity_type: EntityType,
    where_clause: &str,
    out: &mut Vec<EntityPath>,
) -> Result<(), String> {
    let sql = format!(
        "SELECT id, name, tracker_path FROM {table} WHERE {where_clause} AND tracker_path IS NOT NULL"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|_| "entity_lookup_unavailable".to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|_| "entity_lookup_unavailable".to_string())?;
    for row in rows {
        let (entity_id, entity_name, tracker_path) =
            row.map_err(|_| "entity_lookup_unavailable".to_string())?;
        let relative_path = PathBuf::from(tracker_path);
        let canonical_path = match workspace_root.join(&relative_path).canonicalize() {
            Ok(path) => path,
            Err(_) => continue,
        };
        out.push(EntityPath {
            entity_type,
            entity_id,
            entity_name,
            canonical_path,
            relative_path,
        });
    }
    Ok(())
}

fn conventional_root_match(
    conn: &Connection,
    workspace_root: &Path,
    identity: &FileIdentity,
    components: &[String],
) -> Option<EntityPath> {
    if components.len() < 3 {
        return None;
    }
    let (table, entity_type) = match components[0].as_str() {
        "Accounts" => ("accounts", EntityType::Account),
        "People" => ("people", EntityType::Person),
        "Projects" => ("projects", EntityType::Project),
        _ => return None,
    };
    let entity_name = &components[1];
    let sql = format!("SELECT id, name FROM {table} WHERE archived = 0 AND name = ?1");
    let mut stmt = conn.prepare(&sql).ok()?;
    let rows = stmt
        .query_map(params![entity_name], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .ok()?;
    let matches = rows.filter_map(Result::ok).collect::<Vec<_>>();
    if matches.len() != 1 {
        return None;
    }
    let relative_path = PathBuf::from(&components[0]).join(entity_name);
    let canonical_path = workspace_root.join(&relative_path).canonicalize().ok()?;
    if !identity.canonical_path.starts_with(&canonical_path) {
        return None;
    }
    Some(EntityPath {
        entity_type,
        entity_id: matches[0].0.clone(),
        entity_name: matches[0].1.clone(),
        canonical_path,
        relative_path,
    })
}

fn duplicate_groups(candidates: &[Candidate]) -> Vec<DuplicateContentGroupSummary> {
    let mut by_hash = BTreeMap::<&str, (&str, u64)>::new();
    for candidate in candidates {
        by_hash
            .entry(&candidate.content_sha256)
            .and_modify(|(_, count)| *count += 1)
            .or_insert((&candidate.duplicate_group_handle, 1));
    }
    by_hash
        .into_values()
        .filter(|(_, count)| *count > 1)
        .map(|(handle, count)| DuplicateContentGroupSummary {
            duplicate_group_handle: handle.to_string(),
            source_count: count,
        })
        .collect()
}

fn divergence_counts(conn: &Connection, workspace_root: &Path) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    count_sql(
        conn,
        &mut counts,
        "active_link_without_lifecycle",
        "SELECT count(*) FROM document_entity_links l
         LEFT JOIN workspace_file_lifecycle w ON w.file_id = l.file_id
         WHERE l.rejected = 0 AND w.file_id IS NULL",
    );
    count_sql(
        conn,
        &mut counts,
        "content_index_without_lifecycle",
        "SELECT count(*) FROM content_index c
         LEFT JOIN workspace_file_lifecycle w ON w.file_id = c.id
         WHERE w.file_id IS NULL",
    );
    count_sql(
        conn,
        &mut counts,
        "embedding_without_source_lifecycle",
        "SELECT count(*) FROM content_embeddings e
         LEFT JOIN content_index c ON c.id = e.content_file_id
         LEFT JOIN workspace_file_lifecycle w ON w.file_id = c.id
         WHERE c.id IS NULL OR w.file_id IS NULL",
    );

    let lifecycle_missing = count_lifecycle_missing_files(conn, workspace_root);
    if lifecycle_missing > 0 {
        counts.insert("lifecycle_file_missing".to_string(), lifecycle_missing);
    }
    counts
}

fn count_sql(conn: &Connection, counts: &mut BTreeMap<String, u64>, key: &str, sql: &str) {
    let count = conn
        .query_row(sql, [], |row| row.get::<_, i64>(0))
        .unwrap_or(0);
    if count > 0 {
        counts.insert(key.to_string(), count as u64);
    }
}

fn count_lifecycle_missing_files(conn: &Connection, workspace_root: &Path) -> u64 {
    let Ok(mut stmt) = conn.prepare("SELECT canonical_path FROM workspace_file_lifecycle") else {
        return 0;
    };
    let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) else {
        return 0;
    };
    rows.filter_map(Result::ok)
        .filter(|raw| {
            let path = Path::new(raw);
            !path.exists() || !path.starts_with(workspace_root)
        })
        .count() as u64
}

fn create_or_resume_run(
    conn: &Connection,
    run_id: &str,
    mode: BackfillMode,
    root_fingerprint: &str,
    reason_counts: &BTreeMap<String, u64>,
    source_class_counts: &BTreeMap<String, u64>,
    divergence_counts: &BTreeMap<String, u64>,
) -> Result<(), String> {
    let existing: Option<(String, String, String)> = conn
        .query_row(
            "SELECT mode, status, workspace_root_fingerprint
             FROM workspace_backfill_runs
             WHERE run_id = ?1",
            params![run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|_| "backfill_run_unavailable".to_string())?;

    if let Some((existing_mode, status, existing_fingerprint)) = existing {
        if existing_mode != mode.as_str() {
            return Err("backfill_run_mode_mismatch".to_string());
        }
        if existing_fingerprint != root_fingerprint {
            return Err("backfill_run_workspace_mismatch".to_string());
        }
        if !matches!(status.as_str(), "running" | "failed") {
            return Err("backfill_run_not_resumable".to_string());
        }
        conn.execute(
            "UPDATE workspace_backfill_runs
             SET status = 'running',
                 reason_counts_json = ?2,
                 source_class_counts_json = ?3,
                 divergence_counts_json = ?4,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE run_id = ?1",
            params![
                run_id,
                json_map(reason_counts)?,
                json_map(source_class_counts)?,
                json_map(divergence_counts)?,
            ],
        )
        .map_err(|_| "backfill_run_unavailable".to_string())?;
    } else {
        conn.execute(
            "INSERT INTO workspace_backfill_runs
                (run_id, mode, status, workspace_root_fingerprint, actor,
                 reason_counts_json, source_class_counts_json, divergence_counts_json)
             VALUES (?1, ?2, 'running', ?3, ?4, ?5, ?6, ?7)",
            params![
                run_id,
                mode.as_str(),
                root_fingerprint,
                ACTOR,
                json_map(reason_counts)?,
                json_map(source_class_counts)?,
                json_map(divergence_counts)?,
            ],
        )
        .map_err(|_| "backfill_run_unavailable".to_string())?;
    }
    Ok(())
}

fn complete_run(
    conn: &Connection,
    run_id: &str,
    status: &str,
    reason_counts: &BTreeMap<String, u64>,
    source_class_counts: &BTreeMap<String, u64>,
    divergence_counts: &BTreeMap<String, u64>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE workspace_backfill_runs
         SET status = ?1,
             reason_counts_json = ?2,
             source_class_counts_json = ?3,
             divergence_counts_json = ?4,
             completed_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'),
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE run_id = ?5",
        params![
            status,
            json_map(reason_counts)?,
            json_map(source_class_counts)?,
            json_map(divergence_counts)?,
            run_id,
        ],
    )
    .map_err(|_| "backfill_run_unavailable".to_string())?;
    Ok(())
}

fn apply_candidate(
    db: &ActionDb,
    workspace_root: &Path,
    run_id: &str,
    candidate: &Candidate,
    runtime: &WorkspaceBackfillSignalRuntime<'_, '_>,
) -> Result<ApplyOutcome, String> {
    let prior_status: Option<String> = db
        .conn_ref()
        .query_row(
            "SELECT status FROM workspace_backfill_items
             WHERE run_id = ?1 AND source_handle = ?2",
            params![run_id, candidate.source_handle],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "backfill_item_unavailable".to_string())?;
    if prior_status
        .as_deref()
        .is_some_and(|status| matches!(status, "applied" | "skipped"))
    {
        return Ok(ApplyOutcome {
            applied: false,
            skipped_reason: Some("already_processed_in_run".to_string()),
        });
    }

    let (mut file, identity) =
        WorkspaceSourceRegistry::open_validated(workspace_root, &candidate.relative_path)
            .map_err(|_| "source_changed_during_backfill".to_string())?;
    if identity.device != candidate.identity.device || identity.inode != candidate.identity.inode {
        return Err("source_changed_during_backfill".to_string());
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| "file_read_failed".to_string())?;
    let content_sha256 = hex::encode(Sha256::digest(&bytes));
    if content_sha256 != candidate.content_sha256 {
        return Err("source_changed_during_backfill".to_string());
    }
    let source_asof = source_asof_from_file(&file, candidate.observed_at)
        .map_err(|_| "source_changed_during_backfill".to_string())?;
    if source_asof != candidate.source_asof {
        return Err("source_changed_during_backfill".to_string());
    }

    db.with_transaction(|tx_db| {
        upsert_item(tx_db.conn_ref(), run_id, candidate, "planned", None)?;

        let existing = LifecycleRepo::get(tx_db.conn_ref(), &candidate.file_id)
            .map_err(|_| "lifecycle_unavailable".to_string())?;
        let created_lifecycle = existing.is_none();
        if existing.is_none() {
            LifecycleRepo::insert_pending(
                tx_db.conn_ref(),
                &candidate.file_id,
                &candidate.identity,
                &candidate.source_type,
                candidate.source_asof,
                candidate.entity_ref().as_ref(),
            )
            .map_err(|_| "lifecycle_unavailable".to_string())?;
        }

        let mut updated_fields = Vec::new();
        let lifecycle = LifecycleRepo::get(tx_db.conn_ref(), &candidate.file_id)
            .map_err(|_| "lifecycle_unavailable".to_string())?
            .ok_or_else(|| "lifecycle_unavailable".to_string())?;
        if lifecycle.content_sha256.as_deref() != Some(candidate.content_sha256.as_str()) {
            LifecycleRepo::update_content_sha256(
                tx_db.conn_ref(),
                &candidate.file_id,
                &candidate.content_sha256,
            )
            .map_err(|_| "lifecycle_unavailable".to_string())?;
            updated_fields.push("content_sha256");
        }
        if lifecycle.category.as_ref() != candidate.category.as_ref() {
            LifecycleRepo::update_category(
                tx_db.conn_ref(),
                &candidate.file_id,
                candidate.category.as_ref(),
            )
            .map_err(|_| "lifecycle_unavailable".to_string())?;
            updated_fields.push("category");
        }

        let lifecycle = LifecycleRepo::get(tx_db.conn_ref(), &candidate.file_id)
            .map_err(|_| "lifecycle_unavailable".to_string())?
            .ok_or_else(|| "lifecycle_unavailable".to_string())?;
        if candidate.entity_id.is_none() && lifecycle.lifecycle_state == LifecycleState::Pending {
            LifecycleRepo::transition(
                tx_db.conn_ref(),
                &candidate.file_id,
                LifecycleState::Pending,
                LifecycleState::PendingEntityAssignment,
            )
            .map_err(|_| "lifecycle_unavailable".to_string())?;
            updated_fields.push("lifecycle_state");
        }
        if lifecycle.entity_id.is_none() {
            if let (Some(entity_type), Some(entity_id)) =
                (candidate.entity_type, candidate.entity_id.as_deref())
            {
                LifecycleRepo::set_entity(
                    tx_db.conn_ref(),
                    &candidate.file_id,
                    entity_type,
                    entity_id,
                    candidate.entity_name.as_deref(),
                )
                .map_err(|_| "lifecycle_unavailable".to_string())?;
                updated_fields.push("entity");
            }
        }

        let mut created_link_handle = None;
        let mut link_reason = None;
        if let (Some(entity_type), Some(entity_id)) =
            (candidate.entity_type, candidate.entity_id.as_deref())
        {
            let current = LifecycleRepo::get(tx_db.conn_ref(), &candidate.file_id)
                .map_err(|_| "lifecycle_unavailable".to_string())?
                .ok_or_else(|| "lifecycle_unavailable".to_string())?;
            if current
                .entity_id
                .as_deref()
                .is_some_and(|id| id != entity_id)
                || current
                    .entity_type
                    .as_deref()
                    .is_some_and(|kind| kind != entity_type.as_str())
            {
                link_reason = Some("existing_entity_conflict".to_string());
            } else {
                match LinkRepo::add_link_with_outcome_in_tx(
                    tx_db.conn_ref(),
                    &candidate.file_id,
                    entity_type,
                    entity_id,
                    LinkAttributionSource::Backfill,
                    0.95,
                    None,
                    ACTOR,
                ) {
                    Ok(outcome) if outcome.inserted => {
                        let tx_signal_ctx = SignalEmitContext::new(
                            runtime.services,
                            tx_db,
                            Some(runtime.propagation),
                        );
                        runtime
                            .emitter
                            .emit_link_changed(
                                &tx_signal_ctx,
                                &candidate.file_id,
                                entity_type.as_str(),
                                entity_id,
                                ACTOR,
                            )
                            .map_err(|_| "signal_emit_failed".to_string())?;
                        created_link_handle = candidate.link_handle.clone();
                    }
                    Ok(_) => {
                        link_reason = Some("active_link_already_exists".to_string());
                    }
                    Err(LinkError::Tombstoned { .. }) => {
                        link_reason = Some("link_tombstoned".to_string());
                    }
                    Err(_) => return Err("link_unavailable".to_string()),
                }
            }
        }

        let applied =
            created_lifecycle || !updated_fields.is_empty() || created_link_handle.is_some();
        let item_status = if applied { "applied" } else { "skipped" };
        let item_reason = if applied {
            None
        } else {
            Some(link_reason.unwrap_or_else(|| "already_registered".to_string()))
        };
        upsert_item(
            tx_db.conn_ref(),
            run_id,
            candidate,
            item_status,
            item_reason.as_deref(),
        )?;
        record_operation(
            tx_db.conn_ref(),
            run_id,
            candidate,
            "register_source",
            item_status,
            created_lifecycle,
            &updated_fields,
            created_link_handle.as_deref(),
            item_reason.as_deref(),
        )?;
        Ok(ApplyOutcome {
            applied,
            skipped_reason: item_reason,
        })
    })
}

fn upsert_item(
    conn: &Connection,
    run_id: &str,
    candidate: &Candidate,
    status: &str,
    reason_code: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO workspace_backfill_items
            (run_id, source_handle, item_handle, file_id, content_sha256,
             duplicate_group_handle, candidate_kind, entity_type, entity_id, category,
             exposure_state, source_time_basis, source_time_confidence,
             backfill_observed_at, status, reason_code)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
         ON CONFLICT(run_id, source_handle) DO UPDATE SET
             item_handle = excluded.item_handle,
             file_id = excluded.file_id,
             content_sha256 = excluded.content_sha256,
             duplicate_group_handle = excluded.duplicate_group_handle,
             candidate_kind = excluded.candidate_kind,
             entity_type = excluded.entity_type,
             entity_id = excluded.entity_id,
             category = excluded.category,
             exposure_state = excluded.exposure_state,
             source_time_basis = excluded.source_time_basis,
             source_time_confidence = excluded.source_time_confidence,
             backfill_observed_at = excluded.backfill_observed_at,
             status = excluded.status,
             reason_code = excluded.reason_code,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        params![
            run_id,
            candidate.source_handle,
            candidate.item_handle,
            candidate.file_id,
            candidate.content_sha256,
            candidate.duplicate_group_handle,
            candidate.candidate_kind(),
            candidate
                .entity_type
                .map(|entity_type| entity_type.as_str()),
            candidate.entity_id.as_deref(),
            candidate.category.as_ref().map(WorkspaceCategory::as_slug),
            EXPOSURE_PENDING_REVIEW,
            SOURCE_TIME_BASIS,
            SOURCE_TIME_CONFIDENCE,
            candidate
                .observed_at
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            status,
            reason_code,
        ],
    )
    .map_err(|_| "backfill_item_unavailable".to_string())?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn record_operation(
    conn: &Connection,
    run_id: &str,
    candidate: &Candidate,
    operation_kind: &str,
    status: &str,
    created_lifecycle: bool,
    updated_fields: &[&str],
    created_link_handle: Option<&str>,
    reason_code: Option<&str>,
) -> Result<(), String> {
    let fields = serde_json::to_string(updated_fields)
        .map_err(|_| "backfill_operation_unavailable".to_string())?;
    conn.execute(
        "INSERT INTO workspace_backfill_operations
            (run_id, source_handle, operation_kind, status, created_lifecycle,
             updated_lifecycle_fields, created_link_handle, reason_code)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            run_id,
            candidate.source_handle,
            operation_kind,
            status,
            i64::from(created_lifecycle),
            fields,
            created_link_handle,
            reason_code,
        ],
    )
    .map_err(|_| "backfill_operation_unavailable".to_string())?;
    Ok(())
}

fn record_failed_item(
    db: &ActionDb,
    run_id: &str,
    candidate: &Candidate,
    reason_code: &str,
) -> Result<(), String> {
    db.with_transaction(|tx_db| {
        upsert_item(
            tx_db.conn_ref(),
            run_id,
            candidate,
            "failed",
            Some(reason_code),
        )?;
        record_operation(
            tx_db.conn_ref(),
            run_id,
            candidate,
            "register_source",
            "failed",
            false,
            &[],
            None,
            Some(reason_code),
        )
    })
}

fn json_map(map: &BTreeMap<String, u64>) -> Result<String, String> {
    serde_json::to_string(map).map_err(|_| "summary_serialization_failed".to_string())
}

fn normal_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect()
}

fn increment(map: &mut BTreeMap<String, u64>, key: &str) {
    *map.entry(key.to_string()).or_default() += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use abilities_runtime::abilities::workspace_graph::contracts::{
        WorkspaceGraphInput, WorkspaceGraphPrivacyProfile, WorkspaceGraphReadRequest,
        WorkspaceGraphResponse,
    };
    use tempfile::TempDir;

    fn test_db() -> ActionDb {
        crate::db::test_utils::test_db()
    }

    fn runtime<'a>(
        services: &'a ServiceContext<'a>,
        propagation: &'a PropagationEngine,
        emitter: &'a WorkspaceSignalEmitter,
    ) -> WorkspaceBackfillSignalRuntime<'a, 'a> {
        WorkspaceBackfillSignalRuntime {
            services,
            propagation,
            emitter,
        }
    }

    fn service_context() -> ServiceContext<'static> {
        let clock: &'static SystemClock = Box::leak(Box::new(SystemClock));
        let rng: &'static SystemRng = Box::leak(Box::new(SystemRng));
        let external: &'static ExternalClients = Box::leak(Box::new(ExternalClients::default()));
        ServiceContext::new_live(clock, rng, external).with_actor(ACTOR)
    }

    fn seed_account(conn: &Connection, root: &Path) {
        seed_account_with(conn, root, "acct-1", "ExampleCo");
    }

    fn seed_account_with(conn: &Connection, root: &Path, account_id: &str, account_name: &str) {
        let account_dir = root.join("Accounts").join(account_name);
        std::fs::create_dir_all(account_dir.join("notes")).expect("account dirs");
        conn.execute(
            "INSERT OR IGNORE INTO accounts (id, name, tracker_path, updated_at, archived, is_internal)
             VALUES (?1, ?2, ?3, '2026-05-25T00:00:00Z', 0, 0)",
            params![
                account_id,
                account_name,
                format!("Accounts/{account_name}")
            ],
        )
        .expect("account");
    }

    fn table_count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap_or_else(|error| panic!("count {table}: {error}"))
    }

    fn table_counts(conn: &Connection, tables: &[&'static str]) -> Vec<(&'static str, i64)> {
        tables
            .iter()
            .map(|table| (*table, table_count(conn, table)))
            .collect()
    }

    fn assert_tables_empty(conn: &Connection, tables: &[&str]) {
        for table in tables {
            assert_eq!(table_count(conn, table), 0, "{table} must remain empty");
        }
    }

    fn file_id_for(workspace_root: &Path, relative_path: &str) -> String {
        let (_, identity) =
            WorkspaceSourceRegistry::open_validated(workspace_root, Path::new(relative_path))
                .expect("validated source");
        file_id_from_identity(&identity, workspace_root).expect("file id")
    }

    fn set_mtime(path: &Path, time: std::time::SystemTime) {
        filetime::set_file_mtime(path, filetime::FileTime::from_system_time(time))
            .expect("set mtime");
    }

    #[test]
    fn dry_run_discovers_without_writing_rows_or_leaking_paths() {
        let db = test_db();
        let temp = TempDir::new().expect("temp");
        seed_account(db.conn_ref(), temp.path());
        let file_path = temp.path().join("Accounts/ExampleCo/notes/source.md");
        std::fs::write(&file_path, "same source text").expect("file");
        let write_boundary = [
            "workspace_file_lifecycle",
            "workspace_backfill_runs",
            "workspace_backfill_items",
            "workspace_backfill_operations",
            "document_entity_links",
            "document_ingestion_runs",
            "content_index",
            "content_embeddings",
            "intelligence_claims",
            "signal_events",
        ];
        let before = table_counts(db.conn_ref(), &write_boundary);

        let key = BackfillHandleKey::for_tests("install-a");
        let summary = run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::dry_run(Some(temp.path().to_path_buf())),
            &key,
            None,
        )
        .expect("dry-run");

        assert_eq!(summary.mode, "dry_run");
        assert_eq!(summary.eligible_count, 1);
        assert_eq!(
            summary
                .divergence_counts
                .get("filesystem_candidate_without_lifecycle"),
            Some(&1)
        );
        assert_eq!(table_counts(db.conn_ref(), &write_boundary), before);
        assert_eq!(
            std::fs::read_to_string(&file_path).expect("file content"),
            "same source text"
        );

        let json = serde_json::to_string(&summary).expect("summary json");
        assert!(!json.contains("ExampleCo"));
        assert!(!json.contains("source.md"));
        assert!(!json.contains(temp.path().to_string_lossy().as_ref()));
        assert!(!json.contains("same source text"));
    }

    #[test]
    fn apply_registers_pending_review_source_and_graph_excludes_it() {
        let db = test_db();
        let temp = TempDir::new().expect("temp");
        seed_account_with(db.conn_ref(), temp.path(), "acct-signal", "ExampleSignal");
        std::fs::write(
            temp.path().join("Accounts/ExampleSignal/notes/source.md"),
            "source text",
        )
        .expect("file");
        let key = BackfillHandleKey::for_tests("install-a");
        let services = service_context();
        let propagation = default_engine();
        let emitter = WorkspaceSignalEmitter;

        let summary = run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::apply(Some(temp.path().to_path_buf()), None),
            &key,
            Some(runtime(&services, &propagation, &emitter)),
        )
        .expect("apply");

        assert_eq!(summary.applied_count, 1);
        let lifecycle_state: String = db
            .conn_ref()
            .query_row(
                "SELECT lifecycle_state FROM workspace_file_lifecycle",
                [],
                |row| row.get(0),
            )
            .expect("lifecycle state");
        assert_eq!(
            lifecycle_state,
            crate::services::workspace_ingestion::lifecycle::lifecycle_state_slug(
                LifecycleState::Pending
            )
        );
        let exposure_state: String = db
            .conn_ref()
            .query_row(
                "SELECT exposure_state FROM workspace_backfill_items",
                [],
                |row| row.get(0),
            )
            .expect("exposure state");
        assert_eq!(exposure_state, EXPOSURE_PENDING_REVIEW);

        let response = crate::services::workspace_ingestion::graph::read_workspace_graph(
            db.conn_ref(),
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
            &crate::services::workspace_ingestion::graph::WorkspaceGraphDiagnosticKey::for_tests(
                "graph",
            ),
        )
        .expect("graph");
        let WorkspaceGraphResponse::Projection(projection) = response else {
            panic!("expected projection");
        };
        assert!(
            projection.projection.entities.is_empty(),
            "pending-review sources must stay out of workspace graph"
        );
        assert_tables_empty(
            db.conn_ref(),
            &[
                "document_ingestion_runs",
                "content_index",
                "content_embeddings",
                "intelligence_claims",
            ],
        );
    }

    #[test]
    fn apply_emits_link_signal_only_for_inserted_backfill_link() {
        let db = test_db();
        let temp = TempDir::new().expect("temp");
        let unique = uuid::Uuid::new_v4().simple().to_string();
        let account_id = format!("acct-{unique}");
        let account_name = format!("ExampleSignal{}", &unique[..8]);
        seed_account_with(db.conn_ref(), temp.path(), &account_id, &account_name);
        std::fs::write(
            temp.path()
                .join("Accounts")
                .join(&account_name)
                .join("notes/source.md"),
            "source text",
        )
        .expect("file");
        let key = BackfillHandleKey::for_tests("install-a");
        let services = service_context();
        let propagation = default_engine();
        let emitter = WorkspaceSignalEmitter;

        run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::apply(Some(temp.path().to_path_buf()), None),
            &key,
            Some(runtime(&services, &propagation, &emitter)),
        )
        .expect("first apply");
        run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::apply(Some(temp.path().to_path_buf()), None),
            &key,
            Some(runtime(&services, &propagation, &emitter)),
        )
        .expect("second apply");

        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM signal_events WHERE signal_type = 'workspace_file_entity_link_changed'",
                [],
                |row| row.get(0),
            )
            .expect("signal count");
        assert_eq!(count, 1);
        let signals = db
            .conn_ref()
            .prepare("SELECT signal_type, value FROM signal_events ORDER BY created_at")
            .expect("prepare signals")
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .expect("query signals")
            .collect::<Result<Vec<_>, _>>()
            .expect("signals");
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].0, "workspace_file_entity_link_changed");
        let payload: serde_json::Value =
            serde_json::from_str(&signals[0].1).expect("signal payload");
        let keys = payload
            .as_object()
            .expect("payload object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            keys,
            vec!["actor_kind", "entity_id", "entity_type", "file_id"]
        );
        assert_eq!(payload["actor_kind"], "system");
        let rendered = payload.to_string();
        assert!(!rendered.contains(&account_name));
        assert!(!rendered.contains("source.md"));
        assert!(!rendered.contains("source text"));
        assert!(!rendered.contains(temp.path().to_string_lossy().as_ref()));
        assert_tables_empty(
            db.conn_ref(),
            &[
                "document_ingestion_runs",
                "content_index",
                "content_embeddings",
                "intelligence_claims",
            ],
        );
        assert_eq!(
            table_count(db.conn_ref(), "document_entity_links"),
            1,
            "apply may create only the conservative Backfill link for inferred entities"
        );
        assert_eq!(
            table_count(db.conn_ref(), "workspace_file_lifecycle"),
            1,
            "apply registers exactly one source lifecycle row"
        );
        assert_eq!(
            table_count(db.conn_ref(), "workspace_backfill_items"),
            2,
            "each apply run records its own backfill item ledger row"
        );
        assert_eq!(
            table_count(db.conn_ref(), "workspace_backfill_operations"),
            2,
            "each apply run records its own backfill operation ledger row"
        );
        assert_eq!(
            table_count(db.conn_ref(), "signal_events"),
            1,
            "apply must not emit workspace_file_ingested or other extra signals"
        );
        for unexpected in [
            "workspace_file_ingested",
            "workspace_file_pending_entity_assignment",
            "workspace_file_rejected",
            "workspace_file_quarantined",
        ] {
            let count: i64 = db
                .conn_ref()
                .query_row(
                    "SELECT count(*) FROM signal_events WHERE signal_type = ?1",
                    [unexpected],
                    |row| row.get(0),
                )
                .expect("unexpected signal count");
            assert_eq!(count, 0, "{unexpected} must not be emitted by W5-A");
        }
    }

    #[test]
    fn apply_lifecycle_only_registration_emits_no_signal_or_pipeline_rows() {
        let db = test_db();
        let temp = TempDir::new().expect("temp");
        std::fs::create_dir_all(temp.path().join("_inbox")).expect("inbox");
        std::fs::write(temp.path().join("_inbox/source.md"), "unassigned source").expect("file");
        let key = BackfillHandleKey::for_tests("install-a");
        let services = service_context();
        let propagation = default_engine();
        let emitter = WorkspaceSignalEmitter;

        let summary = run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::apply(Some(temp.path().to_path_buf()), None),
            &key,
            Some(runtime(&services, &propagation, &emitter)),
        )
        .expect("apply");

        assert_eq!(summary.applied_count, 1);
        assert_eq!(table_count(db.conn_ref(), "signal_events"), 0);
        assert_tables_empty(
            db.conn_ref(),
            &[
                "document_entity_links",
                "document_ingestion_runs",
                "content_index",
                "content_embeddings",
                "intelligence_claims",
            ],
        );
        let lifecycle_state: String = db
            .conn_ref()
            .query_row(
                "SELECT lifecycle_state FROM workspace_file_lifecycle",
                [],
                |row| row.get(0),
            )
            .expect("lifecycle state");
        assert_eq!(
            lifecycle_state,
            crate::services::workspace_ingestion::lifecycle::lifecycle_state_slug(
                LifecycleState::PendingEntityAssignment
            )
        );
    }

    #[test]
    fn duplicate_content_uses_opaque_group_handles_only() {
        let db = test_db();
        let temp = TempDir::new().expect("temp");
        seed_account(db.conn_ref(), temp.path());
        std::fs::write(
            temp.path().join("Accounts/ExampleCo/notes/a.md"),
            "duplicate source",
        )
        .expect("file a");
        std::fs::write(
            temp.path().join("Accounts/ExampleCo/notes/b.md"),
            "duplicate source",
        )
        .expect("file b");
        let raw_hash = hex::encode(Sha256::digest(b"duplicate source"));
        let key = BackfillHandleKey::for_tests("install-a");
        let summary = run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::dry_run(Some(temp.path().to_path_buf())),
            &key,
            None,
        )
        .expect("dry-run");

        assert_eq!(summary.duplicate_groups.len(), 1);
        assert_eq!(summary.duplicate_groups[0].source_count, 2);
        let json = serde_json::to_string(&summary).expect("summary json");
        assert!(!json.contains(&raw_hash));
        assert!(!json.contains("duplicate source"));
        assert!(summary.duplicate_groups[0]
            .duplicate_group_handle
            .starts_with("dup:v1:"));
    }

    #[test]
    fn handles_are_stable_and_key_or_workspace_scoped() {
        let db = test_db();
        let first = TempDir::new().expect("first temp");
        let second = TempDir::new().expect("second temp");
        seed_account(db.conn_ref(), first.path());
        seed_account(db.conn_ref(), second.path());
        std::fs::write(first.path().join("Accounts/ExampleCo/notes/a.md"), "alpha")
            .expect("first file");
        std::fs::write(second.path().join("Accounts/ExampleCo/notes/a.md"), "alpha")
            .expect("second file");
        let key_a = BackfillHandleKey::for_tests("install-a");
        let key_b = BackfillHandleKey::for_tests("install-b");

        let first_a = run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::dry_run(Some(first.path().to_path_buf())),
            &key_a,
            None,
        )
        .expect("first a");
        let first_a_again = run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::dry_run(Some(first.path().to_path_buf())),
            &key_a,
            None,
        )
        .expect("first a again");
        let first_b = run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::dry_run(Some(first.path().to_path_buf())),
            &key_b,
            None,
        )
        .expect("first b");
        let second_a = run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::dry_run(Some(second.path().to_path_buf())),
            &key_a,
            None,
        )
        .expect("second a");

        assert_eq!(first_a.source_handles, first_a_again.source_handles);
        assert_ne!(first_a.source_handles, first_b.source_handles);
        assert_ne!(first_a.source_handles, second_a.source_handles);
        assert!(first_a.source_handles[0].starts_with("source:v1:"));
    }

    #[test]
    fn future_source_time_is_skipped_without_now_fallback() {
        let db = test_db();
        let temp = TempDir::new().expect("temp");
        seed_account(db.conn_ref(), temp.path());
        let file_path = temp.path().join("Accounts/ExampleCo/notes/future.md");
        std::fs::write(&file_path, "future").expect("file");
        let future = filetime::FileTime::from_system_time(
            std::time::SystemTime::now() + std::time::Duration::from_secs(60 * 60),
        );
        filetime::set_file_mtime(&file_path, future).expect("mtime");
        let key = BackfillHandleKey::for_tests("install-a");

        let summary = run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::dry_run(Some(temp.path().to_path_buf())),
            &key,
            None,
        )
        .expect("dry-run");

        assert_eq!(summary.eligible_count, 0);
        assert_eq!(summary.reason_counts.get("source_time_untrusted"), Some(&1));
    }

    #[test]
    fn apply_records_weak_filesystem_source_time_metadata() {
        let db = test_db();
        let temp = TempDir::new().expect("temp");
        std::fs::create_dir_all(temp.path().join("_inbox")).expect("inbox");
        let file_path = temp.path().join("_inbox/source.md");
        std::fs::write(&file_path, "source time").expect("file");
        let mtime = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        set_mtime(&file_path, mtime);
        let key = BackfillHandleKey::for_tests("install-a");
        let services = service_context();
        let propagation = default_engine();
        let emitter = WorkspaceSignalEmitter;

        run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::apply(Some(temp.path().to_path_buf()), None),
            &key,
            Some(runtime(&services, &propagation, &emitter)),
        )
        .expect("apply");

        let workspace_root = temp.path().canonicalize().expect("root");
        let file_id = file_id_for(&workspace_root, "_inbox/source.md");
        let lifecycle = LifecycleRepo::get(db.conn_ref(), &file_id)
            .expect("lifecycle")
            .expect("lifecycle row");
        let expected: DateTime<Utc> = mtime.into();
        assert_eq!(lifecycle.source_asof.timestamp(), expected.timestamp());

        let (basis, confidence, observed_at): (String, String, String) = db
            .conn_ref()
            .query_row(
                "SELECT source_time_basis, source_time_confidence, backfill_observed_at
                 FROM workspace_backfill_items",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("source time item");
        assert_eq!(basis, SOURCE_TIME_BASIS);
        assert_eq!(confidence, SOURCE_TIME_CONFIDENCE);
        let observed_at = DateTime::parse_from_rfc3339(&observed_at)
            .expect("observed_at")
            .with_timezone(&Utc);
        assert!(observed_at >= lifecycle.source_asof);
    }

    #[test]
    fn near_future_source_time_is_clamped_to_observed_at() {
        let db = test_db();
        let temp = TempDir::new().expect("temp");
        std::fs::create_dir_all(temp.path().join("_inbox")).expect("inbox");
        let file_path = temp.path().join("_inbox/future.md");
        std::fs::write(&file_path, "future but close").expect("file");
        set_mtime(
            &file_path,
            std::time::SystemTime::now() + std::time::Duration::from_secs(60),
        );
        let key = BackfillHandleKey::for_tests("install-a");
        let services = service_context();
        let propagation = default_engine();
        let emitter = WorkspaceSignalEmitter;

        run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::apply(Some(temp.path().to_path_buf()), None),
            &key,
            Some(runtime(&services, &propagation, &emitter)),
        )
        .expect("apply");

        let (source_asof, observed_at): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT w.source_asof, b.backfill_observed_at
                 FROM workspace_file_lifecycle w
                 CROSS JOIN workspace_backfill_items b",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("source times");
        assert_eq!(source_asof, observed_at);
    }

    #[test]
    fn apply_rejects_source_time_changed_after_scan() {
        let db = test_db();
        let temp = TempDir::new().expect("temp");
        std::fs::create_dir_all(temp.path().join("_inbox")).expect("inbox");
        let file_path = temp.path().join("_inbox/source.md");
        std::fs::write(&file_path, "stable content").expect("file");
        set_mtime(
            &file_path,
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000),
        );
        let workspace_root = temp.path().canonicalize().expect("root");
        let key = BackfillHandleKey::for_tests("install-a");
        let root_fingerprint = key.handle(
            "root",
            "dailyos.workspace_backfill.workspace_root_fingerprint.v1",
            &[&workspace_root.to_string_lossy()],
        );
        let scan = scan_workspace(
            db.conn_ref(),
            &workspace_root,
            "run-1",
            &root_fingerprint,
            &key,
            DEFAULT_MAX_FILE_BYTES,
        )
        .expect("scan");
        assert_eq!(scan.candidates.len(), 1);
        set_mtime(
            &file_path,
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_060),
        );
        let services = service_context();
        let propagation = default_engine();
        let emitter = WorkspaceSignalEmitter;

        let err = apply_candidate(
            &db,
            &workspace_root,
            "run-1",
            &scan.candidates[0],
            &runtime(&services, &propagation, &emitter),
        )
        .expect_err("changed mtime rejected");

        assert_eq!(err, "source_changed_during_backfill");
        assert_tables_empty(
            db.conn_ref(),
            &[
                "workspace_file_lifecycle",
                "workspace_backfill_items",
                "workspace_backfill_operations",
                "signal_events",
            ],
        );
    }

    #[test]
    fn resume_rejects_mismatched_workspace_root() {
        let db = test_db();
        let first = TempDir::new().expect("first temp");
        let second = TempDir::new().expect("second temp");
        seed_account(db.conn_ref(), first.path());
        seed_account(db.conn_ref(), second.path());
        std::fs::write(first.path().join("Accounts/ExampleCo/notes/a.md"), "alpha")
            .expect("first file");
        std::fs::write(second.path().join("Accounts/ExampleCo/notes/a.md"), "alpha")
            .expect("second file");
        let key = BackfillHandleKey::for_tests("install-a");
        let services = service_context();
        let propagation = default_engine();
        let emitter = WorkspaceSignalEmitter;

        run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::apply(
                Some(first.path().to_path_buf()),
                Some("run-1".to_string()),
            ),
            &key,
            Some(runtime(&services, &propagation, &emitter)),
        )
        .expect("first apply");
        let err = run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::apply(
                Some(second.path().to_path_buf()),
                Some("run-1".to_string()),
            ),
            &key,
            Some(runtime(&services, &propagation, &emitter)),
        )
        .expect_err("workspace mismatch rejected");

        assert_eq!(err, "backfill_run_workspace_mismatch");
        assert_eq!(table_count(db.conn_ref(), "workspace_backfill_runs"), 1);
    }

    #[test]
    fn resumed_failed_item_refreshes_candidate_metadata() {
        let db = test_db();
        let temp = TempDir::new().expect("temp");
        std::fs::create_dir_all(temp.path().join("_inbox")).expect("inbox");
        let file_path = temp.path().join("_inbox/source.md");
        std::fs::write(&file_path, "old content").expect("old file");
        let workspace_root = temp.path().canonicalize().expect("root");
        let key = BackfillHandleKey::for_tests("install-a");
        let root_fingerprint = key.handle(
            "root",
            "dailyos.workspace_backfill.workspace_root_fingerprint.v1",
            &[&workspace_root.to_string_lossy()],
        );
        let empty = BTreeMap::new();
        create_or_resume_run(
            db.conn_ref(),
            "run-1",
            BackfillMode::Apply,
            &root_fingerprint,
            &empty,
            &empty,
            &empty,
        )
        .expect("run");
        let old_scan = scan_workspace(
            db.conn_ref(),
            &workspace_root,
            "run-1",
            &root_fingerprint,
            &key,
            DEFAULT_MAX_FILE_BYTES,
        )
        .expect("old scan");
        let old_hash = old_scan.candidates[0].content_sha256.clone();
        record_failed_item(
            &db,
            "run-1",
            &old_scan.candidates[0],
            "source_changed_during_backfill",
        )
        .expect("failed item");
        complete_run(db.conn_ref(), "run-1", "failed", &empty, &empty, &empty).expect("failed run");
        std::fs::write(&file_path, "new content").expect("new file");
        let new_hash = hex::encode(Sha256::digest(b"new content"));
        assert_ne!(old_hash, new_hash);
        let services = service_context();
        let propagation = default_engine();
        let emitter = WorkspaceSignalEmitter;

        run_workspace_backfill(
            &db,
            WorkspaceBackfillOptions::apply(
                Some(temp.path().to_path_buf()),
                Some("run-1".to_string()),
            ),
            &key,
            Some(runtime(&services, &propagation, &emitter)),
        )
        .expect("resume");

        let (stored_hash, status): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT content_sha256, status FROM workspace_backfill_items
                 WHERE run_id = 'run-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("resumed item");
        assert_eq!(stored_hash, new_hash);
        assert_ne!(stored_hash, old_hash);
        assert_eq!(status, "applied");
    }
}

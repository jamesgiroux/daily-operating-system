use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::db::{ActionDb, DbAccount, DbPerson, DbProject};
use crate::services::context::ServiceContext;
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityArchiveType {
    Account,
    Project,
    Person,
}

impl EntityArchiveType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Project => "project",
            Self::Person => "person",
        }
    }

    fn from_db(value: &str) -> Result<Self, String> {
        match value {
            "account" => Ok(Self::Account),
            "project" => Ok(Self::Project),
            "person" => Ok(Self::Person),
            _ => Err(format!("unknown entity archive type: {value}")),
        }
    }

    fn active_root(self) -> &'static str {
        match self {
            Self::Account => "Accounts",
            Self::Project => "Projects",
            Self::Person => "People",
        }
    }

    fn archive_root(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Project => "project",
            Self::Person => "person",
        }
    }
}

#[derive(Debug, Clone)]
struct EntityDescriptor {
    entity_type: EntityArchiveType,
    id: String,
    name: String,
    tracker_path: Option<String>,
    parent_id: Option<String>,
    updated_at: String,
    archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkArchivePreview {
    pub entity_type: EntityArchiveType,
    pub requested_ids: Vec<String>,
    pub root_ids: Vec<String>,
    pub changed_ids: Vec<String>,
    pub selected_ids: Vec<String>,
    pub cascaded_child_ids: Vec<String>,
    pub covered_child_ids: Vec<String>,
    pub not_found_ids: Vec<String>,
    pub already_archived_ids: Vec<String>,
    pub total_changed_count: usize,
    pub direct_cascade_count: usize,
    pub plan_id: String,
    pub plan_fingerprint: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveMutationOutcome {
    pub requested_id: String,
    pub entity_type: EntityArchiveType,
    pub changed_ids: Vec<String>,
    pub selected_id_changed: bool,
    pub cascaded_child_ids: Vec<String>,
    pub already_in_target_state_ids: Vec<String>,
    pub not_found_ids: Vec<String>,
    pub intel_queue_cleanup_ids: Vec<String>,
    pub operation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkArchiveItemResult {
    pub entity_type: EntityArchiveType,
    pub entity_id: String,
    pub folder_status: String,
    pub message: Option<String>,
    pub original_relative_path: Option<String>,
    pub archived_relative_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkArchiveResult {
    pub status: String,
    pub preview: BulkArchivePreview,
    pub outcomes: Vec<ArchiveMutationOutcome>,
    pub item_results: Vec<BulkArchiveItemResult>,
    pub changed_ids: Vec<String>,
    pub already_archived_ids: Vec<String>,
    pub not_found_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveFolderRepairItem {
    pub entity_type: EntityArchiveType,
    pub entity_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveFolderRepairPlan {
    pub archived_db_active_folder_count: usize,
    pub pending_metadata_count: usize,
    pub orphan_active_folder_count: usize,
    pub items: Vec<ArchiveFolderRepairItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveFolderRepairResult {
    pub status: String,
    pub plan: ArchiveFolderRepairPlan,
    pub changed_count: usize,
    pub item_results: Vec<BulkArchiveItemResult>,
}

#[derive(Debug, Clone)]
struct FolderMetadata {
    operation_id: String,
    entity_type: EntityArchiveType,
    entity_id: String,
    original_relative_path: String,
    archived_relative_path: Option<String>,
    folder_state: String,
}

#[derive(Debug, Clone)]
struct ArchivedDescendantFolderMove {
    descriptor: EntityDescriptor,
    metadata: FolderMetadata,
    current_relative_path: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveManifest<'a> {
    entity_type: &'a str,
    entity_id: &'a str,
    original_relative_path: &'a str,
    archived_relative_path: &'a str,
    archived_at: &'a str,
    operation_id: &'a str,
}

pub fn preview_bulk_archive(
    db: &ActionDb,
    entity_type: EntityArchiveType,
    ids: Vec<String>,
) -> Result<BulkArchivePreview, String> {
    let requested_ids = normalize_ids(ids);
    let mut selected_rows = Vec::new();
    let mut not_found_ids = Vec::new();
    let mut already_archived_ids = Vec::new();

    for id in &requested_ids {
        match load_descriptor(db, entity_type, id)? {
            Some(row) if row.archived => already_archived_ids.push(id.clone()),
            Some(row) => selected_rows.push(row),
            None => not_found_ids.push(id.clone()),
        }
    }

    let selected_set: BTreeSet<String> = selected_rows.iter().map(|row| row.id.clone()).collect();
    let mut covered_child_ids = Vec::new();
    for row in &selected_rows {
        if has_selected_ancestor(db, entity_type, row, &selected_set)? {
            covered_child_ids.push(row.id.clone());
        }
    }
    let covered_set: BTreeSet<String> = covered_child_ids.iter().cloned().collect();

    let root_rows: Vec<EntityDescriptor> = selected_rows
        .iter()
        .filter(|row| !covered_set.contains(&row.id))
        .cloned()
        .collect();
    let root_ids: Vec<String> = root_rows.iter().map(|row| row.id.clone()).collect();

    let mut cascade_rows = Vec::new();
    for row in &root_rows {
        cascade_rows.extend(load_active_descendants(db, entity_type, &row.id)?);
    }
    let cascaded_child_ids = dedupe_strings(cascade_rows.iter().map(|row| row.id.clone()));

    let mut changed_ids = root_ids.clone();
    changed_ids.extend(cascaded_child_ids.clone());
    changed_ids = dedupe_strings(changed_ids);

    let mut fingerprint_rows = BTreeMap::new();
    for row in selected_rows.iter().chain(cascade_rows.iter()) {
        fingerprint_rows.insert(
            row.id.clone(),
            serde_json::json!({
                "archived": row.archived,
                "parentId": row.parent_id,
                "updatedAt": row.updated_at,
            }),
        );
    }

    let fingerprint_payload = serde_json::json!({
        "entityType": entity_type.as_str(),
        "requestedIds": requested_ids,
        "rootIds": root_ids,
        "changedIds": changed_ids,
        "cascadedChildIds": cascaded_child_ids,
        "coveredChildIds": covered_child_ids,
        "notFoundIds": not_found_ids,
        "alreadyArchivedIds": already_archived_ids,
        "rows": fingerprint_rows,
    });
    let serialized = serde_json::to_vec(&fingerprint_payload)
        .map_err(|e| format!("preview fingerprint serialization failed: {e}"))?;
    let digest = hex::encode(Sha256::digest(&serialized));
    let plan_fingerprint = format!("bulk-archive:v1:{digest}");
    let plan_id = format!("bulk-archive-{}", &digest[..16]);
    let expires_at = (Utc::now() + Duration::minutes(10)).to_rfc3339();

    let selected_ids: Vec<String> = selected_rows.iter().map(|row| row.id.clone()).collect();
    Ok(BulkArchivePreview {
        entity_type,
        requested_ids,
        root_ids,
        changed_ids: changed_ids.clone(),
        selected_ids,
        cascaded_child_ids: cascaded_child_ids.clone(),
        covered_child_ids,
        not_found_ids,
        already_archived_ids,
        total_changed_count: changed_ids.len(),
        direct_cascade_count: cascaded_child_ids.len(),
        plan_id,
        plan_fingerprint,
        expires_at,
    })
}

pub fn execute_bulk_archive(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    entity_type: EntityArchiveType,
    ids: Vec<String>,
    plan_id: &str,
    plan_fingerprint: &str,
) -> Result<(BulkArchivePreview, Vec<ArchiveMutationOutcome>), String> {
    let preview = preview_bulk_archive(db, entity_type, ids)?;
    if preview.plan_id != plan_id || preview.plan_fingerprint != plan_fingerprint {
        return Ok((preview, Vec::new()));
    }

    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let root_ids = preview.root_ids.clone();
    let outcomes = db.with_transaction(|tx| {
        let mut outcomes = Vec::new();
        for root_id in &root_ids {
            outcomes.push(archive_entity_with_outcome_in_tx(
                ctx,
                tx,
                state,
                entity_type,
                root_id,
                true,
                archive_operation_id(entity_type, true),
            )?);
        }
        Ok(outcomes)
    })?;
    Ok((preview, outcomes))
}

fn archive_operation_id(entity_type: EntityArchiveType, archived: bool) -> String {
    format!(
        "{}-{}-{}",
        if archived { "archive" } else { "restore" },
        entity_type.as_str(),
        uuid::Uuid::new_v4()
    )
}

fn has_selected_ancestor(
    db: &ActionDb,
    entity_type: EntityArchiveType,
    row: &EntityDescriptor,
    selected_set: &BTreeSet<String>,
) -> Result<bool, String> {
    let mut parent_id = row.parent_id.clone();
    while let Some(parent) = parent_id {
        if selected_set.contains(&parent) {
            return Ok(true);
        }
        parent_id = load_descriptor(db, entity_type, &parent)?.and_then(|parent| parent.parent_id);
    }
    Ok(false)
}

fn apply_lifecycle_archive_state(
    db: &ActionDb,
    entity_type: EntityArchiveType,
    root_id: &str,
    descendant_ids: &[String],
    archived: bool,
) -> Result<(), String> {
    match entity_type {
        EntityArchiveType::Account => {
            db.archive_account(root_id, archived)
                .map_err(|e| e.to_string())?;
            for descendant_id in descendant_ids {
                db.archive_account(descendant_id, archived)
                    .map_err(|e| e.to_string())?;
            }
        }
        EntityArchiveType::Project => {
            db.archive_project(root_id, archived)
                .map_err(|e| e.to_string())?;
            for descendant_id in descendant_ids {
                db.archive_project(descendant_id, archived)
                    .map_err(|e| e.to_string())?;
            }
        }
        EntityArchiveType::Person => {
            db.archive_person(root_id, archived)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn archive_entity_with_outcome(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    entity_type: EntityArchiveType,
    id: &str,
    archived: bool,
) -> Result<ArchiveMutationOutcome, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let operation_id = archive_operation_id(entity_type, archived);
    db.with_transaction(|tx| {
        archive_entity_with_outcome_in_tx(ctx, tx, state, entity_type, id, archived, operation_id)
    })
}

fn archive_entity_with_outcome_in_tx(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    entity_type: EntityArchiveType,
    id: &str,
    archived: bool,
    operation_id: String,
) -> Result<ArchiveMutationOutcome, String> {
    let Some(row) = load_descriptor(db, entity_type, id)? else {
        return Ok(ArchiveMutationOutcome {
            requested_id: id.to_string(),
            entity_type,
            changed_ids: Vec::new(),
            selected_id_changed: false,
            cascaded_child_ids: Vec::new(),
            already_in_target_state_ids: Vec::new(),
            not_found_ids: vec![id.to_string()],
            intel_queue_cleanup_ids: Vec::new(),
            operation_id,
        });
    };

    if row.archived == archived {
        return Ok(ArchiveMutationOutcome {
            requested_id: id.to_string(),
            entity_type,
            changed_ids: Vec::new(),
            selected_id_changed: false,
            cascaded_child_ids: Vec::new(),
            already_in_target_state_ids: vec![id.to_string()],
            not_found_ids: Vec::new(),
            intel_queue_cleanup_ids: Vec::new(),
            operation_id,
        });
    }

    let cascaded_child_ids: Vec<String> = if archived {
        load_active_descendants(db, entity_type, id)?
            .into_iter()
            .map(|row| row.id)
            .collect()
    } else {
        Vec::new()
    };
    let mut changed_ids = vec![id.to_string()];
    changed_ids.extend(cascaded_child_ids.clone());
    changed_ids = dedupe_strings(changed_ids);
    let signal_type = if archived {
        "entity_archived"
    } else {
        "entity_restored"
    };

    apply_lifecycle_archive_state(db, entity_type, id, &cascaded_child_ids, archived)?;
    if !archived {
        record_restore_intent_for_changed_ids(db, entity_type, &changed_ids)?;
    }
    for changed_id in &changed_ids {
        crate::services::signals::emit_and_propagate(
            ctx,
            db,
            &state.signals.engine,
            entity_type.as_str(),
            changed_id,
            signal_type,
            "user_action",
            None,
            0.9,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
    }

    Ok(ArchiveMutationOutcome {
        requested_id: id.to_string(),
        entity_type,
        changed_ids: changed_ids.clone(),
        selected_id_changed: true,
        cascaded_child_ids,
        already_in_target_state_ids: Vec::new(),
        not_found_ids: Vec::new(),
        intel_queue_cleanup_ids: if archived { changed_ids } else { Vec::new() },
        operation_id,
    })
}

pub fn restore_account_with_outcome(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
    restore_children: bool,
) -> Result<ArchiveMutationOutcome, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let operation_id = format!("restore-account-{}", uuid::Uuid::new_v4());
    let Some(row) = load_descriptor(db, EntityArchiveType::Account, account_id)? else {
        return Ok(ArchiveMutationOutcome {
            requested_id: account_id.to_string(),
            entity_type: EntityArchiveType::Account,
            changed_ids: Vec::new(),
            selected_id_changed: false,
            cascaded_child_ids: Vec::new(),
            already_in_target_state_ids: Vec::new(),
            not_found_ids: vec![account_id.to_string()],
            intel_queue_cleanup_ids: Vec::new(),
            operation_id,
        });
    };
    let child_ids = if restore_children {
        load_archived_descendants(db, EntityArchiveType::Account, account_id)?
            .into_iter()
            .map(|row| row.id)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if !row.archived && child_ids.is_empty() {
        return Ok(ArchiveMutationOutcome {
            requested_id: account_id.to_string(),
            entity_type: EntityArchiveType::Account,
            changed_ids: Vec::new(),
            selected_id_changed: false,
            cascaded_child_ids: Vec::new(),
            already_in_target_state_ids: vec![account_id.to_string()],
            not_found_ids: Vec::new(),
            intel_queue_cleanup_ids: Vec::new(),
            operation_id,
        });
    }

    let mut changed_ids = if row.archived {
        vec![account_id.to_string()]
    } else {
        Vec::new()
    };
    changed_ids.extend(child_ids.clone());
    changed_ids = dedupe_strings(changed_ids);

    db.with_transaction(|tx| {
        tx.restore_account(account_id, restore_children)
            .map_err(|e| e.to_string())?;
        for child_id in &child_ids {
            tx.archive_account(child_id, false)
                .map_err(|e| e.to_string())?;
        }
        record_restore_intent_for_changed_ids(tx, EntityArchiveType::Account, &changed_ids)?;
        for changed_id in &changed_ids {
            crate::services::signals::emit_and_propagate(
                ctx,
                tx,
                &state.signals.engine,
                "account",
                changed_id,
                "entity_restored",
                "user_action",
                None,
                0.9,
            )
            .map_err(|e| format!("signal emit failed: {e}"))?;
        }
        Ok(())
    })?;

    Ok(ArchiveMutationOutcome {
        requested_id: account_id.to_string(),
        entity_type: EntityArchiveType::Account,
        changed_ids,
        selected_id_changed: row.archived,
        cascaded_child_ids: child_ids,
        already_in_target_state_ids: if row.archived {
            Vec::new()
        } else {
            vec![account_id.to_string()]
        },
        not_found_ids: Vec::new(),
        intel_queue_cleanup_ids: Vec::new(),
        operation_id,
    })
}

pub fn restore_account_target_ids(
    db: &ActionDb,
    account_id: &str,
    restore_children: bool,
) -> Result<Vec<String>, String> {
    let mut ids = vec![account_id.to_string()];
    if restore_children {
        ids.extend(
            load_archived_descendants(db, EntityArchiveType::Account, account_id)?
                .into_iter()
                .map(|row| row.id),
        );
    }
    Ok(dedupe_strings(ids))
}

pub fn remove_intel_queue_entries(state: &AppState, outcomes: &[ArchiveMutationOutcome]) {
    let mut ids = BTreeSet::new();
    for outcome in outcomes {
        for id in &outcome.intel_queue_cleanup_ids {
            ids.insert(id.clone());
        }
    }
    for id in ids {
        state.intel_queue.remove_by_entity_id(&id);
    }
}

pub async fn archive_folders_for_outcomes(
    state: Arc<AppState>,
    outcomes: &[ArchiveMutationOutcome],
) -> Vec<BulkArchiveItemResult> {
    let mut results = Vec::new();
    for outcome in outcomes {
        let (move_ids, skipped_ids) =
            match folder_move_targets_for_outcome(state.clone(), outcome).await {
                Ok(targets) => targets,
                Err(message) => {
                    results.push(BulkArchiveItemResult {
                        entity_type: outcome.entity_type,
                        entity_id: outcome.requested_id.clone(),
                        folder_status: "folder_failed".to_string(),
                        message: Some(message),
                        original_relative_path: None,
                        archived_relative_path: None,
                    });
                    continue;
                }
            };
        let mut outcome_results = Vec::new();
        for entity_id in move_ids {
            outcome_results.push(
                archive_folder_for_entity(
                    state.clone(),
                    outcome.entity_type,
                    entity_id,
                    outcome.operation_id.clone(),
                )
                .await,
            );
        }
        for entity_id in skipped_ids {
            outcome_results.push(
                record_skipped_descendant_metadata(
                    state.clone(),
                    outcome,
                    entity_id,
                    &outcome_results,
                )
                .await,
            );
        }
        results.extend(outcome_results);
    }
    results
}

async fn folder_move_targets_for_outcome(
    state: Arc<AppState>,
    outcome: &ArchiveMutationOutcome,
) -> Result<(Vec<String>, BTreeSet<String>), String> {
    let workspace = resolved_workspace_root(&state)?;
    let entity_type = outcome.entity_type;
    let changed_ids = outcome.changed_ids.clone();
    state
        .db_read(move |db| {
            let mut descriptors = Vec::new();
            for entity_id in &changed_ids {
                if let Some(descriptor) = load_descriptor(db, entity_type, entity_id)? {
                    descriptors.push(descriptor);
                }
            }
            partition_folder_move_targets(descriptors, Some(&workspace))
        })
        .await
        .map_err(String::from)
}

pub async fn preflight_restore_targets(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    ids: Vec<String>,
) -> Result<(), String> {
    let workspace = resolved_workspace_root(&state)?;
    for id in ids {
        let Some(metadata) =
            latest_archived_metadata(state.clone(), entity_type, id.clone()).await?
        else {
            continue;
        };
        let original_relative_path = parse_relative_path(&metadata.original_relative_path)?;
        let active_existing =
            validated_existing_source(&workspace, entity_type, &original_relative_path)?;
        if active_existing.is_some()
            && archived_source_exists(&workspace, &metadata.archived_relative_path)?
        {
            return Err("restore_conflict: active target already exists".to_string());
        }
    }
    Ok(())
}

pub async fn restore_folders_for_outcomes(
    state: Arc<AppState>,
    outcomes: &[ArchiveMutationOutcome],
) -> Vec<BulkArchiveItemResult> {
    let mut results = Vec::new();
    for outcome in outcomes {
        for entity_id in &outcome.changed_ids {
            results.push(
                restore_folder_for_entity(state.clone(), outcome.entity_type, entity_id.clone())
                    .await,
            );
        }
    }
    results
}

pub fn snapshot_internal_tracker_paths(db: &ActionDb) -> Result<Vec<String>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT tracker_path FROM accounts
             WHERE tracker_path IS NOT NULL
               AND (tracker_path = 'Internal' OR tracker_path LIKE 'Internal/%')",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn delete_mode_scoped_entity_roots(
    workspace: &Path,
    internal_tracker_paths: &[String],
) -> Result<(), String> {
    let workspace = ensure_canonical_workspace(workspace)?;
    for root in ["Accounts", "Projects", "People"] {
        remove_validated_root(&workspace, Path::new(root))?;
    }
    let archive_entities_root = PathBuf::from("_archive").join("entities");
    remove_validated_root(&workspace, &archive_entities_root)?;
    for tracker_path in internal_tracker_paths {
        let relative = parse_relative_path(tracker_path)?;
        if first_component(&relative).as_deref() == Some("Internal") {
            remove_validated_root(&workspace, &relative)?;
        }
    }
    Ok(())
}

pub async fn plan_entity_archive_folder_reconciliation(
    state: Arc<AppState>,
) -> Result<ArchiveFolderRepairPlan, String> {
    let workspace = resolved_workspace_root(&state)?;
    state
        .db_read(move |db| plan_reconciliation_sync(db, &workspace))
        .await
        .map_err(String::from)
}

fn plan_reconciliation_sync(
    db: &ActionDb,
    workspace: &Path,
) -> Result<ArchiveFolderRepairPlan, String> {
    let mut plan = ArchiveFolderRepairPlan::default();
    for entity_type in [
        EntityArchiveType::Account,
        EntityArchiveType::Project,
        EntityArchiveType::Person,
    ] {
        for row in load_archived_descriptors(db, entity_type)? {
            if let Ok(relative) = active_relative_path(&row, Some(workspace)) {
                if workspace.join(relative).exists() {
                    plan.archived_db_active_folder_count += 1;
                    plan.items.push(ArchiveFolderRepairItem {
                        entity_type,
                        entity_id: row.id,
                        status: "archived_db_active_folder".to_string(),
                    });
                }
            }
        }
    }

    let repair_metadata = load_repair_metadata_rows(db)?;
    plan.pending_metadata_count = repair_metadata.len();
    for row in repair_metadata {
        plan.items.push(ArchiveFolderRepairItem {
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            status: row.folder_state,
        });
    }
    plan.orphan_active_folder_count = count_orphan_active_folders(db, workspace)?;
    Ok(plan)
}

pub async fn apply_entity_archive_folder_reconciliation(
    state: Arc<AppState>,
    confirmed_statuses: Vec<String>,
    max_changes: usize,
) -> Result<ArchiveFolderRepairResult, String> {
    if max_changes == 0 {
        return Err("repair budget must be greater than zero".to_string());
    }
    let confirmed_statuses = confirmed_statuses
        .into_iter()
        .map(|status| status.trim().to_string())
        .filter(|status| !status.is_empty())
        .collect::<BTreeSet<_>>();
    if confirmed_statuses.is_empty() {
        return Err("at least one repair status must be confirmed".to_string());
    }

    let plan = plan_entity_archive_folder_reconciliation(state.clone()).await?;
    let mut candidates = plan
        .items
        .iter()
        .filter(|item| confirmed_statuses.contains(&item.status))
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        (
            left.status.as_str(),
            left.entity_type,
            left.entity_id.as_str(),
        )
            .cmp(&(
                right.status.as_str(),
                right.entity_type,
                right.entity_id.as_str(),
            ))
    });
    candidates.dedup_by(|left, right| {
        left.status == right.status
            && left.entity_type == right.entity_type
            && left.entity_id == right.entity_id
    });

    if candidates.len() > max_changes {
        return Err(format!(
            "repair_budget_exceeded: {} candidate repairs exceed max_changes {max_changes}",
            candidates.len()
        ));
    }

    let mut item_results = Vec::new();
    for item in candidates {
        item_results.push(apply_repair_item(state.clone(), item).await);
    }
    let has_failure = item_results
        .iter()
        .any(|item| !matches!(item.folder_status.as_str(), "succeeded" | "folder_skipped"));
    let status = if item_results.is_empty() {
        "noop"
    } else if has_failure {
        "partial"
    } else {
        "succeeded"
    };

    Ok(ArchiveFolderRepairResult {
        status: status.to_string(),
        plan,
        changed_count: item_results.len(),
        item_results,
    })
}

async fn apply_repair_item(
    state: Arc<AppState>,
    item: ArchiveFolderRepairItem,
) -> BulkArchiveItemResult {
    match item.status.as_str() {
        "archived_db_active_folder" => {
            let operation_id = format!(
                "repair-archive-{}-{}",
                item.entity_type.as_str(),
                uuid::Uuid::new_v4()
            );
            archive_folder_for_entity(state, item.entity_type, item.entity_id, operation_id).await
        }
        "archive_pending" | "archive_failed" => {
            repair_archive_metadata_row(state, item.entity_type, item.entity_id, item.status).await
        }
        "restore_pending" | "restore_failed" => {
            repair_restore_metadata_row(state, item.entity_type, item.entity_id, item.status).await
        }
        _ => BulkArchiveItemResult {
            entity_type: item.entity_type,
            entity_id: item.entity_id,
            folder_status: "folder_skipped".to_string(),
            message: Some("repair status not supported".to_string()),
            original_relative_path: None,
            archived_relative_path: None,
        },
    }
}

async fn repair_archive_metadata_row(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    entity_id: String,
    folder_state: String,
) -> BulkArchiveItemResult {
    match repair_archive_metadata_row_inner(state, entity_type, &entity_id, &folder_state).await {
        Ok(result) => result,
        Err(message) => BulkArchiveItemResult {
            entity_type,
            entity_id,
            folder_status: "folder_failed".to_string(),
            message: Some(message),
            original_relative_path: None,
            archived_relative_path: None,
        },
    }
}

async fn repair_archive_metadata_row_inner(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    entity_id: &str,
    folder_state: &str,
) -> Result<BulkArchiveItemResult, String> {
    let metadata = latest_repair_metadata(
        state.clone(),
        entity_type,
        entity_id.to_string(),
        vec![folder_state],
    )
    .await?
    .ok_or_else(|| "repair metadata no longer exists".to_string())?;
    let archived_relative_path = metadata
        .archived_relative_path
        .clone()
        .ok_or_else(|| "archive repair metadata has no target".to_string())?;
    let workspace = resolved_workspace_root(&state)?;
    let original_relative_path = parse_relative_path(&metadata.original_relative_path)?;
    let original_relative_path_string = path_to_string(&original_relative_path);

    let archived_source = validated_existing_archive_source(&workspace, &archived_relative_path)?;
    let active_source =
        validated_existing_source(&workspace, entity_type, &original_relative_path)?;

    if archived_source.is_some() && active_source.is_some() {
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "folder_failed".to_string(),
            message: Some(
                "archive repair conflict: source and archive target both exist".to_string(),
            ),
            original_relative_path: Some(original_relative_path_string),
            archived_relative_path: Some(archived_relative_path),
        });
    }

    if let Some(archive_path) = archived_source {
        write_archive_manifest_for_metadata(&archive_path, &metadata)?;
        update_folder_state(
            state,
            &metadata.operation_id,
            entity_type,
            entity_id,
            "archived",
            None,
        )
        .await?;
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "succeeded".to_string(),
            message: None,
            original_relative_path: Some(original_relative_path_string),
            archived_relative_path: Some(archived_relative_path),
        });
    }

    let Some(source_path) = active_source else {
        update_folder_state(
            state,
            &metadata.operation_id,
            entity_type,
            entity_id,
            "archive_failed",
            Some("missing_source"),
        )
        .await?;
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "folder_missing".to_string(),
            message: Some("archive source and target are both missing".to_string()),
            original_relative_path: Some(original_relative_path_string),
            archived_relative_path: Some(archived_relative_path),
        });
    };

    let archive_path = archive_target_from_metadata(&workspace, &archived_relative_path)?;
    move_directory_no_overwrite(&source_path, &archive_path)?;
    write_archive_manifest_for_metadata(&archive_path, &metadata)?;
    update_folder_state(
        state,
        &metadata.operation_id,
        entity_type,
        entity_id,
        "archived",
        None,
    )
    .await?;
    Ok(BulkArchiveItemResult {
        entity_type,
        entity_id: entity_id.to_string(),
        folder_status: "succeeded".to_string(),
        message: None,
        original_relative_path: Some(original_relative_path_string),
        archived_relative_path: Some(archived_relative_path),
    })
}

async fn repair_restore_metadata_row(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    entity_id: String,
    folder_state: String,
) -> BulkArchiveItemResult {
    match repair_restore_metadata_row_inner(state, entity_type, &entity_id, &folder_state).await {
        Ok(result) => result,
        Err(message) => BulkArchiveItemResult {
            entity_type,
            entity_id,
            folder_status: "folder_failed".to_string(),
            message: Some(message),
            original_relative_path: None,
            archived_relative_path: None,
        },
    }
}

async fn repair_restore_metadata_row_inner(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    entity_id: &str,
    folder_state: &str,
) -> Result<BulkArchiveItemResult, String> {
    let metadata = latest_repair_metadata(
        state.clone(),
        entity_type,
        entity_id.to_string(),
        vec![folder_state],
    )
    .await?
    .ok_or_else(|| "repair metadata no longer exists".to_string())?;
    let archived_relative_path = metadata
        .archived_relative_path
        .clone()
        .ok_or_else(|| "restore repair metadata has no archive source".to_string())?;
    let workspace = resolved_workspace_root(&state)?;
    let original_relative_path = parse_relative_path(&metadata.original_relative_path)?;
    let original_relative_path_string = path_to_string(&original_relative_path);
    let active_existing =
        validated_existing_source(&workspace, entity_type, &original_relative_path)?;
    let archived_source = validated_existing_archive_source(&workspace, &archived_relative_path)?;

    if active_existing.is_some() {
        if archived_source.is_some() {
            return Ok(BulkArchiveItemResult {
                entity_type,
                entity_id: entity_id.to_string(),
                folder_status: "restore_conflict".to_string(),
                message: Some(
                    "restore repair conflict: active target and archive source both exist"
                        .to_string(),
                ),
                original_relative_path: Some(original_relative_path_string),
                archived_relative_path: Some(archived_relative_path),
            });
        }
        update_folder_restored(state, &metadata.operation_id, entity_type, entity_id).await?;
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "succeeded".to_string(),
            message: None,
            original_relative_path: Some(original_relative_path_string),
            archived_relative_path: Some(archived_relative_path),
        });
    }

    let Some(archived_source) = archived_source else {
        update_folder_state(
            state,
            &metadata.operation_id,
            entity_type,
            entity_id,
            "restore_failed",
            Some("missing_source"),
        )
        .await?;
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "folder_missing".to_string(),
            message: Some("restore source folder missing".to_string()),
            original_relative_path: Some(original_relative_path_string),
            archived_relative_path: Some(archived_relative_path),
        });
    };

    let Some(active_target) =
        validated_restore_target(&workspace, entity_type, &original_relative_path)?
    else {
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "restore_conflict".to_string(),
            message: Some("active target already exists".to_string()),
            original_relative_path: Some(original_relative_path_string),
            archived_relative_path: Some(archived_relative_path),
        });
    };
    move_directory_no_overwrite(&archived_source, &active_target)?;
    update_folder_restored(state, &metadata.operation_id, entity_type, entity_id).await?;
    Ok(BulkArchiveItemResult {
        entity_type,
        entity_id: entity_id.to_string(),
        folder_status: "succeeded".to_string(),
        message: None,
        original_relative_path: Some(original_relative_path_string),
        archived_relative_path: Some(archived_relative_path),
    })
}

async fn archive_folder_for_entity(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    entity_id: String,
    operation_id: String,
) -> BulkArchiveItemResult {
    match archive_folder_for_entity_inner(state, entity_type, &entity_id, &operation_id).await {
        Ok(result) => result,
        Err(message) => BulkArchiveItemResult {
            entity_type,
            entity_id,
            folder_status: "folder_failed".to_string(),
            message: Some(message),
            original_relative_path: None,
            archived_relative_path: None,
        },
    }
}

async fn archive_folder_for_entity_inner(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    entity_id: &str,
    operation_id: &str,
) -> Result<BulkArchiveItemResult, String> {
    let workspace = resolved_workspace_root(&state)?;
    let descriptor = load_descriptor_async(state.clone(), entity_type, entity_id.to_string())
        .await?
        .ok_or_else(|| "entity not found after archive mutation".to_string())?;
    let original_relative_path = active_relative_path(&descriptor, Some(&workspace))?;
    let source_path = validated_existing_source(&workspace, entity_type, &original_relative_path)?;
    let Some(source_path) = source_path else {
        upsert_folder_metadata(
            state,
            FolderMetadataWrite {
                operation_id: operation_id.to_string(),
                entity_type,
                entity_id: entity_id.to_string(),
                original_relative_path: path_to_string(&original_relative_path),
                archived_relative_path: None,
                folder_state: "missing_source".to_string(),
                archived_at: None,
                restored_at: None,
                last_error_code: Some("missing_source".to_string()),
            },
        )
        .await?;
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "missing_source".to_string(),
            message: None,
            original_relative_path: Some(path_to_string(&original_relative_path)),
            archived_relative_path: None,
        });
    };

    let archive_relative_path =
        allocate_archive_relative_path(&workspace, entity_type, entity_id, &descriptor.name)?;
    let archive_path = workspace.join(&archive_relative_path);
    let archived_at = Utc::now().to_rfc3339();

    let pending = FolderMetadataWrite {
        operation_id: operation_id.to_string(),
        entity_type,
        entity_id: entity_id.to_string(),
        original_relative_path: path_to_string(&original_relative_path),
        archived_relative_path: Some(path_to_string(&archive_relative_path)),
        folder_state: "archive_pending".to_string(),
        archived_at: Some(archived_at.clone()),
        restored_at: None,
        last_error_code: None,
    };
    upsert_folder_metadata(state.clone(), pending.clone()).await?;

    if let Err(error) = move_directory_no_overwrite(&source_path, &archive_path) {
        let message = if let Err(metadata_error) = upsert_folder_metadata(
            state,
            FolderMetadataWrite {
                folder_state: "archive_failed".to_string(),
                last_error_code: Some("move_failed".to_string()),
                ..pending
            },
        )
        .await
        {
            format!("{error}; metadata update failed: {metadata_error}")
        } else {
            error
        };
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "folder_failed".to_string(),
            message: Some(message),
            original_relative_path: Some(path_to_string(&original_relative_path)),
            archived_relative_path: Some(path_to_string(&archive_relative_path)),
        });
    }

    let original_relative_path_string = path_to_string(&original_relative_path);
    let archive_relative_path_string = path_to_string(&archive_relative_path);
    let manifest = ArchiveManifest {
        entity_type: entity_type.as_str(),
        entity_id,
        original_relative_path: &original_relative_path_string,
        archived_relative_path: &archive_relative_path_string,
        archived_at: &archived_at,
        operation_id,
    };
    if let Err(error) = write_archive_manifest(&archive_path, &manifest) {
        let message = if let Err(metadata_error) = upsert_folder_metadata(
            state,
            FolderMetadataWrite {
                folder_state: "archive_failed".to_string(),
                last_error_code: Some("manifest_failed".to_string()),
                ..pending
            },
        )
        .await
        {
            format!("{error}; metadata update failed: {metadata_error}")
        } else {
            error
        };
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "folder_failed".to_string(),
            message: Some(message),
            original_relative_path: Some(path_to_string(&original_relative_path)),
            archived_relative_path: Some(path_to_string(&archive_relative_path)),
        });
    }

    match upsert_folder_metadata(
        state,
        FolderMetadataWrite {
            folder_state: "archived".to_string(),
            last_error_code: None,
            ..pending
        },
    )
    .await
    {
        Ok(()) => Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "succeeded".to_string(),
            message: None,
            original_relative_path: Some(path_to_string(&original_relative_path)),
            archived_relative_path: Some(path_to_string(&archive_relative_path)),
        }),
        Err(error) => Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "metadata_pending".to_string(),
            message: Some(error),
            original_relative_path: Some(path_to_string(&original_relative_path)),
            archived_relative_path: Some(path_to_string(&archive_relative_path)),
        }),
    }
}

async fn restore_folder_for_entity(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    entity_id: String,
) -> BulkArchiveItemResult {
    match restore_folder_for_entity_inner(state, entity_type, &entity_id).await {
        Ok(result) => result,
        Err(message) => BulkArchiveItemResult {
            entity_type,
            entity_id,
            folder_status: "folder_failed".to_string(),
            message: Some(message),
            original_relative_path: None,
            archived_relative_path: None,
        },
    }
}

async fn restore_folder_for_entity_inner(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    entity_id: &str,
) -> Result<BulkArchiveItemResult, String> {
    let workspace = resolved_workspace_root(&state)?;
    let Some(metadata) =
        latest_archived_metadata(state.clone(), entity_type, entity_id.to_string()).await?
    else {
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "folder_skipped".to_string(),
            message: Some("no archived folder metadata".to_string()),
            original_relative_path: None,
            archived_relative_path: None,
        });
    };
    let Some(archived_relative_path) = metadata.archived_relative_path.clone() else {
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "folder_skipped".to_string(),
            message: Some("archived folder metadata has no target".to_string()),
            original_relative_path: Some(metadata.original_relative_path),
            archived_relative_path: None,
        });
    };
    let original_relative_path = parse_relative_path(&metadata.original_relative_path)?;
    let original_relative_path_string = path_to_string(&original_relative_path);
    let archived_source = validated_existing_archive_source(&workspace, &archived_relative_path)?;
    let active_existing =
        validated_existing_source(&workspace, entity_type, &original_relative_path)?;

    if active_existing.is_some() {
        if archived_source.is_some() {
            return Ok(BulkArchiveItemResult {
                entity_type,
                entity_id: entity_id.to_string(),
                folder_status: "restore_conflict".to_string(),
                message: Some("active target already exists".to_string()),
                original_relative_path: Some(original_relative_path_string),
                archived_relative_path: Some(archived_relative_path),
            });
        }
        update_folder_restored(state, &metadata.operation_id, entity_type, entity_id).await?;
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "succeeded".to_string(),
            message: Some("active folder already restored".to_string()),
            original_relative_path: Some(original_relative_path_string),
            archived_relative_path: Some(archived_relative_path),
        });
    }

    let Some(active_target) =
        validated_restore_target(&workspace, entity_type, &original_relative_path)?
    else {
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "restore_conflict".to_string(),
            message: Some("active target already exists".to_string()),
            original_relative_path: Some(original_relative_path_string),
            archived_relative_path: Some(archived_relative_path),
        });
    };
    let Some(archived_source) = archived_source else {
        update_folder_state(
            state.clone(),
            &metadata.operation_id,
            entity_type,
            entity_id,
            "restore_failed",
            Some("missing_source"),
        )
        .await?;
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "folder_missing".to_string(),
            message: Some("archived source folder missing".to_string()),
            original_relative_path: Some(original_relative_path_string),
            archived_relative_path: Some(archived_relative_path),
        });
    };

    update_folder_state(
        state.clone(),
        &metadata.operation_id,
        entity_type,
        entity_id,
        "restore_pending",
        None,
    )
    .await?;

    if let Err(error) = preserve_archived_descendant_folders_for_parent_restore(
        state.clone(),
        entity_type,
        entity_id,
        &workspace,
        &original_relative_path,
        &archived_relative_path,
    )
    .await
    {
        let message = if let Err(metadata_error) = update_folder_state(
            state,
            &metadata.operation_id,
            entity_type,
            entity_id,
            "restore_failed",
            Some("descendant_preserve_failed"),
        )
        .await
        {
            format!("{error}; metadata update failed: {metadata_error}")
        } else {
            error
        };
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "folder_failed".to_string(),
            message: Some(message),
            original_relative_path: Some(original_relative_path_string),
            archived_relative_path: Some(archived_relative_path),
        });
    }

    if let Err(error) = move_directory_no_overwrite(&archived_source, &active_target) {
        let message = if let Err(metadata_error) = update_folder_state(
            state,
            &metadata.operation_id,
            entity_type,
            entity_id,
            "restore_failed",
            Some("move_failed"),
        )
        .await
        {
            format!("{error}; metadata update failed: {metadata_error}")
        } else {
            error
        };
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "folder_failed".to_string(),
            message: Some(message),
            original_relative_path: Some(original_relative_path_string),
            archived_relative_path: Some(archived_relative_path),
        });
    }

    update_folder_restored(state, &metadata.operation_id, entity_type, entity_id).await?;
    Ok(BulkArchiveItemResult {
        entity_type,
        entity_id: entity_id.to_string(),
        folder_status: "succeeded".to_string(),
        message: None,
        original_relative_path: Some(original_relative_path_string),
        archived_relative_path: Some(archived_relative_path),
    })
}

async fn record_skipped_descendant_metadata(
    state: Arc<AppState>,
    outcome: &ArchiveMutationOutcome,
    entity_id: String,
    ancestor_results: &[BulkArchiveItemResult],
) -> BulkArchiveItemResult {
    match record_skipped_descendant_metadata_inner(
        state,
        outcome.entity_type,
        &entity_id,
        &outcome.operation_id,
        ancestor_results,
    )
    .await
    {
        Ok(result) => result,
        Err(message) => BulkArchiveItemResult {
            entity_type: outcome.entity_type,
            entity_id,
            folder_status: "metadata_pending".to_string(),
            message: Some(message),
            original_relative_path: None,
            archived_relative_path: None,
        },
    }
}

async fn record_skipped_descendant_metadata_inner(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    entity_id: &str,
    operation_id: &str,
    ancestor_results: &[BulkArchiveItemResult],
) -> Result<BulkArchiveItemResult, String> {
    let workspace = resolved_workspace_root(&state)?;
    let descriptor = load_descriptor_async(state.clone(), entity_type, entity_id.to_string())
        .await?
        .ok_or_else(|| "descendant entity not found after archive mutation".to_string())?;
    let original_relative_path = active_relative_path(&descriptor, Some(&workspace))?;
    let original_relative_path_string = path_to_string(&original_relative_path);

    let Some((archived_relative_path, ancestor_failed)) =
        descendant_archive_relative_path(&original_relative_path, ancestor_results)?
    else {
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "folder_skipped".to_string(),
            message: Some("folder moved with archived ancestor".to_string()),
            original_relative_path: Some(original_relative_path_string),
            archived_relative_path: None,
        });
    };
    let archived_relative_path_string = path_to_string(&archived_relative_path);

    let active_existing =
        validated_existing_source(&workspace, entity_type, &original_relative_path)?;
    let archived_existing =
        validated_existing_archive_source(&workspace, &archived_relative_path_string)?;
    if active_existing.is_some() || archived_existing.is_none() {
        return Ok(BulkArchiveItemResult {
            entity_type,
            entity_id: entity_id.to_string(),
            folder_status: "folder_skipped".to_string(),
            message: Some("folder moved with archived ancestor".to_string()),
            original_relative_path: Some(original_relative_path_string),
            archived_relative_path: None,
        });
    }

    upsert_folder_metadata(
        state,
        FolderMetadataWrite {
            operation_id: operation_id.to_string(),
            entity_type,
            entity_id: entity_id.to_string(),
            original_relative_path: original_relative_path_string.clone(),
            archived_relative_path: Some(archived_relative_path_string.clone()),
            folder_state: if ancestor_failed {
                "archive_failed".to_string()
            } else {
                "archived".to_string()
            },
            archived_at: Some(Utc::now().to_rfc3339()),
            restored_at: None,
            last_error_code: ancestor_failed.then(|| "ancestor_archive_failed".to_string()),
        },
    )
    .await?;

    Ok(BulkArchiveItemResult {
        entity_type,
        entity_id: entity_id.to_string(),
        folder_status: "folder_skipped".to_string(),
        message: Some("folder moved with archived ancestor".to_string()),
        original_relative_path: Some(original_relative_path_string),
        archived_relative_path: Some(archived_relative_path_string),
    })
}

async fn preserve_archived_descendant_folders_for_parent_restore(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    entity_id: &str,
    workspace: &Path,
    original_relative_path: &Path,
    archived_relative_path: &str,
) -> Result<(), String> {
    if entity_type == EntityArchiveType::Person {
        return Ok(());
    }

    let parent_original_relative_path = original_relative_path.to_path_buf();
    let parent_archived_relative_path = parse_relative_path(archived_relative_path)?;
    let workspace_for_read = workspace.to_path_buf();
    let entity_id = entity_id.to_string();
    let mut moves = state
        .db_read(move |db| {
            let mut moves = Vec::new();
            for descriptor in load_archived_descendants(db, entity_type, &entity_id)? {
                let descendant_original_relative_path =
                    active_relative_path(&descriptor, Some(&workspace_for_read))?;
                if !descendant_original_relative_path.starts_with(&parent_original_relative_path) {
                    continue;
                }
                let Some(metadata) =
                    latest_archived_metadata_sync(db, entity_type, &descriptor.id)?
                else {
                    return Err(format!(
                        "archived descendant folder metadata missing: {}",
                        descriptor.id
                    ));
                };
                let Some(current_archived_relative_path) =
                    metadata.archived_relative_path.as_deref()
                else {
                    continue;
                };
                let current_relative_path = parse_relative_path(current_archived_relative_path)?;
                if current_relative_path.starts_with(&parent_archived_relative_path) {
                    moves.push(ArchivedDescendantFolderMove {
                        descriptor,
                        metadata,
                        current_relative_path,
                    });
                }
            }
            Ok(moves)
        })
        .await
        .map_err(String::from)?;

    moves.sort_by(|left, right| {
        right
            .current_relative_path
            .components()
            .count()
            .cmp(&left.current_relative_path.components().count())
            .then_with(|| left.descriptor.id.cmp(&right.descriptor.id))
    });

    for candidate in moves {
        let source_relative_path = path_to_string(&candidate.current_relative_path);
        let Some(source_path) =
            validated_existing_archive_source(workspace, &source_relative_path)?
        else {
            continue;
        };
        let target_relative_path = allocate_archive_relative_path(
            workspace,
            entity_type,
            &candidate.descriptor.id,
            &candidate.descriptor.name,
        )?;
        let target_path = workspace.join(&target_relative_path);
        move_directory_no_overwrite(&source_path, &target_path)?;

        let target_relative_path_string = path_to_string(&target_relative_path);
        if let Err(error) = update_archived_relative_path(
            state.clone(),
            &candidate.metadata.operation_id,
            entity_type,
            &candidate.metadata.entity_id,
            &target_relative_path_string,
        )
        .await
        {
            if let Err(rollback_error) = move_directory_no_overwrite(&target_path, &source_path) {
                return Err(format!(
                    "{error}; descendant archive rollback failed: {rollback_error}"
                ));
            }
            return Err(error);
        }
    }

    Ok(())
}

#[derive(Clone)]
struct FolderMetadataWrite {
    operation_id: String,
    entity_type: EntityArchiveType,
    entity_id: String,
    original_relative_path: String,
    archived_relative_path: Option<String>,
    folder_state: String,
    archived_at: Option<String>,
    restored_at: Option<String>,
    last_error_code: Option<String>,
}

async fn upsert_folder_metadata(
    state: Arc<AppState>,
    write: FolderMetadataWrite,
) -> Result<(), String> {
    state
        .db_write(move |db| {
            db.conn_ref()
                .execute(
                    "INSERT INTO entity_archive_folders (
                        operation_id, entity_type, entity_id, original_relative_path,
                        archived_relative_path, folder_state, archived_at, restored_at,
                        updated_at, last_error_code
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                     ON CONFLICT(operation_id, entity_type, entity_id) DO UPDATE SET
                        original_relative_path = excluded.original_relative_path,
                        archived_relative_path = excluded.archived_relative_path,
                        folder_state = excluded.folder_state,
                        archived_at = COALESCE(excluded.archived_at, entity_archive_folders.archived_at),
                        restored_at = excluded.restored_at,
                        updated_at = excluded.updated_at,
                        last_error_code = excluded.last_error_code",
                    params![
                        write.operation_id,
                        write.entity_type.as_str(),
                        write.entity_id,
                        write.original_relative_path,
                        write.archived_relative_path,
                        write.folder_state,
                        write.archived_at,
                        write.restored_at,
                        Utc::now().to_rfc3339(),
                        write.last_error_code,
                    ],
                )
                .map_err(|e| format!("folder metadata write failed: {e}"))?;
            Ok(())
        })
        .await
        .map_err(String::from)?;
    Ok(())
}

async fn update_archived_relative_path(
    state: Arc<AppState>,
    operation_id: &str,
    entity_type: EntityArchiveType,
    entity_id: &str,
    archived_relative_path: &str,
) -> Result<(), String> {
    let operation_id = operation_id.to_string();
    let entity_id = entity_id.to_string();
    let archived_relative_path = archived_relative_path.to_string();
    state
        .db_write(move |db| {
            db.conn_ref()
                .execute(
                    "UPDATE entity_archive_folders
                        SET archived_relative_path = ?1,
                            updated_at = ?2
                      WHERE operation_id = ?3
                        AND entity_type = ?4
                        AND entity_id = ?5",
                    params![
                        archived_relative_path,
                        Utc::now().to_rfc3339(),
                        operation_id,
                        entity_type.as_str(),
                        entity_id,
                    ],
                )
                .map_err(|e| format!("folder metadata archive path update failed: {e}"))?;
            Ok(())
        })
        .await
        .map_err(String::from)?;
    Ok(())
}

async fn update_folder_state(
    state: Arc<AppState>,
    operation_id: &str,
    entity_type: EntityArchiveType,
    entity_id: &str,
    folder_state: &str,
    last_error_code: Option<&str>,
) -> Result<(), String> {
    let operation_id = operation_id.to_string();
    let entity_id = entity_id.to_string();
    let folder_state = folder_state.to_string();
    let last_error_code = last_error_code.map(str::to_string);
    state
        .db_write(move |db| {
            db.conn_ref()
                .execute(
                    "UPDATE entity_archive_folders
                        SET folder_state = ?1,
                            updated_at = ?2,
                            last_error_code = ?3
                      WHERE operation_id = ?4
                        AND entity_type = ?5
                        AND entity_id = ?6",
                    params![
                        folder_state,
                        Utc::now().to_rfc3339(),
                        last_error_code,
                        operation_id,
                        entity_type.as_str(),
                        entity_id,
                    ],
                )
                .map_err(|e| format!("folder metadata update failed: {e}"))?;
            Ok(())
        })
        .await
        .map_err(String::from)?;
    Ok(())
}

async fn update_folder_restored(
    state: Arc<AppState>,
    operation_id: &str,
    entity_type: EntityArchiveType,
    entity_id: &str,
) -> Result<(), String> {
    let operation_id = operation_id.to_string();
    let entity_id = entity_id.to_string();
    state
        .db_write(move |db| {
            db.conn_ref()
                .execute(
                    "UPDATE entity_archive_folders
                        SET folder_state = 'restored',
                            restored_at = ?1,
                            updated_at = ?1,
                            last_error_code = NULL
                      WHERE operation_id = ?2
                        AND entity_type = ?3
                        AND entity_id = ?4",
                    params![
                        Utc::now().to_rfc3339(),
                        operation_id,
                        entity_type.as_str(),
                        entity_id
                    ],
                )
                .map_err(|e| format!("folder metadata restore update failed: {e}"))?;
            Ok(())
        })
        .await
        .map_err(String::from)?;
    Ok(())
}

async fn latest_archived_metadata(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    entity_id: String,
) -> Result<Option<FolderMetadata>, String> {
    state
        .db_read(move |db| latest_archived_metadata_sync(db, entity_type, &entity_id))
        .await
        .map_err(String::from)
}

fn latest_archived_metadata_sync(
    db: &ActionDb,
    entity_type: EntityArchiveType,
    entity_id: &str,
) -> Result<Option<FolderMetadata>, String> {
    db.conn_ref()
        .query_row(
            "SELECT operation_id, entity_type, entity_id, original_relative_path,
                    archived_relative_path, folder_state
               FROM entity_archive_folders
              WHERE entity_type = ?1
                AND entity_id = ?2
                AND folder_state IN (
                    'archived',
                    'restore_pending',
                    'restore_failed',
                    'archive_pending',
                    'archive_failed'
                )
                AND archived_relative_path IS NOT NULL
              ORDER BY updated_at DESC
              LIMIT 1",
            params![entity_type.as_str(), entity_id],
            |row| {
                Ok(FolderMetadata {
                    operation_id: row.get(0)?,
                    entity_type,
                    entity_id: row.get(2)?,
                    original_relative_path: row.get(3)?,
                    archived_relative_path: row.get(4)?,
                    folder_state: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("folder metadata read failed: {e}"))
}

fn archived_source_exists(
    workspace: &Path,
    archived_relative_path: &Option<String>,
) -> Result<bool, String> {
    let Some(archived_relative_path) = archived_relative_path else {
        return Ok(false);
    };
    validated_existing_archive_source(workspace, archived_relative_path)
        .map(|source| source.is_some())
}

fn record_restore_intent_for_changed_ids(
    db: &ActionDb,
    entity_type: EntityArchiveType,
    changed_ids: &[String],
) -> Result<(), String> {
    for entity_id in changed_ids {
        if let Some(metadata) = latest_archived_metadata_sync(db, entity_type, entity_id)? {
            update_folder_state_sync(
                db,
                &metadata.operation_id,
                entity_type,
                entity_id,
                "restore_pending",
                None,
            )?;
        }
    }
    Ok(())
}

fn update_folder_state_sync(
    db: &ActionDb,
    operation_id: &str,
    entity_type: EntityArchiveType,
    entity_id: &str,
    folder_state: &str,
    last_error_code: Option<&str>,
) -> Result<(), String> {
    db.conn_ref()
        .execute(
            "UPDATE entity_archive_folders
                SET folder_state = ?1,
                    updated_at = ?2,
                    last_error_code = ?3
              WHERE operation_id = ?4
                AND entity_type = ?5
                AND entity_id = ?6",
            params![
                folder_state,
                Utc::now().to_rfc3339(),
                last_error_code,
                operation_id,
                entity_type.as_str(),
                entity_id,
            ],
        )
        .map_err(|e| format!("folder metadata update failed: {e}"))?;
    Ok(())
}

async fn latest_repair_metadata(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    entity_id: String,
    folder_states: Vec<&str>,
) -> Result<Option<FolderMetadata>, String> {
    let states = folder_states
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    state
        .db_read(move |db| latest_repair_metadata_sync(db, entity_type, &entity_id, &states))
        .await
        .map_err(String::from)
}

fn latest_repair_metadata_sync(
    db: &ActionDb,
    entity_type: EntityArchiveType,
    entity_id: &str,
    folder_states: &[String],
) -> Result<Option<FolderMetadata>, String> {
    let rows = load_repair_metadata_rows(db)?;
    Ok(rows.into_iter().find(|row| {
        row.entity_type == entity_type
            && row.entity_id == entity_id
            && folder_states.iter().any(|state| state == &row.folder_state)
    }))
}

fn load_repair_metadata_rows(db: &ActionDb) -> Result<Vec<FolderMetadata>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT operation_id, entity_type, entity_id, original_relative_path,
                    archived_relative_path, folder_state
               FROM entity_archive_folders
              WHERE folder_state IN ('archive_pending','archive_failed','restore_pending','restore_failed')
              ORDER BY updated_at DESC, operation_id DESC",
        )
        .map_err(|e| format!("folder metadata repair read failed: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let entity_type_raw: String = row.get(1)?;
            let entity_type = EntityArchiveType::from_db(&entity_type_raw)
                .map_err(rusqlite::Error::InvalidParameterName)?;
            Ok(FolderMetadata {
                operation_id: row.get(0)?,
                entity_type,
                entity_id: row.get(2)?,
                original_relative_path: row.get(3)?,
                archived_relative_path: row.get(4)?,
                folder_state: row.get(5)?,
            })
        })
        .map_err(|e| format!("folder metadata repair read failed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("folder metadata repair row failed: {e}"))
}

async fn load_descriptor_async(
    state: Arc<AppState>,
    entity_type: EntityArchiveType,
    entity_id: String,
) -> Result<Option<EntityDescriptor>, String> {
    state
        .db_read(move |db| load_descriptor(db, entity_type, &entity_id))
        .await
        .map_err(String::from)
}

fn load_descriptor(
    db: &ActionDb,
    entity_type: EntityArchiveType,
    id: &str,
) -> Result<Option<EntityDescriptor>, String> {
    match entity_type {
        EntityArchiveType::Account => Ok(db
            .get_account(id)
            .map_err(|e| e.to_string())?
            .map(descriptor_from_account)),
        EntityArchiveType::Project => Ok(db
            .get_project(id)
            .map_err(|e| e.to_string())?
            .map(descriptor_from_project)),
        EntityArchiveType::Person => Ok(db
            .get_person(id)
            .map_err(|e| e.to_string())?
            .map(descriptor_from_person)),
    }
}

fn load_active_descendants(
    db: &ActionDb,
    entity_type: EntityArchiveType,
    parent_id: &str,
) -> Result<Vec<EntityDescriptor>, String> {
    match entity_type {
        EntityArchiveType::Account => query_descendant_accounts(db, parent_id, false),
        EntityArchiveType::Project => query_descendant_projects(db, parent_id, false),
        EntityArchiveType::Person => Ok(Vec::new()),
    }
}

fn load_archived_descendants(
    db: &ActionDb,
    entity_type: EntityArchiveType,
    parent_id: &str,
) -> Result<Vec<EntityDescriptor>, String> {
    match entity_type {
        EntityArchiveType::Account => query_descendant_accounts(db, parent_id, true),
        EntityArchiveType::Project => query_descendant_projects(db, parent_id, true),
        EntityArchiveType::Person => Ok(Vec::new()),
    }
}

fn load_archived_descriptors(
    db: &ActionDb,
    entity_type: EntityArchiveType,
) -> Result<Vec<EntityDescriptor>, String> {
    match entity_type {
        EntityArchiveType::Account => Ok(db
            .get_archived_accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(descriptor_from_account)
            .collect()),
        EntityArchiveType::Project => Ok(db
            .get_archived_projects()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(descriptor_from_project)
            .collect()),
        EntityArchiveType::Person => {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT id, email, name, organization, role, relationship, notes,
                            tracker_path, last_seen, first_seen, meeting_count, updated_at, archived,
                            linkedin_url, twitter_handle, phone, photo_url, bio, title_history,
                            company_industry, company_size, company_hq, last_enriched_at, enrichment_sources
                       FROM people WHERE archived = 1 ORDER BY name",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], ActionDb::map_person_row)
                .map_err(|e| e.to_string())?;
            let people = rows
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            Ok(people.into_iter().map(descriptor_from_person).collect())
        }
    }
}

fn partition_folder_move_targets(
    descriptors: Vec<EntityDescriptor>,
    workspace: Option<&Path>,
) -> Result<(Vec<String>, BTreeSet<String>), String> {
    let mut paths = Vec::new();
    for descriptor in descriptors {
        paths.push((
            descriptor.id.clone(),
            active_relative_path(&descriptor, workspace)?,
        ));
    }

    let mut move_ids = Vec::new();
    let mut skipped_ids = BTreeSet::new();
    for (entity_id, relative_path) in &paths {
        let nested_under_changed_ancestor = paths.iter().any(|(other_id, other_path)| {
            other_id != entity_id
                && relative_path != other_path
                && relative_path.starts_with(other_path)
        });
        if nested_under_changed_ancestor {
            skipped_ids.insert(entity_id.clone());
        } else {
            move_ids.push(entity_id.clone());
        }
    }
    Ok((move_ids, skipped_ids))
}

fn descendant_archive_relative_path(
    descendant_original_path: &Path,
    ancestor_results: &[BulkArchiveItemResult],
) -> Result<Option<(PathBuf, bool)>, String> {
    let mut best: Option<(usize, PathBuf, bool)> = None;
    for result in ancestor_results {
        let Some(original_relative_path) = result.original_relative_path.as_deref() else {
            continue;
        };
        let Some(archived_relative_path) = result.archived_relative_path.as_deref() else {
            continue;
        };
        let ancestor_original = parse_relative_path(original_relative_path)?;
        if descendant_original_path == ancestor_original
            || !descendant_original_path.starts_with(&ancestor_original)
        {
            continue;
        }
        let ancestor_archived = parse_relative_path(archived_relative_path)?;
        let suffix = descendant_original_path
            .strip_prefix(&ancestor_original)
            .map_err(|e| format!("descendant archive path derivation failed: {e}"))?;
        let candidate = ancestor_archived.join(suffix);
        let depth = ancestor_original.components().count();
        let ancestor_failed = result.folder_status == "folder_failed";
        if best
            .as_ref()
            .is_none_or(|(best_depth, _, _)| depth > *best_depth)
        {
            best = Some((depth, candidate, ancestor_failed));
        }
    }
    Ok(best.map(|(_, path, ancestor_failed)| (path, ancestor_failed)))
}

fn query_descendant_accounts(
    db: &ActionDb,
    parent_id: &str,
    archived: bool,
) -> Result<Vec<EntityDescriptor>, String> {
    let sql = format!(
        "WITH RECURSIVE descendants(id, depth) AS (
            SELECT id, 1 FROM accounts WHERE parent_id = ?1
            UNION ALL
            SELECT accounts.id, descendants.depth + 1
              FROM accounts
              JOIN descendants ON accounts.parent_id = descendants.id
             WHERE descendants.depth < 10
         )
         SELECT {} FROM accounts
          WHERE id IN (SELECT id FROM descendants)
            AND archived = ?2
          ORDER BY name",
        ActionDb::ACCOUNT_COLUMNS
    );
    let mut stmt = db.conn_ref().prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            params![parent_id, archived as i32],
            ActionDb::map_account_row,
        )
        .map_err(|e| e.to_string())?;
    let rows = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(descriptor_from_account).collect())
}

fn query_descendant_projects(
    db: &ActionDb,
    parent_id: &str,
    archived: bool,
) -> Result<Vec<EntityDescriptor>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "WITH RECURSIVE descendants(id, depth) AS (
                SELECT id, 1 FROM projects WHERE parent_id = ?1
                UNION ALL
                SELECT projects.id, descendants.depth + 1
                  FROM projects
                  JOIN descendants ON projects.parent_id = descendants.id
                 WHERE descendants.depth < 10
             )
             SELECT id, name, status, milestone, owner, target_date,
                    tracker_path, parent_id, updated_at, archived,
                    keywords, keywords_extracted_at, metadata,
                    description, milestones, notes
               FROM projects
              WHERE id IN (SELECT id FROM descendants)
                AND archived = ?2
              ORDER BY name",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            params![parent_id, archived as i32],
            ActionDb::map_project_row,
        )
        .map_err(|e| e.to_string())?;
    let rows = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(descriptor_from_project).collect())
}

fn descriptor_from_account(account: DbAccount) -> EntityDescriptor {
    EntityDescriptor {
        entity_type: EntityArchiveType::Account,
        id: account.id,
        name: account.name,
        tracker_path: account.tracker_path,
        parent_id: account.parent_id,
        updated_at: account.updated_at,
        archived: account.archived,
    }
}

fn descriptor_from_project(project: DbProject) -> EntityDescriptor {
    EntityDescriptor {
        entity_type: EntityArchiveType::Project,
        id: project.id,
        name: project.name,
        tracker_path: project.tracker_path,
        parent_id: project.parent_id,
        updated_at: project.updated_at,
        archived: project.archived,
    }
}

fn descriptor_from_person(person: DbPerson) -> EntityDescriptor {
    EntityDescriptor {
        entity_type: EntityArchiveType::Person,
        id: person.id,
        name: person.name,
        tracker_path: person.tracker_path,
        parent_id: None,
        updated_at: person.updated_at,
        archived: person.archived,
    }
}

fn count_orphan_active_folders(db: &ActionDb, workspace: &Path) -> Result<usize, String> {
    let workspace = ensure_canonical_workspace(workspace)?;
    let known_paths = known_entity_relative_paths(db, &workspace)?;
    let mut count = 0;
    for root in ["Accounts", "Projects", "People"] {
        let root_path = workspace.join(root);
        if !root_path.exists() {
            continue;
        }
        let canonical_root = root_path
            .canonicalize()
            .map_err(|e| format!("canonicalize active root failed: {e}"))?;
        if !canonical_root.starts_with(&workspace) {
            return Err("active root escaped workspace root".to_string());
        }
        for entry in std::fs::read_dir(&canonical_root)
            .map_err(|e| format!("read active root failed: {e}"))?
        {
            let entry = entry.map_err(|e| format!("read active root entry failed: {e}"))?;
            let file_name = entry.file_name();
            if file_name.to_string_lossy().starts_with('.') {
                continue;
            }
            let metadata = entry
                .metadata()
                .map_err(|e| format!("active root entry metadata failed: {e}"))?;
            if !metadata.is_dir() {
                continue;
            }
            let relative = PathBuf::from(root).join(file_name);
            if !known_paths.contains(&relative) {
                count += 1;
            }
        }
    }
    Ok(count)
}

fn known_entity_relative_paths(
    db: &ActionDb,
    workspace: &Path,
) -> Result<BTreeSet<PathBuf>, String> {
    let mut paths = BTreeSet::new();
    for account in db.get_all_accounts().map_err(|e| e.to_string())? {
        paths.insert(active_relative_path(
            &descriptor_from_account(account),
            Some(workspace),
        )?);
    }
    for project in db.get_all_projects().map_err(|e| e.to_string())? {
        paths.insert(active_relative_path(
            &descriptor_from_project(project),
            Some(workspace),
        )?);
    }
    for person in db.get_people(None).map_err(|e| e.to_string())? {
        paths.insert(active_relative_path(
            &descriptor_from_person(person),
            Some(workspace),
        )?);
    }
    for entity_type in [
        EntityArchiveType::Account,
        EntityArchiveType::Project,
        EntityArchiveType::Person,
    ] {
        for descriptor in load_archived_descriptors(db, entity_type)? {
            paths.insert(active_relative_path(&descriptor, Some(workspace))?);
        }
    }
    Ok(paths)
}

fn active_relative_path(
    descriptor: &EntityDescriptor,
    workspace: Option<&Path>,
) -> Result<PathBuf, String> {
    if let Some(tracker_path) = descriptor.tracker_path.as_deref() {
        if let Some(relative) =
            normalized_tracker_relative_path(descriptor.entity_type, tracker_path, workspace)?
        {
            return Ok(relative);
        }
    }

    Ok(PathBuf::from(descriptor.entity_type.active_root())
        .join(crate::util::sanitize_for_filesystem(&descriptor.name)))
}

fn normalized_tracker_relative_path(
    entity_type: EntityArchiveType,
    tracker_path: &str,
    workspace: Option<&Path>,
) -> Result<Option<PathBuf>, String> {
    let tracker_path = Path::new(tracker_path);
    let mut candidate = if tracker_path.is_absolute() {
        let Some(workspace) = workspace else {
            return Ok(None);
        };
        match tracker_path.strip_prefix(workspace) {
            Ok(relative) => relative.to_path_buf(),
            Err(_) => return Ok(None),
        }
    } else {
        tracker_path.to_path_buf()
    };

    if entity_type == EntityArchiveType::Person
        && candidate.file_name().and_then(|name| name.to_str()) == Some("person.json")
    {
        let Some(parent) = candidate.parent() else {
            return Ok(None);
        };
        candidate = parent.to_path_buf();
    }

    let relative = match parse_relative_path(&path_to_string(&candidate)) {
        Ok(relative) => relative,
        Err(_) => return Ok(None),
    };
    let first = first_component(&relative);
    let allowed = match entity_type {
        EntityArchiveType::Account => {
            matches!(first.as_deref(), Some("Accounts") | Some("Internal"))
        }
        _ => first.as_deref() == Some(entity_type.active_root()),
    };
    let has_child_component = relative
        .components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count()
        > 1;
    if allowed && has_child_component {
        Ok(Some(relative))
    } else {
        Ok(None)
    }
}

fn resolved_workspace_root(state: &AppState) -> Result<PathBuf, String> {
    let configured = {
        let guard = state.config.read();
        guard.as_ref().map(|config| config.workspace_path.clone())
    };
    crate::state::resolved_workspace_path(configured.as_deref())
}

fn ensure_canonical_workspace(workspace: &Path) -> Result<PathBuf, String> {
    // dos7-allowed: entity-archive-folder-v150 owns mode-scoped archive folder moves.
    std::fs::create_dir_all(workspace).map_err(|e| format!("create workspace root failed: {e}"))?;
    workspace
        .canonicalize()
        .map_err(|e| format!("canonicalize workspace root failed: {e}"))
}

fn validated_existing_source(
    workspace: &Path,
    entity_type: EntityArchiveType,
    relative_path: &Path,
) -> Result<Option<PathBuf>, String> {
    let workspace = ensure_canonical_workspace(workspace)?;
    let first = first_component(relative_path);
    let allowed = match entity_type {
        EntityArchiveType::Account => {
            matches!(first.as_deref(), Some("Accounts") | Some("Internal"))
        }
        _ => first.as_deref() == Some(entity_type.active_root()),
    };
    if !allowed {
        return Err("source path root is not allowed".to_string());
    }
    validated_existing_dir(&workspace, relative_path)
}

fn validated_existing_archive_source(
    workspace: &Path,
    relative_path: &str,
) -> Result<Option<PathBuf>, String> {
    let workspace = ensure_canonical_workspace(workspace)?;
    let relative = parse_relative_path(relative_path)?;
    if first_component(&relative).as_deref() != Some("_archive") {
        return Err("archived source path root is not allowed".to_string());
    }
    validated_existing_dir(&workspace, &relative)
}

fn validated_restore_target(
    workspace: &Path,
    entity_type: EntityArchiveType,
    relative_path: &Path,
) -> Result<Option<PathBuf>, String> {
    let workspace = ensure_canonical_workspace(workspace)?;
    let first = first_component(relative_path);
    let allowed = match entity_type {
        EntityArchiveType::Account => {
            matches!(first.as_deref(), Some("Accounts") | Some("Internal"))
        }
        _ => first.as_deref() == Some(entity_type.active_root()),
    };
    if !allowed {
        return Err("restore target root is not allowed".to_string());
    }

    let target = workspace.join(relative_path);
    match std::fs::symlink_metadata(&target) {
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("restore target metadata read failed: {error}")),
    }

    let Some(parent_relative_path) = relative_path.parent() else {
        return Err("restore target parent is missing".to_string());
    };
    ensure_normal_parent_path(&workspace, parent_relative_path)?;
    Ok(Some(target))
}

fn ensure_normal_parent_path(workspace: &Path, relative_parent: &Path) -> Result<(), String> {
    let relative_parent = parse_relative_path(&path_to_string(relative_parent))?;
    let mut current = workspace.to_path_buf();
    for component in relative_parent.components() {
        let Component::Normal(name) = component else {
            return Err("restore target parent contains invalid component".to_string());
        };
        current.push(name);
        ensure_normal_dir_component(workspace, &current)?;
    }
    Ok(())
}

fn ensure_normal_dir_component(workspace: &Path, path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err("restore target parent symlink rejected".to_string());
            }
            if !metadata.is_dir() {
                return Err("restore target parent is not a directory".to_string());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(path).map_err(|e| format!("create restore parent failed: {e}"))?;
        }
        Err(error) => return Err(format!("restore parent metadata read failed: {error}")),
    }

    let canonical = path
        .canonicalize()
        .map_err(|e| format!("canonicalize restore parent failed: {e}"))?;
    if !canonical.starts_with(workspace) {
        return Err("restore target parent escaped workspace root".to_string());
    }
    Ok(())
}

fn validated_existing_dir(
    workspace: &Path,
    relative_path: &Path,
) -> Result<Option<PathBuf>, String> {
    let target = workspace.join(relative_path);
    if !target.exists() {
        return Ok(None);
    }
    let target_metadata = std::fs::symlink_metadata(&target)
        .map_err(|e| format!("source metadata read failed: {e}"))?;
    if target_metadata.file_type().is_symlink() {
        return Err("symlink path rejected".to_string());
    }
    if !target_metadata.is_dir() {
        return Err("source path is not a directory".to_string());
    }
    let canonical = target
        .canonicalize()
        .map_err(|e| format!("canonicalize source failed: {e}"))?;
    if !canonical.starts_with(workspace) {
        return Err("source path escaped workspace root".to_string());
    }
    reject_unsafe_tree(&canonical)?;
    Ok(Some(canonical))
}

fn allocate_archive_relative_path(
    workspace: &Path,
    entity_type: EntityArchiveType,
    entity_id: &str,
    name: &str,
) -> Result<PathBuf, String> {
    let workspace = ensure_canonical_workspace(workspace)?;
    let base_dir = PathBuf::from("_archive")
        .join("entities")
        .join(entity_type.archive_root());
    ensure_normal_parent_path(&workspace, &base_dir)?;
    let archive_dir = workspace.join(&base_dir);
    let canonical_parent = archive_dir
        .canonicalize()
        .map_err(|e| format!("canonicalize archive folder failed: {e}"))?;
    if !canonical_parent.starts_with(&workspace) {
        return Err("archive target escaped workspace root".to_string());
    }

    let slug = crate::util::slugify(name);
    let base_name = format!("{entity_id}--{slug}");
    for suffix in 0..1000 {
        let candidate_name = if suffix == 0 {
            base_name.clone()
        } else {
            format!("{base_name}--{suffix}")
        };
        let candidate = base_dir.join(candidate_name);
        if !workspace.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err("could not allocate archive target".to_string())
}

fn move_directory_no_overwrite(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        return Err("target already exists".to_string());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create target parent failed: {e}"))?;
    }
    reject_unsafe_tree(source)?;
    std::fs::rename(source, target).map_err(|e| format!("folder rename failed: {e}"))
}

fn write_archive_manifest(path: &Path, manifest: &ArchiveManifest<'_>) -> Result<(), String> {
    let content = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("archive manifest serialization failed: {e}"))?;
    std::fs::write(path.join(".dailyos-archive.json"), content)
        .map_err(|e| format!("archive manifest write failed: {e}"))
}

fn write_archive_manifest_for_metadata(
    path: &Path,
    metadata: &FolderMetadata,
) -> Result<(), String> {
    let Some(archived_relative_path) = metadata.archived_relative_path.as_deref() else {
        return Err("archive manifest metadata missing archived path".to_string());
    };
    let archived_at = Utc::now().to_rfc3339();
    let manifest = ArchiveManifest {
        entity_type: metadata.entity_type.as_str(),
        entity_id: &metadata.entity_id,
        original_relative_path: &metadata.original_relative_path,
        archived_relative_path,
        archived_at: &archived_at,
        operation_id: &metadata.operation_id,
    };
    write_archive_manifest(path, &manifest)
}

fn archive_target_from_metadata(workspace: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let workspace = ensure_canonical_workspace(workspace)?;
    let relative = parse_relative_path(relative_path)?;
    if first_component(&relative).as_deref() != Some("_archive") {
        return Err("archive repair target root is not allowed".to_string());
    }
    let target = workspace.join(&relative);
    if let Some(parent_relative_path) = relative.parent() {
        ensure_normal_parent_path(&workspace, parent_relative_path)?;
        let parent = workspace.join(parent_relative_path);
        let canonical_parent = parent
            .canonicalize()
            .map_err(|e| format!("canonicalize archive repair target parent failed: {e}"))?;
        if !canonical_parent.starts_with(&workspace) {
            return Err("archive repair target escaped workspace root".to_string());
        }
    }
    Ok(target)
}

fn parse_relative_path(value: &str) -> Result<PathBuf, String> {
    if value.contains('\0') {
        return Err("path contains NUL".to_string());
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err("path must be relative".to_string());
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => normalized.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("path traversal is not allowed".to_string());
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err("path must not be empty".to_string());
    }
    Ok(normalized)
}

fn first_component(path: &Path) -> Option<String> {
    path.components().find_map(|component| match component {
        Component::Normal(value) => Some(value.to_string_lossy().to_string()),
        _ => None,
    })
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn remove_validated_root(workspace: &Path, relative_path: &Path) -> Result<(), String> {
    let relative_path = parse_relative_path(&path_to_string(relative_path))?;
    let target = workspace.join(&relative_path);
    let metadata = match std::fs::symlink_metadata(&target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("delete target metadata read failed: {error}")),
    };
    if metadata.file_type().is_symlink() {
        return Err("delete target symlink rejected".to_string());
    }
    if !metadata.is_dir() {
        return Err("delete target is not a directory".to_string());
    }
    reject_symlink_path_components(workspace, &relative_path)?;
    let canonical = target
        .canonicalize()
        .map_err(|e| format!("canonicalize delete target failed: {e}"))?;
    if !canonical.starts_with(workspace) {
        return Err("delete target escaped workspace root".to_string());
    }
    reject_unsafe_tree(&target)?;
    // dos7-allowed: entity-archive-folder-v150 removes only validated mode-scoped archive roots.
    std::fs::remove_dir_all(target).map_err(|e| format!("delete workspace folder failed: {e}"))
}

fn reject_symlink_path_components(workspace: &Path, relative_path: &Path) -> Result<(), String> {
    let mut current = workspace.to_path_buf();
    for component in relative_path.components() {
        let Component::Normal(name) = component else {
            return Err("delete target path contains invalid component".to_string());
        };
        current.push(name);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err("delete target symlink rejected".to_string());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(format!("delete target metadata read failed: {error}")),
        }
    }
    Ok(())
}

fn reject_unsafe_tree(root: &Path) -> Result<(), String> {
    let metadata =
        std::fs::symlink_metadata(root).map_err(|e| format!("metadata read failed: {e}"))?;
    if metadata.file_type().is_symlink() {
        return Err("symlink path rejected".to_string());
    }
    if metadata.is_file() && has_multiple_hardlinks(&metadata) {
        return Err("hardlinked file rejected".to_string());
    }
    if metadata.is_dir() {
        for entry in std::fs::read_dir(root).map_err(|e| format!("read dir failed: {e}"))? {
            let entry = entry.map_err(|e| format!("read dir entry failed: {e}"))?;
            reject_unsafe_tree(&entry.path())?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn has_multiple_hardlinks(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink() > 1
}

#[cfg(not(unix))]
fn has_multiple_hardlinks(_metadata: &std::fs::Metadata) -> bool {
    false
}

fn normalize_ids(ids: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for id in ids {
        let id = id.trim();
        if !id.is_empty() && seen.insert(id.to_string()) {
            out.push(id.to_string());
        }
    }
    out
}

fn dedupe_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{AccountType, DbAccount, DbProject};

    fn test_db() -> ActionDb {
        ActionDb::from_connection_for_tests(crate::migrations::migrated_in_memory_for_tests())
    }

    fn insert_account(
        db: &ActionDb,
        id: &str,
        name: &str,
        parent_id: Option<&str>,
        archived: bool,
    ) {
        db.upsert_account(&DbAccount {
            id: id.to_string(),
            name: name.to_string(),
            tracker_path: Some(if let Some(parent) = parent_id {
                format!("Accounts/Parent/{name}-{parent}")
            } else {
                format!("Accounts/{name}")
            }),
            parent_id: parent_id.map(str::to_string),
            account_type: AccountType::Customer,
            updated_at: Utc::now().to_rfc3339(),
            archived,
            ..Default::default()
        })
        .expect("insert account");
    }

    fn insert_project(
        db: &ActionDb,
        id: &str,
        name: &str,
        parent_id: Option<&str>,
        archived: bool,
    ) {
        db.upsert_project(&DbProject {
            id: id.to_string(),
            name: name.to_string(),
            status: "active".to_string(),
            tracker_path: Some(if let Some(parent) = parent_id {
                format!("Projects/Parent/{name}-{parent}")
            } else {
                format!("Projects/{name}")
            }),
            parent_id: parent_id.map(str::to_string),
            updated_at: Utc::now().to_rfc3339(),
            archived,
            ..Default::default()
        })
        .expect("insert project");
    }

    async fn test_state_with_workspace(
        db_path: PathBuf,
        workspace_path: &Path,
    ) -> (Arc<AppState>, PathBuf) {
        let db_service =
            crate::db_service::DbService::open_at_with_fixture_provider_for_tests(db_path)
                .await
                .expect("open db service");
        let state = Arc::new(AppState::test_with_db_service(db_service));
        let config: crate::types::Config = serde_json::from_value(serde_json::json!({
            "workspacePath": workspace_path.to_string_lossy().to_string()
        }))
        .expect("test config");
        *state.config.write() = Some(config);
        let resolved_workspace =
            crate::state::resolved_workspace_path(Some(&workspace_path.to_string_lossy()))
                .expect("resolved test workspace");
        std::fs::create_dir_all(&resolved_workspace).expect("create resolved test workspace");
        (state, resolved_workspace)
    }

    #[test]
    fn preview_bulk_archive_dedupes_covered_children_and_reports_non_active_inputs() {
        let db = test_db();
        insert_account(&db, "parent-account", "Parent", None, false);
        insert_account(&db, "child-account", "Child", Some("parent-account"), false);
        insert_account(
            &db,
            "grandchild-account",
            "Grandchild",
            Some("child-account"),
            false,
        );
        insert_account(
            &db,
            "archived-child",
            "Archived Child",
            Some("parent-account"),
            true,
        );

        let preview = preview_bulk_archive(
            &db,
            EntityArchiveType::Account,
            vec![
                "parent-account".to_string(),
                "child-account".to_string(),
                "child-account".to_string(),
                "grandchild-account".to_string(),
                "archived-child".to_string(),
                "missing-account".to_string(),
            ],
        )
        .expect("preview");

        assert_eq!(
            preview.requested_ids,
            vec![
                "parent-account",
                "child-account",
                "grandchild-account",
                "archived-child",
                "missing-account"
            ]
        );
        assert_eq!(preview.root_ids, vec!["parent-account"]);
        assert_eq!(
            preview.covered_child_ids,
            vec!["child-account", "grandchild-account"]
        );
        assert_eq!(
            preview.cascaded_child_ids,
            vec!["child-account", "grandchild-account"]
        );
        assert_eq!(
            preview.changed_ids,
            vec!["parent-account", "child-account", "grandchild-account"]
        );
        assert_eq!(preview.total_changed_count, 3);
        assert_eq!(preview.direct_cascade_count, 2);
        assert_eq!(preview.already_archived_ids, vec!["archived-child"]);
        assert_eq!(preview.not_found_ids, vec!["missing-account"]);
    }

    #[test]
    fn archive_outcome_archives_active_descendants() {
        let db = test_db();
        insert_account(&db, "parent-account", "Parent", None, false);
        insert_account(&db, "child-account", "Child", Some("parent-account"), false);
        insert_account(
            &db,
            "grandchild-account",
            "Grandchild",
            Some("child-account"),
            false,
        );

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external)
            .with_actor("agent:test");
        let state = crate::state::AppState::new();
        let outcome = archive_entity_with_outcome(
            &ctx,
            &db,
            &state,
            EntityArchiveType::Account,
            "parent-account",
            true,
        )
        .expect("archive parent");

        assert_eq!(
            outcome.changed_ids,
            vec!["parent-account", "child-account", "grandchild-account"]
        );
        assert_eq!(
            outcome.cascaded_child_ids,
            vec!["child-account", "grandchild-account"]
        );
        for id in ["parent-account", "child-account", "grandchild-account"] {
            assert!(
                db.get_account(id)
                    .expect("read account")
                    .expect("account exists")
                    .archived,
                "{id} should be archived"
            );
        }
    }

    #[test]
    fn single_entity_restore_does_not_restore_archived_descendants() {
        let db = test_db();
        insert_account(&db, "parent-account", "Parent", None, true);
        insert_account(&db, "child-account", "Child", Some("parent-account"), true);
        insert_account(
            &db,
            "grandchild-account",
            "Grandchild",
            Some("child-account"),
            true,
        );
        insert_project(&db, "parent-project", "Parent", None, true);
        insert_project(&db, "child-project", "Child", Some("parent-project"), true);
        insert_project(
            &db,
            "grandchild-project",
            "Grandchild",
            Some("child-project"),
            true,
        );

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external)
            .with_actor("agent:test");
        let state = crate::state::AppState::new();
        let account_outcome = archive_entity_with_outcome(
            &ctx,
            &db,
            &state,
            EntityArchiveType::Account,
            "parent-account",
            false,
        )
        .expect("restore selected account");
        let project_outcome = archive_entity_with_outcome(
            &ctx,
            &db,
            &state,
            EntityArchiveType::Project,
            "parent-project",
            false,
        )
        .expect("restore selected project");

        assert_eq!(account_outcome.changed_ids, vec!["parent-account"]);
        assert!(account_outcome.cascaded_child_ids.is_empty());
        assert_eq!(project_outcome.changed_ids, vec!["parent-project"]);
        assert!(project_outcome.cascaded_child_ids.is_empty());
        assert!(
            db.get_account("child-account")
                .expect("read child account")
                .expect("child account exists")
                .archived
        );
        assert!(
            db.get_account("grandchild-account")
                .expect("read grandchild account")
                .expect("grandchild account exists")
                .archived
        );
        assert!(
            db.get_project("child-project")
                .expect("read child project")
                .expect("child project exists")
                .archived
        );
        assert!(
            db.get_project("grandchild-project")
                .expect("read grandchild project")
                .expect("grandchild project exists")
                .archived
        );
    }

    #[test]
    fn bulk_archive_rolls_back_prior_roots_when_later_root_fails() {
        let db = test_db();
        insert_account(&db, "first-account", "First", None, false);
        insert_account(&db, "second-account", "Second", None, false);
        let ids = vec!["first-account".to_string(), "second-account".to_string()];
        let preview =
            preview_bulk_archive(&db, EntityArchiveType::Account, ids.clone()).expect("preview");
        db.conn_ref()
            .execute(
                "CREATE TRIGGER fail_second_archive
                 BEFORE UPDATE OF archived ON accounts
                 WHEN NEW.id = 'second-account' AND NEW.archived = 1
                 BEGIN
                   SELECT RAISE(FAIL, 'forced archive failure');
                 END;",
                [],
            )
            .expect("install failure trigger");

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external)
            .with_actor("agent:test");
        let state = crate::state::AppState::new();
        let error = execute_bulk_archive(
            &ctx,
            &db,
            &state,
            EntityArchiveType::Account,
            ids,
            &preview.plan_id,
            &preview.plan_fingerprint,
        )
        .expect_err("bulk archive fails");

        assert!(error.contains("forced archive failure"));
        assert!(
            !db.get_account("first-account")
                .expect("read first")
                .expect("first exists")
                .archived
        );
        assert!(
            !db.get_account("second-account")
                .expect("read second")
                .expect("second exists")
                .archived
        );
    }

    #[test]
    fn folder_move_targets_skip_children_nested_under_changed_parent() {
        let db = test_db();
        insert_account(&db, "parent-account", "Parent", None, false);
        insert_account(&db, "child-account", "Child", Some("parent-account"), false);
        let descriptors = vec![
            load_descriptor(&db, EntityArchiveType::Account, "parent-account")
                .expect("load parent")
                .expect("parent exists"),
            load_descriptor(&db, EntityArchiveType::Account, "child-account")
                .expect("load child")
                .expect("child exists"),
        ];

        let (move_ids, skipped_ids) =
            partition_folder_move_targets(descriptors, None).expect("partition targets");

        assert_eq!(move_ids, vec!["parent-account"]);
        assert!(skipped_ids.contains("child-account"));
    }

    #[tokio::test]
    async fn archive_folders_records_metadata_for_child_skipped_under_parent() {
        let workspace = tempfile::tempdir().expect("workspace");
        let db_dir = tempfile::tempdir().expect("db dir");
        let (state, workspace_path) =
            test_state_with_workspace(db_dir.path().join("dailyos.db"), workspace.path()).await;
        state
            .db_write(|db| {
                insert_account(db, "parent-account", "Parent", None, false);
                insert_account(db, "child-account", "Child", Some("parent-account"), false);
                Ok(())
            })
            .await
            .expect("seed accounts");
        std::fs::create_dir_all(workspace_path.join("Accounts/Parent/Child-parent-account"))
            .expect("create nested child folder");

        let results = archive_folders_for_outcomes(
            state.clone(),
            &[ArchiveMutationOutcome {
                requested_id: "parent-account".to_string(),
                entity_type: EntityArchiveType::Account,
                changed_ids: vec!["parent-account".to_string(), "child-account".to_string()],
                selected_id_changed: true,
                cascaded_child_ids: vec!["child-account".to_string()],
                already_in_target_state_ids: Vec::new(),
                not_found_ids: Vec::new(),
                intel_queue_cleanup_ids: Vec::new(),
                operation_id: "archive-op".to_string(),
            }],
        )
        .await;

        let child_result = results
            .iter()
            .find(|result| result.entity_id == "child-account")
            .expect("child result");
        assert_eq!(child_result.folder_status, "folder_skipped");
        let archived_child_path = child_result
            .archived_relative_path
            .as_ref()
            .expect("child archived path");
        assert!(workspace_path.join(archived_child_path).exists());

        let child_metadata: (String, String) = state
            .db_read(|db| {
                db.conn_ref()
                    .query_row(
                        "SELECT folder_state, archived_relative_path
                           FROM entity_archive_folders
                          WHERE operation_id = 'archive-op'
                            AND entity_type = 'account'
                            AND entity_id = 'child-account'",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|error| error.to_string())
            })
            .await
            .expect("read child metadata");
        assert_eq!(child_metadata.0, "archived");
        assert_eq!(child_metadata.1, *archived_child_path);

        let restore_result = restore_folder_for_entity(
            state.clone(),
            EntityArchiveType::Account,
            "child-account".to_string(),
        )
        .await;

        assert_eq!(restore_result.folder_status, "succeeded");
        assert!(workspace_path
            .join("Accounts/Parent/Child-parent-account")
            .exists());
    }

    #[tokio::test]
    async fn parent_only_restore_keeps_archived_descendant_folder_in_archive() {
        let workspace = tempfile::tempdir().expect("workspace");
        let db_dir = tempfile::tempdir().expect("db dir");
        let (state, workspace_path) =
            test_state_with_workspace(db_dir.path().join("dailyos.db"), workspace.path()).await;
        state
            .db_write(|db| {
                insert_account(db, "parent-account", "Parent", None, false);
                insert_account(db, "child-account", "Child", Some("parent-account"), false);
                Ok(())
            })
            .await
            .expect("seed accounts");
        let child_active_path = workspace_path.join("Accounts/Parent/Child-parent-account");
        std::fs::create_dir_all(&child_active_path).expect("create nested child folder");
        std::fs::write(child_active_path.join("notes.md"), "child workspace data")
            .expect("write child data");

        let archive_results = archive_folders_for_outcomes(
            state.clone(),
            &[ArchiveMutationOutcome {
                requested_id: "parent-account".to_string(),
                entity_type: EntityArchiveType::Account,
                changed_ids: vec!["parent-account".to_string(), "child-account".to_string()],
                selected_id_changed: true,
                cascaded_child_ids: vec!["child-account".to_string()],
                already_in_target_state_ids: Vec::new(),
                not_found_ids: Vec::new(),
                intel_queue_cleanup_ids: Vec::new(),
                operation_id: "archive-op".to_string(),
            }],
        )
        .await;
        assert!(archive_results
            .iter()
            .any(|result| result.entity_id == "child-account"
                && result.folder_status == "folder_skipped"));
        state
            .db_write(|db| {
                db.archive_account("parent-account", true)
                    .map_err(|error| error.to_string())?;
                db.archive_account("child-account", true)
                    .map_err(|error| error.to_string())?;
                db.archive_account("parent-account", false)
                    .map_err(|error| error.to_string())?;
                record_restore_intent_for_changed_ids(
                    db,
                    EntityArchiveType::Account,
                    &["parent-account".to_string()],
                )?;
                Ok(())
            })
            .await
            .expect("restore parent db only");

        let restore_result = restore_folder_for_entity(
            state.clone(),
            EntityArchiveType::Account,
            "parent-account".to_string(),
        )
        .await;

        assert_eq!(restore_result.folder_status, "succeeded");
        assert!(workspace_path.join("Accounts/Parent").exists());
        assert!(
            !child_active_path.exists(),
            "child folder must not become active while child row remains archived"
        );
        assert!(state
            .db_read(|db| {
                Ok(db
                    .get_account("child-account")
                    .map_err(|error| error.to_string())?
                    .expect("child account exists")
                    .archived)
            })
            .await
            .expect("read child archive state"));
        let child_metadata: (String, String) = state
            .db_read(|db| {
                db.conn_ref()
                    .query_row(
                        "SELECT folder_state, archived_relative_path
                           FROM entity_archive_folders
                          WHERE operation_id = 'archive-op'
                            AND entity_type = 'account'
                            AND entity_id = 'child-account'",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|error| error.to_string())
            })
            .await
            .expect("read child metadata");
        assert_eq!(child_metadata.0, "archived");
        assert!(workspace_path.join(&child_metadata.1).exists());
        assert_ne!(
            child_metadata.1,
            "_archive/entities/account/parent-account--parent/Child-parent-account"
        );
    }

    #[test]
    fn restore_account_outcome_restores_archived_children_when_parent_already_active() {
        let db = test_db();
        insert_account(&db, "parent-account", "Parent", None, false);
        insert_account(&db, "child-account", "Child", Some("parent-account"), true);
        insert_account(
            &db,
            "grandchild-account",
            "Grandchild",
            Some("child-account"),
            true,
        );
        let restore_target_ids =
            restore_account_target_ids(&db, "parent-account", true).expect("restore targets");
        assert_eq!(
            restore_target_ids,
            vec!["parent-account", "child-account", "grandchild-account"]
        );

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external)
            .with_actor("agent:test");
        let state = crate::state::AppState::new();
        let outcome = restore_account_with_outcome(&ctx, &db, &state, "parent-account", true)
            .expect("restore account");

        assert!(!outcome.selected_id_changed);
        assert_eq!(outcome.already_in_target_state_ids, vec!["parent-account"]);
        assert_eq!(
            outcome.changed_ids,
            vec!["child-account", "grandchild-account"]
        );
        assert_eq!(
            outcome.cascaded_child_ids,
            vec!["child-account", "grandchild-account"]
        );
        for id in ["child-account", "grandchild-account"] {
            assert!(
                !db.get_account(id)
                    .expect("read account")
                    .expect("account exists")
                    .archived,
                "{id} should be restored"
            );
        }
    }

    #[test]
    fn restore_outcome_records_restore_intent_before_commit() {
        let db = test_db();
        insert_account(&db, "archived-account", "Archived", None, true);
        db.conn_ref()
            .execute(
                "INSERT INTO entity_archive_folders (
                    operation_id, entity_type, entity_id, original_relative_path,
                    archived_relative_path, folder_state, archived_at, restored_at,
                    updated_at, last_error_code
                 ) VALUES (?1, 'account', ?2, ?3, ?4, 'archived', ?5, NULL, ?5, NULL)",
                params![
                    "archive-op",
                    "archived-account",
                    "Accounts/Archived",
                    "_archive/entities/account/archived-account--archived",
                    Utc::now().to_rfc3339(),
                ],
            )
            .expect("insert archive metadata");

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external)
            .with_actor("agent:test");
        let state = crate::state::AppState::new();
        let outcome = archive_entity_with_outcome(
            &ctx,
            &db,
            &state,
            EntityArchiveType::Account,
            "archived-account",
            false,
        )
        .expect("restore entity");

        assert_eq!(outcome.changed_ids, vec!["archived-account"]);
        let folder_state: String = db
            .conn_ref()
            .query_row(
                "SELECT folder_state FROM entity_archive_folders
                 WHERE operation_id = 'archive-op'
                   AND entity_type = 'account'
                   AND entity_id = 'archived-account'",
                [],
                |row| row.get(0),
            )
            .expect("read folder state");
        assert_eq!(folder_state, "restore_pending");
    }

    #[test]
    fn restore_outcome_records_restore_intent_for_pending_archive_target() {
        let db = test_db();
        insert_account(&db, "archived-account", "Archived", None, true);
        db.conn_ref()
            .execute(
                "INSERT INTO entity_archive_folders (
                    operation_id, entity_type, entity_id, original_relative_path,
                    archived_relative_path, folder_state, archived_at, restored_at,
                    updated_at, last_error_code
                 ) VALUES (?1, 'account', ?2, ?3, ?4, 'archive_pending', ?5, NULL, ?5, NULL)",
                params![
                    "archive-op",
                    "archived-account",
                    "Accounts/Archived",
                    "_archive/entities/account/archived-account--archived",
                    Utc::now().to_rfc3339(),
                ],
            )
            .expect("insert archive metadata");

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external)
            .with_actor("agent:test");
        let state = crate::state::AppState::new();
        archive_entity_with_outcome(
            &ctx,
            &db,
            &state,
            EntityArchiveType::Account,
            "archived-account",
            false,
        )
        .expect("restore entity");

        let folder_state: String = db
            .conn_ref()
            .query_row(
                "SELECT folder_state FROM entity_archive_folders
                 WHERE operation_id = 'archive-op'
                   AND entity_type = 'account'
                   AND entity_id = 'archived-account'",
                [],
                |row| row.get(0),
            )
            .expect("read folder state");
        assert_eq!(folder_state, "restore_pending");
    }

    #[tokio::test]
    async fn restore_folder_recovers_failed_archive_when_active_folder_already_exists() {
        let workspace = tempfile::tempdir().expect("workspace");
        let db_dir = tempfile::tempdir().expect("db dir");
        let (state, workspace_path) =
            test_state_with_workspace(db_dir.path().join("dailyos.db"), workspace.path()).await;
        state
            .db_write(|db| {
                insert_account(db, "archived-account", "Archived", None, true);
                db.conn_ref()
                    .execute(
                        "INSERT INTO entity_archive_folders (
                            operation_id, entity_type, entity_id, original_relative_path,
                            archived_relative_path, folder_state, archived_at, restored_at,
                            updated_at, last_error_code
                         ) VALUES (?1, 'account', ?2, ?3, ?4, 'archive_failed', ?5, NULL, ?5, 'move_failed')",
                        params![
                            "archive-op",
                            "archived-account",
                            "Accounts/Archived",
                            "_archive/entities/account/archived-account--archived",
                            Utc::now().to_rfc3339(),
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .await
            .expect("seed archive metadata");
        std::fs::create_dir_all(workspace_path.join("Accounts/Archived"))
            .expect("create active folder");

        preflight_restore_targets(
            state.clone(),
            EntityArchiveType::Account,
            vec!["archived-account".to_string()],
        )
        .await
        .expect("preflight permits already-active failed archive");
        let result = restore_folder_for_entity(
            state.clone(),
            EntityArchiveType::Account,
            "archived-account".to_string(),
        )
        .await;

        assert_eq!(result.folder_status, "succeeded");
        let folder_state: String = state
            .db_read(|db| {
                db.conn_ref()
                    .query_row(
                        "SELECT folder_state FROM entity_archive_folders
                         WHERE operation_id = 'archive-op'
                           AND entity_type = 'account'
                           AND entity_id = 'archived-account'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| error.to_string())
            })
            .await
            .expect("read folder state");
        assert_eq!(folder_state, "restored");
    }

    #[tokio::test]
    async fn restore_folder_marks_missing_archive_source_as_failed() {
        let workspace = tempfile::tempdir().expect("workspace");
        let db_dir = tempfile::tempdir().expect("db dir");
        let (state, workspace_path) =
            test_state_with_workspace(db_dir.path().join("dailyos.db"), workspace.path()).await;
        state
            .db_write(|db| {
                insert_account(db, "archived-account", "Archived", None, true);
                db.conn_ref()
                    .execute(
                        "INSERT INTO entity_archive_folders (
                            operation_id, entity_type, entity_id, original_relative_path,
                            archived_relative_path, folder_state, archived_at, restored_at,
                            updated_at, last_error_code
                         ) VALUES (?1, 'account', ?2, ?3, ?4, 'archived', ?5, NULL, ?5, NULL)",
                        params![
                            "archive-op",
                            "archived-account",
                            "Accounts/MissingSource/Archived",
                            "_archive/entities/account/archived-account--archived",
                            Utc::now().to_rfc3339(),
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .await
            .expect("seed archive metadata");

        let result = restore_folder_for_entity(
            state.clone(),
            EntityArchiveType::Account,
            "archived-account".to_string(),
        )
        .await;

        assert_eq!(result.folder_status, "folder_missing");
        assert!(!workspace_path
            .join("Accounts/MissingSource/Archived")
            .exists());
        let metadata: (String, Option<String>) = state
            .db_read(|db| {
                db.conn_ref()
                    .query_row(
                        "SELECT folder_state, last_error_code FROM entity_archive_folders
                         WHERE operation_id = 'archive-op'
                           AND entity_type = 'account'
                           AND entity_id = 'archived-account'",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|error| error.to_string())
            })
            .await
            .expect("read folder metadata");
        assert_eq!(metadata.0, "restore_failed");
        assert_eq!(metadata.1.as_deref(), Some("missing_source"));
    }

    #[test]
    fn active_relative_path_uses_sanitized_fallback_name() {
        let descriptor = EntityDescriptor {
            entity_type: EntityArchiveType::Person,
            id: "person-a".to_string(),
            name: "A/B".to_string(),
            tracker_path: None,
            parent_id: None,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        };

        let relative = active_relative_path(&descriptor, None).expect("resolve path");

        assert_eq!(relative, PathBuf::from("People").join("A-B"));
    }

    #[test]
    fn active_relative_path_normalizes_person_json_tracker_path() {
        let workspace = tempfile::tempdir().expect("workspace");
        let relative_descriptor = EntityDescriptor {
            entity_type: EntityArchiveType::Person,
            id: "person-a".to_string(),
            name: "Person A".to_string(),
            tracker_path: Some("People/Person A/person.json".to_string()),
            parent_id: None,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        };
        let absolute_descriptor = EntityDescriptor {
            tracker_path: Some(
                workspace
                    .path()
                    .join("People/Person A/person.json")
                    .to_string_lossy()
                    .to_string(),
            ),
            ..relative_descriptor.clone()
        };

        assert_eq!(
            active_relative_path(&relative_descriptor, Some(workspace.path()))
                .expect("relative person path"),
            PathBuf::from("People").join("Person A")
        );
        assert_eq!(
            active_relative_path(&absolute_descriptor, Some(workspace.path()))
                .expect("absolute person path"),
            PathBuf::from("People").join("Person A")
        );
    }

    #[test]
    fn active_relative_path_ignores_root_only_tracker_path() {
        let account = EntityDescriptor {
            entity_type: EntityArchiveType::Account,
            id: "account-root".to_string(),
            name: "RootAccount".to_string(),
            tracker_path: Some("Accounts".to_string()),
            parent_id: None,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        };
        let internal = EntityDescriptor {
            entity_type: EntityArchiveType::Account,
            id: "internal-root".to_string(),
            name: "InternalAccount".to_string(),
            tracker_path: Some("Internal".to_string()),
            parent_id: None,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        };
        let project = EntityDescriptor {
            entity_type: EntityArchiveType::Project,
            id: "project-root".to_string(),
            name: "Launch".to_string(),
            tracker_path: Some("Projects".to_string()),
            parent_id: None,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        };

        assert_eq!(
            active_relative_path(&account, None).expect("account path"),
            PathBuf::from("Accounts").join("RootAccount")
        );
        assert_eq!(
            active_relative_path(&internal, None).expect("internal path"),
            PathBuf::from("Accounts").join("InternalAccount")
        );
        assert_eq!(
            active_relative_path(&project, None).expect("project path"),
            PathBuf::from("Projects").join("Launch")
        );
    }

    #[cfg(unix)]
    #[test]
    fn restore_target_rejects_symlink_parent() {
        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        std::os::unix::fs::symlink(outside.path(), workspace.path().join("Accounts"))
            .expect("symlink active root");

        let error = validated_restore_target(
            workspace.path(),
            EntityArchiveType::Account,
            Path::new("Accounts/Archived"),
        )
        .unwrap_err();

        assert!(error.contains("symlink"));
        assert!(!outside.path().join("Archived").exists());
    }

    #[cfg(unix)]
    #[test]
    fn allocate_archive_relative_path_rejects_symlink_archive_root_before_creating_children() {
        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        std::os::unix::fs::symlink(outside.path(), workspace.path().join("_archive"))
            .expect("symlink archive root");

        let error = allocate_archive_relative_path(
            workspace.path(),
            EntityArchiveType::Account,
            "account-id",
            "Account",
        )
        .unwrap_err();

        assert!(error.contains("symlink"));
        assert!(!outside.path().join("entities").exists());
    }

    #[cfg(unix)]
    #[test]
    fn archive_target_from_metadata_rejects_symlink_archive_root_before_creating_children() {
        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        std::os::unix::fs::symlink(outside.path(), workspace.path().join("_archive"))
            .expect("symlink archive root");

        let error = archive_target_from_metadata(
            workspace.path(),
            "_archive/entities/account/account-id--account",
        )
        .unwrap_err();

        assert!(error.contains("symlink"));
        assert!(!outside.path().join("entities").exists());
    }

    #[test]
    fn delete_mode_scoped_roots_only_removes_internal_paths_referenced_by_db() {
        let workspace = tempfile::tempdir().expect("workspace");
        for relative in [
            "Accounts/Customer",
            "Projects/Launch",
            "People/Contact",
            "_archive/entities/account/archive",
            "Internal/Referenced",
            "Internal/Unreferenced",
        ] {
            std::fs::create_dir_all(workspace.path().join(relative)).expect("create folder");
        }

        delete_mode_scoped_entity_roots(workspace.path(), &["Internal/Referenced".to_string()])
            .expect("delete mode roots");

        assert!(!workspace.path().join("Accounts").exists());
        assert!(!workspace.path().join("Projects").exists());
        assert!(!workspace.path().join("People").exists());
        assert!(!workspace.path().join("_archive/entities").exists());
        assert!(!workspace.path().join("Internal/Referenced").exists());
        assert!(workspace.path().join("Internal/Unreferenced").exists());
    }

    #[cfg(unix)]
    #[test]
    fn delete_mode_scoped_roots_rejects_symlink_roots_before_recursing() {
        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        std::fs::create_dir_all(outside.path().join("Preserve")).expect("outside folder");
        std::os::unix::fs::symlink(outside.path(), workspace.path().join("Accounts"))
            .expect("symlink account root");

        let error = delete_mode_scoped_entity_roots(workspace.path(), &[]).unwrap_err();

        assert!(error.contains("symlink"));
        assert!(outside.path().join("Preserve").exists());
    }

    #[test]
    fn repair_plan_reports_actionable_metadata_and_true_orphans() {
        let db = test_db();
        let workspace = tempfile::tempdir().expect("workspace");
        insert_account(&db, "active-account", "Active", None, false);
        insert_account(&db, "archived-account", "Archived", None, true);

        for relative in ["Accounts/Active", "Accounts/Archived", "Accounts/Orphan"] {
            std::fs::create_dir_all(workspace.path().join(relative)).expect("create folder");
        }

        db.conn_ref()
            .execute(
                "INSERT INTO entity_archive_folders (
                    operation_id, entity_type, entity_id, original_relative_path,
                    archived_relative_path, folder_state, archived_at, restored_at,
                    updated_at, last_error_code
                 ) VALUES (?1, 'account', ?2, ?3, ?4, ?5, ?6, NULL, ?6, NULL)",
                params![
                    "archive-op",
                    "active-account",
                    "Accounts/Active",
                    "_archive/entities/account/active-account--active",
                    "archive_pending",
                    Utc::now().to_rfc3339(),
                ],
            )
            .expect("insert archive metadata");
        db.conn_ref()
            .execute(
                "INSERT INTO entity_archive_folders (
                    operation_id, entity_type, entity_id, original_relative_path,
                    archived_relative_path, folder_state, archived_at, restored_at,
                    updated_at, last_error_code
                 ) VALUES (?1, 'account', ?2, ?3, ?4, ?5, ?6, NULL, ?6, NULL)",
                params![
                    "restore-op",
                    "archived-account",
                    "Accounts/Archived",
                    "_archive/entities/account/archived-account--archived",
                    "restore_failed",
                    Utc::now().to_rfc3339(),
                ],
            )
            .expect("insert restore metadata");

        let plan = plan_reconciliation_sync(&db, workspace.path()).expect("repair plan");

        assert_eq!(plan.archived_db_active_folder_count, 1);
        assert_eq!(plan.pending_metadata_count, 2);
        assert_eq!(plan.orphan_active_folder_count, 1);
        assert!(plan.items.iter().any(|item| {
            item.entity_type == EntityArchiveType::Account
                && item.entity_id == "archived-account"
                && item.status == "archived_db_active_folder"
        }));
        assert!(plan.items.iter().any(|item| {
            item.entity_type == EntityArchiveType::Account
                && item.entity_id == "active-account"
                && item.status == "archive_pending"
        }));
        assert!(plan.items.iter().any(|item| {
            item.entity_type == EntityArchiveType::Account
                && item.entity_id == "archived-account"
                && item.status == "restore_failed"
        }));
    }
}

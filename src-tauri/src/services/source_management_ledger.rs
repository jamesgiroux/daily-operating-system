//! Source-management ledger read service for workspace sources.
//!
//! This service composes the W1-W3 workspace substrate into a privacy-safe
//! source list for external surfaces. It never returns raw paths, file IDs,
//! link IDs, run IDs, claim text, or provenance blobs.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use abilities_runtime::abilities::get_entity_intelligence::contracts::Cursor;
use abilities_runtime::abilities::provenance::subject::SubjectRef;
use abilities_runtime::abilities::provenance::trust::claim_trust_band_from_score;
use abilities_runtime::abilities::source_management_ledger::contracts::{
    SourceManagementActionKind, SourceManagementActionPolicy, SourceManagementActionReceipt,
    SourceManagementActionRequest, SourceManagementEntity, SourceManagementIngestionRun,
    SourceManagementLedgerPage, SourceManagementLedgerPrivacyProfile,
    SourceManagementLedgerReadRequest, SourceManagementLedgerResponse, SourceManagementSource,
    SourceManagementSourceActions, SourceManagementTrustBandSummary, SourceManagementUserOverride,
};
use abilities_runtime::abilities::trust::types::TrustBand;
use abilities_runtime::services::context::{
    ServiceContext, SourceManagementActionError, SourceManagementLedgerReadError,
};
use abilities_runtime::services::workspace_intake::{EntityRefDto, WorkspaceIntakeRequest};
use abilities_runtime::types::{
    subject_ref_from_json as claim_subject_ref_from_json, ClaimSubjectRef,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::db::ActionDb;
use crate::entity::EntityType;
use crate::services::workspace_ingestion::contracts::{SignalEmitContext, SignalEmitter};
use crate::services::workspace_ingestion::graph::WorkspaceGraphDiagnosticKey;
use crate::services::workspace_ingestion::lifecycle::{
    lifecycle_state_from_slug, lifecycle_state_slug, LifecycleRepo, LifecycleState,
};
use crate::services::workspace_ingestion::link::{LinkAttributionSource, LinkError, LinkRepo};
use crate::services::workspace_ingestion::pipeline::{quarantine_source, QuarantineActor};
use crate::services::workspace_ingestion::signals::{
    emit_source_policy_changed, WorkspaceSignalEmitter, WorkspaceSourcePolicyChangedInput,
};
use crate::signals::propagation::PropagationEngine;

const SCHEMA_VERSION: u32 = 1;
const DEFAULT_PAGE_SIZE: u32 = 25;
const MAX_PAGE_SIZE: u32 = 100;
const MAX_RUN_HISTORY_PER_SOURCE: usize = 5;
const WORKSPACE_SOURCE_PREFIX: &str = "workspace_file:";
const ACTIONS_READY_REASON: &str = "";
const ACTION_ALREADY_QUARANTINED_REASON: &str = "already_quarantined";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CursorPayload {
    schema_version: u32,
    offset: u64,
    request_fingerprint: String,
}

#[derive(Debug, Clone)]
struct NormalizedQuery {
    entity_type: String,
    entity_id: String,
    page_size: u32,
    cursor: Option<Cursor>,
    privacy_profile: SourceManagementLedgerPrivacyProfile,
}

#[derive(Debug, Clone)]
struct LifecycleRow {
    file_id: String,
    canonical_path: String,
    source_type: String,
    lifecycle_state: String,
    source_asof: String,
    category: Option<String>,
    user_override_at: Option<String>,
    entity_type: Option<String>,
    entity_id: Option<String>,
}

#[derive(Debug, Clone)]
struct LinkRow {
    file_id: String,
    entity_type: String,
    entity_id: String,
    attribution_source: String,
    confidence: f64,
    user_override_at: Option<String>,
}

#[derive(Debug, Clone)]
struct PlacementPreviewRow {
    source_handle: String,
    content_type: Option<String>,
}

#[derive(Debug, Clone)]
struct WorkspaceClaim {
    subjects: BTreeSet<EntitySubject>,
    source_file_ids: BTreeSet<String>,
    trust_score: Option<f64>,
    sensitivity: String,
}

struct SourceBuildContext<'a> {
    run_history: &'a BTreeMap<String, Vec<SourceManagementIngestionRun>>,
    placement_handles: &'a BTreeMap<String, PlacementPreviewRow>,
    claims: &'a [WorkspaceClaim],
    privacy_profile: SourceManagementLedgerPrivacyProfile,
    diagnostic_key: &'a WorkspaceGraphDiagnosticKey,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EntitySubject {
    entity_type: String,
    entity_id: String,
}

pub fn read_source_management_ledger(
    conn: &Connection,
    request: SourceManagementLedgerReadRequest,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<SourceManagementLedgerResponse, SourceManagementLedgerReadError> {
    let tx = conn.unchecked_transaction().map_err(read_failed)?;
    let response = read_source_management_ledger_snapshot(&tx, request, diagnostic_key)?;
    tx.commit().map_err(read_failed)?;
    Ok(response)
}

pub fn apply_source_management_action(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    workspace_root: PathBuf,
    signal_engine: Option<Arc<PropagationEngine>>,
    request: SourceManagementActionRequest,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<SourceManagementActionReceipt, SourceManagementActionError> {
    ctx.check_mutation_allowed()
        .map_err(|error| SourceManagementActionError::ActionFailed(error.to_string()))?;
    validate_action_input(&request)?;

    let conn = db.conn_ref();
    let lifecycle_rows = load_lifecycle_rows(conn).map_err(action_read_failed)?;
    let active_links = load_active_links(conn).map_err(action_read_failed)?;
    let target = resolve_action_target(&request, &lifecycle_rows, &active_links, diagnostic_key)?;

    match request.input.action {
        SourceManagementActionKind::Reingest => {
            apply_reingest_action(ctx, workspace_root, signal_engine, &request, &target, conn)
        }
        SourceManagementActionKind::Quarantine => {
            apply_quarantine_action(ctx, db, signal_engine, &request, &target)
        }
        SourceManagementActionKind::Relink => {
            apply_relink_action(ctx, db, signal_engine, &request, &target)
        }
        SourceManagementActionKind::Ignore => apply_lifecycle_policy_action(
            ctx,
            db,
            signal_engine,
            &request,
            &target,
            LifecycleState::Ignored,
            "ignored",
        ),
        SourceManagementActionKind::Scratchpad => apply_lifecycle_policy_action(
            ctx,
            db,
            signal_engine,
            &request,
            &target,
            LifecycleState::Scratchpad,
            "scratchpad",
        ),
        SourceManagementActionKind::Archive => apply_lifecycle_policy_action(
            ctx,
            db,
            signal_engine,
            &request,
            &target,
            LifecycleState::Archived,
            "archived",
        ),
        SourceManagementActionKind::Delete => apply_lifecycle_policy_action(
            ctx,
            db,
            signal_engine,
            &request,
            &target,
            LifecycleState::Deleted,
            "deleted",
        ),
    }
}

#[derive(Debug, Clone)]
struct ActionTarget {
    lifecycle: LifecycleRow,
}

fn validate_action_input(
    request: &SourceManagementActionRequest,
) -> Result<(), SourceManagementActionError> {
    if request.input.schema_version != SCHEMA_VERSION {
        return Err(SourceManagementActionError::InvalidRequest(
            "unsupported_schema_version".to_string(),
        ));
    }
    let entity_type = normalize_required_filter(request.input.entity_type.clone());
    let entity_id = normalize_required_filter(request.input.entity_id.clone());
    if entity_type.is_empty() || entity_id.is_empty() {
        return Err(SourceManagementActionError::InvalidRequest(
            "entity_type_and_entity_id_are_required".to_string(),
        ));
    }
    validate_entity_type(&entity_type)
        .map_err(|error| SourceManagementActionError::InvalidRequest(error.to_string()))?;
    if !valid_surface_source_handle(&request.input.source_key)
        || !request.input.source_key.starts_with("source:v1:")
    {
        return Err(SourceManagementActionError::InvalidRequest(
            "invalid_source_key".to_string(),
        ));
    }
    if request.actor_id.trim().is_empty() || request.actor_id.len() > 160 {
        return Err(SourceManagementActionError::InvalidRequest(
            "invalid_actor".to_string(),
        ));
    }
    Ok(())
}

fn resolve_action_target(
    request: &SourceManagementActionRequest,
    lifecycle_rows: &BTreeMap<String, LifecycleRow>,
    active_links: &[LinkRow],
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<ActionTarget, SourceManagementActionError> {
    let query = NormalizedQuery {
        entity_type: normalize_required_filter(request.input.entity_type.clone()),
        entity_id: normalize_required_filter(request.input.entity_id.clone()),
        page_size: 1,
        cursor: None,
        privacy_profile: SourceManagementLedgerPrivacyProfile::SurfaceClient,
    };
    let mut matches = lifecycle_rows.values().filter(|lifecycle| {
        diagnostic_key.workspace_source_handle(&lifecycle.file_id) == request.input.source_key
            && lifecycle_matches_entity(lifecycle, active_links, &query)
    });
    let Some(lifecycle) = matches.next() else {
        return Err(SourceManagementActionError::InvalidRequest(
            "source_not_found_for_entity".to_string(),
        ));
    };
    if matches.next().is_some() {
        return Err(SourceManagementActionError::ActionFailed(
            "source key resolved ambiguously".to_string(),
        ));
    }
    Ok(ActionTarget {
        lifecycle: lifecycle.clone(),
    })
}

fn lifecycle_matches_entity(
    lifecycle: &LifecycleRow,
    active_links: &[LinkRow],
    query: &NormalizedQuery,
) -> bool {
    let linked = active_links.iter().any(|link| {
        link.file_id == lifecycle.file_id
            && entity_matches_query(query, &link.entity_type, &link.entity_id)
    });
    linked
        || lifecycle
            .entity_type
            .as_deref()
            .zip(lifecycle.entity_id.as_deref())
            .is_some_and(|(entity_type, entity_id)| {
                entity_matches_query(query, entity_type, entity_id)
            })
}

fn apply_reingest_action(
    ctx: &ServiceContext<'_>,
    workspace_root: PathBuf,
    signal_engine: Option<Arc<PropagationEngine>>,
    request: &SourceManagementActionRequest,
    target: &ActionTarget,
    conn: &Connection,
) -> Result<SourceManagementActionReceipt, SourceManagementActionError> {
    let receipt = crate::services::workspace_ingestion::workspace_intake_impl::ingest_sync(
        ctx,
        workspace_root,
        signal_engine,
        WorkspaceIntakeRequest {
            file_ref: target.lifecycle.canonical_path.clone(),
            source_type_slug: target.lifecycle.source_type.clone(),
            entity: Some(EntityRefDto {
                entity_type_slug: normalize_required_filter(request.input.entity_type.clone()),
                entity_id: normalize_required_filter(request.input.entity_id.clone()),
                entity_name: None,
            }),
            mode_slug: "forced".to_string(),
            category_slug: target.lifecycle.category.clone(),
        },
        request.actor_id.clone(),
    )
    .map_err(|error| SourceManagementActionError::ActionFailed(error.to_string()))?;

    Ok(action_receipt(
        request,
        "reingested",
        &receipt.lifecycle_state_after_slug,
        latest_run_for_file(conn, &receipt.file_id)?,
    ))
}

fn apply_quarantine_action(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    signal_engine: Option<Arc<PropagationEngine>>,
    request: &SourceManagementActionRequest,
    target: &ActionTarget,
) -> Result<SourceManagementActionReceipt, SourceManagementActionError> {
    let propagation = signal_engine.as_deref().ok_or_else(|| {
        SourceManagementActionError::ActionFailed("signal propagation unavailable".to_string())
    })?;
    let signal_ctx = SignalEmitContext::new(ctx, db, Some(propagation));
    let emitter = WorkspaceSignalEmitter;
    let reason = request
        .input
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .unwrap_or("user_quarantine");
    quarantine_source(
        &signal_ctx,
        &emitter,
        &target.lifecycle.file_id,
        reason,
        QuarantineActor::User {
            user_id: request.actor_id.clone(),
        },
    )
    .map_err(|error| SourceManagementActionError::ActionFailed(error.to_string()))?;
    Ok(action_receipt(
        request,
        "quarantined",
        "quarantined",
        latest_run_for_file(db.conn_ref(), &target.lifecycle.file_id)?,
    ))
}

fn apply_relink_action(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    signal_engine: Option<Arc<PropagationEngine>>,
    request: &SourceManagementActionRequest,
    target: &ActionTarget,
) -> Result<SourceManagementActionReceipt, SourceManagementActionError> {
    let propagation = signal_engine.as_deref().ok_or_else(|| {
        SourceManagementActionError::ActionFailed("signal propagation unavailable".to_string())
    })?;
    let entity_type = parse_entity_type(&request.input.entity_type)?;
    let entity_id = normalize_required_filter(request.input.entity_id.clone());
    let signal_ctx = SignalEmitContext::new(ctx, db, Some(propagation));
    let emitter = WorkspaceSignalEmitter;
    match LinkRepo::override_link(
        db.conn_ref(),
        &signal_ctx,
        &emitter,
        &target.lifecycle.file_id,
        entity_type,
        &entity_id,
        &request.actor_id,
    ) {
        Ok(()) => {}
        Err(LinkError::NotFound) => {
            LinkRepo::add_link(
                db.conn_ref(),
                &target.lifecycle.file_id,
                entity_type,
                &entity_id,
                LinkAttributionSource::UserRelink,
                1.0,
                Some("user confirmed source link"),
                &request.actor_id,
            )
            .map_err(|error| SourceManagementActionError::ActionFailed(error.to_string()))?;
            emitter
                .emit_link_changed(
                    &signal_ctx,
                    &target.lifecycle.file_id,
                    &request.input.entity_type,
                    &entity_id,
                    &request.actor_id,
                )
                .map_err(|error| SourceManagementActionError::ActionFailed(error.to_string()))?;
        }
        Err(error) => return Err(SourceManagementActionError::ActionFailed(error.to_string())),
    }
    Ok(action_receipt(
        request,
        "linked",
        &target.lifecycle.lifecycle_state,
        latest_run_for_file(db.conn_ref(), &target.lifecycle.file_id)?,
    ))
}

fn apply_lifecycle_policy_action(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    signal_engine: Option<Arc<PropagationEngine>>,
    request: &SourceManagementActionRequest,
    target: &ActionTarget,
    target_state: LifecycleState,
    status: &str,
) -> Result<SourceManagementActionReceipt, SourceManagementActionError> {
    let propagation = signal_engine.as_deref().ok_or_else(|| {
        SourceManagementActionError::ActionFailed("signal propagation unavailable".to_string())
    })?;
    let entity_type = normalize_required_filter(request.input.entity_type.clone());
    let entity_id = normalize_required_filter(request.input.entity_id.clone());
    let reason = request
        .input
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .unwrap_or("user_requested");
    let action_slug = action_slug(request.input.action);
    let target_lifecycle = lifecycle_state_slug(target_state);

    db.with_transaction(|tx_db| {
        let tx_conn = tx_db.conn_ref();
        let current =
            lifecycle_state_from_slug(&target.lifecycle.lifecycle_state).ok_or_else(|| {
                format!(
                    "unknown source lifecycle state: {}",
                    target.lifecycle.lifecycle_state
                )
            })?;
        LifecycleRepo::record_user_override(tx_conn, &target.lifecycle.file_id, &request.actor_id)
            .map_err(|error| error.to_string())?;
        LifecycleRepo::transition(tx_conn, &target.lifecycle.file_id, current, target_state)
            .map_err(|error| error.to_string())?;
        let signal_ctx = SignalEmitContext::new(ctx, tx_db, Some(propagation));
        emit_source_policy_changed(
            &signal_ctx,
            WorkspaceSourcePolicyChangedInput {
                source_handle: &request.input.source_key,
                policy_action: action_slug,
                reason,
                actor: &request.actor_id,
                entity_type: Some(entity_type.as_str()),
                entity_id: Some(entity_id.as_str()),
                policy_receipt_id: None,
            },
        )
        .map_err(|error| error.to_string())
    })
    .map_err(SourceManagementActionError::ActionFailed)?;

    Ok(action_receipt(
        request,
        status,
        target_lifecycle,
        latest_run_for_file(db.conn_ref(), &target.lifecycle.file_id)?,
    ))
}

fn action_slug(action: SourceManagementActionKind) -> &'static str {
    match action {
        SourceManagementActionKind::Reingest => "reingest",
        SourceManagementActionKind::Quarantine => "quarantine",
        SourceManagementActionKind::Relink => "relink",
        SourceManagementActionKind::Ignore => "ignore",
        SourceManagementActionKind::Scratchpad => "scratchpad",
        SourceManagementActionKind::Archive => "archive",
        SourceManagementActionKind::Delete => "delete",
    }
}

fn action_receipt(
    request: &SourceManagementActionRequest,
    status: &str,
    lifecycle_state: &str,
    latest_run: Option<SourceManagementIngestionRun>,
) -> SourceManagementActionReceipt {
    SourceManagementActionReceipt {
        schema_version: SCHEMA_VERSION,
        action: request.input.action,
        status: status.to_string(),
        source_key: request.input.source_key.clone(),
        lifecycle_state: lifecycle_state.to_string(),
        latest_run,
    }
}

fn latest_run_for_file(
    conn: &Connection,
    file_id: &str,
) -> Result<Option<SourceManagementIngestionRun>, SourceManagementActionError> {
    let history = load_run_history(conn).map_err(action_read_failed)?;
    Ok(history.get(file_id).and_then(|runs| runs.first().cloned()))
}

fn parse_entity_type(entity_type: &str) -> Result<EntityType, SourceManagementActionError> {
    match normalize_required_filter(entity_type.to_string()).as_str() {
        "account" => Ok(EntityType::Account),
        "person" => Ok(EntityType::Person),
        "project" => Ok(EntityType::Project),
        _ => Err(SourceManagementActionError::InvalidRequest(
            "unsupported_entity_type".to_string(),
        )),
    }
}

fn action_read_failed(error: SourceManagementLedgerReadError) -> SourceManagementActionError {
    SourceManagementActionError::ActionFailed(error.to_string())
}

fn read_source_management_ledger_snapshot(
    conn: &Connection,
    request: SourceManagementLedgerReadRequest,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<SourceManagementLedgerResponse, SourceManagementLedgerReadError> {
    let query = normalize_query(request)?;
    let request_fingerprint = request_fingerprint(&query);
    let offset = cursor_offset(query.cursor.as_ref(), &request_fingerprint)?;
    let lifecycle_rows = load_lifecycle_rows(conn)?;
    let active_links = load_active_links(conn)?;
    let run_history = load_run_history(conn)?;
    let placement_handles = load_placement_preview_handles(conn)?;
    let claims = load_workspace_claims(conn)?;

    let mut sources = build_sources(
        &query,
        &lifecycle_rows,
        &active_links,
        &run_history,
        &placement_handles,
        &claims,
        diagnostic_key,
    );
    sources.sort_by(source_order);

    let start = offset as usize;
    let page_size = query.page_size as usize;
    let end = start.saturating_add(page_size).min(sources.len());
    let page_sources = if start >= sources.len() {
        Vec::new()
    } else {
        sources[start..end].to_vec()
    };
    let has_more = end < sources.len();
    let next_cursor = if has_more {
        Some(encode_cursor(CursorPayload {
            schema_version: SCHEMA_VERSION,
            offset: end as u64,
            request_fingerprint,
        })?)
    } else {
        None
    };

    Ok(SourceManagementLedgerResponse {
        schema_version: SCHEMA_VERSION,
        page: SourceManagementLedgerPage {
            next_cursor,
            has_more,
        },
        action_policy: enabled_action_policy(),
        sources: page_sources,
    })
}

fn normalize_query(
    request: SourceManagementLedgerReadRequest,
) -> Result<NormalizedQuery, SourceManagementLedgerReadError> {
    let input = request.input;
    if input.schema_version != SCHEMA_VERSION {
        return Err(SourceManagementLedgerReadError::InvalidFilter(
            "unsupported_schema_version".to_string(),
        ));
    }
    let entity_type = normalize_required_filter(input.entity_type);
    let entity_id = normalize_required_filter(input.entity_id);
    if entity_type.is_empty() || entity_id.is_empty() {
        return Err(SourceManagementLedgerReadError::InvalidFilter(
            "entity_type_and_entity_id_are_required".to_string(),
        ));
    }
    validate_entity_type(&entity_type)?;

    let page_size = if input.page_size == 0 {
        DEFAULT_PAGE_SIZE
    } else if input.page_size > MAX_PAGE_SIZE {
        return Err(SourceManagementLedgerReadError::PageSizeTooLarge {
            requested: input.page_size,
            max: MAX_PAGE_SIZE,
        });
    } else {
        input.page_size
    };

    Ok(NormalizedQuery {
        entity_type,
        entity_id,
        page_size,
        cursor: input.cursor,
        privacy_profile: request.privacy_profile,
    })
}

fn normalize_required_filter(value: String) -> String {
    value.trim().to_ascii_lowercase()
}

fn validate_entity_type(entity_type: &str) -> Result<(), SourceManagementLedgerReadError> {
    if matches!(entity_type, "account" | "person" | "project") {
        return Ok(());
    }
    Err(SourceManagementLedgerReadError::InvalidFilter(format!(
        "unsupported_entity_type:{entity_type}"
    )))
}

fn request_fingerprint(query: &NormalizedQuery) -> String {
    let raw = serde_json::json!({
        "schemaVersion": SCHEMA_VERSION,
        "entityType": query.entity_type,
        "entityId": query.entity_id,
        "pageSize": query.page_size,
        "privacyProfile": query.privacy_profile,
    });
    let digest = sha2::Sha256::digest(raw.to_string().as_bytes());
    hex::encode(&digest[..16])
}

fn cursor_offset(
    cursor: Option<&Cursor>,
    request_fingerprint: &str,
) -> Result<u64, SourceManagementLedgerReadError> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    let payload = decode_cursor(cursor)?;
    if payload.schema_version != SCHEMA_VERSION {
        return Err(SourceManagementLedgerReadError::InvalidCursor(
            "unsupported_schema_version".to_string(),
        ));
    }
    if payload.request_fingerprint != request_fingerprint {
        return Err(SourceManagementLedgerReadError::InvalidCursor(
            "request_filter_changed_restart_required".to_string(),
        ));
    }
    Ok(payload.offset)
}

fn encode_cursor(payload: CursorPayload) -> Result<Cursor, SourceManagementLedgerReadError> {
    serde_json::to_vec(&payload)
        .map(|bytes| Cursor::new(URL_SAFE_NO_PAD.encode(bytes)))
        .map_err(|error| SourceManagementLedgerReadError::InvalidCursor(error.to_string()))
}

fn decode_cursor(cursor: &Cursor) -> Result<CursorPayload, SourceManagementLedgerReadError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(cursor.as_str())
        .map_err(|_| SourceManagementLedgerReadError::InvalidCursor("malformed".to_string()))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| SourceManagementLedgerReadError::InvalidCursor("malformed".to_string()))
}

fn load_lifecycle_rows(
    conn: &Connection,
) -> Result<BTreeMap<String, LifecycleRow>, SourceManagementLedgerReadError> {
    let mut stmt = conn
        .prepare(
            "SELECT file_id, canonical_path, source_type, lifecycle_state, source_asof, category,
                    user_override_at, entity_type, entity_id
             FROM workspace_file_lifecycle",
        )
        .map_err(read_failed)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(LifecycleRow {
                file_id: row.get(0)?,
                canonical_path: row.get(1)?,
                source_type: row.get(2)?,
                lifecycle_state: row.get(3)?,
                source_asof: row.get(4)?,
                category: row.get(5)?,
                user_override_at: row.get(6)?,
                entity_type: row.get(7)?,
                entity_id: row.get(8)?,
            })
        })
        .map_err(read_failed)?;
    let mut map = BTreeMap::new();
    for row in rows {
        let row = row.map_err(read_failed)?;
        map.insert(row.file_id.clone(), row);
    }
    Ok(map)
}

fn load_active_links(conn: &Connection) -> Result<Vec<LinkRow>, SourceManagementLedgerReadError> {
    let mut stmt = conn
        .prepare(
            "SELECT file_id, entity_type, entity_id, attribution_source, confidence,
                    user_override_at
             FROM document_entity_links
             WHERE rejected = 0",
        )
        .map_err(read_failed)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(LinkRow {
                file_id: row.get(0)?,
                entity_type: row.get(1)?,
                entity_id: row.get(2)?,
                attribution_source: row.get(3)?,
                confidence: row.get(4)?,
                user_override_at: row.get(5)?,
            })
        })
        .map_err(read_failed)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(read_failed)
}

fn load_run_history(
    conn: &Connection,
) -> Result<BTreeMap<String, Vec<SourceManagementIngestionRun>>, SourceManagementLedgerReadError> {
    let mut stmt = conn
        .prepare(
            "SELECT file_id, mode, started_at, completed_at, status, claim_count_produced
             FROM document_ingestion_runs
             ORDER BY file_id, started_at DESC, id DESC",
        )
        .map_err(read_failed)?;
    let rows = stmt
        .query_map([], |row| {
            let claim_count: i64 = row.get(5)?;
            Ok((
                row.get::<_, String>(0)?,
                SourceManagementIngestionRun {
                    mode: row.get(1)?,
                    started_at: row.get(2)?,
                    completed_at: row.get(3)?,
                    status: row.get(4)?,
                    claim_count_produced: claim_count.max(0).min(u32::MAX as i64) as u32,
                },
            ))
        })
        .map_err(read_failed)?;
    let mut history = BTreeMap::<String, Vec<SourceManagementIngestionRun>>::new();
    for row in rows {
        let (file_id, run) = row.map_err(read_failed)?;
        let runs = history.entry(file_id).or_default();
        if runs.len() < MAX_RUN_HISTORY_PER_SOURCE {
            runs.push(run);
        }
    }
    Ok(history)
}

fn load_placement_preview_handles(
    conn: &Connection,
) -> Result<BTreeMap<String, PlacementPreviewRow>, SourceManagementLedgerReadError> {
    let mut stmt = conn
        .prepare(
            "SELECT file_id, source_handle, content_type
             FROM workspace_placement_idempotency
             WHERE status = 'succeeded' AND source_handle IS NOT NULL
             ORDER BY file_id, updated_at DESC, idempotency_id DESC",
        )
        .map_err(read_failed)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                PlacementPreviewRow {
                    source_handle: row.get(1)?,
                    content_type: row.get(2)?,
                },
            ))
        })
        .map_err(read_failed)?;
    let mut map = BTreeMap::new();
    for row in rows {
        let (file_id, placement) = row.map_err(read_failed)?;
        map.entry(file_id).or_insert(placement);
    }
    Ok(map)
}

fn load_workspace_claims(
    conn: &Connection,
) -> Result<Vec<WorkspaceClaim>, SourceManagementLedgerReadError> {
    let mut claims = BTreeMap::<String, WorkspaceClaim>::new();
    let mut stmt = conn
        .prepare(
            "SELECT id, subject_ref, trust_score, sensitivity, source_ref
             FROM intelligence_claims
             WHERE claim_state = 'active' AND surfacing_state = 'active'",
        )
        .map_err(read_failed)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<f64>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(read_failed)?;
    for row in rows {
        let (claim_id, subject_ref, trust_score, sensitivity, source_ref) =
            row.map_err(read_failed)?;
        let mut source_file_ids = BTreeSet::new();
        if let Some(file_id) = workspace_file_id_from_source_ref(source_ref.as_deref()) {
            source_file_ids.insert(file_id.to_string());
        }
        claims.insert(
            claim_id,
            WorkspaceClaim {
                subjects: subjects_from_json(&subject_ref),
                source_file_ids,
                trust_score,
                sensitivity,
            },
        );
    }

    if table_exists(conn, "claim_semantic_evidence")? {
        let mut stmt = conn
            .prepare(
                "SELECT canonical_claim_id, source_ref
                 FROM claim_semantic_evidence
                 WHERE source_ref IS NOT NULL",
            )
            .map_err(read_failed)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .map_err(read_failed)?;
        for row in rows {
            let (claim_id, source_ref) = row.map_err(read_failed)?;
            let Some(file_id) = workspace_file_id_from_source_ref(source_ref.as_deref()) else {
                continue;
            };
            if let Some(claim) = claims.get_mut(&claim_id) {
                claim.source_file_ids.insert(file_id.to_string());
            }
        }
    }

    Ok(claims
        .into_values()
        .filter(|claim| !claim.source_file_ids.is_empty())
        .collect())
}

fn build_sources(
    query: &NormalizedQuery,
    lifecycle_rows: &BTreeMap<String, LifecycleRow>,
    active_links: &[LinkRow],
    run_history: &BTreeMap<String, Vec<SourceManagementIngestionRun>>,
    placement_handles: &BTreeMap<String, PlacementPreviewRow>,
    claims: &[WorkspaceClaim],
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Vec<SourceManagementSource> {
    let source_context = SourceBuildContext {
        run_history,
        placement_handles,
        claims,
        privacy_profile: query.privacy_profile,
        diagnostic_key,
    };
    let links_by_file =
        active_links
            .iter()
            .fold(BTreeMap::<String, Vec<&LinkRow>>::new(), |mut map, link| {
                map.entry(link.file_id.clone()).or_default().push(link);
                map
            });
    let mut sources = Vec::new();
    for lifecycle in lifecycle_rows.values() {
        let mut produced_from_link = false;
        if let Some(links) = links_by_file.get(&lifecycle.file_id) {
            for link in links {
                if !entity_matches_query(query, &link.entity_type, &link.entity_id) {
                    continue;
                }
                produced_from_link = true;
                let subject = EntitySubject {
                    entity_type: link.entity_type.clone(),
                    entity_id: link.entity_id.clone(),
                };
                sources.push(source_from_parts(
                    lifecycle,
                    Some(entity_from_link(link)),
                    Some(&subject),
                    &source_context,
                ));
            }
        }
        if produced_from_link {
            continue;
        }
        let Some(entity_type) = lifecycle.entity_type.as_deref() else {
            continue;
        };
        let Some(entity_id) = lifecycle.entity_id.as_deref() else {
            continue;
        };
        if !entity_matches_query(query, entity_type, entity_id) {
            continue;
        }
        let subject = EntitySubject {
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
        };
        sources.push(source_from_parts(
            lifecycle,
            Some(SourceManagementEntity {
                entity_type: entity_type.to_string(),
                entity_id: entity_id.to_string(),
                attribution_source: "lifecycle_binding".to_string(),
                confidence_bps: 10_000,
                user_override_at: lifecycle.user_override_at.clone(),
            }),
            Some(&subject),
            &source_context,
        ));
    }
    sources
}

fn source_from_parts(
    lifecycle: &LifecycleRow,
    entity: Option<SourceManagementEntity>,
    subject: Option<&EntitySubject>,
    context: &SourceBuildContext<'_>,
) -> SourceManagementSource {
    let ingestion_runs = context
        .run_history
        .get(&lifecycle.file_id)
        .cloned()
        .unwrap_or_default();
    let latest_run = ingestion_runs.first().cloned();
    let source_handle = context
        .placement_handles
        .get(&lifecycle.file_id)
        .filter(|placement| {
            valid_surface_source_handle(&placement.source_handle)
                && markdown_content_type(placement.content_type.as_deref())
        })
        .map(|placement| placement.source_handle.clone());
    SourceManagementSource {
        source_key: context
            .diagnostic_key
            .workspace_source_handle(&lifecycle.file_id),
        preview_available: source_handle.is_some(),
        source_handle,
        source_kind: lifecycle.source_type.clone(),
        lifecycle_state: lifecycle.lifecycle_state.clone(),
        category: lifecycle.category.clone(),
        source_asof: lifecycle.source_asof.clone(),
        entity,
        user_override: lifecycle
            .user_override_at
            .clone()
            .map(|at| SourceManagementUserOverride { present: true, at }),
        latest_run,
        ingestion_runs,
        trust_band_summary: trust_summary_for_source(
            &lifecycle.file_id,
            subject,
            context.claims,
            context.privacy_profile,
        ),
        actions: source_actions_for(lifecycle),
    }
}

fn entity_from_link(link: &LinkRow) -> SourceManagementEntity {
    SourceManagementEntity {
        entity_type: link.entity_type.clone(),
        entity_id: link.entity_id.clone(),
        attribution_source: link.attribution_source.clone(),
        confidence_bps: confidence_bps(link.confidence),
        user_override_at: link.user_override_at.clone(),
    }
}

fn entity_matches_query(query: &NormalizedQuery, entity_type: &str, entity_id: &str) -> bool {
    query.entity_type == entity_type && query.entity_id == entity_id
}

fn confidence_bps(confidence: f64) -> u16 {
    if !confidence.is_finite() {
        return 0;
    }
    (confidence.clamp(0.0, 1.0) * 10_000.0).round() as u16
}

fn valid_surface_source_handle(source_handle: &str) -> bool {
    !source_handle.is_empty()
        && source_handle.len() <= 160
        && source_handle
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '-'))
}

fn markdown_content_type(content_type: Option<&str>) -> bool {
    matches!(
        content_type,
        Some("text/markdown") | Some("text/x-markdown")
    )
}

fn trust_summary_for_source(
    file_id: &str,
    subject: Option<&EntitySubject>,
    claims: &[WorkspaceClaim],
    privacy_profile: SourceManagementLedgerPrivacyProfile,
) -> SourceManagementTrustBandSummary {
    let mut summary = SourceManagementTrustBandSummary::default();
    for claim in claims {
        if !claim.source_file_ids.contains(file_id) || !sensitivity_allowed(claim, privacy_profile)
        {
            continue;
        }
        if subject.is_some_and(|subject| !claim.subjects.contains(subject)) {
            continue;
        }
        summary.total += 1;
        match claim_trust_band_from_score(claim.trust_score) {
            TrustBand::LikelyCurrent => summary.likely_current += 1,
            TrustBand::UseWithCaution => summary.use_with_caution += 1,
            TrustBand::NeedsVerification => summary.needs_verification += 1,
            TrustBand::Unscored => summary.unscored += 1,
        }
    }
    summary
}

fn sensitivity_allowed(
    claim: &WorkspaceClaim,
    privacy_profile: SourceManagementLedgerPrivacyProfile,
) -> bool {
    match privacy_profile {
        SourceManagementLedgerPrivacyProfile::FirstParty => true,
        SourceManagementLedgerPrivacyProfile::SurfaceClient => {
            matches!(claim.sensitivity.as_str(), "public" | "internal")
        }
    }
}

fn source_order(a: &SourceManagementSource, b: &SourceManagementSource) -> std::cmp::Ordering {
    b.source_asof
        .cmp(&a.source_asof)
        .then_with(|| a.source_key.cmp(&b.source_key))
        .then_with(|| {
            a.entity
                .as_ref()
                .map(|entity| (&entity.entity_type, &entity.entity_id))
                .cmp(
                    &b.entity
                        .as_ref()
                        .map(|entity| (&entity.entity_type, &entity.entity_id)),
                )
        })
}

fn enabled_action_policy() -> SourceManagementActionPolicy {
    SourceManagementActionPolicy {
        reingest_enabled: true,
        quarantine_enabled: true,
        relink_enabled: true,
        ignore_enabled: true,
        scratchpad_enabled: true,
        archive_enabled: true,
        delete_enabled: true,
        disabled_reason: ACTIONS_READY_REASON.to_string(),
    }
}

fn source_actions_for(lifecycle: &LifecycleRow) -> SourceManagementSourceActions {
    let can_quarantine = lifecycle.lifecycle_state != "quarantined";
    let already_ignored = lifecycle.lifecycle_state == "ignored";
    let already_scratchpad = lifecycle.lifecycle_state == "scratchpad";
    let already_archived = lifecycle.lifecycle_state == "archived";
    let already_deleted = lifecycle.lifecycle_state == "deleted";
    SourceManagementSourceActions {
        can_reingest: true,
        can_quarantine,
        can_relink: true,
        can_ignore: !already_ignored,
        can_scratchpad: !already_scratchpad,
        can_archive: !already_archived,
        can_delete: !already_deleted,
        disabled_reason: if can_quarantine {
            ACTIONS_READY_REASON
        } else {
            ACTION_ALREADY_QUARANTINED_REASON
        }
        .to_string(),
    }
}

fn workspace_file_id_from_source_ref(source_ref: Option<&str>) -> Option<&str> {
    source_ref?.strip_prefix(WORKSPACE_SOURCE_PREFIX)
}

fn subjects_from_json(raw: &str) -> BTreeSet<EntitySubject> {
    if let Ok(subject) = serde_json::from_str::<SubjectRef>(raw) {
        return subjects_from_ref(&subject);
    }
    let Some(value) = serde_json::from_str::<serde_json::Value>(raw).ok() else {
        return BTreeSet::new();
    };
    if let Ok(subject) = claim_subject_ref_from_json(&value) {
        return subjects_from_claim_ref(&subject);
    }
    let Some(object) = value.as_object() else {
        return BTreeSet::new();
    };
    for key in ["account", "person", "project"] {
        if let Some(id) = object.get(key).and_then(|v| v.as_str()) {
            return BTreeSet::from([EntitySubject {
                entity_type: key.to_string(),
                entity_id: id.to_string(),
            }]);
        }
    }
    BTreeSet::new()
}

fn subjects_from_claim_ref(subject: &ClaimSubjectRef) -> BTreeSet<EntitySubject> {
    match subject {
        ClaimSubjectRef::Account { id } => BTreeSet::from([EntitySubject {
            entity_type: "account".to_string(),
            entity_id: id.clone(),
        }]),
        ClaimSubjectRef::Person { id } => BTreeSet::from([EntitySubject {
            entity_type: "person".to_string(),
            entity_id: id.clone(),
        }]),
        ClaimSubjectRef::Project { id } => BTreeSet::from([EntitySubject {
            entity_type: "project".to_string(),
            entity_id: id.clone(),
        }]),
        ClaimSubjectRef::Multi(subjects) => subjects
            .iter()
            .flat_map(subjects_from_claim_ref)
            .collect::<BTreeSet<_>>(),
        ClaimSubjectRef::Action { .. }
        | ClaimSubjectRef::Meeting { .. }
        | ClaimSubjectRef::Email { .. }
        | ClaimSubjectRef::Global => BTreeSet::new(),
    }
}

fn subjects_from_ref(subject: &SubjectRef) -> BTreeSet<EntitySubject> {
    match subject {
        SubjectRef::Account(id) => BTreeSet::from([EntitySubject {
            entity_type: "account".to_string(),
            entity_id: id.clone(),
        }]),
        SubjectRef::Person(id) => BTreeSet::from([EntitySubject {
            entity_type: "person".to_string(),
            entity_id: id.clone(),
        }]),
        SubjectRef::Project(id) => BTreeSet::from([EntitySubject {
            entity_type: "project".to_string(),
            entity_id: id.clone(),
        }]),
        SubjectRef::Action(id) => BTreeSet::from([EntitySubject {
            entity_type: "action".to_string(),
            entity_id: id.clone(),
        }]),
        SubjectRef::Multi(subjects) => subjects
            .iter()
            .flat_map(subjects_from_ref)
            .collect::<BTreeSet<_>>(),
        SubjectRef::Global | SubjectRef::Meeting(_) | SubjectRef::User(_) | SubjectRef::Unknown => {
            BTreeSet::new()
        }
    }
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, SourceManagementLedgerReadError> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
        )",
        params![table],
        |row| row.get::<_, i64>(0),
    )
    .map(|exists| exists == 1)
    .map_err(read_failed)
}

fn read_failed(error: rusqlite::Error) -> SourceManagementLedgerReadError {
    SourceManagementLedgerReadError::ReadFailed(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("conn");
        conn.execute_batch(
            "CREATE TABLE workspace_file_lifecycle (
                file_id TEXT PRIMARY KEY,
                canonical_path TEXT NOT NULL,
                device INTEGER NOT NULL DEFAULT 1,
                inode INTEGER NOT NULL DEFAULT 1,
                source_type TEXT NOT NULL,
                data_source TEXT NOT NULL DEFAULT '{}',
                lifecycle_state TEXT NOT NULL,
                source_asof TEXT NOT NULL,
                entity_id TEXT,
                entity_type TEXT,
                content_sha256 TEXT,
                user_override_actor TEXT,
                user_override_at TEXT,
                category TEXT,
                created_at TEXT NOT NULL DEFAULT '2026-05-24T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-05-24T00:00:00Z'
             );
             CREATE TABLE document_entity_links (
                link_id TEXT NOT NULL UNIQUE,
                file_id TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                attribution_source TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.5,
                rationale TEXT,
                actor TEXT NOT NULL DEFAULT 'system',
                user_override_actor TEXT,
                user_override_at TEXT,
                rejected INTEGER NOT NULL DEFAULT 0,
                rejected_at TEXT,
                rejected_reason TEXT,
                created_at TEXT NOT NULL DEFAULT '2026-05-24T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-05-24T00:00:00Z'
             );
             CREATE TABLE document_ingestion_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL UNIQUE,
                file_id TEXT NOT NULL,
                mode TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                status TEXT NOT NULL,
                content_sha256 TEXT NOT NULL,
                file_size_bytes INTEGER NOT NULL,
                extractor_version TEXT NOT NULL,
                claim_count_produced INTEGER NOT NULL DEFAULT 0,
                error_log TEXT,
                retry_of_run_id TEXT
             );
             CREATE TABLE workspace_placement_idempotency (
                idempotency_id TEXT PRIMARY KEY,
                actor_id TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                content_sha256 TEXT NOT NULL,
                content_type TEXT NOT NULL,
                category_slug TEXT NOT NULL,
                client_dedup_key TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL,
                document_handle TEXT,
                source_handle TEXT,
                file_id TEXT,
                run_id TEXT,
                chosen_filename TEXT,
                source_asof TEXT,
                lifecycle_state TEXT,
                claim_count_produced INTEGER NOT NULL DEFAULT 0,
                error_code TEXT,
                started_at TEXT NOT NULL DEFAULT '2026-05-24T00:00:00Z',
                stale_after TEXT NOT NULL DEFAULT '2026-05-24T01:00:00Z',
                created_at TEXT NOT NULL DEFAULT '2026-05-24T00:00:00Z',
                updated_at TEXT NOT NULL DEFAULT '2026-05-24T00:00:00Z'
             );
             CREATE TABLE intelligence_claims (
                id TEXT PRIMARY KEY,
                subject_ref TEXT NOT NULL,
                claim_type TEXT NOT NULL,
                text TEXT NOT NULL,
                dedup_key TEXT NOT NULL,
                actor TEXT NOT NULL,
                data_source TEXT NOT NULL,
                source_ref TEXT,
                source_asof TEXT,
                observed_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                provenance_json TEXT NOT NULL,
                claim_state TEXT NOT NULL,
                surfacing_state TEXT NOT NULL,
                trust_score REAL,
                temporal_scope TEXT NOT NULL,
                sensitivity TEXT NOT NULL
             );",
        )
        .expect("schema");
        conn
    }

    fn diagnostic_key() -> WorkspaceGraphDiagnosticKey {
        WorkspaceGraphDiagnosticKey::for_tests("source-management-ledger")
    }

    fn default_request() -> SourceManagementLedgerReadRequest {
        SourceManagementLedgerReadRequest {
            input: abilities_runtime::abilities::source_management_ledger::contracts::SourceManagementLedgerInput {
                schema_version: 1,
                entity_type: "account".to_string(),
                entity_id: "acct-test-001".to_string(),
                cursor: None,
                page_size: 25,
            },
            privacy_profile: SourceManagementLedgerPrivacyProfile::SurfaceClient,
        }
    }

    #[test]
    fn source_management_input_requires_entity_scope() {
        let error = serde_json::from_value::<
            abilities_runtime::abilities::source_management_ledger::contracts::SourceManagementLedgerInput,
        >(serde_json::json!({
            "schemaVersion": 1,
            "pageSize": 25
        }))
        .expect_err("entity scope is required");

        assert!(
            error.to_string().contains("entityType"),
            "unexpected serde error: {error}"
        );
    }

    #[test]
    fn blank_entity_scope_is_rejected() {
        let conn = setup_conn();
        let mut request = default_request();
        request.input.entity_id = " ".to_string();

        let error = read_source_management_ledger(&conn, request, &diagnostic_key())
            .expect_err("blank entity rejected");

        assert!(matches!(
            error,
            SourceManagementLedgerReadError::InvalidFilter(message)
                if message == "entity_type_and_entity_id_are_required"
        ));
    }

    fn insert_workspace_source(conn: &Connection) {
        crate::services::workspace_ingestion::graph::insert_source_management_workspace_fixture_for_tests(conn);
        conn.execute(
            "INSERT INTO workspace_placement_idempotency (
                idempotency_id, actor_id, entity_type, entity_id, content_sha256,
                content_type, category_slug, status, source_handle, file_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                "idem-alpha",
                "surface-client",
                "account",
                "acct-test-001",
                "content-hash",
                "text/markdown",
                "notes",
                "succeeded",
                "source_opaque_alpha",
                "pathhash-alpha"
            ],
        )
        .expect("placement");
    }

    fn insert_claim(
        conn: &Connection,
        claim_id: &str,
        sensitivity: &str,
        trust_score: Option<f64>,
    ) {
        let subject = serde_json::to_string(&SubjectRef::Account("acct-test-001".to_string()))
            .expect("subject");
        conn.execute(
            "INSERT INTO intelligence_claims /* dos7-allowed: read-model fixture seeds claim trust aggregates */ (
                id, subject_ref, claim_type, text, dedup_key, actor, data_source,
                source_ref, source_asof, observed_at, created_at, provenance_json,
                claim_state, surfacing_state, trust_score, temporal_scope, sensitivity
             ) VALUES (?1, ?2, 'account_status', 'raw claim text must not render', ?1,
                'system', '{}', ?3, '2026-05-24T10:00:00Z',
                '2026-05-24T10:00:00Z', '2026-05-24T10:00:00Z', '{}',
                'active', 'active', ?4, 'state', ?5)",
            params![
                claim_id,
                subject,
                "workspace_file:pathhash-alpha",
                trust_score,
                sensitivity
            ],
        )
        .expect("claim");
    }

    #[test]
    fn entity_scoped_ledger_redacts_internal_ids_and_paths() {
        let conn = setup_conn();
        insert_workspace_source(&conn);
        insert_claim(&conn, "claim-alpha", "internal", Some(0.92));
        insert_claim(&conn, "claim-secret", "user_only", Some(0.92));

        let response = read_source_management_ledger(&conn, default_request(), &diagnostic_key())
            .expect("ledger");

        assert_eq!(response.sources.len(), 1);
        let source = &response.sources[0];
        assert!(source.source_key.starts_with("source:v1:"));
        assert_eq!(source.source_handle.as_deref(), Some("source_opaque_alpha"));
        assert_eq!(source.preview_available, true);
        assert_eq!(source.lifecycle_state, "ingested");
        assert_eq!(source.category.as_deref(), Some("notes"));
        assert_eq!(
            source.latest_run.as_ref().expect("latest run").status,
            "success"
        );
        assert_eq!(source.trust_band_summary.total, 1);
        assert_eq!(source.trust_band_summary.likely_current, 1);
        assert!(source.actions.can_reingest);
        assert!(source.actions.can_quarantine);
        assert!(source.actions.can_relink);
        assert_eq!(source.actions.disabled_reason, ACTIONS_READY_REASON);

        let serialized = serde_json::to_string(&response).expect("serialize");
        assert!(!serialized.contains("pathhash-alpha"));
        assert!(!serialized.contains("link-alpha"));
        assert!(!serialized.contains("run-alpha"));
        assert!(!serialized.contains("/Users/example"));
        assert!(!serialized.contains("raw claim text"));
        assert!(!serialized.contains("claim-secret"));
    }

    #[test]
    fn first_party_counts_private_claims_surface_client_does_not() {
        let conn = setup_conn();
        insert_workspace_source(&conn);
        insert_claim(&conn, "claim-internal", "internal", Some(0.92));
        insert_claim(&conn, "claim-private", "user_only", Some(0.92));

        let mut first_party = default_request();
        first_party.privacy_profile = SourceManagementLedgerPrivacyProfile::FirstParty;
        let response =
            read_source_management_ledger(&conn, first_party, &diagnostic_key()).expect("ledger");
        assert_eq!(response.sources[0].trust_band_summary.total, 2);

        let response = read_source_management_ledger(&conn, default_request(), &diagnostic_key())
            .expect("ledger");
        assert_eq!(response.sources[0].trust_band_summary.total, 1);
    }
}

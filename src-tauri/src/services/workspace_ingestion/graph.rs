//! Workspace graph projection and audit compiler.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use abilities_runtime::abilities::get_entity_intelligence::contracts::Cursor;
use abilities_runtime::abilities::provenance::subject::SubjectRef;
use abilities_runtime::abilities::provenance::trust::claim_trust_band_from_score;
use abilities_runtime::abilities::trust::types::TrustBand;
use abilities_runtime::abilities::workspace_graph::contracts::{
    WorkspaceGraphAudit, WorkspaceGraphAuditGap, WorkspaceGraphClaimSummary, WorkspaceGraphEntity,
    WorkspaceGraphFileLink, WorkspaceGraphInput, WorkspaceGraphNotModified, WorkspaceGraphPage,
    WorkspaceGraphPrivacyProfile, WorkspaceGraphProjection, WorkspaceGraphProjectionBody,
    WorkspaceGraphReadRequest, WorkspaceGraphResponse, WorkspaceGraphUserOverride,
};
use abilities_runtime::services::context::WorkspaceGraphReadError;
use abilities_runtime::types::{
    subject_ref_from_json as claim_subject_ref_from_json, ClaimSubjectRef,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::contracts::WorkspaceCategory;

type HmacSha256 = Hmac<Sha256>;

const SCHEMA_VERSION: u32 = 1;
const DEFAULT_PAGE_SIZE: u32 = 50;
const MAX_PAGE_SIZE: u32 = 200;
const WORKSPACE_SOURCE_PREFIX: &str = "workspace_file:";
const DIAGNOSTIC_KEY_DERIVATION_DOMAIN: &[u8] = b"DAILYOS-WORKSPACE-GRAPH-DIAGNOSTIC-HANDLE-V1\n";

#[derive(Clone)]
pub struct WorkspaceGraphDiagnosticKey {
    bytes: [u8; 32],
}

impl WorkspaceGraphDiagnosticKey {
    fn from_install_secret(secret: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(DIAGNOSTIC_KEY_DERIVATION_DOMAIN);
        hasher.update(secret);
        let digest = hasher.finalize();
        let mut bytes = [0_u8; 32];
        bytes.copy_from_slice(&digest);
        Self { bytes }
    }

    fn from_derived_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    #[cfg(any(test, feature = "test-harness", debug_assertions))]
    pub(crate) fn for_tests(label: &str) -> Self {
        Self::from_install_secret(label.as_bytes())
    }

    pub(crate) fn workspace_source_handle(&self, file_id: &str) -> String {
        diagnostic_handle("source", "workspace_file", file_id, self)
    }
}

impl fmt::Debug for WorkspaceGraphDiagnosticKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WorkspaceGraphDiagnosticKey([REDACTED])")
    }
}

impl Zeroize for WorkspaceGraphDiagnosticKey {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl ZeroizeOnDrop for WorkspaceGraphDiagnosticKey {}

pub fn local_install_diagnostic_key() -> Result<WorkspaceGraphDiagnosticKey, String> {
    let bytes = crate::db::local_db_workspace_graph_diagnostic_key_bytes()
        .map_err(|error| format!("diagnostic key unavailable: {error}"))?;
    Ok(WorkspaceGraphDiagnosticKey::from_derived_bytes(bytes))
}

#[cfg(any(test, feature = "test-harness", debug_assertions))]
#[doc(hidden)]
pub fn diagnostic_key_for_tests(label: &str) -> WorkspaceGraphDiagnosticKey {
    WorkspaceGraphDiagnosticKey::for_tests(label)
}

#[cfg(test)]
pub(crate) fn insert_source_management_workspace_fixture_for_tests(conn: &Connection) {
    conn.execute(
        "INSERT INTO workspace_file_lifecycle (
            file_id, canonical_path, source_type, lifecycle_state, source_asof,
            entity_type, entity_id, category, user_override_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            "pathhash-alpha",
            "/Users/example/workspace/private.md",
            "mcp_placement",
            "ingested",
            "2026-05-24T10:00:00Z",
            "account",
            "acct-test-001",
            "notes",
            "2026-05-24T11:00:00Z"
        ],
    )
    .expect("lifecycle");
    conn.execute(
        "INSERT INTO document_entity_links (
            link_id, file_id, entity_type, entity_id, attribution_source,
            confidence, user_override_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            "link-alpha",
            "pathhash-alpha",
            "account",
            "acct-test-001",
            "entity_hint",
            0.87_f64,
            "2026-05-24T11:30:00Z"
        ],
    )
    .expect("link");
    conn.execute(
        "INSERT INTO document_ingestion_runs (
            run_id, file_id, mode, started_at, completed_at, status,
            content_sha256, file_size_bytes, extractor_version,
            claim_count_produced
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            "run-alpha",
            "pathhash-alpha",
            "initial",
            "2026-05-24T10:05:00Z",
            "2026-05-24T10:05:05Z",
            "success",
            "content-hash",
            1024_i64,
            "extractor-v1",
            2_i64
        ],
    )
    .expect("run");
}

#[cfg(test)]
pub(crate) fn insert_markdown_preview_lifecycle_fixture_for_tests(
    conn: &Connection,
    canonical_path: &str,
) {
    conn.execute(
        "INSERT INTO workspace_file_lifecycle
            (file_id, canonical_path, source_asof, lifecycle_state)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            "file-alpha",
            canonical_path,
            "2026-05-24T10:00:00.000Z",
            "ingested"
        ],
    )
    .expect("lifecycle");
}

#[cfg(test)]
pub(crate) fn insert_markdown_preview_run_fixture_for_tests(conn: &Connection, run_id: &str) {
    conn.execute(
        "INSERT INTO document_ingestion_runs (run_id, file_id, status, started_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![run_id, "file-alpha", "success", "2026-05-24T10:00:01.000Z"],
    )
    .expect("run");
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CursorPayload {
    schema_version: u32,
    offset: u64,
    request_fingerprint: String,
    snapshot_graph_version: String,
}

#[derive(Debug, Clone)]
struct NormalizedQuery {
    entity_types: BTreeSet<String>,
    entity_ids: BTreeSet<String>,
    category_filter: BTreeSet<String>,
    include_entity_names: bool,
    page_size: u32,
    cursor: Option<Cursor>,
    if_none_match: Option<String>,
    privacy_profile: WorkspaceGraphPrivacyProfile,
}

#[derive(Debug, Clone, Serialize)]
struct FingerprintProjection<'a> {
    entities: &'a [WorkspaceGraphEntity],
    audit_gaps: &'a [WorkspaceGraphAuditGap],
    request_fingerprint: &'a str,
}

#[derive(Debug, Clone)]
struct LifecycleRow {
    file_id: String,
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
    link_handle: String,
    file_id: String,
    entity_type: String,
    entity_id: String,
    attribution_source: String,
    confidence: f64,
    user_override_at: Option<String>,
}

#[derive(Debug, Clone)]
struct FileLinkRow {
    lifecycle: LifecycleRow,
    link: LinkRow,
    source_handle: Option<String>,
}

#[derive(Debug, Clone)]
struct WorkspaceClaim {
    claim_handle: String,
    subjects: BTreeSet<EntitySubject>,
    source_file_ids: BTreeSet<String>,
    trust_score: Option<f64>,
    sensitivity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct EntitySubject {
    entity_type: String,
    entity_id: String,
}

pub fn read_workspace_graph(
    conn: &Connection,
    request: WorkspaceGraphReadRequest,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<WorkspaceGraphResponse, WorkspaceGraphReadError> {
    let tx = conn.unchecked_transaction().map_err(read_failed)?;
    let response = read_workspace_graph_snapshot(&tx, request, diagnostic_key)?;
    tx.commit().map_err(read_failed)?;
    Ok(response)
}

pub fn read_workspace_graph_from_local_db(
    request: WorkspaceGraphReadRequest,
) -> Result<WorkspaceGraphResponse, WorkspaceGraphReadError> {
    let db = crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
        .map_err(|error| WorkspaceGraphReadError::ReadFailed(error.to_string()))?;
    let diagnostic_key =
        local_install_diagnostic_key().map_err(WorkspaceGraphReadError::ReadFailed)?;
    read_workspace_graph(db.conn_ref(), request, &diagnostic_key)
}

fn read_workspace_graph_snapshot(
    conn: &Connection,
    request: WorkspaceGraphReadRequest,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<WorkspaceGraphResponse, WorkspaceGraphReadError> {
    let query = normalize_query(request.input, request.privacy_profile)?;
    let request_fingerprint = request_fingerprint(&query);

    if query.cursor.is_some() && query.if_none_match.is_some() {
        return Err(WorkspaceGraphReadError::InvalidCursor(
            "if_none_match_not_allowed_with_cursor".to_string(),
        ));
    }

    let lifecycle_rows = load_lifecycle_rows(conn)?;
    let active_links = load_active_links(conn)?;
    let run_handles = load_latest_run_handles(conn)?;
    let claims = load_workspace_claims(conn, diagnostic_key)?;

    let full_entities = build_entities(
        conn,
        &query,
        &lifecycle_rows,
        &active_links,
        &run_handles,
        &claims,
    )?;
    let mut audit_gaps = build_audit_gaps(
        &query,
        &lifecycle_rows,
        &active_links,
        &run_handles,
        &claims,
        diagnostic_key,
    );
    audit_gaps.sort_by(gap_order);
    let graph_version = graph_version(&full_entities, &audit_gaps, &request_fingerprint)?;

    if query.cursor.is_none()
        && query
            .if_none_match
            .as_deref()
            .is_some_and(|etag| etag == graph_version)
    {
        return Ok(WorkspaceGraphResponse::NotModified(
            WorkspaceGraphNotModified {
                schema_version: SCHEMA_VERSION,
                graph_version,
            },
        ));
    }

    let offset = cursor_offset(query.cursor.as_ref(), &request_fingerprint, &graph_version)?;
    let total_hint = full_entities.len() as u64;
    let page_size = query.page_size as usize;
    let start = offset as usize;
    let end = start.saturating_add(page_size).min(full_entities.len());
    let entities = if start >= full_entities.len() {
        Vec::new()
    } else {
        full_entities[start..end].to_vec()
    };
    let has_more = (end as u64) < total_hint;
    let next_cursor = if has_more {
        Some(encode_cursor(CursorPayload {
            schema_version: SCHEMA_VERSION,
            offset: end as u64,
            request_fingerprint,
            snapshot_graph_version: graph_version.clone(),
        })?)
    } else {
        None
    };

    let audit = audit_report(graph_version.clone(), audit_gaps);
    Ok(WorkspaceGraphResponse::Projection(
        WorkspaceGraphProjection {
            schema_version: SCHEMA_VERSION,
            graph_version,
            page: WorkspaceGraphPage {
                next_cursor,
                has_more,
            },
            projection: WorkspaceGraphProjectionBody { entities },
            audit,
        },
    ))
}

fn normalize_query(
    input: WorkspaceGraphInput,
    privacy_profile: WorkspaceGraphPrivacyProfile,
) -> Result<NormalizedQuery, WorkspaceGraphReadError> {
    if input.schema_version != SCHEMA_VERSION {
        return Err(WorkspaceGraphReadError::InvalidFilter(format!(
            "unsupported schema_version `{}`",
            input.schema_version
        )));
    }
    let page_size = if input.page_size == 0 {
        DEFAULT_PAGE_SIZE
    } else if input.page_size > MAX_PAGE_SIZE {
        return Err(WorkspaceGraphReadError::PageSizeTooLarge {
            requested: input.page_size,
            max: MAX_PAGE_SIZE,
        });
    } else {
        input.page_size
    };

    let mut entity_types = BTreeSet::new();
    let mut entity_ids = BTreeSet::new();
    if let Some(filter) = input.entity_filter {
        if let Some(kinds) = filter.entity_types {
            for kind in kinds {
                entity_types.insert(normalize_entity_type(&kind)?);
            }
        }
        if let Some(ids) = filter.entity_ids {
            for id in ids {
                let trimmed = id.trim();
                if trimmed.is_empty() {
                    return Err(WorkspaceGraphReadError::InvalidFilter(
                        "entity id filter cannot contain empty values".to_string(),
                    ));
                }
                entity_ids.insert(trimmed.to_string());
            }
        }
    }

    let mut category_filter = BTreeSet::new();
    if let Some(categories) = input.category_filter {
        for category in categories {
            let slug = category.trim();
            if slug == "inbox" || WorkspaceCategory::from_slug(slug).is_none() {
                return Err(WorkspaceGraphReadError::InvalidFilter(format!(
                    "unsupported category `{slug}`"
                )));
            }
            category_filter.insert(slug.to_string());
        }
    }

    Ok(NormalizedQuery {
        entity_types,
        entity_ids,
        category_filter,
        include_entity_names: input.include_entity_names,
        page_size,
        cursor: input.cursor,
        if_none_match: input.if_none_match,
        privacy_profile,
    })
}

fn normalize_entity_type(value: &str) -> Result<String, WorkspaceGraphReadError> {
    let trimmed = value.trim();
    match trimmed {
        "account" | "person" | "project" => Ok(trimmed.to_string()),
        other => Err(WorkspaceGraphReadError::InvalidFilter(format!(
            "unsupported entity_type `{other}`"
        ))),
    }
}

fn is_supported_entity_type(value: &str) -> bool {
    matches!(value, "account" | "person" | "project")
}

fn request_fingerprint(query: &NormalizedQuery) -> String {
    stable_hash(&json!({
        "schema_version": SCHEMA_VERSION,
        "entity_types": query.entity_types,
        "entity_ids": query.entity_ids,
        "category_filter": query.category_filter,
        "include_entity_names": query.include_entity_names,
        "page_size": query.page_size,
        "privacy_profile": query.privacy_profile,
    }))
}

fn graph_version(
    entities: &[WorkspaceGraphEntity],
    audit_gaps: &[WorkspaceGraphAuditGap],
    request_fingerprint: &str,
) -> Result<String, WorkspaceGraphReadError> {
    let value = FingerprintProjection {
        entities,
        audit_gaps,
        request_fingerprint,
    };
    serde_json::to_value(value)
        .map(|value| format!("v1:{}", stable_hash(&value)))
        .map_err(|error| WorkspaceGraphReadError::AuditFailed(error.to_string()))
}

fn stable_hash(value: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(
        serde_json::to_vec(value)
            .expect("serializing serde_json::Value should not fail")
            .as_slice(),
    );
    hex::encode(hasher.finalize())
}

fn cursor_offset(
    cursor: Option<&Cursor>,
    request_fingerprint: &str,
    graph_version: &str,
) -> Result<u64, WorkspaceGraphReadError> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    let payload = decode_cursor(cursor)?;
    if payload.schema_version != SCHEMA_VERSION {
        return Err(WorkspaceGraphReadError::InvalidCursor(
            "schema_mismatch".to_string(),
        ));
    }
    if payload.request_fingerprint != request_fingerprint {
        return Err(WorkspaceGraphReadError::InvalidCursor(
            "request_fingerprint_mismatch".to_string(),
        ));
    }
    if payload.snapshot_graph_version != graph_version {
        return Err(WorkspaceGraphReadError::InvalidCursor(
            "graph_version_changed_restart_required".to_string(),
        ));
    }
    Ok(payload.offset)
}

fn encode_cursor(payload: CursorPayload) -> Result<Cursor, WorkspaceGraphReadError> {
    serde_json::to_vec(&payload)
        .map(|bytes| Cursor::new(URL_SAFE_NO_PAD.encode(bytes)))
        .map_err(|error| WorkspaceGraphReadError::InvalidCursor(error.to_string()))
}

fn decode_cursor(cursor: &Cursor) -> Result<CursorPayload, WorkspaceGraphReadError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(cursor.as_str())
        .map_err(|_| WorkspaceGraphReadError::InvalidCursor("malformed".to_string()))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| WorkspaceGraphReadError::InvalidCursor("malformed".to_string()))
}

fn load_lifecycle_rows(
    conn: &Connection,
) -> Result<BTreeMap<String, LifecycleRow>, WorkspaceGraphReadError> {
    let mut stmt = conn
        .prepare(
            "SELECT file_id, source_type, lifecycle_state, source_asof, category,
                    user_override_at, entity_type, entity_id
             FROM workspace_file_lifecycle",
        )
        .map_err(read_failed)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(LifecycleRow {
                file_id: row.get(0)?,
                source_type: row.get(1)?,
                lifecycle_state: row.get(2)?,
                source_asof: row.get(3)?,
                category: row.get(4)?,
                user_override_at: row.get(5)?,
                entity_type: row.get(6)?,
                entity_id: row.get(7)?,
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

fn load_active_links(conn: &Connection) -> Result<Vec<LinkRow>, WorkspaceGraphReadError> {
    let mut stmt = conn
        .prepare(
            "SELECT link_id, file_id, entity_type, entity_id, attribution_source, confidence,
                    user_override_at
             FROM document_entity_links
             WHERE rejected = 0",
        )
        .map_err(read_failed)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(LinkRow {
                link_handle: row.get(0)?,
                file_id: row.get(1)?,
                entity_type: row.get(2)?,
                entity_id: row.get(3)?,
                attribution_source: row.get(4)?,
                confidence: row.get(5)?,
                user_override_at: row.get(6)?,
            })
        })
        .map_err(read_failed)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(read_failed)
}

fn load_latest_run_handles(
    conn: &Connection,
) -> Result<BTreeMap<String, String>, WorkspaceGraphReadError> {
    let mut stmt = conn
        .prepare(
            "SELECT file_id, run_id
             FROM document_ingestion_runs
             ORDER BY file_id, started_at DESC, id DESC",
        )
        .map_err(read_failed)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(read_failed)?;
    let mut map = BTreeMap::new();
    for row in rows {
        let (file_id, run_id) = row.map_err(read_failed)?;
        map.entry(file_id).or_insert(run_id);
    }
    Ok(map)
}

fn load_workspace_claims(
    conn: &Connection,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<Vec<WorkspaceClaim>, WorkspaceGraphReadError> {
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
            claim_id.clone(),
            WorkspaceClaim {
                claim_handle: diagnostic_handle("claim", "claim", &claim_id, diagnostic_key),
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

fn build_entities(
    conn: &Connection,
    query: &NormalizedQuery,
    lifecycle_rows: &BTreeMap<String, LifecycleRow>,
    active_links: &[LinkRow],
    run_handles: &BTreeMap<String, String>,
    claims: &[WorkspaceClaim],
) -> Result<Vec<WorkspaceGraphEntity>, WorkspaceGraphReadError> {
    let mut by_entity = BTreeMap::<EntitySubject, Vec<FileLinkRow>>::new();
    for link in active_links {
        if !entity_matches_filter(query, &link.entity_type, &link.entity_id) {
            continue;
        }
        let Some(lifecycle) = lifecycle_rows.get(&link.file_id) else {
            continue;
        };
        if lifecycle.lifecycle_state != "ingested" {
            continue;
        }
        if !query.category_filter.is_empty()
            && !lifecycle
                .category
                .as_deref()
                .is_some_and(|category| query.category_filter.contains(category))
        {
            continue;
        }
        let subject = EntitySubject {
            entity_type: link.entity_type.clone(),
            entity_id: link.entity_id.clone(),
        };
        by_entity.entry(subject).or_default().push(FileLinkRow {
            lifecycle: lifecycle.clone(),
            link: link.clone(),
            source_handle: run_handles.get(&link.file_id).cloned(),
        });
    }

    let mut entities = Vec::new();
    for (subject, mut file_links) in by_entity {
        file_links.sort_by(|a, b| a.link.link_handle.cmp(&b.link.link_handle));
        let entity_name = if query.include_entity_names {
            resolve_entity_name(conn, &subject.entity_type, &subject.entity_id)?
        } else {
            None
        };
        let included_file_ids = file_links
            .iter()
            .map(|row| row.link.file_id.clone())
            .collect::<BTreeSet<_>>();
        let claim_summary =
            claim_summary_for_entity(&subject, &included_file_ids, claims, query.privacy_profile);
        let file_links = file_links.into_iter().map(file_link_from_row).collect();
        entities.push(WorkspaceGraphEntity {
            entity_type: subject.entity_type,
            entity_id: subject.entity_id,
            entity_name,
            file_links,
            claim_summary,
        });
    }
    Ok(entities)
}

fn entity_matches_filter(query: &NormalizedQuery, entity_type: &str, entity_id: &str) -> bool {
    if !is_supported_entity_type(entity_type) {
        return false;
    }
    if !query.entity_types.is_empty() && !query.entity_types.contains(entity_type) {
        return false;
    }
    query.entity_ids.is_empty() || query.entity_ids.contains(entity_id)
}

fn resolve_entity_name(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
) -> Result<Option<String>, WorkspaceGraphReadError> {
    let sql = match entity_type {
        "account" => "SELECT name FROM accounts WHERE id = ?1 AND archived = 0",
        "person" => {
            "SELECT CASE WHEN trim(name) = '' THEN email ELSE name END FROM people WHERE id = ?1 AND archived = 0"
        }
        "project" => "SELECT name FROM projects WHERE id = ?1 AND archived = 0",
        _ => return Ok(None),
    };
    conn.query_row(sql, params![entity_id], |row| row.get::<_, String>(0))
        .optional()
        .map_err(read_failed)
}

fn file_link_from_row(row: FileLinkRow) -> WorkspaceGraphFileLink {
    WorkspaceGraphFileLink {
        link_handle: row.link.link_handle,
        source_handle: row.source_handle,
        data_source_kind: "workspace_file".to_string(),
        workspace_file_kind: row.lifecycle.source_type,
        lifecycle_state: row.lifecycle.lifecycle_state,
        category: row.lifecycle.category,
        source_asof: row.lifecycle.source_asof,
        attribution_source: row.link.attribution_source,
        confidence: row.link.confidence,
        user_override: row
            .link
            .user_override_at
            .map(|at| WorkspaceGraphUserOverride { present: true, at }),
    }
}

fn claim_summary_for_entity(
    subject: &EntitySubject,
    included_file_ids: &BTreeSet<String>,
    claims: &[WorkspaceClaim],
    privacy_profile: WorkspaceGraphPrivacyProfile,
) -> WorkspaceGraphClaimSummary {
    let mut summary = WorkspaceGraphClaimSummary::default();
    for claim in claims {
        if !claim.subjects.contains(subject)
            || claim.source_file_ids.is_disjoint(included_file_ids)
            || !sensitivity_allowed(claim, privacy_profile)
        {
            continue;
        }
        summary.total += 1;
        match claim_trust_band_from_score(claim.trust_score) {
            TrustBand::LikelyCurrent => summary.by_trust_band.likely_current += 1,
            TrustBand::UseWithCaution => summary.by_trust_band.use_with_caution += 1,
            TrustBand::NeedsVerification => summary.by_trust_band.needs_verification += 1,
            TrustBand::Unscored => summary.by_trust_band.unscored += 1,
        }
    }
    summary
}

fn sensitivity_allowed(
    claim: &WorkspaceClaim,
    privacy_profile: WorkspaceGraphPrivacyProfile,
) -> bool {
    match privacy_profile {
        WorkspaceGraphPrivacyProfile::FirstParty => true,
        WorkspaceGraphPrivacyProfile::SurfaceClient => {
            matches!(claim.sensitivity.as_str(), "public" | "internal")
        }
    }
}

fn build_audit_gaps(
    query: &NormalizedQuery,
    lifecycle_rows: &BTreeMap<String, LifecycleRow>,
    active_links: &[LinkRow],
    run_handles: &BTreeMap<String, String>,
    claims: &[WorkspaceClaim],
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Vec<WorkspaceGraphAuditGap> {
    let links_by_file =
        active_links
            .iter()
            .fold(BTreeMap::<String, Vec<&LinkRow>>::new(), |mut map, link| {
                map.entry(link.file_id.clone()).or_default().push(link);
                map
            });
    let mut gaps = Vec::new();

    for claim in claims {
        if !sensitivity_allowed(claim, query.privacy_profile) {
            continue;
        }
        for file_id in &claim.source_file_ids {
            let workspace_source_handle = workspace_source_handle(file_id, diagnostic_key);
            let source_handle = run_handles.get(file_id).cloned();
            match lifecycle_rows.get(file_id) {
                None => {
                    if !query.category_filter.is_empty()
                        || !claim_matches_entity_filter(query, claim)
                    {
                        continue;
                    }
                    gaps.push(claim_gap(ClaimGapInput {
                        category: "workspace_claim_without_lifecycle",
                        reason: "missing_lifecycle",
                        workspace_source_handle: &workspace_source_handle,
                        source_handle,
                        link_handle: None,
                        claim,
                        lifecycle: None,
                        linked_subject: None,
                        diagnostic_key,
                    }));
                }
                Some(lifecycle) => {
                    let links = links_by_file.get(file_id).cloned().unwrap_or_default();
                    if !category_matches_filter(query, lifecycle)
                        || !claim_lifecycle_or_links_match_entity_filter(
                            query, claim, lifecycle, &links,
                        )
                    {
                        continue;
                    }
                    if links.is_empty() {
                        gaps.push(claim_gap(ClaimGapInput {
                            category: "workspace_claim_without_link",
                            reason: "missing_active_link",
                            workspace_source_handle: &workspace_source_handle,
                            source_handle: source_handle.clone(),
                            link_handle: None,
                            claim,
                            lifecycle: Some(lifecycle),
                            linked_subject: None,
                            diagnostic_key,
                        }));
                    }
                    if matches!(
                        lifecycle.lifecycle_state.as_str(),
                        "rejected" | "quarantined" | "superseded"
                    ) {
                        gaps.push(claim_gap(ClaimGapInput {
                            category: "excluded_file_has_active_workspace_claim",
                            reason: "excluded_lifecycle_has_active_claim",
                            workspace_source_handle: &workspace_source_handle,
                            source_handle: source_handle.clone(),
                            link_handle: None,
                            claim,
                            lifecycle: Some(lifecycle),
                            linked_subject: None,
                            diagnostic_key,
                        }));
                    }
                    for link in links {
                        let linked = EntitySubject {
                            entity_type: link.entity_type.clone(),
                            entity_id: link.entity_id.clone(),
                        };
                        if !claim.subjects.is_empty() && !claim.subjects.contains(&linked) {
                            gaps.push(claim_gap(ClaimGapInput {
                                category: "workspace_claim_subject_mismatch",
                                reason: "claim_subject_does_not_match_linked_entity",
                                workspace_source_handle: &workspace_source_handle,
                                source_handle: source_handle.clone(),
                                link_handle: Some(link.link_handle.clone()),
                                claim,
                                lifecycle: Some(lifecycle),
                                linked_subject: Some(&linked),
                                diagnostic_key,
                            }));
                        }
                    }
                }
            }
        }
    }

    for link in active_links {
        if lifecycle_rows.contains_key(&link.file_id) {
            continue;
        }
        if !query.category_filter.is_empty() || !link_matches_entity_filter(query, link) {
            continue;
        }
        let linked = EntitySubject {
            entity_type: link.entity_type.clone(),
            entity_id: link.entity_id.clone(),
        };
        let workspace_source_handle = workspace_source_handle(&link.file_id, diagnostic_key);
        gaps.push(WorkspaceGraphAuditGap {
            category: "orphan_link".to_string(),
            gap_id: gap_id(
                "orphan_link",
                &workspace_source_handle,
                None,
                Some(&link.link_handle),
                None,
                "missing_lifecycle",
                diagnostic_key,
            ),
            workspace_source_handle,
            source_handle: run_handles.get(&link.file_id).cloned(),
            link_handle: Some(link.link_handle.clone()),
            claim_handle: None,
            entity_type: Some(linked.entity_type),
            entity_id: Some(linked.entity_id),
            linked_entity_type: None,
            linked_entity_id: None,
            claim_entity_type: None,
            claim_entity_id: None,
            workspace_file_kind: None,
            lifecycle_state: None,
            reason: "missing_lifecycle".to_string(),
        });
    }

    for (file_id, lifecycle) in lifecycle_rows {
        if lifecycle.lifecycle_state != "ingested" || links_by_file.contains_key(file_id) {
            continue;
        }
        if !lifecycle_matches_audit_filter(query, lifecycle) {
            continue;
        }
        let workspace_source_handle = workspace_source_handle(file_id, diagnostic_key);
        gaps.push(WorkspaceGraphAuditGap {
            category: "file_without_active_link".to_string(),
            gap_id: gap_id(
                "file_without_active_link",
                &workspace_source_handle,
                run_handles.get(file_id).map(String::as_str),
                None,
                None,
                "missing_active_link",
                diagnostic_key,
            ),
            workspace_source_handle,
            source_handle: run_handles.get(file_id).cloned(),
            link_handle: None,
            claim_handle: None,
            entity_type: lifecycle
                .entity_subject()
                .map(|subject| subject.entity_type),
            entity_id: lifecycle.entity_subject().map(|subject| subject.entity_id),
            linked_entity_type: None,
            linked_entity_id: None,
            claim_entity_type: None,
            claim_entity_id: None,
            workspace_file_kind: Some(lifecycle.source_type.clone()),
            lifecycle_state: Some(lifecycle.lifecycle_state.clone()),
            reason: "missing_active_link".to_string(),
        });
    }

    gaps
}

fn claim_matches_entity_filter(query: &NormalizedQuery, claim: &WorkspaceClaim) -> bool {
    entity_filter_is_empty(query)
        || claim
            .subjects
            .iter()
            .any(|subject| subject_matches_entity_filter(query, subject))
}

fn claim_lifecycle_or_links_match_entity_filter(
    query: &NormalizedQuery,
    claim: &WorkspaceClaim,
    lifecycle: &LifecycleRow,
    links: &[&LinkRow],
) -> bool {
    entity_filter_is_empty(query)
        || claim_matches_entity_filter(query, claim)
        || lifecycle
            .entity_subject()
            .as_ref()
            .is_some_and(|subject| subject_matches_entity_filter(query, subject))
        || links
            .iter()
            .any(|link| link_matches_entity_filter(query, link))
}

fn lifecycle_matches_audit_filter(query: &NormalizedQuery, lifecycle: &LifecycleRow) -> bool {
    category_matches_filter(query, lifecycle)
        && (entity_filter_is_empty(query)
            || lifecycle
                .entity_subject()
                .as_ref()
                .is_some_and(|subject| subject_matches_entity_filter(query, subject)))
}

fn category_matches_filter(query: &NormalizedQuery, lifecycle: &LifecycleRow) -> bool {
    query.category_filter.is_empty()
        || lifecycle
            .category
            .as_deref()
            .is_some_and(|category| query.category_filter.contains(category))
}

fn link_matches_entity_filter(query: &NormalizedQuery, link: &LinkRow) -> bool {
    entity_matches_filter(query, &link.entity_type, &link.entity_id)
}

fn subject_matches_entity_filter(query: &NormalizedQuery, subject: &EntitySubject) -> bool {
    entity_matches_filter(query, &subject.entity_type, &subject.entity_id)
}

fn entity_filter_is_empty(query: &NormalizedQuery) -> bool {
    query.entity_types.is_empty() && query.entity_ids.is_empty()
}

struct ClaimGapInput<'a> {
    category: &'a str,
    reason: &'a str,
    workspace_source_handle: &'a str,
    source_handle: Option<String>,
    link_handle: Option<String>,
    claim: &'a WorkspaceClaim,
    lifecycle: Option<&'a LifecycleRow>,
    linked_subject: Option<&'a EntitySubject>,
    diagnostic_key: &'a WorkspaceGraphDiagnosticKey,
}

fn claim_gap(input: ClaimGapInput<'_>) -> WorkspaceGraphAuditGap {
    let ClaimGapInput {
        category,
        reason,
        workspace_source_handle,
        source_handle,
        link_handle,
        claim,
        lifecycle,
        linked_subject,
        diagnostic_key,
    } = input;
    let claim_subject = claim.primary_subject();
    WorkspaceGraphAuditGap {
        category: category.to_string(),
        gap_id: gap_id(
            category,
            workspace_source_handle,
            source_handle.as_deref(),
            link_handle.as_deref(),
            Some(&claim.claim_handle),
            reason,
            diagnostic_key,
        ),
        workspace_source_handle: workspace_source_handle.to_string(),
        source_handle,
        link_handle,
        claim_handle: Some(claim.claim_handle.clone()),
        entity_type: claim_subject
            .as_ref()
            .map(|subject| subject.entity_type.clone()),
        entity_id: claim_subject
            .as_ref()
            .map(|subject| subject.entity_id.clone()),
        linked_entity_type: linked_subject.map(|subject| subject.entity_type.clone()),
        linked_entity_id: linked_subject.map(|subject| subject.entity_id.clone()),
        claim_entity_type: claim_subject
            .as_ref()
            .map(|subject| subject.entity_type.clone()),
        claim_entity_id: claim_subject
            .as_ref()
            .map(|subject| subject.entity_id.clone()),
        workspace_file_kind: lifecycle.map(|row| row.source_type.clone()),
        lifecycle_state: lifecycle.map(|row| row.lifecycle_state.clone()),
        reason: reason.to_string(),
    }
}

fn audit_report(graph_version: String, gaps: Vec<WorkspaceGraphAuditGap>) -> WorkspaceGraphAudit {
    let mut gap_counts = BTreeMap::<String, u32>::new();
    for gap in &gaps {
        *gap_counts.entry(gap.category.clone()).or_default() += 1;
    }
    WorkspaceGraphAudit {
        schema_version: SCHEMA_VERSION,
        graph_version,
        gap_counts,
        gaps,
    }
}

fn gap_order(a: &WorkspaceGraphAuditGap, b: &WorkspaceGraphAuditGap) -> std::cmp::Ordering {
    (
        &a.category,
        &a.entity_type,
        &a.entity_id,
        &a.linked_entity_type,
        &a.linked_entity_id,
        &a.claim_entity_type,
        &a.claim_entity_id,
        &a.workspace_source_handle,
        &a.source_handle,
        &a.link_handle,
        &a.claim_handle,
        &a.reason,
    )
        .cmp(&(
            &b.category,
            &b.entity_type,
            &b.entity_id,
            &b.linked_entity_type,
            &b.linked_entity_id,
            &b.claim_entity_type,
            &b.claim_entity_id,
            &b.workspace_source_handle,
            &b.source_handle,
            &b.link_handle,
            &b.claim_handle,
            &b.reason,
        ))
}

fn gap_id(
    category: &str,
    workspace_source_handle: &str,
    source_handle: Option<&str>,
    link_handle: Option<&str>,
    claim_handle: Option<&str>,
    reason: &str,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> String {
    let value = json!({
        "category": category,
        "workspace_source_handle": workspace_source_handle,
        "source_handle": source_handle,
        "link_handle": link_handle,
        "claim_handle": claim_handle,
        "reason": reason,
    });
    format!(
        "gap:v1:{}",
        diagnostic_digest("gap", &value.to_string(), diagnostic_key)
    )
}

fn workspace_source_handle(file_id: &str, diagnostic_key: &WorkspaceGraphDiagnosticKey) -> String {
    diagnostic_handle("source", "workspace_source", file_id, diagnostic_key)
}

fn diagnostic_handle(
    prefix: &str,
    domain: &str,
    value: &str,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> String {
    format!(
        "{prefix}:v1:{}",
        diagnostic_digest(domain, value, diagnostic_key)
    )
}

fn diagnostic_digest(
    domain: &str,
    value: &str,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> String {
    let mut mac =
        HmacSha256::new_from_slice(&diagnostic_key.bytes).expect("HMAC key length is valid");
    mac.update(domain.as_bytes());
    mac.update(&[0]);
    mac.update(value.as_bytes());
    let bytes = mac.finalize().into_bytes();
    hex::encode(&bytes[..16])
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
        ClaimSubjectRef::Meeting { .. }
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
        SubjectRef::Multi(subjects) => subjects
            .iter()
            .flat_map(subjects_from_ref)
            .collect::<BTreeSet<_>>(),
        _ => BTreeSet::new(),
    }
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, WorkspaceGraphReadError> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        params![table],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value == 1)
    .map_err(read_failed)
}

fn read_failed(error: rusqlite::Error) -> WorkspaceGraphReadError {
    WorkspaceGraphReadError::ReadFailed(error.to_string())
}

impl LifecycleRow {
    fn entity_subject(&self) -> Option<EntitySubject> {
        Some(EntitySubject {
            entity_type: self.entity_type()?,
            entity_id: self.entity_id()?,
        })
    }

    fn entity_type(&self) -> Option<String> {
        self.entity_type.clone()
    }

    fn entity_id(&self) -> Option<String> {
        self.entity_id.clone()
    }
}

impl WorkspaceClaim {
    fn primary_subject(&self) -> Option<EntitySubject> {
        self.subjects.iter().next().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use abilities_runtime::abilities::workspace_graph::contracts::WorkspaceGraphEntityFilter;
    use rusqlite::Connection;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("memory db");
        conn.execute_batch(
            r#"
            CREATE TABLE workspace_file_lifecycle (
                file_id TEXT PRIMARY KEY,
                canonical_path TEXT NOT NULL,
                device INTEGER NOT NULL,
                inode INTEGER NOT NULL,
                source_type TEXT NOT NULL,
                data_source TEXT NOT NULL,
                lifecycle_state TEXT NOT NULL,
                source_asof TEXT NOT NULL,
                entity_id TEXT,
                entity_type TEXT,
                content_sha256 TEXT,
                category TEXT,
                user_override_actor TEXT,
                user_override_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
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
            CREATE TABLE document_entity_links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                link_id TEXT NOT NULL UNIQUE,
                file_id TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                attribution_source TEXT NOT NULL,
                confidence REAL NOT NULL,
                rationale TEXT,
                actor TEXT NOT NULL,
                user_override_actor TEXT,
                user_override_at TEXT,
                rejected INTEGER NOT NULL DEFAULT 0,
                rejected_at TEXT,
                rejected_reason TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE intelligence_claims (
                id TEXT PRIMARY KEY,
                subject_ref TEXT NOT NULL,
                claim_type TEXT NOT NULL,
                field_path TEXT,
                topic_key TEXT,
                text TEXT NOT NULL,
                dedup_key TEXT NOT NULL,
                item_hash TEXT,
                actor TEXT NOT NULL,
                data_source TEXT NOT NULL,
                source_ref TEXT,
                source_asof TEXT,
                observed_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                provenance_json TEXT NOT NULL,
                metadata_json TEXT,
                claim_state TEXT NOT NULL,
                surfacing_state TEXT NOT NULL,
                demotion_reason TEXT,
                reactivated_at TEXT,
                retraction_reason TEXT,
                expires_at TEXT,
                superseded_by TEXT,
                trust_score REAL,
                trust_computed_at TEXT,
                trust_version INTEGER,
                thread_id TEXT,
                temporal_scope TEXT NOT NULL,
                sensitivity TEXT NOT NULL
            );
            CREATE TABLE claim_semantic_evidence (
                id TEXT PRIMARY KEY,
                canonical_claim_id TEXT NOT NULL,
                corroboration_id TEXT,
                data_source TEXT NOT NULL,
                source_ref TEXT,
                source_asof TEXT,
                provenance_json TEXT NOT NULL,
                original_text TEXT NOT NULL,
                actor TEXT NOT NULL,
                observed_at TEXT NOT NULL,
                thread_id TEXT,
                source_mechanism TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE accounts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                archived INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE people (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL,
                name TEXT NOT NULL,
                archived INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                archived INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )
        .expect("schema");
        conn
    }

    fn default_request() -> WorkspaceGraphReadRequest {
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
        }
    }

    fn test_diagnostic_key() -> WorkspaceGraphDiagnosticKey {
        WorkspaceGraphDiagnosticKey::for_tests("workspace-graph-test-install")
    }

    fn read_test_workspace_graph(
        conn: &Connection,
        request: WorkspaceGraphReadRequest,
    ) -> Result<WorkspaceGraphResponse, WorkspaceGraphReadError> {
        read_workspace_graph(conn, request, &test_diagnostic_key())
    }

    fn insert_lifecycle(conn: &Connection, file_id: &str, state: &str, entity_id: &str) {
        insert_lifecycle_with_category(conn, file_id, state, entity_id, "notes");
    }

    fn insert_lifecycle_with_category(
        conn: &Connection,
        file_id: &str,
        state: &str,
        entity_id: &str,
        category: &str,
    ) {
        conn.execute(
            "INSERT INTO workspace_file_lifecycle (
                file_id, canonical_path, device, inode, source_type, data_source,
                lifecycle_state, source_asof, entity_id, entity_type, content_sha256,
                category, created_at, updated_at
             ) VALUES (?1, ?2, 1, 1, 'entity_doc', '{}', ?3,
                '2026-05-24T00:00:00Z', ?4, 'account', 'hash', ?5,
                '2026-05-24T00:00:00Z', '2026-05-24T00:00:00Z')",
            params![
                file_id,
                format!("/workspace/{file_id}.md"),
                state,
                entity_id,
                category
            ],
        )
        .expect("insert lifecycle");
    }

    fn insert_link(conn: &Connection, file_id: &str, link_id: &str, entity_id: &str) {
        insert_link_with_type(conn, file_id, link_id, "account", entity_id);
    }

    fn insert_link_with_type(
        conn: &Connection,
        file_id: &str,
        link_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) {
        conn.execute(
            "INSERT INTO document_entity_links (
                link_id, file_id, entity_type, entity_id, attribution_source,
                confidence, actor, rejected, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, 'automatic', 0.8, 'system',
                0, '2026-05-24T00:00:00Z', '2026-05-24T00:00:00Z')",
            params![link_id, file_id, entity_type, entity_id],
        )
        .expect("insert link");
    }

    fn insert_run(conn: &Connection, file_id: &str, run_id: &str) {
        conn.execute(
            "INSERT INTO document_ingestion_runs (
                run_id, file_id, mode, started_at, status, content_sha256,
                file_size_bytes, extractor_version
             ) VALUES (?1, ?2, 'initial', '2026-05-24T00:00:00Z',
                'success', 'hash', 10, 'test')",
            params![run_id, file_id],
        )
        .expect("insert run");
    }

    fn insert_claim(
        conn: &Connection,
        claim_id: &str,
        file_id: &str,
        entity_id: &str,
        sensitivity: &str,
        trust_score: Option<f64>,
    ) {
        let subject = serde_json::to_string(&SubjectRef::Account(entity_id.to_string()))
            .expect("subject json");
        insert_claim_with_subject(conn, claim_id, file_id, &subject, sensitivity, trust_score);
    }

    fn insert_claim_with_subject(
        conn: &Connection,
        claim_id: &str,
        file_id: &str,
        subject_ref: &str,
        sensitivity: &str,
        trust_score: Option<f64>,
    ) {
        let source_ref = format!("{WORKSPACE_SOURCE_PREFIX}{file_id}");
        insert_claim_with_subject_and_source_ref(
            conn,
            claim_id,
            subject_ref,
            &source_ref,
            sensitivity,
            trust_score,
        );
    }

    fn insert_claim_with_subject_and_source_ref(
        conn: &Connection,
        claim_id: &str,
        subject_ref: &str,
        source_ref: &str,
        sensitivity: &str,
        trust_score: Option<f64>,
    ) {
        conn.execute(
            "INSERT INTO intelligence_claims /* dos7-allowed: read-model fixture seeds legacy workspace claim shapes */ (
                id, subject_ref, claim_type, text, dedup_key, actor, data_source,
                source_ref, source_asof, observed_at, created_at, provenance_json,
                claim_state, surfacing_state, trust_score, temporal_scope, sensitivity
             ) VALUES (?1, ?2, 'account_status', 'redacted in output', ?1,
                'system', '{}', ?3, '2026-05-24T00:00:00Z',
                '2026-05-24T00:00:00Z', '2026-05-24T00:00:00Z', '{}',
                'active', 'active', ?4, 'state', ?5)",
            params![
                claim_id,
                subject_ref,
                source_ref,
                trust_score,
                sensitivity
            ],
        )
        .expect("insert claim");
    }

    #[test]
    fn projection_uses_opaque_handles_and_redacts_path_derived_file_id() {
        let conn = setup_conn();
        conn.execute(
            "INSERT INTO accounts (id, name) VALUES ('account-1', 'Example Account')",
            [],
        )
        .expect("account");
        insert_lifecycle(&conn, "pathhash-a", "ingested", "account-1");
        insert_run(&conn, "pathhash-a", "11111111-1111-4111-8111-111111111111");
        insert_link(
            &conn,
            "pathhash-a",
            "22222222-2222-4222-8222-222222222222",
            "account-1",
        );
        conn.execute(
            "UPDATE workspace_file_lifecycle
             SET user_override_at = '2026-05-24T02:00:00Z'
             WHERE file_id = 'pathhash-a'",
            [],
        )
        .expect("lifecycle override");
        conn.execute(
            "UPDATE document_entity_links
             SET user_override_at = '2026-05-24T01:00:00Z'
             WHERE link_id = '22222222-2222-4222-8222-222222222222'",
            [],
        )
        .expect("link override");
        insert_claim(
            &conn,
            "claim-1",
            "pathhash-a",
            "account-1",
            "internal",
            Some(0.95),
        );
        insert_claim(
            &conn,
            "claim-2",
            "pathhash-a",
            "account-1",
            "user_only",
            Some(0.95),
        );

        let mut request = default_request();
        request.input.include_entity_names = true;
        let response = read_test_workspace_graph(&conn, request).expect("graph response");
        let WorkspaceGraphResponse::Projection(projection) = response else {
            panic!("expected projection");
        };

        assert_eq!(projection.projection.entities.len(), 1);
        let entity = &projection.projection.entities[0];
        assert_eq!(entity.entity_name.as_deref(), Some("Example Account"));
        assert_eq!(entity.claim_summary.total, 2);
        assert_eq!(
            entity.file_links[0].link_handle,
            "22222222-2222-4222-8222-222222222222"
        );
        assert_eq!(
            entity.file_links[0].source_handle.as_deref(),
            Some("11111111-1111-4111-8111-111111111111")
        );
        assert_eq!(
            entity.file_links[0]
                .user_override
                .as_ref()
                .map(|override_info| override_info.at.as_str()),
            Some("2026-05-24T01:00:00Z")
        );

        let serialized = serde_json::to_string(&projection).expect("serialize");
        assert!(!serialized.contains("pathhash-a"));
        assert!(!serialized.contains("/workspace/"));
        assert!(!serialized.contains("redacted in output"));
        let value = serde_json::to_value(&projection).expect("projection json");
        assert_eq!(value["page"]["hasMore"], serde_json::Value::Bool(false));
        assert!(value["projection"]["entities"].is_array());
        assert!(value.get("entities").is_none());
        assert!(value["projection"]["entities"][0]["claimSummary"]["byTrustBand"].is_object());

        let mut surface_request = default_request();
        surface_request.privacy_profile = WorkspaceGraphPrivacyProfile::SurfaceClient;
        let response = read_test_workspace_graph(&conn, surface_request).expect("surface graph");
        let WorkspaceGraphResponse::Projection(surface_projection) = response else {
            panic!("expected projection");
        };
        assert_eq!(
            surface_projection.projection.entities[0]
                .claim_summary
                .total,
            1
        );
    }

    #[test]
    fn diagnostic_handles_are_keyed_per_install() {
        let conn = setup_conn();
        insert_lifecycle(&conn, "pathhash-a", "ingested", "account-1");

        let first_key = WorkspaceGraphDiagnosticKey::for_tests("install-a");
        let second_key = WorkspaceGraphDiagnosticKey::for_tests("install-b");
        let first_response =
            read_workspace_graph(&conn, default_request(), &first_key).expect("first graph");
        let second_response =
            read_workspace_graph(&conn, default_request(), &second_key).expect("second graph");
        let WorkspaceGraphResponse::Projection(first_projection) = first_response else {
            panic!("expected first projection");
        };
        let WorkspaceGraphResponse::Projection(second_projection) = second_response else {
            panic!("expected second projection");
        };

        assert_ne!(
            first_projection.audit.gaps[0].workspace_source_handle,
            second_projection.audit.gaps[0].workspace_source_handle
        );
        assert_ne!(
            first_projection.graph_version,
            second_projection.graph_version
        );
    }

    #[test]
    fn entity_names_omit_archived_rows() {
        let conn = setup_conn();
        conn.execute(
            "INSERT INTO accounts (id, name, archived)
             VALUES ('account-archived', 'Archived Account', 1)",
            [],
        )
        .expect("archived account");
        insert_lifecycle(&conn, "pathhash-a", "ingested", "account-archived");
        insert_link(
            &conn,
            "pathhash-a",
            "11111111-1111-4111-8111-111111111111",
            "account-archived",
        );

        let mut request = default_request();
        request.input.include_entity_names = true;
        let response = read_test_workspace_graph(&conn, request).expect("graph response");
        let WorkspaceGraphResponse::Projection(projection) = response else {
            panic!("expected projection");
        };

        assert_eq!(projection.projection.entities.len(), 1);
        assert_eq!(projection.projection.entities[0].entity_name, None);
    }

    #[test]
    fn claim_summary_counts_only_returned_file_links() {
        let conn = setup_conn();
        insert_lifecycle_with_category(&conn, "pathhash-a", "ingested", "account-1", "notes");
        insert_link(
            &conn,
            "pathhash-a",
            "11111111-1111-4111-8111-111111111111",
            "account-1",
        );
        insert_claim(
            &conn,
            "claim-1",
            "pathhash-a",
            "account-1",
            "internal",
            Some(0.95),
        );
        insert_lifecycle_with_category(&conn, "pathhash-b", "ingested", "account-1", "contracts");
        insert_link(
            &conn,
            "pathhash-b",
            "22222222-2222-4222-8222-222222222222",
            "account-1",
        );
        insert_claim(
            &conn,
            "claim-2",
            "pathhash-b",
            "account-1",
            "internal",
            Some(0.95),
        );

        let mut request = default_request();
        request.input.category_filter = Some(vec!["notes".to_string()]);
        let response = read_test_workspace_graph(&conn, request).expect("graph response");
        let WorkspaceGraphResponse::Projection(projection) = response else {
            panic!("expected projection");
        };

        assert_eq!(projection.projection.entities[0].file_links.len(), 1);
        assert_eq!(
            projection.projection.entities[0].file_links[0]
                .category
                .as_deref(),
            Some("notes")
        );
        assert_eq!(projection.projection.entities[0].claim_summary.total, 1);
    }

    #[test]
    fn audit_respects_entity_filter() {
        let conn = setup_conn();
        insert_lifecycle(&conn, "pathhash-a", "ingested", "account-1");
        insert_lifecycle(&conn, "pathhash-b", "ingested", "account-2");

        let mut request = default_request();
        request.input.entity_filter = Some(WorkspaceGraphEntityFilter {
            entity_types: Some(vec!["account".to_string()]),
            entity_ids: Some(vec!["account-1".to_string()]),
        });
        let response = read_test_workspace_graph(&conn, request).expect("graph response");
        let WorkspaceGraphResponse::Projection(projection) = response else {
            panic!("expected projection");
        };
        let gaps = projection
            .audit
            .gaps
            .iter()
            .filter(|gap| gap.category == "file_without_active_link")
            .collect::<Vec<_>>();

        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].entity_id.as_deref(), Some("account-1"));
    }

    #[test]
    fn plural_entity_type_filter_supports_multiple_canonical_types() {
        let conn = setup_conn();
        insert_lifecycle(&conn, "pathhash-a", "ingested", "account-1");
        insert_link_with_type(
            &conn,
            "pathhash-a",
            "11111111-1111-4111-8111-111111111111",
            "account",
            "account-1",
        );
        insert_lifecycle(&conn, "pathhash-b", "ingested", "person-1");
        insert_link_with_type(
            &conn,
            "pathhash-b",
            "22222222-2222-4222-8222-222222222222",
            "person",
            "person-1",
        );
        insert_lifecycle(&conn, "pathhash-c", "ingested", "project-1");
        insert_link_with_type(
            &conn,
            "pathhash-c",
            "33333333-3333-4333-8333-333333333333",
            "project",
            "project-1",
        );

        let mut request = default_request();
        request.input.entity_filter = Some(WorkspaceGraphEntityFilter {
            entity_types: Some(vec!["account".to_string(), "person".to_string()]),
            entity_ids: None,
        });
        let response = read_test_workspace_graph(&conn, request).expect("graph response");
        let WorkspaceGraphResponse::Projection(projection) = response else {
            panic!("expected projection");
        };
        let entity_types = projection
            .projection
            .entities
            .iter()
            .map(|entity| entity.entity_type.as_str())
            .collect::<Vec<_>>();

        assert_eq!(entity_types, vec!["account", "person"]);
    }

    #[test]
    fn projection_excludes_unsupported_link_entity_types() {
        let conn = setup_conn();
        insert_lifecycle(&conn, "pathhash-a", "ingested", "other-1");
        insert_link_with_type(
            &conn,
            "pathhash-a",
            "11111111-1111-4111-8111-111111111111",
            "other",
            "other-1",
        );

        let response = read_test_workspace_graph(&conn, default_request()).expect("graph response");
        let WorkspaceGraphResponse::Projection(projection) = response else {
            panic!("expected projection");
        };

        assert!(projection.projection.entities.is_empty());
    }

    #[test]
    fn surface_client_audit_redacts_sensitive_claim_gaps() {
        let conn = setup_conn();
        insert_claim(
            &conn,
            "claim-1",
            "pathhash-missing-internal",
            "account-1",
            "internal",
            Some(0.95),
        );
        insert_claim(
            &conn,
            "claim-2",
            "pathhash-missing-user-only",
            "account-1",
            "user_only",
            Some(0.95),
        );

        let mut request = default_request();
        request.privacy_profile = WorkspaceGraphPrivacyProfile::SurfaceClient;
        let response = read_test_workspace_graph(&conn, request).expect("graph response");
        let WorkspaceGraphResponse::Projection(projection) = response else {
            panic!("expected projection");
        };
        let gaps = projection
            .audit
            .gaps
            .iter()
            .filter(|gap| gap.category == "workspace_claim_without_lifecycle")
            .collect::<Vec<_>>();

        assert_eq!(gaps.len(), 1);
        let serialized = serde_json::to_string(&projection.audit).expect("serialize");
        assert!(!serialized.contains("pathhash-missing-user-only"));
    }

    #[test]
    fn db_shaped_pascal_case_subject_matches_semantic_evidence_source() {
        let conn = setup_conn();
        insert_lifecycle(&conn, "pathhash-a", "ingested", "account-1");
        insert_link(
            &conn,
            "pathhash-a",
            "11111111-1111-4111-8111-111111111111",
            "account-1",
        );
        let subject = serde_json::json!({
            "kind": "Account",
            "id": "account-1"
        })
        .to_string();
        insert_claim_with_subject_and_source_ref(
            &conn,
            "claim-1",
            &subject,
            "manual:source",
            "internal",
            Some(0.95),
        );
        conn.execute(
            "INSERT INTO claim_semantic_evidence (
                id, canonical_claim_id, data_source, source_ref, source_asof,
                provenance_json, original_text, actor, observed_at, source_mechanism, created_at
             ) VALUES (
                'evidence-1', 'claim-1', '{}', 'workspace_file:pathhash-a',
                '2026-05-24T00:00:00Z', '{}', 'redacted in output', 'system',
                '2026-05-24T00:00:00Z', 'workspace_ingestion', '2026-05-24T00:00:00Z'
             )",
            [],
        )
        .expect("semantic evidence");

        let response = read_test_workspace_graph(&conn, default_request()).expect("graph response");
        let WorkspaceGraphResponse::Projection(projection) = response else {
            panic!("expected projection");
        };

        assert_eq!(projection.projection.entities[0].claim_summary.total, 1);
        assert!(projection
            .audit
            .gaps
            .iter()
            .all(|gap| gap.category != "workspace_claim_subject_mismatch"));
    }

    #[test]
    fn db_shaped_multi_subject_claim_matches_linked_subject_without_mismatch_gap() {
        let conn = setup_conn();
        insert_lifecycle(&conn, "pathhash-a", "ingested", "account-1");
        insert_link(
            &conn,
            "pathhash-a",
            "11111111-1111-4111-8111-111111111111",
            "account-1",
        );
        let subject = serde_json::json!({
            "kind": "multi",
            "subjects": [
                { "kind": "project", "id": "project-1" },
                { "kind": "Account", "id": "account-1" }
            ]
        })
        .to_string();
        insert_claim_with_subject(
            &conn,
            "claim-1",
            "pathhash-a",
            &subject,
            "internal",
            Some(0.95),
        );

        let response = read_test_workspace_graph(&conn, default_request()).expect("graph response");
        let WorkspaceGraphResponse::Projection(projection) = response else {
            panic!("expected projection");
        };

        assert_eq!(projection.projection.entities[0].claim_summary.total, 1);
        assert!(projection
            .audit
            .gaps
            .iter()
            .all(|gap| gap.category != "workspace_claim_subject_mismatch"));
    }

    #[test]
    fn multi_subject_claim_matches_linked_subject_without_mismatch_gap() {
        let conn = setup_conn();
        insert_lifecycle(&conn, "pathhash-a", "ingested", "account-1");
        insert_link(
            &conn,
            "pathhash-a",
            "11111111-1111-4111-8111-111111111111",
            "account-1",
        );
        let subject = serde_json::to_string(&SubjectRef::Multi(vec![
            SubjectRef::Project("project-1".to_string()),
            SubjectRef::Account("account-1".to_string()),
        ]))
        .expect("subject json");
        insert_claim_with_subject(
            &conn,
            "claim-1",
            "pathhash-a",
            &subject,
            "internal",
            Some(0.95),
        );

        let response = read_test_workspace_graph(&conn, default_request()).expect("graph response");
        let WorkspaceGraphResponse::Projection(projection) = response else {
            panic!("expected projection");
        };

        assert_eq!(projection.projection.entities[0].claim_summary.total, 1);
        assert!(projection
            .audit
            .gaps
            .iter()
            .all(|gap| gap.category != "workspace_claim_subject_mismatch"));
    }

    #[test]
    fn audit_disambiguates_handleless_lifecycle_gaps_without_raw_file_ids() {
        let conn = setup_conn();
        insert_lifecycle(&conn, "pathhash-a", "ingested", "account-1");
        insert_lifecycle(&conn, "pathhash-b", "ingested", "account-2");

        let response = read_test_workspace_graph(&conn, default_request()).expect("graph response");
        let WorkspaceGraphResponse::Projection(projection) = response else {
            panic!("expected projection");
        };
        let gaps = projection
            .audit
            .gaps
            .iter()
            .filter(|gap| gap.category == "file_without_active_link")
            .collect::<Vec<_>>();
        assert_eq!(gaps.len(), 2);
        assert_ne!(gaps[0].gap_id, gaps[1].gap_id);
        assert_ne!(
            gaps[0].workspace_source_handle,
            gaps[1].workspace_source_handle
        );

        let serialized = serde_json::to_string(&projection.audit).expect("serialize");
        assert!(!serialized.contains("pathhash-a"));
        assert!(!serialized.contains("pathhash-b"));
        assert!(!serialized.contains("/workspace/"));

        let value = serde_json::to_value(&projection.audit).expect("audit json");
        let gap = value["gaps"][0].as_object().expect("gap object");
        assert!(gap.get("sourceHandle").is_some_and(|value| value.is_null()));
        assert!(gap.get("linkHandle").is_some_and(|value| value.is_null()));
        assert!(gap.get("claimHandle").is_some_and(|value| value.is_null()));
        assert!(gap
            .get("linkedEntityType")
            .is_some_and(|value| value.is_null()));
        assert!(gap
            .get("claimEntityType")
            .is_some_and(|value| value.is_null()));
    }

    #[test]
    fn cursor_is_invalidated_by_mutation_outside_current_page() {
        let conn = setup_conn();
        insert_lifecycle(&conn, "pathhash-a", "ingested", "account-1");
        insert_link(
            &conn,
            "pathhash-a",
            "11111111-1111-4111-8111-111111111111",
            "account-1",
        );
        insert_lifecycle(&conn, "pathhash-b", "ingested", "account-2");
        insert_link(
            &conn,
            "pathhash-b",
            "22222222-2222-4222-8222-222222222222",
            "account-2",
        );

        let mut request = default_request();
        request.input.page_size = 1;
        let response = read_test_workspace_graph(&conn, request.clone()).expect("first page");
        let WorkspaceGraphResponse::Projection(first_page) = response else {
            panic!("expected projection");
        };
        let cursor = first_page.page.next_cursor.expect("next cursor");

        insert_lifecycle(&conn, "pathhash-c", "ingested", "account-3");
        insert_link(
            &conn,
            "pathhash-c",
            "33333333-3333-4333-8333-333333333333",
            "account-3",
        );

        request.input.cursor = Some(cursor);
        let error = read_test_workspace_graph(&conn, request).expect_err("cursor invalidated");
        assert!(
            matches!(error, WorkspaceGraphReadError::InvalidCursor(message) if message == "graph_version_changed_restart_required")
        );
    }

    #[test]
    fn if_none_match_is_first_page_only() {
        let conn = setup_conn();
        insert_lifecycle(&conn, "pathhash-a", "ingested", "account-1");
        insert_link(
            &conn,
            "pathhash-a",
            "11111111-1111-4111-8111-111111111111",
            "account-1",
        );
        insert_lifecycle(&conn, "pathhash-b", "ingested", "account-2");
        insert_link(
            &conn,
            "pathhash-b",
            "22222222-2222-4222-8222-222222222222",
            "account-2",
        );

        let response = read_test_workspace_graph(&conn, default_request()).expect("first page");
        let WorkspaceGraphResponse::Projection(projection) = response else {
            panic!("expected projection");
        };
        let graph_version = projection.graph_version;

        let mut not_modified_request = default_request();
        not_modified_request.input.if_none_match = Some(graph_version.clone());
        let response =
            read_test_workspace_graph(&conn, not_modified_request).expect("not modified response");
        assert!(matches!(response, WorkspaceGraphResponse::NotModified(_)));

        let mut cursor_request = default_request();
        cursor_request.input.page_size = 1;
        let response = read_test_workspace_graph(&conn, cursor_request.clone()).expect("page");
        let WorkspaceGraphResponse::Projection(page) = response else {
            panic!("expected projection");
        };
        cursor_request.input.cursor = page.page.next_cursor;
        cursor_request.input.if_none_match = Some(graph_version);
        let error = read_test_workspace_graph(&conn, cursor_request).expect_err("cursor plus etag");
        assert!(
            matches!(error, WorkspaceGraphReadError::InvalidCursor(message) if message == "if_none_match_not_allowed_with_cursor")
        );
    }
}

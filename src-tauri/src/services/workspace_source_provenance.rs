//! Privacy-safe provenance read service for workspace memory sources.
//!
//! The MCP handler uses this service instead of carrying SQL locally. The
//! response intentionally excludes file paths, raw file IDs, run IDs, claim
//! IDs, and claim text.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use abilities_runtime::abilities::provenance::trust::claim_trust_band_from_score;
use abilities_runtime::abilities::trust::types::TrustBand;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

use crate::db::{ActionDb, LocalKeychain};
use crate::services::workspace_ingestion::graph::{
    local_install_diagnostic_key, WorkspaceGraphDiagnosticKey,
};

const TOOL_NAME: &str = "dailyos.read.workspace_source_provenance";
const MAX_RUN_HISTORY: usize = 5;
const WORKSPACE_SOURCE_PREFIX: &str = "workspace_file:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceSourceProvenanceReadError {
    InvalidHandle(String),
    ReadFailed(String),
}

impl std::fmt::Display for WorkspaceSourceProvenanceReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHandle(message) => formatter.write_str(message),
            Self::ReadFailed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for WorkspaceSourceProvenanceReadError {}

pub fn normalize_workspace_source_entry_id(
    entry_id: &str,
) -> Result<String, WorkspaceSourceProvenanceReadError> {
    let value = entry_id.trim();
    if value.is_empty() {
        return Err(WorkspaceSourceProvenanceReadError::InvalidHandle(
            "entry_id must be a non-empty opaque workspace memory identifier".to_string(),
        ));
    }
    if value.len() > 200
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-'))
    {
        return Err(WorkspaceSourceProvenanceReadError::InvalidHandle(
            "entry_id must be an opaque handle, not a path or free-form value".to_string(),
        ));
    }
    Ok(value.to_string())
}

pub fn read_workspace_source_provenance_from_local_db(
    entry_id: &str,
) -> Result<Option<Value>, WorkspaceSourceProvenanceReadError> {
    let db = ActionDb::open_readonly(Arc::new(LocalKeychain::new()))
        .map_err(|error| WorkspaceSourceProvenanceReadError::ReadFailed(error.to_string()))?;
    let diagnostic_key =
        local_install_diagnostic_key().map_err(WorkspaceSourceProvenanceReadError::ReadFailed)?;
    read_workspace_source_provenance(db.conn_ref(), entry_id, &diagnostic_key)
}

pub fn read_workspace_source_provenance(
    conn: &Connection,
    entry_id: &str,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<Option<Value>, WorkspaceSourceProvenanceReadError> {
    let entry_id = normalize_workspace_source_entry_id(entry_id)?;
    let Some(file_id) = resolve_file_id(conn, &entry_id, diagnostic_key)? else {
        return Ok(None);
    };
    let Some(lifecycle) = load_lifecycle(conn, &file_id, diagnostic_key)? else {
        return Ok(None);
    };
    let entities = load_entities(conn, &file_id)?;
    let ingestion_runs = load_ingestion_runs(conn, &file_id)?;
    let claim_attribution = load_claim_attribution(conn, &file_id)?;

    Ok(Some(json!({
        "schemaVersion": 1,
        "toolName": TOOL_NAME,
        "status": "ok",
        "source": lifecycle,
        "entities": entities,
        "ingestionRuns": ingestion_runs,
        "claimAttribution": claim_attribution,
        "privacy": {
            "filePathIncluded": false,
            "rawFileIdIncluded": false,
            "rawRunIdsIncluded": false,
            "rawClaimIdsIncluded": false,
            "rawClaimTextIncluded": false
        }
    })))
}

fn resolve_file_id(
    conn: &Connection,
    entry_id: &str,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<Option<String>, WorkspaceSourceProvenanceReadError> {
    if entry_id.starts_with("source:v1:") {
        let mut stmt = conn
            .prepare("SELECT file_id FROM workspace_file_lifecycle")
            .map_err(read_failed)?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(read_failed)?;
        for row in rows {
            let file_id = row.map_err(read_failed)?;
            if diagnostic_key.workspace_source_handle(&file_id) == entry_id {
                return Ok(Some(file_id));
            }
        }
        return Ok(None);
    }

    if let Some(file_id) = conn
        .query_row(
            "SELECT file_id FROM document_entity_links WHERE link_id = ?1",
            params![entry_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(read_failed)?
    {
        return Ok(Some(file_id));
    }

    if table_exists(conn, "workspace_placement_idempotency")? {
        if let Some(file_id) = conn
            .query_row(
                "SELECT file_id FROM workspace_placement_idempotency
                 WHERE source_handle = ?1 AND file_id IS NOT NULL
                 ORDER BY updated_at DESC
                 LIMIT 1",
                params![entry_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(read_failed)?
        {
            return Ok(Some(file_id));
        }
    }

    conn.query_row(
        "SELECT file_id FROM document_ingestion_runs WHERE run_id = ?1",
        params![entry_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(read_failed)
}

fn load_lifecycle(
    conn: &Connection,
    file_id: &str,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<Option<Value>, WorkspaceSourceProvenanceReadError> {
    conn.query_row(
        "SELECT source_type, lifecycle_state, source_asof, category,
                user_override_at, entity_type, entity_id
         FROM workspace_file_lifecycle
         WHERE file_id = ?1",
        params![file_id],
        |row| {
            let user_override_at: Option<String> = row.get(4)?;
            let entity_type: Option<String> = row.get(5)?;
            let entity_id: Option<String> = row.get(6)?;
            Ok(json!({
                "sourceKey": diagnostic_key.workspace_source_handle(file_id),
                "sourceKind": row.get::<_, String>(0)?,
                "lifecycleState": row.get::<_, String>(1)?,
                "sourceAsOf": row.get::<_, String>(2)?,
                "category": row.get::<_, Option<String>>(3)?,
                "entity": match (entity_type, entity_id) {
                    (Some(entity_type), Some(entity_id)) => json!({
                        "entityType": entity_type,
                        "entityId": entity_id,
                        "attributionSource": "lifecycle_binding"
                    }),
                    _ => Value::Null,
                },
                "userOverride": user_override_at.map(|at| json!({
                    "present": true,
                    "at": at
                })),
            }))
        },
    )
    .optional()
    .map_err(read_failed)
}

fn load_entities(
    conn: &Connection,
    file_id: &str,
) -> Result<Vec<Value>, WorkspaceSourceProvenanceReadError> {
    let mut stmt = conn
        .prepare(
            "SELECT entity_type, entity_id, attribution_source, confidence, user_override_at
             FROM document_entity_links
             WHERE file_id = ?1 AND rejected = 0
             ORDER BY entity_type, entity_id",
        )
        .map_err(read_failed)?;
    let rows = stmt
        .query_map(params![file_id], |row| {
            let confidence: f64 = row.get(3)?;
            let user_override_at: Option<String> = row.get(4)?;
            Ok(json!({
                "entityType": row.get::<_, String>(0)?,
                "entityId": row.get::<_, String>(1)?,
                "attributionSource": row.get::<_, String>(2)?,
                "confidenceBps": confidence_bps(confidence),
                "userOverrideAt": user_override_at,
            }))
        })
        .map_err(read_failed)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(read_failed)
}

fn load_ingestion_runs(
    conn: &Connection,
    file_id: &str,
) -> Result<Vec<Value>, WorkspaceSourceProvenanceReadError> {
    let mut stmt = conn
        .prepare(
            "SELECT mode, started_at, completed_at, status, claim_count_produced
             FROM document_ingestion_runs
             WHERE file_id = ?1
             ORDER BY started_at DESC, id DESC
             LIMIT ?2",
        )
        .map_err(read_failed)?;
    let rows = stmt
        .query_map(params![file_id, MAX_RUN_HISTORY as i64], |row| {
            let claim_count: i64 = row.get(4)?;
            Ok(json!({
                "mode": row.get::<_, String>(0)?,
                "startedAt": row.get::<_, String>(1)?,
                "completedAt": row.get::<_, Option<String>>(2)?,
                "status": row.get::<_, String>(3)?,
                "claimCountProduced": claim_count.max(0).min(u32::MAX as i64) as u32,
            }))
        })
        .map_err(read_failed)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(read_failed)
}

fn load_claim_attribution(
    conn: &Connection,
    file_id: &str,
) -> Result<Value, WorkspaceSourceProvenanceReadError> {
    let source_ref = format!("{WORKSPACE_SOURCE_PREFIX}{file_id}");
    let mut rows = Vec::new();
    collect_claim_rows_for_source_ref(conn, &source_ref, &mut rows)?;
    if table_exists(conn, "claim_semantic_evidence")? {
        collect_claim_rows_for_semantic_evidence(conn, &source_ref, &mut rows)?;
    }

    let mut seen = BTreeSet::new();
    let mut total = 0_u32;
    let mut by_trust_band = BTreeMap::<String, u32>::new();
    let mut by_claim_type = BTreeMap::<String, u32>::new();
    for row in rows {
        if !seen.insert(row.claim_id) || !matches!(row.sensitivity.as_str(), "public" | "internal")
        {
            continue;
        }
        total += 1;
        *by_claim_type.entry(row.claim_type).or_default() += 1;
        *by_trust_band
            .entry(trust_band_label(row.trust_score).to_string())
            .or_default() += 1;
    }

    Ok(json!({
        "total": total,
        "byTrustBand": by_trust_band,
        "byClaimType": by_claim_type,
        "rawClaimIdsIncluded": false,
        "rawClaimTextIncluded": false
    }))
}

#[derive(Debug)]
struct ClaimAttributionRow {
    claim_id: String,
    claim_type: String,
    trust_score: Option<f64>,
    sensitivity: String,
}

fn collect_claim_rows_for_source_ref(
    conn: &Connection,
    source_ref: &str,
    rows: &mut Vec<ClaimAttributionRow>,
) -> Result<(), WorkspaceSourceProvenanceReadError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, claim_type, trust_score, sensitivity
             FROM intelligence_claims
             WHERE claim_state = 'active'
               AND surfacing_state = 'active'
               AND source_ref = ?1",
        )
        .map_err(read_failed)?;
    let mapped = stmt
        .query_map(params![source_ref], claim_row_from_sql)
        .map_err(read_failed)?;
    for row in mapped {
        rows.push(row.map_err(read_failed)?);
    }
    Ok(())
}

fn collect_claim_rows_for_semantic_evidence(
    conn: &Connection,
    source_ref: &str,
    rows: &mut Vec<ClaimAttributionRow>,
) -> Result<(), WorkspaceSourceProvenanceReadError> {
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.claim_type, c.trust_score, c.sensitivity
             FROM intelligence_claims c
             JOIN claim_semantic_evidence e ON e.canonical_claim_id = c.id
             WHERE c.claim_state = 'active'
               AND c.surfacing_state = 'active'
               AND e.source_ref = ?1",
        )
        .map_err(read_failed)?;
    let mapped = stmt
        .query_map(params![source_ref], claim_row_from_sql)
        .map_err(read_failed)?;
    for row in mapped {
        rows.push(row.map_err(read_failed)?);
    }
    Ok(())
}

fn claim_row_from_sql(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClaimAttributionRow> {
    Ok(ClaimAttributionRow {
        claim_id: row.get(0)?,
        claim_type: row.get(1)?,
        trust_score: row.get(2)?,
        sensitivity: row.get(3)?,
    })
}

pub fn trust_band_label(score: Option<f64>) -> &'static str {
    match claim_trust_band_from_score(score) {
        TrustBand::LikelyCurrent => "likely_current",
        TrustBand::UseWithCaution => "use_with_caution",
        TrustBand::NeedsVerification => "needs_verification",
        TrustBand::Unscored => "unscored",
    }
}

fn confidence_bps(confidence: f64) -> u16 {
    if !confidence.is_finite() {
        return 0;
    }
    (confidence.clamp(0.0, 1.0) * 10_000.0).round() as u16
}

fn table_exists(
    conn: &Connection,
    table: &str,
) -> Result<bool, WorkspaceSourceProvenanceReadError> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
        params![table],
        |_| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(read_failed)
}

fn read_failed(error: rusqlite::Error) -> WorkspaceSourceProvenanceReadError {
    WorkspaceSourceProvenanceReadError::ReadFailed(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_like_entry_ids() {
        let err = normalize_workspace_source_entry_id("../private.md")
            .expect_err("path-like ids rejected");
        assert!(matches!(
            err,
            WorkspaceSourceProvenanceReadError::InvalidHandle(_)
        ));
    }

    #[test]
    fn trust_band_labels_are_host_stable() {
        assert_eq!(trust_band_label(Some(0.95)), "likely_current");
        assert_eq!(trust_band_label(None), "unscored");
    }
}

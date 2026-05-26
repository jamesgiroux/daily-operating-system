//! MCP resource support for privacy-rendered entity and source profiles.
//!
//! Resources are transport-level MCP surfaces, not invocable tools. The
//! resource URI carries only a server-derived handle; raw account/person IDs
//! and file IDs stay server-side.

use std::sync::Arc;

use abilities_runtime::abilities::workspace_graph::contracts::{
    WorkspaceGraphEntityFilter, WorkspaceGraphInput, WorkspaceGraphPrivacyProfile,
    WorkspaceGraphReadRequest, WorkspaceGraphResponse,
};
use rmcp::model::{
    Annotated, RawResource, RawResourceTemplate, Resource, ResourceContents, ResourceTemplate,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};

use crate::db::{ActionDb, LocalKeychain};
use crate::services::mcp_v2::contracts::Scope;
use crate::services::workspace_ingestion::graph::{
    local_install_diagnostic_key, read_workspace_graph, WorkspaceGraphDiagnosticKey,
};
use crate::services::workspace_source_provenance::read_workspace_source_provenance;

pub const ENTITY_RESOURCE_SCOPE: &str = "dailyos.read.entity_resource";
pub const ENTITY_NAMES_SCOPE: &str = "read.entity_names";

const ACCOUNT_KIND: &str = "account";
const PERSON_KIND: &str = "person";
const SOURCE_KIND: &str = "source";
const RESOURCE_MIME_TYPE: &str = "application/json";
const LIST_LIMIT_PER_KIND: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Account,
    Person,
    Source,
}

impl ResourceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Account => ACCOUNT_KIND,
            Self::Person => PERSON_KIND,
            Self::Source => SOURCE_KIND,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyOsResourceRef {
    pub kind: ResourceKind,
    pub handle: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ResourceReadError {
    #[error("resource requires dailyos.read.entity_resource scope")]
    Unauthorized,
    #[error("{0}")]
    BadUri(String),
    #[error("resource not found")]
    NotFound,
    #[error("resource read failed: {0}")]
    ReadFailed(String),
}

pub fn can_read_entity_resources(scopes: &[Scope]) -> bool {
    scopes
        .iter()
        .any(|scope| scope.as_str() == ENTITY_RESOURCE_SCOPE)
}

pub fn can_read_entity_names(scopes: &[Scope]) -> bool {
    scopes
        .iter()
        .any(|scope| scope.as_str() == ENTITY_NAMES_SCOPE)
}

pub fn parse_dailyos_resource_uri(uri: &str) -> Result<DailyOsResourceRef, ResourceReadError> {
    let Some(rest) = uri.strip_prefix("dailyos://") else {
        return Err(ResourceReadError::BadUri(
            "resource URI must use the dailyos:// scheme".to_string(),
        ));
    };
    let Some((kind, handle)) = rest.split_once('/') else {
        return Err(ResourceReadError::BadUri(
            "resource URI must be dailyos://{account|person|source}/{handle}".to_string(),
        ));
    };
    if handle.contains('/') || handle.contains('\\') || handle.contains('.') || handle.is_empty() {
        return Err(ResourceReadError::BadUri(
            "resource handle must be an opaque handle, not a path".to_string(),
        ));
    }
    if handle.len() > 160
        || !handle
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-'))
    {
        return Err(ResourceReadError::BadUri(
            "resource handle contains unsupported characters".to_string(),
        ));
    }
    let kind = match kind {
        ACCOUNT_KIND => ResourceKind::Account,
        PERSON_KIND => ResourceKind::Person,
        SOURCE_KIND => ResourceKind::Source,
        _ => {
            return Err(ResourceReadError::BadUri(
                "resource kind must be account, person, or source".to_string(),
            ))
        }
    };
    Ok(DailyOsResourceRef {
        kind,
        handle: handle.to_string(),
    })
}

pub fn mint_resource_handle(
    kind: ResourceKind,
    stable_id: &str,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> String {
    format!(
        "resource:v1:{}:{}",
        kind.as_str(),
        diagnostic_key.mcp_resource_handle(kind.as_str(), stable_id)
    )
}

pub fn list_resource_templates() -> Vec<ResourceTemplate> {
    [
        (
            "dailyos://account/{handle}",
            "DailyOS account profile",
            "Privacy-rendered account profile keyed by an opaque DailyOS resource handle.",
        ),
        (
            "dailyos://person/{handle}",
            "DailyOS person profile",
            "Privacy-rendered person profile keyed by an opaque DailyOS resource handle.",
        ),
        (
            "dailyos://source/{handle}",
            "DailyOS source profile",
            "Privacy-rendered source provenance profile keyed by an opaque DailyOS resource handle.",
        ),
    ]
    .into_iter()
    .map(|(uri_template, name, description)| {
        Annotated::new(
            RawResourceTemplate {
                uri_template: uri_template.to_string(),
                name: name.to_string(),
                description: Some(description.to_string()),
                mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            },
            None,
        )
    })
    .collect()
}

pub fn list_resource_summaries_from_local_db(
    scopes: &[Scope],
) -> Result<Vec<Resource>, ResourceReadError> {
    let db = ActionDb::open_readonly(Arc::new(LocalKeychain::new()))
        .map_err(|error| ResourceReadError::ReadFailed(error.to_string()))?;
    let diagnostic_key = local_install_diagnostic_key().map_err(ResourceReadError::ReadFailed)?;
    list_resource_summaries(db.conn_ref(), scopes, &diagnostic_key)
}

pub fn list_resource_summaries(
    conn: &Connection,
    scopes: &[Scope],
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<Vec<Resource>, ResourceReadError> {
    require_resource_scope(scopes)?;
    let include_names = can_read_entity_names(scopes);
    let mut resources = Vec::new();
    resources.extend(list_account_resources(conn, include_names, diagnostic_key)?);
    resources.extend(list_person_resources(conn, include_names, diagnostic_key)?);
    resources.extend(list_source_resources(conn, diagnostic_key)?);
    Ok(resources)
}

pub fn read_resource_from_local_db(
    uri: &str,
    scopes: &[Scope],
) -> Result<Value, ResourceReadError> {
    let db = ActionDb::open_readonly(Arc::new(LocalKeychain::new()))
        .map_err(|error| ResourceReadError::ReadFailed(error.to_string()))?;
    let diagnostic_key = local_install_diagnostic_key().map_err(ResourceReadError::ReadFailed)?;
    read_resource(db.conn_ref(), uri, scopes, &diagnostic_key)
}

pub fn read_resource(
    conn: &Connection,
    uri: &str,
    scopes: &[Scope],
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<Value, ResourceReadError> {
    require_resource_scope(scopes)?;
    let resource_ref = parse_dailyos_resource_uri(uri)?;
    let include_names = can_read_entity_names(scopes);
    match resource_ref.kind {
        ResourceKind::Account => read_entity_resource(
            conn,
            uri,
            ResourceKind::Account,
            &resource_ref.handle,
            include_names,
            diagnostic_key,
        ),
        ResourceKind::Person => read_entity_resource(
            conn,
            uri,
            ResourceKind::Person,
            &resource_ref.handle,
            include_names,
            diagnostic_key,
        ),
        ResourceKind::Source => read_source_resource(
            conn,
            uri,
            &resource_ref.handle,
            include_names,
            diagnostic_key,
        ),
    }
}

pub fn resource_contents(uri: &str, value: &Value) -> Result<ResourceContents, ResourceReadError> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| ResourceReadError::ReadFailed(error.to_string()))?;
    Ok(ResourceContents::TextResourceContents {
        uri: uri.to_string(),
        mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
        text,
    })
}

fn require_resource_scope(scopes: &[Scope]) -> Result<(), ResourceReadError> {
    if can_read_entity_resources(scopes) {
        Ok(())
    } else {
        Err(ResourceReadError::Unauthorized)
    }
}

fn list_account_resources(
    conn: &Connection,
    include_names: bool,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<Vec<Resource>, ResourceReadError> {
    if !table_exists(conn, "accounts")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT id, name
             FROM accounts
             WHERE COALESCE(archived, 0) = 0
             ORDER BY COALESCE(updated_at, '') DESC, name ASC
             LIMIT ?1",
        )
        .map_err(read_failed)?;
    let rows = stmt
        .query_map(params![LIST_LIMIT_PER_KIND as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(read_failed)?;
    let mut resources = Vec::new();
    for row in rows {
        let (id, name) = row.map_err(read_failed)?;
        resources.push(resource_summary(
            ResourceKind::Account,
            &id,
            name.as_deref(),
            include_names,
            diagnostic_key,
        ));
    }
    Ok(resources)
}

fn list_person_resources(
    conn: &Connection,
    include_names: bool,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<Vec<Resource>, ResourceReadError> {
    if !table_exists(conn, "people")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT id, CASE WHEN trim(COALESCE(name, '')) = '' THEN email ELSE name END
             FROM people
             WHERE COALESCE(archived, 0) = 0
             ORDER BY COALESCE(updated_at, '') DESC, name ASC
             LIMIT ?1",
        )
        .map_err(read_failed)?;
    let rows = stmt
        .query_map(params![LIST_LIMIT_PER_KIND as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(read_failed)?;
    let mut resources = Vec::new();
    for row in rows {
        let (id, name) = row.map_err(read_failed)?;
        resources.push(resource_summary(
            ResourceKind::Person,
            &id,
            name.as_deref(),
            include_names,
            diagnostic_key,
        ));
    }
    Ok(resources)
}

fn list_source_resources(
    conn: &Connection,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<Vec<Resource>, ResourceReadError> {
    if !table_exists(conn, "workspace_file_lifecycle")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT file_id, source_type, category, source_asof
             FROM workspace_file_lifecycle
             ORDER BY COALESCE(source_asof, '') DESC, file_id ASC
             LIMIT ?1",
        )
        .map_err(read_failed)?;
    let rows = stmt
        .query_map(params![LIST_LIMIT_PER_KIND as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(read_failed)?;
    let mut resources = Vec::new();
    for row in rows {
        let (file_id, source_type, category, source_asof) = row.map_err(read_failed)?;
        let handle = diagnostic_key.workspace_source_handle(&file_id);
        let label = source_type.unwrap_or_else(|| "workspace_file".to_string());
        let mut description = format!("DailyOS source profile ({label})");
        if let Some(category) = category {
            description.push_str(&format!(", category: {category}"));
        }
        if let Some(source_asof) = source_asof {
            description.push_str(&format!(", source as of: {source_asof}"));
        }
        resources.push(Annotated::new(
            RawResource {
                uri: resource_uri(ResourceKind::Source, &handle),
                name: format!("DailyOS source {}", short_handle(&handle)),
                description: Some(description),
                mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
                size: None,
            },
            None,
        ));
    }
    Ok(resources)
}

fn resource_summary(
    kind: ResourceKind,
    stable_id: &str,
    entity_name: Option<&str>,
    include_names: bool,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Resource {
    let handle = mint_resource_handle(kind, stable_id, diagnostic_key);
    let kind_label = kind.as_str();
    let name = if include_names {
        entity_name
            .filter(|name| !name.trim().is_empty())
            .map(|name| format!("{kind_label}: {name}"))
            .unwrap_or_else(|| format!("DailyOS {kind_label} {}", short_handle(&handle)))
    } else {
        format!("DailyOS {kind_label} {}", short_handle(&handle))
    };
    Annotated::new(
        RawResource {
            uri: resource_uri(kind, &handle),
            name,
            description: Some(format!(
                "Privacy-rendered DailyOS {kind_label} profile. The URI handle is opaque."
            )),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            size: None,
        },
        None,
    )
}

fn read_entity_resource(
    conn: &Connection,
    uri: &str,
    kind: ResourceKind,
    handle: &str,
    include_names: bool,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<Value, ResourceReadError> {
    let Some(entity) = resolve_entity_handle(conn, kind, handle, diagnostic_key)? else {
        return Err(ResourceReadError::NotFound);
    };
    let graph = read_workspace_graph(
        conn,
        WorkspaceGraphReadRequest {
            input: WorkspaceGraphInput {
                schema_version: 1,
                entity_filter: Some(WorkspaceGraphEntityFilter {
                    entity_types: Some(vec![kind.as_str().to_string()]),
                    entity_ids: Some(vec![entity.stable_id.clone()]),
                }),
                category_filter: None,
                cursor: None,
                if_none_match: None,
                include_entity_names: include_names,
                page_size: 1,
            },
            privacy_profile: WorkspaceGraphPrivacyProfile::SurfaceClient,
        },
        diagnostic_key,
    )
    .map_err(|error| ResourceReadError::ReadFailed(error.to_string()))?;

    let (file_links, claim_summary, graph_version) = match graph {
        WorkspaceGraphResponse::Projection(projection) => {
            let graph_version = projection.graph_version;
            let entity_projection = projection.projection.entities.into_iter().next();
            if let Some(entity_projection) = entity_projection {
                (
                    serde_json::to_value(entity_projection.file_links).map_err(encode_failed)?,
                    serde_json::to_value(entity_projection.claim_summary).map_err(encode_failed)?,
                    graph_version,
                )
            } else {
                (
                    json!([]),
                    json!({ "total": 0, "byTrustBand": {} }),
                    graph_version,
                )
            }
        }
        WorkspaceGraphResponse::NotModified(not_modified) => (
            json!([]),
            json!({ "total": 0, "byTrustBand": {} }),
            not_modified.graph_version,
        ),
    };

    Ok(json!({
        "schemaVersion": 1,
        "uri": uri,
        "resourceType": kind.as_str(),
        "handle": handle,
        "entity": {
            "type": kind.as_str(),
            "name": if include_names { entity.name } else { None },
            "nameIncluded": include_names
        },
        "workspaceMemory": {
            "graphVersion": graph_version,
            "fileLinks": file_links,
            "claimSummary": claim_summary
        },
        "privacy": {
            "rawEntityIdIncluded": false,
            "entityNameIncluded": include_names,
            "rawFilePathIncluded": false,
            "rawFileIdIncluded": false,
            "rawClaimIdsIncluded": false,
            "rawClaimTextIncluded": false
        }
    }))
}

fn read_source_resource(
    conn: &Connection,
    uri: &str,
    handle: &str,
    include_names: bool,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<Value, ResourceReadError> {
    let Some(mut value) = read_workspace_source_provenance(conn, handle, diagnostic_key)
        .map_err(|error| ResourceReadError::ReadFailed(error.to_string()))?
    else {
        return Err(ResourceReadError::NotFound);
    };
    scrub_source_provenance(conn, &mut value, include_names, diagnostic_key)?;
    Ok(json!({
        "schemaVersion": 1,
        "uri": uri,
        "resourceType": SOURCE_KIND,
        "handle": handle,
        "sourceProvenance": value,
        "privacy": {
            "rawEntityIdIncluded": false,
            "entityNameIncluded": include_names,
            "rawFilePathIncluded": false,
            "rawFileIdIncluded": false,
            "rawClaimIdsIncluded": false,
            "rawClaimTextIncluded": false
        }
    }))
}

fn scrub_source_provenance(
    conn: &Connection,
    value: &mut Value,
    include_names: bool,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<(), ResourceReadError> {
    if let Some(source) = value.get_mut("source").and_then(Value::as_object_mut) {
        if let Some(entity) = source.get_mut("entity").and_then(Value::as_object_mut) {
            scrub_entity_binding(conn, entity, include_names, diagnostic_key)?;
        }
    }
    if let Some(entities) = value.get_mut("entities").and_then(Value::as_array_mut) {
        for entity in entities {
            if let Some(entity) = entity.as_object_mut() {
                scrub_entity_binding(conn, entity, include_names, diagnostic_key)?;
            }
        }
    }
    Ok(())
}

fn scrub_entity_binding(
    conn: &Connection,
    entity: &mut Map<String, Value>,
    include_names: bool,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<(), ResourceReadError> {
    let entity_type = entity
        .get("entityType")
        .and_then(Value::as_str)
        .map(str::to_string);
    let entity_id = entity
        .remove("entityId")
        .and_then(|value| value.as_str().map(str::to_string));
    if let (Some(entity_type), Some(entity_id)) = (entity_type, entity_id) {
        let kind = match entity_type.as_str() {
            ACCOUNT_KIND => Some(ResourceKind::Account),
            PERSON_KIND => Some(ResourceKind::Person),
            _ => None,
        };
        if let Some(kind) = kind {
            entity.insert(
                "entityHandle".to_string(),
                Value::String(mint_resource_handle(kind, &entity_id, diagnostic_key)),
            );
            if include_names {
                entity.insert(
                    "entityName".to_string(),
                    entity_name(conn, kind, &entity_id)?
                        .map(Value::String)
                        .unwrap_or(Value::Null),
                );
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
struct ResolvedEntity {
    stable_id: String,
    name: Option<String>,
}

fn resolve_entity_handle(
    conn: &Connection,
    kind: ResourceKind,
    handle: &str,
    diagnostic_key: &WorkspaceGraphDiagnosticKey,
) -> Result<Option<ResolvedEntity>, ResourceReadError> {
    let sql = match kind {
        ResourceKind::Account => {
            "SELECT id, name FROM accounts WHERE COALESCE(archived, 0) = 0 ORDER BY id ASC"
        }
        ResourceKind::Person => {
            "SELECT id, CASE WHEN trim(COALESCE(name, '')) = '' THEN email ELSE name END
             FROM people WHERE COALESCE(archived, 0) = 0 ORDER BY id ASC"
        }
        ResourceKind::Source => return Ok(None),
    };
    let mut stmt = conn.prepare(sql).map_err(read_failed)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(read_failed)?;
    for row in rows {
        let (stable_id, name) = row.map_err(read_failed)?;
        if mint_resource_handle(kind, &stable_id, diagnostic_key) == handle {
            return Ok(Some(ResolvedEntity { stable_id, name }));
        }
    }
    Ok(None)
}

fn entity_name(
    conn: &Connection,
    kind: ResourceKind,
    stable_id: &str,
) -> Result<Option<String>, ResourceReadError> {
    let sql = match kind {
        ResourceKind::Account => {
            "SELECT name FROM accounts WHERE id = ?1 AND COALESCE(archived, 0) = 0"
        }
        ResourceKind::Person => {
            "SELECT CASE WHEN trim(COALESCE(name, '')) = '' THEN email ELSE name END
             FROM people WHERE id = ?1 AND COALESCE(archived, 0) = 0"
        }
        ResourceKind::Source => return Ok(None),
    };
    conn.query_row(sql, params![stable_id], |row| {
        row.get::<_, Option<String>>(0)
    })
    .optional()
    .map_err(read_failed)
    .map(Option::flatten)
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, ResourceReadError> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type = 'table' AND name = ?1
        )",
        params![table],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count != 0)
    .map_err(read_failed)
}

fn resource_uri(kind: ResourceKind, handle: &str) -> String {
    format!("dailyos://{}/{handle}", kind.as_str())
}

fn short_handle(handle: &str) -> String {
    handle
        .rsplit(':')
        .next()
        .unwrap_or(handle)
        .chars()
        .take(8)
        .collect()
}

fn read_failed(error: rusqlite::Error) -> ResourceReadError {
    ResourceReadError::ReadFailed(error.to_string())
}

fn encode_failed(error: serde_json::Error) -> ResourceReadError {
    ResourceReadError::ReadFailed(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::workspace_ingestion::graph::diagnostic_key_for_tests;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("memory db");
        conn.execute_batch(
            r#"
            CREATE TABLE accounts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                archived INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT
            );
            CREATE TABLE people (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL,
                name TEXT NOT NULL,
                archived INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT
            );
            CREATE TABLE workspace_file_lifecycle (
                file_id TEXT PRIMARY KEY,
                canonical_path TEXT NOT NULL,
                source_type TEXT NOT NULL,
                lifecycle_state TEXT NOT NULL,
                source_asof TEXT NOT NULL,
                entity_id TEXT,
                entity_type TEXT,
                category TEXT,
                user_override_at TEXT
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
                claim_count_produced INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE document_entity_links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                link_id TEXT NOT NULL UNIQUE,
                file_id TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                attribution_source TEXT NOT NULL,
                confidence REAL NOT NULL,
                user_override_at TEXT,
                rejected INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT
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
            "#,
        )
        .expect("schema");
        conn
    }

    fn seed_account_source(conn: &Connection) {
        conn.execute(
            "INSERT INTO accounts (id, name, archived, updated_at)
             VALUES ('acct-1', 'Example Account', 0, '2026-05-25T10:00:00Z')",
            [],
        )
        .expect("account");
        conn.execute(
            "INSERT INTO workspace_file_lifecycle
             (file_id, canonical_path, source_type, lifecycle_state, source_asof, entity_id, entity_type, category)
             VALUES ('file-1', '/private/example.md', 'mcp_placement', 'ingested', '2026-05-25T10:00:00Z', 'acct-1', 'account', 'notes')",
            [],
        )
        .expect("lifecycle");
        conn.execute(
            "INSERT INTO document_entity_links
             (link_id, file_id, entity_type, entity_id, attribution_source, confidence)
             VALUES ('link-1', 'file-1', 'account', 'acct-1', 'entity_hint', 0.9)",
            [],
        )
        .expect("link");
        conn.execute(
            "INSERT INTO document_ingestion_runs
             (run_id, file_id, mode, started_at, completed_at, status, content_sha256, file_size_bytes, extractor_version, claim_count_produced)
             VALUES ('run-1', 'file-1', 'initial', '2026-05-25T10:00:00Z', '2026-05-25T10:00:01Z', 'success', 'hash', 100, 'test', 1)",
            [],
        )
        .expect("run");
        conn.execute(
            "INSERT INTO intelligence_claims
             (id, subject_ref, claim_type, text, dedup_key, actor, data_source, source_ref,
              observed_at, created_at, provenance_json, claim_state, surfacing_state,
              temporal_scope, sensitivity, trust_score)
             VALUES
             ('claim-1', ?1, 'account_fact', 'Private claim text must not leak', 'dedup-1',
              'system', 'workspace', 'workspace_file:file-1', '2026-05-25T10:00:00Z',
              '2026-05-25T10:00:00Z', '{}', 'active', 'active', 'current', 'internal', 0.82)",
            [r#"[{"type":"account","id":"acct-1"}]"#],
        )
        .expect("claim");
    }

    #[test]
    fn parser_accepts_only_dailyos_resource_uri_shapes() {
        let parsed =
            parse_dailyos_resource_uri("dailyos://account/resource:v1:account:abc").unwrap();
        assert_eq!(parsed.kind, ResourceKind::Account);
        assert_eq!(parsed.handle, "resource:v1:account:abc");

        assert!(parse_dailyos_resource_uri("file:///tmp/secret").is_err());
        assert!(parse_dailyos_resource_uri("dailyos://account/../secret").is_err());
        assert!(parse_dailyos_resource_uri("dailyos://project/resource:v1:project:abc").is_err());
    }

    #[test]
    fn handle_minting_is_deterministic_and_opaque() {
        let key = diagnostic_key_for_tests("mcp-resource-handle");
        let first = mint_resource_handle(ResourceKind::Account, "acct-1", &key);
        let second = mint_resource_handle(ResourceKind::Account, "acct-1", &key);
        assert_eq!(first, second);
        assert!(first.starts_with("resource:v1:account:"));
        assert!(!first.contains("acct-1"));
    }

    #[test]
    fn raw_entity_ids_do_not_resolve_as_resource_handles() {
        let conn = setup_conn();
        seed_account_source(&conn);
        let key = diagnostic_key_for_tests("mcp-resource-resolve");
        let scopes = vec![Scope::new(ENTITY_RESOURCE_SCOPE)];
        let err = read_resource(&conn, "dailyos://account/acct-1", &scopes, &key).unwrap_err();
        assert!(matches!(err, ResourceReadError::NotFound));
    }

    #[test]
    fn account_resource_omits_raw_ids_names_and_claim_text_without_name_scope() {
        let conn = setup_conn();
        seed_account_source(&conn);
        let key = diagnostic_key_for_tests("mcp-account-resource");
        let handle = mint_resource_handle(ResourceKind::Account, "acct-1", &key);
        let value = read_resource(
            &conn,
            &format!("dailyos://account/{handle}"),
            &[Scope::new(ENTITY_RESOURCE_SCOPE)],
            &key,
        )
        .expect("resource");
        let serialized = serde_json::to_string(&value).expect("json");
        assert!(!serialized.contains("acct-1"));
        assert!(!serialized.contains("Example Account"));
        assert!(!serialized.contains("Private claim text"));
        assert_eq!(value["privacy"]["rawEntityIdIncluded"], false);
        assert_eq!(value["entity"]["name"], Value::Null);
    }

    #[test]
    fn source_resource_replaces_entity_ids_with_resource_handles() {
        let conn = setup_conn();
        seed_account_source(&conn);
        let key = diagnostic_key_for_tests("mcp-source-resource");
        let source_handle = key.workspace_source_handle("file-1");
        let value = read_resource(
            &conn,
            &format!("dailyos://source/{source_handle}"),
            &[
                Scope::new(ENTITY_RESOURCE_SCOPE),
                Scope::new(ENTITY_NAMES_SCOPE),
            ],
            &key,
        )
        .expect("source resource");
        let serialized = serde_json::to_string(&value).expect("json");
        assert!(!serialized.contains("acct-1"));
        assert!(!serialized.contains("/private/example.md"));
        assert!(!serialized.contains("Private claim text"));
        assert!(serialized.contains("Example Account"));
        assert!(serialized.contains("entityHandle"));
    }
}

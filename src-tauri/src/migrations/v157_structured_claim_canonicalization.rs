use rusqlite::{params, Connection};

use crate::services::claims::{
    metadata_with_non_semantic_mergeable, structural_field_content_hash,
    structured_claim_json_for_row,
};
use abilities_runtime::structured_claim::{
    ClaimStatus as StructuredClaimStatus, Polarity, StructuredClaim,
};

use super::MigrationError;

const CLAIMS_TABLE: &str = "intelligence_claims";

struct ClaimRow {
    id: String,
    subject_ref: String,
    claim_type: String,
    field_path: Option<String>,
    topic_key: Option<String>,
    text: String,
    metadata_json: Option<String>,
    verification_state: String,
}

pub(super) fn migrate_v157_structured_claim_canonicalization(
    conn: &Connection,
) -> Result<(), MigrationError> {
    if !table_exists(conn, CLAIMS_TABLE)? {
        return Err(format!("required table {CLAIMS_TABLE} is missing"));
    }

    execute_batch(conn, "BEGIN IMMEDIATE;", "begin immediate transaction")?;
    let result = migrate_in_transaction(conn);
    match result {
        Ok(()) => execute_batch(conn, "COMMIT;", "commit transaction"),
        Err(error) => {
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort cleanup after migration failure"
            )]
            let _ = conn.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

fn migrate_in_transaction(conn: &Connection) -> Result<(), MigrationError> {
    add_column_if_missing(conn, "structured_claim_json", "TEXT")?;
    add_column_if_missing(conn, "predicate_ref", "TEXT")?;
    add_column_if_missing(conn, "polarity", "TEXT")?;
    add_column_if_missing(conn, "object_value", "JSON")?;
    add_column_if_missing(conn, "qualifiers", "JSON")?;
    add_column_if_missing(conn, "structural_canonical_id", "TEXT")?;
    add_column_if_missing(
        conn,
        "canonical_status",
        "TEXT NOT NULL DEFAULT 'pending_backfill'
            CHECK (canonical_status IN ('pending_backfill','legacy_unmigrated','live'))",
    )?;
    add_column_if_missing(
        conn,
        "non_semantic_mergeable",
        "BOOLEAN NOT NULL DEFAULT TRUE",
    )?;
    add_column_if_missing(conn, "structural_field_content_hash", "TEXT")?;
    add_column_if_missing(conn, "backfill_epoch", "INTEGER NOT NULL DEFAULT 0")?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS canonicalization_decisions (
            decision_id TEXT PRIMARY KEY,
            claim_id_a TEXT NOT NULL,
            claim_id_b TEXT NOT NULL,
            decision TEXT NOT NULL
                CHECK (decision IN ('merge','fork','fork_ambiguous','fork_contradiction','fork_filtered')),
            mode TEXT NOT NULL CHECK (mode IN ('shadow','live')),
            is_authoritative BOOLEAN NOT NULL GENERATED ALWAYS AS (mode = 'live') STORED,
            field_scores JSONB NOT NULL,
            reason TEXT NOT NULL,
            reason_secondary JSONB,
            threshold_band TEXT CHECK (
                threshold_band IS NULL OR threshold_band IN ('high','ambiguous','low')
            ),
            embedding_model_version TEXT,
            comparator_threshold_version TEXT,
            field_provenance JSONB NOT NULL,
            canonicalization_mode TEXT NOT NULL CHECK (
                canonicalization_mode IN ('full','hash_fallback')
            ),
            supersedes_decision_id TEXT REFERENCES canonicalization_decisions(decision_id),
            idempotency_key TEXT NOT NULL UNIQUE,
            claim_a_revision_hash TEXT NOT NULL,
            claim_b_revision_hash TEXT NOT NULL,
            evaluated_at TIMESTAMP NOT NULL,
            FOREIGN KEY (claim_id_a) REFERENCES intelligence_claims(id),
            FOREIGN KEY (claim_id_b) REFERENCES intelligence_claims(id)
        );

        CREATE TABLE IF NOT EXISTS ambiguous_claim_pairs (
            pair_id TEXT PRIMARY KEY,
            claim_id_a TEXT NOT NULL,
            claim_id_b TEXT NOT NULL,
            field_scores JSONB NOT NULL,
            decision_id TEXT NOT NULL REFERENCES canonicalization_decisions(decision_id),
            user_resolution TEXT CHECK (
                user_resolution IS NULL
                OR user_resolution IN ('merged','forked','contradicted','needs_user_decision')
            ),
            user_resolved_at TIMESTAMP,
            reconcile_attempts INT NOT NULL DEFAULT 0,
            next_reconcile_at TIMESTAMP,
            last_schema_version TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL,
            FOREIGN KEY (claim_id_a) REFERENCES intelligence_claims(id),
            FOREIGN KEY (claim_id_b) REFERENCES intelligence_claims(id)
        );

        CREATE INDEX IF NOT EXISTS idx_canonicalization_decisions_pair
            ON canonicalization_decisions(claim_id_a, claim_id_b, mode);
        CREATE INDEX IF NOT EXISTS idx_canonicalization_decisions_idempotency
            ON canonicalization_decisions(idempotency_key);
        CREATE INDEX IF NOT EXISTS idx_ambiguous_claim_pairs_decision
            ON ambiguous_claim_pairs(decision_id);
        CREATE INDEX IF NOT EXISTS idx_ambiguous_claim_pairs_reconcile
            ON ambiguous_claim_pairs(last_schema_version, next_reconcile_at);
        CREATE INDEX IF NOT EXISTS idx_intelligence_claims_canonical_status
            ON intelligence_claims(canonical_status, non_semantic_mergeable);
        ",
    )
    .map_err(|e| format!("create ADR-0131 canonicalization tables: {e}"))?;

    for row in claim_rows(conn)? {
        match structured_claim_json_for_row(
            &row.subject_ref,
            &row.claim_type,
            row.field_path.as_deref(),
            row.topic_key.as_deref(),
            &row.text,
            row.metadata_json.as_deref(),
            &row.verification_state,
        ) {
            Ok(Some(structured_claim_json)) => {
                let structured = serde_json::from_str::<StructuredClaim>(&structured_claim_json)
                    .map_err(|e| format!("parse structured sidecar for {}: {e}", row.id))?;
                let predicate_ref = structured.predicate.registry_id();
                let polarity = match structured.polarity {
                    Polarity::Affirm => "affirm",
                    Polarity::Negate => "negate",
                };
                let object_value = serde_json::to_string(&structured.object)
                    .map_err(|e| format!("serialize object_value for {}: {e}", row.id))?;
                let qualifiers = serde_json::to_string(&structured.qualifiers)
                    .map_err(|e| format!("serialize qualifiers for {}: {e}", row.id))?;
                let status = match structured.status {
                    StructuredClaimStatus::Confirmed => "confirmed",
                    StructuredClaimStatus::Pending => "pending",
                    StructuredClaimStatus::Unknown => "unknown",
                };
                let content_hash = structural_field_content_hash(
                    Some(&predicate_ref),
                    Some(polarity),
                    Some(&object_value),
                    Some(&qualifiers),
                    status,
                );
                conn.execute(
                    "UPDATE intelligence_claims
                     SET structured_claim_json = ?1,
                         predicate_ref = ?2,
                         polarity = ?3,
                         object_value = ?4,
                         qualifiers = ?5,
                         structural_field_content_hash = ?6,
                         canonical_status = 'live',
                         non_semantic_mergeable = FALSE,
                         backfill_epoch = backfill_epoch + 1
                     WHERE id = ?7",
                    params![
                        structured_claim_json,
                        predicate_ref,
                        polarity,
                        object_value,
                        qualifiers,
                        content_hash,
                        row.id
                    ],
                )
                .map_err(|e| format!("backfill structured claim sidecar: {e}"))?;
            }
            Ok(None) | Err(_) => {
                let metadata_json = metadata_with_non_semantic_mergeable(
                    row.metadata_json.as_deref(),
                )
                .ok_or_else(|| "mark unresolved claim non-semantic mergeable".to_string())?;
                conn.execute(
                    "UPDATE intelligence_claims
                     SET metadata_json = ?1,
                         structured_claim_json = NULL,
                         predicate_ref = NULL,
                         polarity = NULL,
                         object_value = NULL,
                         qualifiers = NULL,
                         structural_field_content_hash = NULL,
                         canonical_status = 'legacy_unmigrated',
                         non_semantic_mergeable = TRUE,
                         backfill_epoch = backfill_epoch + 1
                     WHERE id = ?2",
                    params![metadata_json, row.id],
                )
                .map_err(|e| format!("mark unresolved structured claim fail-closed: {e}"))?;
            }
        }
    }

    Ok(())
}

fn claim_rows(conn: &Connection) -> Result<Vec<ClaimRow>, MigrationError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, subject_ref, claim_type, field_path, topic_key, text,
                    metadata_json, verification_state
             FROM intelligence_claims
             ORDER BY created_at, id",
        )
        .map_err(|e| format!("prepare structured claim backfill scan: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ClaimRow {
                id: row.get(0)?,
                subject_ref: row.get(1)?,
                claim_type: row.get(2)?,
                field_path: row.get(3)?,
                topic_key: row.get(4)?,
                text: row.get(5)?,
                metadata_json: row.get(6)?,
                verification_state: row.get(7)?,
            })
        })
        .map_err(|e| format!("query structured claim backfill scan: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("read structured claim backfill row: {e}"))?;
    Ok(rows)
}

fn add_column_if_missing(
    conn: &Connection,
    column_name: &str,
    column_definition: &str,
) -> Result<(), MigrationError> {
    if column_exists(conn, CLAIMS_TABLE, column_name)? {
        return Ok(());
    }
    conn.execute(
        &format!("ALTER TABLE {CLAIMS_TABLE} ADD COLUMN {column_name} {column_definition}"),
        [],
    )
    .map_err(|e| format!("add {CLAIMS_TABLE}.{column_name}: {e}"))?;
    Ok(())
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, MigrationError> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM sqlite_master
            WHERE type = 'table' AND name = ?1
        )",
        [table_name],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count != 0)
    .map_err(|e| format!("check table {table_name}: {e}"))
}

fn column_exists(
    conn: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, MigrationError> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .map_err(|e| format!("prepare table info for {table_name}: {e}"))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("query table info for {table_name}: {e}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("read table info for {table_name}: {e}"))?
    {
        let name: String = row
            .get(1)
            .map_err(|e| format!("read column name for {table_name}: {e}"))?;
        if name == column_name {
            return Ok(true);
        }
    }
    Ok(false)
}

fn execute_batch(conn: &Connection, sql: &str, label: &str) -> Result<(), MigrationError> {
    conn.execute_batch(sql).map_err(|e| format!("{label}: {e}"))
}

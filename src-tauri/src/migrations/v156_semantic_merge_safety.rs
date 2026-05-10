use std::collections::HashSet;

use rusqlite::{params, Connection};
use serde_json::{Map, Value};

use crate::services::claims::semantic_high_salience_qualifiers;

use super::MigrationError;

const CLAIMS_TABLE: &str = "intelligence_claims";
const SEMANTIC_QUALIFIERS_METADATA_KEY: &str = "semantic_qualifiers";
const NON_SEMANTIC_MERGEABLE_METADATA_KEY: &str = "non_semantic_mergeable";
const ORIGINAL_TEXT_KEYS: &[&str] = &[
    "original_text",
    "originalText",
    "source_text",
    "sourceText",
    "raw_text",
    "rawText",
    "claim_text",
    "claimText",
    "evidence_text",
    "evidenceText",
    "quote",
    "snippet",
    "text",
];

struct LegacyClaimMetadata {
    id: String,
    metadata_json: Option<String>,
    provenance_json: String,
}

pub(super) fn migrate_v156_semantic_merge_safety(conn: &Connection) -> Result<(), MigrationError> {
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
    let rows = legacy_claim_rows(conn)?;
    for row in rows {
        let Some(metadata_json) =
            migrated_metadata_json(row.metadata_json.as_deref(), &row.provenance_json)?
        else {
            continue;
        };
        conn.execute(
            "UPDATE intelligence_claims
             SET metadata_json = ?1
             WHERE id = ?2",
            params![metadata_json, row.id],
        )
        .map_err(|e| format!("update legacy claim metadata: {e}"))?;
    }

    Ok(())
}

fn legacy_claim_rows(conn: &Connection) -> Result<Vec<LegacyClaimMetadata>, MigrationError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, metadata_json, provenance_json
             FROM intelligence_claims
             WHERE claim_state = 'active'
               AND surfacing_state = 'active'",
        )
        .map_err(|e| format!("prepare legacy claim scan: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(LegacyClaimMetadata {
                id: row.get(0)?,
                metadata_json: row.get(1)?,
                provenance_json: row.get(2)?,
            })
        })
        .map_err(|e| format!("query legacy claim scan: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("read legacy claim scan row: {e}"))?;
    Ok(rows)
}

fn migrated_metadata_json(
    metadata_json: Option<&str>,
    provenance_json: &str,
) -> Result<Option<String>, MigrationError> {
    let mut metadata = match metadata_json {
        Some(raw) => match serde_json::from_str::<Value>(raw) {
            Ok(Value::Object(map)) => map,
            Ok(_) | Err(_) => return Ok(None),
        },
        None => Map::new(),
    };

    if metadata.contains_key(SEMANTIC_QUALIFIERS_METADATA_KEY)
        || metadata
            .get(NON_SEMANTIC_MERGEABLE_METADATA_KEY)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return Ok(None);
    }

    if let Some(qualifiers) = recover_qualifiers_from_provenance(provenance_json) {
        metadata.insert(
            SEMANTIC_QUALIFIERS_METADATA_KEY.to_string(),
            sorted_qualifier_value(&qualifiers),
        );
    } else {
        metadata.insert(
            NON_SEMANTIC_MERGEABLE_METADATA_KEY.to_string(),
            Value::Bool(true),
        );
    }

    Ok(Some(Value::Object(metadata).to_string()))
}

fn recover_qualifiers_from_provenance(provenance_json: &str) -> Option<HashSet<String>> {
    let provenance = serde_json::from_str::<Value>(provenance_json).ok()?;
    let mut candidates = Vec::new();
    collect_original_text_candidates(&provenance, &mut candidates);

    candidates.into_iter().find_map(|text| {
        let qualifiers = semantic_high_salience_qualifiers(&text);
        if !qualifiers.is_empty() || text.chars().any(|ch| ch.is_ascii_uppercase()) {
            Some(qualifiers)
        } else {
            None
        }
    })
}

fn collect_original_text_candidates(value: &Value, candidates: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for key in ORIGINAL_TEXT_KEYS {
                if let Some(text) = map
                    .get(*key)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                {
                    candidates.push(text.to_string());
                }
            }

            for child in map.values() {
                collect_original_text_candidates(child, candidates);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_original_text_candidates(child, candidates);
            }
        }
        _ => {}
    }
}

fn sorted_qualifier_value(qualifiers: &HashSet<String>) -> Value {
    let mut sorted = qualifiers.iter().cloned().collect::<Vec<_>>();
    sorted.sort();
    Value::Array(sorted.into_iter().map(Value::String).collect())
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

fn execute_batch(conn: &Connection, sql: &str, label: &str) -> Result<(), MigrationError> {
    conn.execute_batch(sql).map_err(|e| format!("{label}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_backfills_recoverable_qualifiers_and_marks_unknown_legacy_claims() {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE intelligence_claims (
                id TEXT PRIMARY KEY,
                metadata_json TEXT,
                provenance_json TEXT NOT NULL,
                claim_state TEXT NOT NULL,
                surfacing_state TEXT NOT NULL
            );",
        )
        .expect("create minimal claims table");
        conn.execute(
            "INSERT INTO intelligence_claims
             (id, metadata_json, provenance_json, claim_state, surfacing_state)
             VALUES (?1, NULL, ?2, 'active', 'active')",
            params![
                "recoverable",
                serde_json::json!({
                    "source": {
                        "original_text": "US Phase 2 budget approval is pending with finance"
                    }
                })
                .to_string(),
            ],
        )
        .expect("seed recoverable claim");
        conn.execute(
            "INSERT INTO intelligence_claims
             (id, metadata_json, provenance_json, claim_state, surfacing_state)
             VALUES ('unknown', NULL, '{}', 'active', 'active')",
            [],
        )
        .expect("seed unknown claim");
        conn.execute(
            "INSERT INTO intelligence_claims
             (id, metadata_json, provenance_json, claim_state, surfacing_state)
             VALUES (?1, ?2, '{}', 'active', 'active')",
            params![
                "known",
                serde_json::json!({
                    SEMANTIC_QUALIFIERS_METADATA_KEY: []
                })
                .to_string(),
            ],
        )
        .expect("seed already-known claim");

        migrate_v156_semantic_merge_safety(&conn).expect("migration succeeds");

        let recoverable = metadata_for(&conn, "recoverable");
        assert_eq!(
            recoverable[SEMANTIC_QUALIFIERS_METADATA_KEY],
            serde_json::json!(["region:US"])
        );
        assert!(recoverable
            .get(NON_SEMANTIC_MERGEABLE_METADATA_KEY)
            .is_none());

        let unknown = metadata_for(&conn, "unknown");
        assert_eq!(
            unknown[NON_SEMANTIC_MERGEABLE_METADATA_KEY],
            Value::Bool(true)
        );

        let known = metadata_for(&conn, "known");
        assert_eq!(
            known[SEMANTIC_QUALIFIERS_METADATA_KEY],
            serde_json::json!([])
        );
        assert!(known.get(NON_SEMANTIC_MERGEABLE_METADATA_KEY).is_none());
    }

    fn metadata_for(conn: &Connection, id: &str) -> Value {
        let metadata: String = conn
            .query_row(
                "SELECT metadata_json FROM intelligence_claims WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .expect("read metadata");
        serde_json::from_str(&metadata).expect("parse metadata")
    }
}

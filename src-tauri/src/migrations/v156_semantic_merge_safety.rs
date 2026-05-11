use std::collections::HashSet;

use rusqlite::{params, Connection};
use serde_json::{Map, Value};

use crate::services::claims::semantic_high_salience_qualifiers;

use super::MigrationError;

const CLAIMS_TABLE: &str = "intelligence_claims";
const SEMANTIC_QUALIFIERS_METADATA_KEY: &str = "semantic_qualifiers";
const NON_SEMANTIC_MERGEABLE_METADATA_KEY: &str = "non_semantic_mergeable";
const LEGACY_SEMANTIC_QUALIFIERS_METADATA_KEY: &str = "dos280_semantic_qualifiers";
const LEGACY_NON_SEMANTIC_MERGEABLE_METADATA_KEY: &str = "dos280_non_semantic_mergeable";
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
    text: String,
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
        let Some(metadata_json) = migrated_metadata_json(
            row.metadata_json.as_deref(),
            &row.provenance_json,
            &row.text,
        )?
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
            "SELECT id, text, metadata_json, provenance_json
             FROM intelligence_claims
             WHERE claim_state = 'active'
               AND surfacing_state = 'active'",
        )
        .map_err(|e| format!("prepare legacy claim scan: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(LegacyClaimMetadata {
                id: row.get(0)?,
                text: row.get(1)?,
                metadata_json: row.get(2)?,
                provenance_json: row.get(3)?,
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
    canonical_text: &str,
) -> Result<Option<String>, MigrationError> {
    let mut metadata = match metadata_json {
        Some(raw) => match serde_json::from_str::<Value>(raw) {
            Ok(Value::Object(map)) => map,
            Ok(_) | Err(_) => return Ok(None),
        },
        None => Map::new(),
    };

    if [
        SEMANTIC_QUALIFIERS_METADATA_KEY,
        LEGACY_SEMANTIC_QUALIFIERS_METADATA_KEY,
    ]
    .iter()
    .any(|key| metadata.contains_key(*key))
        || [
            NON_SEMANTIC_MERGEABLE_METADATA_KEY,
            LEGACY_NON_SEMANTIC_MERGEABLE_METADATA_KEY,
        ]
        .iter()
        .any(|key| metadata.get(*key).and_then(Value::as_bool).unwrap_or(false))
    {
        return Ok(None);
    }

    if let Some(qualifiers) = recover_qualifiers_from_provenance(provenance_json, canonical_text) {
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

fn recover_qualifiers_from_provenance(
    provenance_json: &str,
    canonical_text: &str,
) -> Option<HashSet<String>> {
    let provenance = serde_json::from_str::<Value>(provenance_json).ok()?;
    let mut candidates = Vec::new();
    collect_original_text_candidates(&provenance, &mut candidates);
    if !candidate_numeric_scopes_match_canonical(&candidates, canonical_text) {
        return None;
    }

    let mut recovered = HashSet::new();
    let mut all_candidates_confident_empty = !candidates.is_empty();
    for text in candidates {
        let qualifiers = semantic_high_salience_qualifiers(&text);
        if qualifiers.is_empty() {
            all_candidates_confident_empty &=
                provenance_text_candidate_confidently_preserves_case(&text);
        } else {
            recovered.extend(qualifiers);
        }
    }

    if !recovered.is_empty() {
        Some(recovered)
    } else if all_candidates_confident_empty {
        Some(HashSet::new())
    } else {
        None
    }
}

fn candidate_numeric_scopes_match_canonical(candidates: &[String], canonical_text: &str) -> bool {
    let canonical_numeric_scopes = semantic_numeric_scopes(canonical_text);
    candidates
        .iter()
        .all(|candidate| semantic_numeric_scopes(candidate) == canonical_numeric_scopes)
}

fn semantic_numeric_scopes(text: &str) -> HashSet<String> {
    let mut normalized = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
        } else {
            normalized.push(' ');
        }
    }

    normalized
        .split_whitespace()
        .filter(|token| token.chars().all(|ch| ch.is_ascii_digit()))
        .map(ToString::to_string)
        .collect()
}

fn provenance_text_candidate_confidently_preserves_case(text: &str) -> bool {
    text.chars().any(|ch| ch.is_ascii_uppercase())
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
                text TEXT NOT NULL,
                metadata_json TEXT,
                provenance_json TEXT NOT NULL,
                claim_state TEXT NOT NULL,
                surfacing_state TEXT NOT NULL
            );",
        )
        .expect("create minimal claims table");
        seed_active_claim(
            &conn,
            "recoverable",
            "US Phase 2 budget approval is pending with finance",
            None,
            serde_json::json!({
                "source": {
                    "original_text": "US Phase 2 budget approval is pending with finance"
                }
            }),
        );
        seed_active_claim(
            &conn,
            "nested-scoped",
            "US Phase 2 budget approval is pending with finance",
            None,
            serde_json::json!({
                "events": [
                    {
                        "parent": {
                            "text": "Phase 2 budget approval is pending with finance"
                        }
                    },
                    {
                        "child": {
                            "original_text": "US Phase 2 budget approval is pending with finance"
                        }
                    }
                ]
            }),
        );
        seed_active_claim(
            &conn,
            "ambiguous-empty",
            "Phase 2 budget approval is pending with finance",
            None,
            serde_json::json!({
                "events": [
                    {
                        "parent": {
                            "text": "Phase 2 budget approval is pending with finance"
                        }
                    },
                    {
                        "child": {
                            "original_text": "phase 2 budget approval is pending with finance"
                        }
                    }
                ]
            }),
        );
        seed_active_claim(
            &conn,
            "mixed-numeric-empty",
            "Budget approval is pending with finance",
            None,
            serde_json::json!({
                "events": [
                    {
                        "parent": {
                            "text": "Phase 2 budget approval is pending with finance"
                        }
                    },
                    {
                        "child": {
                            "original_text": "Budget approval is pending with finance"
                        }
                    }
                ]
            }),
        );
        seed_active_claim(
            &conn,
            "canonical-numeric-mismatch",
            "Phase 2 budget approval is pending with finance",
            None,
            serde_json::json!({
                "source": {
                    "original_text": "Budget approval is pending with finance"
                }
            }),
        );
        seed_active_claim(
            &conn,
            "unknown",
            "Unknown claim",
            None,
            serde_json::json!({}),
        );
        seed_active_claim(
            &conn,
            "known",
            "Known claim",
            Some(serde_json::json!({
                SEMANTIC_QUALIFIERS_METADATA_KEY: []
            })),
            serde_json::json!({}),
        );
        seed_active_claim(
            &conn,
            "legacy-known",
            "Legacy known claim",
            Some(serde_json::json!({
                LEGACY_SEMANTIC_QUALIFIERS_METADATA_KEY: ["region:US"]
            })),
            serde_json::json!({}),
        );

        migrate_v156_semantic_merge_safety(&conn).expect("migration succeeds");

        let recoverable = metadata_for(&conn, "recoverable");
        assert_eq!(
            recoverable[SEMANTIC_QUALIFIERS_METADATA_KEY],
            serde_json::json!(["region:US"])
        );
        assert!(recoverable
            .get(NON_SEMANTIC_MERGEABLE_METADATA_KEY)
            .is_none());

        let nested_scoped = metadata_for(&conn, "nested-scoped");
        assert_eq!(
            nested_scoped[SEMANTIC_QUALIFIERS_METADATA_KEY],
            serde_json::json!(["region:US"])
        );
        assert!(nested_scoped
            .get(NON_SEMANTIC_MERGEABLE_METADATA_KEY)
            .is_none());

        let ambiguous_empty = metadata_for(&conn, "ambiguous-empty");
        assert_eq!(
            ambiguous_empty[NON_SEMANTIC_MERGEABLE_METADATA_KEY],
            Value::Bool(true)
        );

        let mixed_numeric_empty = metadata_for(&conn, "mixed-numeric-empty");
        assert_eq!(
            mixed_numeric_empty[NON_SEMANTIC_MERGEABLE_METADATA_KEY],
            Value::Bool(true)
        );
        assert!(mixed_numeric_empty
            .get(SEMANTIC_QUALIFIERS_METADATA_KEY)
            .is_none());

        let canonical_numeric_mismatch = metadata_for(&conn, "canonical-numeric-mismatch");
        assert_eq!(
            canonical_numeric_mismatch[NON_SEMANTIC_MERGEABLE_METADATA_KEY],
            Value::Bool(true)
        );
        assert!(canonical_numeric_mismatch
            .get(SEMANTIC_QUALIFIERS_METADATA_KEY)
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

        let legacy_known = metadata_for(&conn, "legacy-known");
        assert_eq!(
            legacy_known[LEGACY_SEMANTIC_QUALIFIERS_METADATA_KEY],
            serde_json::json!(["region:US"])
        );
        assert!(legacy_known.get(SEMANTIC_QUALIFIERS_METADATA_KEY).is_none());
        assert!(legacy_known
            .get(NON_SEMANTIC_MERGEABLE_METADATA_KEY)
            .is_none());
    }

    fn seed_active_claim(
        conn: &Connection,
        id: &str,
        text: &str,
        metadata: Option<Value>,
        provenance: Value,
    ) {
        let metadata_json = metadata.map(|value| value.to_string());
        conn.execute(
            "INSERT INTO intelligence_claims
             (id, text, metadata_json, provenance_json, claim_state, surfacing_state)
             VALUES (?1, ?2, ?3, ?4, 'active', 'active')",
            params![id, text, metadata_json, provenance.to_string()],
        )
        .expect("seed active claim");
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

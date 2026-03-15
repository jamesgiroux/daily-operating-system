//! ADR-0098 data lifecycle primitives.
//!
//! Source-aware purge infrastructure used when connector credentials are revoked.

use std::collections::HashMap;

use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::{ActionDb, DbError};
use crate::db::people::FieldSource;

/// Provenance source for persisted data with purge semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    User,
    Clay,
    Glean,
    Gravatar,
    Google,
    Ai,
}

impl DataSource {
    pub fn as_str(self) -> &'static str {
        match self {
            DataSource::User => "user",
            DataSource::Clay => "clay",
            DataSource::Glean => "glean",
            DataSource::Gravatar => "gravatar",
            DataSource::Google => "google",
            DataSource::Ai => "ai",
        }
    }
}

/// Purge result counts for audit logging.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PurgeReport {
    pub source: String,
    pub people_cleared: usize,
    pub signals_deleted: usize,
    pub relationships_deleted: usize,
    pub enrichment_sources_cleared: usize,
    #[serde(default)]
    pub assessments_cleared: usize,
    #[serde(default)]
    pub emails_deleted: usize,
    #[serde(default)]
    pub email_signals_deleted: usize,
    #[serde(default)]
    pub caches_deleted: usize,
}

fn is_profile_field(field: &str) -> bool {
    matches!(
        field,
        "linkedin_url"
            | "twitter_handle"
            | "phone"
            | "photo_url"
            | "bio"
            | "title_history"
            | "organization"
            | "role"
            | "company_industry"
            | "company_size"
            | "company_hq"
    )
}

fn purge_people_profile_fields_by_source(
    db: &ActionDb,
    source: DataSource,
) -> Result<usize, rusqlite::Error> {
    let source_str = source.as_str();
    let mut stmt = db.conn_ref().prepare(
        "SELECT id, enrichment_sources
         FROM people
         WHERE enrichment_sources IS NOT NULL
           AND enrichment_sources != ''",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut fields_cleared = 0usize;
    for row in rows {
        let (person_id, enrichment_sources_json) = row?;
        let mut sources: HashMap<String, FieldSource> =
            match serde_json::from_str(&enrichment_sources_json) {
                Ok(parsed) => parsed,
                Err(_) => continue,
            };

        let fields_to_clear: Vec<String> = sources
            .iter()
            .filter_map(|(field, meta)| {
                if meta.source == source_str && is_profile_field(field) {
                    Some(field.clone())
                } else {
                    None
                }
            })
            .collect();

        if fields_to_clear.is_empty() {
            continue;
        }

        fields_cleared += fields_to_clear.len();
        for field in &fields_to_clear {
            sources.remove(field);
        }

        let mut set_clauses: Vec<String> = fields_to_clear
            .iter()
            .filter(|field| is_profile_field(field))
            .map(|field| format!("{field} = NULL"))
            .collect();

        set_clauses.push("enrichment_sources = ?1".to_string());
        set_clauses.push("updated_at = ?2".to_string());

        let sql = format!("UPDATE people SET {} WHERE id = ?3", set_clauses.join(", "));
        let new_sources = if sources.is_empty() {
            None
        } else {
            serde_json::to_string(&sources).ok()
        };

        db.conn_ref().execute(
            &sql,
            params![new_sources, Utc::now().to_rfc3339(), person_id],
        )?;
    }

    Ok(fields_cleared)
}

fn table_exists(db: &ActionDb, table: &str) -> bool {
    db.conn_ref()
        .query_row(
            "SELECT EXISTS(
                SELECT 1
                FROM sqlite_master
                WHERE type = 'table' AND name = ?1
            )",
            [table],
            |row| row.get::<_, i64>(0),
        )
        .map(|exists| exists == 1)
        .unwrap_or(false)
}

/// Purge data originating from a revoked source.
///
/// Notes:
/// - User-entered data (`DataSource::User`) is never purged.
pub fn purge_source(db: &ActionDb, source: DataSource) -> Result<PurgeReport, DbError> {
    if source == DataSource::User {
        return Ok(PurgeReport {
            source: source.as_str().to_string(),
            ..Default::default()
        });
    }

    db.with_transaction(|tx| {
        let source_str = source.as_str();

        let people_cleared = tx
            .conn_ref()
            .execute(
                "DELETE FROM account_stakeholders WHERE data_source = ?1",
                [source_str],
            )
            .map_err(|e| format!("purge account_stakeholders failed: {e}"))?;

        let signals_deleted = if source == DataSource::Glean {
            tx.conn_ref()
                .execute(
                    "DELETE FROM signal_events
                     WHERE source IN (
                        'glean',
                        'glean_search',
                        'glean_org',
                        'glean_crm',
                        'glean_zendesk',
                        'glean_gong',
                        'glean_chat',
                        'glean_synthesis'
                     )",
                    [],
                )
                .map_err(|e| format!("purge signal_events failed: {e}"))?
        } else {
            tx.conn_ref()
                .execute("DELETE FROM signal_events WHERE source = ?1", [source_str])
                .map_err(|e| format!("purge signal_events failed: {e}"))?
        };

        let relationships_deleted = tx
            .conn_ref()
            .execute("DELETE FROM person_relationships WHERE source = ?1", [source_str])
            .map_err(|e| format!("purge person_relationships failed: {e}"))?;

        let mut assessments_cleared = 0usize;
        let mut emails_deleted = 0usize;
        let mut email_signals_deleted = 0usize;
        let mut caches_deleted = 0usize;

        match source {
            DataSource::Glean => {
                if table_exists(tx, "entity_assessment") {
                    assessments_cleared = tx
                        .conn_ref()
                        .execute(
                            "UPDATE entity_assessment
                             SET org_health_json = NULL,
                                 executive_assessment = NULL,
                                 risks_json = NULL,
                                 stakeholder_insights_json = NULL,
                                 current_state_json = NULL,
                                 company_context_json = NULL,
                                 dimensions_json = NULL
                             WHERE source_manifest_json LIKE '%glean%' OR org_health_json IS NOT NULL",
                            [],
                        )
                        .map_err(|e| format!("purge entity_assessment failed: {e}"))?;
                }

                if table_exists(tx, "glean_document_cache") {
                    caches_deleted = tx
                        .conn_ref()
                        .execute("DELETE FROM glean_document_cache", [])
                        .map_err(|e| format!("purge glean_document_cache failed: {e}"))?;
                }
            }
            DataSource::Google => {
                if table_exists(tx, "emails") {
                    emails_deleted = tx
                        .conn_ref()
                        .execute("DELETE FROM emails", [])
                        .map_err(|e| format!("purge emails failed: {e}"))?;
                }
                if table_exists(tx, "email_signals") {
                    email_signals_deleted = tx
                        .conn_ref()
                        .execute("DELETE FROM email_signals", [])
                        .map_err(|e| format!("purge email_signals failed: {e}"))?;
                }
                if table_exists(tx, "email_threads") {
                    let _ = tx.conn_ref().execute("DELETE FROM email_threads", []);
                }
                if table_exists(tx, "meetings") {
                    let _ = tx
                        .conn_ref()
                        .execute("UPDATE meetings SET description = NULL WHERE description IS NOT NULL", []);
                }
            }
            DataSource::Gravatar => {
                if table_exists(tx, "gravatar_cache") {
                    caches_deleted = tx
                        .conn_ref()
                        .execute("DELETE FROM gravatar_cache", [])
                        .map_err(|e| format!("purge gravatar_cache failed: {e}"))?;
                }
            }
            _ => {}
        }

        let enrichment_sources_cleared = purge_people_profile_fields_by_source(tx, source)
            .map_err(|e| format!("purge people enrichment_sources failed: {e}"))?;

        Ok(PurgeReport {
            source: source_str.to_string(),
            people_cleared,
            signals_deleted,
            relationships_deleted,
            enrichment_sources_cleared,
            assessments_cleared,
            emails_deleted,
            email_signals_deleted,
            caches_deleted,
        })
    })
    .map_err(DbError::Migration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::db::DbPerson;

    fn seed_person(db: &ActionDb, id: &str, enrichment_sources: Option<String>) {
        let person = DbPerson {
            id: id.to_string(),
            email: format!("{id}@example.com"),
            name: id.to_string(),
            organization: Some("Acme".to_string()),
            role: Some("Manager".to_string()),
            relationship: "external".to_string(),
            notes: None,
            tracker_path: None,
            last_seen: None,
            first_seen: None,
            meeting_count: 0,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            linkedin_url: Some("https://linkedin.com/in/example".to_string()),
            twitter_handle: None,
            phone: None,
            photo_url: None,
            bio: Some("Profile".to_string()),
            title_history: None,
            company_industry: None,
            company_size: None,
            company_hq: None,
            last_enriched_at: None,
            enrichment_sources: enrichment_sources.clone(),
        };
        db.upsert_person(&person).expect("seed person");
        if let Some(enrichment_sources) = enrichment_sources {
            db.conn_ref()
                .execute(
                    "UPDATE people
                     SET linkedin_url = ?1, bio = ?2, enrichment_sources = ?3
                     WHERE id = ?4",
                    params![
                        "https://linkedin.com/in/example",
                        "Profile",
                        enrichment_sources,
                        id
                    ],
                )
                .expect("seed enrichment_sources");
        }
    }

    #[test]
    fn purge_source_removes_tagged_rows_and_preserves_user_rows() {
        let db = test_db();
        seed_person(&db, "p1", None);

        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, role, data_source)
                 VALUES ('a1', 'p1', 'champion', 'glean')",
                [],
            )
            .expect("seed glean stakeholder");
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, role, data_source)
                 VALUES ('a2', 'p1', 'champion', 'user')",
                [],
            )
            .expect("seed user stakeholder");

        db.conn_ref()
            .execute(
                "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, source, confidence)
                 VALUES ('s-glean', 'account', 'a1', 'profile_update', 'glean', 0.8)",
                [],
            )
            .expect("seed glean signal");
        db.conn_ref()
            .execute(
                "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, source, confidence)
                 VALUES ('s-user', 'account', 'a1', 'profile_update', 'user', 0.8)",
                [],
            )
            .expect("seed user signal");

        db.conn_ref()
            .execute(
                "INSERT INTO person_relationships
                 (id, from_person_id, to_person_id, relationship_type, direction, confidence, source)
                 VALUES ('r-glean', 'p1', 'p1', 'peer', 'symmetric', 0.8, 'glean')",
                [],
            )
            .expect("seed glean relationship");

        let report = purge_source(&db, DataSource::Glean).expect("purge");
        assert_eq!(report.people_cleared, 1);
        assert_eq!(report.signals_deleted, 1);
        assert_eq!(report.relationships_deleted, 1);

        let remaining_user_stakeholders: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE data_source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user stakeholders");
        assert_eq!(remaining_user_stakeholders, 1);

        let remaining_user_signals: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user signals");
        assert_eq!(remaining_user_signals, 1);
    }

    #[test]
    fn purge_source_clears_profile_fields_by_provenance() {
        let db = test_db();
        seed_person(
            &db,
            "p2",
            Some(
                serde_json::json!({
                    "linkedin_url": {"source": "clay", "at": "2026-03-07T00:00:00Z"},
                    "bio": {"source": "user", "at": "2026-03-07T00:00:00Z"}
                })
                .to_string(),
            ),
        );

        let report = purge_source(&db, DataSource::Clay).expect("purge clay");
        assert_eq!(report.enrichment_sources_cleared, 1);

        let (linkedin, bio, sources): (Option<String>, Option<String>, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT linkedin_url, bio, enrichment_sources FROM people WHERE id = 'p2'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("person after purge");

        assert!(
            linkedin.is_none(),
            "clay-sourced linkedin should be cleared"
        );
        assert_eq!(
            bio.as_deref(),
            Some("Profile"),
            "user-sourced bio should remain"
        );

        let sources_json = sources.expect("remaining enrichment sources");
        assert!(
            !sources_json.contains("linkedin_url"),
            "clay field provenance should be removed"
        );
        assert!(
            sources_json.contains("\"bio\""),
            "non-purged provenance should remain"
        );
    }
}

//! ADR-0098 data lifecycle primitives.
//!
//! Source-aware purge infrastructure used when connector credentials are revoked.
//! Also provides DB growth monitoring and age-based purge (I614).

use std::collections::HashMap;

use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::{ActionDb, DbError};
use crate::db::people::FieldSource;

// =============================================================================
// I614: DB growth monitoring
// =============================================================================

/// Row count for a single monitored table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRowCount {
    pub table_name: String,
    pub row_count: i64,
}

/// DB growth report: file size + row counts for key tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbGrowthReport {
    /// DB file size in bytes.
    pub file_size_bytes: u64,
    /// Human-readable file size (e.g. "152 MB").
    pub file_size_display: String,
    /// Row counts for monitored tables.
    pub table_counts: Vec<TableRowCount>,
    /// Timestamp of this report.
    pub reported_at: String,
}

/// Tables to monitor for growth.
const MONITORED_TABLES: &[&str] = &[
    "signal_events",
    "email_signals",
    "emails",
    "entity_assessment",
    "captured_commitments",
    "content_embeddings",
    "person_relationships",
    "meetings",
];

fn format_file_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

/// Get DB file size and row counts for key tables.
pub fn db_growth_report(db: &ActionDb) -> DbGrowthReport {
    let file_size_bytes = ActionDb::db_path_public()
        .ok()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .unwrap_or(0);

    let table_counts: Vec<TableRowCount> = MONITORED_TABLES
        .iter()
        .filter_map(|table| {
            if !table_exists(db, table) {
                return None;
            }
            // Table name is from a static list, not user input -- safe to interpolate.
            let sql = format!("SELECT COUNT(*) FROM {table}");
            let count = db
                .conn_ref()
                .query_row(&sql, [], |row| row.get::<_, i64>(0))
                .unwrap_or(0);
            Some(TableRowCount {
                table_name: table.to_string(),
                row_count: count,
            })
        })
        .collect();

    DbGrowthReport {
        file_size_bytes,
        file_size_display: format_file_size(file_size_bytes),
        table_counts,
        reported_at: Utc::now().to_rfc3339(),
    }
}

/// Log DB size at startup. Warn at 300MB, error at 500MB.
/// Returns the size in bytes for further action (e.g. Tauri event at 500MB+).
pub fn log_db_size_at_startup() -> u64 {
    let size = ActionDb::db_path_public()
        .ok()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .unwrap_or(0);

    let display = format_file_size(size);

    if size >= 500_000_000 {
        log::error!(
            "DB size: {} -- exceeds 500MB threshold, consider purging",
            display
        );
    } else if size >= 300_000_000 {
        log::warn!("DB size: {} -- approaching 500MB threshold", display);
    } else {
        log::info!("DB size: {}", display);
    }

    size
}

// =============================================================================
// I614: Age-based purge
// =============================================================================

/// Result of an age-based purge run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgePurgeReport {
    pub signals_purged: usize,
    pub email_signals_purged: usize,
    pub emails_purged: usize,
    pub embeddings_purged: usize,
}

impl AgePurgeReport {
    pub fn total(&self) -> usize {
        self.signals_purged
            + self.email_signals_purged
            + self.emails_purged
            + self.embeddings_purged
    }
}

/// Purge signal_events older than `days`, preserving user corrections.
///
/// User corrections (source = 'user_correction') are never purged regardless
/// of age because they represent explicit user intent with confidence 1.0.
pub fn purge_aged_signals(db: &ActionDb, days: i64) -> Result<usize, DbError> {
    if !table_exists(db, "signal_events") {
        return Ok(0);
    }
    db.conn_ref()
        .execute(
            "DELETE FROM signal_events
             WHERE created_at < datetime('now', ?1)
               AND source != 'user_correction'",
            params![format!("-{days} days")],
        )
        .map_err(|e| DbError::Migration(format!("purge_aged_signals: {e}")))
}

/// Purge deactivated email_signals older than `days`.
pub fn purge_aged_email_signals(db: &ActionDb, days: i64) -> Result<usize, DbError> {
    if !table_exists(db, "email_signals") {
        return Ok(0);
    }
    db.conn_ref()
        .execute(
            "DELETE FROM email_signals
             WHERE deactivated_at IS NOT NULL
               AND deactivated_at < datetime('now', ?1)",
            params![format!("-{days} days")],
        )
        .map_err(|e| DbError::Migration(format!("purge_aged_email_signals: {e}")))
}

/// Purge resolved emails older than `days`.
///
/// L2 cycle-4 fix #2: Email-subject `intelligence_claims` rows for the
/// purged email IDs are transitioned to `claim_state='withdrawn'` so
/// (a) PRE-GATE no longer blocks future re-imports of the same
/// email_id (PRE-GATE only matches `tombstoned`), and (b) the audit
/// trail row survives — claim `text`, `subject_ref`, and assertion
/// columns remain frozen per the immutability allowlist.
///
/// L2 cycle-5 fix #3: the cascade UPDATE and the email DELETE run in
/// a single SQLite transaction (`BEGIN IMMEDIATE` / `COMMIT`) so a
/// crash or DELETE failure between them no longer leaves still-present
/// resolved emails with their claims already withdrawn.
pub fn purge_aged_emails(db: &ActionDb, days: i64) -> Result<usize, DbError> {
    if !table_exists(db, "emails") {
        return Ok(0);
    }
    let cutoff = format!("-{days} days");
    let claims_table_present = table_exists(db, "intelligence_claims");
    let conn = db.conn_ref();

    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| DbError::Migration(format!("purge_aged_emails BEGIN: {e}")))?;

    let result = (|| -> Result<usize, DbError> {
        // Withdraw Email-subject claims for emails we are about to
        // purge. Run BEFORE the DELETE so we can still join against
        // `emails` to identify which email_ids are aging out.
        if claims_table_present {
            conn.execute(
                "UPDATE intelligence_claims \
                 SET claim_state = 'withdrawn', \
                     retraction_reason = coalesce(retraction_reason, 'subject_purged') \
                 WHERE lower(json_extract(subject_ref, '$.kind')) = 'email' \
                   AND claim_state IN ('active', 'tombstoned', 'dormant') \
                   AND json_valid(subject_ref) = 1 \
                   AND json_extract(subject_ref, '$.id') IN ( \
                       SELECT email_id FROM emails \
                       WHERE resolved_at IS NOT NULL \
                         AND resolved_at < datetime('now', ?1) \
                   )",
                params![cutoff],
            )
            .map_err(|e| {
                DbError::Migration(format!("purge_aged_emails: withdraw email claims: {e}"))
            })?;
        }

        conn.execute(
            "DELETE FROM emails
             WHERE resolved_at IS NOT NULL
               AND resolved_at < datetime('now', ?1)",
            params![cutoff],
        )
        .map_err(|e| DbError::Migration(format!("purge_aged_emails: {e}")))
    })();

    match result {
        Ok(deleted) => {
            conn.execute_batch("COMMIT")
                .map_err(|e| DbError::Migration(format!("purge_aged_emails COMMIT: {e}")))?;
            Ok(deleted)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

/// Purge content_embeddings for content_files that no longer exist.
pub fn purge_orphaned_embeddings(db: &ActionDb) -> Result<usize, DbError> {
    if !table_exists(db, "content_embeddings") || !table_exists(db, "content_index") {
        return Ok(0);
    }
    db.conn_ref()
        .execute(
            "DELETE FROM content_embeddings
             WHERE content_file_id NOT IN (SELECT id FROM content_index)",
            [],
        )
        .map_err(|e| DbError::Migration(format!("purge_orphaned_embeddings: {e}")))
}

/// Run all age-based purge operations. Returns a report of what was purged.
///
/// Retention defaults (from I614 spec):
/// - signal_events: 180 days (user corrections preserved)
/// - email_signals (deactivated): 30 days
/// - emails (resolved): 60 days
/// - content_embeddings: orphans only
pub fn run_age_based_purge(db: &ActionDb) -> AgePurgeReport {
    let signals_purged = purge_aged_signals(db, 180).unwrap_or_else(|e| {
        log::warn!("Age purge: signal_events failed: {e}");
        0
    });

    let email_signals_purged = purge_aged_email_signals(db, 30).unwrap_or_else(|e| {
        log::warn!("Age purge: email_signals failed: {e}");
        0
    });

    let emails_purged = purge_aged_emails(db, 60).unwrap_or_else(|e| {
        log::warn!("Age purge: emails failed: {e}");
        0
    });

    let embeddings_purged = purge_orphaned_embeddings(db).unwrap_or_else(|e| {
        log::warn!("Age purge: content_embeddings failed: {e}");
        0
    });

    AgePurgeReport {
        signals_purged,
        email_signals_purged,
        emails_purged,
        embeddings_purged,
    }
}

/// Check DB file size and return bytes. Used by the hygiene loop to
/// decide whether to emit a `db-size-warning` Tauri event.
pub fn db_file_size_bytes() -> u64 {
    ActionDb::db_path_public()
        .ok()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .unwrap_or(0)
}

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

        tx.conn_ref()
            .execute(
                "DELETE FROM account_stakeholder_roles WHERE data_source = ?1",
                [source_str],
            )
            .map_err(|e| format!("purge account_stakeholder_roles failed: {e}"))?;
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
                    // L2 cycle-4 fix #2 + cycle-5 fix #2: withdraw
                    // Email-subject claims BEFORE deleting the source
                    // emails so the join can still identify which
                    // email_ids existed. The claim rows survive (audit
                    // trail) but are no longer matched by PRE-GATE /
                    // suppression.
                    //
                    // Cycle-5 fix #2: cascade errors propagate via `?`
                    // so the surrounding `with_transaction` rolls back
                    // the email DELETE if the cascade UPDATE fails
                    // (e.g. malformed historical subject_ref JSON).
                    // The previous `let _ = ...` swallowed errors and
                    // committed stale Email tombstones after the
                    // source rows were gone.
                    if table_exists(tx, "intelligence_claims") {
                        tx.conn_ref().execute(
                            "UPDATE intelligence_claims \
                             SET claim_state = 'withdrawn', \
                                 retraction_reason = coalesce(retraction_reason, 'subject_purged') \
                             WHERE lower(json_extract(subject_ref, '$.kind')) = 'email' \
                               AND claim_state IN ('active', 'tombstoned', 'dormant') \
                               AND json_valid(subject_ref) = 1 \
                               AND json_extract(subject_ref, '$.id') IN \
                                   (SELECT email_id FROM emails)",
                            [],
                        )
                        .map_err(|e| format!("withdraw email claims before purge failed: {e}"))?;
                    }
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
                "INSERT INTO account_stakeholders (account_id, person_id, data_source)
                 VALUES ('a1', 'p1', 'glean')",
                [],
            )
            .expect("seed glean stakeholder");
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source)
                 VALUES ('a1', 'p1', 'champion', 'glean')",
                [],
            )
            .expect("seed glean stakeholder role");
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, data_source)
                 VALUES ('a2', 'p1', 'user')",
                [],
            )
            .expect("seed user stakeholder");
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source)
                 VALUES ('a2', 'p1', 'champion', 'user')",
                [],
            )
            .expect("seed user stakeholder role");

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

    // --- I614: Age-based purge tests ---

    #[test]
    fn purge_aged_signals_preserves_user_corrections() {
        let db = test_db();

        // Insert an old signal (200 days ago) from a regular source
        db.conn_ref()
            .execute(
                "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, source, confidence, created_at)
                 VALUES ('s-old', 'account', 'a1', 'profile_update', 'email_enrichment', 0.7,
                         datetime('now', '-200 days'))",
                [],
            )
            .expect("seed old signal");

        // Insert an old user_correction (200 days ago) -- must be preserved
        db.conn_ref()
            .execute(
                "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, source, confidence, created_at)
                 VALUES ('s-correction', 'account', 'a1', 'entity_correction', 'user_correction', 1.0,
                         datetime('now', '-200 days'))",
                [],
            )
            .expect("seed old user correction");

        // Insert a recent signal (10 days ago) -- must be preserved
        db.conn_ref()
            .execute(
                "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, source, confidence, created_at)
                 VALUES ('s-recent', 'account', 'a1', 'profile_update', 'email_enrichment', 0.7,
                         datetime('now', '-10 days'))",
                [],
            )
            .expect("seed recent signal");

        let purged = purge_aged_signals(&db, 180).expect("purge");
        assert_eq!(
            purged, 1,
            "only the old non-correction signal should be purged"
        );

        // Verify user_correction is still there
        let correction_exists: bool = db
            .conn_ref()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM signal_events WHERE id = 's-correction')",
                [],
                |row| row.get(0),
            )
            .expect("check correction");
        assert!(correction_exists, "user corrections must never be purged");

        // Verify recent signal is still there
        let recent_exists: bool = db
            .conn_ref()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM signal_events WHERE id = 's-recent')",
                [],
                |row| row.get(0),
            )
            .expect("check recent");
        assert!(recent_exists, "recent signals must not be purged");

        // Verify old signal is gone
        let old_exists: bool = db
            .conn_ref()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM signal_events WHERE id = 's-old')",
                [],
                |row| row.get(0),
            )
            .expect("check old");
        assert!(!old_exists, "old non-correction signal should be purged");
    }

    #[test]
    fn purge_aged_email_signals_only_removes_deactivated() {
        let db = test_db();

        // Insert a deactivated email signal from 60 days ago
        db.conn_ref()
            .execute(
                "INSERT INTO email_signals (email_id, entity_id, entity_type, signal_type, signal_text, detected_at, deactivated_at)
                 VALUES ('e1', 'a1', 'account', 'risk', 'test', datetime('now', '-60 days'), datetime('now', '-60 days'))",
                [],
            )
            .expect("seed old deactivated");

        // Insert an active email signal from 60 days ago -- must be preserved
        db.conn_ref()
            .execute(
                "INSERT INTO email_signals (email_id, entity_id, entity_type, signal_type, signal_text, detected_at)
                 VALUES ('e2', 'a1', 'account', 'win', 'test', datetime('now', '-60 days'))",
                [],
            )
            .expect("seed old active");

        let purged = purge_aged_email_signals(&db, 30).expect("purge");
        assert_eq!(purged, 1, "only the deactivated signal should be purged");

        let remaining: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM email_signals", [], |row| row.get(0))
            .expect("count remaining");
        assert_eq!(remaining, 1, "active signal must remain");
    }

    #[test]
    fn purge_aged_emails_only_removes_resolved() {
        let db = test_db();

        // Insert a resolved email from 90 days ago
        db.conn_ref()
            .execute(
                "INSERT INTO emails (email_id, resolved_at, received_at)
                 VALUES ('em1', datetime('now', '-90 days'), datetime('now', '-90 days'))",
                [],
            )
            .expect("seed old resolved");

        // Insert an unresolved email from 90 days ago -- must be preserved
        db.conn_ref()
            .execute(
                "INSERT INTO emails (email_id, received_at)
                 VALUES ('em2', datetime('now', '-90 days'))",
                [],
            )
            .expect("seed old unresolved");

        let purged = purge_aged_emails(&db, 60).expect("purge");
        assert_eq!(purged, 1, "only the old resolved email should be purged");

        let remaining: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM emails", [], |row| row.get(0))
            .expect("count remaining");
        assert_eq!(remaining, 1, "unresolved email must remain");
    }

    /// L2 cycle-4 fix #2: aged-email purge must transition Email-subject
    /// `intelligence_claims` rows for the purged email_ids to
    /// `claim_state='withdrawn'` so the substrate doesn't carry stale
    /// suppression for a re-imported email_id. Claim rows survive
    /// (audit trail); their assertion columns remain frozen.
    #[test]
    fn purge_aged_emails_withdraws_email_subject_claim_rows() {
        let db = test_db();

        // Seed two emails, only em1 resolved+aged → eligible for purge.
        db.conn_ref()
            .execute(
                "INSERT INTO emails (email_id, resolved_at, received_at) \
                 VALUES ('em1', datetime('now', '-90 days'), datetime('now', '-90 days'))",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO emails (email_id, received_at) \
                 VALUES ('em2', datetime('now', '-90 days'))",
                [],
            )
            .unwrap();

        // Seed an Email-subject tombstone claim for each email.
        // dos7-allowed: cycle-4 fix #2 cascade test seed
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims \
                 (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
                  actor, data_source, observed_at, created_at, provenance_json, \
                  claim_state, surfacing_state, retraction_reason, temporal_scope, sensitivity) \
                 VALUES \
                 ('claim-em1', '{\"kind\":\"Email\",\"id\":\"em1\"}', 'email_dismissed', \
                  'commitment', 'reply by friday', 'k1', 'h1', 'user', 'user_dismissal', \
                  '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', '{}', \
                  'tombstoned', 'dormant', 'user_removal', 'state', 'internal'), \
                 ('claim-em2', '{\"kind\":\"Email\",\"id\":\"em2\"}', 'email_dismissed', \
                  'question', 'is this still happening', 'k2', 'h2', 'user', 'user_dismissal', \
                  '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', '{}', \
                  'tombstoned', 'dormant', 'user_removal', 'state', 'internal')",
                [],
            )
            .unwrap();

        purge_aged_emails(&db, 60).expect("purge");

        // em1's claim → withdrawn (its email got purged).
        let em1_state: String = db
            .conn_ref()
            .query_row(
                "SELECT claim_state FROM intelligence_claims WHERE id = 'claim-em1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            em1_state, "withdrawn",
            "claim for the purged email must be withdrawn so PRE-GATE no longer matches"
        );

        // em2's claim → unchanged (its email wasn't purged).
        let em2_state: String = db
            .conn_ref()
            .query_row(
                "SELECT claim_state FROM intelligence_claims WHERE id = 'claim-em2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            em2_state, "tombstoned",
            "claim for the surviving email must remain tombstoned"
        );
    }

    #[test]
    fn db_growth_report_returns_table_counts() {
        let db = test_db();

        // Insert some signals
        db.conn_ref()
            .execute(
                "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, source, confidence)
                 VALUES ('s1', 'account', 'a1', 'test', 'test', 0.5)",
                [],
            )
            .expect("seed signal");

        let report = db_growth_report(&db);
        // Should have entries for existing tables
        assert!(!report.table_counts.is_empty(), "should have table counts");

        // signal_events should show 1 row
        let signal_count = report
            .table_counts
            .iter()
            .find(|tc| tc.table_name == "signal_events")
            .expect("signal_events entry");
        assert_eq!(signal_count.row_count, 1);
    }

    #[test]
    fn run_age_based_purge_is_idempotent() {
        let db = test_db();

        // Run purge on empty tables -- should not error
        let report1 = run_age_based_purge(&db);
        assert_eq!(report1.total(), 0);

        // Run again -- still no error, still 0
        let report2 = run_age_based_purge(&db);
        assert_eq!(report2.total(), 0);
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

//! ADR-0098 data lifecycle primitives.
//!
//! Source-aware purge infrastructure used when connector credentials are revoked.
//! Also provides DB growth monitoring and age-based purge.

use std::collections::HashMap;

use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::{ActionDb, DbError};
use crate::db::people::FieldSource;

// =============================================================================
// DB growth monitoring
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
// Age-based purge
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
#[must_use = "check whether aged signals were purged before reporting retained signal volume"]
pub fn purge_aged_signals(db: &ActionDb, days: i64) -> Result<usize, DbError> {
    if !table_exists(db, "signal_events") {
        return Ok(0);
    }
    db.conn_ref()
        .execute(
            "DELETE FROM signal_events
             WHERE created_at < datetime('now', ?1)
               AND data_source != 'user_correction'",
            params![format!("-{days} days")],
        )
        .map_err(|e| DbError::Migration(format!("purge_aged_signals: {e}")))
}

/// Purge deactivated email_signals older than `days`.
#[must_use = "check whether aged email signals were purged before reporting retained email signal volume"]
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
#[must_use = "check whether aged emails were purged before reporting retained email storage"]
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
            // L2 cycle-6 fix #1: the helper restricts the UPDATE to rows whose
            // subject_ref is valid JSON before evaluating json_extract, so
            // malformed historical subject_ref values do not abort the purge.
            crate::services::claims::withdraw_email_subject_claims_for_aged_resolved_emails(
                db, &cutoff,
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
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            // best-effort: preserve the original purge error if rollback itself fails.
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

/// Purge content_embeddings for content_files that no longer exist.
#[must_use = "check whether orphaned embeddings were purged before reporting embedding cleanup"]
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
/// Retention defaults (from spec):
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
    #[serde(default)]
    pub temporal_rows_invalidated: usize,
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
#[must_use = "check whether the source purge completed before treating imported data as removed"]
pub fn purge_source(db: &ActionDb, source: DataSource) -> Result<PurgeReport, DbError> {
    if source == DataSource::User {
        return Ok(PurgeReport {
            source: source.as_str().to_string(),
            ..Default::default()
        });
    }

    db.with_transaction(|tx| {
        let source_str = source.as_str();
        let temporal_rows_invalidated =
            crate::services::temporal::mark_source_invalidated_in_db(tx, source_str, Utc::now())
                .map_err(|e| format!("invalidate temporal source rows failed: {e}"))?;
        let affected_accounts = {
            let mut stmt = tx
                .conn_ref()
                .prepare(
                    "SELECT DISTINCT account_id
                     FROM account_stakeholders
                     WHERE data_source = ?1",
                )
                .map_err(|e| format!("select purged stakeholder accounts failed: {e}"))?;
            let rows = stmt
                .query_map([source_str], |row| row.get::<_, String>(0))
                .map_err(|e| format!("query purged stakeholder accounts failed: {e}"))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("collect purged stakeholder accounts failed: {e}"))?
        };

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let ext = crate::services::context::ExternalClients::default();
        let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
        let mutation_source = format!("purge_source:{source_str}");
        let people_cleared = crate::services::stakeholder_writer::write_with_stakeholders_changed_for_entities(
            &ctx,
            tx,
            &mutation_source,
            |tx| {
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
                let affected_entities = affected_accounts
                    .into_iter()
                    .map(|account_id| (account_id, "account".to_string()))
                    .collect();
                Ok((people_cleared, affected_entities))
            },
        )
        .map_err(|e| format!("emit stakeholder purge signal failed: {e}"))?;

        let signals_deleted = if source == DataSource::Glean {
            tx.conn_ref()
                .execute(
                    "DELETE FROM signal_events
                     WHERE data_source IN (
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
                .execute("DELETE FROM signal_events WHERE data_source = ?1", [source_str])
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
                        // L2 cycle-6 fix #1: the helper filters via subquery
                        // so json_extract is only evaluated on rows whose
                        // subject_ref is valid JSON. See purge_aged_emails for
                        // the same pattern.
                        crate::services::claims::withdraw_email_subject_claims_for_existing_emails(
                            tx,
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
                    tx.conn_ref()
                        .execute("DELETE FROM email_threads", [])
                        .map_err(|e| format!("purge email_threads failed: {e}"))?;
                }
                if table_exists(tx, "meetings") {
                    tx.conn_ref()
                        .execute(
                            "UPDATE meetings SET description = NULL WHERE description IS NOT NULL",
                            [],
                        )
                        .map_err(|e| format!("purge meeting descriptions failed: {e}"))?;
                }
            }
            DataSource::Gravatar if table_exists(tx, "gravatar_cache") => {
                caches_deleted = tx
                    .conn_ref()
                    .execute("DELETE FROM gravatar_cache", [])
                    .map_err(|e| format!("purge gravatar_cache failed: {e}"))?;
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
            temporal_rows_invalidated,
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

    fn seed_signal(
        db: &ActionDb,
        id: &str,
        entity_type: &str,
        entity_id: &str,
        signal_type: &str,
        source: &str,
        confidence: f64,
        created_at: Option<&str>,
    ) {
        let fallback_created_at;
        let created_at = match created_at {
            Some(value) => value,
            None => {
                fallback_created_at = Utc::now().to_rfc3339();
                &fallback_created_at
            }
        };
        crate::signals::bus::emit_signal_fixture_event(
            db,
            id,
            entity_type,
            entity_id,
            signal_type,
            source,
            None,
            confidence,
            None,
            created_at,
        )
        .expect("seed signal");
    }

    #[test]
    fn purge_source_removes_tagged_rows_and_preserves_user_rows() {
        let db = test_db();
        seed_person(&db, "p1", None);
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at)
                 VALUES ('a1', 'Glean Account', '2026-05-03T12:00:00+00:00')",
                [],
            )
            .expect("seed glean account");
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at)
                 VALUES ('a2', 'User Account', '2026-05-03T12:00:00+00:00')",
                [],
            )
            .expect("seed user account");

        db.link_person_to_account_with_source("a1", "p1", "champion", "glean")
            .expect("seed glean stakeholder");
        db.link_person_to_account_with_source("a2", "p1", "champion", "user")
            .expect("seed user stakeholder");

        seed_signal(
            &db,
            "s-glean",
            "account",
            "a1",
            "profile_update",
            "glean",
            0.8,
            None,
        );
        seed_signal(
            &db,
            "s-user",
            "account",
            "a1",
            "profile_update",
            "user",
            0.8,
            None,
        );

        let now = Utc::now();
        let temporal_source_refs =
            serde_json::to_string(&[crate::abilities::provenance::SourceRef::Direct {
                data_source: crate::abilities::provenance::DataSource::Glean {
                    downstream: crate::abilities::provenance::GleanDownstream::Documents,
                },
                identifier: crate::abilities::provenance::SourceIdentifier::Document {
                    document_id: crate::abilities::provenance::DocumentId::new("doc-glean"),
                    chunk_id: None,
                },
                observed_at: now,
                source_asof: Some(now),
            }])
            .expect("serialize temporal source refs");
        db.conn_ref()
            .execute(
                "INSERT INTO entity_engagement_curve (
                     entity_type, entity_id, week_start, meetings_count, emails_count,
                     bidirectional_ratio, source_refs_json
                 ) VALUES ('account', 'a1', '2026-05-04T00:00:00Z', 0, 1, 0.0, ?1)",
                [temporal_source_refs],
            )
            .expect("seed glean temporal row");

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
        assert_eq!(report.temporal_rows_invalidated, 1);

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
                "SELECT COUNT(*) FROM signal_events WHERE data_source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user signals");
        assert_eq!(remaining_user_signals, 1);

        let invalidated_temporal_rows: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                 FROM entity_engagement_curve
                 WHERE entity_type = 'account'
                   AND entity_id = 'a1'
                   AND source_invalidated_at IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .expect("count invalidated temporal rows");
        assert_eq!(invalidated_temporal_rows, 1);
    }

    // --- Age-based purge tests ---

    #[test]
    fn purge_aged_signals_preserves_user_corrections() {
        let db = test_db();

        let old = (Utc::now() - chrono::Duration::days(200)).to_rfc3339();
        let recent = (Utc::now() - chrono::Duration::days(10)).to_rfc3339();

        seed_signal(
            &db,
            "s-old",
            "account",
            "a1",
            "profile_update",
            "email_enrichment",
            0.7,
            Some(&old),
        );
        seed_signal(
            &db,
            "s-correction",
            "account",
            "a1",
            "entity_correction",
            "user_correction",
            1.0,
            Some(&old),
        );
        seed_signal(
            &db,
            "s-recent",
            "account",
            "a1",
            "profile_update",
            "email_enrichment",
            0.7,
            Some(&recent),
        );

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
        // dos7-allowed: cascade tests seed Email-subject claim rows
        // directly so hard-delete behavior can be asserted in isolation.
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims /* dos7-allowed: test seed for Email-subject purge */ \
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

    /// L2 cycle-6 fix #1: a malformed historical `subject_ref` (not
    /// valid JSON) must NOT abort the cascade. The previous WHERE
    /// shape evaluated `json_extract(subject_ref, '$.kind')` before
    /// `json_valid(subject_ref) = 1` so SQLite raised "malformed
    /// JSON" mid-purge and rolled back the whole transaction. The
    /// fix funnels json_extract through a subquery that filters on
    /// json_valid first.
    #[test]
    fn purge_aged_emails_skips_malformed_subject_ref_without_aborting() {
        let db = test_db();

        // Aged resolved email → eligible for purge.
        db.conn_ref()
            .execute(
                "INSERT INTO emails (email_id, resolved_at, received_at) \
                 VALUES ('em-purge', datetime('now', '-90 days'), datetime('now', '-90 days'))",
                [],
            )
            .unwrap();

        // Seed a valid Email-subject claim AND a malformed-JSON claim.
        // dos7-allowed: cascade tests seed malformed historical rows
        // directly to assert the purge path skips invalid subject JSON.
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims /* dos7-allowed: test seed for malformed Email-subject purge */ \
                 (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
                  actor, data_source, observed_at, created_at, provenance_json, \
                  claim_state, surfacing_state, retraction_reason, temporal_scope, sensitivity) \
                 VALUES \
                 ('claim-valid', '{\"kind\":\"Email\",\"id\":\"em-purge\"}', 'email_dismissed', \
                  'commitment', 'reply by friday', 'k-v', 'h-v', 'user', 'user_dismissal', \
                  '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', '{}', \
                  'tombstoned', 'dormant', 'user_removal', 'state', 'internal'), \
                 ('claim-malformed', 'this is not json', 'email_dismissed', \
                  'commitment', 'orphan', 'k-m', 'h-m', 'user', 'user_dismissal', \
                  '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', '{}', \
                  'tombstoned', 'dormant', 'user_removal', 'state', 'internal')",
                [],
            )
            .unwrap();

        let purged =
            purge_aged_emails(&db, 60).expect("malformed subject_ref must NOT abort the cascade");
        assert_eq!(purged, 1);

        // Valid claim was withdrawn.
        let valid_state: String = db
            .conn_ref()
            .query_row(
                "SELECT claim_state FROM intelligence_claims WHERE id = 'claim-valid'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(valid_state, "withdrawn");

        // Malformed claim was untouched (skipped by the json_valid filter).
        let malformed_state: String = db
            .conn_ref()
            .query_row(
                "SELECT claim_state FROM intelligence_claims WHERE id = 'claim-malformed'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(malformed_state, "tombstoned");
    }

    #[test]
    fn db_growth_report_returns_table_counts() {
        let db = test_db();

        seed_signal(&db, "s1", "account", "a1", "test", "test", 0.5, None);

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

// =============================================================================
// One-time init backfills and migrations (DOS-94 — moved from db/core.rs)
//
// These ActionDb methods run during startup paths in `db::core` and convert
// legacy state to current schema invariants. Visibility is `pub(super)` so
// `db::core` can keep its existing call sites via `Self::` / `self.`.
// =============================================================================

use rusqlite::Connection;

impl ActionDb {
    /// Re-key meeting IDs to canonical event IDs and update dependent references.
    #[must_use = "check that meeting identity backfill succeeded before relying on canonical meeting IDs"]
    pub(super) fn backfill_meeting_identity(conn: &Connection) -> Result<(), DbError> {
        let rows: Vec<(String, String)> = {
            let mut stmt = conn.prepare(
                "SELECT id, calendar_event_id
                 FROM meetings
                 WHERE calendar_event_id IS NOT NULL
                   AND trim(calendar_event_id) != ''",
            )?;
            let mapped = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut items = Vec::new();
            for row in mapped {
                items.push(row?);
            }
            items
        };

        for (old_id, calendar_event_id) in rows {
            let canonical_id = Self::sanitize_calendar_event_id(&calendar_event_id);
            if canonical_id.is_empty() || canonical_id == old_id {
                continue;
            }

            let canonical_exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM meetings WHERE id = ?1",
                params![canonical_id],
                |r| r.get(0),
            )?;

            if canonical_exists > 0 {
                // Merge sparse fields from old row into canonical row (meetings table).
                conn.execute(
                    "UPDATE meetings
                     SET title = COALESCE(title, (SELECT title FROM meetings WHERE id = ?1)),
                         meeting_type = COALESCE(meeting_type, (SELECT meeting_type FROM meetings WHERE id = ?1)),
                         start_time = COALESCE(start_time, (SELECT start_time FROM meetings WHERE id = ?1)),
                         end_time = COALESCE(end_time, (SELECT end_time FROM meetings WHERE id = ?1)),
                         attendees = COALESCE(attendees, (SELECT attendees FROM meetings WHERE id = ?1)),
                         notes_path = COALESCE(notes_path, (SELECT notes_path FROM meetings WHERE id = ?1)),
                         description = COALESCE(description, (SELECT description FROM meetings WHERE id = ?1))
                     WHERE id = ?2",
                    params![old_id, canonical_id],
                )?;
                // Merge meeting_prep fields
                conn.execute(
                    "UPDATE meeting_prep
                     SET prep_context_json = COALESCE(prep_context_json, (SELECT prep_context_json FROM meeting_prep WHERE meeting_id = ?1)),
                         user_agenda_json = COALESCE(user_agenda_json, (SELECT user_agenda_json FROM meeting_prep WHERE meeting_id = ?1)),
                         user_notes = COALESCE(user_notes, (SELECT user_notes FROM meeting_prep WHERE meeting_id = ?1)),
                         prep_frozen_json = COALESCE(prep_frozen_json, (SELECT prep_frozen_json FROM meeting_prep WHERE meeting_id = ?1)),
                         prep_frozen_at = COALESCE(prep_frozen_at, (SELECT prep_frozen_at FROM meeting_prep WHERE meeting_id = ?1)),
                         prep_snapshot_path = COALESCE(prep_snapshot_path, (SELECT prep_snapshot_path FROM meeting_prep WHERE meeting_id = ?1)),
                         prep_snapshot_hash = COALESCE(prep_snapshot_hash, (SELECT prep_snapshot_hash FROM meeting_prep WHERE meeting_id = ?1))
                     WHERE meeting_id = ?2",
                    params![old_id, canonical_id],
                )?;
                // Merge meeting_transcripts fields
                conn.execute(
                    "UPDATE meeting_transcripts
                     SET summary = COALESCE(summary, (SELECT summary FROM meeting_transcripts WHERE meeting_id = ?1)),
                         transcript_path = COALESCE(transcript_path, (SELECT transcript_path FROM meeting_transcripts WHERE meeting_id = ?1)),
                         transcript_processed_at = COALESCE(transcript_processed_at, (SELECT transcript_processed_at FROM meeting_transcripts WHERE meeting_id = ?1))
                     WHERE meeting_id = ?2",
                    params![old_id, canonical_id],
                )?;
            } else {
                conn.execute(
                    "UPDATE meetings SET id = ?1 WHERE id = ?2",
                    params![canonical_id, old_id],
                )?;
                // Child tables updated via CASCADE on meetings.id
            }

            // Update foreign references.
            conn.execute(
                "UPDATE captures SET meeting_id = ?1 WHERE meeting_id = ?2",
                params![canonical_id, old_id],
            )?;
            conn.execute(
                "UPDATE meeting_entities SET meeting_id = ?1 WHERE meeting_id = ?2",
                params![canonical_id, old_id],
            )?;
            conn.execute(
                "UPDATE meeting_attendees SET meeting_id = ?1 WHERE meeting_id = ?2",
                params![canonical_id, old_id],
            )?;
            conn.execute(
                "UPDATE actions
                 SET source_id = ?1
                 WHERE source_type IN ('transcript', 'post_meeting') AND source_id = ?2",
                params![canonical_id, old_id],
            )?;

            // Update reviewed state keys.
            conn.execute(
                "UPDATE meeting_prep_state
                 SET prep_file = ?1
                 WHERE prep_file = ?2 OR prep_file = ?3",
                params![canonical_id, old_id, format!("preps/{}.json", old_id)],
            )?;

            if canonical_exists > 0 {
                // CASCADE will clean up meeting_prep and meeting_transcripts
                conn.execute("DELETE FROM meetings WHERE id = ?1", params![old_id])?;
            }
        }
        Ok(())
    }

    #[must_use = "check that meeting user-layer backfill succeeded before relying on populated agenda/notes columns"]
    pub(super) fn backfill_meeting_user_layer(conn: &Connection) -> Result<(), DbError> {
        let rows: Vec<(String, String, Option<String>, Option<String>)> = {
            let mut stmt = conn.prepare(
                "SELECT mp.meeting_id, mp.prep_context_json, mp.user_agenda_json, mp.user_notes
                 FROM meeting_prep mp
                 WHERE mp.prep_context_json IS NOT NULL
                   AND trim(mp.prep_context_json) != ''",
            )?;
            let mapped = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?;
            let mut items = Vec::new();
            for row in mapped {
                items.push(row?);
            }
            items
        };

        for (meeting_id, prep_json, agenda_existing, notes_existing) in rows {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&prep_json) else {
                continue;
            };
            let agenda_from_prep = value
                .get("userAgenda")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<String>>()
                })
                .filter(|v| !v.is_empty())
                .and_then(|v| serde_json::to_string(&v).ok());
            let notes_from_prep = value
                .get("userNotes")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            let agenda_target = agenda_existing.or(agenda_from_prep);
            let notes_target = notes_existing.or(notes_from_prep);
            if agenda_target.is_none() && notes_target.is_none() {
                continue;
            }

            conn.execute(
                "UPDATE meeting_prep
                 SET user_agenda_json = COALESCE(user_agenda_json, ?1),
                     user_notes = COALESCE(user_notes, ?2)
                 WHERE meeting_id = ?3",
                params![agenda_target, notes_target, meeting_id],
            )?;
        }
        Ok(())
    }

    /// Auto-dismiss stakeholder suggestions for internal team members.
    /// Cleans up suggestions that were created before the internal filter was added.
    #[must_use = "check that internal stakeholder suggestion cleanup succeeded before assuming the queue is filtered"]
    pub(super) fn dismiss_internal_stakeholder_suggestions(conn: &Connection) -> Result<(), DbError> {
        let dismissed = conn.execute(
            "UPDATE stakeholder_suggestions SET status = 'dismissed', resolved_at = datetime('now')
             WHERE status = 'pending' AND (
               (person_id IS NOT NULL AND EXISTS (
                 SELECT 1 FROM people p WHERE p.id = stakeholder_suggestions.person_id AND p.relationship = 'internal'
               ))
               OR (suggested_email IS NOT NULL AND EXISTS (
                 SELECT 1 FROM people p2 WHERE p2.email = LOWER(stakeholder_suggestions.suggested_email) AND p2.relationship = 'internal'
               ))
               OR (suggested_name IS NOT NULL AND EXISTS (
                 SELECT 1 FROM people p3 WHERE LOWER(p3.name) = LOWER(stakeholder_suggestions.suggested_name) AND p3.relationship = 'internal'
               ))
             )",
            [],
        )?;
        if dismissed > 0 {
            log::info!(
                "I652: auto-dismissed {} internal stakeholder suggestions",
                dismissed
            );
        }
        Ok(())
    }

    /// Backfill engagement/assessment columns on `account_stakeholders` from
    /// the legacy `entity_assessment.stakeholder_insights_json` blob.
    /// Runs once at startup — only touches rows where `engagement IS NULL`.
    #[must_use = "check that stakeholder column backfill succeeded before relying on engagement/assessment values"]
    pub(super) fn backfill_stakeholder_columns(conn: &Connection) -> Result<(), DbError> {
        // Step 1: Find all account_stakeholders rows missing engagement.
        let rows: Vec<(String, String)> = {
            let mut stmt = conn.prepare(
                "SELECT account_id, person_id FROM account_stakeholders WHERE engagement IS NULL",
            )?;
            let mapped = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut items = Vec::new();
            for row in mapped {
                items.push(row?);
            }
            items
        };

        if rows.is_empty() {
            return Ok(());
        }

        // Step 2: Group by account_id to avoid repeated JSON parses.
        let mut by_account: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (account_id, person_id) in &rows {
            by_account
                .entry(account_id.clone())
                .or_default()
                .push(person_id.clone());
        }

        let mut updated = 0u32;
        for (account_id, person_ids) in &by_account {
            // Step 3: Read the stakeholder_insights_json for this entity.
            let json_opt: Option<String> = conn
                .query_row(
                    "SELECT stakeholder_insights_json FROM entity_assessment WHERE entity_id = ?1",
                    params![account_id],
                    |row| row.get(0),
                )
                .ok();

            let json_str = match json_opt {
                Some(ref s) if !s.is_empty() => s.as_str(),
                _ => continue,
            };

            let entries: Vec<serde_json::Value> = match serde_json::from_str(json_str) {
                Ok(v) => v,
                Err(err) => {
                    log::warn!(
                        "I652 backfill: failed to parse stakeholder_insights_json for {}: {}",
                        account_id,
                        err
                    );
                    continue;
                }
            };

            // Step 4: For each person_id, find a matching entry and update.
            for person_id in person_ids {
                let matching = entries.iter().find(|e| {
                    e.get("person_id")
                        .and_then(|v| v.as_str())
                        .map(|pid| pid == person_id)
                        .unwrap_or(false)
                });

                if let Some(entry) = matching {
                    let engagement = entry.get("engagement").and_then(|v| v.as_str());
                    let assessment = entry.get("assessment").and_then(|v| v.as_str());

                    if engagement.is_some() || assessment.is_some() {
                        conn.execute(
                            "UPDATE account_stakeholders
                             SET engagement = COALESCE(engagement, ?1),
                                 assessment = COALESCE(assessment, ?2),
                                 data_source_engagement = COALESCE(data_source_engagement, 'ai'),
                                 data_source_assessment = COALESCE(data_source_assessment, 'ai')
                             WHERE account_id = ?3 AND person_id = ?4
                               AND engagement IS NULL",
                            params![engagement, assessment, account_id, person_id],
                        )?;
                        updated += 1;
                    }
                }
            }
        }

        if updated > 0 {
            log::info!(
                "I652 backfill: populated engagement/assessment for {} stakeholder rows",
                updated
            );
        }
        Ok(())
    }

    /// Check if a one-time init task has been completed.
    ///
    /// Returns true if the task has already run and been marked in init_tasks.
    pub(super) fn is_init_task_completed(conn: &Connection, task_name: &str) -> Result<bool, DbError> {
        let completed = conn
            .query_row(
                "SELECT 1 FROM init_tasks WHERE task_name = ?1",
                params![task_name],
                |_| Ok(true),
            )
            .unwrap_or(false);
        Ok(completed)
    }

    /// Mark a one-time init task as completed.
    #[must_use = "check that init-task mark succeeded before assuming the guarded backfill will not re-run on next startup"]
    pub(super) fn mark_init_task_completed(conn: &Connection, task_name: &str) -> Result<(), DbError> {
        conn.execute(
            "INSERT OR IGNORE INTO init_tasks (task_name) VALUES (?1)",
            params![task_name],
        )?;
        Ok(())
    }

    /// Guarded backfill: Account domains from meeting attendees (Path 2b entity resolution).
    ///
    /// Runs exactly once. Subsequent calls are guarded by init_tasks table.
    #[must_use = "check that the guarded domain backfill succeeded before relying on account_domains for entity resolution"]
    pub(super) fn run_guarded_init_backfill_account_domains(&self) -> Result<(), DbError> {
        // v3: purge ALL domains (v1 and v2 both stored contaminated data),
        // then re-backfill with domain-base matching (only stores a domain
        // on an account when the domain base matches the account name/slug).
        const TASK_NAME: &str = "backfill_account_domains_v3";

        if Self::is_init_task_completed(&self.conn, TASK_NAME)? {
            return Ok(());
        }

        // Purge all contaminated domains — the new backfill will repopulate correctly
        let purged = self.conn.execute("DELETE FROM account_domains", [])?;
        if purged > 0 {
            log::info!(
                "Entity resolution: purged all {} account_domains rows (v3 clean slate)",
                purged,
            );
        }

        // Also clean up over-linked future meetings (>2 entity links = contamination symptom)
        let cleaned = self.conn.execute(
            "DELETE FROM meeting_entities WHERE meeting_id IN (
                SELECT me.meeting_id FROM meeting_entities me
                JOIN meetings m ON m.id = me.meeting_id
                WHERE m.start_time > datetime('now')
                GROUP BY me.meeting_id HAVING COUNT(*) > 2
            )",
            [],
        )?;
        if cleaned > 0 {
            log::info!(
                "Entity resolution: purged {} over-linked future meeting_entities rows",
                cleaned,
            );
            // Clear prep for those meetings so they regenerate
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            // best-effort: orphan cleanup succeeded; stale prep is regenerated by later prep invalidation paths.
            let _ = self.conn.execute(
                "UPDATE meeting_prep SET prep_frozen_json = NULL, prep_context_json = NULL
                 WHERE meeting_id IN (
                     SELECT id FROM meetings WHERE start_time > datetime('now')
                     AND id NOT IN (SELECT meeting_id FROM meeting_entities)
                 )",
                [],
            );
        }

        // Re-backfill with domain-base matching
        let inserted = self.backfill_account_domains_from_meetings()?;

        Self::mark_init_task_completed(&self.conn, TASK_NAME)?;

        if inserted > 0 {
            log::info!(
                "Entity resolution: backfilled {} account→domain mappings (domain-base matching)",
                inserted
            );
        }

        Ok(())
    }
}

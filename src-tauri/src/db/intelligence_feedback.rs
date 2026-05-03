//! Intelligence feedback persistence.

use super::ActionDb;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

/// A row from the `intelligence_feedback` table.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackRow {
    pub id: String,
    pub entity_id: String,
    pub entity_type: String,
    pub field: String,
    pub feedback_type: String,
    pub previous_value: Option<String>,
    pub context: Option<String>,
    pub created_at: String,
}

/// Parameters for inserting intelligence feedback.
#[derive(Debug)]
pub struct FeedbackInput<'a> {
    pub id: &'a str,
    pub entity_id: &'a str,
    pub entity_type: &'a str,
    pub field: &'a str,
    pub feedback_type: &'a str,
    pub previous_value: Option<&'a str>,
    pub context: Option<&'a str>,
}

/// Decision returned by `ActionDb::is_suppressed`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuppressionDecision {
    /// A tombstone actively suppresses the item.
    Suppressed {
        /// Matching tombstone identifier.
        tombstone_id: TombstoneId,
        /// Match tier that caused suppression.
        reason: SuppressionReason,
        /// Parsed dismissal timestamp for audit lineage.
        dismissed_at: DateTime<Utc>,
        /// Optional tombstone source scope for audit lineage.
        source_scope: Option<String>,
    },
    /// No active matching tombstone was found.
    NotSuppressed,
    /// A matching tombstone tier was malformed.
    Malformed {
        /// Malformed tombstone identifier, or `database` for storage errors.
        record_id: TombstoneId,
        /// Reason the candidate could not be evaluated safely.
        reason: MalformedReason,
    },
}

/// The specificity tier that matched a suppression tombstone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuppressionReason {
    /// The provided canonical item hash matched the tombstone.
    HashMatch,
    /// The provided item text matched the tombstone exactly.
    ExactTextMatch,
    /// A keyless field-wide tombstone matched.
    KeylessFieldSuppression,
}

/// A malformed suppression tombstone or lookup failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MalformedReason {
    /// A timestamp field could not be parsed as RFC3339 or the legacy SQLite
    /// UTC fallback format.
    UnparsableTimestamp { field: &'static str },
    /// `expires_at` is earlier than `dismissed_at`.
    InvalidExpiry,
    /// SQLite lookup failed; this is surfaced as data-plane malformed so
    /// callers cannot silently fail open.
    DatabaseError(String),
}

/// Opaque tombstone identifier. Legacy rows stringify INTEGER PRIMARY KEY;
///  can carry UUID claim identifiers through the same surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TombstoneId(pub String);

#[derive(Debug, Clone)]
struct TombstoneCandidate {
    id: TombstoneId,
    dismissed_at: String,
    expires_at: Option<String>,
    superseded_by_evidence_after: Option<String>,
    item_hash: Option<String>,
    item_key: Option<String>,
    source_scope: Option<String>,
}

#[derive(Debug, Default)]
struct SuppressionCandidateTiers {
    hash: Vec<TombstoneCandidate>,
    exact: Vec<TombstoneCandidate>,
    keyless: Vec<TombstoneCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RowEvaluation {
    Active { dismissed_at: DateTime<Utc> },
    Skipped,
    Malformed(MalformedReason),
}

impl ActionDb {
    /// Insert or replace an intelligence feedback record.
    /// Uses ON CONFLICT on the UNIQUE(entity_id, entity_type, field) constraint
    /// so changing a vote replaces the previous one (AC16).
    pub fn insert_intelligence_feedback(
        &self,
        input: &FeedbackInput<'_>,
    ) -> Result<(), String> {
        self.conn_ref()
            .execute(
                "INSERT INTO intelligence_feedback \
                 (id, entity_id, entity_type, field, feedback_type, previous_value, context) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                 ON CONFLICT(entity_id, entity_type, field) DO UPDATE SET \
                 id = excluded.id, \
                 feedback_type = excluded.feedback_type, \
                 previous_value = excluded.previous_value, \
                 context = excluded.context, \
                 created_at = datetime('now')",
                rusqlite::params![
                    input.id,
                    input.entity_id,
                    input.entity_type,
                    input.field,
                    input.feedback_type,
                    input.previous_value,
                    input.context,
                ],
            )
            .map_err(|e| format!("Insert intelligence feedback: {e}"))?;
        Ok(())
    }

    /// Check if an entity-scoped intelligence item is suppressed.
    ///
    /// The lookup is infallible at the type level: storage errors and malformed
    /// tombstones return `SuppressionDecision::Malformed`, forcing callers to
    /// choose an explicit fail-open or fail-closed policy with audit logging.
    ///
    /// Precedence is resolved in Rust after a bounded candidate fetch:
    /// `item_hash` match beats exact `item_key`, which beats keyless
    /// field-wide tombstones. Within a matching tier, malformed rows are
    /// skipped when a valid candidate in the same tier can decide the result;
    /// `Malformed` is returned only when every candidate in that tier is
    /// malformed. Expired and superseded rows are ignored and can fall through
    /// to less-specific tiers.
    ///
    /// Existing `item_hash` rows may have been written before  locked
    /// canonicalization. New writers use `intelligence::canonicalization`;
    /// existing rows are read best-effort and fall through to text/keyless
    /// matching when their stored hash does not match the locked rule.
    pub fn is_suppressed(
        &self,
        entity_id: &str,
        field_key: &str,
        item_key: Option<&str>,
        item_hash: Option<&str>,
        sourced_at: Option<&str>,
    ) -> SuppressionDecision {
        let candidates = match self.fetch_suppression_candidates(
            entity_id,
            field_key,
            item_key,
            item_hash,
        ) {
            Ok(candidates) => candidates,
            Err(err) => {
                return SuppressionDecision::Malformed {
                    record_id: TombstoneId("database".to_string()),
                    reason: MalformedReason::DatabaseError(err.to_string()),
                };
            }
        };

        let now = Utc::now();

        if item_hash.is_some() {
            let tier = candidates.hash.iter();
            if let Some(decision) =
                resolve_suppression_tier(tier, SuppressionReason::HashMatch, sourced_at, now)
            {
                return decision;
            }
        }

        if item_key.is_some() {
            let tier = candidates.exact.iter();
            if let Some(decision) = resolve_suppression_tier(
                tier,
                SuppressionReason::ExactTextMatch,
                sourced_at,
                now,
            ) {
                return decision;
            }
        }

        let tier = candidates
            .keyless
            .iter()
            .inspect(|candidate| debug_assert!(candidate.item_hash.is_none()));
        if let Some(decision) = resolve_suppression_tier(
            tier,
            SuppressionReason::KeylessFieldSuppression,
            sourced_at,
            now,
        ) {
            return decision;
        }

        SuppressionDecision::NotSuppressed
    }

    /// Parallel suppression lookup backed by `intelligence_claims`.
    ///
    /// This intentionally lives alongside the legacy `is_suppressed` reader:
    /// callers continue to read `suppression_tombstones` until a follow-up
    /// swaps them after parity is proven.
    pub fn is_suppressed_via_claims(
        &self,
        entity_id: &str,
        field_key: &str,
        item_key: Option<&str>,
        item_hash: Option<&str>,
        sourced_at: Option<&str>,
    ) -> SuppressionDecision {
        let candidates = match self.fetch_suppression_claim_candidates(
            entity_id,
            field_key,
            item_key,
            item_hash,
        ) {
            Ok(candidates) => candidates,
            Err(err) => {
                return SuppressionDecision::Malformed {
                    record_id: TombstoneId("database".to_string()),
                    reason: MalformedReason::DatabaseError(err.to_string()),
                };
            }
        };

        let now = Utc::now();

        if item_hash.is_some() {
            let tier = candidates.hash.iter();
            if let Some(decision) =
                resolve_suppression_tier(tier, SuppressionReason::HashMatch, sourced_at, now)
            {
                return decision;
            }
        }

        if item_key.is_some() {
            let tier = candidates.exact.iter();
            if let Some(decision) = resolve_suppression_tier(
                tier,
                SuppressionReason::ExactTextMatch,
                sourced_at,
                now,
            ) {
                return decision;
            }
        }

        let tier = candidates.keyless.iter();
        if let Some(decision) = resolve_suppression_tier(
            tier,
            SuppressionReason::KeylessFieldSuppression,
            sourced_at,
            now,
        ) {
            return decision;
        }

        SuppressionDecision::NotSuppressed
    }

    fn fetch_suppression_candidates(
        &self,
        entity_id: &str,
        field_key: &str,
        item_key: Option<&str>,
        item_hash: Option<&str>,
    ) -> rusqlite::Result<SuppressionCandidateTiers> {
        let mut candidates = SuppressionCandidateTiers::default();

        if let Some(item_hash) = item_hash {
            candidates.hash = self.query_suppression_candidates(
                "SELECT id, dismissed_at, expires_at, superseded_by_evidence_after, \
                        item_hash, item_key, source_scope \
                 FROM suppression_tombstones \
                 WHERE entity_id = ?1 \
                   AND field_key = ?2 \
                   AND item_hash IS NOT NULL \
                   AND item_hash = ?4 \
                 ORDER BY dismissed_at DESC \
                 LIMIT 16",
                rusqlite::params![entity_id, field_key, item_key, item_hash],
            )?;
        }

        if let Some(item_key) = item_key {
            candidates.exact = self.query_suppression_candidates(
                "SELECT id, dismissed_at, expires_at, superseded_by_evidence_after, \
                        item_hash, item_key, source_scope \
                 FROM suppression_tombstones \
                 WHERE entity_id = ?1 \
                   AND field_key = ?2 \
                   AND item_key IS NOT NULL \
                   AND item_key = ?3 \
                 ORDER BY dismissed_at DESC \
                 LIMIT 16",
                rusqlite::params![entity_id, field_key, item_key],
            )?;
        }

        candidates.keyless = self.query_suppression_candidates(
            "SELECT id, dismissed_at, expires_at, superseded_by_evidence_after, \
                    item_hash, item_key, source_scope \
             FROM suppression_tombstones \
             WHERE entity_id = ?1 \
               AND field_key = ?2 \
               AND item_key IS NULL \
               AND item_hash IS NULL \
             ORDER BY dismissed_at DESC \
             LIMIT 16",
            rusqlite::params![entity_id, field_key],
        )?;

        Ok(candidates)
    }

    fn fetch_suppression_claim_candidates(
        &self,
        entity_id: &str,
        field_key: &str,
        item_key: Option<&str>,
        item_hash: Option<&str>,
    ) -> rusqlite::Result<SuppressionCandidateTiers> {
        let mut candidates = SuppressionCandidateTiers::default();

        // L2 cycle-11 fix #2: match via json_extract on $.kind+$.id
        // instead of exact subject_ref string equality. The SQL m1
        // backfill writes subject_ref via json_object which escapes
        // quotes/backslashes/controls, so a format!-built lookup
        // string for an entity_id with weird characters wouldn't
        // match. json_extract is order-agnostic AND
        // escape-agnostic, so the lookup parity holds across
        // legacy + runtime subject_ref shapes.
        //
        // Also: json_valid(subject_ref)=1 guard prevents malformed
        // historical rows from raising "malformed JSON" mid-query
        // (same hazard cycle-7 fix #2 closed for PRE-GATE).
        const KIND_ID_PREDICATE: &str =
            "json_valid(subject_ref) = 1 \
             AND lower(json_extract(subject_ref, '$.kind')) = 'account' \
             AND json_extract(subject_ref, '$.id') = ?1";

        if let Some(item_hash) = item_hash {
            candidates.hash = self.query_suppression_claim_candidates(
                "SELECT id, created_at, expires_at, metadata_json, item_hash, text \
                 FROM intelligence_claims \
                 WHERE json_valid(subject_ref) = 1 \
                   AND lower(json_extract(subject_ref, '$.kind')) = 'account' \
                   AND json_extract(subject_ref, '$.id') = ?1 \
                   AND claim_type = 'risk' \
                   AND field_path = ?2 \
                   AND claim_state = 'tombstoned' \
                   AND item_hash IS NOT NULL \
                   AND item_hash = ?3 \
                 ORDER BY created_at DESC \
                 LIMIT 16",
                rusqlite::params![entity_id, field_key, item_hash],
            )?;
        }

        if let Some(item_key) = item_key {
            candidates.exact = self.query_suppression_claim_candidates(
                "SELECT id, created_at, expires_at, metadata_json, item_hash, text \
                 FROM intelligence_claims \
                 WHERE json_valid(subject_ref) = 1 \
                   AND lower(json_extract(subject_ref, '$.kind')) = 'account' \
                   AND json_extract(subject_ref, '$.id') = ?1 \
                   AND claim_type = 'risk' \
                   AND field_path = ?2 \
                   AND claim_state = 'tombstoned' \
                   AND text = ?3 \
                 ORDER BY created_at DESC \
                 LIMIT 16",
                rusqlite::params![entity_id, field_key, item_key],
            )?;
        }

        candidates.keyless = self.query_suppression_claim_candidates(
            "SELECT id, created_at, expires_at, metadata_json, item_hash, text \
             FROM intelligence_claims \
             WHERE json_valid(subject_ref) = 1 \
               AND lower(json_extract(subject_ref, '$.kind')) = 'account' \
               AND json_extract(subject_ref, '$.id') = ?1 \
               AND claim_type = 'risk' \
               AND field_path = ?2 \
               AND claim_state = 'tombstoned' \
               AND text = '<keyless>' \
             ORDER BY created_at DESC \
             LIMIT 16",
            rusqlite::params![entity_id, field_key],
        )?;

        // KIND_ID_PREDICATE is documented for clarity but not used
        // as a SQL fragment (rusqlite doesn't support fragment splicing).
        let _ = KIND_ID_PREDICATE;
        Ok(candidates)
    }

    fn query_suppression_candidates<P>(
        &self,
        sql: &str,
        params: P,
    ) -> rusqlite::Result<Vec<TombstoneCandidate>>
    where
        P: rusqlite::Params,
    {
        let mut stmt = self.conn_ref().prepare(sql)?;
        let rows = stmt.query_map(params, |row| {
            let id: i64 = row.get(0)?;
            Ok(TombstoneCandidate {
                id: TombstoneId(id.to_string()),
                dismissed_at: row.get(1)?,
                expires_at: row.get(2)?,
                superseded_by_evidence_after: row.get(3)?,
                item_hash: row.get(4)?,
                item_key: row.get(5)?,
                source_scope: row.get(6)?,
            })
        })?;

        rows.collect()
    }

    fn query_suppression_claim_candidates<P>(
        &self,
        sql: &str,
        params: P,
    ) -> rusqlite::Result<Vec<TombstoneCandidate>>
    where
        P: rusqlite::Params,
    {
        let mut stmt = self.conn_ref().prepare(sql)?;
        let rows = stmt.query_map(params, |row| {
            let metadata_json: Option<String> = row.get(3)?;
            let (source_scope, superseded_by_evidence_after) =
                parse_claim_suppression_metadata(metadata_json.as_deref());
            Ok(TombstoneCandidate {
                id: TombstoneId(row.get(0)?),
                dismissed_at: row.get(1)?,
                expires_at: row.get(2)?,
                superseded_by_evidence_after,
                item_hash: row.get(4)?,
                item_key: row.get(5)?,
                source_scope,
            })
        })?;

        rows.collect()
    }

    /// Get all feedback for an entity, newest first.
    ///
    /// Compatibility read during the feedback-pipeline collapse:
    /// - legacy `intelligence_feedback` votes remain visible
    /// - new `entity_feedback_events` rows are mapped back into the old
    ///   positive/negative/replaced shape for existing thumbs surfaces
    pub fn get_entity_feedback(
        &self,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<Vec<FeedbackRow>, String> {
        let conn = self.conn_ref();
        let mut stmt = conn
            .prepare(
                "SELECT id, entity_id, entity_type, field, feedback_type, \
                 previous_value, context, created_at \
                 FROM (
                    SELECT id, entity_id, entity_type, field, feedback_type, \
                           previous_value, context, created_at \
                    FROM intelligence_feedback \
                    WHERE entity_id = ?1 AND entity_type = ?2

                    UNION ALL

                    SELECT CAST(id AS TEXT) AS id,
                           entity_id,
                           entity_type,
                           CASE
                               WHEN source_kind = 'field_conflict'
                                   THEN 'account_field_conflict:' || field_key || ':' || COALESCE(corrected_value, '')
                               ELSE field_key
                           END AS field,
                           CASE
                               WHEN feedback_type IN ('confirmed', 'accept') THEN 'positive'
                               WHEN feedback_type IN ('rejected', 'dismissed', 'reject', 'dismiss') THEN 'negative'
                               WHEN feedback_type = 'corrected' THEN 'replaced'
                               ELSE feedback_type
                           END AS feedback_type,
                           previous_value,
                           reason AS context,
                           created_at
                    FROM entity_feedback_events \
                    WHERE entity_id = ?1 AND entity_type = ?2 \
                      AND feedback_type IN ('confirmed', 'rejected', 'corrected', 'dismissed', 'accept', 'reject', 'dismiss')
                 ) \
                 ORDER BY created_at DESC",
            )
            .map_err(|e| format!("Prepare get_entity_feedback: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params![entity_id, entity_type], |row| {
                Ok(FeedbackRow {
                    id: row.get(0)?,
                    entity_id: row.get(1)?,
                    entity_type: row.get(2)?,
                    field: row.get(3)?,
                    feedback_type: row.get(4)?,
                    previous_value: row.get(5)?,
                    context: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| format!("Query get_entity_feedback: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Collect get_entity_feedback: {e}"))
    }
}

fn resolve_suppression_tier<'a>(
    candidates: impl Iterator<Item = &'a TombstoneCandidate>,
    reason: SuppressionReason,
    sourced_at: Option<&str>,
    now: DateTime<Utc>,
) -> Option<SuppressionDecision> {
    let mut saw_candidate = false;
    let mut saw_non_malformed = false;
    let mut first_malformed: Option<(TombstoneId, MalformedReason)> = None;
    let mut latest_active: Option<(&TombstoneCandidate, DateTime<Utc>)> = None;

    for candidate in candidates {
        saw_candidate = true;
        match evaluate_candidate(candidate, sourced_at, now) {
            RowEvaluation::Active { dismissed_at } => {
                saw_non_malformed = true;
                if latest_active
                    .as_ref()
                    .map(|(_, latest_at)| dismissed_at > *latest_at)
                    .unwrap_or(true)
                {
                    latest_active = Some((candidate, dismissed_at));
                }
            }
            RowEvaluation::Skipped => {
                saw_non_malformed = true;
            }
            RowEvaluation::Malformed(malformed_reason) => {
                if first_malformed.is_none() {
                    first_malformed = Some((candidate.id.clone(), malformed_reason));
                }
            }
        }
    }

    if let Some((candidate, dismissed_at)) = latest_active {
        return Some(SuppressionDecision::Suppressed {
            tombstone_id: candidate.id.clone(),
            reason,
            dismissed_at,
            source_scope: candidate.source_scope.clone(),
        });
    }

    if saw_candidate && !saw_non_malformed {
        if let Some((record_id, malformed_reason)) = first_malformed {
            return Some(SuppressionDecision::Malformed {
                record_id,
                reason: malformed_reason,
            });
        }
    }

    None
}

fn parse_claim_suppression_metadata(
    metadata_json: Option<&str>,
) -> (Option<String>, Option<String>) {
    let Some(metadata_json) = metadata_json else {
        return (None, None);
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(metadata_json) else {
        return (None, None);
    };

    let source_scope = value
        .get("source_scope")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let superseded_by_evidence_after = value
        .get("superseded_by_evidence_after")
        .and_then(|value| value.as_str())
        .map(str::to_string);

    (source_scope, superseded_by_evidence_after)
}

fn evaluate_candidate(
    candidate: &TombstoneCandidate,
    sourced_at: Option<&str>,
    now: DateTime<Utc>,
) -> RowEvaluation {
    let dismissed_at = match parse_timestamp(&candidate.dismissed_at, "dismissed_at") {
        Ok(value) => value,
        Err(reason) => return RowEvaluation::Malformed(reason),
    };
    let expires_at = match parse_optional_timestamp(candidate.expires_at.as_deref(), "expires_at")
    {
        Ok(value) => value,
        Err(reason) => return RowEvaluation::Malformed(reason),
    };
    let superseded_by = match parse_optional_timestamp(
        candidate.superseded_by_evidence_after.as_deref(),
        "superseded_by_evidence_after",
    ) {
        Ok(value) => value,
        Err(reason) => return RowEvaluation::Malformed(reason),
    };
    let sourced_at = match parse_optional_timestamp(sourced_at, "sourced_at") {
        Ok(value) => value,
        Err(reason) => return RowEvaluation::Malformed(reason),
    };

    if let Some(expires_at) = expires_at {
        if expires_at < dismissed_at {
            return RowEvaluation::Malformed(MalformedReason::InvalidExpiry);
        }
        if expires_at < now {
            return RowEvaluation::Skipped;
        }
    }

    if let Some(sourced_at) = sourced_at {
        let superseded_after = superseded_by.unwrap_or(dismissed_at);
        if sourced_at > superseded_after {
            return RowEvaluation::Skipped;
        }
    }

    RowEvaluation::Active { dismissed_at }
}

fn parse_optional_timestamp(
    value: Option<&str>,
    field: &'static str,
) -> Result<Option<DateTime<Utc>>, MalformedReason> {
    value
        .map(|raw| parse_timestamp(raw, field))
        .transpose()
}

fn parse_timestamp(raw: &str, field: &'static str) -> Result<DateTime<Utc>, MalformedReason> {
    if let Ok(value) = DateTime::parse_from_rfc3339(raw) {
        return Ok(value.with_timezone(&Utc));
    }

    for format in [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
    ] {
        if let Ok(value) = NaiveDateTime::parse_from_str(raw, format) {
            return Ok(Utc.from_utc_datetime(&value));
        }
    }

    Err(MalformedReason::UnparsableTimestamp { field })
}

#[cfg(test)]
mod tests {
    use super::{
        MalformedReason, SuppressionDecision, SuppressionReason, TombstoneId,
    };
    use crate::db::test_utils::test_db;
    use crate::db::ActionDb;
    use crate::intelligence::canonicalization::{item_hash, ItemKind};
    use rusqlite::Connection;

    #[derive(Clone, Copy)]
    struct Tombstone<'a> {
        entity_id: &'a str,
        field_key: &'a str,
        item_key: Option<&'a str>,
        item_hash: Option<&'a str>,
        dismissed_at: &'a str,
        source_scope: Option<&'a str>,
        expires_at: Option<&'a str>,
        superseded_by_evidence_after: Option<&'a str>,
    }

    impl<'a> Tombstone<'a> {
        fn exact(item_key: &'a str, dismissed_at: &'a str) -> Self {
            Self {
                entity_id: "acct-1",
                field_key: "risks",
                item_key: Some(item_key),
                item_hash: None,
                dismissed_at,
                source_scope: None,
                expires_at: None,
                superseded_by_evidence_after: None,
            }
        }
    }

    fn insert_tombstone(db: &ActionDb, tombstone: Tombstone<'_>) -> TombstoneId {
        db.conn_ref()
            .execute(
                "INSERT INTO suppression_tombstones \
                 (entity_id, field_key, item_key, item_hash, dismissed_at, \
                  source_scope, expires_at, superseded_by_evidence_after) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    tombstone.entity_id,
                    tombstone.field_key,
                    tombstone.item_key,
                    tombstone.item_hash,
                    tombstone.dismissed_at,
                    tombstone.source_scope,
                    tombstone.expires_at,
                    tombstone.superseded_by_evidence_after,
                ],
            )
            .expect("insert tombstone");
        TombstoneId(db.conn_ref().last_insert_rowid().to_string())
    }

    fn setup_pair_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        crate::migrations::run_migrations(&conn).expect("apply migrations");
        conn.execute_batch("PRAGMA foreign_keys = OFF;")
            .expect("disable FK for tests");
        conn
    }

    fn seed_pair(conn: &Connection, tombstone: Tombstone<'_>) -> TombstoneId {
        let db = ActionDb::from_conn(conn);
        let tombstone_id = insert_tombstone(db, tombstone);
        let claim_id = format!("m1-{}", tombstone_id.0);
        let source_id = tombstone_id
            .0
            .parse::<i64>()
            .expect("numeric tombstone id");
        let subject_ref = format!(
            r#"{{"kind":"Account","id":"{}"}}"#,
            tombstone.entity_id
        );
        let text = tombstone.item_key.unwrap_or("<keyless>");
        let dedup_item = tombstone
            .item_hash
            .or(tombstone.item_key)
            .unwrap_or("<keyless>");
        let dedup_key = format!(
            "{}:{}:risk:{}",
            dedup_item, tombstone.entity_id, tombstone.field_key
        );
        let claim_item_hash = tombstone.item_hash.unwrap_or("");
        let provenance_json = serde_json::json!({
            "backfill_mechanism": "suppression_tombstones",
            "source_table": "suppression_tombstones",
            "source_id": source_id
        })
        .to_string();
        let metadata_json = serde_json::json!({
            "item_key": tombstone.item_key,
            "item_hash": tombstone.item_hash,
            "source_scope": tombstone.source_scope,
            "superseded_by_evidence_after": tombstone.superseded_by_evidence_after
        })
        .to_string();

        conn.execute(
            "INSERT INTO intelligence_claims \
             (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
              actor, data_source, observed_at, created_at, provenance_json, metadata_json, \
              claim_state, surfacing_state, retraction_reason, expires_at, temporal_scope, \
              sensitivity) \
             VALUES (?1, ?2, 'risk', ?3, ?4, ?5, ?6, \
                     'system_backfill', 'legacy_dismissal', ?7, ?7, ?8, ?9, \
                     'tombstoned', 'active', 'user_removal', ?10, 'state', \
                     'internal')",
            rusqlite::params![
                claim_id,
                subject_ref,
                tombstone.field_key,
                text,
                dedup_key,
                claim_item_hash,
                tombstone.dismissed_at,
                provenance_json,
                metadata_json,
                tombstone.expires_at,
            ],
        )
        .expect("insert claim tombstone");

        tombstone_id
    }

    fn assert_decision_parity(
        legacy: SuppressionDecision,
        via_claims: SuppressionDecision,
    ) {
        match (legacy, via_claims) {
            (
                SuppressionDecision::Suppressed {
                    reason: legacy_reason,
                    dismissed_at: legacy_dismissed_at,
                    source_scope: legacy_source_scope,
                    ..
                },
                SuppressionDecision::Suppressed {
                    reason: claims_reason,
                    dismissed_at: claims_dismissed_at,
                    source_scope: claims_source_scope,
                    ..
                },
            ) => {
                assert_eq!(legacy_reason, claims_reason);
                assert_eq!(legacy_dismissed_at, claims_dismissed_at);
                assert_eq!(legacy_source_scope, claims_source_scope);
            }
            (
                SuppressionDecision::Malformed {
                    reason: legacy_reason,
                    ..
                },
                SuppressionDecision::Malformed {
                    reason: claims_reason,
                    ..
                },
            ) => {
                assert_eq!(legacy_reason, claims_reason);
            }
            (SuppressionDecision::NotSuppressed, SuppressionDecision::NotSuppressed) => {}
            (legacy, via_claims) => {
                panic!("expected parity, legacy={legacy:?}, via_claims={via_claims:?}")
            }
        }
    }

    fn assert_pair_decision(
        conn: &Connection,
        entity_id: &str,
        field_key: &str,
        item_key: Option<&str>,
        item_hash: Option<&str>,
        sourced_at: Option<&str>,
    ) {
        let db = ActionDb::from_conn(conn);
        let legacy = db.is_suppressed(entity_id, field_key, item_key, item_hash, sourced_at);
        let via_claims =
            db.is_suppressed_via_claims(entity_id, field_key, item_key, item_hash, sourced_at);

        assert_decision_parity(legacy, via_claims);
    }

    fn assert_suppressed_reason(
        decision: SuppressionDecision,
        expected_reason: SuppressionReason,
    ) -> TombstoneId {
        match decision {
            SuppressionDecision::Suppressed {
                tombstone_id,
                reason,
                ..
            } => {
                assert_eq!(reason, expected_reason);
                tombstone_id
            }
            other => panic!("expected suppressed, got {other:?}"),
        }
    }

    #[test]
    fn is_suppressed_exact_item_key_match() {
        let db = test_db();
        let expected_id = insert_tombstone(
            &db,
            Tombstone::exact("Champion went dark", "2026-01-01T00:00:00Z"),
        );

        let decision = db.is_suppressed(
            "acct-1",
            "risks",
            Some("Champion went dark"),
            None,
            None,
        );

        assert_eq!(
            assert_suppressed_reason(decision, SuppressionReason::ExactTextMatch),
            expected_id
        );
    }

    #[test]
    fn is_suppressed_keyless_field_wide_match() {
        let db = test_db();
        insert_tombstone(
            &db,
            Tombstone {
                item_key: None,
                dismissed_at: "2026-01-01T00:00:00Z",
                ..Tombstone::exact("unused", "2026-01-01T00:00:00Z")
            },
        );

        let decision = db.is_suppressed("acct-1", "risks", Some("Any risk"), None, None);

        assert_suppressed_reason(decision, SuppressionReason::KeylessFieldSuppression);
    }

    #[test]
    fn is_suppressed_multiple_tombstones_uses_latest() {
        let db = test_db();
        insert_tombstone(
            &db,
            Tombstone {
                source_scope: Some("older"),
                ..Tombstone::exact("Churn risk", "2026-01-01T00:00:00Z")
            },
        );
        let expected_id = insert_tombstone(
            &db,
            Tombstone {
                source_scope: Some("newer"),
                ..Tombstone::exact("Churn risk", "2026-02-01T00:00:00Z")
            },
        );

        let decision = db.is_suppressed("acct-1", "risks", Some("Churn risk"), None, None);

        match decision {
            SuppressionDecision::Suppressed {
                tombstone_id,
                source_scope,
                ..
            } => {
                assert_eq!(tombstone_id, expected_id);
                assert_eq!(source_scope.as_deref(), Some("newer"));
            }
            other => panic!("expected suppressed, got {other:?}"),
        }
    }

    #[test]
    fn is_suppressed_expired_tombstone_returns_not_suppressed() {
        let db = test_db();
        insert_tombstone(
            &db,
            Tombstone {
                expires_at: Some("2000-01-02T00:00:00Z"),
                ..Tombstone::exact("Old risk", "2000-01-01T00:00:00Z")
            },
        );

        let decision = db.is_suppressed("acct-1", "risks", Some("Old risk"), None, None);

        assert_eq!(decision, SuppressionDecision::NotSuppressed);
    }

    #[test]
    fn is_suppressed_superseded_by_newer_evidence() {
        let db = test_db();
        insert_tombstone(
            &db,
            Tombstone {
                superseded_by_evidence_after: Some("2026-01-15T00:00:00Z"),
                ..Tombstone::exact("Pipeline risk", "2026-01-01T00:00:00Z")
            },
        );

        let decision = db.is_suppressed(
            "acct-1",
            "risks",
            Some("Pipeline risk"),
            None,
            Some("2026-02-01T00:00:00Z"),
        );

        assert_eq!(decision, SuppressionDecision::NotSuppressed);
    }

    #[test]
    fn is_suppressed_superseded_with_older_evidence_remains_suppressed() {
        let db = test_db();
        insert_tombstone(
            &db,
            Tombstone {
                superseded_by_evidence_after: Some("2026-02-01T00:00:00Z"),
                ..Tombstone::exact("Pipeline risk", "2026-01-01T00:00:00Z")
            },
        );

        let decision = db.is_suppressed(
            "acct-1",
            "risks",
            Some("Pipeline risk"),
            None,
            Some("2026-01-15T00:00:00Z"),
        );

        assert_suppressed_reason(decision, SuppressionReason::ExactTextMatch);
    }

    #[test]
    fn is_suppressed_z_vs_offset_timezone_consistent() {
        let db = test_db();
        insert_tombstone(
            &db,
            Tombstone {
                source_scope: Some("offset"),
                ..Tombstone::exact("Timezone risk", "2099-01-01T00:30:00+01:00")
            },
        );
        insert_tombstone(
            &db,
            Tombstone {
                source_scope: Some("z"),
                ..Tombstone::exact("Timezone risk", "2099-01-01T00:00:00Z")
            },
        );

        let decision = db.is_suppressed("acct-1", "risks", Some("Timezone risk"), None, None);

        match decision {
            SuppressionDecision::Suppressed { source_scope, .. } => {
                assert_eq!(source_scope.as_deref(), Some("z"));
            }
            other => panic!("expected suppressed, got {other:?}"),
        }
    }

    #[test]
    fn is_suppressed_subsecond_precision_consistent() {
        let db = test_db();
        insert_tombstone(
            &db,
            Tombstone {
                source_scope: Some("whole"),
                ..Tombstone::exact("Precision risk", "2099-01-01T00:00:00Z")
            },
        );
        insert_tombstone(
            &db,
            Tombstone {
                source_scope: Some("subsecond"),
                ..Tombstone::exact("Precision risk", "2099-01-01T00:00:00.500Z")
            },
        );

        let decision = db.is_suppressed("acct-1", "risks", Some("Precision risk"), None, None);

        match decision {
            SuppressionDecision::Suppressed { source_scope, .. } => {
                assert_eq!(source_scope.as_deref(), Some("subsecond"));
            }
            other => panic!("expected suppressed, got {other:?}"),
        }
    }

    #[test]
    fn is_suppressed_malformed_tombstone_timestamp_returns_malformed() {
        let db = test_db();
        let expected_id = insert_tombstone(&db, Tombstone::exact("Bad date", "not-a-date"));

        let decision = db.is_suppressed("acct-1", "risks", Some("Bad date"), None, None);

        assert_eq!(
            decision,
            SuppressionDecision::Malformed {
                record_id: expected_id,
                reason: MalformedReason::UnparsableTimestamp {
                    field: "dismissed_at"
                },
            }
        );
    }

    #[test]
    fn is_suppressed_malformed_item_sourced_at_returns_malformed() {
        let db = test_db();
        let expected_id = insert_tombstone(
            &db,
            Tombstone::exact("Bad source date", "2026-01-01T00:00:00Z"),
        );

        let decision = db.is_suppressed(
            "acct-1",
            "risks",
            Some("Bad source date"),
            None,
            Some("not-a-date"),
        );

        assert_eq!(
            decision,
            SuppressionDecision::Malformed {
                record_id: expected_id,
                reason: MalformedReason::UnparsableTimestamp {
                    field: "sourced_at"
                },
            }
        );
    }

    #[test]
    fn is_suppressed_hash_match_beats_exact_key() {
        let db = test_db();
        let hash = item_hash(ItemKind::Risk, "Normalized risk");
        insert_tombstone(
            &db,
            Tombstone {
                source_scope: Some("exact"),
                ..Tombstone::exact("Normalized risk", "2026-03-01T00:00:00Z")
            },
        );
        let expected_id = insert_tombstone(
            &db,
            Tombstone {
                item_key: None,
                item_hash: Some(&hash),
                source_scope: Some("hash"),
                ..Tombstone::exact("unused", "2026-01-01T00:00:00Z")
            },
        );

        let decision = db.is_suppressed(
            "acct-1",
            "risks",
            Some("Normalized risk"),
            Some(&hash),
            None,
        );

        assert_eq!(
            assert_suppressed_reason(decision, SuppressionReason::HashMatch),
            expected_id
        );
    }

    #[test]
    fn is_suppressed_hash_match_with_different_item_key_text() {
        let db = test_db();
        let hash = item_hash(ItemKind::Risk, "ARR at risk");
        insert_tombstone(
            &db,
            Tombstone {
                item_key: Some("ARR   at risk"),
                item_hash: Some(&hash),
                ..Tombstone::exact("unused", "2026-01-01T00:00:00Z")
            },
        );

        let decision = db.is_suppressed(
            "acct-1",
            "risks",
            Some("ARR at risk"),
            Some(&hash),
            None,
        );

        assert_suppressed_reason(decision, SuppressionReason::HashMatch);
    }

    #[test]
    fn is_suppressed_more_than_32_keyless_does_not_evict_hash_match() {
        let db = test_db();
        let hash = item_hash(ItemKind::Risk, "Hash risk");
        let expected_id = insert_tombstone(
            &db,
            Tombstone {
                item_key: None,
                item_hash: Some(&hash),
                dismissed_at: "2026-01-01T00:00:00Z",
                ..Tombstone::exact("unused", "2026-01-01T00:00:00Z")
            },
        );

        for minute in 0..35 {
            let dismissed_at = format!("2026-02-01T00:{minute:02}:00Z");
            insert_tombstone(
                &db,
                Tombstone {
                    item_key: None,
                    item_hash: None,
                    dismissed_at: &dismissed_at,
                    ..Tombstone::exact("unused", "2026-02-01T00:00:00Z")
                },
            );
        }

        let decision = db.is_suppressed("acct-1", "risks", Some("Hash risk"), Some(&hash), None);

        assert_eq!(
            assert_suppressed_reason(decision, SuppressionReason::HashMatch),
            expected_id
        );
    }

    #[test]
    fn is_suppressed_more_than_32_keyless_does_not_evict_exact_match() {
        let db = test_db();
        let expected_id = insert_tombstone(
            &db,
            Tombstone::exact("Exact risk", "2026-01-01T00:00:00Z"),
        );

        for minute in 0..35 {
            let dismissed_at = format!("2026-02-01T00:{minute:02}:00Z");
            insert_tombstone(
                &db,
                Tombstone {
                    item_key: None,
                    item_hash: None,
                    dismissed_at: &dismissed_at,
                    ..Tombstone::exact("unused", "2026-02-01T00:00:00Z")
                },
            );
        }

        let decision = db.is_suppressed("acct-1", "risks", Some("Exact risk"), None, None);

        assert_eq!(
            assert_suppressed_reason(decision, SuppressionReason::ExactTextMatch),
            expected_id
        );
    }

    #[test]
    fn is_suppressed_per_tier_limit_does_not_overflow_hash_tier() {
        let db = test_db();
        let hash = item_hash(ItemKind::Risk, "Hash tier risk");
        let mut expected_id = None;

        for second in 0..20 {
            let dismissed_at = format!("2026-03-01T00:00:{second:02}Z");
            let tombstone_id = insert_tombstone(
                &db,
                Tombstone {
                    item_key: None,
                    item_hash: Some(&hash),
                    dismissed_at: &dismissed_at,
                    ..Tombstone::exact("unused", "2026-03-01T00:00:00Z")
                },
            );
            if second == 19 {
                expected_id = Some(tombstone_id);
            }
        }

        let decision = db.is_suppressed(
            "acct-1",
            "risks",
            Some("Hash tier risk"),
            Some(&hash),
            None,
        );

        assert_eq!(
            assert_suppressed_reason(decision, SuppressionReason::HashMatch),
            expected_id.expect("latest hash tombstone id")
        );
    }

    #[test]
    fn is_suppressed_keyless_tier_not_evicted_by_hash_only_rows() {
        let db = test_db();
        insert_tombstone(
            &db,
            Tombstone {
                item_key: None,
                item_hash: None,
                dismissed_at: "2026-01-01T00:00:00Z",
                ..Tombstone::exact("unused", "2026-01-01T00:00:00Z")
            },
        );

        for second in 0..20 {
            let dismissed_at = format!("2026-02-01T00:00:{second:02}Z");
            let hash = format!("hash-only-{second}");
            insert_tombstone(
                &db,
                Tombstone {
                    item_key: None,
                    item_hash: Some(&hash),
                    dismissed_at: &dismissed_at,
                    ..Tombstone::exact("unused", "2026-02-01T00:00:00Z")
                },
            );
        }

        let decision = db.is_suppressed("acct-1", "risks", Some("Any risk"), None, None);

        assert_suppressed_reason(decision, SuppressionReason::KeylessFieldSuppression);
    }

    #[test]
    fn is_suppressed_no_matching_tombstone_returns_not_suppressed() {
        let db = test_db();
        insert_tombstone(
            &db,
            Tombstone::exact("Different risk", "2026-01-01T00:00:00Z"),
        );

        let decision = db.is_suppressed("acct-1", "risks", Some("Missing risk"), None, None);

        assert_eq!(decision, SuppressionDecision::NotSuppressed);
    }

    #[test]
    fn is_suppressed_different_field_key_returns_not_suppressed() {
        let db = test_db();
        insert_tombstone(
            &db,
            Tombstone::exact("Field risk", "2026-01-01T00:00:00Z"),
        );

        let decision = db.is_suppressed("acct-1", "recentWins", Some("Field risk"), None, None);

        assert_eq!(decision, SuppressionDecision::NotSuppressed);
    }

    #[test]
    fn is_suppressed_inverted_expiry() {
        let db = test_db();
        let expected_id = insert_tombstone(
            &db,
            Tombstone {
                expires_at: Some("2026-01-01T00:00:00Z"),
                ..Tombstone::exact("Bad expiry", "2026-02-01T00:00:00Z")
            },
        );

        let decision = db.is_suppressed("acct-1", "risks", Some("Bad expiry"), None, None);

        assert_eq!(
            decision,
            SuppressionDecision::Malformed {
                record_id: expected_id,
                reason: MalformedReason::InvalidExpiry,
            }
        );
    }

    #[test]
    fn is_suppressed_iterates_past_malformed_within_tier() {
        let db = test_db();
        insert_tombstone(&db, Tombstone::exact("Mixed date", "not-a-date"));
        let expected_id = insert_tombstone(
            &db,
            Tombstone::exact("Mixed date", "2026-01-01T00:00:00Z"),
        );

        let decision = db.is_suppressed("acct-1", "risks", Some("Mixed date"), None, None);

        assert_eq!(
            assert_suppressed_reason(decision, SuppressionReason::ExactTextMatch),
            expected_id
        );
    }

    #[test]
    fn parity_is_suppressed_exact_item_key_match() {
        let conn = setup_pair_db();
        seed_pair(
            &conn,
            Tombstone::exact("Champion went dark", "2026-01-01T00:00:00Z"),
        );

        assert_pair_decision(
            &conn,
            "acct-1",
            "risks",
            Some("Champion went dark"),
            None,
            None,
        );
    }

    #[test]
    fn parity_is_suppressed_keyless_field_wide_match() {
        let conn = setup_pair_db();
        seed_pair(
            &conn,
            Tombstone {
                item_key: None,
                dismissed_at: "2026-01-01T00:00:00Z",
                ..Tombstone::exact("unused", "2026-01-01T00:00:00Z")
            },
        );

        assert_pair_decision(&conn, "acct-1", "risks", Some("Any risk"), None, None);
    }

    #[test]
    fn parity_is_suppressed_multiple_tombstones_uses_latest() {
        let conn = setup_pair_db();
        seed_pair(
            &conn,
            Tombstone {
                source_scope: Some("older"),
                ..Tombstone::exact("Churn risk", "2026-01-01T00:00:00Z")
            },
        );
        seed_pair(
            &conn,
            Tombstone {
                source_scope: Some("newer"),
                ..Tombstone::exact("Churn risk", "2026-02-01T00:00:00Z")
            },
        );

        assert_pair_decision(&conn, "acct-1", "risks", Some("Churn risk"), None, None);
    }

    #[test]
    fn parity_is_suppressed_expired_tombstone_returns_not_suppressed() {
        let conn = setup_pair_db();
        seed_pair(
            &conn,
            Tombstone {
                expires_at: Some("2000-01-02T00:00:00Z"),
                ..Tombstone::exact("Old risk", "2000-01-01T00:00:00Z")
            },
        );

        assert_pair_decision(&conn, "acct-1", "risks", Some("Old risk"), None, None);
    }

    #[test]
    fn parity_is_suppressed_superseded_by_newer_evidence() {
        let conn = setup_pair_db();
        seed_pair(
            &conn,
            Tombstone {
                superseded_by_evidence_after: Some("2026-01-15T00:00:00Z"),
                ..Tombstone::exact("Pipeline risk", "2026-01-01T00:00:00Z")
            },
        );

        assert_pair_decision(
            &conn,
            "acct-1",
            "risks",
            Some("Pipeline risk"),
            None,
            Some("2026-02-01T00:00:00Z"),
        );
    }

    #[test]
    fn parity_is_suppressed_superseded_with_older_evidence_remains_suppressed() {
        let conn = setup_pair_db();
        seed_pair(
            &conn,
            Tombstone {
                superseded_by_evidence_after: Some("2026-02-01T00:00:00Z"),
                ..Tombstone::exact("Pipeline risk", "2026-01-01T00:00:00Z")
            },
        );

        assert_pair_decision(
            &conn,
            "acct-1",
            "risks",
            Some("Pipeline risk"),
            None,
            Some("2026-01-15T00:00:00Z"),
        );
    }

    #[test]
    fn parity_is_suppressed_z_vs_offset_timezone_consistent() {
        let conn = setup_pair_db();
        seed_pair(
            &conn,
            Tombstone {
                source_scope: Some("offset"),
                ..Tombstone::exact("Timezone risk", "2099-01-01T00:30:00+01:00")
            },
        );
        seed_pair(
            &conn,
            Tombstone {
                source_scope: Some("z"),
                ..Tombstone::exact("Timezone risk", "2099-01-01T00:00:00Z")
            },
        );

        assert_pair_decision(&conn, "acct-1", "risks", Some("Timezone risk"), None, None);
    }

    #[test]
    fn parity_is_suppressed_subsecond_precision_consistent() {
        let conn = setup_pair_db();
        seed_pair(
            &conn,
            Tombstone {
                source_scope: Some("whole"),
                ..Tombstone::exact("Precision risk", "2099-01-01T00:00:00Z")
            },
        );
        seed_pair(
            &conn,
            Tombstone {
                source_scope: Some("subsecond"),
                ..Tombstone::exact("Precision risk", "2099-01-01T00:00:00.500Z")
            },
        );

        assert_pair_decision(&conn, "acct-1", "risks", Some("Precision risk"), None, None);
    }

    #[test]
    fn parity_is_suppressed_malformed_tombstone_timestamp_returns_malformed() {
        let conn = setup_pair_db();
        seed_pair(&conn, Tombstone::exact("Bad date", "not-a-date"));

        assert_pair_decision(&conn, "acct-1", "risks", Some("Bad date"), None, None);
    }

    #[test]
    fn parity_is_suppressed_hash_match_beats_exact_key() {
        let conn = setup_pair_db();
        let hash = item_hash(ItemKind::Risk, "Normalized risk");
        seed_pair(
            &conn,
            Tombstone {
                source_scope: Some("exact"),
                ..Tombstone::exact("Normalized risk", "2026-03-01T00:00:00Z")
            },
        );
        seed_pair(
            &conn,
            Tombstone {
                item_key: None,
                item_hash: Some(&hash),
                source_scope: Some("hash"),
                ..Tombstone::exact("unused", "2026-01-01T00:00:00Z")
            },
        );

        assert_pair_decision(
            &conn,
            "acct-1",
            "risks",
            Some("Normalized risk"),
            Some(&hash),
            None,
        );
    }

    #[test]
    fn parity_is_suppressed_hash_match_with_different_item_key_text() {
        let conn = setup_pair_db();
        let hash = item_hash(ItemKind::Risk, "ARR at risk");
        seed_pair(
            &conn,
            Tombstone {
                item_key: Some("ARR   at risk"),
                item_hash: Some(&hash),
                ..Tombstone::exact("unused", "2026-01-01T00:00:00Z")
            },
        );

        assert_pair_decision(
            &conn,
            "acct-1",
            "risks",
            Some("ARR at risk"),
            Some(&hash),
            None,
        );
    }

    #[test]
    fn parity_is_suppressed_no_matching_tombstone_returns_not_suppressed() {
        let conn = setup_pair_db();
        seed_pair(
            &conn,
            Tombstone::exact("Different risk", "2026-01-01T00:00:00Z"),
        );

        assert_pair_decision(&conn, "acct-1", "risks", Some("Missing risk"), None, None);
    }

    #[test]
    fn parity_is_suppressed_inverted_expiry() {
        let conn = setup_pair_db();
        seed_pair(
            &conn,
            Tombstone {
                expires_at: Some("2026-01-01T00:00:00Z"),
                ..Tombstone::exact("Bad expiry", "2026-02-01T00:00:00Z")
            },
        );

        assert_pair_decision(&conn, "acct-1", "risks", Some("Bad expiry"), None, None);
    }

    /// L2 cycle-11 fix #2: is_suppressed_via_claims must match
    /// claims whose subject_ref was escape-encoded by SQLite's
    /// `json_object()` for an entity_id containing quotes or
    /// backslashes. The previous lookup built subject_ref via raw
    /// format! and did string-equality match — escaped IDs would
    /// silently miss, breaking parity with the legacy reader.
    #[test]
    fn parity_is_suppressed_with_escaped_entity_id_chars() {
        let conn = setup_pair_db();
        let evil_id = r#"acct-with-"quote"-and-\backslash"#;

        // Seed BOTH legacy (suppression_tombstones with the raw
        // entity_id) and the claims-backed row (subject_ref built
        // via json_object so quotes/backslashes are escaped).
        let db = ActionDb::from_conn(&conn);
        let tombstone_id = db
            .create_suppression_tombstone(evil_id, "risks", Some("dismissed text"), None, None, None)
            .expect("seed legacy tombstone");
        let claim_id = format!("m1-{}", tombstone_id);
        // dos7-allowed: cycle-11 parity-regression seed
        conn.execute(
            "INSERT INTO intelligence_claims \
             (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
              actor, data_source, observed_at, created_at, provenance_json, \
              claim_state, surfacing_state, retraction_reason, \
              temporal_scope, sensitivity) \
             VALUES (?1, json_object('kind','Account','id',?2), 'risk', 'risks', \
                     'dismissed text', \
                     'evil-dedup', '', 'system_backfill', 'legacy_dismissal', \
                     '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', '{}', \
                     'tombstoned', 'active', 'user_removal', 'state', 'internal')",
            rusqlite::params![&claim_id, evil_id],
        )
        .expect("seed claims-backed row with escaped subject_ref");

        let db = ActionDb::from_conn(&conn);
        let legacy = db.is_suppressed(evil_id, "risks", Some("dismissed text"), None, None);
        let via_claims =
            db.is_suppressed_via_claims(evil_id, "risks", Some("dismissed text"), None, None);

        assert!(
            matches!(legacy, SuppressionDecision::Suppressed { .. }),
            "legacy tombstone reader should suppress for the escaped-id account"
        );
        assert!(
            matches!(via_claims, SuppressionDecision::Suppressed { .. }),
            "claims-backed reader must suppress for the escaped-id account too \
             (was missing because of format!-built subject_ref equality)"
        );
    }
}

use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::params;
use serde_json::Value;

use crate::abilities::provenance::source_time::{
    parse_source_timestamp, SourceTimestampImplausibleReason, SourceTimestampMalformedReason,
    SourceTimestampStatus,
};
use crate::db::ActionDb;
use crate::intelligence::io::{read_intelligence_json, IntelligenceJson, ItemSource};
use crate::services::context::ServiceContext;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct BackfillSummary {
    pub total_legacy_claims: usize,
    pub accepted: usize,
    pub implausible: usize,
    pub malformed_quarantined: usize,
    pub missing_item_source: usize,
    pub coverage_pct: f64,
}

#[derive(Debug)]
pub enum BackfillError {
    Mode(String),
    Rusqlite(rusqlite::Error),
    MigrationGate(String),
}

impl std::fmt::Display for BackfillError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackfillError::Mode(message) => write!(f, "{message}"),
            BackfillError::Rusqlite(error) => write!(f, "{error}"),
            BackfillError::MigrationGate(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for BackfillError {}

impl From<rusqlite::Error> for BackfillError {
    fn from(error: rusqlite::Error) -> Self {
        BackfillError::Rusqlite(error)
    }
}

#[derive(Debug)]
struct LegacyClaimRow {
    id: String,
    subject_ref: String,
    field_path: Option<String>,
    item_hash: Option<String>,
    observed_at: String,
    provenance_json: String,
    metadata_json: Option<String>,
}

pub fn backfill_source_asof_for_legacy_claims(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    workspace_root: &Path,
    now: DateTime<Utc>,
) -> Result<BackfillSummary, BackfillError> {
    ctx.check_mutation_allowed()
        .map_err(|e| BackfillError::Mode(e.to_string()))?;

    let mut typed_result: Option<Result<BackfillSummary, BackfillError>> = None;
    let transaction_result = db.with_transaction(|tx| {
        let result = backfill_source_asof_for_legacy_claims_tx(tx, workspace_root, now);
        let transaction_return = match &result {
            Ok(_) => Ok(()),
            Err(error) => Err(error.to_string()),
        };
        typed_result = Some(result);
        transaction_return
    });

    let summary = match transaction_result {
        Ok(()) => match typed_result {
            Some(Ok(summary)) => summary,
            Some(Err(error)) => return Err(error),
            None => {
                return Err(BackfillError::Mode(
                    "source_asof backfill transaction did not run".to_string(),
                ))
            }
        },
        Err(message) => match typed_result {
            Some(Err(error)) => return Err(error),
            Some(Ok(_)) | None => return Err(BackfillError::Mode(message)),
        },
    };

    if summary.malformed_quarantined > 0 {
        return Err(BackfillError::MigrationGate(
            "malformed source_asof quarantined".to_string(),
        ));
    }
    if summary.coverage_pct < 0.95 {
        return Err(BackfillError::MigrationGate(
            "coverage below 95%".to_string(),
        ));
    }

    Ok(summary)
}

fn backfill_source_asof_for_legacy_claims_tx(
    tx: &ActionDb,
    workspace_root: &Path,
    now: DateTime<Utc>,
) -> Result<BackfillSummary, BackfillError> {
    let rows = load_legacy_claim_rows(tx)?;
    let mut summary = BackfillSummary {
        total_legacy_claims: rows.len(),
        coverage_pct: 1.0,
        ..Default::default()
    };

    for row in rows {
        let metadata = parse_optional_json_object(row.metadata_json.as_deref(), "metadata_json")?;
        let provenance = parse_required_json_object(&row.provenance_json, "provenance_json")?;
        let mechanism = backfill_mechanism(&metadata, &provenance);
        let candidate = source_timestamp_candidate(
            &row,
            &metadata,
            &provenance,
            mechanism.as_deref(),
            workspace_root,
        );

        let Some(candidate_raw) = candidate else {
            summary.missing_item_source += 1;
            mark_legacy_unattributed(tx, &row.id)?;
            continue;
        };

        match parse_source_timestamp(Some(&candidate_raw), now, None) {
            SourceTimestampStatus::Accepted(parsed) => {
                lift_source_asof(tx, &row.id, &parsed.to_rfc3339())?;
                summary.accepted += 1;
            }
            SourceTimestampStatus::Implausible { parsed, reason } => {
                lift_implausible_source_asof(tx, &row.id, &parsed.to_rfc3339(), metadata, reason)?;
                summary.implausible += 1;
            }
            SourceTimestampStatus::Malformed(reason) => {
                insert_quarantine(
                    tx,
                    &row,
                    &metadata,
                    mechanism.as_deref(),
                    &candidate_raw,
                    malformed_reason_label(reason),
                    now,
                )?;
                summary.malformed_quarantined += 1;
            }
            SourceTimestampStatus::Missing => {
                debug_assert!(
                    false,
                    "source timestamp parser returned Missing for Some input"
                );
                summary.missing_item_source += 1;
                mark_legacy_unattributed(tx, &row.id)?;
            }
        }
    }

    let denominator = summary
        .total_legacy_claims
        .saturating_sub(summary.missing_item_source);
    summary.coverage_pct = if denominator == 0 {
        1.0
    } else {
        summary.accepted as f64 / denominator as f64
    };

    Ok(summary)
}

fn load_legacy_claim_rows(tx: &ActionDb) -> Result<Vec<LegacyClaimRow>, BackfillError> {
    let mut stmt = tx.conn_ref().prepare(
        "SELECT id, subject_ref, field_path, item_hash, observed_at, \
                provenance_json, metadata_json \
         FROM intelligence_claims \
         WHERE source_asof IS NULL \
           AND data_source = 'legacy_dismissal' \
         ORDER BY id",
    )?;
    let mapped = stmt.query_map([], |row| {
        Ok(LegacyClaimRow {
            id: row.get(0)?,
            subject_ref: row.get(1)?,
            field_path: row.get(2)?,
            item_hash: row.get(3)?,
            observed_at: row.get(4)?,
            provenance_json: row.get(5)?,
            metadata_json: row.get(6)?,
        })
    })?;

    let mut rows = Vec::new();
    for row in mapped {
        rows.push(row?);
    }
    Ok(rows)
}

fn lift_source_asof(tx: &ActionDb, claim_id: &str, source_asof: &str) -> Result<(), BackfillError> {
    tx.conn_ref().execute(
        "UPDATE intelligence_claims /* dos7-allowed: source-asof cutover lifts legacy timestamp audit value */ \
         SET source_asof = ?1 /* dos7-allowed: source-asof cutover lifts legacy timestamp audit value */ \
         WHERE id = ?2",
        params![source_asof, claim_id],
    )?;
    Ok(())
}

fn lift_implausible_source_asof(
    tx: &ActionDb,
    claim_id: &str,
    source_asof: &str,
    mut metadata: Value,
    reason: SourceTimestampImplausibleReason,
) -> Result<(), BackfillError> {
    let reason = implausible_reason_label(reason);
    metadata["source_asof_backfill_warning"] = serde_json::json!({
        "kind": "source_timestamp_implausible",
        "reason": reason,
        "freshness_eligible": false
    });
    let metadata_json = serde_json::to_string(&metadata)
        .map_err(|e| BackfillError::Mode(format!("serialize metadata_json: {e}")))?;

    tx.conn_ref().execute(
        "UPDATE intelligence_claims /* dos7-allowed: source-asof cutover flags implausible audit value */ \
         SET source_asof = ?1 /* dos7-allowed: source-asof cutover lifts implausible audit value */, \
             metadata_json = ?2 \
         WHERE id = ?3",
        params![source_asof, metadata_json, claim_id],
    )?;
    Ok(())
}

fn mark_legacy_unattributed(tx: &ActionDb, claim_id: &str) -> Result<(), BackfillError> {
    tx.conn_ref().execute(
        "UPDATE intelligence_claims /* dos7-allowed: source-asof cutover marks legacy unattributed rows */ \
         SET data_source = 'legacy_unattributed' \
         WHERE id = ?1",
        params![claim_id],
    )?;
    Ok(())
}

fn insert_quarantine(
    tx: &ActionDb,
    row: &LegacyClaimRow,
    metadata: &Value,
    mechanism: Option<&str>,
    raw_sourced_at: &str,
    reason: &str,
    now: DateTime<Utc>,
) -> Result<(), BackfillError> {
    tx.conn_ref().execute(
        "INSERT OR IGNORE INTO source_asof_backfill_quarantine (
             id, claim_source, legacy_entity_id, legacy_field_path,
             legacy_item_hash, raw_sourced_at, reason, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            &row.id,
            claim_source(mechanism),
            legacy_entity_id(row, metadata),
            legacy_field_path(row, metadata),
            legacy_item_hash(row, metadata),
            raw_sourced_at,
            reason,
            now.to_rfc3339(),
        ],
    )?;
    Ok(())
}

fn parse_optional_json_object(raw: Option<&str>, column: &str) -> Result<Value, BackfillError> {
    match raw {
        Some(value) if !value.trim().is_empty() => parse_json_object(value, column),
        _ => Ok(serde_json::json!({})),
    }
}

fn parse_required_json_object(raw: &str, column: &str) -> Result<Value, BackfillError> {
    parse_json_object(raw, column)
}

fn parse_json_object(raw: &str, column: &str) -> Result<Value, BackfillError> {
    let value = serde_json::from_str::<Value>(raw)
        .map_err(|e| BackfillError::Mode(format!("{column} is not valid JSON: {e}")))?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(BackfillError::Mode(format!(
            "{column} is not a JSON object"
        )))
    }
}

fn read_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn backfill_mechanism(metadata: &Value, provenance: &Value) -> Option<String> {
    read_string(metadata, "backfill_mechanism")
        .or_else(|| read_string(provenance, "backfill_mechanism"))
}

fn metadata_indicates_known_timestamp_source(mechanism: Option<&str>, metadata: &Value) -> bool {
    if read_string(metadata, "source_timestamp_source").is_some() {
        return true;
    }

    matches!(
        mechanism,
        Some(
            "suppression_tombstones"
                | "account_stakeholder_roles"
                | "email_dismissals"
                | "meeting_entity_dismissals"
                | "linking_dismissals"
                | "briefing_callouts"
                | "nudge_dismissals"
                | "triage_snoozes"
        )
    )
}

fn source_timestamp_candidate(
    row: &LegacyClaimRow,
    metadata: &Value,
    provenance: &Value,
    mechanism: Option<&str>,
    workspace_root: &Path,
) -> Option<String> {
    if is_dismissed_item_json(mechanism, provenance) {
        return m9_item_source_sourced_at(row, metadata, workspace_root);
    }

    read_string(metadata, "raw_sourced_at").or_else(|| {
        metadata_indicates_known_timestamp_source(mechanism, metadata)
            .then(|| row.observed_at.clone())
    })
}

fn is_dismissed_item_json(mechanism: Option<&str>, provenance: &Value) -> bool {
    matches!(mechanism, Some("dismissed_item_json"))
        || read_string(provenance, "source_table").as_deref() == Some("intelligence.json")
}

fn m9_item_source_sourced_at(
    row: &LegacyClaimRow,
    metadata: &Value,
    workspace_root: &Path,
) -> Option<String> {
    let field = read_string(metadata, "field").or_else(|| row.field_path.clone())?;
    let content = read_string(metadata, "content")?;
    let (subject_kind, subject_id) = subject_kind_and_id(row, metadata)?;
    let intel = read_subject_intelligence_json(workspace_root, &subject_kind, &subject_id)?;
    sourced_at_for_dismissed_content(&intel, &field, &content)
}

fn subject_kind_and_id(row: &LegacyClaimRow, metadata: &Value) -> Option<(String, String)> {
    let subject = serde_json::from_str::<Value>(&row.subject_ref).ok();
    let kind = subject
        .as_ref()
        .and_then(|value| read_string(value, "kind"))
        .or_else(|| read_string(metadata, "entity_type"))?;
    let id = subject
        .as_ref()
        .and_then(|value| read_string(value, "id"))
        .or_else(|| read_string(metadata, "entity_id"))?;
    Some((kind.to_ascii_lowercase(), id))
}

fn read_subject_intelligence_json(
    workspace_root: &Path,
    subject_kind: &str,
    subject_id: &str,
) -> Option<IntelligenceJson> {
    let dir_name = match subject_kind {
        "account" | "accounts" => "Accounts",
        "person" | "people" => "People",
        "project" | "projects" => "Projects",
        _ => return None,
    };
    let kind_root = workspace_root.join(dir_name);
    let direct = kind_root.join(subject_id);
    if direct.join("intelligence.json").is_file() {
        if let Ok(intel) = read_intelligence_json(&direct) {
            return Some(intel);
        }
    }

    let entries = std::fs::read_dir(kind_root).ok()?;
    for entry in entries.flatten() {
        let entity_dir = entry.path();
        if !entity_dir.join("intelligence.json").is_file() {
            continue;
        }
        let Ok(intel) = read_intelligence_json(&entity_dir) else {
            continue;
        };
        if intel.entity_id == subject_id
            || entity_dir
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == subject_id)
        {
            return Some(intel);
        }
    }
    None
}

fn sourced_at_for_dismissed_content(
    intel: &IntelligenceJson,
    field: &str,
    content: &str,
) -> Option<String> {
    match field {
        "risks" => intel
            .risks
            .iter()
            .find(|item| text_matches_dismissed_content(&item.text, content))
            .and_then(|item| sourced_at(item.item_source.as_ref())),
        "recentWins" | "recent_wins" => intel
            .recent_wins
            .iter()
            .find(|item| text_matches_dismissed_content(&item.text, content))
            .and_then(|item| sourced_at(item.item_source.as_ref())),
        "stakeholderInsights" | "stakeholder_insights" => intel
            .stakeholder_insights
            .iter()
            .find(|item| text_matches_dismissed_content(&item.name, content))
            .and_then(|item| sourced_at(item.item_source.as_ref())),
        "valueDelivered" | "value_delivered" => intel
            .value_delivered
            .iter()
            .find(|item| text_matches_dismissed_content(&item.statement, content))
            .and_then(|item| sourced_at(item.item_source.as_ref())),
        "competitiveContext" | "competitive_context" => intel
            .competitive_context
            .iter()
            .find(|item| text_matches_dismissed_content(&item.competitor, content))
            .and_then(|item| sourced_at(item.item_source.as_ref())),
        "organizationalChanges" | "organizational_changes" => intel
            .organizational_changes
            .iter()
            .find(|item| text_matches_dismissed_content(&item.person, content))
            .and_then(|item| sourced_at(item.item_source.as_ref())),
        "expansionSignals" | "expansion_signals" => intel
            .expansion_signals
            .iter()
            .find(|item| text_matches_dismissed_content(&item.opportunity, content))
            .and_then(|item| sourced_at(item.item_source.as_ref())),
        "openCommitments" | "open_commitments" => intel
            .open_commitments
            .as_deref()
            .unwrap_or_default()
            .iter()
            .find(|item| text_matches_dismissed_content(&item.description, content))
            .and_then(|item| sourced_at(item.item_source.as_ref())),
        _ => None,
    }
}

fn text_matches_dismissed_content(candidate: &str, content: &str) -> bool {
    let candidate = candidate.trim();
    let content = content.trim();
    if candidate.is_empty() || content.is_empty() {
        return false;
    }
    candidate.eq_ignore_ascii_case(content)
        || candidate
            .to_ascii_lowercase()
            .contains(&content.to_ascii_lowercase())
}

fn sourced_at(source: Option<&ItemSource>) -> Option<String> {
    source
        .map(|source| source.sourced_at.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn claim_source(mechanism: Option<&str>) -> String {
    let Some(mechanism) = mechanism else {
        return "unknown_legacy_backfill".to_string();
    };

    let migration = match mechanism {
        "suppression_tombstones"
        | "account_stakeholder_roles"
        | "email_dismissals"
        | "meeting_entity_dismissals" => "migration_130",
        "linking_dismissals" | "briefing_callouts" | "nudge_dismissals" | "triage_snoozes" => {
            "migration_131"
        }
        "dismissed_item_json" => "cutover",
        _ => "unknown",
    };
    format!("{migration}_{mechanism}")
}

fn legacy_entity_id(row: &LegacyClaimRow, metadata: &Value) -> String {
    serde_json::from_str::<Value>(&row.subject_ref)
        .ok()
        .and_then(|value| read_string(&value, "id"))
        .or_else(|| read_string(metadata, "entity_id"))
        .or_else(|| read_string(metadata, "owner_id"))
        .or_else(|| read_string(metadata, "account_id"))
        .unwrap_or_else(|| "<unknown>".to_string())
}

fn legacy_field_path(row: &LegacyClaimRow, metadata: &Value) -> String {
    row.field_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| read_string(metadata, "field"))
        .or_else(|| read_string(metadata, "item_type"))
        .or_else(|| read_string(metadata, "entity_type"))
        .unwrap_or_default()
}

fn legacy_item_hash(row: &LegacyClaimRow, metadata: &Value) -> Option<String> {
    row.item_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| read_string(metadata, "item_hash"))
}

fn malformed_reason_label(reason: SourceTimestampMalformedReason) -> &'static str {
    match reason {
        SourceTimestampMalformedReason::Unparseable => "unparseable",
        SourceTimestampMalformedReason::MissingTimezone => "missing_timezone",
        SourceTimestampMalformedReason::BeforeMinimumPlausibleDate => "before_2015",
        SourceTimestampMalformedReason::FarFuture => "far_future",
    }
}

fn implausible_reason_label(reason: SourceTimestampImplausibleReason) -> &'static str {
    match reason {
        SourceTimestampImplausibleReason::BeforeEntityOrigin => "before_entity_origin",
        SourceTimestampImplausibleReason::NearFuture => "near_future",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};
    use chrono::TimeZone;
    use rusqlite::Connection;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap()
    }

    fn fixture_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn fresh_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../migrations/129_dos_7_claims_schema.sql"))
            .unwrap();
        conn.execute_batch(include_str!(
            "../migrations/136_dos_299_source_asof_quarantine.sql"
        ))
        .unwrap();
        conn
    }

    fn seed_claim(
        db: &ActionDb,
        id: &str,
        observed_at: &str,
        mechanism: Option<&str>,
        metadata: Value,
    ) {
        seed_claim_for_subject(
            db,
            id,
            r#"{"kind":"Account","id":"acct-1"}"#,
            observed_at,
            mechanism,
            metadata,
        );
    }

    fn seed_claim_for_subject(
        db: &ActionDb,
        id: &str,
        subject_ref: &str,
        observed_at: &str,
        mechanism: Option<&str>,
        metadata: Value,
    ) {
        let provenance_json = mechanism
            .map(|mechanism| serde_json::json!({ "backfill_mechanism": mechanism }))
            .unwrap_or_else(|| serde_json::json!({}))
            .to_string();
        let metadata_json = metadata.to_string();
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims /* dos7-allowed: source-asof backfill test seed */ (
                    id, subject_ref, claim_type, field_path, text, dedup_key, item_hash,
                    actor, data_source, observed_at, created_at,
                    provenance_json, metadata_json,
                    claim_state, surfacing_state, retraction_reason,
                    temporal_scope, sensitivity
                 ) VALUES (
                    ?1, ?2, 'risk', 'risks',
                    'risk text', ?1, 'hash-1', 'system_backfill', 'legacy_dismissal',
                    ?3, ?3, ?4, ?5, 'tombstoned', 'active', 'user_removal',
                    'state', 'internal'
                 )",
                params![id, subject_ref, observed_at, provenance_json, metadata_json],
            )
            .unwrap();
    }

    fn run_with_workspace(
        db: &ActionDb,
        workspace_root: &Path,
    ) -> Result<BackfillSummary, BackfillError> {
        let clock = FixedClock::new(now());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);
        backfill_source_asof_for_legacy_claims(&ctx, db, workspace_root, now())
    }

    fn run(db: &ActionDb) -> Result<BackfillSummary, BackfillError> {
        let workspace = tempfile::tempdir().expect("workspace");
        run_with_workspace(db, workspace.path())
    }

    #[test]
    fn backfill_lifts_item_source_sourced_at_to_source_asof() {
        let conn = fresh_db();
        let db = ActionDb::from_conn(&conn);
        let workspace = tempfile::tempdir().expect("workspace");
        let entity_dir = workspace.path().join("Accounts").join("Account Fixture");
        std::fs::create_dir_all(&entity_dir).unwrap();
        std::fs::write(
            entity_dir.join("intelligence.json"),
            serde_json::json!({
                "version": 1,
                "entityId": "account-fixture",
                "entityType": "account",
                "risks": [{
                    "text": "Renewal blocker",
                    "itemSource": {
                        "source": "meeting",
                        "confidence": 0.8,
                        "sourcedAt": "2026-04-10T09:30:00Z"
                    }
                }],
                "dismissedItems": [{
                    "field": "risks",
                    "content": "Renewal blocker",
                    "dismissedAt": "2026-04-15T00:00:00Z"
                }]
            })
            .to_string(),
        )
        .unwrap();
        seed_claim_for_subject(
            db,
            "m1-1",
            r#"{"kind":"Account","id":"account-fixture"}"#,
            "2026-04-15T00:00:00Z",
            Some("dismissed_item_json"),
            serde_json::json!({
                "field": "risks",
                "content": "Renewal blocker",
                "dismissed_at": "2026-04-15T00:00:00Z"
            }),
        );

        let summary = run_with_workspace(db, workspace.path()).unwrap();

        assert_eq!(summary.accepted, 1);
        assert_eq!(summary.coverage_pct, 1.0);
        let source_asof: String = conn
            .query_row(
                "SELECT source_asof FROM intelligence_claims WHERE id = 'm1-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source_asof, "2026-04-10T09:30:00+00:00");
    }

    #[test]
    fn backfill_implausible_lifts_and_warns() {
        let conn = fresh_db();
        let db = ActionDb::from_conn(&conn);
        for index in 0..20 {
            seed_claim(
                db,
                &format!("m1-ok-{index}"),
                "2026-04-15T00:00:00Z",
                Some("suppression_tombstones"),
                serde_json::json!({ "raw_sourced_at": "2026-04-10T09:30:00Z" }),
            );
        }
        seed_claim(
            db,
            "m1-implausible",
            "2026-04-15T00:00:00Z",
            Some("suppression_tombstones"),
            serde_json::json!({ "raw_sourced_at": "2026-07-01T00:00:00Z" }),
        );

        let summary = run(db).unwrap();

        assert_eq!(summary.accepted, 20);
        assert_eq!(summary.implausible, 1);
        let (source_asof, metadata_json): (String, String) = conn
            .query_row(
                "SELECT source_asof, metadata_json FROM intelligence_claims WHERE id = 'm1-implausible'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(source_asof, "2026-07-01T00:00:00+00:00");
        assert!(metadata_json.contains("source_timestamp_implausible"));
        assert!(metadata_json.contains("near_future"));
    }

    #[test]
    fn backfill_malformed_quarantines_and_halts() {
        let conn = fresh_db();
        let db = ActionDb::from_conn(&conn);
        seed_claim(
            db,
            "m1-bad",
            "2026-04-15T00:00:00Z",
            Some("suppression_tombstones"),
            serde_json::json!({ "raw_sourced_at": "garbleZ" }),
        );

        let err = run(db).unwrap_err();

        assert!(matches!(err, BackfillError::MigrationGate(_)));
        let (count, reason): (i64, String) = conn
            .query_row(
                "SELECT count(*), max(reason) FROM source_asof_backfill_quarantine",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(reason, "unparseable");
    }

    #[test]
    fn backfill_missing_item_source_uses_legacy_unattributed() {
        let conn = fresh_db();
        let db = ActionDb::from_conn(&conn);
        seed_claim(
            db,
            "legacy-missing",
            "2026-04-15T00:00:00Z",
            None,
            serde_json::json!({}),
        );

        let summary = run(db).unwrap();

        assert_eq!(summary.missing_item_source, 1);
        assert_eq!(summary.coverage_pct, 1.0);
        let data_source: String = conn
            .query_row(
                "SELECT data_source FROM intelligence_claims WHERE id = 'legacy-missing'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(data_source, "legacy_unattributed");
    }

    #[test]
    fn backfill_coverage_below_threshold_halts_migration() {
        let conn = fresh_db();
        let db = ActionDb::from_conn(&conn);
        for index in 0..18 {
            seed_claim(
                db,
                &format!("m1-ok-{index}"),
                "2026-04-15T00:00:00Z",
                Some("suppression_tombstones"),
                serde_json::json!({ "raw_sourced_at": "2026-04-10T09:30:00Z" }),
            );
        }
        seed_claim(
            db,
            "m1-implausible",
            "2026-04-15T00:00:00Z",
            Some("suppression_tombstones"),
            serde_json::json!({ "raw_sourced_at": "2026-07-01T00:00:00Z" }),
        );

        let err = run(db).unwrap_err();

        assert!(matches!(
            err,
            BackfillError::MigrationGate(message) if message == "coverage below 95%"
        ));
        let lifted: i64 = conn
            .query_row(
                "SELECT count(*) FROM intelligence_claims WHERE source_asof IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(lifted, 19);
    }

    #[test]
    fn backfill_coverage_above_threshold_succeeds() {
        let conn = fresh_db();
        let db = ActionDb::from_conn(&conn);
        for index in 0..19 {
            seed_claim(
                db,
                &format!("m1-ok-{index}"),
                "2026-04-15T00:00:00Z",
                Some("suppression_tombstones"),
                serde_json::json!({ "raw_sourced_at": "2026-04-10T09:30:00Z" }),
            );
        }
        seed_claim(
            db,
            "m1-implausible",
            "2026-04-15T00:00:00Z",
            Some("suppression_tombstones"),
            serde_json::json!({ "raw_sourced_at": "2026-07-01T00:00:00Z" }),
        );

        let summary = run(db).unwrap();

        assert_eq!(summary.accepted, 19);
        assert_eq!(summary.implausible, 1);
        assert_eq!(summary.coverage_pct, 0.95);
    }
}

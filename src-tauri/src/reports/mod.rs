//! Report infrastructure for v0.15.0.
//!
//! Manages the reports table: SWOT analyses, Account Health Reviews,
//! EBR/QBR, Weekly Impact, Monthly Wrapped.
//!
//! Pattern: two-phase pipeline (gather input under brief DB lock,
//! run PTY without lock), same as risk_briefing.rs.

pub mod account_health;
pub mod book_of_business;
pub mod ebr_qbr;
pub mod generator;
pub mod invalidation;
pub mod monthly_wrapped;
pub mod prompts;
pub mod risk;
pub mod swot;
pub mod weekly_impact;

use chrono::Utc;
use rusqlite::params;
use sha2::{Digest, Sha256};

use crate::db::ActionDb;

// =============================================================================
// Types
// =============================================================================

/// All supported report types. Stored as string in DB.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    Swot,
    AccountHealth,
    EbrQbr,
    WeeklyImpact,
    MonthlyWrapped,
    RiskBriefing,
    BookOfBusiness,
}

impl ReportType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReportType::Swot => "swot",
            ReportType::AccountHealth => "account_health",
            ReportType::EbrQbr => "ebr_qbr",
            ReportType::WeeklyImpact => "weekly_impact",
            ReportType::MonthlyWrapped => "monthly_wrapped",
            ReportType::RiskBriefing => "risk_briefing",
            ReportType::BookOfBusiness => "book_of_business",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "swot" => Some(ReportType::Swot),
            "account_health" => Some(ReportType::AccountHealth),
            "ebr_qbr" => Some(ReportType::EbrQbr),
            "weekly_impact" => Some(ReportType::WeeklyImpact),
            "monthly_wrapped" => Some(ReportType::MonthlyWrapped),
            "risk_briefing" => Some(ReportType::RiskBriefing),
            "book_of_business" => Some(ReportType::BookOfBusiness),
            _ => None,
        }
    }
}

/// A report row from the DB.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRow {
    pub id: String,
    pub entity_id: String,
    pub entity_type: String,
    pub report_type: String,
    pub content_json: String,
    pub generated_at: String,
    pub intel_hash: String,
    pub is_stale: bool,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Intel hash computation
// =============================================================================

/// Compute a hash of the current intelligence state for an entity.
/// Used to detect when intelligence changed and cached reports are stale.
pub fn compute_intel_hash(entity_id: &str, entity_type: &str, db: &ActionDb) -> String {
    // Hash the entity_assessment row's enriched_at + a content sample
    let intel_str: String = db
        .conn_ref()
        .query_row(
            "SELECT COALESCE(enriched_at, '') || '|' || COALESCE(executive_assessment, '') FROM entity_assessment WHERE entity_id = ?1",
            params![entity_id],
            |row| row.get(0),
        )
        .unwrap_or_default();

    let input = format!("{}:{}:{}", entity_id, entity_type, intel_str);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

/// Compute an aggregate hash across all active accounts' intelligence state.
/// Used for the Book of Business report to detect when any account intelligence changed.
pub fn compute_aggregate_intel_hash(db: &ActionDb) -> String {
    let concat: String = db
        .conn_ref()
        .query_row(
            "SELECT GROUP_CONCAT(COALESCE(ea.enriched_at, ''), '|')
             FROM accounts a
             LEFT JOIN entity_assessment ea ON ea.entity_id = a.id
             WHERE a.archived = 0
             ORDER BY a.id",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    let mut hasher = Sha256::new();
    hasher.update(concat.as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

// =============================================================================
// DB read/write
// =============================================================================

/// Fetch a single report by entity + type.
pub fn get_report(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    report_type: &str,
) -> Result<Option<ReportRow>, String> {
    let result = db
        .conn_ref()
        .query_row(
            "SELECT id, entity_id, entity_type, report_type, content_json, generated_at,
                    intel_hash, is_stale, created_at, updated_at
             FROM reports
             WHERE entity_id = ?1 AND entity_type = ?2 AND report_type = ?3",
            params![entity_id, entity_type, report_type],
            |row| {
                Ok(ReportRow {
                    id: row.get(0)?,
                    entity_id: row.get(1)?,
                    entity_type: row.get(2)?,
                    report_type: row.get(3)?,
                    content_json: row.get(4)?,
                    generated_at: row.get(5)?,
                    intel_hash: row.get(6)?,
                    is_stale: row.get::<_, i64>(7)? != 0,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .map(Some)
        .or_else(|e| {
            if e == rusqlite::Error::QueryReturnedNoRows {
                Ok(None)
            } else {
                Err(format!("Failed to query report: {}", e))
            }
        })?;
    Ok(result)
}

/// Fetch all reports for an entity.
pub fn get_reports_for_entity(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
) -> Result<Vec<ReportRow>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT id, entity_id, entity_type, report_type, content_json, generated_at,
                    intel_hash, is_stale, created_at, updated_at
             FROM reports
             WHERE entity_id = ?1 AND entity_type = ?2
             ORDER BY generated_at DESC",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let rows = stmt
        .query_map(params![entity_id, entity_type], |row| {
            Ok(ReportRow {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                entity_type: row.get(2)?,
                report_type: row.get(3)?,
                content_json: row.get(4)?,
                generated_at: row.get(5)?,
                intel_hash: row.get(6)?,
                is_stale: row.get::<_, i64>(7)? != 0,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| format!("Failed to query reports: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect reports: {}", e))
}

/// Insert or replace a report.
pub fn upsert_report(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    report_type: &str,
    content_json: &str,
    intel_hash: &str,
) -> Result<String, String> {
    let id = format!("{}-{}-{}", entity_id, entity_type, report_type);
    let now = Utc::now().to_rfc3339();

    db.conn_ref()
        .execute(
            "INSERT INTO reports (id, entity_id, entity_type, report_type, content_json, generated_at, intel_hash, is_stale, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?6)
             ON CONFLICT(entity_id, entity_type, report_type) DO UPDATE SET
               content_json = excluded.content_json,
               generated_at = excluded.generated_at,
               intel_hash = excluded.intel_hash,
               is_stale = 0,
               updated_at = excluded.updated_at",
            params![id, entity_id, entity_type, report_type, content_json, now, intel_hash],
        )
        .map_err(|e| format!("Failed to upsert report: {}", e))?;

    Ok(id)
}

/// Update content_json for an existing report (user edits).
pub fn save_report_content(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    report_type: &str,
    content_json: &str,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    db.conn_ref()
        .execute(
            "UPDATE reports SET content_json = ?1, updated_at = ?2 WHERE entity_id = ?3 AND entity_type = ?4 AND report_type = ?5",
            rusqlite::params![content_json, now, entity_id, entity_type, report_type],
        )
        .map_err(|e| format!("Failed to save report: {}", e))?;
    Ok(())
}

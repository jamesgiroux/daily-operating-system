//! Risk briefing adapter for the reports infrastructure.
//!
//! Wraps the existing risk_briefing.rs gather/run pipeline and stores
//! results in the reports table alongside other report types.

use crate::db::ActionDb;
use crate::reports::{compute_intel_hash, upsert_report};
use crate::types::RiskBriefing;

/// After generating a risk briefing via risk_briefing.rs, store it
/// in the reports table for unified tracking.
pub fn store_risk_briefing_in_reports(
    db: &ActionDb,
    account_id: &str,
    briefing: &RiskBriefing,
) -> Result<(), String> {
    let content_json = serde_json::to_string(briefing)
        .map_err(|e| format!("Failed to serialize risk briefing: {}", e))?;

    let intel_hash = compute_intel_hash(account_id, "account", db);

    upsert_report(
        db,
        account_id,
        "account",
        "risk_briefing",
        &content_json,
        &intel_hash,
    )?;

    log::debug!("reports: stored risk_briefing for account {}", account_id);
    Ok(())
}

/// Try to load a risk briefing from the reports table.
/// Returns None if not found or if content can't be parsed.
pub fn load_risk_briefing_from_reports(db: &ActionDb, account_id: &str) -> Option<RiskBriefing> {
    let report =
        crate::reports::get_report(db, account_id, "account", "risk_briefing").ok()??;

    serde_json::from_str::<RiskBriefing>(&report.content_json)
        .map_err(|e| {
            log::warn!(
                "reports: failed to parse risk_briefing from reports table for {}: {}",
                account_id,
                e
            );
            e
        })
        .ok()
}

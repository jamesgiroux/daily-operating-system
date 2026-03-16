//! Book of Business report (I547).
//!
//! Cross-account portfolio report. Gathers all active accounts with health,
//! ARR, renewal data, and activity metrics. AI generates narrative analysis;
//! metrics and snapshot are pre-computed from DB data.
//!
//! I547: Parallel generation — 6 Wave 1 sections + sequential executiveSummary.
//! Optional Glean pre-fetch injects enterprise context into section prompts.

use std::path::PathBuf;
use std::time::Instant;

use chrono::{Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::db::ActionDb;
use crate::pty::{ModelTier, PtyManager};
use crate::reports::compute_aggregate_intel_hash;
use crate::reports::generator::ReportGeneratorInput;
use crate::reports::prompts::build_report_preamble;
use crate::types::AiModelConfig;

// =============================================================================
// Output schema
// =============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookOfBusinessContent {
    pub period_label: String,
    pub total_accounts: i32,
    pub total_arr: f64,
    pub at_risk_arr: f64,
    pub upcoming_renewals: i32,
    pub upcoming_renewals_arr: f64,
    pub executive_summary: String,
    pub top_risks: Vec<BookRiskItem>,
    pub top_opportunities: Vec<BookOpportunityItem>,
    pub account_snapshot: Vec<AccountSnapshotRow>,
    pub deep_dives: Vec<AccountDeepDive>,
    pub value_delivered: Vec<ValueDeliveredRow>,
    pub key_themes: Vec<BookTheme>,
    pub leadership_asks: Vec<LeadershipAsk>,
    pub has_leadership_asks: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookRiskItem {
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub risk: String,
    #[serde(default)]
    pub arr: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookOpportunityItem {
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub opportunity: String,
    #[serde(default)]
    pub estimated_value: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSnapshotRow {
    pub account_id: String,
    pub account_name: String,
    pub health_band: Option<String>,
    pub health_trend: Option<String>,
    pub health_score: Option<f64>,
    pub arr: Option<f64>,
    pub lifecycle: Option<String>,
    pub renewal_date: Option<String>,
    pub meeting_count_90d: i32,
    pub key_contact: Option<String>,
    #[serde(default)]
    pub is_parent: bool,
    #[serde(default)]
    pub bu_count: Option<u32>,
    #[serde(default)]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDeepDive {
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub arr: Option<f64>,
    #[serde(default)]
    pub renewal_date: Option<String>,
    #[serde(default)]
    pub status_narrative: String,
    #[serde(default)]
    pub renewal_or_growth_impact: String,
    #[serde(default)]
    pub active_workstreams: Vec<String>,
    #[serde(default)]
    pub risks_and_gaps: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueDeliveredRow {
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub headline_outcome: String,
    #[serde(default)]
    pub why_it_matters: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookTheme {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub narrative: String,
    #[serde(default)]
    pub cited_accounts: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeadershipAsk {
    #[serde(default)]
    pub ask: String,
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub impacted_accounts: Vec<String>,
    #[serde(default)]
    pub status: Option<String>,
}

// =============================================================================
// Pre-computed metrics (passed through extra_data)
// =============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookMetrics {
    pub period_label: String,
    pub total_accounts: i32,
    pub total_arr: f64,
    pub at_risk_arr: f64,
    pub upcoming_renewals: i32,
    pub upcoming_renewals_arr: f64,
    pub account_snapshot: Vec<AccountSnapshotRow>,
}

// =============================================================================
// I547: Structured gather output for parallel generation
// =============================================================================

/// Raw data gathered from DB — used to build per-section prompts.
/// Separates data gathering from prompt construction so sections can
/// be generated in parallel with section-specific prompts.
#[derive(Clone)]
pub struct BookGatherOutput {
    pub workspace: PathBuf,
    pub ai_models: AiModelConfig,
    pub intel_hash: String,
    pub user_entity_id: String,
    pub user_name: String,
    pub user_role: String,
    pub active_preset: String,
    pub metrics: BookMetrics,
    pub raw_accounts: Vec<RawAccountRow>,
    pub snapshot: Vec<AccountSnapshotRow>,
    pub open_actions: String,
    pub email_signals: String,
    pub captures: String,
    pub spotlight_ids: Vec<String>,
    /// Pre-computed user context block (I413 semantic search results).
    /// Injected into every section prompt so the parallel path matches
    /// the monolithic path's user context quality.
    pub user_context_block: String,
}

/// I547: Glean pre-fetched portfolio context. Each field is `None` on
/// timeout/error (non-fatal). Sections generate from local DB data only
/// when Glean context is unavailable.
#[derive(Debug, Clone, Default)]
pub struct GleanPortfolioContext {
    pub risk_pulse: Option<String>,
    pub pipeline_signals: Option<String>,
    pub themes: Option<String>,
}

/// I547: Progressive event emitted per section completion.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BobSectionProgress {
    pub section_name: String,
    pub completed: u32,
    pub total: u32,
    pub wave: u32,
}

// =============================================================================
// Helpers
// =============================================================================

/// Resolve health band: user-set manual health is the primary indicator,
/// falling back to computed intelligence score when no manual health exists.
fn resolve_health_band(manual_health: Option<&str>, score: Option<f64>) -> Option<String> {
    // Manual health (green/yellow/red) takes priority
    match manual_health {
        Some("red") => return Some("at-risk".to_string()),
        Some("yellow") => return Some("watch".to_string()),
        Some("green") => return Some("healthy".to_string()),
        _ => {}
    }
    // Fall back to computed score
    match score {
        Some(s) if s >= 70.0 => Some("healthy".to_string()),
        Some(s) if s >= 40.0 => Some("watch".to_string()),
        Some(_) => Some("at-risk".to_string()),
        None => None,
    }
}

fn is_within_90_days(renewal_date: &Option<String>) -> bool {
    let date_str = match renewal_date {
        Some(d) if !d.is_empty() => d,
        _ => return false,
    };
    let today = Utc::now().date_naive();
    let cutoff = today + Duration::days(90);
    // Parse YYYY-MM-DD prefix
    let parsed = date_str
        .get(..10)
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    match parsed {
        Some(d) => d >= today && d <= cutoff,
        None => false,
    }
}

/// Truncate a string to at most `max_chars`, appending "..." if truncated.
fn truncate(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        s.to_string()
    } else {
        let boundary = s
            .char_indices()
            .nth(max_chars)
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        format!("{}...", &s[..boundary])
    }
}

/// Extract the first paragraph from a string (up to first double-newline or full string).
fn first_paragraph(s: &str, max_chars: usize) -> String {
    let para = s.split("\n\n").next().unwrap_or(s);
    truncate(para, max_chars)
}

// =============================================================================
// Data gathering (Phase 1) — I547 refactored
// =============================================================================

/// Internal struct to hold raw account data from the DB before building snapshot rows.
/// Pulls rich intelligence from entity_assessment — the output of the Intelligence Loop.
#[derive(Clone)]
pub struct RawAccountRow {
    pub id: String,
    pub name: String,
    pub arr: Option<f64>,
    pub contract_end: Option<String>,
    pub lifecycle: Option<String>,
    pub executive_assessment: Option<String>,
    pub health_score: Option<f64>,
    pub health_trend: Option<String>,
    pub parent_id: Option<String>,
    /// User-set health RAG: "green", "yellow", "red" (primary at-risk indicator)
    pub manual_health: Option<String>,
    // Rich intelligence fields from entity_assessment (Intelligence Loop output)
    pub risks_json: Option<String>,
    pub recent_wins_json: Option<String>,
    pub stakeholder_insights_json: Option<String>,
    pub value_delivered: Option<String>,
    pub open_commitments: Option<String>,
    pub current_state_json: Option<String>,
}

/// I547: Gather raw portfolio data from DB. Returns structured output
/// that can be used to build per-section prompts for parallel generation.
pub fn gather_book_of_business_data(
    workspace: &std::path::Path,
    db: &ActionDb,
    ai_models: AiModelConfig,
    active_preset: &str,
    spotlight_account_ids: Option<&[String]>,
) -> Result<BookGatherOutput, String> {
    // 1. All active customer accounts with health/ARR/renewal + rich intelligence
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT a.id, a.name, a.arr, a.contract_end, a.lifecycle,
                    ea.executive_assessment, eq.health_score, eq.health_trend,
                    a.parent_id, a.health,
                    ea.risks_json, ea.recent_wins_json, ea.stakeholder_insights_json,
                    ea.value_delivered, ea.open_commitments, ea.current_state_json
             FROM accounts a
             LEFT JOIN entity_assessment ea ON ea.entity_id = a.id
             LEFT JOIN entity_quality eq ON eq.entity_id = a.id
             WHERE a.archived = 0
               AND COALESCE(a.account_type, 'customer') = 'customer'
             ORDER BY COALESCE(a.arr, 0) DESC",
        )
        .map_err(|e| format!("Failed to prepare accounts query: {}", e))?;

    let raw_accounts: Vec<RawAccountRow> = stmt
        .query_map([], |row| {
            Ok(RawAccountRow {
                id: row.get(0)?,
                name: row.get(1)?,
                arr: row.get(2)?,
                contract_end: row.get(3)?,
                lifecycle: row.get(4)?,
                executive_assessment: row.get(5)?,
                health_score: row.get(6)?,
                health_trend: row.get(7)?,
                parent_id: row.get(8)?,
                manual_health: row.get(9)?,
                risks_json: row.get(10)?,
                recent_wins_json: row.get(11)?,
                stakeholder_insights_json: row.get(12)?,
                value_delivered: row.get(13)?,
                open_commitments: row.get(14)?,
                current_state_json: row.get(15)?,
            })
        })
        .map_err(|e| format!("Failed to query accounts: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    // 2. Primary stakeholder per account (from account_stakeholders table)
    let mut primary_contact_stmt = db
        .conn_ref()
        .prepare(
            "SELECT p.name FROM account_stakeholders ast
             JOIN people p ON p.id = ast.person_id
             WHERE ast.account_id = ?1
             ORDER BY CASE ast.role
               WHEN 'champion' THEN 1
               WHEN 'exec_sponsor' THEN 2
               WHEN 'tam' THEN 3
               WHEN 'csm' THEN 4
               ELSE 5
             END
             LIMIT 1",
        )
        .map_err(|e| format!("Failed to prepare primary contact query: {}", e))?;

    // 3. Meeting counts per account (90d)
    let ninety_days_ago = (Utc::now() - Duration::days(90))
        .format("%Y-%m-%d")
        .to_string();

    let mut meeting_count_stmt = db
        .conn_ref()
        .prepare(
            "SELECT COUNT(*) FROM meeting_entities me
             JOIN meetings m ON m.id = me.meeting_id
             WHERE me.entity_id = ?1 AND me.entity_type = 'account'
               AND m.start_time >= ?2",
        )
        .map_err(|e| format!("Failed to prepare meeting count query: {}", e))?;

    // Build hierarchy: identify which accounts are parents (have children)
    let mut children_of: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    for (i, acct) in raw_accounts.iter().enumerate() {
        if let Some(pid) = &acct.parent_id {
            children_of.entry(pid.clone()).or_default().push(i);
        }
    }

    // Build snapshot rows
    let mut snapshot: Vec<AccountSnapshotRow> = Vec::with_capacity(raw_accounts.len());
    for acct in &raw_accounts {
        let key_contact: Option<String> = primary_contact_stmt
            .query_row(rusqlite::params![acct.id], |row| row.get(0))
            .ok();

        let meeting_count_90d: i32 = meeting_count_stmt
            .query_row(rusqlite::params![acct.id, ninety_days_ago], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap_or(0) as i32;

        let health_band = resolve_health_band(acct.manual_health.as_deref(), acct.health_score);
        let is_parent = children_of.contains_key(&acct.id);
        let bu_count = children_of.get(&acct.id).map(|c| c.len() as u32);

        snapshot.push(AccountSnapshotRow {
            account_id: acct.id.clone(),
            account_name: acct.name.clone(),
            health_band,
            health_trend: acct.health_trend.clone(),
            health_score: acct.health_score,
            arr: acct.arr,
            lifecycle: acct.lifecycle.clone(),
            renewal_date: acct.contract_end.clone(),
            meeting_count_90d,
            key_contact,
            is_parent,
            bu_count,
            parent_id: acct.parent_id.clone(),
        });
    }

    // 4. Top 20 open actions across customer accounts
    let open_actions: String = db
        .conn_ref()
        .prepare(
            "SELECT act.title, a.name FROM actions act
             JOIN accounts a ON a.id = act.entity_id
             WHERE act.status = 'open' AND act.entity_type = 'account'
               AND a.archived = 0
               AND COALESCE(a.account_type, 'customer') = 'customer'
             ORDER BY act.due_date ASC NULLS LAST
             LIMIT 20",
        )
        .and_then(|mut s| {
            let rows = s.query_map([], |row| {
                let title: String = row.get(0)?;
                let acct_name: String = row.get::<_, Option<String>>(1)?.unwrap_or_default();
                Ok(format!("- [{}] {}", acct_name, title))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    // 5. Email signal counts per customer account (recent 90d)
    let email_signals: String = db
        .conn_ref()
        .prepare(
            "SELECT a.name, COUNT(*) as cnt
             FROM signal_events se
             JOIN accounts a ON a.id = se.entity_id
             WHERE se.entity_type = 'account'
               AND se.signal_type LIKE '%email%'
               AND se.created_at >= ?1
               AND a.archived = 0
               AND COALESCE(a.account_type, 'customer') = 'customer'
             GROUP BY se.entity_id
             ORDER BY cnt DESC
             LIMIT 20",
        )
        .and_then(|mut s| {
            let rows = s.query_map(rusqlite::params![ninety_days_ago], |row| {
                let name: String = row.get(0)?;
                let cnt: i64 = row.get(1)?;
                Ok(format!("- {} ({} email signals)", name, cnt))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    // 6. User entity context
    let (user_name, user_role): (String, String) = db
        .conn_ref()
        .query_row(
            "SELECT COALESCE(name, ''), COALESCE(role, '') FROM user_entity LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap_or_else(|_| (String::new(), String::new()));

    // Compute aggregate metrics
    let total_accounts = snapshot.len() as i32;
    let total_arr: f64 = snapshot.iter().filter_map(|s| s.arr).sum();
    let at_risk_arr: f64 = snapshot
        .iter()
        .filter(|s| matches!(s.health_band.as_deref(), Some("at-risk" | "watch")))
        .filter_map(|s| s.arr)
        .sum();
    let upcoming_renewals: i32 = snapshot
        .iter()
        .filter(|s| is_within_90_days(&s.renewal_date))
        .count() as i32;
    let upcoming_renewals_arr: f64 = snapshot
        .iter()
        .filter(|s| is_within_90_days(&s.renewal_date))
        .filter_map(|s| s.arr)
        .sum();

    let now = Utc::now();
    let period_label = format!("{} {}", now.format("%B"), now.year());

    let metrics = BookMetrics {
        period_label,
        total_accounts,
        total_arr,
        at_risk_arr,
        upcoming_renewals,
        upcoming_renewals_arr,
        account_snapshot: snapshot.clone(),
    };

    let intel_hash = compute_aggregate_intel_hash(db);

    let user_entity_id: String = db
        .conn_ref()
        .query_row(
            "SELECT CAST(id AS TEXT) FROM user_entity LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "1".to_string());

    // Aggregate captures across all active customer accounts (90d), urgency-sorted
    let captures: String = {
        let ninety_ago = (Utc::now() - Duration::days(90))
            .format("%Y-%m-%d")
            .to_string();
        db.conn_ref()
            .prepare(
                "SELECT c.capture_type, c.content, c.sub_type, c.urgency, c.impact,
                        c.evidence_quote, a.name as account_name, c.captured_at
                 FROM captures c
                 JOIN accounts a ON a.id = c.account_id
                 WHERE c.captured_at >= ?1
                   AND a.archived = 0
                   AND COALESCE(a.account_type, 'customer') = 'customer'
                 ORDER BY
                   CASE c.urgency WHEN 'red' THEN 1 WHEN 'yellow' THEN 2 WHEN 'green_watch' THEN 3 ELSE 4 END,
                   c.captured_at DESC
                 LIMIT 30",
            )
            .and_then(|mut s| {
                let rows = s.query_map(rusqlite::params![ninety_ago], |row| {
                    let ctype: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    let sub_type: Option<String> = row.get(2)?;
                    let urgency: Option<String> = row.get(3)?;
                    let _impact: Option<String> = row.get(4)?;
                    let quote: Option<String> = row.get(5)?;
                    let acct_name: String = row.get(6)?;
                    let captured: String = row.get(7)?;
                    let date = captured.split('T').next().unwrap_or(&captured).to_string();
                    let urg = urgency.map(|u| format!("[{}] ", u)).unwrap_or_default();
                    let sub = sub_type.map(|s| format!("[{}] ", s)).unwrap_or_default();
                    let q = quote.map(|q| format!(" #\"{}\"", q)).unwrap_or_default();
                    Ok(format!(
                        "- [{}] {}{}{}: {} ({}){}",
                        acct_name, urg, sub, ctype, content, date, q
                    ))
                })?;
                Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
            })
            .unwrap_or_default()
    };

    let spotlight_ids = spotlight_account_ids
        .map(|ids| ids.to_vec())
        .unwrap_or_default();

    Ok(BookGatherOutput {
        workspace: workspace.to_path_buf(),
        ai_models,
        intel_hash,
        user_entity_id,
        user_name,
        user_role,
        active_preset: active_preset.to_string(),
        metrics,
        raw_accounts,
        snapshot,
        open_actions,
        email_signals,
        captures,
        spotlight_ids,
        user_context_block: String::new(), // Populated by service layer
    })
}

/// Thin adapter: convert `BookGatherOutput` to `ReportGeneratorInput` for
/// the monolithic fallback path (used when parallel generation fails).
pub fn gather_to_report_input(gather: &BookGatherOutput) -> Result<ReportGeneratorInput, String> {
    let prompt = build_book_of_business_prompt(
        &gather.raw_accounts,
        &gather.snapshot,
        &gather.open_actions,
        &gather.email_signals,
        &gather.captures,
        &gather.user_name,
        &gather.user_role,
        &gather.active_preset,
        &gather.metrics.period_label,
        &gather.metrics,
        if gather.spotlight_ids.is_empty() {
            None
        } else {
            Some(&gather.spotlight_ids)
        },
    );

    let extra_data = serde_json::to_string(&gather.metrics)
        .map_err(|e| format!("Failed to serialize BookMetrics: {}", e))?;

    Ok(ReportGeneratorInput {
        entity_id: gather.user_entity_id.clone(),
        entity_type: "user".to_string(),
        report_type: "book_of_business".to_string(),
        entity_name: "Book of Business".to_string(),
        workspace: gather.workspace.clone(),
        prompt,
        ai_models: gather.ai_models.clone(),
        intel_hash: gather.intel_hash.clone(),
        extra_data: Some(extra_data),
    })
}

/// Legacy entry point — gathers data AND builds the monolithic prompt.
/// Kept for backward compatibility with the existing service layer path.
pub fn gather_book_of_business_input(
    workspace: &std::path::Path,
    db: &ActionDb,
    ai_models: AiModelConfig,
    active_preset: &str,
    spotlight_account_ids: Option<&[String]>,
) -> Result<ReportGeneratorInput, String> {
    let gather = gather_book_of_business_data(workspace, db, ai_models, active_preset, spotlight_account_ids)?;
    gather_to_report_input(&gather)
}

// =============================================================================
// I547: Shared prompt building blocks
// =============================================================================

/// Build the shared preamble + portfolio context block used by all sections.
fn build_portfolio_context_block(gather: &BookGatherOutput) -> String {
    let mut prompt = build_report_preamble("Portfolio", "book_of_business", "user");

    prompt.push_str(&format!(
        "Role preset: {}. User: {} ({})\n\n",
        crate::util::sanitize_external_field(&gather.active_preset),
        crate::util::sanitize_external_field(&gather.user_name),
        crate::util::sanitize_external_field(&gather.user_role),
    ));

    prompt.push_str(&format!(
        "# Portfolio Overview: {}\n\n\
         Total accounts: {} | Total ARR: ${:.0} | At-risk ARR: ${:.0}\n\
         Upcoming renewals (90d): {} | Upcoming renewal ARR: ${:.0}\n\n",
        gather.metrics.period_label,
        gather.metrics.total_accounts,
        gather.metrics.total_arr,
        gather.metrics.at_risk_arr,
        gather.metrics.upcoming_renewals,
        gather.metrics.upcoming_renewals_arr,
    ));

    prompt
}

/// Build the account data block for prompts. Includes all portfolio accounts
/// with tiered detail (top 10 full intelligence, 11-20 condensed, 20+ minimal).
fn build_account_data_block(
    raw_accounts: &[RawAccountRow],
    snapshot: &[AccountSnapshotRow],
) -> String {
    let mut prompt = String::new();
    prompt.push_str("## Account Details\n\n");

    let accounts_to_emit: Vec<&AccountSnapshotRow> = snapshot.iter().collect();

    let mut emitted: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut tier_idx = 0usize;

    let emit_account = |prompt: &mut String, snap: &AccountSnapshotRow, tier: usize, indent: &str| {
        let raw = raw_accounts
            .iter()
            .find(|r| r.id == snap.account_id);
        let assessment = raw
            .and_then(|r| r.executive_assessment.as_deref())
            .unwrap_or("");

        let arr_str = snap.arr.map(|a| format!("${:.0}", a)).unwrap_or_else(|| "N/A".to_string());
        let renewal_str = snap.renewal_date.as_deref().unwrap_or("N/A");
        let lifecycle_str = snap.lifecycle.as_deref().unwrap_or("N/A");
        let contact_str = snap.key_contact.as_deref().unwrap_or("N/A");
        let band_str = snap.health_band.as_deref().unwrap_or("unknown");

        if tier < 10 {
            // Tier 1: Full intelligence context from the Intelligence Loop
            prompt.push_str(&format!(
                "{}### {} ({})\n",
                indent,
                crate::util::sanitize_external_field(&snap.account_name),
                band_str,
            ));
            prompt.push_str(&format!(
                "{}ARR: {} | Lifecycle: {} | Renewal: {} | Meetings (90d): {} | Contact: {}\n",
                indent, arr_str, lifecycle_str, renewal_str, snap.meeting_count_90d, contact_str,
            ));

            // Executive assessment
            let excerpt = truncate(assessment, 400);
            if !excerpt.is_empty() {
                prompt.push_str(&format!("{}Assessment: ", indent));
                prompt.push_str(&crate::util::wrap_user_data(&excerpt));
                prompt.push('\n');
            }

            // Rich intelligence from entity_assessment (already computed by Intelligence Loop)
            if let Some(r) = raw {
                if let Some(ref risks) = r.risks_json {
                    if risks.len() > 2 { // not empty "[]"
                        prompt.push_str(&format!("{}Known risks: {}\n", indent, truncate(risks, 300)));
                    }
                }
                if let Some(ref wins) = r.recent_wins_json {
                    if wins.len() > 2 {
                        prompt.push_str(&format!("{}Recent wins: {}\n", indent, truncate(wins, 300)));
                    }
                }
                if let Some(ref val) = r.value_delivered {
                    if !val.is_empty() {
                        prompt.push_str(&format!("{}Value delivered: {}\n", indent, truncate(val, 200)));
                    }
                }
                if let Some(ref commits) = r.open_commitments {
                    if !commits.is_empty() {
                        prompt.push_str(&format!("{}Open commitments: {}\n", indent, truncate(commits, 200)));
                    }
                }
                if let Some(ref stakeholders) = r.stakeholder_insights_json {
                    if stakeholders.len() > 2 {
                        prompt.push_str(&format!("{}Stakeholder insights: {}\n", indent, truncate(stakeholders, 300)));
                    }
                }
            }
            prompt.push('\n');
        } else if tier < 20 {
            // Tier 2: Assessment + key risks/wins only
            prompt.push_str(&format!(
                "{}**{}** ({}) | ARR: {} | Renewal: {} | Meetings: {}\n",
                indent,
                crate::util::sanitize_external_field(&snap.account_name),
                band_str, arr_str, renewal_str, snap.meeting_count_90d,
            ));
            let para = first_paragraph(assessment, 200);
            if !para.is_empty() {
                prompt.push_str(&crate::util::wrap_user_data(&para));
                prompt.push('\n');
            }
            if let Some(r) = raw {
                if let Some(ref risks) = r.risks_json {
                    if risks.len() > 2 {
                        prompt.push_str(&format!("{}Risks: {}\n", indent, truncate(risks, 150)));
                    }
                }
                if let Some(ref wins) = r.recent_wins_json {
                    if wins.len() > 2 {
                        prompt.push_str(&format!("{}Wins: {}\n", indent, truncate(wins, 150)));
                    }
                }
            }
            prompt.push('\n');
        } else {
            // Tier 3: Minimal — name, health, ARR
            prompt.push_str(&format!(
                "{}- {} | {} | ARR: {}\n",
                indent,
                crate::util::sanitize_external_field(&snap.account_name),
                band_str, arr_str,
            ));
        }
    };

    // Emit parent groups first
    for snap in accounts_to_emit.iter() {
        if !snap.is_parent {
            continue;
        }
        let bu_count = snap.bu_count.unwrap_or(0);
        let child_arr: f64 = snapshot
            .iter()
            .filter(|s| s.parent_id.as_deref() == Some(&snap.account_id))
            .filter_map(|s| s.arr)
            .sum();
        let total_arr = snap.arr.unwrap_or(0.0) + child_arr;

        prompt.push_str(&format!(
            "## {} (Parent — {} business units, ${:.0} combined ARR)\n\n",
            crate::util::sanitize_external_field(&snap.account_name),
            bu_count, total_arr,
        ));
        emit_account(&mut prompt, snap, tier_idx, "");
        emitted.insert(snap.account_id.clone());
        tier_idx += 1;

        let mut children: Vec<&AccountSnapshotRow> = snapshot
            .iter()
            .filter(|s| s.parent_id.as_deref() == Some(&snap.account_id))
            .collect();
        children.sort_by(|a, b| {
            b.arr
                .unwrap_or(0.0)
                .partial_cmp(&a.arr.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for child in children {
            emit_account(&mut prompt, child, tier_idx, "  ");
            emitted.insert(child.account_id.clone());
            tier_idx += 1;
        }
        prompt.push('\n');
    }

    // Standalone accounts
    for snap in accounts_to_emit.iter() {
        if emitted.contains(&snap.account_id) {
            continue;
        }
        emit_account(&mut prompt, snap, tier_idx, "");
        tier_idx += 1;
    }
    prompt.push('\n');

    prompt
}

/// Build a focused deep-dive context block for spotlight accounts.
/// Gives the AI the full pre-computed intelligence for each account so it can
/// synthesize a narrative without re-analyzing from scratch. Much smaller than
/// the full portfolio block — prevents timeouts on 10+ account portfolios.
fn build_spotlight_detail_block(
    raw_accounts: &[RawAccountRow],
    snapshot: &[AccountSnapshotRow],
    spotlight_ids: &[String],
) -> String {
    let mut prompt = String::new();
    prompt.push_str("## Spotlight Account Details\n\n");
    prompt.push_str("Below is the full intelligence context for each account that needs a deep dive.\n");
    prompt.push_str("Use this data to write the statusNarrative, renewalOrGrowthImpact, activeWorkstreams, and risksAndGaps.\n\n");

    for id in spotlight_ids {
        let snap = match snapshot.iter().find(|s| s.account_id == *id) {
            Some(s) => s,
            None => continue,
        };
        let raw = raw_accounts.iter().find(|r| r.id == *id);

        let arr_str = snap.arr.map(|a| format!("${:.0}", a)).unwrap_or_else(|| "N/A".to_string());
        let band_str = snap.health_band.as_deref().unwrap_or("unknown");
        let renewal_str = snap.renewal_date.as_deref().unwrap_or("N/A");
        let lifecycle_str = snap.lifecycle.as_deref().unwrap_or("N/A");
        let contact_str = snap.key_contact.as_deref().unwrap_or("N/A");

        prompt.push_str(&format!(
            "### {} (ID: {})\n",
            crate::util::sanitize_external_field(&snap.account_name),
            id,
        ));
        prompt.push_str(&format!(
            "Health: {} | ARR: {} | Lifecycle: {} | Renewal: {} | Meetings (90d): {} | Contact: {}\n",
            band_str, arr_str, lifecycle_str, renewal_str, snap.meeting_count_90d, contact_str,
        ));

        if let Some(r) = raw {
            if let Some(ref assessment) = r.executive_assessment {
                if !assessment.is_empty() {
                    prompt.push_str("Executive assessment: ");
                    prompt.push_str(&crate::util::wrap_user_data(&truncate(assessment, 600)));
                    prompt.push('\n');
                }
            }
            if let Some(ref risks) = r.risks_json {
                if risks.len() > 2 {
                    prompt.push_str("Known risks: ");
                    prompt.push_str(&crate::util::wrap_user_data(&truncate(risks, 500)));
                    prompt.push('\n');
                }
            }
            if let Some(ref wins) = r.recent_wins_json {
                if wins.len() > 2 {
                    prompt.push_str("Recent wins: ");
                    prompt.push_str(&crate::util::wrap_user_data(&truncate(wins, 500)));
                    prompt.push('\n');
                }
            }
            if let Some(ref val) = r.value_delivered {
                if !val.is_empty() {
                    prompt.push_str("Value delivered: ");
                    prompt.push_str(&crate::util::wrap_user_data(&truncate(val, 300)));
                    prompt.push('\n');
                }
            }
            if let Some(ref commits) = r.open_commitments {
                if !commits.is_empty() {
                    prompt.push_str("Open commitments: ");
                    prompt.push_str(&crate::util::wrap_user_data(&truncate(commits, 300)));
                    prompt.push('\n');
                }
            }
            if let Some(ref stakeholders) = r.stakeholder_insights_json {
                if stakeholders.len() > 2 {
                    prompt.push_str("Stakeholder insights: ");
                    prompt.push_str(&crate::util::wrap_user_data(&truncate(stakeholders, 400)));
                    prompt.push('\n');
                }
            }
            if let Some(ref state) = r.current_state_json {
                if state.len() > 2 {
                    prompt.push_str("Current state: ");
                    prompt.push_str(&crate::util::wrap_user_data(&truncate(state, 400)));
                    prompt.push('\n');
                }
            }
        }
        prompt.push('\n');
    }

    prompt
}

/// Append activity context (actions, emails, captures) to a prompt.
fn append_activity_context(prompt: &mut String, gather: &BookGatherOutput) {
    if !gather.open_actions.is_empty() {
        prompt.push_str("## Open Actions (top 20)\n");
        prompt.push_str(&crate::util::wrap_user_data(&gather.open_actions));
        prompt.push_str("\n\n");
    }

    if !gather.email_signals.is_empty() {
        prompt.push_str("## Email Activity (90d)\n");
        prompt.push_str(&crate::util::wrap_user_data(&gather.email_signals));
        prompt.push_str("\n\n");
    }

    if !gather.captures.is_empty() {
        prompt.push_str("## Portfolio Captures (urgency-sorted, 90d)\n");
        prompt.push_str(&crate::util::wrap_user_data(&gather.captures));
        prompt.push_str("\n\n");
    }
}

/// Append spotlight account instructions to a prompt.
/// Section-aware: gives per-section prioritization rules matching the monolithic path.
fn append_spotlight_instructions(prompt: &mut String, gather: &BookGatherOutput, section: &str) {
    if gather.spotlight_ids.is_empty() {
        return;
    }
    prompt.push_str("## Spotlight Accounts (User-Selected)\n\n");
    prompt.push_str(
        "The user has selected these accounts as the focus of this review. \
         They are the accounts the user plans to discuss with leadership.\n",
    );

    // Per-section spotlight prioritization rules
    let section_rule = match section {
        "topRisks" => "Lead with risks from these accounts (add others only if critical).",
        "topOpportunities" => "Lead with opportunities from these accounts.",
        "deepDives" => "You MUST include a deepDive for each selected account.",
        "valueDelivered" => "Prioritize outcomes from these accounts.",
        "keyThemes" => "Themes should be grounded in patterns across these accounts.",
        "leadershipAsks" => "Asks should relate to these accounts where applicable.",
        "executiveSummary" => "Ground the summary in these accounts' collective story.",
        _ => "Prioritize these accounts in your output.",
    };
    prompt.push_str(&format!(
        "For this section: {}\n\
         You may include other accounts where warranted, but the selected accounts are the narrative center.\n\n",
        section_rule,
    ));

    for id in &gather.spotlight_ids {
        if let Some(snap) = gather.snapshot.iter().find(|s| s.account_id == *id) {
            prompt.push_str(&format!(
                "- {} (ID: {})\n",
                crate::util::sanitize_external_field(&snap.account_name),
                id,
            ));
        }
    }
    prompt.push('\n');
}

// =============================================================================
// I547: Per-section prompt builders (Step 2)
// =============================================================================

/// Wave 1 cross-cutting section names (generated in parallel).
/// deepDives is handled separately — one PTY call per spotlight account.
const WAVE1_SECTIONS: &[&str] = &[
    "topRisks",
    "topOpportunities",
    "valueDelivered",
    "keyThemes",
    "leadershipAsks",
];

/// Build a prompt for a single BoB section.
fn build_bob_section_prompt(
    section: &str,
    gather: &BookGatherOutput,
    glean_ctx: &GleanPortfolioContext,
) -> String {
    let mut prompt = build_portfolio_context_block(gather);

    if section == "deepDives" && !gather.spotlight_ids.is_empty() {
        // DeepDives with spotlights: focused context for spotlight accounts only.
        // Full portfolio context causes timeouts — deepDives needs deep detail on
        // fewer accounts, not shallow detail on all accounts.
        prompt.push_str(&build_spotlight_detail_block(
            &gather.raw_accounts,
            &gather.snapshot,
            &gather.spotlight_ids,
        ));
    } else {
        prompt.push_str(&build_account_data_block(
            &gather.raw_accounts,
            &gather.snapshot,
        ));
    }

    // Activity context (skip for deepDives — it has focused account context instead)
    if section != "deepDives" {
        append_activity_context(&mut prompt, gather);
    }

    // Spotlight instructions
    append_spotlight_instructions(&mut prompt, gather, section);

    // User context (I413 — semantic search from user knowledge base)
    if !gather.user_context_block.is_empty() {
        prompt.push_str(&gather.user_context_block);
    }

    // Glean supplement
    let glean_block = match section {
        "topRisks" => glean_ctx.risk_pulse.as_deref(),
        "topOpportunities" => glean_ctx.pipeline_signals.as_deref(),
        "deepDives" => {
            // Combine risk_pulse + pipeline_signals for spotlight accounts
            let parts: Vec<&str> = [
                glean_ctx.risk_pulse.as_deref(),
                glean_ctx.pipeline_signals.as_deref(),
            ]
            .iter()
            .copied()
            .flatten()
            .collect();
            if parts.is_empty() {
                None
            } else {
                // Build inline — can't return a reference to a local
                prompt.push_str("## Enterprise Context (Glean)\n");
                for part in parts {
                    prompt.push_str(&crate::util::wrap_user_data(part));
                    prompt.push('\n');
                }
                prompt.push('\n');
                None // already appended
            }
        }
        "valueDelivered" => glean_ctx.pipeline_signals.as_deref(),
        "keyThemes" => glean_ctx.themes.as_deref(),
        "leadershipAsks" => {
            let parts: Vec<&str> = [
                glean_ctx.risk_pulse.as_deref(),
                glean_ctx.themes.as_deref(),
            ]
            .iter()
            .copied()
            .flatten()
            .collect();
            if parts.is_empty() {
                None
            } else {
                prompt.push_str("## Enterprise Context (Glean)\n");
                for part in parts {
                    prompt.push_str(&crate::util::wrap_user_data(part));
                    prompt.push('\n');
                }
                prompt.push('\n');
                None
            }
        }
        _ => None,
    };

    if let Some(ctx) = glean_block {
        prompt.push_str("## Enterprise Context (Glean)\n");
        prompt.push_str(&crate::util::wrap_user_data(ctx));
        prompt.push_str("\n\n");
    }

    // Section-specific output schema + rules
    prompt.push_str("## Output Format\n\n");
    prompt.push_str("Respond with ONLY valid JSON (no markdown fences) matching this schema exactly:\n\n");

    match section {
        "topRisks" => {
            prompt.push_str(r#"{ "topRisks": [ { "accountName": "Name", "risk": "Specific risk — max 25 words", "arr": 100000.0 } ] }"#);
            prompt.push_str("\n\n## Rules\n\n");
            prompt.push_str(
                "- 3-5 items. Each names a specific account with a specific, concrete risk.\n\
                 - Write each risk as a single punchy statement (max 25 words) a VP can absorb in a glance.\n\
                 - Include the account's ARR if known (null if not).\n\
                 - Do NOT fabricate data. If the data is sparse, say so. Empty arrays are acceptable.\n\
                 - Do NOT over-dramatize risk. Missing a named contact does not mean the relationship is failing.\n\
                 - CAPTURES: When captures with RED urgency are present, use them to ground risks.\n\
                 - Write for the ear, not the page. Short sentences. Active voice. No jargon.\n",
            );
        }
        "topOpportunities" => {
            prompt.push_str(r#"{ "topOpportunities": [ { "accountName": "Name", "opportunity": "Specific opportunity — max 30 words", "estimatedValue": "Potential impact or null" } ] }"#);
            prompt.push_str("\n\n## Rules\n\n");
            prompt.push_str(
                "- 2-4 items. Account name + specific opportunity as a single clear statement.\n\
                 - Do NOT fabricate data. Empty arrays are acceptable.\n\
                 - Write for the ear, not the page. Short sentences. Active voice.\n",
            );
        }
        "deepDives" => {
            prompt.push_str(r#"{ "deepDives": [ { "accountName": "Name", "accountId": "ID", "arr": 100000.0, "renewalDate": "2026-06-30 or null", "statusNarrative": "2-3 sentences", "renewalOrGrowthImpact": "One sentence on revenue impact", "activeWorkstreams": ["Workstream 1"], "risksAndGaps": ["Risk 1"] } ] }"#);
            prompt.push_str("\n\n## Rules\n\n");
            prompt.push_str(
                "- Each deep dive statusNarrative should be 2-3 sentences you could say out loud.\n\
                 - renewalOrGrowthImpact: one sentence on what this means for revenue.\n\
                 - activeWorkstreams and risksAndGaps: short bullet phrases, not sentences.\n\
                 - Include arr and renewalDate from the data.\n\
                 - For PARENT accounts with BUs: include parent-level when the overall relationship warrants it.\n\
                 - You MUST include a deepDive for each spotlight account.\n\
                 - Do NOT fabricate dates or commitments. Do NOT mention AI or signals.\n",
            );
        }
        "valueDelivered" => {
            prompt.push_str(r#"{ "valueDelivered": [ { "accountName": "Name", "headlineOutcome": "What was achieved — max 20 words", "whyItMatters": "Business impact — max 20 words", "source": "Where observed, or null" } ] }"#);
            prompt.push_str("\n\n## Rules\n\n");
            prompt.push_str(
                "- 2-4 items. Concrete outcomes — what was actually achieved, not generic statements.\n\
                 - headlineOutcome and whyItMatters should each be one crisp sentence.\n\
                 - CAPTURES: Use captures to ground value delivered.\n\
                 - Do NOT fabricate data. Empty arrays are acceptable.\n",
            );
        }
        "keyThemes" => {
            prompt.push_str(r#"{ "keyThemes": [ { "title": "Theme title", "narrative": "2-3 sentences", "citedAccounts": ["Account A", "Account B"] } ] }"#);
            prompt.push_str("\n\n## Rules\n\n");
            prompt.push_str(
                "- 2-3 cross-portfolio patterns. The 'so what' — patterns a leader needs to know.\n\
                 - Each narrative is 2-3 sentences. citedAccounts lists which accounts illustrate the theme.\n\
                 - Do NOT fabricate data. Write for the ear, not the page.\n",
            );
        }
        "leadershipAsks" => {
            prompt.push_str(r#"{ "leadershipAsks": [ { "ask": "Specific ask", "context": "Why — grounded in data", "impactedAccounts": ["Account A"], "status": "new | in-progress | blocked | null" } ] }"#);
            prompt.push_str("\n\n## Rules\n\n");
            prompt.push_str(
                "- 1-3 items. What you need from leadership to unblock progress. Be specific.\n\
                 - impactedAccounts lists affected accounts.\n\
                 - Do NOT fabricate data. Empty arrays are acceptable.\n",
            );
        }
        _ => {}
    }

    // Common rules suffix
    prompt.push_str(
        "- Do NOT include accountSnapshot, totalAccounts, totalArr, or any pre-computed metrics.\n\
         - Do NOT mention AI, enrichment, signals, or internal app mechanics. Use human language.\n\
         - A large parent account with many BUs can be healthy even if only a few BUs are active.\n",
    );

    prompt
}

/// Build a per-account deep dive prompt. Uses the pre-computed intelligence
/// for one account — small, focused prompt that completes in ~10-20s.
fn build_single_deep_dive_prompt(
    gather: &BookGatherOutput,
    account_id: &str,
    glean_ctx: &GleanPortfolioContext,
) -> Option<String> {
    let snap = gather.snapshot.iter().find(|s| s.account_id == account_id)?;
    let raw = gather.raw_accounts.iter().find(|r| r.id == account_id);

    let mut prompt = String::with_capacity(4096);
    prompt.push_str("You are a senior customer success strategist preparing a deep dive slide for a leadership presentation.\n");
    prompt.push_str("Ground every claim in the data provided. Use executive-ready language.\n\n");

    let arr_str = snap.arr.map(|a| format!("${:.0}", a)).unwrap_or_else(|| "N/A".to_string());
    let band_str = snap.health_band.as_deref().unwrap_or("unknown");
    let renewal_str = snap.renewal_date.as_deref().unwrap_or("N/A");
    let lifecycle_str = snap.lifecycle.as_deref().unwrap_or("N/A");
    let contact_str = snap.key_contact.as_deref().unwrap_or("N/A");

    prompt.push_str(&format!(
        "# {} (ID: {})\n\n",
        crate::util::sanitize_external_field(&snap.account_name),
        account_id,
    ));
    prompt.push_str(&format!(
        "Health: {} | ARR: {} | Lifecycle: {} | Renewal: {} | Meetings (90d): {} | Contact: {}\n\n",
        band_str, arr_str, lifecycle_str, renewal_str, snap.meeting_count_90d, contact_str,
    ));

    // Feed pre-computed intelligence
    if let Some(r) = raw {
        if let Some(ref assessment) = r.executive_assessment {
            if !assessment.is_empty() {
                prompt.push_str("## Executive Assessment\n");
                prompt.push_str(&crate::util::wrap_user_data(assessment));
                prompt.push_str("\n\n");
            }
        }
        if let Some(ref state) = r.current_state_json {
            if state.len() > 2 {
                prompt.push_str("## Current State\n");
                prompt.push_str(&crate::util::wrap_user_data(state));
                prompt.push_str("\n\n");
            }
        }
        if let Some(ref risks) = r.risks_json {
            if risks.len() > 2 {
                prompt.push_str("## Known Risks\n");
                prompt.push_str(&crate::util::wrap_user_data(risks));
                prompt.push_str("\n\n");
            }
        }
        if let Some(ref wins) = r.recent_wins_json {
            if wins.len() > 2 {
                prompt.push_str("## Recent Wins\n");
                prompt.push_str(&crate::util::wrap_user_data(wins));
                prompt.push_str("\n\n");
            }
        }
        if let Some(ref val) = r.value_delivered {
            if !val.is_empty() {
                prompt.push_str("## Value Delivered\n");
                prompt.push_str(&crate::util::wrap_user_data(val));
                prompt.push_str("\n\n");
            }
        }
        if let Some(ref commits) = r.open_commitments {
            if !commits.is_empty() {
                prompt.push_str("## Open Commitments\n");
                prompt.push_str(&crate::util::wrap_user_data(commits));
                prompt.push_str("\n\n");
            }
        }
        if let Some(ref stakeholders) = r.stakeholder_insights_json {
            if stakeholders.len() > 2 {
                prompt.push_str("## Stakeholder Insights\n");
                prompt.push_str(&crate::util::wrap_user_data(stakeholders));
                prompt.push_str("\n\n");
            }
        }
    }

    // Glean supplement for deep dives
    let glean_parts: Vec<&str> = [
        glean_ctx.risk_pulse.as_deref(),
        glean_ctx.pipeline_signals.as_deref(),
    ].iter().copied().flatten().collect();
    if !glean_parts.is_empty() {
        prompt.push_str("## Enterprise Context (Glean)\n");
        for part in glean_parts {
            prompt.push_str(&crate::util::wrap_user_data(part));
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    prompt.push_str("## Output Format\n\n");
    prompt.push_str("Respond with ONLY valid JSON (no markdown fences):\n\n");
    prompt.push_str(&format!(
        r#"{{ "accountName": "{}", "accountId": "{}", "arr": {}, "renewalDate": {}, "statusNarrative": "2-3 sentences", "renewalOrGrowthImpact": "One sentence", "activeWorkstreams": ["..."], "risksAndGaps": ["..."] }}"#,
        crate::util::sanitize_external_field(&snap.account_name),
        account_id,
        snap.arr.map(|a| format!("{:.1}", a)).unwrap_or_else(|| "null".to_string()),
        snap.renewal_date.as_ref().map(|d| format!("\"{}\"", d)).unwrap_or_else(|| "null".to_string()),
    ));

    prompt.push_str("\n\n## Rules\n\n");
    prompt.push_str(
        "- statusNarrative: 2-3 sentences you could say out loud — the story of this account right now.\n\
         - renewalOrGrowthImpact: one sentence on what this means for revenue.\n\
         - activeWorkstreams and risksAndGaps: short bullet phrases, not sentences.\n\
         - Ground everything in the data above. Do NOT fabricate dates or commitments.\n\
         - Do NOT mention AI, enrichment, signals, or internal mechanics.\n\
         - Write for the ear, not the page. Short sentences. Active voice.\n",
    );

    Some(prompt)
}

/// Build the executiveSummary prompt using Wave 1 results as context.
fn build_executive_summary_prompt(
    gather: &BookGatherOutput,
    glean_ctx: &GleanPortfolioContext,
    wave1_results: &serde_json::Value,
) -> String {
    let mut prompt = build_portfolio_context_block(gather);

    // User context (I413)
    if !gather.user_context_block.is_empty() {
        prompt.push_str(&gather.user_context_block);
    }

    prompt.push_str("## Already-Generated Sections (use as context for executive summary)\n\n");
    prompt.push_str(&crate::util::wrap_user_data(
        &serde_json::to_string_pretty(wave1_results).unwrap_or_default(),
    ));
    prompt.push_str("\n\n");

    // Include all Glean context for executive summary
    let all_glean: Vec<&str> = [
        glean_ctx.risk_pulse.as_deref(),
        glean_ctx.pipeline_signals.as_deref(),
        glean_ctx.themes.as_deref(),
    ]
    .iter()
    .copied()
    .flatten()
    .collect();
    if !all_glean.is_empty() {
        prompt.push_str("## Enterprise Context (Glean)\n");
        for part in all_glean {
            prompt.push_str(&crate::util::wrap_user_data(part));
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    prompt.push_str("## Output Format\n\n");
    prompt.push_str("Respond with ONLY valid JSON (no markdown fences):\n\n");
    prompt.push_str(r#"{ "executiveSummary": "2-4 sentence portfolio-level executive summary." }"#);
    prompt.push_str("\n\n## Rules\n\n");
    prompt.push_str(
        "- 2-4 sentences, portfolio-level. The opening statement you'd say walking into the room.\n\
         - Mention overall health posture, the one thing that matters most this period, and where attention is needed.\n\
         - Not a list of accounts. Not a recap of each section.\n\
         - Grounded in the actual section data above — don't invent new claims.\n\
         - Write for the ear, not the page. Short sentences. Active voice. No jargon.\n\
         - Do NOT mention AI, enrichment, signals, or internal app mechanics.\n",
    );

    prompt
}

// =============================================================================
// I547: Parallel execution engine (Step 3)
// =============================================================================

/// Run parallel BoB generation:
/// - 5 cross-cutting sections in parallel (topRisks, topOpportunities, valueDelivered, keyThemes, leadershipAsks)
/// - N per-account deep dive calls in parallel (one per spotlight account)
/// - Sequential executiveSummary using all Wave 1 results
/// Emits progressive `bob-section-progress` events via AppHandle.
pub fn run_parallel_bob_generation(
    gather: &BookGatherOutput,
    glean_ctx: &GleanPortfolioContext,
    metrics: &BookMetrics,
    app_handle: Option<&AppHandle>,
) -> Result<AiBookResponse, String> {
    let overall_start = Instant::now();
    let total_sections = WAVE1_SECTIONS.len() as u32 + 1 + 1; // 5 cross-cutting + deepDives + executiveSummary

    // Channel for receiving section results
    let (tx, rx) = std::sync::mpsc::channel();

    // Spawn one thread per cross-cutting section
    for &section in WAVE1_SECTIONS {
        let section_prompt = build_bob_section_prompt(section, gather, glean_ctx);
        let workspace = gather.workspace.clone();
        let ai_cfg = gather.ai_models.clone();
        let sec_name = section.to_string();
        let sender = tx.clone();

        std::thread::spawn(move || {
            let sec_start = Instant::now();

            // Synthesis tier — BoB sections require narrative reasoning, not extraction.
            let pty = PtyManager::for_tier(ModelTier::Synthesis, &ai_cfg)
                .with_timeout(60)
                .with_nice_priority(10);

            let result = pty
                .spawn_claude(&workspace, &section_prompt)
                .map_err(|e| format!("PTY error for section {}: {}", sec_name, e))
                .and_then(|output| {
                    let json_str = crate::risk_briefing::extract_json_object(&output.stdout)
                        .ok_or_else(|| {
                            format!(
                                "No JSON in {} response ({}ms)",
                                sec_name,
                                sec_start.elapsed().as_millis()
                            )
                        })?;

                    let value: serde_json::Value =
                        serde_json::from_str(&json_str).map_err(|e| {
                            format!("Parse error for section {}: {}", sec_name, e)
                        })?;

                    log::info!(
                        "[I547] Section {} completed in {}ms",
                        sec_name,
                        sec_start.elapsed().as_millis()
                    );

                    Ok((sec_name, value))
                });

            let _ = sender.send(result);
        });
    }

    // Spawn per-account deep dive threads (one per spotlight account).
    // If no spotlights selected, fall back to a single deepDives section call.
    if gather.spotlight_ids.is_empty() {
        // No spotlights — use the standard section prompt for deepDives
        let section_prompt = build_bob_section_prompt("deepDives", gather, glean_ctx);
        let workspace = gather.workspace.clone();
        let ai_cfg = gather.ai_models.clone();
        let sender = tx.clone();

        std::thread::spawn(move || {
            let sec_start = Instant::now();
            let pty = PtyManager::for_tier(ModelTier::Synthesis, &ai_cfg)
                .with_timeout(90)
                .with_nice_priority(10);

            let result = pty
                .spawn_claude(&workspace, &section_prompt)
                .map_err(|e| format!("PTY error for section deepDives: {}", e))
                .and_then(|output| {
                    let json_str = crate::risk_briefing::extract_json_object(&output.stdout)
                        .ok_or_else(|| format!("No JSON in deepDives response ({}ms)", sec_start.elapsed().as_millis()))?;
                    let value: serde_json::Value = serde_json::from_str(&json_str)
                        .map_err(|e| format!("Parse error for section deepDives: {}", e))?;
                    log::info!("[I547] Section deepDives completed in {}ms", sec_start.elapsed().as_millis());
                    Ok(("deepDives".to_string(), value))
                });

            let _ = sender.send(result);
        });
    } else {
        // Per-account deep dive calls — small focused prompts, trivially parallel
        let deep_dive_tx = tx.clone();
        let workspace = gather.workspace.clone();
        let ai_cfg = gather.ai_models.clone();
        let spotlight_ids = gather.spotlight_ids.clone();

        // Collect all per-account prompts upfront (before moving into thread)
        let account_prompts: Vec<(String, String)> = spotlight_ids
            .iter()
            .filter_map(|id| {
                build_single_deep_dive_prompt(gather, id, glean_ctx)
                    .map(|prompt| (id.clone(), prompt))
            })
            .collect();

        // Spawn a coordinator thread that fans out per-account calls
        std::thread::spawn(move || {
            let (acct_tx, acct_rx) = std::sync::mpsc::channel();

            for (acct_id, prompt) in account_prompts {
                let ws = workspace.clone();
                let cfg = ai_cfg.clone();
                let sender = acct_tx.clone();

                std::thread::spawn(move || {
                    let start = Instant::now();
                    let pty = PtyManager::for_tier(ModelTier::Synthesis, &cfg)
                        .with_timeout(45)
                        .with_nice_priority(10);

                    let result = pty.spawn_claude(&ws, &prompt);
                    match result {
                        Ok(output) => {
                            if let Some(json_str) = crate::risk_briefing::extract_json_object(&output.stdout) {
                                if let Ok(dive) = serde_json::from_str::<AccountDeepDive>(&json_str) {
                                    log::info!("[I547] Deep dive for {} completed in {}ms", acct_id, start.elapsed().as_millis());
                                    let _ = sender.send(Ok(dive));
                                    return;
                                } else {
                                    log::warn!("[I547] Deep dive parse failed for {}: {}", acct_id,
                                        json_str.chars().take(200).collect::<String>());
                                }
                            }
                            log::warn!("[I547] No JSON in deep dive for {} ({}ms)", acct_id, start.elapsed().as_millis());
                            let _ = sender.send(Err(format!("No JSON for {}", acct_id)));
                        }
                        Err(e) => {
                            log::warn!("[I547] Deep dive PTY error for {}: {} ({}ms)", acct_id, e, start.elapsed().as_millis());
                            let _ = sender.send(Err(format!("PTY error for {}: {}", acct_id, e)));
                        }
                    }
                });
            }

            drop(acct_tx);

            // Collect all per-account results into a single deepDives array
            let mut dives = Vec::new();
            for result in acct_rx {
                if let Ok(dive) = result {
                    dives.push(dive);
                }
            }

            // Send as a combined deepDives section result
            let combined = serde_json::json!({ "deepDives": dives });
            log::info!("[I547] All deep dives collected: {} accounts", dives.len());
            let _ = deep_dive_tx.send(Ok(("deepDives".to_string(), combined)));
        });
    }

    // Drop our sender so rx iterator ends after all threads finish
    drop(tx);

    // Process Wave 1 results as they arrive
    let mut response = AiBookResponse::default();
    let mut wave1_json = serde_json::json!({});
    let mut succeeded = 0u32;
    let mut failed_sections: Vec<String> = Vec::new();

    for result in rx {
        match result {
            Ok((sec_name, value)) => {
                merge_section_into(&mut response, &sec_name, &value);
                wave1_json[&sec_name] = value[&sec_name].clone();
                succeeded += 1;

                if let Some(handle) = app_handle {
                    let _ = handle.emit(
                        "bob-section-progress",
                        BobSectionProgress {
                            section_name: sec_name.clone(),
                            completed: succeeded,
                            total: total_sections,
                            wave: 1,
                        },
                    );
                    let _ = handle.emit(
                        "bob-section-content",
                        assemble_book_content(response.clone(), metrics.clone()),
                    );
                }

                log::info!(
                    "[I547] Wave 1: {}/{} sections completed (latest: {})",
                    succeeded,
                    WAVE1_SECTIONS.len(),
                    sec_name
                );
            }
            Err(e) => {
                let sec = e
                    .split("section ")
                    .nth(1)
                    .and_then(|s| s.split(':').next())
                    .unwrap_or("unknown")
                    .to_string();
                log::warn!("[I547] Wave 1 section failed: {}", e);
                failed_sections.push(sec);
            }
        }
    }

    if succeeded == 0 {
        return Err("All Wave 1 sections failed — falling back to monolithic".to_string());
    }

    // Wave 2: executiveSummary (sequential, uses Wave 1 results as context)
    let exec_start = Instant::now();
    let exec_prompt = build_executive_summary_prompt(gather, glean_ctx, &wave1_json);

    let pty = PtyManager::for_tier(ModelTier::Synthesis, &gather.ai_models)
        .with_timeout(60)
        .with_nice_priority(10);

    match pty.spawn_claude(&gather.workspace, &exec_prompt) {
        Ok(output) => {
            if let Some(json_str) = crate::risk_briefing::extract_json_object(&output.stdout) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(summary) = value
                        .get("executiveSummary")
                        .and_then(|v| v.as_str())
                    {
                        response.executive_summary = summary.to_string();
                    }
                }
            }
            log::info!(
                "[I547] Executive summary completed in {}ms",
                exec_start.elapsed().as_millis()
            );
        }
        Err(e) => {
            log::warn!("[I547] Executive summary failed: {}", e);
            response.executive_summary =
                "Portfolio review generated — executive summary unavailable.".to_string();
        }
    }

    if let Some(handle) = app_handle {
        let _ = handle.emit(
            "bob-section-progress",
            BobSectionProgress {
                section_name: "executiveSummary".to_string(),
                completed: succeeded + 1,
                total: total_sections,
                wave: 2,
            },
        );
        let _ = handle.emit(
            "bob-section-content",
            assemble_book_content(response.clone(), metrics.clone()),
        );
    }

    let total_ms = overall_start.elapsed().as_millis();
    log::info!(
        "[I547] Parallel BoB: {}/6 Wave 1 + executiveSummary in {}ms (failed: {:?})",
        succeeded,
        total_ms,
        failed_sections,
    );

    Ok(response)
}

/// Merge a parsed section value into the combined response.
fn merge_section_into(response: &mut AiBookResponse, section: &str, value: &serde_json::Value) {
    // Helper: extract + deserialize a section array with logging
    fn extract_section<T: serde::de::DeserializeOwned>(
        value: &serde_json::Value,
        key: &str,
    ) -> Option<Vec<T>> {
        match value.get(key) {
            Some(arr) => match serde_json::from_value::<Vec<T>>(arr.clone()) {
                Ok(items) => {
                    log::info!("[I547] Merged {} — {} items", key, items.len());
                    Some(items)
                }
                Err(e) => {
                    log::warn!("[I547] Failed to deserialize {}: {} — raw: {}", key, e,
                        serde_json::to_string(arr).unwrap_or_default().chars().take(200).collect::<String>());
                    None
                }
            },
            None => {
                log::warn!("[I547] Section {} missing key '{}' in response: {}",
                    key, key,
                    serde_json::to_string(value).unwrap_or_default().chars().take(200).collect::<String>());
                None
            }
        }
    }

    match section {
        "topRisks" => {
            if let Some(items) = extract_section::<BookRiskItem>(value, "topRisks") {
                response.top_risks = items;
            }
        }
        "topOpportunities" => {
            if let Some(items) = extract_section::<BookOpportunityItem>(value, "topOpportunities") {
                response.top_opportunities = items;
            }
        }
        "deepDives" => {
            if let Some(items) = extract_section::<AccountDeepDive>(value, "deepDives") {
                response.deep_dives = items;
            }
        }
        "valueDelivered" => {
            if let Some(items) = extract_section::<ValueDeliveredRow>(value, "valueDelivered") {
                response.value_delivered = items;
            }
        }
        "keyThemes" => {
            if let Some(items) = extract_section::<BookTheme>(value, "keyThemes") {
                response.key_themes = items;
            }
        }
        "leadershipAsks" => {
            if let Some(items) = extract_section::<LeadershipAsk>(value, "leadershipAsks") {
                response.leadership_asks = items;
            }
        }
        _ => {}
    }
}

// =============================================================================
// I547: Glean pre-fetch (Step 4)
// =============================================================================

/// Pre-fetch enterprise context from Glean for portfolio-level insights.
/// Three parallel MCP chat calls with 15s timeout each. Each field is
/// `None` on timeout/error (non-fatal).
pub fn prefetch_glean_portfolio_context(
    endpoint: &str,
    account_names: &[String],
) -> GleanPortfolioContext {
    let accounts_str = account_names.join(", ");

    let (tx, rx) = std::sync::mpsc::channel();

    // 3 parallel Glean queries
    let queries: Vec<(&str, String)> = vec![
        (
            "risk_pulse",
            format!(
                "For these accounts: {}. \
                 Summarize any escalations, at-risk deals, negative call sentiment, \
                 or concerning support trends per account. Be specific with account names. \
                 If no risk signals, say so.",
                accounts_str
            ),
        ),
        (
            "pipeline_signals",
            format!(
                "For these accounts: {}. \
                 Summarize expansion opportunities, deal pipeline status, upsell signals, \
                 and positive momentum per account. Be specific with account names. \
                 If no pipeline signals, say so.",
                accounts_str
            ),
        ),
        (
            "themes",
            format!(
                "Looking across these accounts: {}. \
                 What cross-portfolio patterns do you see in Slack discussions, \
                 support trends, competitive mentions, or shared challenges? \
                 Summarize 2-3 themes.",
                accounts_str
            ),
        ),
    ];

    for (key, query) in queries {
        let ep = endpoint.to_string();
        let sender = tx.clone();

        std::thread::spawn(move || {
            // Create a temporary tokio runtime for the async Glean call
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    log::warn!("[I547] Failed to create runtime for Glean {}: {}", key, e);
                    let _ = sender.send((key, None));
                    return;
                }
            };

            let result = rt.block_on(async {
                let client =
                    crate::context_provider::glean::GleanMcpClient::new(&ep);
                tokio::time::timeout(
                    std::time::Duration::from_secs(15),
                    client.chat(&query, None),
                )
                .await
            });

            let text = match result {
                Ok(Ok(text)) => {
                    log::info!(
                        "[I547] Glean {} pre-fetch: {} chars",
                        key,
                        text.len()
                    );
                    Some(text)
                }
                Ok(Err(e)) => {
                    log::warn!("[I547] Glean {} pre-fetch failed: {}", key, e);
                    None
                }
                Err(_) => {
                    log::warn!("[I547] Glean {} pre-fetch timed out", key);
                    None
                }
            };

            let _ = sender.send((key, text));
        });
    }

    drop(tx);

    let mut ctx = GleanPortfolioContext::default();
    for (key, value) in rx {
        match key {
            "risk_pulse" => ctx.risk_pulse = value,
            "pipeline_signals" => ctx.pipeline_signals = value,
            "themes" => ctx.themes = value,
            _ => {}
        }
    }

    ctx
}

// =============================================================================
// Prompt building — monolithic (legacy fallback)
// =============================================================================

#[allow(clippy::too_many_arguments)]
fn build_book_of_business_prompt(
    raw_accounts: &[RawAccountRow],
    snapshot: &[AccountSnapshotRow],
    open_actions: &str,
    email_signals: &str,
    captures: &str,
    user_name: &str,
    user_role: &str,
    active_preset: &str,
    period_label: &str,
    metrics: &BookMetrics,
    spotlight_account_ids: Option<&[String]>,
) -> String {
    let mut prompt = build_report_preamble("Portfolio", "book_of_business", "user");

    // Role preset context
    prompt.push_str(&format!(
        "Role preset: {}. User: {} ({})\n\n",
        crate::util::sanitize_external_field(active_preset),
        crate::util::sanitize_external_field(user_name),
        crate::util::sanitize_external_field(user_role),
    ));

    // Portfolio summary
    prompt.push_str(&format!(
        "# Portfolio Overview: {}\n\n\
         Total accounts: {} | Total ARR: ${:.0} | At-risk ARR: ${:.0}\n\
         Upcoming renewals (90d): {} | Upcoming renewal ARR: ${:.0}\n\n",
        period_label,
        metrics.total_accounts,
        metrics.total_arr,
        metrics.at_risk_arr,
        metrics.upcoming_renewals,
        metrics.upcoming_renewals_arr,
    ));

    // Per-account data
    prompt.push_str(&build_account_data_block(raw_accounts, snapshot));

    // Activity summary
    if !open_actions.is_empty() {
        prompt.push_str("## Open Actions (top 20)\n");
        prompt.push_str(&crate::util::wrap_user_data(open_actions));
        prompt.push_str("\n\n");
    }

    if !email_signals.is_empty() {
        prompt.push_str("## Email Activity (90d)\n");
        prompt.push_str(&crate::util::wrap_user_data(email_signals));
        prompt.push_str("\n\n");
    }

    if !captures.is_empty() {
        prompt.push_str("## Portfolio Captures (urgency-sorted, 90d)\n");
        prompt.push_str(&crate::util::wrap_user_data(captures));
        prompt.push_str("\n\n");
    }

    // Spotlight accounts (user-selected)
    if let Some(ids) = spotlight_account_ids {
        if !ids.is_empty() {
            prompt.push_str("## Spotlight Accounts (User-Selected)\n\n");
            prompt.push_str(
                "The user has selected these accounts as the focus of this review. \
                 They are the accounts the user plans to discuss with leadership. \
                 ALL sections of the report should prioritize these accounts:\n\
                 - topRisks: lead with risks from these accounts (add others only if critical)\n\
                 - topOpportunities: lead with opportunities from these accounts\n\
                 - deepDives: you MUST include a deepDive for each selected account\n\
                 - valueDelivered: prioritize outcomes from these accounts\n\
                 - keyThemes: themes should be grounded in patterns across these accounts\n\
                 - leadershipAsks: asks should relate to these accounts where applicable\n\n\
                 You may include other accounts where warranted, but the selected accounts are the narrative center.\n\n",
            );
            for id in ids {
                if let Some(snap) = snapshot.iter().find(|s| s.account_id == *id) {
                    prompt.push_str(&format!(
                        "- {} (ID: {})\n",
                        crate::util::sanitize_external_field(&snap.account_name),
                        id,
                    ));
                }
            }
            prompt.push('\n');
        }
    }

    // Output schema — AI-generated fields only
    prompt.push_str("## Output Format\n\n");
    prompt.push_str(
        "Respond with ONLY valid JSON (no markdown fences) matching this schema exactly:\n\n",
    );
    prompt.push_str(
        r#"{
  "executiveSummary": "2-4 sentence portfolio-level executive summary. Grounded in the data above. Direct, not generic.",
  "topRisks": [
    {
      "accountName": "Name of the account",
      "risk": "What the risk is — specific, max 30 words",
      "arr": 100000.0
    }
  ],
  "topOpportunities": [
    {
      "accountName": "Name of the account",
      "opportunity": "What the opportunity is — specific, max 30 words",
      "estimatedValue": "Potential impact — expansion, deepening, new use case, or null"
    }
  ],
  "deepDives": [
    {
      "accountName": "Account name",
      "accountId": "Account ID from the data",
      "arr": 100000.0,
      "renewalDate": "2026-06-30 or null",
      "statusNarrative": "2-3 sentence status summary grounded in the data",
      "renewalOrGrowthImpact": "Impact on renewal or growth — always provide a statement",
      "activeWorkstreams": ["Active workstream 1", "Active workstream 2"],
      "risksAndGaps": ["Risk or gap 1", "Risk or gap 2"]
    }
  ],
  "valueDelivered": [
    {
      "accountName": "Account name",
      "headlineOutcome": "What was achieved — specific, max 20 words",
      "whyItMatters": "Business impact — max 20 words",
      "source": "Where this outcome was observed, or null"
    }
  ],
  "keyThemes": [
    {
      "title": "Theme title — e.g. 'Renewal Readiness' or 'Adoption Gaps'",
      "narrative": "2-3 sentences describing the cross-account pattern",
      "citedAccounts": ["Account A", "Account B"]
    }
  ],
  "leadershipAsks": [
    {
      "ask": "Specific ask — what you need",
      "context": "Why — grounded in account data",
      "impactedAccounts": ["Account A"],
      "status": "new | in-progress | blocked | null"
    }
  ]
}"#,
    );

    prompt.push_str("\n\n## Rules\n\n");
    prompt.push_str(
        "This report will be displayed as a slide-deck presentation for leadership. Write content that reads like presentation talking points, not dense analysis.\n\n\
         - executiveSummary: 2-4 sentences, portfolio-level. The opening statement you'd say walking into the room. Mention overall health posture, the one thing that matters most this period, and where attention is needed. Not a list of accounts.\n\
         - topRisks: 3-5 items. Each names a specific account with a specific, concrete risk. Write each risk as a single punchy statement (max 25 words) a VP can absorb in a glance. Include the account's ARR if known (null if not).\n\
         - topOpportunities: 2-4 items. Same format — account name + specific opportunity as a single clear statement.\n\
         - deepDives: The accounts that warrant a slide in the presentation.\n\
           - For PARENT accounts with business units: include a parent-level deep dive when the overall relationship warrants discussion (e.g., cross-BU themes, executive relationship, combined risk). Then include separate deep dives for individual BUs that have their own notable story (high activity, risk, or renewal).\n\
           - Each deep dive statusNarrative should be 2-3 sentences you could say out loud — the story of this account right now.\n\
           - renewalOrGrowthImpact: one sentence on what this means for revenue.\n\
           - activeWorkstreams and risksAndGaps: short bullet phrases, not sentences.\n\
           - Include arr and renewalDate from the data.\n\
         - valueDelivered: 2-4 items. Concrete outcomes — what was actually achieved, not generic statements. headlineOutcome and whyItMatters should each be one crisp sentence.\n\
         - keyThemes: 2-3 cross-portfolio patterns. These are the 'so what' — the patterns a leader needs to know about. Each narrative is 2-3 sentences. citedAccounts lists which accounts illustrate the theme.\n\
         - leadershipAsks: 1-3 items. What you need from leadership to unblock progress. Be specific. impactedAccounts lists affected accounts.\n\
         - Do NOT include accountSnapshot, totalAccounts, totalArr, or any pre-computed metrics in your response. Those are provided separately.\n\
         - Do NOT mention AI, enrichment, signals, or internal app mechanics. Use human language.\n\
         - Do NOT fabricate data. If the data is sparse, say so. Empty arrays are acceptable.\n\
         - Do NOT fabricate specific dates, deadlines, or commitments unless they appear verbatim in the data. If no date is in the data, do not invent one.\n\
         - Do NOT over-dramatize risk. Missing a named contact does not mean the relationship is failing — some accounts are managed passively for monitoring and renewals. Assess risk proportionally to ARR and strategic importance.\n\
         - CAPTURES: When Portfolio Captures are present, use them to ground topRisks (RED urgency captures), topOpportunities, and valueDelivered. Captures with account attribution provide concrete evidence — cite them.\n\
         - A large parent account with many BUs can be healthy even if only one or two BUs are actively engaged — that is normal portfolio management, not a structural failure.\n\
         - Write for the ear, not the page. Short sentences. Active voice. No jargon.\n",
    );

    prompt
}

// =============================================================================
// Response parsing
// =============================================================================

/// Intermediate struct for parsing AI-generated fields only.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiBookResponse {
    #[serde(default)]
    pub executive_summary: String,
    #[serde(default)]
    pub top_risks: Vec<BookRiskItem>,
    #[serde(default)]
    pub top_opportunities: Vec<BookOpportunityItem>,
    #[serde(default)]
    pub deep_dives: Vec<AccountDeepDive>,
    #[serde(default)]
    pub value_delivered: Vec<ValueDeliveredRow>,
    #[serde(default)]
    pub key_themes: Vec<BookTheme>,
    #[serde(default)]
    pub leadership_asks: Vec<LeadershipAsk>,
}

/// Convert an AiBookResponse + metrics into the final BookOfBusinessContent.
pub fn assemble_book_content(
    ai: AiBookResponse,
    metrics: BookMetrics,
) -> BookOfBusinessContent {
    let has_leadership_asks = !ai.leadership_asks.is_empty();

    BookOfBusinessContent {
        period_label: metrics.period_label,
        total_accounts: metrics.total_accounts,
        total_arr: metrics.total_arr,
        at_risk_arr: metrics.at_risk_arr,
        upcoming_renewals: metrics.upcoming_renewals,
        upcoming_renewals_arr: metrics.upcoming_renewals_arr,
        executive_summary: ai.executive_summary,
        top_risks: ai.top_risks,
        top_opportunities: ai.top_opportunities,
        account_snapshot: metrics.account_snapshot,
        deep_dives: ai.deep_dives,
        value_delivered: ai.value_delivered,
        key_themes: ai.key_themes,
        leadership_asks: ai.leadership_asks,
        has_leadership_asks,
    }
}

pub fn parse_book_of_business_response(
    stdout: &str,
    metrics: BookMetrics,
) -> Result<BookOfBusinessContent, String> {
    let json_str = crate::risk_briefing::extract_json_object(stdout)
        .ok_or_else(|| "No valid JSON object found in Book of Business response".to_string())?;

    let ai: AiBookResponse = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse Book of Business JSON: {}", e))?;

    Ok(assemble_book_content(ai, metrics))
}

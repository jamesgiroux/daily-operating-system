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
// Output schema — template-aligned (14 slides)
// =============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookOfBusinessContent {
    // Slide 1: Executive Summary
    #[serde(default)]
    pub period_label: String,
    #[serde(default)]
    pub executive_summary: String, // AI
    #[serde(default)]
    pub total_accounts: i32, // M
    #[serde(default)]
    pub total_arr: f64, // M
    #[serde(default)]
    pub at_risk_arr: f64, // M
    #[serde(default)]
    pub committed_expansion: f64, // M (user-editable)
    #[serde(default)]
    pub projected_churn: f64, // AI
    #[serde(default)]
    pub top_risks_summary: Vec<String>, // M — top 3 risk one-liners
    #[serde(default)]
    pub top_opportunities_summary: Vec<String>, // M — top 3 opportunity one-liners
    #[serde(default)]
    pub biggest_risk: Option<BiggestItem>, // M
    #[serde(default)]
    pub biggest_upside: Option<BiggestItem>, // M
    #[serde(default)]
    pub elt_help_required: bool, // AI

    // Slide 2: Portfolio Health Overview
    #[serde(default)]
    pub health_overview: PortfolioHealthOverview, // M

    // Slide 3: Risk & Retention Concerns (table)
    #[serde(default)]
    pub risk_accounts: Vec<RiskAccountRow>, // M

    // Slide 4: Highest Retention Risk (deep dives on top 2-3)
    #[serde(default)]
    pub retention_risk_deep_dives: Vec<RetentionRiskDeepDive>, // AI

    // Slide 5: Retention Save Motions (table)
    #[serde(default)]
    pub save_motions: Vec<SaveMotion>, // AI

    // Slide 6: Expansion Potential (table)
    #[serde(default)]
    pub expansion_accounts: Vec<ExpansionRow>, // M

    // Slide 7: Expansion Readiness & Risk (table)
    #[serde(default)]
    pub expansion_readiness: Vec<ExpansionReadiness>, // AI

    // Slide 8: Year-End Outlook (metrics)
    #[serde(default)]
    pub year_end_outlook: YearEndOutlook, // M

    // Slide 9: Year-End Landing Scenarios
    #[serde(default)]
    pub landing_scenarios: LandingScenarios, // AI

    // Slide 10+14: What I Need / Decisions & Leadership Asks (combined)
    #[serde(default)]
    pub leadership_asks: Vec<LeadershipAsk>, // AI

    // Slide 11: Top 5 Account Focus (spotlight accounts)
    #[serde(default)]
    pub account_focus: Vec<AccountFocus>, // AI

    // Slide 12: Q→Q Focus (priority bullets)
    #[serde(default)]
    pub quarterly_focus: QuarterlyFocus, // AI

    // Slide 13: Key Themes
    #[serde(default)]
    pub key_themes: Vec<BookTheme>, // AI

    // Carried forward from current schema
    #[serde(default)]
    pub account_snapshot: Vec<AccountSnapshotRow>, // M
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiggestItem {
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub arr: f64,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioHealthOverview {
    #[serde(default)]
    pub healthy_count: i32,
    #[serde(default)]
    pub healthy_arr: f64,
    #[serde(default)]
    pub medium_count: i32,
    #[serde(default)]
    pub medium_arr: f64,
    #[serde(default)]
    pub high_risk_count: i32,
    #[serde(default)]
    pub high_risk_arr: f64,
    #[serde(default)]
    pub secure_arr: f64,
    #[serde(default)]
    pub renewals_90d: i32,
    #[serde(default)]
    pub renewals_90d_arr: f64,
    #[serde(default)]
    pub renewals_180d: i32,
    #[serde(default)]
    pub renewals_180d_arr: f64,
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
pub struct RiskAccountRow {
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub arr: f64,
    #[serde(default)]
    pub renewal_timing: String,
    #[serde(default)]
    pub risk_level: String,
    #[serde(default)]
    pub primary_risk_driver: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionRiskDeepDive {
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub arr: f64,
    #[serde(default)]
    pub why_at_risk: String,
    #[serde(default)]
    pub save_confidence: String,
    #[serde(default)]
    pub next_90_days: String,
    #[serde(default)]
    pub key_tactics: Vec<String>,
    #[serde(default)]
    pub success_signals: Vec<String>,
    #[serde(default)]
    pub help_needed: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveMotion {
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub risk: String,
    #[serde(default)]
    pub save_motion: String,
    #[serde(default)]
    pub timeline: String,
    #[serde(default)]
    pub success_signals: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpansionRow {
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub arr: f64,
    #[serde(default)]
    pub readiness: String,
    #[serde(default)]
    pub expansion_type: String,
    #[serde(default)]
    pub estimated_value: String,
    #[serde(default)]
    pub timing: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpansionReadiness {
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub readiness: String,
    #[serde(default)]
    pub primary_risk: String,
    #[serde(default)]
    pub next_action: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YearEndOutlook {
    #[serde(default)]
    pub starting_arr: f64,
    #[serde(default)]
    pub at_risk_arr: f64,
    #[serde(default)]
    pub committed_expansion: f64,
    #[serde(default)]
    pub expected_churn: f64,
    #[serde(default)]
    pub projected_eoy_arr: f64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LandingScenarios {
    #[serde(default)]
    pub best: ScenarioRow,
    #[serde(default)]
    pub expected: ScenarioRow,
    #[serde(default)]
    pub worst: ScenarioRow,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioRow {
    #[serde(default)]
    pub key_assumptions: String,
    #[serde(default)]
    pub attrition: String,
    #[serde(default)]
    pub expansion: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeadershipAsk {
    #[serde(default)]
    pub support_needed: String,
    #[serde(default)]
    pub why_it_matters: String,
    #[serde(default)]
    pub impacted_accounts: Vec<String>,
    #[serde(default)]
    pub dollar_impact: Option<String>,
    #[serde(default)]
    pub timing: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountFocus {
    #[serde(default)]
    pub rank: i32,
    #[serde(default)]
    pub account_name: String,
    #[serde(default)]
    pub arr: f64,
    #[serde(default)]
    pub primary_objective: String,
    #[serde(default)]
    pub key_tactics: Vec<String>,
    #[serde(default)]
    pub success_signals: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarterlyFocus {
    #[serde(default)]
    pub retention: Vec<String>,
    #[serde(default)]
    pub expansion: Vec<String>,
    #[serde(default)]
    pub execution: Vec<String>,
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

fn is_within_n_days(renewal_date: &Option<String>, days: i64) -> bool {
    let date_str = match renewal_date {
        Some(d) if !d.is_empty() => d,
        _ => return false,
    };
    let today = Utc::now().date_naive();
    let cutoff = today + Duration::days(days);
    let parsed = date_str
        .get(..10)
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    match parsed {
        Some(d) => d >= today && d <= cutoff,
        None => false,
    }
}

fn is_within_90_days(renewal_date: &Option<String>) -> bool {
    is_within_n_days(renewal_date, 90)
}

fn is_within_180_days(renewal_date: &Option<String>) -> bool {
    is_within_n_days(renewal_date, 180)
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
// Mechanical builders — instant from DB data (Steps 2, 3, 6, 8)
// =============================================================================

/// Build portfolio health overview from snapshot data. Pure arithmetic.
pub fn build_health_overview(snapshot: &[AccountSnapshotRow]) -> PortfolioHealthOverview {
    let mut healthy_count = 0i32;
    let mut healthy_arr = 0.0f64;
    let mut medium_count = 0i32;
    let mut medium_arr = 0.0f64;
    let mut high_risk_count = 0i32;
    let mut high_risk_arr = 0.0f64;
    let mut renewals_90d = 0i32;
    let mut renewals_90d_arr = 0.0f64;
    let mut renewals_180d = 0i32;
    let mut renewals_180d_arr = 0.0f64;

    for s in snapshot {
        let arr = s.arr.unwrap_or(0.0);
        match s.health_band.as_deref() {
            Some("healthy") => {
                healthy_count += 1;
                healthy_arr += arr;
            }
            Some("watch") => {
                medium_count += 1;
                medium_arr += arr;
            }
            Some("at-risk") => {
                high_risk_count += 1;
                high_risk_arr += arr;
            }
            _ => {
                // Unknown health — count as medium/watch
                medium_count += 1;
                medium_arr += arr;
            }
        }
        if is_within_90_days(&s.renewal_date) {
            renewals_90d += 1;
            renewals_90d_arr += arr;
        }
        if is_within_180_days(&s.renewal_date) {
            renewals_180d += 1;
            renewals_180d_arr += arr;
        }
    }

    // Secure ARR: weighted formula (100% healthy + 75% watch + 40% at-risk)
    let secure_arr = healthy_arr + (medium_arr * 0.75) + (high_risk_arr * 0.4);

    PortfolioHealthOverview {
        healthy_count,
        healthy_arr,
        medium_count,
        medium_arr,
        high_risk_count,
        high_risk_arr,
        secure_arr,
        renewals_90d,
        renewals_90d_arr,
        renewals_180d,
        renewals_180d_arr,
    }
}

/// Build risk accounts table from snapshot + raw intelligence. Sorted by ARR desc.
pub fn build_risk_accounts(
    snapshot: &[AccountSnapshotRow],
    raw_accounts: &[RawAccountRow],
) -> Vec<RiskAccountRow> {
    let mut rows: Vec<RiskAccountRow> = snapshot
        .iter()
        .filter(|s| matches!(s.health_band.as_deref(), Some("at-risk" | "watch")))
        .map(|s| {
            let raw = raw_accounts.iter().find(|r| r.id == s.account_id);
            let primary_driver = raw
                .and_then(|r| r.risks_json.as_deref())
                .and_then(|json| {
                    serde_json::from_str::<Vec<serde_json::Value>>(json)
                        .ok()
                        .and_then(|arr| {
                            arr.first().and_then(|v| {
                                v.get("risk")
                                    .or_else(|| v.get("description"))
                                    .or_else(|| v.get("text"))
                                    .and_then(|s| s.as_str())
                                    .map(|s| truncate(s, 80))
                            })
                        })
                })
                .unwrap_or_default();

            let renewal_timing = s.renewal_date.as_deref().unwrap_or("N/A").to_string();

            let risk_level = s.health_band.as_deref().unwrap_or("unknown").to_string();

            RiskAccountRow {
                account_name: s.account_name.clone(),
                arr: s.arr.unwrap_or(0.0),
                renewal_timing,
                risk_level,
                primary_risk_driver: primary_driver,
            }
        })
        .collect();

    rows.sort_by(|a, b| {
        b.arr
            .partial_cmp(&a.arr)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

/// Build expansion accounts table from raw intelligence. Accounts with wins or
/// positive captures that suggest expansion potential.
pub fn build_expansion_accounts(
    snapshot: &[AccountSnapshotRow],
    raw_accounts: &[RawAccountRow],
) -> Vec<ExpansionRow> {
    let mut rows: Vec<ExpansionRow> = Vec::new();

    for s in snapshot {
        let raw = match raw_accounts.iter().find(|r| r.id == s.account_id) {
            Some(r) => r,
            None => continue,
        };

        // Check for recent wins with expansion signals
        if let Some(ref wins_json) = raw.recent_wins_json {
            if wins_json.len() > 2 {
                if let Ok(wins) = serde_json::from_str::<Vec<serde_json::Value>>(wins_json) {
                    for win in wins.iter().take(1) {
                        let impact = win
                            .get("impact")
                            .or_else(|| win.get("description"))
                            .or_else(|| win.get("text"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if !impact.is_empty() {
                            rows.push(ExpansionRow {
                                account_name: s.account_name.clone(),
                                arr: s.arr.unwrap_or(0.0),
                                readiness: "Exploring".to_string(),
                                expansion_type: "Growth".to_string(),
                                estimated_value: String::new(),
                                timing: String::new(),
                            });
                            break;
                        }
                    }
                }
            }
        }
    }

    rows.sort_by(|a, b| {
        b.arr
            .partial_cmp(&a.arr)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

/// Build year-end outlook from aggregate metrics. Pure arithmetic.
pub fn build_year_end_outlook(total_arr: f64, at_risk_arr: f64) -> YearEndOutlook {
    YearEndOutlook {
        starting_arr: total_arr,
        at_risk_arr,
        committed_expansion: 0.0,                   // User edits this
        expected_churn: 0.0,                        // AI fills or user edits
        projected_eoy_arr: total_arr - at_risk_arr, // Conservative default
    }
}

/// Build top 3 risk one-liners from risk accounts for the exec summary cover.
fn build_top_risks_summary(risk_accounts: &[RiskAccountRow]) -> Vec<String> {
    risk_accounts
        .iter()
        .take(3)
        .map(|r| {
            if r.primary_risk_driver.is_empty() {
                format!(
                    "{} — ${:.0}k {}",
                    r.account_name,
                    r.arr / 1000.0,
                    r.risk_level
                )
            } else {
                format!("{} — {}", r.account_name, r.primary_risk_driver)
            }
        })
        .collect()
}

/// Build top 3 opportunity one-liners from expansion accounts.
fn build_top_opportunities_summary(expansion_accounts: &[ExpansionRow]) -> Vec<String> {
    expansion_accounts
        .iter()
        .take(3)
        .map(|e| {
            format!(
                "{} — ${:.0}k {}",
                e.account_name,
                e.arr / 1000.0,
                if e.expansion_type.is_empty() {
                    "expansion potential"
                } else {
                    &e.expansion_type
                }
            )
        })
        .collect()
}

/// Find the biggest at-risk account for the exec summary.
fn find_biggest_risk(risk_accounts: &[RiskAccountRow]) -> Option<BiggestItem> {
    risk_accounts.first().map(|r| BiggestItem {
        account_name: r.account_name.clone(),
        arr: r.arr,
        description: r.primary_risk_driver.clone(),
    })
}

/// Find the biggest expansion opportunity for the exec summary.
fn find_biggest_upside(expansion_accounts: &[ExpansionRow]) -> Option<BiggestItem> {
    expansion_accounts.first().map(|e| BiggestItem {
        account_name: e.account_name.clone(),
        arr: e.arr,
        description: e.expansion_type.clone(),
    })
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
    let gather = gather_book_of_business_data(
        workspace,
        db,
        ai_models,
        active_preset,
        spotlight_account_ids,
    )?;
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

    let emit_account = |prompt: &mut String,
                        snap: &AccountSnapshotRow,
                        tier: usize,
                        indent: &str| {
        let raw = raw_accounts.iter().find(|r| r.id == snap.account_id);
        let assessment = raw
            .and_then(|r| r.executive_assessment.as_deref())
            .unwrap_or("");

        let arr_str = snap
            .arr
            .map(|a| format!("${:.0}", a))
            .unwrap_or_else(|| "N/A".to_string());
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
                    if risks.len() > 2 {
                        // not empty "[]"
                        prompt.push_str(&format!(
                            "{}Known risks: {}\n",
                            indent,
                            truncate(risks, 300)
                        ));
                    }
                }
                if let Some(ref wins) = r.recent_wins_json {
                    if wins.len() > 2 {
                        prompt.push_str(&format!(
                            "{}Recent wins: {}\n",
                            indent,
                            truncate(wins, 300)
                        ));
                    }
                }
                if let Some(ref val) = r.value_delivered {
                    if !val.is_empty() {
                        prompt.push_str(&format!(
                            "{}Value delivered: {}\n",
                            indent,
                            truncate(val, 200)
                        ));
                    }
                }
                if let Some(ref commits) = r.open_commitments {
                    if !commits.is_empty() {
                        prompt.push_str(&format!(
                            "{}Open commitments: {}\n",
                            indent,
                            truncate(commits, 200)
                        ));
                    }
                }
                if let Some(ref stakeholders) = r.stakeholder_insights_json {
                    if stakeholders.len() > 2 {
                        prompt.push_str(&format!(
                            "{}Stakeholder insights: {}\n",
                            indent,
                            truncate(stakeholders, 300)
                        ));
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
                band_str,
                arr_str,
                renewal_str,
                snap.meeting_count_90d,
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
                band_str,
                arr_str,
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
            bu_count,
            total_arr,
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
    prompt.push_str(
        "Below is the full intelligence context for each account that needs a deep dive.\n",
    );
    prompt.push_str("Use this data to write the statusNarrative, renewalOrGrowthImpact, activeWorkstreams, and risksAndGaps.\n\n");

    for id in spotlight_ids {
        let snap = match snapshot.iter().find(|s| s.account_id == *id) {
            Some(s) => s,
            None => continue,
        };
        let raw = raw_accounts.iter().find(|r| r.id == *id);

        let arr_str = snap
            .arr
            .map(|a| format!("${:.0}", a))
            .unwrap_or_else(|| "N/A".to_string());
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

/// Progress phases emitted during generation.
const BOB_PROGRESS_PHASES: &[&str] = &[
    "healthOverview",
    "riskAccounts",
    "expansionAccounts",
    "yearEndOutlook",
    "synthesis",
];

/// Build the single synthesis prompt. Contains pre-computed mechanical data +
/// per-spotlight intelligence. AI generates all narrative sections in one call.
fn build_synthesis_prompt(
    gather: &BookGatherOutput,
    glean_ctx: &GleanPortfolioContext,
    risk_accounts: &[RiskAccountRow],
    expansion_accounts: &[ExpansionRow],
) -> String {
    let mut prompt = String::with_capacity(8192);
    prompt.push_str("You are a senior customer success strategist preparing a Book of Business review for leadership.\n");
    prompt.push_str("Ground every claim in the data provided. Use executive-ready language.\n\n");

    // Portfolio metrics summary (compact)
    prompt.push_str(&format!(
        "# Portfolio: {}\n\n\
         Total accounts: {} | Total ARR: ${:.0} | At-risk ARR: ${:.0}\n\n",
        gather.metrics.period_label,
        gather.metrics.total_accounts,
        gather.metrics.total_arr,
        gather.metrics.at_risk_arr,
    ));

    // Risk accounts (pre-computed table data)
    if !risk_accounts.is_empty() {
        prompt.push_str("## Risk Accounts (pre-computed)\n");
        for r in risk_accounts {
            prompt.push_str(&format!(
                "- {} | ARR: ${:.0} | Renewal: {} | Risk: {} | Driver: {}\n",
                crate::util::sanitize_external_field(&r.account_name),
                r.arr,
                r.renewal_timing,
                r.risk_level,
                if r.primary_risk_driver.is_empty() {
                    "—"
                } else {
                    &r.primary_risk_driver
                },
            ));
        }
        prompt.push('\n');
    }

    // Expansion accounts (pre-computed table data)
    if !expansion_accounts.is_empty() {
        prompt.push_str("## Expansion Accounts (pre-computed)\n");
        for e in expansion_accounts {
            prompt.push_str(&format!(
                "- {} | ARR: ${:.0} | Type: {}\n",
                crate::util::sanitize_external_field(&e.account_name),
                e.arr,
                if e.expansion_type.is_empty() {
                    "Growth"
                } else {
                    &e.expansion_type
                },
            ));
        }
        prompt.push('\n');
    }

    // Spotlight account intelligence (deep context for top accounts)
    if !gather.spotlight_ids.is_empty() {
        prompt.push_str(&build_spotlight_detail_block(
            &gather.raw_accounts,
            &gather.snapshot,
            &gather.spotlight_ids,
        ));
    } else {
        // No spotlights — include top 5 accounts by ARR for focus
        let top_ids: Vec<String> = gather
            .snapshot
            .iter()
            .take(5)
            .map(|s| s.account_id.clone())
            .collect();
        if !top_ids.is_empty() {
            prompt.push_str(&build_spotlight_detail_block(
                &gather.raw_accounts,
                &gather.snapshot,
                &top_ids,
            ));
        }
    }

    // User context (I413)
    if !gather.user_context_block.is_empty() {
        prompt.push_str(&gather.user_context_block);
    }

    // Glean context (all available)
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

    // Output schema — all AI sections in one JSON
    prompt.push_str("## Output Format\n\n");
    prompt.push_str("Respond with ONLY valid JSON (no markdown fences) matching this schema:\n\n");
    prompt.push_str(r#"{
  "executiveSummary": "2-4 sentence portfolio narrative for the opening slide",
  "projectedChurn": 0.0,
  "eltHelpRequired": true,
  "retentionRiskDeepDives": [
    { "accountName": "Name", "arr": 100000.0, "whyAtRisk": "Why this account is at risk", "saveConfidence": "High/Medium/Low", "next90Days": "What happens in the next 90 days", "keyTactics": ["Tactic 1"], "successSignals": ["Signal 1"], "helpNeeded": ["Help 1"] }
  ],
  "saveMotions": [
    { "accountName": "Name", "risk": "The risk", "saveMotion": "The save motion", "timeline": "Q2 2026", "successSignals": "What success looks like" }
  ],
  "expansionReadiness": [
    { "accountName": "Name", "readiness": "High/Medium/Low", "primaryRisk": "Main risk to expansion", "nextAction": "Next step" }
  ],
  "landingScenarios": {
    "best": { "keyAssumptions": "...", "attrition": "$0", "expansion": "$X", "notes": "..." },
    "expected": { "keyAssumptions": "...", "attrition": "$X", "expansion": "$Y", "notes": "..." },
    "worst": { "keyAssumptions": "...", "attrition": "$X", "expansion": "$0", "notes": "..." }
  },
  "leadershipAsks": [
    { "supportNeeded": "What you need", "whyItMatters": "Why", "impactedAccounts": ["Account A"], "dollarImpact": "$X or null", "timing": "When" }
  ],
  "accountFocus": [
    { "rank": 1, "accountName": "Name", "arr": 100000.0, "primaryObjective": "The objective", "keyTactics": ["Tactic 1"], "successSignals": ["Signal 1"] }
  ],
  "quarterlyFocus": {
    "retention": ["Priority 1"],
    "expansion": ["Priority 1"],
    "execution": ["Priority 1"]
  },
  "keyThemes": [
    { "title": "Theme", "narrative": "2-3 sentences", "citedAccounts": ["Account A"] }
  ]
}"#);

    prompt.push_str("\n\n## Rules\n\n");
    prompt.push_str(
        "This report is a leadership slide deck. Write content that reads like presentation talking points.\n\n\
         - executiveSummary: 2-4 sentences, portfolio-level. The opening statement walking into the room.\n\
         - projectedChurn: estimated churn ARR based on risk analysis. Use 0 if unknown.\n\
         - eltHelpRequired: true if any leadership ask is present.\n\
         - retentionRiskDeepDives: top 2-3 at-risk accounts with strategic detail.\n\
         - saveMotions: one row per at-risk account with a concrete save motion.\n\
         - expansionReadiness: one row per expansion account with readiness assessment.\n\
         - landingScenarios: best/expected/worst year-end outcomes with assumptions.\n\
         - leadershipAsks: 1-3 specific asks with dollar impact and timing.\n\
         - accountFocus: top 5 accounts ranked by importance with objective-driven focus.\n\
         - quarterlyFocus: 2-3 bullets each for retention, expansion, execution priorities.\n\
         - keyThemes: 2-3 cross-portfolio patterns.\n\
         - Do NOT fabricate data, dates, or commitments.\n\
         - Do NOT mention AI, enrichment, signals, or internal mechanics.\n\
         - Write for the ear, not the page. Short sentences. Active voice.\n",
    );

    prompt
}

/// Build a per-account deep dive prompt. (Legacy — kept for monolithic fallback.)
#[allow(dead_code)]
fn build_single_deep_dive_prompt(
    gather: &BookGatherOutput,
    account_id: &str,
    glean_ctx: &GleanPortfolioContext,
) -> Option<String> {
    let snap = gather
        .snapshot
        .iter()
        .find(|s| s.account_id == account_id)?;
    let raw = gather.raw_accounts.iter().find(|r| r.id == account_id);

    let mut prompt = String::with_capacity(4096);
    prompt.push_str("You are a senior customer success strategist preparing a deep dive slide for a leadership presentation.\n");
    prompt.push_str("Ground every claim in the data provided. Use executive-ready language.\n\n");

    let arr_str = snap
        .arr
        .map(|a| format!("${:.0}", a))
        .unwrap_or_else(|| "N/A".to_string());
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
    ]
    .iter()
    .copied()
    .flatten()
    .collect();
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

/// Build the executiveSummary prompt using Wave 1 results as context. (Legacy — kept for monolithic fallback.)
#[allow(dead_code)]
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

/// Run BoB generation: mechanical sections (instant) + one synthesis PTY call.
/// Emits progressive `bob-section-progress` events via AppHandle.
pub fn run_bob_generation(
    gather: &BookGatherOutput,
    glean_ctx: &GleanPortfolioContext,
    _metrics: &BookMetrics,
    app_handle: Option<&AppHandle>,
) -> Result<AiBookResponse, String> {
    let overall_start = Instant::now();
    let total_phases = BOB_PROGRESS_PHASES.len() as u32;

    // Emit helper
    let emit_progress = |handle: Option<&AppHandle>, phase: &str, completed: u32| {
        if let Some(h) = handle {
            let _ = h.emit(
                "bob-section-progress",
                BobSectionProgress {
                    section_name: phase.to_string(),
                    completed,
                    total: total_phases,
                    wave: 1,
                },
            );
        }
    };

    // Phase 1: Instant — mechanical sections from DB
    let health_overview = build_health_overview(&gather.snapshot);
    emit_progress(app_handle, "healthOverview", 1);

    let risk_accounts = build_risk_accounts(&gather.snapshot, &gather.raw_accounts);
    emit_progress(app_handle, "riskAccounts", 2);

    let expansion_accounts = build_expansion_accounts(&gather.snapshot, &gather.raw_accounts);
    emit_progress(app_handle, "expansionAccounts", 3);

    let _year_end_outlook =
        build_year_end_outlook(gather.metrics.total_arr, gather.metrics.at_risk_arr);
    emit_progress(app_handle, "yearEndOutlook", 4);

    log::info!(
        "[BoB] Mechanical sections completed in {}ms (health: {}/{}/{}, risk: {}, expansion: {})",
        overall_start.elapsed().as_millis(),
        health_overview.healthy_count,
        health_overview.medium_count,
        health_overview.high_risk_count,
        risk_accounts.len(),
        expansion_accounts.len(),
    );

    // Step 2: One AI synthesis call for all narrative sections
    let synthesis_start = Instant::now();
    let synthesis_prompt =
        build_synthesis_prompt(gather, glean_ctx, &risk_accounts, &expansion_accounts);

    let pty = PtyManager::for_tier(ModelTier::Synthesis, &gather.ai_models)
        .with_timeout(90)
        .with_nice_priority(10);

    let response = match pty.spawn_claude(&gather.workspace, &synthesis_prompt) {
        Ok(output) => {
            let json_str =
                crate::risk_briefing::extract_json_object(&output.stdout).ok_or_else(|| {
                    format!(
                        "No JSON in BoB synthesis response ({}ms)",
                        synthesis_start.elapsed().as_millis()
                    )
                })?;

            let ai: AiBookResponse = serde_json::from_str(&json_str)
                .map_err(|e| format!("Failed to parse BoB synthesis JSON: {}", e))?;

            log::info!(
                "[BoB] Synthesis completed in {}ms — themes: {}, asks: {}, focus: {}",
                synthesis_start.elapsed().as_millis(),
                ai.key_themes.len(),
                ai.leadership_asks.len(),
                ai.account_focus.len(),
            );

            ai
        }
        Err(e) => {
            log::warn!("[BoB] Synthesis PTY failed: {}", e);
            // Return a default response with mechanical data only
            AiBookResponse {
                executive_summary: "Portfolio review generated — AI synthesis unavailable."
                    .to_string(),
                ..Default::default()
            }
        }
    };

    emit_progress(app_handle, "synthesis", 5);

    let total_ms = overall_start.elapsed().as_millis();
    log::info!(
        "[BoB] Total generation: {}ms (mechanical: instant, synthesis: {}ms)",
        total_ms,
        synthesis_start.elapsed().as_millis(),
    );

    // Return the AI response — caller assembles with mechanical data
    // Store mechanical data in a side channel so the caller can use it
    // (We return AiBookResponse for API compat; caller calls assemble_book_content)
    Ok(response)
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
                let client = crate::context_provider::glean::GleanMcpClient::new(&ep);
                tokio::time::timeout(
                    std::time::Duration::from_secs(15),
                    client.chat(&query, None),
                )
                .await
            });

            let text = match result {
                Ok(Ok(text)) => {
                    log::info!("[I547] Glean {} pre-fetch: {} chars", key, text.len());
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

/// Intermediate struct for parsing AI synthesis response.
/// One call generates all AI sections together.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiBookResponse {
    #[serde(default)]
    pub executive_summary: String,
    #[serde(default)]
    pub projected_churn: f64,
    #[serde(default)]
    pub elt_help_required: bool,
    #[serde(default)]
    pub retention_risk_deep_dives: Vec<RetentionRiskDeepDive>,
    #[serde(default)]
    pub save_motions: Vec<SaveMotion>,
    #[serde(default)]
    pub expansion_readiness: Vec<ExpansionReadiness>,
    #[serde(default)]
    pub landing_scenarios: LandingScenarios,
    #[serde(default)]
    pub leadership_asks: Vec<LeadershipAsk>,
    #[serde(default)]
    pub account_focus: Vec<AccountFocus>,
    #[serde(default)]
    pub quarterly_focus: QuarterlyFocus,
    #[serde(default)]
    pub key_themes: Vec<BookTheme>,
}

/// Assemble final content from mechanical sections + AI synthesis.
pub fn assemble_book_content(
    ai: AiBookResponse,
    metrics: BookMetrics,
    health_overview: PortfolioHealthOverview,
    risk_accounts: Vec<RiskAccountRow>,
    expansion_accounts: Vec<ExpansionRow>,
    year_end_outlook: YearEndOutlook,
) -> BookOfBusinessContent {
    let top_risks_summary = build_top_risks_summary(&risk_accounts);
    let top_opportunities_summary = build_top_opportunities_summary(&expansion_accounts);
    let biggest_risk = find_biggest_risk(&risk_accounts);
    let biggest_upside = find_biggest_upside(&expansion_accounts);

    BookOfBusinessContent {
        period_label: metrics.period_label,
        executive_summary: ai.executive_summary,
        total_accounts: metrics.total_accounts,
        total_arr: metrics.total_arr,
        at_risk_arr: metrics.at_risk_arr,
        committed_expansion: 0.0,
        projected_churn: ai.projected_churn,
        top_risks_summary,
        top_opportunities_summary,
        biggest_risk,
        biggest_upside,
        elt_help_required: ai.elt_help_required,
        health_overview,
        risk_accounts,
        retention_risk_deep_dives: ai.retention_risk_deep_dives,
        save_motions: ai.save_motions,
        expansion_accounts,
        expansion_readiness: ai.expansion_readiness,
        year_end_outlook,
        landing_scenarios: ai.landing_scenarios,
        leadership_asks: ai.leadership_asks,
        account_focus: ai.account_focus,
        quarterly_focus: ai.quarterly_focus,
        key_themes: ai.key_themes,
        account_snapshot: metrics.account_snapshot,
    }
}

pub fn parse_book_of_business_response(
    stdout: &str,
    metrics: BookMetrics,
    health_overview: PortfolioHealthOverview,
    risk_accounts: Vec<RiskAccountRow>,
    expansion_accounts: Vec<ExpansionRow>,
    year_end_outlook: YearEndOutlook,
) -> Result<BookOfBusinessContent, String> {
    let json_str = crate::risk_briefing::extract_json_object(stdout)
        .ok_or_else(|| "No valid JSON object found in Book of Business response".to_string())?;

    let ai: AiBookResponse = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse Book of Business JSON: {}", e))?;

    Ok(assemble_book_content(
        ai,
        metrics,
        health_overview,
        risk_accounts,
        expansion_accounts,
        year_end_outlook,
    ))
}

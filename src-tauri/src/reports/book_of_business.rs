//! Book of Business report (I547).
//!
//! Cross-account portfolio report. Gathers all active accounts with health,
//! ARR, renewal data, and activity metrics. AI generates narrative analysis;
//! metrics and snapshot are pre-computed from DB data.

use chrono::{Datelike, Duration, Utc};

use crate::db::ActionDb;
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
    pub account_name: String,
    pub risk: String,
    pub arr: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookOpportunityItem {
    pub account_name: String,
    pub opportunity: String,
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
    pub account_name: String,
    pub account_id: String,
    pub arr: Option<f64>,
    pub renewal_date: Option<String>,
    pub status_narrative: String,
    pub renewal_or_growth_impact: String,
    pub active_workstreams: Vec<String>,
    pub risks_and_gaps: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueDeliveredRow {
    pub account_name: String,
    pub headline_outcome: String,
    pub why_it_matters: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookTheme {
    pub title: String,
    pub narrative: String,
    pub cited_accounts: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeadershipAsk {
    pub ask: String,
    pub context: String,
    pub impacted_accounts: Vec<String>,
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
// Data gathering (Phase 1)
// =============================================================================

/// Internal struct to hold raw account data from the DB before building snapshot rows.
struct RawAccountRow {
    id: String,
    name: String,
    arr: Option<f64>,
    contract_end: Option<String>,
    lifecycle: Option<String>,
    executive_assessment: Option<String>,
    health_score: Option<f64>,
    health_trend: Option<String>,
    parent_id: Option<String>,
    /// User-set health RAG: "green", "yellow", "red" (primary at-risk indicator)
    manual_health: Option<String>,
}

pub fn gather_book_of_business_input(
    workspace: &std::path::Path,
    db: &ActionDb,
    ai_models: AiModelConfig,
    active_preset: &str,
    spotlight_account_ids: Option<&[String]>,
) -> Result<ReportGeneratorInput, String> {
    // 1. All active accounts with health/ARR/renewal
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT a.id, a.name, a.arr, a.contract_end, a.lifecycle,
                    ea.executive_assessment, eq.health_score, eq.health_trend,
                    a.parent_id, a.health
             FROM accounts a
             LEFT JOIN entity_assessment ea ON ea.entity_id = a.id
             LEFT JOIN entity_quality eq ON eq.entity_id = a.id
             WHERE a.archived = 0
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

    // 4. Top 20 open actions across all accounts
    let open_actions: String = db
        .conn_ref()
        .prepare(
            "SELECT act.title, a.name FROM actions act
             LEFT JOIN accounts a ON a.id = act.entity_id
             WHERE act.status = 'open' AND act.entity_type = 'account'
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

    // 5. Email signal counts per account (recent 90d)
    let email_signals: String = db
        .conn_ref()
        .prepare(
            "SELECT a.name, COUNT(*) as cnt
             FROM signal_events se
             JOIN accounts a ON a.id = se.entity_id
             WHERE se.entity_type = 'account'
               AND se.signal_type LIKE '%email%'
               AND se.created_at >= ?1
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
        period_label: period_label.clone(),
        total_accounts,
        total_arr,
        at_risk_arr,
        upcoming_renewals,
        upcoming_renewals_arr,
        account_snapshot: snapshot.clone(),
    };

    let extra_data = serde_json::to_string(&metrics)
        .map_err(|e| format!("Failed to serialize BookMetrics: {}", e))?;

    // Build prompt
    let prompt = build_book_of_business_prompt(
        &raw_accounts,
        &snapshot,
        &open_actions,
        &email_signals,
        &user_name,
        &user_role,
        active_preset,
        &period_label,
        &metrics,
        spotlight_account_ids,
    );

    let intel_hash = compute_aggregate_intel_hash(db);

    let user_entity_id: String = db
        .conn_ref()
        .query_row(
            "SELECT CAST(id AS TEXT) FROM user_entity LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "1".to_string());

    Ok(ReportGeneratorInput {
        entity_id: user_entity_id,
        entity_type: "user".to_string(),
        report_type: "book_of_business".to_string(),
        entity_name: "Book of Business".to_string(),
        workspace: workspace.to_path_buf(),
        prompt,
        ai_models,
        intel_hash,
        extra_data: Some(extra_data),
    })
}

// =============================================================================
// Prompt building
// =============================================================================

#[allow(clippy::too_many_arguments)]
fn build_book_of_business_prompt(
    raw_accounts: &[RawAccountRow],
    snapshot: &[AccountSnapshotRow],
    open_actions: &str,
    email_signals: &str,
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

    // Per-account data — grouped by parent/child hierarchy
    prompt.push_str("## Account Details\n\n");

    // Track which accounts have been emitted (to avoid duplicates in hierarchy)
    let mut emitted: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Helper: build per-account context string
    let emit_account = |prompt: &mut String, snap: &AccountSnapshotRow, tier: usize, indent: &str| {
        let assessment = raw_accounts
            .iter()
            .find(|r| r.id == snap.account_id)
            .and_then(|r| r.executive_assessment.as_deref())
            .unwrap_or("");

        let arr_str = snap.arr.map(|a| format!("${:.0}", a)).unwrap_or_else(|| "N/A".to_string());
        let renewal_str = snap.renewal_date.as_deref().unwrap_or("N/A");
        let lifecycle_str = snap.lifecycle.as_deref().unwrap_or("N/A");
        let contact_str = snap.key_contact.as_deref().unwrap_or("N/A");
        let band_str = snap.health_band.as_deref().unwrap_or("unknown");

        if tier < 10 {
            let excerpt = truncate(assessment, 500);
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
            if !excerpt.is_empty() {
                prompt.push_str(&crate::util::wrap_user_data(&excerpt));
                prompt.push('\n');
            }
            prompt.push('\n');
        } else if tier < 20 {
            let para = first_paragraph(assessment, 200);
            prompt.push_str(&format!(
                "{}**{}** ({}) | ARR: {} | Renewal: {} | Meetings: {}\n",
                indent,
                crate::util::sanitize_external_field(&snap.account_name),
                band_str, arr_str, renewal_str, snap.meeting_count_90d,
            ));
            if !para.is_empty() {
                prompt.push_str(&crate::util::wrap_user_data(&para));
                prompt.push('\n');
            }
            prompt.push('\n');
        } else {
            prompt.push_str(&format!(
                "{}- {} | {} | ARR: {}\n",
                indent,
                crate::util::sanitize_external_field(&snap.account_name),
                band_str, arr_str,
            ));
        }
    };

    // First: emit parent groups (parent header + children)
    let mut tier_idx = 0usize;
    for snap in snapshot.iter() {
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
        // Emit parent's own data
        emit_account(&mut prompt, snap, tier_idx, "");
        emitted.insert(snap.account_id.clone());
        tier_idx += 1;

        // Emit children sorted by ARR desc
        let mut children: Vec<&AccountSnapshotRow> = snapshot
            .iter()
            .filter(|s| s.parent_id.as_deref() == Some(&snap.account_id))
            .collect();
        children.sort_by(|a, b| {
            b.arr.unwrap_or(0.0).partial_cmp(&a.arr.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for child in children {
            emit_account(&mut prompt, child, tier_idx, "  ");
            emitted.insert(child.account_id.clone());
            tier_idx += 1;
        }
        prompt.push('\n');
    }

    // Then: emit standalone accounts (not parent, not child)
    for snap in snapshot.iter() {
        if emitted.contains(&snap.account_id) {
            continue;
        }
        emit_account(&mut prompt, snap, tier_idx, "");
        tier_idx += 1;
    }
    prompt.push('\n');

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
         - Write for the ear, not the page. Short sentences. Active voice. No jargon.\n",
    );

    prompt
}

// =============================================================================
// Response parsing
// =============================================================================

/// Intermediate struct for parsing AI-generated fields only.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiBookResponse {
    #[serde(default)]
    executive_summary: String,
    #[serde(default)]
    top_risks: Vec<BookRiskItem>,
    #[serde(default)]
    top_opportunities: Vec<BookOpportunityItem>,
    #[serde(default)]
    deep_dives: Vec<AccountDeepDive>,
    #[serde(default)]
    value_delivered: Vec<ValueDeliveredRow>,
    #[serde(default)]
    key_themes: Vec<BookTheme>,
    #[serde(default)]
    leadership_asks: Vec<LeadershipAsk>,
}

pub fn parse_book_of_business_response(
    stdout: &str,
    metrics: BookMetrics,
) -> Result<BookOfBusinessContent, String> {
    let json_str = crate::risk_briefing::extract_json_object(stdout)
        .ok_or_else(|| "No valid JSON object found in Book of Business response".to_string())?;

    let ai: AiBookResponse = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse Book of Business JSON: {}", e))?;

    let has_leadership_asks = !ai.leadership_asks.is_empty();

    Ok(BookOfBusinessContent {
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
    })
}

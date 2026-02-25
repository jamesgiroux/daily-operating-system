//! Monthly Wrapped report (I419).
//!
//! Prior calendar month. Warmer, reflective tone.
//! Honest miss: highest-importance priority with zero activity shown prominently.
//! Comparison with prior month if that report exists.

use chrono::{Datelike, NaiveDate, Utc};

use crate::db::ActionDb;
use crate::reports::generator::ReportGeneratorInput;
use crate::reports::prompts::build_report_preamble;
use crate::types::AiModelConfig;

// =============================================================================
// Output schema
// =============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyWin {
    pub headline: String,
    pub detail: Option<String>,
    pub source: String, // meeting ID or date — required
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriorityProgress {
    pub priority_text: String,
    pub progress: String,          // "strong", "some", "none"
    pub evidence: Option<String>,  // What happened, or null if none
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyWrappedContent {
    pub month_label: String,               // e.g., "January 2026"
    pub headline_stat: String,             // "X meetings, Y actions, Z signals"
    pub opening_reflection: String,        // 1-2 sentences, warm tone
    pub top_wins: Vec<MonthlyWin>,         // 2-4 wins with real citations
    pub priority_progress: Vec<PriorityProgress>, // ALL priorities — no omissions
    pub honest_miss: Option<String>,       // Highest-importance priority with zero activity
    pub momentum_builder: String,          // 1-2 sentences looking forward
    pub by_the_numbers: Vec<String>,       // Simple stats: ["X meetings", "Y actions closed"]
}

// =============================================================================
// Time range helpers
// =============================================================================

/// Returns (first_day, last_day) of the prior calendar month.
pub fn prior_calendar_month() -> (NaiveDate, NaiveDate) {
    let today = Utc::now().date_naive();
    let first_of_this_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .unwrap_or(today);
    let last_of_prior = first_of_this_month.pred_opt().unwrap_or(today);
    let first_of_prior = NaiveDate::from_ymd_opt(last_of_prior.year(), last_of_prior.month(), 1)
        .unwrap_or(last_of_prior);
    (first_of_prior, last_of_prior)
}

// =============================================================================
// Prompt
// =============================================================================

fn build_monthly_wrapped_prompt(
    db: &ActionDb,
    month_start: NaiveDate,
    month_end: NaiveDate,
) -> String {
    let month_start_str = month_start.format("%Y-%m-%d").to_string();
    let month_end_str = format!("{} 23:59:59", month_end.format("%Y-%m-%d"));
    let month_label = month_start.format("%B %Y").to_string();

    // Gather user priorities
    let priorities_json: String = db
        .conn_ref()
        .query_row(
            "SELECT COALESCE(annual_priorities, '[]') || ' | ' || COALESCE(quarterly_priorities, '[]') FROM user_entity LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    // Meetings for the month
    let meeting_count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM meetings_history WHERE start_time >= ?1 AND start_time <= ?2 AND meeting_type NOT IN ('personal', 'focus', 'blocked')",
            rusqlite::params![month_start_str, month_end_str],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let meetings: String = db
        .conn_ref()
        .prepare(
            "SELECT id, title, start_time, meeting_type FROM meetings_history
             WHERE start_time >= ?1 AND start_time <= ?2
               AND meeting_type NOT IN ('personal', 'focus', 'blocked')
             ORDER BY start_time",
        )
        .and_then(|mut s| {
            let rows = s.query_map(
                rusqlite::params![month_start_str, month_end_str],
                |row| {
                    let id: String = row.get(0)?;
                    let title: String = row.get(1)?;
                    let time: String = row.get(2)?;
                    Ok(format!("- [{}] {} | {}", id, time, title))
                },
            )?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    // Completed actions for the month
    let action_count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM actions WHERE completed_at >= ?1 AND completed_at <= ?2",
            rusqlite::params![month_start_str, month_end_str],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let completed_actions: String = db
        .conn_ref()
        .prepare(
            "SELECT title, completed_at FROM actions
             WHERE completed_at >= ?1 AND completed_at <= ?2
             ORDER BY completed_at",
        )
        .and_then(|mut s| {
            let rows = s.query_map(
                rusqlite::params![month_start_str, month_end_str],
                |row| {
                    let title: String = row.get(0)?;
                    let completed: String = row.get(1)?;
                    Ok(format!("- {} (completed {})", title, completed))
                },
            )?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    // Signal events for the month
    let signal_count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM signal_events WHERE created_at >= ?1 AND created_at <= ?2",
            rusqlite::params![month_start_str, month_end_str],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let signals: String = db
        .conn_ref()
        .prepare(
            "SELECT se.signal_type, se.value, se.source, COALESCE(a.name, p.name, '') as entity_name
             FROM signal_events se
             LEFT JOIN accounts a ON se.entity_id = a.id AND se.entity_type = 'account'
             LEFT JOIN people p ON se.entity_id = p.id AND se.entity_type = 'person'
             WHERE se.created_at >= ?1 AND se.created_at <= ?2
             ORDER BY se.confidence DESC
             LIMIT 30",
        )
        .and_then(|mut s| {
            let rows = s.query_map(
                rusqlite::params![month_start_str, month_end_str],
                |row| {
                    let stype: String = row.get(0)?;
                    let val: String = row.get::<_, Option<String>>(1)?.unwrap_or_default();
                    let entity: String = row.get(3)?;
                    Ok(format!("- [{}] {} — {}", stype, entity, val))
                },
            )?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    let mut prompt = build_report_preamble("you", "monthly_wrapped", "user");

    prompt.push_str(&format!("# Month: {}\n\n", month_label));
    prompt.push_str(&format!(
        "## Stats at a Glance\n- {} meetings\n- {} actions completed\n- {} intelligence signals\n\n",
        meeting_count, action_count, signal_count
    ));

    if !priorities_json.trim().is_empty() {
        prompt.push_str("## Your Priorities\n");
        prompt.push_str(&crate::util::wrap_user_data(&priorities_json));
        prompt.push_str("\n\n");
    }

    if !meetings.is_empty() {
        prompt.push_str("## Meetings\n");
        prompt.push_str(&crate::util::wrap_user_data(&meetings));
        prompt.push_str("\n\n");
    }

    if !completed_actions.is_empty() {
        prompt.push_str("## Completed Actions\n");
        prompt.push_str(&crate::util::wrap_user_data(&completed_actions));
        prompt.push_str("\n\n");
    }

    if !signals.is_empty() {
        prompt.push_str("## Intelligence Signals\n");
        prompt.push_str(&crate::util::wrap_user_data(&signals));
        prompt.push_str("\n\n");
    }

    prompt.push_str("# Output Format\n\n");
    prompt.push_str("Write with warmth and honesty — this is a personal reflection, not a performance review.\n");
    prompt.push_str("Respond with ONLY a valid JSON object (no markdown fences):\n\n");
    prompt.push_str(&format!(
        r#"{{
  "monthLabel": "{month_label}",
  "headlineStat": "{meeting_count} meetings, {action_count} actions, {signal_count} signals — one phrase",
  "openingReflection": "1-2 warm sentences summarizing the month's overall character. Not a list.",
  "topWins": [
    {{
      "headline": "Win in max 12 words",
      "detail": "1 sentence detail or null",
      "source": "meeting-id or date — REQUIRED"
    }}
  ],
  "priorityProgress": [
    {{
      "priorityText": "The exact priority text",
      "progress": "strong|some|none",
      "evidence": "What happened this month or null if none"
    }}
  ],
  "honestMiss": "If any priority has progress=none, state the highest-importance one honestly. null if all made progress.",
  "momentumBuilder": "1-2 sentences looking forward to next month without making promises.",
  "byTheNumbers": ["{meeting_count} external meetings", "{action_count} actions closed", "{signal_count} intelligence signals"]
}}"#,
        month_label = month_label,
        meeting_count = meeting_count,
        action_count = action_count,
        signal_count = signal_count
    ));

    prompt.push_str("\n\n# Rules\n");
    prompt.push_str("- priority_progress: Include ALL priorities. Never omit one with zero activity.\n");
    prompt.push_str("- honest_miss: If any priority has progress='none', set honest_miss to the most important one. Don't soften it — be honest.\n");
    prompt.push_str("- top_wins: 2–4 wins with REAL citations. source must be a meeting ID from the meetings list above.\n");
    prompt.push_str("- tone: Personal and reflective, not corporate. Like a good journal entry.\n");

    prompt
}

// =============================================================================
// Generation input (Phase 1)
// =============================================================================

pub fn gather_monthly_wrapped_input(
    workspace: &std::path::Path,
    db: &ActionDb,
    ai_models: AiModelConfig,
) -> Result<ReportGeneratorInput, String> {
    let (month_start, month_end) = prior_calendar_month();
    let intel_hash = format!("month-{}", month_start.format("%Y-%m"));
    let prompt = build_monthly_wrapped_prompt(db, month_start, month_end);

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
        report_type: "monthly_wrapped".to_string(),
        entity_name: "Monthly Wrapped".to_string(),
        workspace: workspace.to_path_buf(),
        prompt,
        ai_models,
        intel_hash,
    })
}

// =============================================================================
// Response parsing
// =============================================================================

pub fn parse_monthly_wrapped_response(stdout: &str) -> Result<MonthlyWrappedContent, String> {
    let json_str = crate::risk_briefing::extract_json_object(stdout)
        .ok_or_else(|| "No valid JSON object found in Monthly Wrapped response".to_string())?;

    serde_json::from_str::<MonthlyWrappedContent>(&json_str)
        .map_err(|e| format!("Failed to parse Monthly Wrapped JSON: {}", e))
}

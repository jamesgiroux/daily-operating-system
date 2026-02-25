//! Weekly Impact Report (I418).
//!
//! Covers the prior Mon–Sun work week. Gathers user priorities,
//! signal events, meetings history, and completed actions.
//! Quality gate: priorities_moved must cite real event IDs.

use chrono::{Datelike, Duration, NaiveDate, Utc};

use crate::db::ActionDb;
use crate::reports::generator::ReportGeneratorInput;
use crate::reports::prompts::build_report_preamble;
use crate::types::AiModelConfig;

// =============================================================================
// Output schema
// =============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriorityMove {
    pub priority_text: String,
    pub what_happened: String,
    pub source: String, // meeting ID or signal ID — MUST be non-null
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyImpactContent {
    pub week_label: String,        // e.g., "Feb 17–21, 2026"
    pub headline_stat: String,     // e.g., "7 meetings, 3 actions closed"
    pub priorities_moved: Vec<PriorityMove>,
    pub wins: Vec<String>,
    pub activity_summary: String,  // Volume: what you did in 2 sentences
    pub watch: Vec<String>,        // Things to keep an eye on
    pub carry_forward: Vec<String>, // Priority items with zero progress this week
}

// =============================================================================
// Time range helpers
// =============================================================================

/// Returns (monday, sunday) for the prior work week.
pub fn prior_work_week() -> (NaiveDate, NaiveDate) {
    let today = Utc::now().date_naive();
    let days_since_monday = today.weekday().num_days_from_monday() as i64;
    // Start of this week (Monday)
    let this_monday = today - Duration::days(days_since_monday);
    // Prior week
    let prior_monday = this_monday - Duration::days(7);
    let prior_sunday = prior_monday + Duration::days(6);
    (prior_monday, prior_sunday)
}

// =============================================================================
// Prompt
// =============================================================================

fn build_weekly_impact_prompt(
    db: &ActionDb,
    week_start: NaiveDate,
    week_end: NaiveDate,
) -> String {
    let week_start_str = week_start.format("%Y-%m-%d").to_string();
    let week_end_str = format!("{} 23:59:59", week_end.format("%Y-%m-%d"));
    let week_label = format!(
        "{} – {}",
        week_start.format("%b %-d"),
        week_end.format("%-d, %Y")
    );

    // Gather user priorities
    let priorities_json: String = db
        .conn_ref()
        .query_row(
            "SELECT COALESCE(annual_priorities, '[]') || ' ' || COALESCE(quarterly_priorities, '[]') FROM user_entity LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    // Gather meetings for the week
    let meetings: String = db
        .conn_ref()
        .prepare(
            "SELECT title, start_time, meeting_type FROM meetings_history
             WHERE start_time >= ?1 AND start_time <= ?2
               AND meeting_type NOT IN ('personal', 'focus', 'blocked')
             ORDER BY start_time",
        )
        .and_then(|mut s| {
            let rows = s.query_map(
                rusqlite::params![week_start_str, week_end_str],
                |row| {
                    let title: String = row.get(0)?;
                    let time: String = row.get(1)?;
                    let mtype: String = row.get(2)?;
                    Ok(format!("- {} | {} | {}", time, mtype, title))
                },
            )?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    // Gather signal events for the week
    let signals: String = db
        .conn_ref()
        .prepare(
            "SELECT se.signal_type, se.value, se.source, se.confidence, se.created_at,
                    COALESCE(a.name, p.name, '') as entity_name
             FROM signal_events se
             LEFT JOIN accounts a ON se.entity_id = a.id AND se.entity_type = 'account'
             LEFT JOIN people p ON se.entity_id = p.id AND se.entity_type = 'person'
             WHERE se.created_at >= ?1 AND se.created_at <= ?2
             ORDER BY se.confidence DESC
             LIMIT 20",
        )
        .and_then(|mut s| {
            let rows = s.query_map(
                rusqlite::params![week_start_str, week_end_str],
                |row| {
                    let stype: String = row.get(0)?;
                    let val: String = row.get::<_, Option<String>>(1)?.unwrap_or_default();
                    let src: String = row.get(2)?;
                    let entity: String = row.get(5)?;
                    Ok(format!("- [{}] {} — {} (source: {})", stype, entity, val, src))
                },
            )?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    // Gather completed actions for the week
    let completed_actions: String = db
        .conn_ref()
        .prepare(
            "SELECT title, completed_at FROM actions
             WHERE completed_at >= ?1 AND completed_at <= ?2
             ORDER BY completed_at",
        )
        .and_then(|mut s| {
            let rows = s.query_map(
                rusqlite::params![week_start_str, week_end_str],
                |row| {
                    let title: String = row.get(0)?;
                    let completed: String = row.get(1)?;
                    Ok(format!("- {} (completed {})", title, completed))
                },
            )?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    let meeting_count = if meetings.is_empty() { 0 } else { meetings.lines().count() };
    let action_count = if completed_actions.is_empty() { 0 } else { completed_actions.lines().count() };

    let mut prompt = build_report_preamble("you", "weekly_impact", "user");

    prompt.push_str(&format!("# Week: {}\n\n", week_label));

    if !priorities_json.trim().is_empty() && priorities_json.trim() != " " {
        prompt.push_str("## Your Priorities\n");
        prompt.push_str(&crate::util::wrap_user_data(&priorities_json));
        prompt.push_str("\n\n");
    } else {
        prompt.push_str("## Your Priorities\n(No priorities set — skip priorities_moved, use carry_forward: [])\n\n");
    }

    if !meetings.is_empty() {
        prompt.push_str(&format!("## Meetings This Week ({} total)\n", meeting_count));
        prompt.push_str(&crate::util::wrap_user_data(&meetings));
        prompt.push_str("\n\n");
    } else {
        prompt.push_str("## Meetings This Week\n(none)\n\n");
    }

    if !signals.is_empty() {
        prompt.push_str("## Intelligence Signals\n");
        prompt.push_str(&crate::util::wrap_user_data(&signals));
        prompt.push_str("\n\n");
    }

    if !completed_actions.is_empty() {
        prompt.push_str(&format!("## Completed Actions ({} total)\n", action_count));
        prompt.push_str(&crate::util::wrap_user_data(&completed_actions));
        prompt.push_str("\n\n");
    } else {
        prompt.push_str("## Completed Actions\n(none)\n\n");
    }

    prompt.push_str("# Output Format\n\n");
    prompt.push_str("Respond with ONLY a valid JSON object (no markdown fences) matching this schema:\n\n");
    prompt.push_str(&format!(
        r#"{{
  "weekLabel": "{week_label}",
  "headlineStat": "X meetings, Y actions closed — one phrase summary, max 10 words",
  "prioritiesMoved": [
    {{
      "priorityText": "The priority this relates to",
      "whatHappened": "What actually happened, max 20 words",
      "source": "meeting-id or signal-id — REQUIRED, never null"
    }}
  ],
  "wins": ["Win 1, max 15 words", "Win 2"],
  "activitySummary": "2 sentences on what you did volume-wise this week.",
  "watch": ["Concern or pattern to monitor, max 15 words"],
  "carryForward": ["Priority with zero activity this week — be honest. Max 15 words."]
}}"#,
        week_label = week_label
    ));

    prompt.push_str("\n\n# Rules\n");
    prompt.push_str("- priorities_moved: ONLY include if there is a real meeting or signal that demonstrates progress. Set source to the meeting ID or signal ID. NEVER fabricate.\n");
    prompt.push_str("- If no priorities are set: priorities_moved = [], carry_forward = [].\n");
    prompt.push_str("- carry_forward: Show ALL priorities with zero activity, even if uncomfortable. Do NOT omit them.\n");
    prompt.push_str("- wins: 1–3 concrete wins from the data. Not generic.\n");
    prompt.push_str("- watch: 1–3 items. Patterns, risks, or things needing follow-up.\n");

    prompt
}

// =============================================================================
// Generation input (Phase 1)
// =============================================================================

pub fn gather_weekly_impact_input(
    _workspace: &std::path::Path,
    db: &ActionDb,
    ai_models: AiModelConfig,
) -> Result<ReportGeneratorInput, String> {
    let (week_start, week_end) = prior_work_week();
    let intel_hash = format!("week-{}", week_start.format("%Y-%m-%d"));
    let prompt = build_weekly_impact_prompt(db, week_start, week_end);

    // "user" entity — use the actual numeric ID as string
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
        report_type: "weekly_impact".to_string(),
        entity_name: "Weekly Impact".to_string(),
        workspace: _workspace.to_path_buf(),
        prompt,
        ai_models,
        intel_hash,
    })
}

// =============================================================================
// Response parsing
// =============================================================================

pub fn parse_weekly_impact_response(stdout: &str) -> Result<WeeklyImpactContent, String> {
    let json_str = crate::risk_briefing::extract_json_object(stdout)
        .ok_or_else(|| "No valid JSON object found in Weekly Impact response".to_string())?;

    serde_json::from_str::<WeeklyImpactContent>(&json_str)
        .map_err(|e| format!("Failed to parse Weekly Impact JSON: {}", e))
}

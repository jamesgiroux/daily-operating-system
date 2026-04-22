//! Weekly Impact Report (I418).
//!
//! Covers the prior Mon–Sun work week. Gathers user priorities,
//! signal events, meetings history, and completed actions.
//! Personalized to the user's active role preset.
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
pub struct WeeklyImpactMove {
    pub priority_text: String,
    pub what_happened: String,
    /// meeting ID or date — required
    pub source: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyImpactItem {
    pub text: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyImpactContent {
    /// e.g. "Feb 17 – 21, 2026"
    pub week_label: String,
    pub total_meetings: i32,
    pub total_actions_closed: i32,
    /// Punchy editorial one-liner: "7 meetings. 3 closes. 2 first impressions."
    pub headline: String,
    /// What actually moved — each item needs a source citation
    pub priorities_moved: Vec<WeeklyImpactMove>,
    /// Specific wins, 1-3
    pub wins: Vec<WeeklyImpactItem>,
    /// 2-sentence editorial summary of the week's activity
    pub what_you_did: String,
    /// Things to monitor, 1-3
    pub watch: Vec<WeeklyImpactItem>,
    /// 1-3 forward items into next week
    pub into_next_week: Vec<String>,
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
// Preset vocabulary helpers
// =============================================================================

fn entity_noun_for_preset(preset: &str) -> &'static str {
    match preset {
        "sales" => "deal",
        "agency" | "consulting" => "client",
        "partnerships" => "partner",
        "product" => "initiative",
        "the-desk" => "project",
        _ => "account",
    }
}

// =============================================================================
// Prompt
// =============================================================================

fn build_weekly_impact_prompt(
    db: &ActionDb,
    week_start: NaiveDate,
    week_end: NaiveDate,
    active_preset: &str,
) -> String {
    let week_start_str = week_start.format("%Y-%m-%d").to_string();
    let week_end_str = format!("{} 23:59:59", week_end.format("%Y-%m-%d"));
    let week_label = format!(
        "{} – {}",
        week_start.format("%b %-d"),
        week_end.format("%-d, %Y")
    );
    let entity_noun = entity_noun_for_preset(active_preset);

    // Gather user priorities
    let priorities_json: String = db
        .conn_ref()
        .query_row(
            "SELECT COALESCE(annual_priorities, '[]') || ' ' || COALESCE(quarterly_priorities, '[]') FROM user_entity LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    // Gather meetings for the week. Citation format: date + title (not raw calendar IDs).
    let meetings: String = db
        .conn_ref()
        .prepare(
            "SELECT title, start_time, meeting_type FROM meetings
             WHERE start_time >= ?1 AND start_time <= ?2
               AND meeting_type NOT IN ('personal', 'focus', 'blocked')
             ORDER BY start_time",
        )
        .and_then(|mut s| {
            let rows = s.query_map(rusqlite::params![week_start_str, week_end_str], |row| {
                let title: String = row.get(0)?;
                let time: String = row.get(1)?;
                let mtype: String = row.get(2)?;
                // Use only the date portion as the citation reference — readable and unambiguous
                let date = time.split('T').next().unwrap_or(&time).to_string();
                Ok(format!("- {} | {} | {}", date, mtype, title))
            })?;
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
            let rows = s.query_map(rusqlite::params![week_start_str, week_end_str], |row| {
                let stype: String = row.get(0)?;
                let val: String = row.get::<_, Option<String>>(1)?.unwrap_or_default();
                let src: String = row.get(2)?;
                let entity: String = row.get(5)?;
                Ok(format!(
                    "- [{}] {} — {} (source: {})",
                    stype, entity, val, src
                ))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    // Gather meeting summaries for the week (from transcripts)
    let meeting_summaries: String = db
        .conn_ref()
        .prepare(
            "SELECT m.title, m.start_time, mt.summary
             FROM meetings m
             JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             WHERE m.start_time >= ?1 AND m.start_time <= ?2
               AND mt.summary IS NOT NULL AND mt.summary != ''
             ORDER BY m.start_time
             LIMIT 30",
        )
        .and_then(|mut s| {
            let rows = s.query_map(rusqlite::params![week_start_str, week_end_str], |row| {
                let title: String = row.get(0)?;
                let time: String = row.get(1)?;
                let summary: String = row.get(2)?;
                let date = time.split('T').next().unwrap_or(&time).to_string();
                Ok(format!("- {} | {} | Summary: {}", date, title, summary))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    // Gather captures for the week (wins, risks, decisions with metadata)
    let week_captures: String = db
        .conn_ref()
        .prepare(
            "SELECT capture_type, content, sub_type, urgency, impact,
                    evidence_quote, meeting_title, captured_at
             FROM captures
             WHERE captured_at >= ?1 AND captured_at <= ?2
             ORDER BY CASE urgency WHEN 'red' THEN 0 WHEN 'yellow' THEN 1 WHEN 'green_watch' THEN 2 ELSE 3 END,
                      captured_at
             LIMIT 30",
        )
        .and_then(|mut s| {
            let rows = s.query_map(rusqlite::params![week_start_str, week_end_str], |row| {
                let ctype: String = row.get(0)?;
                let content: String = row.get(1)?;
                let sub_type: Option<String> = row.get(2)?;
                let urgency: Option<String> = row.get(3)?;
                let _impact: Option<String> = row.get(4)?;
                let quote: Option<String> = row.get(5)?;
                let mtitle: Option<String> = row.get(6)?;
                let captured: String = row.get(7)?;
                let date = captured.split('T').next().unwrap_or(&captured).to_string();
                let sub = sub_type.map(|s| format!("[{}] ", s)).unwrap_or_default();
                let urg = urgency.map(|u| format!("[{}] ", u)).unwrap_or_default();
                let src = mtitle.map(|t| format!(" — from {}", t)).unwrap_or_default();
                let q = quote.map(|q| format!(" #\"{}\"", q)).unwrap_or_default();
                Ok(format!("- {}: {}{}{} ({}){}{}",
                    ctype.to_uppercase(), urg, sub, content, date, src, q
                ))
            })?;
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
            let rows = s.query_map(rusqlite::params![week_start_str, week_end_str], |row| {
                let title: String = row.get(0)?;
                let completed: String = row.get(1)?;
                Ok(format!("- {} (completed {})", title, completed))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    let meeting_count = if meetings.is_empty() {
        0
    } else {
        meetings.lines().count()
    };
    let action_count = if completed_actions.is_empty() {
        0
    } else {
        completed_actions.lines().count()
    };

    let mut prompt = build_report_preamble("you", "weekly_impact", "user");

    // Preset context line
    prompt.push_str(&format!(
        "Role preset: {} ({} vocabulary). Adapt all output to use '{}' not 'account' where applicable.\n\n",
        active_preset, entity_noun, entity_noun
    ));

    prompt.push_str(&format!("# Week: {}\n\n", week_label));

    if !priorities_json.trim().is_empty() && priorities_json.trim() != " " {
        prompt.push_str("## Your Priorities\n");
        prompt.push_str(&crate::util::wrap_user_data(&priorities_json));
        prompt.push_str("\n\n");
    } else {
        prompt.push_str("## Your Priorities\n(No priorities set — priorities_moved: [], into_next_week: [])\n\n");
    }

    if !meetings.is_empty() {
        prompt.push_str(&format!(
            "## Meetings This Week ({} total — dates included for citations)\n",
            meeting_count
        ));
        prompt.push_str(&crate::util::wrap_user_data(&meetings));
        prompt.push_str("\n\n");
    } else {
        prompt.push_str("## Meetings This Week\n(none)\n\n");
    }

    if !meeting_summaries.is_empty() {
        prompt.push_str("## Meeting Content This Week (summaries from transcripts)\n");
        prompt.push_str(&crate::util::wrap_user_data(&meeting_summaries));
        prompt.push_str("\n\n");
    }

    if !week_captures.is_empty() {
        // Group captures by type for readability
        prompt.push_str("## Outcomes Captured This Week (wins, risks, decisions)\n");
        prompt.push_str(&crate::util::wrap_user_data(&week_captures));
        prompt.push_str("\n\n");
    }

    if !signals.is_empty() {
        prompt.push_str("## Updates Captured\n");
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

    prompt.push_str("## Output Format\n\n");
    prompt.push_str(
        "Respond with ONLY valid JSON (no markdown fences) matching this schema exactly:\n\n",
    );
    prompt.push_str(&format!(
        r#"{{
  "weekLabel": "{week_label}",
  "totalMeetings": {meeting_count},
  "totalActionsClosed": {action_count},
  "headline": "Punchy editorial one-liner. Like a newspaper headline. E.g. '7 meetings. 3 closes. 2 first impressions.' Max 15 words.",
  "prioritiesMoved": [
    {{
      "priorityText": "The priority this relates to",
      "whatHappened": "What actually happened, max 20 words",
      "source": "date from the meetings list above, e.g. '2026-02-17' — REQUIRED, never null or fabricated"
    }}
  ],
  "wins": [
    {{
      "text": "Specific win, max 15 words",
      "source": "date from meetings list e.g. '2026-02-17', or null if not from a specific meeting"
    }}
  ],
  "whatYouDid": "2 editorial sentences summarizing the week's activity. Volume + character of the work.",
  "watch": [
    {{
      "text": "Concern or pattern to monitor, max 15 words",
      "source": null
    }}
  ],
  "intoNextWeek": ["Forward item 1 — specific, max 15 words", "Forward item 2"]
}}"#,
        week_label = week_label,
        meeting_count = meeting_count,
        action_count = action_count,
    ));

    prompt.push_str("\n\n## Rules\n");
    prompt.push_str(&format!(
        "- totalMeetings and totalActionsClosed: use the EXACT counts from the data. Do not change them.\n\
         - headline: punchy editorial one-liner — make it feel like a headline, not a summary sentence.\n\
         - prioritiesMoved: ONLY include if there is a real meeting or signal demonstrating progress. source must be a date from the meetings list above. Cite specific meeting outcomes from the Meeting Content and Outcomes Captured sections — not just meeting titles. NEVER fabricate.\n\
         - If no priorities are set: prioritiesMoved = []. intoNextWeek still MUST have 2-3 items.\n\
         - wins: 1-3 concrete wins from the data. Reference actual captured wins with evidence from the Outcomes Captured section where available. Each needs to be specific — not 'had a good meeting'.\n\
         - whatYouDid: editorial, not corporate. Describes the week's character, not just volume.\n\
         - watch: 1-3 items — patterns, risks, follow-up gaps. Reference actual captured risks with urgency context from the Outcomes Captured section where available. Specific.\n\
         - intoNextWeek: ALWAYS 2-3 items — never empty. Derive from watch items, open threads from wins, or next logical steps from the week's work. If no priorities, infer from the meetings and what came up. E.g. 'Follow up with [name] on [thing]', 'Schedule [meeting]', 'Close [action]'. Use '{}' vocabulary.\n\
         - Do NOT mention AI, enrichment, or internal app mechanics in any output text. Use human language.\n",
        entity_noun
    ));

    prompt
}

// =============================================================================
// Generation input (Phase 1)
// =============================================================================

pub fn gather_weekly_impact_input(
    workspace: &std::path::Path,
    db: &ActionDb,
    ai_models: AiModelConfig,
    active_preset: &str,
) -> Result<ReportGeneratorInput, String> {
    let (week_start, week_end) = prior_work_week();
    let intel_hash = format!("week-{}", week_start.format("%Y-%m-%d"));
    let prompt = build_weekly_impact_prompt(db, week_start, week_end, active_preset);

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
        workspace: workspace.to_path_buf(),
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

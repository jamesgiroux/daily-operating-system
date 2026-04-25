//! Monthly Wrapped report (I419).
//!
//! Prior calendar month. Spotify Wrapped energy — celebration, not a report.
//! Personalized to the user's active role preset with a personality type assignment.

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
pub struct WrappedPersonality {
    /// e.g. "The Relationship Architect"
    pub type_name: String,
    /// 1-2 sentences on what defines this type
    pub description: String,
    /// "Your defining move this month: [specific observation]"
    pub key_signal: String,
    /// "Only 18% this month" — AI invents a plausible-feeling percentage
    pub rarity_label: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrappedMoment {
    /// "A first" / "Peak week" / "Your best day" / "A surprise"
    pub label: String,
    /// "First meeting with Natasha Brown on Jan 20"
    pub headline: String,
    /// optional 1-line context
    pub subtext: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyWrappedContent {
    /// e.g. "February 2026"
    pub month_label: String,
    /// total meetings in the period
    pub total_conversations: i32,
    /// unique accounts/deals/clients touched
    pub total_entities_touched: i32,
    /// unique people met
    pub total_people_met: i32,
    /// signal events captured
    pub signals_captured: i32,
    /// most active account/deal/client name
    pub top_entity_name: String,
    /// how many times you engaged them
    pub top_entity_touches: i32,
    /// 2-3 specific memorable moments
    pub moments: Vec<WrappedMoment>,
    /// "You didn't know this: [something specific]"
    pub hidden_pattern: String,
    /// role-specific personality type assignment
    pub personality: WrappedPersonality,
    /// % of touches on priority entities — null if no priorities set
    pub priority_alignment_pct: Option<i32>,
    /// "On track" / "Worth a look" / "Your call" — null if no priorities
    pub priority_alignment_label: Option<String>,
    /// single biggest win of the month, celebrated
    pub top_win: String,
    /// forward-framed, never judgmental
    pub carry_forward: String,
    /// "Your [month] in three words:"
    pub word_one: String,
    pub word_two: String,
    pub word_three: String,
}

// =============================================================================
// Time range helpers
// =============================================================================

/// Returns (first_day, last_day) of the prior calendar month.
pub fn prior_calendar_month() -> (NaiveDate, NaiveDate) {
    let today = Utc::now().date_naive();
    let first_of_this_month =
        NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    let last_of_prior = first_of_this_month.pred_opt().unwrap_or(today);
    let first_of_prior = NaiveDate::from_ymd_opt(last_of_prior.year(), last_of_prior.month(), 1)
        .unwrap_or(last_of_prior);
    (first_of_prior, last_of_prior)
}

// =============================================================================
// Personality type matrix by preset
// =============================================================================

fn personality_types_for_preset(preset: &str) -> &'static str {
    match preset {
        "sales" => {
            "- The Pipeline Builder: high outreach volume, new deal creation, discovery-focused\n\
             - The Momentum Closer: deal velocity, stage progression, converting\n\
             - The Trusted Advisor: fewer deals, deeper engagement, relationship-led\n\
             - The Territory Owner: broad coverage, consistent touchpoints across territory"
        }
        "leadership" => {
            "- The Strategic Connector: bridges across teams and portfolios, high cross-entity activity\n\
             - The Force Multiplier: enables others, high team-facing engagement, amplifies output\n\
             - The Portfolio Conductor: oversees diverse entities, broad awareness, orchestration\n\
             - The Signal Reader: intelligence-first, pattern synthesis, high context intake"
        }
        "agency" => {
            "- The Account Juggler: many active clients, high context-switching, parallel threads\n\
             - The Delivery Captain: completion-focused, high action rate, deliverable-oriented\n\
             - The Relationship Anchor: consistently deepens core client relationships, reliability\n\
             - The Growth Scout: new business focus, expansion signals, new client meetings"
        }
        "consulting" => {
            "- The Trusted Advisor: deep engagement on fewer engagements, advisor-level trust\n\
             - The Insight Generator: high signal and capture rate, analysis-first\n\
             - The Deliverable Driver: action completion, milestone-focused, outcomes-oriented\n\
             - The Scope Expander: identifies expansion opportunities, growth signals across clients"
        }
        "affiliates-partnerships" | "affiliates" | "partnerships" => {
            "- The Alliance Builder: new partnerships initiated, relationship-first, introductions\n\
             - The Creator Operator: high follow-through on creator campaigns, performance-focused\n\
             - The Co-Sell Champion: cross-functional deal collaboration, joint-selling focused\n\
             - The Integration Connector: technical depth, product-partner alignment"
        }
        "product-marketing" | "product" | "marketing" => {
            "- The Voice Amplifier: high customer meeting count, carries customer voice internally\n\
             - The Signal Synthesizer: translates customer signals into clear product insights\n\
             - The Campaign Orchestrator: many parallel programs, high coordination, launch-focused\n\
             - The Velocity Driver: high action completion, ships things, bias toward done"
        }
        "core" | "the-desk" => {
            "- The Connector: broad relationship diversity, bridges people and contexts\n\
             - The Finisher: high follow-through rate, commits and delivers\n\
             - The Knowledge Builder: heavy context and signal capture, builds the knowledge base\n\
             - The Consistent Presence: reliable, steady engagement rhythm, always there"
        }
        // customer-success and default
        _ => {
            "- The Relationship Architect: systematic, high breadth, multi-threaded account coverage\n\
             - The Champion Builder: focus on new stakeholders, first meetings, relationship-building\n\
             - The Renewal Strategist: high activity on renewal-adjacent accounts, value documentation\n\
             - The Steady Hand: consistent reliable cadence, no big swings, accounts trust you to show up"
        }
    }
}

fn entity_noun_for_preset(preset: &str) -> &'static str {
    match preset {
        "affiliates-partnerships" | "affiliates" | "partnerships" => "partner",
        "product-marketing" | "product" | "marketing" => "initiative",
        "core" | "the-desk" => "project",
        _ => "account",
    }
}

// =============================================================================
// Prompt
// =============================================================================

fn build_monthly_wrapped_prompt(
    db: &ActionDb,
    month_start: NaiveDate,
    month_end: NaiveDate,
    active_preset: &str,
) -> String {
    let month_start_str = month_start.format("%Y-%m-%d").to_string();
    let month_end_str = format!("{} 23:59:59", month_end.format("%Y-%m-%d"));
    let month_label = month_start.format("%B %Y").to_string();
    let entity_noun = entity_noun_for_preset(active_preset);
    let personality_types = personality_types_for_preset(active_preset);

    // Gather user priorities
    let priorities_json: String = db
        .conn_ref()
        .query_row(
            "SELECT COALESCE(annual_priorities, '[]') || ' | ' || COALESCE(quarterly_priorities, '[]') FROM user_entity LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    // Meeting count
    let meeting_count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM meetings WHERE start_time >= ?1 AND start_time <= ?2 AND meeting_type NOT IN ('personal', 'focus', 'blocked')",
            rusqlite::params![month_start_str, month_end_str],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Full meeting list with IDs for citation
    let meetings: String = db
        .conn_ref()
        .prepare(
            "SELECT id, title, start_time, meeting_type FROM meetings
             WHERE start_time >= ?1 AND start_time <= ?2
               AND meeting_type NOT IN ('personal', 'focus', 'blocked')
             ORDER BY start_time",
        )
        .and_then(|mut s| {
            let rows = s.query_map(rusqlite::params![month_start_str, month_end_str], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let time: String = row.get(2)?;
                Ok(format!("- [{}] {} | {}", id, time, title))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    // Completed actions
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
            let rows = s.query_map(rusqlite::params![month_start_str, month_end_str], |row| {
                let title: String = row.get(0)?;
                let completed: String = row.get(1)?;
                Ok(format!("- {} (completed {})", title, completed))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    // Signal events
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
            "SELECT se.signal_type, se.value, COALESCE(a.name, p.name, '') as entity_name
             FROM signal_events se
             LEFT JOIN accounts a ON se.entity_id = a.id AND se.entity_type = 'account'
             LEFT JOIN people p ON se.entity_id = p.id AND se.entity_type = 'person'
             WHERE se.created_at >= ?1 AND se.created_at <= ?2
             ORDER BY se.confidence DESC
             LIMIT 30",
        )
        .and_then(|mut s| {
            let rows = s.query_map(rusqlite::params![month_start_str, month_end_str], |row| {
                let stype: String = row.get(0)?;
                let val: String = row.get::<_, Option<String>>(1)?.unwrap_or_default();
                let entity: String = row.get(2)?;
                Ok(format!("- [{}] {} — {}", stype, entity, val))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    // Unique entities touched (accounts linked to meetings)
    let entities_touched: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(DISTINCT me.entity_id) FROM meeting_entities me
             JOIN meetings m ON m.id = me.meeting_id
             WHERE m.start_time >= ?1 AND m.start_time <= ?2 AND me.entity_type = 'account'",
            rusqlite::params![month_start_str, month_end_str],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Unique people met
    let people_met: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(DISTINCT ma.person_id) FROM meeting_attendees ma
             JOIN meetings m ON m.id = ma.meeting_id
             WHERE m.start_time >= ?1 AND m.start_time <= ?2",
            rusqlite::params![month_start_str, month_end_str],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Top entity (most meetings)
    let (top_entity_name, top_entity_touches) = db
        .conn_ref()
        .query_row(
            "SELECT a.name, COUNT(*) as cnt
             FROM meeting_entities me
             JOIN meetings m ON m.id = me.meeting_id
             JOIN accounts a ON a.id = me.entity_id
             WHERE m.start_time >= ?1 AND m.start_time <= ?2 AND me.entity_type = 'account'
             GROUP BY me.entity_id ORDER BY cnt DESC LIMIT 1",
            rusqlite::params![month_start_str, month_end_str],
            |row| {
                let name: String = row.get(0)?;
                let cnt: i64 = row.get(1)?;
                Ok((name, cnt))
            },
        )
        .unwrap_or_else(|_| ("(none)".to_string(), 0));

    // Build prompt
    let mut prompt = String::with_capacity(8192);

    // Custom preamble — this is NOT a report, it's a celebration
    prompt.push_str(&format!(
        "You are writing a Monthly Wrapped experience for a {entity_noun}-facing professional using DailyOS.\n\
         This is NOT a report. It's a celebration — think Spotify Wrapped energy. Whimsical, specific, personal.\n\
         The user's role preset is: {active_preset} ({entity_noun} vocabulary)\n\
         Month: {month_label}\n\n"
    ));

    prompt.push_str("## Personality Types for This Role\n");
    prompt.push_str("Assign ONE of these based on the month's data patterns:\n");
    prompt.push_str(personality_types);
    prompt.push_str("\n\n");

    prompt.push_str("## Month Data\n\n");
    prompt.push_str(&format!(
        "Stats: {} meetings, {} actions completed, {} updates captured\n\
         Unique {}s touched: {}\n\
         Unique people met: {}\n\
         Top {}: {} ({} engagements)\n\n",
        meeting_count,
        action_count,
        signal_count,
        entity_noun,
        entities_touched,
        people_met,
        entity_noun,
        top_entity_name,
        top_entity_touches
    ));

    if !priorities_json.trim().is_empty() {
        prompt.push_str("### Priorities\n");
        prompt.push_str(&crate::util::wrap_user_data(&priorities_json));
        prompt.push_str("\n\n");
    }

    if !meetings.is_empty() {
        prompt.push_str("### Meetings (with IDs for citation)\n");
        prompt.push_str(&crate::util::wrap_user_data(&meetings));
        prompt.push_str("\n\n");
    }

    if !completed_actions.is_empty() {
        prompt.push_str("### Completed Actions\n");
        prompt.push_str(&crate::util::wrap_user_data(&completed_actions));
        prompt.push_str("\n\n");
    }

    if !signals.is_empty() {
        prompt.push_str("### Updates Captured\n");
        prompt.push_str(&crate::util::wrap_user_data(&signals));
        prompt.push_str("\n\n");
    }

    // Meeting summaries for the month (from transcripts)
    let month_summaries: String = db
        .conn_ref()
        .prepare(
            "SELECT m.title, m.start_time, mt.summary
             FROM meetings m
             JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             WHERE m.start_time >= ?1 AND m.start_time <= ?2
               AND mt.summary IS NOT NULL AND mt.summary != ''
             ORDER BY m.start_time
             LIMIT 50",
        )
        .and_then(|mut s| {
            let rows = s.query_map(rusqlite::params![month_start_str, month_end_str], |row| {
                let title: String = row.get(0)?;
                let time: String = row.get(1)?;
                let summary: String = row.get(2)?;
                let date = time.split('T').next().unwrap_or(&time).to_string();
                Ok(format!("- {} | {} | {}", date, title, summary))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    if !month_summaries.is_empty() {
        prompt.push_str("### Meeting Summaries (from transcripts)\n");
        prompt.push_str(&crate::util::wrap_user_data(&month_summaries));
        prompt.push_str("\n\n");
    }

    // Captures for the month (wins, risks, decisions)
    let month_captures: String = db
        .conn_ref()
        .prepare(
            "SELECT capture_type, content, sub_type, urgency, impact,
                    evidence_quote, meeting_title, captured_at
             FROM captures
             WHERE captured_at >= ?1 AND captured_at <= ?2
             ORDER BY CASE urgency WHEN 'red' THEN 0 WHEN 'yellow' THEN 1 WHEN 'green_watch' THEN 2 ELSE 3 END,
                      captured_at
             LIMIT 50",
        )
        .and_then(|mut s| {
            let rows = s.query_map(rusqlite::params![month_start_str, month_end_str], |row| {
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
                Ok(format!("- {}: {}{}{}{} ({}){}",
                    ctype.to_uppercase(), urg, sub, content, src, date, q
                ))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    if !month_captures.is_empty() {
        prompt.push_str("### Outcomes Captured (wins, risks, decisions)\n");
        prompt.push_str(&crate::util::wrap_user_data(&month_captures));
        prompt.push_str("\n\n");
    }

    // Champion health assessments from the month
    let key_advocate_health: String = db
        .conn_ref()
        .prepare(
            "SELECT m.title, mch.champion_name, mch.champion_status, mch.champion_evidence
             FROM meeting_champion_health mch
             JOIN meetings m ON m.id = mch.meeting_id
             WHERE m.start_time >= ?1 AND m.start_time <= ?2
             ORDER BY m.start_time
             LIMIT 30",
        )
        .and_then(|mut s| {
            let rows = s.query_map(rusqlite::params![month_start_str, month_end_str], |row| {
                let title: String = row.get(0)?;
                let name: String = row.get(1)?;
                let status: String = row.get(2)?;
                let evidence: Option<String> = row.get(3)?;
                let ev = evidence.map(|e| format!(" — {}", e)).unwrap_or_default();
                Ok(format!("- {} | {} | status: {}{}", title, name, status, ev))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    if !key_advocate_health.is_empty() {
        prompt.push_str("### Champion Health Trends\n");
        prompt.push_str(&crate::util::wrap_user_data(&key_advocate_health));
        prompt.push_str("\n\n");
    }

    // Interaction dynamics trends (engagement quality)
    let dynamics: String = db
        .conn_ref()
        .prepare(
            "SELECT m.title, mid.talk_balance_customer_pct,
                    mid.question_density, mid.decision_maker_active
             FROM meeting_interaction_dynamics mid
             JOIN meetings m ON m.id = mid.meeting_id
             WHERE m.start_time >= ?1 AND m.start_time <= ?2
             ORDER BY m.start_time
             LIMIT 30",
        )
        .and_then(|mut s| {
            let rows = s.query_map(rusqlite::params![month_start_str, month_end_str], |row| {
                let title: String = row.get(0)?;
                let talk_pct: Option<f64> = row.get(1)?;
                let q_density: Option<f64> = row.get(2)?;
                let dm_active: Option<bool> = row.get(3)?;
                let talk = talk_pct
                    .map(|p| format!(" | customer talk: {:.0}%", p))
                    .unwrap_or_default();
                let qd = q_density
                    .map(|d| format!(" | question density: {:.1}", d))
                    .unwrap_or_default();
                let dm = dm_active
                    .map(|a| format!(" | decision-maker active: {}", a))
                    .unwrap_or_default();
                Ok(format!("- {}{}{}{}", title, talk, qd, dm))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    if !dynamics.is_empty() {
        prompt.push_str("### Engagement Dynamics\n");
        prompt.push_str(&crate::util::wrap_user_data(&dynamics));
        prompt.push_str("\n\n");
    }

    prompt.push_str("## Output Format\n\n");
    prompt.push_str(
        "Respond with ONLY valid JSON (no markdown fences) matching this schema exactly:\n\n",
    );
    prompt.push_str(&format!(
        r#"{{
  "monthLabel": "{month_label}",
  "totalConversations": {meeting_count},
  "totalEntitiesTouched": {entities_touched},
  "totalPeopleMet": {people_met},
  "signalsCaptured": {signal_count},
  "topEntityName": "{top_entity_name}",
  "topEntityTouches": {top_entity_touches},
  "moments": [
    {{
      "label": "A first | Peak week | Your best day | A surprise",
      "headline": "Specific moment — use real names and dates from the data",
      "subtext": "optional 1-line context or null"
    }}
  ],
  "hiddenPattern": "Something the user might not have noticed. Be specific. E.g. 'Your {entity_noun} meetings are 3x more likely on Tuesdays' or 'You followed up on 94% of updates this month.'",
  "personality": {{
    "typeName": "The [Type Name] from the list above",
    "description": "1-2 sentences specific to what defined THIS user's month — not generic",
    "keySignal": "Your defining move this month: [specific observation from the data]",
    "rarityLabel": "Only X% this month — pick a percentage that makes the type feel meaningfully earned"
  }},
  "priorityAlignmentPct": null,
  "priorityAlignmentLabel": null,
  "topWin": "Single biggest win, celebrated. Be specific and direct.",
  "carryForward": "One thing, forward-framed. 'Bring [Entity X] back into rotation' not 'You neglected [Entity X].'",
  "wordOne": "evocative word",
  "wordTwo": "evocative word",
  "wordThree": "evocative word"
}}"#,
        month_label = month_label,
        meeting_count = meeting_count,
        entities_touched = entities_touched,
        people_met = people_met,
        signal_count = signal_count,
        top_entity_name = top_entity_name.replace('"', "\\\""),
        top_entity_touches = top_entity_touches,
        entity_noun = entity_noun,
    ));

    prompt.push_str("\n\n## Rules\n");
    prompt.push_str(&format!(
        "- totalConversations, totalEntitiesTouched, totalPeopleMet, signalsCaptured: use the EXACT numbers from the data above. Do not change them.\n\
         - topEntityName / topEntityTouches: use the values from the data above.\n\
         - moments: 2-3 SPECIFIC moments. Not 'a productive month' — 'First meeting with [actual name] on [actual date].' Use real names and dates from the meeting list. Cite actual meeting outcomes from Meeting Summaries and Outcomes Captured when available.\n\
         - hiddenPattern: Something genuinely interesting the user might not have noticed. Use engagement dynamics and champion health trends when available. Be specific.\n\
         - personality: Assign the type that BEST fits the data. description and keySignal must reference this user's actual month, not be generic. rarityLabel should feel real ('Only 14% this month') — pick a % that makes the type feel earned.\n\
         - priorityAlignmentPct: if priorities are set, estimate what % of {entity_noun} touches were on priority entities. null if no priorities.\n\
         - priorityAlignmentLabel: 'On track' if >60%, 'Worth a look' if 40-60%, 'Your call' if <40%, null if no priorities.\n\
         - topWin: One achievement. Reference the strongest captured win from the Outcomes Captured section if available. Celebrate it with specificity.\n\
         - carryForward: One thing, forward-framed. Never guilt-inducing.\n\
         - wordOne/wordTwo/wordThree: Specific and evocative. Not 'busy' or 'productive'. Try: momentum, foundation, expansion, steady, breakthrough, reconnection.\n\
         - Do NOT mention AI, enrichment, signals, or internal app mechanics in any output text. Use human language.\n"
    ));

    prompt
}

// =============================================================================
// Generation input (Phase 1)
// =============================================================================

pub fn gather_monthly_wrapped_input(
    workspace: &std::path::Path,
    db: &ActionDb,
    ai_models: AiModelConfig,
    active_preset: &str,
) -> Result<ReportGeneratorInput, String> {
    let (month_start, month_end) = prior_calendar_month();
    let intel_hash = format!("month-{}", month_start.format("%Y-%m"));
    let prompt = build_monthly_wrapped_prompt(db, month_start, month_end, active_preset);

    let user_entity_id: String = db
        .conn_ref()
        .query_row(
            "SELECT CAST(id AS TEXT) FROM user_entity LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "1".to_string());

    // Override the generic preamble — monthly wrapped uses its own framing built above.
    // build_report_preamble is called here for consistency but the prompt already
    // has the correct framing as its first section.
    let _ = build_report_preamble("you", "monthly_wrapped", "user");

    Ok(ReportGeneratorInput {
        entity_id: user_entity_id,
        entity_type: "user".to_string(),
        report_type: "monthly_wrapped".to_string(),
        entity_name: "Monthly Wrapped".to_string(),
        workspace: workspace.to_path_buf(),
        prompt,
        ai_models,
        intel_hash,
        extra_data: None,
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

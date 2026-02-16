use std::collections::HashSet;

use chrono::{Datelike, NaiveDate, Utc};

use crate::db::{ActionDb, DbAction};
use crate::helpers::{build_external_account_hints, normalize_key};
use crate::types::{CalendarEvent, LiveProactiveSuggestion, MeetingType, TimeBlock};

fn meeting_type_label(meeting_type: &MeetingType) -> &'static str {
    match meeting_type {
        MeetingType::Customer => "customer",
        MeetingType::Qbr => "qbr",
        MeetingType::Training => "training",
        MeetingType::Internal => "internal",
        MeetingType::TeamSync => "team_sync",
        MeetingType::OneOnOne => "one_on_one",
        MeetingType::Partnership => "partnership",
        MeetingType::AllHands => "all_hands",
        MeetingType::External => "external",
        MeetingType::Personal => "personal",
    }
}

fn estimate_effort_minutes(action: &DbAction) -> u32 {
    let mut effort: u32 = match action.priority.as_str() {
        "P1" => 60,
        "P2" => 45,
        _ => 30,
    };
    if action.status == "waiting" {
        effort = effort.saturating_sub(10);
    }
    effort.max(20)
}

fn capacity_fit(block_minutes: u32, action: &DbAction) -> f32 {
    let effort = estimate_effort_minutes(action) as i32;
    let diff = (block_minutes as i32 - effort).unsigned_abs() as f32;
    (1.0 - (diff / 120.0)).clamp(0.05, 1.0)
}

fn urgency_impact(action: &DbAction, today: NaiveDate) -> f32 {
    let baseline: f32 = match action.due_date.as_deref() {
        Some(due) => match NaiveDate::parse_from_str(due, "%Y-%m-%d") {
            Ok(date) if date < today => 1.0,
            Ok(date) if date == today => 0.92,
            Ok(date) if date <= today + chrono::Duration::days(2) => 0.84,
            Ok(date) if date <= today + chrono::Duration::days(7) => 0.72,
            Ok(_) => 0.58,
            Err(_) => 0.5,
        },
        None => 0.45,
    };

    let priority_lift: f32 = match action.priority.as_str() {
        "P1" => 0.18,
        "P2" => 0.1,
        _ => 0.03,
    };

    (baseline + priority_lift).clamp(0.0, 1.0)
}

fn confidence(
    action: &DbAction,
    nearest_meeting_id: Option<&str>,
    day_events: &[CalendarEvent],
) -> f32 {
    let mut score: f32 = 0.45;
    if action.due_date.is_some() {
        score += 0.1;
    }
    if action.source_id.is_some() {
        score += 0.08;
    }

    if let Some(meeting_id) = nearest_meeting_id {
        score += 0.1;
        if action
            .source_id
            .as_deref()
            .is_some_and(|source_id| normalize_key(source_id) == normalize_key(meeting_id))
        {
            score += 0.15;
        }
    }

    if let Some(ref account_id) = action.account_id {
        let action_key = normalize_key(account_id);
        if day_events.iter().any(|event| {
            event
                .account
                .as_deref()
                .is_some_and(|account| normalize_key(account) == action_key)
        }) {
            score += 0.12;
        }
    }

    score.clamp(0.0, 1.0)
}

fn total_score(capacity_fit: f32, urgency_impact: f32, confidence: f32) -> f32 {
    (capacity_fit * 0.45) + (urgency_impact * 0.4) + (confidence * 0.15)
}

fn nearest_meeting_id_for_block(block: &TimeBlock, day_events: &[CalendarEvent]) -> Option<String> {
    let block_start = chrono::DateTime::parse_from_rfc3339(&block.start)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))?;

    day_events
        .iter()
        .min_by_key(|event| (event.start - block_start).num_minutes().unsigned_abs())
        .map(|event| {
            crate::workflow::deliver::meeting_primary_id(
                Some(event.id.as_str()),
                &event.title,
                &event.start.to_rfc3339(),
                meeting_type_label(&event.meeting_type),
            )
        })
        .filter(|id| !id.is_empty())
}

fn due_reason(action: &DbAction, today: NaiveDate) -> String {
    match action.due_date.as_deref() {
        Some(due) => match NaiveDate::parse_from_str(due, "%Y-%m-%d") {
            Ok(date) if date < today => "overdue follow-through".to_string(),
            Ok(date) if date == today => "due today".to_string(),
            Ok(date) if date <= today + chrono::Duration::days(2) => "due this week".to_string(),
            Ok(_) => "upcoming commitment".to_string(),
            Err(_) => "priority commitment".to_string(),
        },
        None => "priority opportunity".to_string(),
    }
}

fn build_reason(
    action: &DbAction,
    block_minutes: u32,
    capacity_fit: f32,
    today: NaiveDate,
) -> String {
    let fit_label = if capacity_fit >= 0.75 {
        "strong"
    } else if capacity_fit >= 0.55 {
        "good"
    } else {
        "stretch"
    };
    format!(
        "{} · {}m block is a {} fit",
        due_reason(action, today),
        block_minutes,
        fit_label
    )
}

fn select_suggestion_for_block(
    block: &TimeBlock,
    day_events: &[CalendarEvent],
    actions: &[DbAction],
    used_action_ids: &HashSet<String>,
    day: &str,
    date: NaiveDate,
    today: NaiveDate,
) -> Option<LiveProactiveSuggestion> {
    let nearest_meeting_id = nearest_meeting_id_for_block(block, day_events);

    let mut best: Option<(f32, f32, LiveProactiveSuggestion)> = None;

    for action in actions.iter().filter(|a| !used_action_ids.contains(&a.id)) {
        let cap = capacity_fit(block.duration_minutes, action);
        let urg = urgency_impact(action, today);
        let conf = confidence(action, nearest_meeting_id.as_deref(), day_events);
        let total = total_score(cap, urg, conf);

        let candidate = LiveProactiveSuggestion {
            day: day.to_string(),
            date: date.to_string(),
            start: block.start.clone(),
            end: block.end.clone(),
            duration_minutes: block.duration_minutes,
            title: action.title.clone(),
            reason: build_reason(action, block.duration_minutes, cap, today),
            source: "live".to_string(),
            action_id: Some(action.id.clone()),
            meeting_id: nearest_meeting_id.clone(),
            capacity_fit: cap,
            urgency_impact: urg,
            confidence: conf,
            total_score: total,
        };

        let replace = match &best {
            None => true,
            Some((best_total, best_urgency, _)) => {
                total > *best_total
                    || ((total - *best_total).abs() < f32::EPSILON && urg > *best_urgency)
            }
        };

        if replace {
            best = Some((total, urg, candidate));
        }
    }

    best.map(|(_, _, suggestion)| suggestion)
}

pub fn suggest_from_live_inputs(
    monday: NaiveDate,
    live_events: &[CalendarEvent],
    actions: &[DbAction],
    today: NaiveDate,
) -> Vec<LiveProactiveSuggestion> {
    let mut used_action_ids = HashSet::new();
    let mut output = Vec::new();

    for offset in 0..5 {
        let date = monday + chrono::Duration::days(offset);
        let day_events: Vec<CalendarEvent> = live_events
            .iter()
            .filter(|event| event.start.date_naive() == date && !event.is_all_day)
            .cloned()
            .collect();
        let blocks = crate::queries::schedule::available_blocks_from_live(&day_events, date);
        let day_label = date.format("%A").to_string();

        for block in blocks {
            if let Some(suggestion) = select_suggestion_for_block(
                &block,
                &day_events,
                actions,
                &used_action_ids,
                &day_label,
                date,
                today,
            ) {
                if let Some(ref action_id) = suggestion.action_id {
                    used_action_ids.insert(action_id.clone());
                }
                output.push(suggestion);
            }
        }
    }

    output
}

pub fn load_live_suggestion_inputs(
    db: &ActionDb,
) -> Result<(HashSet<String>, Vec<DbAction>), String> {
    let account_hints = build_external_account_hints(db);
    let actions = db
        .get_focus_candidate_actions(7)
        .map_err(|e| format!("Failed to query candidate actions: {}", e))?;
    Ok((account_hints, actions))
}

/// Fetch and classify week calendar events from Google Calendar API.
///
/// Separated from suggestion computation to enable TTL caching (W6).
pub async fn fetch_week_events(
    config: &crate::types::Config,
    account_hints: &HashSet<String>,
) -> Result<Vec<CalendarEvent>, String> {
    let tz: chrono_tz::Tz = config
        .schedules
        .today
        .timezone
        .parse()
        .unwrap_or(chrono_tz::America::New_York);
    let today = Utc::now().with_timezone(&tz).date_naive();
    let monday = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
    let friday = monday + chrono::Duration::days(4);

    let access_token = crate::google_api::get_valid_access_token()
        .await
        .map_err(|e| format!("Google auth unavailable: {}", e))?;

    let raw_events = crate::google_api::calendar::fetch_events(&access_token, monday, friday)
        .await
        .map_err(|e| format!("Failed to fetch live week events: {}", e))?;

    let user_domains = config.resolved_user_domains();
    let live_events: Vec<CalendarEvent> = raw_events
        .iter()
        .map(|raw| {
            crate::google_api::classify::classify_meeting_multi(raw, &user_domains, account_hints)
        })
        .map(|classified| classified.to_calendar_event())
        .filter(|event| !event.is_all_day)
        .collect();

    Ok(live_events)
}

/// Compute proactive suggestions from pre-fetched (possibly cached) calendar events.
///
/// Pure computation — no API calls. Safe to call with cached data.
pub fn compute_suggestions_from_events(
    config: &crate::types::Config,
    live_events: &[CalendarEvent],
    actions: &[DbAction],
) -> Result<Vec<LiveProactiveSuggestion>, String> {
    let tz: chrono_tz::Tz = config
        .schedules
        .today
        .timezone
        .parse()
        .unwrap_or(chrono_tz::America::New_York);
    let today = Utc::now().with_timezone(&tz).date_naive();
    let monday = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);

    Ok(suggest_from_live_inputs(
        monday,
        live_events,
        actions,
        today,
    ))
}

/// Full fetch + compute pipeline. Convenience for callers that don't need caching.
pub async fn get_live_proactive_suggestions(
    config: &crate::types::Config,
    account_hints: HashSet<String>,
    actions: Vec<DbAction>,
) -> Result<Vec<LiveProactiveSuggestion>, String> {
    let live_events = fetch_week_events(config, &account_hints).await?;
    compute_suggestions_from_events(config, &live_events, &actions)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn make_action(id: &str, title: &str, priority: &str, due_date: Option<&str>) -> DbAction {
        DbAction {
            id: id.to_string(),
            title: title.to_string(),
            priority: priority.to_string(),
            status: "pending".to_string(),
            created_at: "2026-02-10T08:00:00Z".to_string(),
            due_date: due_date.map(|d| d.to_string()),
            completed_at: None,
            account_id: None,
            project_id: None,
            source_type: None,
            source_id: None,
            source_label: None,
            context: None,
            waiting_on: None,
            updated_at: "2026-02-10T08:00:00Z".to_string(),
            person_id: None,
        }
    }

    fn make_event(id: &str, start_h: u32, end_h: u32, day: NaiveDate) -> CalendarEvent {
        CalendarEvent {
            id: id.to_string(),
            title: "Meeting".to_string(),
            start: Utc
                .with_ymd_and_hms(day.year(), day.month(), day.day(), start_h, 0, 0)
                .unwrap(),
            end: Utc
                .with_ymd_and_hms(day.year(), day.month(), day.day(), end_h, 0, 0)
                .unwrap(),
            meeting_type: MeetingType::Internal,
            account: None,
            attendees: vec![],
            is_all_day: false,
        }
    }

    #[test]
    fn deterministic_score_orders_urgent_before_nonurgent() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 10).unwrap();
        let block = TimeBlock {
            day: "Tuesday".to_string(),
            start: "2026-02-10T14:00:00+00:00".to_string(),
            end: "2026-02-10T15:00:00+00:00".to_string(),
            duration_minutes: 60,
            suggested_use: None,
            action_id: None,
            meeting_id: None,
        };
        let actions = vec![
            make_action("a1", "Overdue escalation", "P2", Some("2026-02-09")),
            make_action("a2", "Nice to have cleanup", "P3", None),
        ];

        let best = select_suggestion_for_block(
            &block,
            &[],
            &actions,
            &HashSet::new(),
            "Tuesday",
            today,
            today,
        )
        .expect("suggestion");

        assert_eq!(best.action_id.as_deref(), Some("a1"));
    }

    #[test]
    fn suggestions_dedupe_reused_actions_across_blocks() {
        let monday = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap();
        let today = monday;

        let events = vec![make_event("evt-1", 10, 11, monday)];
        let actions = vec![
            make_action("a1", "Critical follow-up", "P1", Some("2026-02-09")),
            make_action("a2", "Secondary task", "P2", Some("2026-02-10")),
        ];

        let suggestions = suggest_from_live_inputs(monday, &events, &actions, today);
        let action_ids: Vec<String> = suggestions
            .iter()
            .filter_map(|s| s.action_id.clone())
            .collect();
        let unique_ids: HashSet<String> = action_ids.iter().cloned().collect();

        assert_eq!(action_ids.len(), unique_ids.len());
    }

    #[test]
    fn live_divergence_when_meeting_added_changes_capacity() {
        let day = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap();
        let actions = vec![make_action("a1", "Do work", "P2", Some("2026-02-09"))];

        let mut events = vec![make_event("evt-am", 10, 11, day)];
        let before = suggest_from_live_inputs(day, &events, &actions, day);

        events.push(make_event("evt-midday", 12, 13, day));
        let after = suggest_from_live_inputs(day, &events, &actions, day);

        let before_total: u32 = before.iter().map(|s| s.duration_minutes).sum();
        let after_total: u32 = after.iter().map(|s| s.duration_minutes).sum();
        assert!(after_total < before_total || after.len() <= before.len());
    }

    #[test]
    fn live_divergence_when_meeting_removed_restores_capacity() {
        let day = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap();
        let actions = vec![make_action("a1", "Do work", "P2", Some("2026-02-09"))];

        let with_extra = vec![
            make_event("evt-am", 10, 11, day),
            make_event("evt-midday", 12, 13, day),
        ];
        let reduced = suggest_from_live_inputs(day, &with_extra, &actions, day);

        let without_extra = vec![make_event("evt-am", 10, 11, day)];
        let restored = suggest_from_live_inputs(day, &without_extra, &actions, day);

        let reduced_total: u32 = reduced.iter().map(|s| s.duration_minutes).sum();
        let restored_total: u32 = restored.iter().map(|s| s.duration_minutes).sum();
        assert!(restored_total >= reduced_total);
    }
}

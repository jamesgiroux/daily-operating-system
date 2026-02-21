use chrono::{Local, NaiveDate};

use crate::db::DbAction;
use crate::types::{FocusImplications, PrioritizedFocusAction};

const P1_BASE_SCORE: i32 = 60;
const P2_BASE_SCORE: i32 = 40;
const P3_BASE_SCORE: i32 = 20;

#[derive(Debug, Clone)]
struct ScoredAction {
    action: DbAction,
    score: i32,
    urgency_points: i32,
    effort_minutes: u32,
    reason: String,
    due_today_or_overdue: bool,
    blocked: bool,
}

pub fn prioritize_actions(
    actions: Vec<DbAction>,
    available_minutes: u32,
) -> (Vec<PrioritizedFocusAction>, Vec<String>, FocusImplications) {
    let today = Local::now().date_naive();

    let mut scored: Vec<ScoredAction> = actions
        .into_iter()
        .map(|action| score_action(action, today))
        .collect();

    scored.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| {
                compare_due_date(a.action.due_date.as_deref(), b.action.due_date.as_deref())
            })
            .then_with(|| a.action.title.cmp(&b.action.title))
    });

    let mut remaining = available_minutes;
    let mut prioritized = Vec::with_capacity(scored.len());

    for item in scored {
        let feasible = item.effort_minutes <= remaining;
        if feasible {
            remaining = remaining.saturating_sub(item.effort_minutes);
        }

        let at_risk =
            (item.due_today_or_overdue && !feasible) || (item.blocked && item.urgency_points >= 40);

        prioritized.push(PrioritizedFocusAction {
            action: item.action,
            score: item.score,
            effort_minutes: item.effort_minutes,
            feasible,
            at_risk,
            reason: item.reason,
        });
    }

    let top_three = pick_top_three(&prioritized);
    let implications = build_implications(&prioritized, available_minutes);

    (prioritized, top_three, implications)
}

fn score_action(action: DbAction, today: NaiveDate) -> ScoredAction {
    let base = match action.priority.as_str() {
        "P1" => P1_BASE_SCORE,
        "P2" => P2_BASE_SCORE,
        _ => P3_BASE_SCORE,
    };

    let due = action
        .due_date
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let mut urgency_points = 0;
    let mut due_label = "No due date".to_string();
    let mut due_today_or_overdue = false;

    if let Some(due_date) = due {
        let delta = (due_date - today).num_days();
        if delta < 0 {
            let overdue_days = (-delta) as i32;
            urgency_points = 50 + overdue_days.min(10);
            due_label = format!(
                "Overdue by {} day{}",
                overdue_days,
                if overdue_days == 1 { "" } else { "s" }
            );
            due_today_or_overdue = true;
        } else if delta == 0 {
            urgency_points = 40;
            due_label = "Due today".to_string();
            due_today_or_overdue = true;
        } else if delta == 1 {
            urgency_points = 25;
            due_label = "Due tomorrow".to_string();
        } else if delta <= 3 {
            urgency_points = 15;
            due_label = format!("Due in {} days", delta);
        } else if delta <= 7 {
            urgency_points = 8;
            due_label = format!("Due in {} days", delta);
        } else {
            due_label = format!("Due on {}", due_date);
        }
    }

    let customer_points = if action.account_id.is_some() { 15 } else { 0 };
    let blocked = action.status == "waiting"
        || action
            .waiting_on
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
    let blocked_penalty = if blocked { -25 } else { 0 };

    let score = base + urgency_points + customer_points + blocked_penalty;
    let effort_minutes = estimate_effort_minutes(&action.title, &action.priority);

    let mut reason_parts = vec![due_label, format!("{} priority", action.priority)];
    if customer_points > 0 {
        reason_parts.push("customer-facing".to_string());
    }
    if blocked {
        reason_parts.push("currently blocked/waiting".to_string());
    }

    let reason = reason_parts.join("; ");

    ScoredAction {
        action,
        score,
        urgency_points,
        effort_minutes,
        reason,
        due_today_or_overdue,
        blocked,
    }
}

fn compare_due_date(a: Option<&str>, b: Option<&str>) -> std::cmp::Ordering {
    let parse = |s: Option<&str>| s.and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());
    let a_d = parse(a);
    let b_d = parse(b);

    // Earlier dates rank higher. Missing dates rank last.
    match (a_d, b_d) {
        (Some(da), Some(db)) => da.cmp(&db),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn estimate_effort_minutes(title: &str, priority: &str) -> u32 {
    let t = title.to_lowercase();
    let quick_markers = ["quick", "reply", "email", "review"];
    let deep_markers = ["deep", "proposal", "design", "plan", "deck", "analysis"];

    if quick_markers.iter().any(|m| t.contains(m)) {
        return 20;
    }
    if deep_markers.iter().any(|m| t.contains(m)) {
        return 90;
    }

    match priority {
        "P1" => 45,
        "P2" => 30,
        _ => 20,
    }
}

fn pick_top_three(actions: &[PrioritizedFocusAction]) -> Vec<String> {
    let mut picked = Vec::new();

    for action in actions.iter().filter(|a| a.feasible) {
        if picked.len() == 3 {
            break;
        }
        picked.push(action.action.id.clone());
    }

    if picked.len() < 3 {
        for action in actions.iter() {
            if picked.len() == 3 {
                break;
            }
            if picked.iter().any(|id| id == &action.action.id) {
                continue;
            }
            picked.push(action.action.id.clone());
        }
    }

    picked
}

fn build_implications(
    actions: &[PrioritizedFocusAction],
    available_minutes: u32,
) -> FocusImplications {
    let achievable_count = actions.iter().filter(|a| a.feasible).count() as u32;
    let at_risk_count = actions.iter().filter(|a| a.at_risk).count() as u32;
    let total_count = actions.len() as u32;

    let summary = if total_count == 0 {
        "No pending or waiting actions to prioritize today.".to_string()
    } else {
        format!(
            "You have {} minutes available. {} of {} actions look achievable today; {} are at risk.",
            available_minutes, achievable_count, total_count, at_risk_count
        )
    };

    FocusImplications {
        achievable_count,
        total_count,
        at_risk_count,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(
        id: &str,
        title: &str,
        priority: &str,
        due_date: Option<&str>,
        status: &str,
        account: Option<&str>,
        waiting_on: Option<&str>,
    ) -> DbAction {
        DbAction {
            id: id.to_string(),
            title: title.to_string(),
            priority: priority.to_string(),
            status: status.to_string(),
            created_at: "2026-02-01T00:00:00Z".to_string(),
            due_date: due_date.map(ToString::to_string),
            completed_at: None,
            account_id: account.map(ToString::to_string),
            project_id: None,
            source_type: None,
            source_id: None,
            source_label: None,
            context: None,
            waiting_on: waiting_on.map(ToString::to_string),
            updated_at: "2026-02-01T00:00:00Z".to_string(),
            person_id: None,
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
        }
    }

    #[test]
    fn overdue_p1_ranks_above_non_urgent() {
        let today = Local::now().date_naive();
        let overdue = today.pred_opt().unwrap().to_string();
        let next_week = (today + chrono::Duration::days(8)).to_string();

        let actions = vec![
            action(
                "a1",
                "Write proposal",
                "P1",
                Some(&next_week),
                "pending",
                None,
                None,
            ),
            action(
                "a2",
                "Reply to customer",
                "P2",
                Some(&overdue),
                "pending",
                Some("acme"),
                None,
            ),
        ];

        let (ranked, _, _) = prioritize_actions(actions, 120);
        assert_eq!(ranked[0].action.id, "a2");
    }

    #[test]
    fn waiting_penalty_reduces_rank() {
        let today = Local::now().date_naive().to_string();
        let actions = vec![
            action(
                "a1",
                "Deep analysis",
                "P1",
                Some(&today),
                "pending",
                Some("acme"),
                None,
            ),
            action(
                "a2",
                "Deep analysis",
                "P1",
                Some(&today),
                "waiting",
                Some("acme"),
                Some("Legal"),
            ),
        ];

        let (ranked, _, _) = prioritize_actions(actions, 120);
        assert_eq!(ranked[0].action.id, "a1");
    }

    #[test]
    fn achievable_respects_capacity() {
        let today = Local::now().date_naive().to_string();
        let actions = vec![
            action(
                "a1",
                "Deep proposal",
                "P1",
                Some(&today),
                "pending",
                None,
                None,
            ), // 90
            action(
                "a2",
                "Quick reply",
                "P1",
                Some(&today),
                "pending",
                None,
                None,
            ), // 20
        ];

        let (ranked, _, implications) = prioritize_actions(actions, 60);
        assert!(!ranked[0].feasible);
        assert!(ranked[1].feasible);
        assert_eq!(implications.achievable_count, 1);
    }

    #[test]
    fn top_three_fills_with_stretch_items() {
        let today = Local::now().date_naive().to_string();
        let actions = vec![
            action(
                "a1",
                "Deep proposal",
                "P1",
                Some(&today),
                "pending",
                None,
                None,
            ),
            action(
                "a2",
                "Deep design",
                "P1",
                Some(&today),
                "pending",
                None,
                None,
            ),
            action("a3", "Deep plan", "P1", Some(&today), "pending", None, None),
        ];

        let (_, top_three, _) = prioritize_actions(actions, 30);
        assert_eq!(top_three.len(), 3);
    }

    #[test]
    fn marks_overdue_unfeasible_as_at_risk() {
        let overdue = Local::now().date_naive().pred_opt().unwrap().to_string();
        let actions = vec![action(
            "a1",
            "Deep analysis",
            "P1",
            Some(&overdue),
            "pending",
            Some("acme"),
            None,
        )];

        let (ranked, _, implications) = prioritize_actions(actions, 15);
        assert!(ranked[0].at_risk);
        assert_eq!(implications.at_risk_count, 1);
    }
}

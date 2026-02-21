//! Executive Intelligence computation (I42).
//!
//! Pure computation layer that cross-references SQLite data + today's schedule
//! to surface signals: decisions due, stale delegations, portfolio alerts,
//! cancelable meetings, and skip-today items.
//!
//! All mechanical sections (DECIDE, WAITING ON, PORTFOLIO, CANCEL/PROTECT)
//! are pure Rust queries. SKIP TODAY is populated from AI enrichment output
//! (passed in, not computed here).

use serde::{Deserialize, Serialize};

use crate::db::{ActionDb, DbAccount};
use crate::types::{Meeting, MeetingType, OverlayStatus};

/// Thresholds for intelligence signals.
const STALE_DELEGATION_DAYS: i32 = 3;
const DECISION_LOOKAHEAD_DAYS: i32 = 3; // 72 hours
const RENEWAL_ALERT_DAYS: i32 = 60;
const STALE_ACCOUNT_DAYS: i32 = 30;

// ─────────────────────────────────────────────────────────────────────
// Signal types
// ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionSignal {
    pub action_id: String,
    pub title: String,
    pub due_date: Option<String>,
    pub account: Option<String>,
    pub priority: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationSignal {
    pub action_id: String,
    pub title: String,
    pub waiting_on: Option<String>,
    pub created_at: String,
    pub account: Option<String>,
    pub days_stale: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioAlert {
    pub account_id: String,
    pub account_name: String,
    pub signal: PortfolioSignalType,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioSignalType {
    RenewalApproaching,
    StaleContact,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelableSignal {
    pub meeting_id: String,
    pub title: String,
    pub time: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkipSignal {
    pub item: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonAlert {
    pub person_id: String,
    pub name: String,
    pub signal: PersonSignalType,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonSignalType {
    StaleRelationship,
    NewFace,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalCounts {
    pub decisions: usize,
    pub delegations: usize,
    pub portfolio_alerts: usize,
    pub cancelable: usize,
    pub skip_today: usize,
    pub person_alerts: usize,
}

/// Top-level intelligence result returned to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutiveIntelligence {
    pub decisions: Vec<DecisionSignal>,
    pub delegations: Vec<DelegationSignal>,
    pub portfolio_alerts: Vec<PortfolioAlert>,
    pub cancelable_meetings: Vec<CancelableSignal>,
    pub skip_today: Vec<SkipSignal>,
    pub person_alerts: Vec<PersonAlert>,
    pub signal_counts: SignalCounts,
}

// ─────────────────────────────────────────────────────────────────────
// Computation
// ─────────────────────────────────────────────────────────────────────

/// Compute executive intelligence from DB data and today's schedule.
///
/// `schedule_meetings` is the merged meeting list from the dashboard
/// (includes overlay status). `profile` gates portfolio alerts to CS.
/// `skip_today` is passed in from cached AI enrichment output.
pub fn compute_executive_intelligence(
    db: &ActionDb,
    schedule_meetings: &[Meeting],
    profile: &str,
    skip_today: Vec<SkipSignal>,
) -> ExecutiveIntelligence {
    let decisions = compute_decisions(db);
    let delegations = compute_delegations(db);
    let portfolio_alerts = compute_portfolio_alerts(db, profile);
    let cancelable_meetings = compute_cancelable(schedule_meetings);
    let person_alerts = compute_person_alerts(db);

    let signal_counts = SignalCounts {
        decisions: decisions.len(),
        delegations: delegations.len(),
        portfolio_alerts: portfolio_alerts.len(),
        cancelable: cancelable_meetings.len(),
        skip_today: skip_today.len(),
        person_alerts: person_alerts.len(),
    };

    ExecutiveIntelligence {
        decisions,
        delegations,
        portfolio_alerts,
        cancelable_meetings,
        skip_today,
        person_alerts,
        signal_counts,
    }
}

/// AI-flagged actions that need decisions within the lookahead window.
fn compute_decisions(db: &ActionDb) -> Vec<DecisionSignal> {
    let actions = db
        .get_flagged_decisions(DECISION_LOOKAHEAD_DAYS)
        .unwrap_or_default();

    actions
        .into_iter()
        .map(|a| DecisionSignal {
            action_id: a.id,
            title: a.title,
            due_date: a.due_date,
            account: a.account_id,
            priority: a.priority,
        })
        .collect()
}

/// Waiting actions that have been stale for more than the threshold.
fn compute_delegations(db: &ActionDb) -> Vec<DelegationSignal> {
    let actions = db
        .get_stale_delegations(STALE_DELEGATION_DAYS)
        .unwrap_or_default();

    actions
        .into_iter()
        .map(|a| {
            let days = days_since(&a.created_at);
            DelegationSignal {
                action_id: a.id,
                title: a.title,
                waiting_on: a.waiting_on,
                created_at: a.created_at,
                account: a.account_id,
                days_stale: days,
            }
        })
        .collect()
}

/// Portfolio alerts: renewals + stale contacts. CS-profile only.
fn compute_portfolio_alerts(db: &ActionDb, profile: &str) -> Vec<PortfolioAlert> {
    if profile != "customer-success" {
        return Vec::new();
    }

    let mut alerts = Vec::new();

    // Renewal alerts
    if let Ok(accounts) = db.get_renewal_alerts(RENEWAL_ALERT_DAYS) {
        for acct in accounts {
            let days = days_until_date(acct.contract_end.as_deref().unwrap_or(""));
            let detail = format_renewal_detail(&acct, days);
            alerts.push(PortfolioAlert {
                account_id: acct.id,
                account_name: acct.name,
                signal: PortfolioSignalType::RenewalApproaching,
                detail,
            });
        }
    }

    // Stale contact alerts
    if let Ok(accounts) = db.get_stale_accounts(STALE_ACCOUNT_DAYS) {
        for acct in accounts {
            let days = days_since(&acct.updated_at);
            alerts.push(PortfolioAlert {
                account_id: acct.id,
                account_name: acct.name,
                signal: PortfolioSignalType::StaleContact,
                detail: format!("No contact in {} days", days),
            });
        }
    }

    alerts
}

/// Internal meetings with no prep context — candidates for cancellation.
fn compute_cancelable(meetings: &[Meeting]) -> Vec<CancelableSignal> {
    meetings
        .iter()
        .filter(|m| is_cancelable_candidate(m))
        .map(|m| CancelableSignal {
            meeting_id: m.id.clone(),
            title: m.title.clone(),
            time: m.time.clone(),
            reason: "Internal meeting with no prep context".to_string(),
        })
        .collect()
}

/// A meeting is a cancellation candidate if it's internal/team_sync,
/// has no prep, and isn't already cancelled.
fn is_cancelable_candidate(m: &Meeting) -> bool {
    let is_internal = matches!(
        m.meeting_type,
        MeetingType::Internal | MeetingType::TeamSync
    );
    let no_prep = !m.has_prep;
    let not_cancelled = m.overlay_status.as_ref() != Some(&OverlayStatus::Cancelled);

    is_internal && no_prep && not_cancelled
}

/// Person-level alerts: stale relationships and new faces (I51).
fn compute_person_alerts(db: &ActionDb) -> Vec<PersonAlert> {
    let mut alerts = Vec::new();
    let now = chrono::Utc::now();

    // Get all external people
    let people = db.get_people(Some("external")).unwrap_or_default();

    for person in &people {
        // Stale relationship: cold external contact linked to an entity
        if let Ok(signals) = db.get_person_signals(&person.id) {
            if signals.temperature == "cold" {
                // Only alert if linked to at least one entity
                if let Ok(entities) = db.get_entities_for_person(&person.id) {
                    if !entities.is_empty() {
                        let entity_names: Vec<String> =
                            entities.iter().map(|e| e.name.clone()).collect();
                        alerts.push(PersonAlert {
                            person_id: person.id.clone(),
                            name: person.name.clone(),
                            signal: PersonSignalType::StaleRelationship,
                            detail: format!(
                                "No contact in 60+ days (linked to {})",
                                entity_names.join(", ")
                            ),
                        });
                    }
                }
            }
        }
    }

    // New faces: people first seen in the last 7 days
    let all_people = db.get_people(None).unwrap_or_default();
    for person in &all_people {
        if let Some(ref first_seen) = person.first_seen {
            if let Ok(fs) = chrono::DateTime::parse_from_rfc3339(first_seen) {
                let days = (now - fs.with_timezone(&chrono::Utc)).num_days();
                if days <= 7 {
                    alerts.push(PersonAlert {
                        person_id: person.id.clone(),
                        name: person.name.clone(),
                        signal: PersonSignalType::NewFace,
                        detail: format!(
                            "New contact ({})",
                            person.organization.as_deref().unwrap_or("unknown org")
                        ),
                    });
                }
            }
        }
    }

    alerts
}

// ─────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────

/// Days elapsed since a given RFC 3339 or ISO date string.
fn days_since(date_str: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(date_str)
        .map(|dt| (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_days())
        .unwrap_or(0)
}

/// Days until a given YYYY-MM-DD date string (positive = future).
fn days_until_date(date_str: &str) -> i64 {
    chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map(|d| {
            let today = chrono::Utc::now().date_naive();
            (d - today).num_days()
        })
        .unwrap_or(0)
}

fn format_renewal_detail(acct: &DbAccount, days_until: i64) -> String {
    let arr_part = acct
        .arr
        .map(|a| format!(" (${:.0}k ARR)", a / 1000.0))
        .unwrap_or_default();
    format!("Renewal in {} days{}", days_until, arr_part)
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::types::{Meeting, MeetingType, OverlayStatus};

    fn sample_meeting(id: &str, title: &str, mt: MeetingType) -> Meeting {
        Meeting {
            id: id.to_string(),
            calendar_event_id: None,
            time: "09:00".to_string(),
            end_time: Some("10:00".to_string()),
            start_iso: None,
            title: title.to_string(),
            meeting_type: mt,
            prep: None,
            is_current: None,
            prep_file: None,
            has_prep: false,
            overlay_status: Some(OverlayStatus::New),
            prep_reviewed: None,
            linked_entities: None,
            suggested_unarchive_account_id: None,
            intelligence_quality: None,
            calendar_attendees: None,
            calendar_description: None,
        }
    }

    #[test]
    fn test_empty_state() {
        let db = test_db();
        let result = compute_executive_intelligence(&db, &[], "customer-success", vec![]);
        assert!(result.decisions.is_empty());
        assert!(result.delegations.is_empty());
        assert!(result.portfolio_alerts.is_empty());
        assert!(result.cancelable_meetings.is_empty());
        assert!(result.skip_today.is_empty());
        assert_eq!(result.signal_counts.decisions, 0);
    }

    #[test]
    fn test_cancelable_internal_no_prep() {
        let meetings = vec![
            sample_meeting("m1", "Team standup", MeetingType::Internal),
            sample_meeting("m2", "Acme QBR", MeetingType::Customer),
        ];

        let db = test_db();
        let result = compute_executive_intelligence(&db, &meetings, "customer-success", vec![]);
        assert_eq!(result.cancelable_meetings.len(), 1);
        assert_eq!(result.cancelable_meetings[0].title, "Team standup");
    }

    #[test]
    fn test_cancelable_excludes_prepped() {
        let mut m = sample_meeting("m1", "Team standup", MeetingType::Internal);
        m.has_prep = true;

        let db = test_db();
        let result = compute_executive_intelligence(&db, &[m], "customer-success", vec![]);
        assert!(result.cancelable_meetings.is_empty());
    }

    #[test]
    fn test_cancelable_excludes_cancelled() {
        let mut m = sample_meeting("m1", "Team standup", MeetingType::Internal);
        m.overlay_status = Some(OverlayStatus::Cancelled);

        let db = test_db();
        let result = compute_executive_intelligence(&db, &[m], "customer-success", vec![]);
        assert!(result.cancelable_meetings.is_empty());
    }

    #[test]
    fn test_cancelable_team_sync() {
        let meetings = vec![sample_meeting("m1", "Weekly sync", MeetingType::TeamSync)];

        let db = test_db();
        let result = compute_executive_intelligence(&db, &meetings, "customer-success", vec![]);
        assert_eq!(result.cancelable_meetings.len(), 1);
    }

    #[test]
    fn test_portfolio_alerts_cs_only() {
        let db = test_db();

        // Insert an account with upcoming renewal
        let acct = crate::db::DbAccount {
            id: "renew-corp".to_string(),
            name: "Renew Corp".to_string(),
            lifecycle: Some("steady-state".to_string()),
            arr: Some(100_000.0),
            health: Some("green".to_string()),
            contract_start: None,
            contract_end: Some(
                (chrono::Utc::now() + chrono::Duration::days(30))
                    .format("%Y-%m-%d")
                    .to_string(),
            ),
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: chrono::Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
        metadata: None,
        };
        db.upsert_account(&acct).expect("upsert");

        // CS profile — should see alert
        let cs_result = compute_executive_intelligence(&db, &[], "customer-success", vec![]);
        assert!(!cs_result.portfolio_alerts.is_empty());

        // Non-CS profile — should NOT see portfolio alerts
        let general_result = compute_executive_intelligence(&db, &[], "general", vec![]);
        assert!(general_result.portfolio_alerts.is_empty());
    }

    #[test]
    fn test_skip_today_passed_through() {
        let db = test_db();
        let skip = vec![
            SkipSignal {
                item: "Low-pri email batch".to_string(),
                reason: "No deadlines today".to_string(),
            },
            SkipSignal {
                item: "Archive cleanup".to_string(),
                reason: "Already done this week".to_string(),
            },
        ];

        let result = compute_executive_intelligence(&db, &[], "customer-success", skip);
        assert_eq!(result.skip_today.len(), 2);
        assert_eq!(result.signal_counts.skip_today, 2);
    }

    #[test]
    fn test_signal_counts_aggregate() {
        let db = test_db();

        // Create a stale delegation
        let now = chrono::Utc::now().to_rfc3339();
        let stale_action = crate::db::DbAction {
            id: "wait-old".to_string(),
            title: "Stale delegation".to_string(),
            priority: "P2".to_string(),
            status: "waiting".to_string(),
            created_at: "2020-01-01T00:00:00Z".to_string(),
            due_date: None,
            completed_at: None,
            account_id: None,
            project_id: None,
            source_type: None,
            source_id: None,
            source_label: None,
            context: None,
            waiting_on: Some("Legal".to_string()),
            updated_at: now,
            person_id: None,
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
        };
        db.upsert_action(&stale_action).expect("insert");

        let meetings = vec![sample_meeting("m1", "Standup", MeetingType::Internal)];

        let result = compute_executive_intelligence(&db, &meetings, "customer-success", vec![]);
        assert_eq!(result.signal_counts.delegations, 1);
        assert_eq!(result.signal_counts.cancelable, 1);
        assert_eq!(result.signal_counts.decisions, 0);
    }

    #[test]
    fn test_days_since_helper() {
        // A date far in the past should give a large positive number
        let days = days_since("2020-01-01T00:00:00Z");
        assert!(days > 1000);

        // Invalid date returns 0
        let bad = days_since("not-a-date");
        assert_eq!(bad, 0);
    }

    #[test]
    fn test_days_until_date_helper() {
        let future = (chrono::Utc::now() + chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        let days = days_until_date(&future);
        assert!((29..=31).contains(&days));

        let past = "2020-01-01";
        let days = days_until_date(past);
        assert!(days < 0);
    }
}

//! Watch composer: service-owned triage for action and claim-backed rows.
//!
//! | Watch row variant | Source | Default trust |
//! |---|---|---|
//! | `WatchSuggestedActionRow` | Active surfaced `suggested_outcome` meeting claims with a materialized action id. | Claim trust score, or `Unscored` when absent. |
//! | `WatchOpenActionRow` | Action lifecycle status for work pressing today. | `Unscored`. |
//! | `WatchParkedRow` | Active action snooze records with a non-empty reason. | `Unscored`. |
//! | `WatchAgingRow` | Backlog action lifecycle rows at or beyond the aging threshold. | `Unscored`. |

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

use crate::abilities::provenance::trust::claim_trust_band_from_score;
use crate::abilities::trust::TrustBand;
use crate::db::actions::{ActionSnoozeRecord, WatchSuggestedOutcomeClaim};
use crate::db::DbAction;
use crate::services::briefing_view_model::{
    CorrectionState, InferredActionOption, InferredActionSelectorViewModel, LifecycleMixin,
    RenderedProvenanceSummary, TrustBandWire, TrustMixin, WatchAgingOption, WatchAgingOptionId,
    WatchAgingRow, WatchOpenActionRow, WatchParkedRow, WatchRowViewModel, WatchSuggestedActionRow,
    WatchViewModel,
};
use crate::services::context::Clock;
use crate::state::AppState;

const AGING_THRESHOLD_DAYS: i64 = 14;

#[derive(Default)]
struct WatchInputs {
    actions: Vec<DbAction>,
    suggested: Vec<WatchSuggestedCandidate>,
    snoozes: Vec<ActionSnoozeRecord>,
}

#[derive(Clone)]
struct WatchSuggestedCandidate {
    claim: WatchSuggestedOutcomeClaim,
    action: DbAction,
}

pub async fn compose_watch(state: &AppState) -> WatchViewModel {
    let now = state.clock.now();
    let today = now.date_naive();
    let now_iso = now.to_rfc3339();

    let inputs = state
        .db_write(move |db| {
            let mut suggested = Vec::new();
            for claim in db
                .get_watch_suggested_outcome_claims()
                .map_err(|e| e.to_string())?
            {
                if !meeting_is_today(&claim, today) {
                    continue;
                }
                let title = suggested_action_title(&claim.text);
                match db.get_or_create_watch_suggested_action(&claim, &title, &now_iso) {
                    Ok(action) => suggested.push(WatchSuggestedCandidate { claim, action }),
                    Err(error) => {
                        log::warn!(
                            "watch: failed to materialize suggested action from claim: {error}"
                        );
                    }
                }
            }

            Ok(WatchInputs {
                actions: db
                    .get_watch_action_candidates()
                    .map_err(|e| e.to_string())?,
                suggested,
                snoozes: db.get_action_snoozes().map_err(|e| e.to_string())?,
            })
        })
        .await
        .unwrap_or_default();

    let rows = compose_watch_rows(inputs, now, today);
    let count = rows.len();

    WatchViewModel {
        label: "Watch".to_string(),
        heading: "Worth a look".to_string(),
        count_label: count.to_string(),
        summary: format_summary(count),
        rows,
    }
}

fn format_summary(count: usize) -> String {
    if count == 0 {
        "Nothing pressing today.".to_string()
    } else if count == 1 {
        "1 item to triage.".to_string()
    } else {
        format!("{count} items to triage.")
    }
}

fn compose_watch_rows(
    inputs: WatchInputs,
    now: DateTime<Utc>,
    today: NaiveDate,
) -> Vec<WatchRowViewModel> {
    let active_snoozes: HashMap<String, ActionSnoozeRecord> = inputs
        .snoozes
        .into_iter()
        .filter(|snooze| snooze_is_active(snooze, now, today))
        .map(|snooze| (snooze.action_id.clone(), snooze))
        .collect();
    let suggested_by_action: HashMap<String, WatchSuggestedCandidate> = inputs
        .suggested
        .into_iter()
        .filter(|candidate| candidate.action.status == crate::action_status::BACKLOG)
        .map(|candidate| (candidate.action.id.clone(), candidate))
        .collect();

    let mut rows = Vec::new();
    let mut seen = HashSet::new();

    for action in inputs.actions {
        if seen.contains(&action.id) || is_terminal_action(&action) {
            continue;
        }

        let suggested = suggested_by_action.get(&action.id);
        let open_pressing = open_action_presses_today(&action, today);
        let suggested_pressing =
            suggested.is_some() && action.status == crate::action_status::BACKLOG;

        if let Some(snooze) = active_snoozes.get(&action.id) {
            if open_pressing || suggested_pressing {
                seen.insert(action.id.clone());
                if let Some(row) = map_parked_row(&action, snooze) {
                    rows.push(row);
                }
            } else if action.status == crate::action_status::BACKLOG {
                seen.insert(action.id.clone());
            }
            continue;
        }

        if let Some(row) = map_aging_row(&action, today) {
            seen.insert(action.id.clone());
            rows.push(row);
            continue;
        }

        if let Some(candidate) = suggested {
            seen.insert(action.id.clone());
            rows.push(map_suggested_action_row(candidate));
            continue;
        }

        if open_pressing {
            seen.insert(action.id.clone());
            rows.push(map_open_action_row(&action));
        }
    }

    for candidate in suggested_by_action.into_values() {
        if seen.contains(&candidate.action.id) || is_terminal_action(&candidate.action) {
            continue;
        }
        if let Some(snooze) = active_snoozes.get(&candidate.action.id) {
            seen.insert(candidate.action.id.clone());
            if let Some(row) = map_parked_row(&candidate.action, snooze) {
                rows.push(row);
            }
            continue;
        }
        seen.insert(candidate.action.id.clone());
        rows.push(map_suggested_action_row(&candidate));
    }

    rows
}

fn map_open_action_row(action: &DbAction) -> WatchRowViewModel {
    WatchRowViewModel::OpenAction(WatchOpenActionRow {
        trust: unscored_trust(),
        who: action_who(action),
        what: action.title.clone(),
        action_id: action.id.clone(),
        check_button_label: "Mark complete".to_string(),
    })
}

fn map_parked_row(action: &DbAction, snooze: &ActionSnoozeRecord) -> Option<WatchRowViewModel> {
    let reason = snooze.reason.trim();
    if reason.is_empty() {
        return None;
    }
    Some(WatchRowViewModel::Parked(WatchParkedRow {
        trust: unscored_trust(),
        who: action_who(action),
        what: action.title.clone(),
        parked_label: format!("Parked: {reason}"),
    }))
}

fn map_aging_row(action: &DbAction, today: NaiveDate) -> Option<WatchRowViewModel> {
    let (since, age_days) = aging_basis(action, today)?;
    Some(WatchRowViewModel::Aging(WatchAgingRow {
        trust: unscored_trust(),
        who: action_who(action),
        what: action.title.clone(),
        action_id: action.id.clone(),
        age_label: format_age_label(age_days),
        since: since.to_string(),
        options: vec![
            WatchAgingOption {
                id: WatchAgingOptionId::Restore,
                label: "Restore".to_string(),
            },
            WatchAgingOption {
                id: WatchAgingOptionId::Archive,
                label: "Archive".to_string(),
            },
        ],
    }))
}

fn map_suggested_action_row(candidate: &WatchSuggestedCandidate) -> WatchRowViewModel {
    WatchRowViewModel::SuggestedAction(WatchSuggestedActionRow {
        trust: claim_trust(&candidate.claim),
        lifecycle: LifecycleMixin {
            correction_state: Some(correction_state(&candidate.claim.verification_state)),
        },
        who: action_who(&candidate.action),
        what: candidate.action.title.clone(),
        action_id: candidate.action.id.clone(),
        selector: suggested_action_selector(),
    })
}

fn suggested_action_selector() -> InferredActionSelectorViewModel {
    InferredActionSelectorViewModel {
        trigger_label: "Choose action".to_string(),
        options: vec![
            InferredActionOption {
                id: "snooze".to_string(),
                label: "Snooze".to_string(),
                confidence: None,
                divider: None,
            },
            InferredActionOption {
                id: "dismiss".to_string(),
                label: "Dismiss".to_string(),
                confidence: None,
                divider: None,
            },
            InferredActionOption {
                id: "add_to_meeting".to_string(),
                label: "Add to meeting".to_string(),
                confidence: None,
                divider: None,
            },
        ],
        selected_option_id: "snooze".to_string(),
    }
}

fn unscored_trust() -> TrustMixin {
    TrustMixin {
        trust_band: TrustBandWire::Unscored,
        trust_field_path: None,
        trust_source_date: None,
        rendered_provenance: None,
    }
}

fn claim_trust(claim: &WatchSuggestedOutcomeClaim) -> TrustMixin {
    TrustMixin {
        trust_band: trust_band_wire(claim_trust_band_from_score(claim.trust_score)),
        trust_field_path: claim.field_path.clone(),
        trust_source_date: claim.trust_computed_at.clone().map(Some),
        rendered_provenance: rendered_provenance(&claim.provenance_json),
    }
}

fn rendered_provenance(raw: &str) -> Option<RenderedProvenanceSummary> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed)
        .ok()
        .map(|value| RenderedProvenanceSummary {
            surface: None,
            value,
        })
}

fn trust_band_wire(band: TrustBand) -> TrustBandWire {
    match band {
        TrustBand::LikelyCurrent => TrustBandWire::LikelyCurrent,
        TrustBand::UseWithCaution => TrustBandWire::UseWithCaution,
        TrustBand::NeedsVerification => TrustBandWire::NeedsVerification,
        TrustBand::Unscored => TrustBandWire::Unscored,
    }
}

fn correction_state(raw: &str) -> CorrectionState {
    match raw {
        "contested" | "needs_user_decision" => CorrectionState::Contested,
        _ => CorrectionState::None,
    }
}

fn action_who(action: &DbAction) -> String {
    action
        .account_name
        .clone()
        .or_else(|| action.account_id.clone())
        .or_else(|| action.source_label.clone())
        .unwrap_or_else(|| "—".to_string())
}

fn open_action_presses_today(action: &DbAction, today: NaiveDate) -> bool {
    if action.status == crate::action_status::STARTED {
        return true;
    }
    if action.status != crate::action_status::UNSTARTED {
        return false;
    }
    if action
        .due_date
        .as_deref()
        .and_then(parse_iso_date)
        .is_some_and(|due| due <= today)
    {
        return true;
    }
    action.priority <= crate::action_status::PRIORITY_HIGH
        && [action.created_at.as_str(), action.updated_at.as_str()]
            .into_iter()
            .filter_map(parse_iso_date)
            .any(|date| date == today)
}

fn aging_basis(action: &DbAction, today: NaiveDate) -> Option<(NaiveDate, i64)> {
    if action.status != crate::action_status::BACKLOG {
        return None;
    }
    let due_basis = action
        .due_date
        .as_deref()
        .and_then(parse_iso_date)
        .filter(|due| *due <= today);
    let basis = due_basis.or_else(|| parse_iso_date(&action.created_at))?;
    let age_days = today.signed_duration_since(basis).num_days();
    (age_days >= AGING_THRESHOLD_DAYS).then_some((basis, age_days))
}

fn format_age_label(age_days: i64) -> String {
    if age_days >= 30 {
        "30d+".to_string()
    } else {
        format!("{age_days}d")
    }
}

fn is_terminal_action(action: &DbAction) -> bool {
    crate::action_status::CLOSED_STATUSES.contains(&action.status.as_str())
}

fn meeting_is_today(claim: &WatchSuggestedOutcomeClaim, today: NaiveDate) -> bool {
    parse_iso_date(&claim.meeting_start) == Some(today)
}

fn suggested_action_title(text: &str) -> String {
    text.split_once(':')
        .map(|(title, _)| title)
        .unwrap_or(text)
        .trim()
        .to_string()
}

fn snooze_is_active(snooze: &ActionSnoozeRecord, now: DateTime<Utc>, today: NaiveDate) -> bool {
    let value = snooze.snoozed_until.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return dt.with_timezone(&Utc) > now;
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return date >= today;
    }
    parse_naive_datetime(value).is_some_and(|dt| dt > now.naive_utc())
}

fn parse_iso_date(value: &str) -> Option<NaiveDate> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return Some(date);
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc).date_naive());
    }
    parse_naive_datetime(value).map(|dt| dt.date())
}

fn parse_naive_datetime(value: &str) -> Option<NaiveDateTime> {
    ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"]
        .into_iter()
        .find_map(|fmt| NaiveDateTime::parse_from_str(value, fmt).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-05-07T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn empty_branch_fixture() -> WatchViewModel {
        WatchViewModel {
            label: "Watch".to_string(),
            heading: "Worth a look".to_string(),
            count_label: "0".to_string(),
            summary: "Nothing pressing today.".to_string(),
            rows: vec![],
        }
    }

    fn action(id: &str, status: &str) -> DbAction {
        DbAction {
            id: id.to_string(),
            title: format!("Action {id}"),
            priority: crate::action_status::PRIORITY_MEDIUM,
            status: status.to_string(),
            created_at: "2026-05-07T09:00:00Z".to_string(),
            due_date: Some("2026-05-07".to_string()),
            completed_at: None,
            account_id: Some("acct-1".to_string()),
            project_id: None,
            source_type: None,
            source_id: None,
            source_label: None,
            action_kind: crate::action_status::KIND_TASK.to_string(),
            context: None,
            waiting_on: None,
            updated_at: "2026-05-07T09:00:00Z".to_string(),
            person_id: None,
            account_name: Some("Globex".to_string()),
            next_meeting_title: None,
            next_meeting_start: None,
            needs_decision: false,
            decision_owner: None,
            decision_stakes: None,
            linear_identifier: None,
            linear_url: None,
        }
    }

    fn claim(meeting_start: &str) -> WatchSuggestedOutcomeClaim {
        WatchSuggestedOutcomeClaim {
            claim_id: "claim-1".to_string(),
            text: "Send rollout plan: customer asked for next steps".to_string(),
            field_path: Some("/suggested_outcomes/0".to_string()),
            provenance_json: r#"{"sources":[{"id":"src-1"}]}"#.to_string(),
            trust_score: Some(0.82),
            trust_computed_at: Some("2026-05-07T10:00:00Z".to_string()),
            verification_state: "active".to_string(),
            meeting_id: "meeting-1".to_string(),
            meeting_title: "Customer Sync".to_string(),
            meeting_start: meeting_start.to_string(),
        }
    }

    fn suggested_candidate(action_id: &str) -> WatchSuggestedCandidate {
        let mut action = action(action_id, crate::action_status::BACKLOG);
        action.title = "Send rollout plan".to_string();
        action.source_type = Some("intelligence_claim".to_string());
        action.source_id = Some("claim-1".to_string());
        WatchSuggestedCandidate {
            claim: claim("2026-05-07T14:00:00Z"),
            action,
        }
    }

    fn snooze(action_id: &str, reason: &str) -> ActionSnoozeRecord {
        ActionSnoozeRecord {
            action_id: action_id.to_string(),
            snoozed_until: "2026-05-08T12:00:00Z".to_string(),
            reason: reason.to_string(),
            source: "daily_briefing".to_string(),
        }
    }

    #[test]
    fn watch_empty_branch_zero_rows() {
        let vm = empty_branch_fixture();
        assert!(vm.rows.is_empty());
        assert_eq!(vm.heading, "Worth a look");
    }

    #[test]
    fn watch_serializes_to_camel_case_wire_shape() {
        let vm = empty_branch_fixture();
        let s = serde_json::to_string(&vm).expect("serialize");
        let parsed: Value = serde_json::from_str(&s).expect("parse");
        assert_eq!(parsed["heading"], "Worth a look");
        assert_eq!(parsed["countLabel"], "0");
        assert_eq!(parsed["rows"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn today_active_due_action_maps_to_open_action() {
        let rows = compose_watch_rows(
            WatchInputs {
                actions: vec![action("a1", crate::action_status::UNSTARTED)],
                ..WatchInputs::default()
            },
            now(),
            date("2026-05-07"),
        );
        match &rows[0] {
            WatchRowViewModel::OpenAction(open) => {
                assert_eq!(open.action_id, "a1");
                assert_eq!(open.who, "Globex");
                assert_eq!(open.check_button_label, "Mark complete");
            }
            _ => panic!("expected open action"),
        }
    }

    #[test]
    fn future_non_started_action_is_filtered_out() {
        let mut future = action("future", crate::action_status::UNSTARTED);
        future.due_date = Some("2026-05-08".to_string());
        future.created_at = "2026-05-06T09:00:00Z".to_string();
        future.updated_at = "2026-05-06T09:00:00Z".to_string();

        let rows = compose_watch_rows(
            WatchInputs {
                actions: vec![future],
                ..WatchInputs::default()
            },
            now(),
            date("2026-05-07"),
        );
        assert!(rows.is_empty());
    }

    #[test]
    fn high_priority_action_updated_today_is_pressing_without_due_date() {
        let mut action = action("p1", crate::action_status::UNSTARTED);
        action.due_date = None;
        action.priority = crate::action_status::PRIORITY_HIGH;
        action.created_at = "2026-05-06T23:00:00Z".to_string();
        action.updated_at = "2026-05-07T00:30:00-04:00".to_string();

        let rows = compose_watch_rows(
            WatchInputs {
                actions: vec![action],
                ..WatchInputs::default()
            },
            now(),
            date("2026-05-07"),
        );
        assert!(matches!(
            rows.first(),
            Some(WatchRowViewModel::OpenAction(_))
        ));
    }

    #[test]
    fn suggested_outcome_claim_maps_to_suggested_action_with_trust_and_provenance() {
        let candidate = suggested_candidate("suggested-1");
        let rows = compose_watch_rows(
            WatchInputs {
                actions: vec![candidate.action.clone()],
                suggested: vec![candidate],
                ..WatchInputs::default()
            },
            now(),
            date("2026-05-07"),
        );
        match &rows[0] {
            WatchRowViewModel::SuggestedAction(row) => {
                assert_eq!(row.action_id, "suggested-1");
                assert_eq!(row.what, "Send rollout plan");
                assert_eq!(row.trust.trust_band, TrustBandWire::LikelyCurrent);
                assert!(row.trust.rendered_provenance.is_some());
                assert_eq!(row.lifecycle.correction_state, Some(CorrectionState::None));
                let option_ids: Vec<_> =
                    row.selector.options.iter().map(|o| o.id.as_str()).collect();
                assert_eq!(option_ids, vec!["snooze", "dismiss", "add_to_meeting"]);
            }
            _ => panic!("expected suggested action"),
        }
    }

    #[test]
    fn non_today_suggested_outcome_claim_is_filtered_before_materialization() {
        assert!(meeting_is_today(
            &claim("2026-05-07T23:30:00Z"),
            date("2026-05-07")
        ));
        assert!(!meeting_is_today(
            &claim("2026-05-08T00:30:00Z"),
            date("2026-05-07")
        ));
    }

    #[test]
    fn backlog_ages_at_fourteen_days_not_thirteen() {
        let mut thirteen = action("fresh", crate::action_status::BACKLOG);
        thirteen.due_date = None;
        thirteen.created_at = "2026-04-24T09:00:00Z".to_string();

        let mut fourteen = action("aging", crate::action_status::BACKLOG);
        fourteen.due_date = None;
        fourteen.created_at = "2026-04-23T09:00:00Z".to_string();

        assert!(map_aging_row(&thirteen, date("2026-05-07")).is_none());
        match map_aging_row(&fourteen, date("2026-05-07")).expect("aging row") {
            WatchRowViewModel::Aging(row) => {
                assert_eq!(row.action_id, "aging");
                assert_eq!(row.age_label, "14d");
                assert_eq!(row.since, "2026-04-23");
                assert_eq!(row.options.len(), 2);
            }
            _ => panic!("expected aging row"),
        }
    }

    #[test]
    fn thirty_day_backlog_uses_compact_age_label() {
        let mut action = action("old", crate::action_status::BACKLOG);
        action.due_date = Some("2026-04-01".to_string());
        match map_aging_row(&action, date("2026-05-07")).expect("aging row") {
            WatchRowViewModel::Aging(row) => assert_eq!(row.age_label, "30d+"),
            _ => panic!("expected aging row"),
        }
    }

    #[test]
    fn active_snooze_with_reason_maps_to_parked_and_empty_reason_suppresses() {
        let action = action("a1", crate::action_status::UNSTARTED);
        let rows = compose_watch_rows(
            WatchInputs {
                actions: vec![action.clone()],
                snoozes: vec![snooze("a1", "Waiting on the customer")],
                ..WatchInputs::default()
            },
            now(),
            date("2026-05-07"),
        );
        match &rows[0] {
            WatchRowViewModel::Parked(row) => {
                assert_eq!(row.parked_label, "Parked: Waiting on the customer");
                assert_eq!(row.what, "Action a1");
            }
            _ => panic!("expected parked row"),
        }

        let rows = compose_watch_rows(
            WatchInputs {
                actions: vec![action],
                snoozes: vec![snooze("a1", "   ")],
                ..WatchInputs::default()
            },
            now(),
            date("2026-05-07"),
        );
        assert!(rows.is_empty());
    }

    #[test]
    fn terminal_statuses_never_emit_rows() {
        let rows = compose_watch_rows(
            WatchInputs {
                actions: vec![
                    action("done", crate::action_status::COMPLETED),
                    action("cancelled", crate::action_status::CANCELLED),
                    action("archived", crate::action_status::ARCHIVED),
                ],
                ..WatchInputs::default()
            },
            now(),
            date("2026-05-07"),
        );
        assert!(rows.is_empty());
    }

    #[test]
    fn row_precedence_prevents_duplicates_for_same_action_id() {
        let mut stale = action("same", crate::action_status::BACKLOG);
        stale.created_at = "2026-04-01T09:00:00Z".to_string();
        stale.due_date = None;
        let mut candidate = suggested_candidate("same");
        candidate.action = stale.clone();

        let rows = compose_watch_rows(
            WatchInputs {
                actions: vec![stale],
                suggested: vec![candidate],
                ..WatchInputs::default()
            },
            now(),
            date("2026-05-07"),
        );
        assert_eq!(rows.len(), 1);
        assert!(matches!(rows[0], WatchRowViewModel::Aging(_)));

        let candidate = suggested_candidate("parked");
        let rows = compose_watch_rows(
            WatchInputs {
                actions: vec![candidate.action.clone()],
                suggested: vec![candidate],
                snoozes: vec![snooze("parked", "Later this week")],
            },
            now(),
            date("2026-05-07"),
        );
        assert_eq!(rows.len(), 1);
        assert!(matches!(rows[0], WatchRowViewModel::Parked(_)));
    }

    #[test]
    fn summary_pluralizes_correctly() {
        assert_eq!(format_summary(0), "Nothing pressing today.");
        assert_eq!(format_summary(1), "1 item to triage.");
        assert_eq!(format_summary(7), "7 items to triage.");
    }
}

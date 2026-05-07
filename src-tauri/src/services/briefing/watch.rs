//! Watch composer — produces the `WatchViewModel` slice.
//!
//! **Trust source:** existing actions service (`services::actions::
//! get_all_actions`). Maps the returned `Action` list into `WatchRow`
//! variants via lightweight triage rules:
//!
//! - `is_overdue == true` OR active status (`Started` / `Unstarted`)
//!   → `WatchOpenActionRow` (the user has work to do today).
//! - `Backlog` status → `WatchAgingRow` (parked-but-old; user can
//!   restore or archive).
//! - everything else (cancelled, archived, completed) is filtered out.
//!
//! **W2a default:** if the upstream returns Empty/Error, falls back to
//! the empty-branch shape. Trust band is `Unscored` until DOS-411
//! claim-lifecycle wire-in promotes correctionState (DOS-428, W4).
//!
//! **Unblocked at:** DOS-415 fully fleshes the triage rules — adds
//! `WatchSuggestedActionRow` (claim-bearing suggestions from the
//! abilities runtime) and `WatchParkedRow` (snoozed-with-reason).
//! These need new mutation existence checks
//! (`actions::add_to_meeting`, snooze-with-reason).

use crate::services::actions::{get_all_actions, ActionsResult};
use crate::services::briefing_view_model::{
    TrustBandWire, TrustMixin, WatchAgingRow, WatchOpenActionRow,
    WatchRowViewModel, WatchViewModel,
};
use crate::state::AppState;
use crate::types::{Action, ActionStatus};

pub async fn compose_watch(state: &AppState) -> WatchViewModel {
    let actions: Vec<Action> = match get_all_actions(state).await {
        ActionsResult::Success { data } => data,
        _ => vec![],
    };

    let rows: Vec<WatchRowViewModel> = actions
        .into_iter()
        .filter_map(map_action_to_watch_row)
        .collect();
    let count = rows.len();

    WatchViewModel {
        label: "Watch".to_string(),
        heading: "Worth a look".to_string(),
        count_label: format!("{}", count),
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
        format!("{} items to triage.", count)
    }
}

fn map_action_to_watch_row(action: Action) -> Option<WatchRowViewModel> {
    let trust = TrustMixin {
        trust_band: TrustBandWire::Unscored,
        trust_field_path: None,
        trust_source_date: None,
        rendered_provenance: None,
    };
    let who = action.account.clone().unwrap_or_else(|| "—".to_string());

    match action.status {
        // Active work today: pick overdue + active actions.
        ActionStatus::Started | ActionStatus::Unstarted => {
            Some(WatchRowViewModel::OpenAction(WatchOpenActionRow {
                trust,
                who,
                what: action.title,
                action_id: action.id,
                check_button_label: "Mark complete".to_string(),
            }))
        }
        // Parked-but-old → aging row with restore/archive options.
        ActionStatus::Backlog => Some(WatchRowViewModel::Aging(WatchAgingRow {
            trust,
            who,
            what: action.title,
            action_id: action.id.clone(),
            age_label: action
                .days_overdue
                .map(|d| format!("{}d", d))
                .unwrap_or_default(),
            since: action.due_date.unwrap_or_default(),
            options: vec![
                crate::services::briefing_view_model::WatchAgingOption {
                    id: crate::services::briefing_view_model::WatchAgingOptionId::Restore,
                    label: "Restore".to_string(),
                },
                crate::services::briefing_view_model::WatchAgingOption {
                    id: crate::services::briefing_view_model::WatchAgingOptionId::Archive,
                    label: "Archive".to_string(),
                },
            ],
        })),
        // Filtered out: completed, cancelled, archived.
        ActionStatus::Completed | ActionStatus::Cancelled | ActionStatus::Archived => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn empty_branch_fixture() -> WatchViewModel {
        WatchViewModel {
            label: "Watch".to_string(),
            heading: "Worth a look".to_string(),
            count_label: "0".to_string(),
            summary: "Nothing pressing today.".to_string(),
            rows: vec![],
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

    fn make_action(id: &str, status: ActionStatus) -> Action {
        Action {
            id: id.to_string(),
            title: format!("Action {}", id),
            account: Some("Globex".to_string()),
            due_date: Some("2026-04-22".to_string()),
            priority: crate::types::Priority::Medium,
            status,
            is_overdue: Some(false),
            context: None,
            source: None,
            days_overdue: None,
        }
    }

    #[test]
    fn started_action_maps_to_open_action_row() {
        let action = make_action("a1", ActionStatus::Started);
        let row = map_action_to_watch_row(action).expect("should map");
        match row {
            WatchRowViewModel::OpenAction(open) => {
                assert_eq!(open.action_id, "a1");
                assert_eq!(open.who, "Globex");
                assert_eq!(open.check_button_label, "Mark complete");
            }
            _ => panic!("expected OpenAction"),
        }
    }

    #[test]
    fn unstarted_action_also_maps_to_open_action_row() {
        let action = make_action("a2", ActionStatus::Unstarted);
        assert!(matches!(
            map_action_to_watch_row(action),
            Some(WatchRowViewModel::OpenAction(_))
        ));
    }

    #[test]
    fn backlog_action_maps_to_aging_row_with_restore_archive_options() {
        let mut action = make_action("a3", ActionStatus::Backlog);
        action.days_overdue = Some(14);
        let row = map_action_to_watch_row(action).expect("should map");
        match row {
            WatchRowViewModel::Aging(aging) => {
                assert_eq!(aging.action_id, "a3");
                assert_eq!(aging.age_label, "14d");
                assert_eq!(aging.options.len(), 2);
                assert!(aging
                    .options
                    .iter()
                    .any(|o| matches!(
                        o.id,
                        crate::services::briefing_view_model::WatchAgingOptionId::Restore
                    )));
                assert!(aging
                    .options
                    .iter()
                    .any(|o| matches!(
                        o.id,
                        crate::services::briefing_view_model::WatchAgingOptionId::Archive
                    )));
            }
            _ => panic!("expected Aging"),
        }
    }

    #[test]
    fn terminal_statuses_filter_out_of_watch() {
        for status in [
            ActionStatus::Completed,
            ActionStatus::Cancelled,
            ActionStatus::Archived,
        ] {
            let action = make_action("a", status.clone());
            assert!(
                map_action_to_watch_row(action).is_none(),
                "{:?} should filter out",
                status
            );
        }
    }

    #[test]
    fn missing_account_renders_em_dash_placeholder() {
        let mut action = make_action("a", ActionStatus::Started);
        action.account = None;
        let row = map_action_to_watch_row(action).expect("should map");
        match row {
            WatchRowViewModel::OpenAction(open) => assert_eq!(open.who, "—"),
            _ => panic!("expected OpenAction"),
        }
    }

    #[test]
    fn summary_pluralizes_correctly() {
        assert_eq!(format_summary(0), "Nothing pressing today.");
        assert_eq!(format_summary(1), "1 item to triage.");
        assert_eq!(format_summary(7), "7 items to triage.");
    }
}

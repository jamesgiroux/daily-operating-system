//! Watch composer — produces the `WatchViewModel` slice.
//!
//! **Trust source:** action triage. Pulls from `services::actions` (open
//! actions, suggested actions, parked, aging) and applies the briefing's
//! rules: TODAY-relevance, claim-bearing for suggested rows, age
//! threshold for aging.
//!
//! **W2a default:** empty rows list. Editorial copy renders ("Worth a
//! look" / "0" / "Nothing pressing today."). Real triage is DOS-415
//! follow-up; that ticket's L0 plan declares the rule set (TODAY filter,
//! aging threshold, suggested-action source) and any new mutations the
//! Watch row variants depend on (`actions::add_to_meeting` is the
//! mutation most likely to need creation).
//!
//! **Unblocked at:** DOS-415 (Watch triage impl). The mutation-existence
//! verification in DOS-415's L0 plan must confirm or create the four
//! mutations Watch variants emit: `actions::snooze`, `actions::dismiss`,
//! `actions::add_to_meeting`, `actions::mark_complete`,
//! `actions::restore`, `actions::archive`.

use crate::services::briefing_view_model::WatchViewModel;
use crate::state::AppState;

pub async fn compose_watch(_state: &AppState) -> WatchViewModel {
    WatchViewModel {
        label: "Watch".to_string(),
        heading: "Worth a look".to_string(),
        count_label: "0".to_string(),
        summary: "Nothing pressing today.".to_string(),
        rows: vec![],
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
}

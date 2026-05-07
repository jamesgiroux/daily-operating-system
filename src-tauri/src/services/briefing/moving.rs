//! Moving composer — produces the `MovingViewModel` slice.
//!
//! **Trust source:** the heaviest in W2a. Aggregates from email
//! (`services::emails`), Gong calls, Zendesk tickets, Slack threads,
//! Linear issues (via Glean), meetings (`services::dashboard`), and
//! lifecycle changes (DOS-419 layered adapter, W2b). Each contributes
//! signals that the composer ranks by 24h change-magnitude and groups
//! per entity.
//!
//! **W2a default:** empty entities list. Editorial copy renders
//! ("What's moving" / "0 entities" / "Quiet."). Real aggregation is
//! DOS-414 follow-up; that ticket's L0 plan must declare which
//! upstream sources are wired today and which require additional
//! producer tickets.
//!
//! **Unblocked at:** DOS-414 (Moving aggregation impl). Lifecycle
//! signals additionally need DOS-419 (W2b lifecycle adapter).

use crate::services::briefing_view_model::MovingViewModel;
use crate::state::AppState;

pub async fn compose_moving(_state: &AppState) -> MovingViewModel {
    MovingViewModel {
        label: "Moving".to_string(),
        heading: "What's moving".to_string(),
        count_label: "0 entities".to_string(),
        summary: "Quiet.".to_string(),
        entities: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn empty_branch_fixture() -> MovingViewModel {
        MovingViewModel {
            label: "Moving".to_string(),
            heading: "What's moving".to_string(),
            count_label: "0 entities".to_string(),
            summary: "Quiet.".to_string(),
            entities: vec![],
        }
    }

    #[test]
    fn moving_empty_branch_renders_editorial_copy() {
        let vm = empty_branch_fixture();
        assert!(vm.entities.is_empty());
        assert_eq!(vm.heading, "What's moving");
        assert_eq!(vm.count_label, "0 entities");
    }

    #[test]
    fn moving_serializes_to_camel_case_wire_shape() {
        let vm = empty_branch_fixture();
        let s = serde_json::to_string(&vm).expect("serialize");
        let parsed: Value = serde_json::from_str(&s).expect("parse");
        assert_eq!(parsed["heading"], "What's moving");
        assert_eq!(parsed["countLabel"], "0 entities");
        assert_eq!(parsed["entities"].as_array().unwrap().len(), 0);
    }
}

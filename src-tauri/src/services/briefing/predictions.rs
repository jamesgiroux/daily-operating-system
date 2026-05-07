//! Predictions composer — produces the `PredictionsViewModel` slice of the
//! Daily Briefing.
//!
//! **Trust source:** abilities-runtime prediction outputs. DOS-218 emits
//! prediction invocations during meeting prep; DOS-219 reconciles
//! post-meeting. The briefing's Predictions section needs a *forward-looking*
//! feed of "today's predictions across all entities" — that producer is not
//! yet wired.
//!
//! **W2a default:** empty list. Returns `count: 0` with editorial copy. Trust
//! band per item is `Unscored` (n/a while empty).
//!
//! **Unblocked at:** DOS-431 (canonical cutover) or earlier if a forward-
//! feed producer ships. The empty branch is intentional and tracked — not a
//! placeholder to overlook at W4 wire-in.

use crate::services::briefing_view_model::PredictionsViewModel;
use crate::state::AppState;

/// Compose the Predictions slice. `async` to match the orchestrator
/// signature; no I/O today, returns the empty-list shape until upstream
/// wires in.
pub async fn compose_predictions(_state: &AppState) -> PredictionsViewModel {
    PredictionsViewModel {
        label: "Predictions".to_string(),
        count_label: "0 today".to_string(),
        collapsed_label: "0 predictions today".to_string(),
        expand_hint: "expand".to_string(),
        count: 0,
        predictions: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// Reproduces `compose_predictions`'s empty-branch return value without
    /// constructing a real `AppState` (which triggers heavy I/O — config
    /// load, audit log, DB open). The wire-shape test below verifies serde
    /// behavior; the orchestrator integration test (W2b) exercises the
    /// composer with live state.
    fn empty_branch_fixture() -> PredictionsViewModel {
        PredictionsViewModel {
            label: "Predictions".to_string(),
            count_label: "0 today".to_string(),
            collapsed_label: "0 predictions today".to_string(),
            expand_hint: "expand".to_string(),
            count: 0,
            predictions: vec![],
        }
    }

    #[test]
    fn empty_branch_produces_zero_count_with_editorial_copy() {
        let vm = empty_branch_fixture();
        assert_eq!(vm.count, 0);
        assert!(vm.predictions.is_empty());
        assert_eq!(vm.label, "Predictions");
        assert_eq!(vm.count_label, "0 today");
        assert_eq!(vm.collapsed_label, "0 predictions today");
        assert_eq!(vm.expand_hint, "expand");
    }

    #[test]
    fn empty_branch_serializes_to_camel_case_wire_shape() {
        let vm = empty_branch_fixture();
        let s = serde_json::to_string(&vm).expect("serialize");
        let parsed: Value = serde_json::from_str(&s).expect("parse");
        assert_eq!(parsed["count"], 0);
        assert_eq!(parsed["countLabel"], "0 today");
        assert_eq!(parsed["collapsedLabel"], "0 predictions today");
        assert_eq!(parsed["expandHint"], "expand");
        assert_eq!(parsed["predictions"].as_array().unwrap().len(), 0);
    }
}

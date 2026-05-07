//! Lead composer — produces the `LeadViewModel` slice.
//!
//! **Trust source:** the lead headline is editorial — composed by the
//! orchestrator from the day's primary signal (top Moving entity, calendar
//! density, lifecycle pressure). Today there is no producer that emits a
//! "lead headline" claim; the headline is rendered editorial copy.
//!
//! **W2a default:** static editorial copy. Composer returns a generic
//! "today" headline + capacity sentence. No `TrustMixin` on the slice
//! itself — `LeadViewModel` is plain editorial text.
//!
//! **Unblocked at:** DOS-426 Lead-content-source ticket if/when the
//! editorial copy needs to be derived from claims (e.g., naming the top
//! moving entity in the headline). Not on the v1.4.x critical path.

use crate::services::briefing_view_model::{LeadHeadline, LeadViewModel};
use crate::state::AppState;

pub async fn compose_lead(_state: &AppState) -> LeadViewModel {
    LeadViewModel {
        headline: LeadHeadline {
            lead: "Today is yours to shape.".to_string(),
            punch_line: None,
        },
        focus_capacity: "Connect your sources to bring your day into focus.".to_string(),
        focus_block: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn empty_branch_fixture() -> LeadViewModel {
        LeadViewModel {
            headline: LeadHeadline {
                lead: "Today is yours to shape.".to_string(),
                punch_line: None,
            },
            focus_capacity: "Connect your sources to bring your day into focus.".to_string(),
            focus_block: None,
        }
    }

    #[test]
    fn lead_serializes_to_camel_case_wire_shape() {
        let vm = empty_branch_fixture();
        let s = serde_json::to_string(&vm).expect("serialize");
        let parsed: Value = serde_json::from_str(&s).expect("parse");
        assert_eq!(parsed["headline"]["lead"], "Today is yours to shape.");
        assert!(parsed["headline"].get("punchLine").is_none());
        assert_eq!(
            parsed["focusCapacity"],
            "Connect your sources to bring your day into focus."
        );
        assert!(parsed.get("focusBlock").is_none());
    }
}

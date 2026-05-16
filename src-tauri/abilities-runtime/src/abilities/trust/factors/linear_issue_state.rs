use super::super::config::{
    FACTOR_MAX, FACTOR_MIN, LINEAR_KNOWN_ATTRIBUTE_CHANGE_WEIGHT, LINEAR_SUBJECT_MISMATCH_WEIGHT,
    LINEAR_UNCATEGORIZED_ISSUE_WEIGHT,
};
use super::super::types::{LinearIssueStateSignal, TrustFactorInputs};

const KNOWN_STATE_CHANGE_WEIGHT: f64 = FACTOR_MAX;

pub fn linear_issue_state_weight(input: &TrustFactorInputs) -> f64 {
    if !input.linear_issue_state.subject_matches {
        return LINEAR_SUBJECT_MISMATCH_WEIGHT;
    }

    match input.linear_issue_state.signal {
        LinearIssueStateSignal::None => FACTOR_MAX,
        LinearIssueStateSignal::StateChangedToInProgress
        | LinearIssueStateSignal::StateChangedToBlocked
        | LinearIssueStateSignal::StateChangedToDone
        | LinearIssueStateSignal::PriorityChangedToUrgent => KNOWN_STATE_CHANGE_WEIGHT,
        LinearIssueStateSignal::AssigneeChanged => LINEAR_KNOWN_ATTRIBUTE_CHANGE_WEIGHT,
        LinearIssueStateSignal::UncategorizedIssue => LINEAR_UNCATEGORIZED_ISSUE_WEIGHT,
    }
    .clamp(FACTOR_MIN, FACTOR_MAX)
}

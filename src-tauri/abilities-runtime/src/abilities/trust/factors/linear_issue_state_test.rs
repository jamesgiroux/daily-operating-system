use super::super::config::LINEAR_SUBJECT_MISMATCH_WEIGHT;
use super::super::types::{
    FreshnessContext, LinearIssueStateContext, LinearIssueStateSignal, SourceLifecycleState,
    TrustFactorInputs, UserFeedbackSignal,
};
use super::linear_issue_state::linear_issue_state_weight;

fn inputs(signal: LinearIssueStateSignal) -> TrustFactorInputs {
    TrustFactorInputs {
        source_reliability: 1.0,
        source_reliability_corroborators: Vec::new(),
        freshness: FreshnessContext {
            timestamp_known: true,
            age_days: 0.0,
        },
        corroboration_strength: 1.0,
        contradiction_count: 0,
        user_feedback: UserFeedbackSignal::None,
        subject_fit_confidence: 1.0,
        internal_consistency: 1.0,
        source_lifecycle: SourceLifecycleState::Active,
        linear_issue_state: LinearIssueStateContext {
            signal,
            subject_matches: true,
        },
        read_state_indeterminate: false,
    }
}

#[test]
fn known_state_change_outweighs_uncategorized_issue() {
    let known = inputs(LinearIssueStateSignal::StateChangedToBlocked);
    let uncategorized = inputs(LinearIssueStateSignal::UncategorizedIssue);

    assert!(linear_issue_state_weight(&known) > linear_issue_state_weight(&uncategorized));
}

#[test]
fn subject_mismatch_downweights_issue_state() {
    let mut input = inputs(LinearIssueStateSignal::StateChangedToDone);
    input.linear_issue_state.subject_matches = false;

    assert_eq!(
        linear_issue_state_weight(&input),
        LINEAR_SUBJECT_MISMATCH_WEIGHT
    );
}

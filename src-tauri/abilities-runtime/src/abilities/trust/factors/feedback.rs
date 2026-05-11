use super::super::config::{TrustConfig, FACTOR_MAX};
use super::super::types::{TrustFactorInputs, UserFeedbackSignal};

pub fn user_feedback_weight(input: &TrustFactorInputs, config: &TrustConfig) -> f64 {
    match input.user_feedback {
        UserFeedbackSignal::None => FACTOR_MAX,
        UserFeedbackSignal::Confirmed => config.feedback_boost,
        UserFeedbackSignal::Corrected => FACTOR_MAX - config.feedback_penalty,
        UserFeedbackSignal::Retracted | UserFeedbackSignal::WrongSubject => config.feedback_penalty,
    }
}

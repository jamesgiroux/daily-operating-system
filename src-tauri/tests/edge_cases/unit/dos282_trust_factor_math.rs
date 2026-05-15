use dailyos_lib::abilities::trust::{FreshnessContext, SourceLifecycleState, TrustBand, UserFeedbackSignal};

use crate::support::{
    baseline_trust_inputs, band_of, compile_test_trust, corroborator, score_of, trust_factor_value,
};

#[test]
fn trust_compiler_applies_freshness_reliability_corroboration_contradiction_and_feedback() {
    let baseline = score_of(baseline_trust_inputs());

    let mut stale = baseline_trust_inputs();
    stale.freshness = FreshnessContext {
        timestamp_known: true,
        age_days: 365.0,
    };
    assert!(score_of(stale) < baseline);

    let mut low_reliability = baseline_trust_inputs();
    low_reliability.source_reliability = 0.25;
    assert!(score_of(low_reliability) < baseline);

    let mut corroborated = baseline_trust_inputs();
    corroborated.corroboration_strength = 0.5;
    let low_corroboration = score_of(corroborated.clone());
    corroborated.corroboration_strength = 1.0;
    assert!(score_of(corroborated) > low_corroboration);

    let mut contradicted = baseline_trust_inputs();
    contradicted.contradiction_count = 2;
    contradicted.source_reliability_corroborators = vec![corroborator(0.9, false)];
    assert_eq!(band_of(contradicted), TrustBand::NeedsVerification);

    let mut corrected = baseline_trust_inputs();
    corrected.user_feedback = UserFeedbackSignal::Corrected;
    assert!(score_of(corrected) < baseline);

    let mut withdrawn = baseline_trust_inputs();
    withdrawn.source_lifecycle = SourceLifecycleState::Withdrawn;
    let computation = compile_test_trust(withdrawn);
    assert_eq!(computation.band, TrustBand::NeedsVerification);
    assert_eq!(
        trust_factor_value(&computation, "source_lifecycle_weight"),
        0.0
    );
}

use std::collections::HashSet;

use crate::db::claims::{ClaimSensitivity, TemporalScope};

use super::config::TrustConfig;
use super::types::{
    CorroboratorWeight, CrossEntityCoherenceInput, CrossEntityHit, CrossEntityHitKind,
    FreshnessContext, SourceLifecycleState, SourceReliabilityInput, SurfaceClass,
    TrustFactorInputs, UserFeedbackSignal,
};

const LN_2: f64 = std::f64::consts::LN_2;

const STOPLIST: &[&str] = &[
    "open",
    "pilot",
    "plan",
    "monday",
    "notion",
    "mercury",
    "ramp",
    "handshake",
    "bridge",
    "flow",
    "base",
    "peak",
    "note",
    "space",
    "link",
    "next",
    "sync",
    "ready",
    "clear",
    "front",
    "core",
    "post",
    "meet",
    "call",
    "talk",
    "chat",
    "dash",
    "pulse",
    "track",
    "task",
    "work",
    "team",
    "loop",
    "zoom",
    "slack",
    "linear",
    "email",
    "meeting",
    "customer",
    "account",
    "company",
    "product",
    "group",
];

#[derive(Debug, Clone, PartialEq)]
pub struct CrossEntityCoherenceResult {
    pub value: f64,
    pub hits: Vec<CrossEntityHit>,
    pub skipped_expected_context: bool,
}

pub fn source_reliability(input: &TrustFactorInputs) -> f64 {
    if input.source_reliability_corroborators.is_empty() {
        input.source_reliability
    } else {
        source_reliability_from_corroborators(&input.source_reliability_corroborators)
    }
}

pub fn source_reliability_aggregated(input: &SourceReliabilityInput) -> f64 {
    source_reliability_from_corroborators(&input.corroborators)
}

fn source_reliability_from_corroborators(corroborators: &[CorroboratorWeight]) -> f64 {
    if corroborators.is_empty() {
        return 0.0;
    }

    let confirm_sum: f64 = corroborators
        .iter()
        .filter(|corroborator| corroborator.confirms)
        .map(|corroborator| corroborator.evidence_weight)
        .sum();
    let contradict_sum: f64 = corroborators
        .iter()
        .filter(|corroborator| !corroborator.confirms)
        .map(|corroborator| corroborator.evidence_weight)
        .sum();
    let net = (confirm_sum - contradict_sum) / (confirm_sum + contradict_sum).max(1.0);
    ((net + 1.0) / 2.0).clamp(0.0, 1.0)
}

pub fn source_lifecycle_weight(input: &TrustFactorInputs) -> f64 {
    match input.source_lifecycle {
        SourceLifecycleState::Active => 1.0,
        SourceLifecycleState::Withdrawn | SourceLifecycleState::Dismissed => 0.0,
    }
}

pub fn freshness_weight(
    ctx: &FreshnessContext,
    temporal_scope: &TemporalScope,
    config: &TrustConfig,
) -> f64 {
    let base = match temporal_scope {
        // PointInTime is intrinsically fixed at the observation instant. Closed
        // is a historical claim whose observation window has ended; freshness is
        // fixed at that window end so later observed data should not decay or
        // refresh the claim through this factor.
        TemporalScope::PointInTime | TemporalScope::Closed => 1.0,
        TemporalScope::State | TemporalScope::Trend => {
            let age_days = ctx.age_days.max(0.0);
            (-LN_2 * age_days / config.freshness_half_life_days).exp()
        }
    };

    if ctx.timestamp_known {
        base
    } else {
        base * config.unknown_timestamp_penalty
    }
}

pub fn corroboration_weight(input: &TrustFactorInputs) -> f64 {
    input.corroboration_strength
}

pub fn contradiction_penalty(input: &TrustFactorInputs, config: &TrustConfig) -> f64 {
    if input.contradiction_count == 0 {
        return 1.0;
    }

    (1.0 - config.contradiction_multiplier).powi(input.contradiction_count as i32)
}

pub fn user_feedback_weight(input: &TrustFactorInputs, config: &TrustConfig) -> f64 {
    match input.user_feedback {
        UserFeedbackSignal::None => 1.0,
        UserFeedbackSignal::Confirmed => config.feedback_boost,
        UserFeedbackSignal::Corrected => 1.0 - config.feedback_penalty,
        UserFeedbackSignal::Retracted | UserFeedbackSignal::WrongSubject => config.feedback_penalty,
    }
}

pub fn subject_fit_confidence(input: &TrustFactorInputs) -> f64 {
    input.subject_fit_confidence
}

pub fn internal_consistency(input: &TrustFactorInputs) -> f64 {
    input.internal_consistency
}

pub fn cross_entity_coherence(
    input: &CrossEntityCoherenceInput,
    config: &TrustConfig,
) -> CrossEntityCoherenceResult {
    if input.cross_entity_context_expected {
        return CrossEntityCoherenceResult {
            value: 1.0,
            hits: Vec::new(),
            skipped_expected_context: true,
        };
    }

    let hits = cross_entity_hits(input);
    let value = if hits.is_empty() {
        1.0
    } else {
        (1.0 - config.cross_entity_hit_penalty).powi(hits.len() as i32)
    };

    CrossEntityCoherenceResult {
        value,
        hits,
        skipped_expected_context: false,
    }
}

/// Returns the sensitivity-aware-filtering factor as a SOFT trust signal.
///
/// 1.0 when the claim's sensitivity is permitted on the target surface,
/// 0.0 when not. 1.0 when target_surface is None (no surface specified —
/// internal/eval contexts that don't render).
///
/// # Soft signal contract
///
/// On a violation (e.g. a Confidential claim on a Public surface), this
/// returns 0.0 raw, which the geometric-mean aggregator clamps to the
/// configured floor (default 0.05) before contributing to the final trust
/// score. Combined with the other 6 factors the resulting score lands in
/// the NeedsVerification band or below — the score signals "do not surface
/// this content under the current policy" without producing a hard reject
/// at the trust-math layer.
///
/// The rendering layer is the policy enforcer: any surface that consumes
/// trust-scored claims MUST suppress claims at NeedsVerification or below
/// before rendering. This is the consumer-policy contract. Rationale: keeps
/// the trust math composable (sensitivity is one factor among 7), preserves
/// auditability (the factor breakdown shows why the score dropped), and
/// lets a single rendering policy knob cover multiple suppression scenarios
/// (NeedsVerification from low corroboration, sensitivity violation,
/// contradiction, etc.).
///
/// If a future surface needs hard-reject semantics (e.g. a compliance
/// export flow that must NEVER include Confidential content), that surface
/// should run sensitivity_aware_filtering at its own boundary BEFORE
/// invoking the trust compiler — the compiler is not the gate.
pub fn sensitivity_aware_filtering(
    claim_sensitivity: &ClaimSensitivity,
    target_surface: Option<SurfaceClass>,
) -> f64 {
    match target_surface {
        None | Some(SurfaceClass::UserOnly) => 1.0,
        Some(SurfaceClass::Confidential) => match claim_sensitivity {
            ClaimSensitivity::Public
            | ClaimSensitivity::Internal
            | ClaimSensitivity::Confidential => 1.0,
            ClaimSensitivity::UserOnly => 0.0,
        },
        Some(SurfaceClass::Internal) => match claim_sensitivity {
            ClaimSensitivity::Public | ClaimSensitivity::Internal => 1.0,
            ClaimSensitivity::Confidential | ClaimSensitivity::UserOnly => 0.0,
        },
        Some(SurfaceClass::Public) => match claim_sensitivity {
            ClaimSensitivity::Public => 1.0,
            ClaimSensitivity::Internal
            | ClaimSensitivity::Confidential
            | ClaimSensitivity::UserOnly => 0.0,
        },
    }
}

fn cross_entity_hits(input: &CrossEntityCoherenceInput) -> Vec<CrossEntityHit> {
    if input.claim_text.trim().is_empty() {
        return Vec::new();
    }

    let text_lower = input.claim_text.to_lowercase();
    let target_domains: Vec<String> = input
        .target_footprint
        .domains
        .iter()
        .map(|domain| domain.to_lowercase())
        .collect();
    let target_names: Vec<String> = input
        .target_footprint
        .names
        .iter()
        .chain(input.target_footprint.allowed_aliases.iter())
        .map(|name| name.to_lowercase())
        .collect();
    let target_name_in_text = target_names
        .iter()
        .any(|name| whole_word_contains(&text_lower, name));

    let mut hits = Vec::new();
    let mut seen_tokens = HashSet::<String>::new();

    for footprint in input
        .portfolio_footprints
        .iter()
        .filter(|footprint| !is_target_or_related(input, &footprint.subject))
    {
        for domain in &footprint.domains {
            let token = domain.to_lowercase();
            if token.is_empty() || is_target_owned(&token, &target_domains) {
                continue;
            }
            if !seen_tokens.insert(token.clone()) {
                continue;
            }
            if whole_word_contains(&text_lower, &token) {
                hits.push(CrossEntityHit {
                    token,
                    kind: CrossEntityHitKind::Domain,
                    source_subject: Some(footprint.subject.clone()),
                });
            }
        }
    }

    for token in extract_vip_hosts(&text_lower) {
        if is_target_owned(&token, &target_domains) || !seen_tokens.insert(token.clone()) {
            continue;
        }
        let source_subject = input
            .portfolio_footprints
            .iter()
            .find(|footprint| {
                !is_target_or_related(input, &footprint.subject)
                    && footprint
                        .domains
                        .iter()
                        .any(|domain| domain.eq_ignore_ascii_case(&token))
            })
            .map(|footprint| footprint.subject.clone());
        hits.push(CrossEntityHit {
            token,
            kind: CrossEntityHitKind::InfrastructureId,
            source_subject,
        });
    }

    if !target_name_in_text {
        for footprint in input
            .portfolio_footprints
            .iter()
            .filter(|footprint| !is_target_or_related(input, &footprint.subject))
        {
            for name in &footprint.names {
                let token = name.to_lowercase();
                if token.len() < 4 || STOPLIST.contains(&token.as_str()) {
                    continue;
                }
                if !seen_tokens.insert(token.clone()) {
                    continue;
                }
                if whole_word_contains(&text_lower, &token) {
                    hits.push(CrossEntityHit {
                        token,
                        kind: CrossEntityHitKind::CompanyName,
                        source_subject: Some(footprint.subject.clone()),
                    });
                }
            }
        }
    }

    hits
}

fn is_target_or_related(
    input: &CrossEntityCoherenceInput,
    subject: &crate::abilities::provenance::SubjectRef,
) -> bool {
    &input.target_footprint.subject == subject
        || input
            .target_footprint
            .related_subjects
            .iter()
            .any(|related| related == subject)
}

fn is_target_owned(token: &str, target_domains: &[String]) -> bool {
    target_domains.iter().any(|target_domain| {
        !target_domain.is_empty()
            && (token == target_domain || token.ends_with(&format!(".{target_domain}")))
    })
}

fn whole_word_contains(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }

    let haystack = haystack.as_bytes();
    let needle = needle.as_bytes();
    let mut i = 0;
    while i + needle.len() <= haystack.len() {
        if &haystack[i..i + needle.len()] == needle {
            let left_ok = i == 0 || !haystack[i - 1].is_ascii_alphanumeric();
            let right_idx = i + needle.len();
            let right_ok =
                right_idx == haystack.len() || !haystack[right_idx].is_ascii_alphanumeric();
            if left_ok && right_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn extract_vip_hosts(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        if &bytes[i..i + 3] == b"vip" {
            if i > 0 && bytes[i - 1].is_ascii_alphanumeric() {
                i += 1;
                continue;
            }

            let mut j = i + 3;
            while j < bytes.len() {
                let c = bytes[j];
                if c.is_ascii_alphanumeric() || c == b'-' || c == b'.' {
                    j += 1;
                } else {
                    break;
                }
            }

            let candidate = &text[i..j];
            let dashed_vip_host =
                candidate.ends_with(".com") && candidate.len() > 4 && candidate.contains('-');
            let dotted_vip_host =
                candidate.ends_with(".com") && candidate.matches('.').count() >= 2;
            if dashed_vip_host || dotted_vip_host {
                out.push(candidate.trim_end_matches('.').to_string());
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::provenance::SubjectRef;
    use crate::abilities::trust::{EntityFootprint, TargetFootprint};

    fn target_footprint() -> TargetFootprint {
        TargetFootprint {
            subject: SubjectRef::Account("acct-target".to_string()),
            names: vec!["Target Account".to_string()],
            domains: vec!["target.example.com".to_string()],
            related_subjects: Vec::new(),
            allowed_aliases: vec!["TargetCo".to_string()],
        }
    }

    fn other_footprint() -> EntityFootprint {
        EntityFootprint {
            subject: SubjectRef::Account("acct-other".to_string()),
            names: vec!["Other Company".to_string()],
            domains: vec!["globex.example".to_string()],
            infrastructure_ids: Vec::new(),
        }
    }

    fn coherence_input(
        claim_text: &str,
        cross_entity_context_expected: bool,
    ) -> CrossEntityCoherenceInput {
        CrossEntityCoherenceInput {
            claim_text: claim_text.to_string(),
            target_footprint: target_footprint(),
            portfolio_footprints: vec![other_footprint()],
            cross_entity_context_expected,
        }
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn sensitivity_factor_passes_when_target_surface_is_none() {
        assert_close(
            sensitivity_aware_filtering(&ClaimSensitivity::UserOnly, None),
            1.0,
        );
    }

    #[test]
    fn sensitivity_factor_passes_for_public_claim_on_public_surface() {
        assert_close(
            sensitivity_aware_filtering(&ClaimSensitivity::Public, Some(SurfaceClass::Public)),
            1.0,
        );
    }

    #[test]
    fn sensitivity_factor_blocks_internal_claim_on_public_surface() {
        assert_close(
            sensitivity_aware_filtering(&ClaimSensitivity::Internal, Some(SurfaceClass::Public)),
            0.0,
        );
    }

    #[test]
    fn sensitivity_factor_blocks_confidential_claim_on_public_surface() {
        assert_close(
            sensitivity_aware_filtering(
                &ClaimSensitivity::Confidential,
                Some(SurfaceClass::Public),
            ),
            0.0,
        );
    }

    #[test]
    fn sensitivity_factor_blocks_user_only_claim_on_public_surface() {
        assert_close(
            sensitivity_aware_filtering(&ClaimSensitivity::UserOnly, Some(SurfaceClass::Public)),
            0.0,
        );
    }

    #[test]
    fn sensitivity_factor_passes_for_internal_claim_on_internal_surface() {
        assert_close(
            sensitivity_aware_filtering(&ClaimSensitivity::Internal, Some(SurfaceClass::Internal)),
            1.0,
        );
    }

    #[test]
    fn sensitivity_factor_blocks_confidential_claim_on_internal_surface() {
        assert_close(
            sensitivity_aware_filtering(
                &ClaimSensitivity::Confidential,
                Some(SurfaceClass::Internal),
            ),
            0.0,
        );
    }

    #[test]
    fn sensitivity_factor_passes_for_confidential_claim_on_confidential_surface() {
        assert_close(
            sensitivity_aware_filtering(
                &ClaimSensitivity::Confidential,
                Some(SurfaceClass::Confidential),
            ),
            1.0,
        );
    }

    #[test]
    fn sensitivity_factor_blocks_user_only_claim_on_confidential_surface() {
        assert_close(
            sensitivity_aware_filtering(
                &ClaimSensitivity::UserOnly,
                Some(SurfaceClass::Confidential),
            ),
            0.0,
        );
    }

    #[test]
    fn sensitivity_factor_passes_user_only_claim_on_user_only_surface() {
        assert_close(
            sensitivity_aware_filtering(&ClaimSensitivity::UserOnly, Some(SurfaceClass::UserOnly)),
            1.0,
        );
    }

    #[test]
    fn cross_entity_coherence_clean_claim_scores_one() {
        let result = cross_entity_coherence(
            &coherence_input("Target Account renewal risk is stable.", false),
            &TrustConfig::default(),
        );

        assert_close(result.value, 1.0);
        assert!(result.hits.is_empty());
        assert!(!result.skipped_expected_context);
    }

    #[test]
    fn cross_entity_coherence_foreign_domain_scores_low_without_rejecting() {
        let config = TrustConfig::default();
        let result = cross_entity_coherence(
            &coherence_input(
                "The renewal note accidentally references globex.example.",
                false,
            ),
            &config,
        );

        assert_close(result.value, 1.0 - config.cross_entity_hit_penalty);
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].kind, CrossEntityHitKind::Domain);
        assert_eq!(result.hits[0].token, "globex.example");
        assert!(!result.skipped_expected_context);
    }

    #[test]
    fn cross_entity_coherence_foreign_vip_host_scores_low() {
        let config = TrustConfig::default();
        let result = cross_entity_coherence(
            &coherence_input(
                "The support note points at vip-foreign.com for the renewal.",
                false,
            ),
            &config,
        );

        assert_close(result.value, 1.0 - config.cross_entity_hit_penalty);
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].kind, CrossEntityHitKind::InfrastructureId);
        assert_eq!(result.hits[0].token, "vip-foreign.com");
    }

    #[test]
    fn cross_entity_coherence_company_name_suppressed_when_target_name_present() {
        let result = cross_entity_coherence(
            &coherence_input(
                "Target Account discussed Other Company in a competitive note.",
                false,
            ),
            &TrustConfig::default(),
        );

        assert_close(result.value, 1.0);
        assert!(result.hits.is_empty());
    }

    #[test]
    fn cross_entity_coherence_allows_target_subdomain() {
        let result = cross_entity_coherence(
            &coherence_input(
                "Target Account traffic moved through vip.app.target.example.com today",
                false,
            ),
            &TrustConfig::default(),
        );

        assert_close(result.value, 1.0);
        assert!(result.hits.is_empty());
    }

    #[test]
    fn cross_entity_coherence_skips_when_context_expected() {
        let result = cross_entity_coherence(
            &coherence_input(
                "Peer benchmark compares Target Account with globex.example.",
                true,
            ),
            &TrustConfig::default(),
        );

        assert_close(result.value, 1.0);
        assert!(result.hits.is_empty());
        assert!(result.skipped_expected_context);
    }
}

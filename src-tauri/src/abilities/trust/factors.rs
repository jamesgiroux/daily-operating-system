use std::collections::HashSet;

use crate::db::claims::TemporalScope;

use super::config::TrustConfig;
use super::types::{
    CrossEntityCoherenceInput, CrossEntityHit, CrossEntityHitKind, FreshnessContext,
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
    input.source_reliability
}

pub fn freshness_weight(
    ctx: &FreshnessContext,
    temporal_scope: &TemporalScope,
    config: &TrustConfig,
) -> f64 {
    let base = match temporal_scope {
        TemporalScope::PointInTime => 1.0,
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

//! Deterministic `why this now` rationale construction.
//!
//! Given a scored candidate, returns a typed `WhyThisNow` payload
//! built from the primary factor and a closed set of triggers. No
//! provider / LLM call from this module.

use chrono::{DateTime, Utc};

use super::contracts::{
    FactorRationale, SalienceFactor, SalienceFactorKind, SalienceScore, TriggerKind, TriggerRef,
    WhyThisNow,
};

const MAX_TRIGGER_SUMMARY_ITEMS: usize = 3;

pub fn why_this_now_for_score(
    score: &SalienceScore,
    triggers: &[TriggerRef],
    now: DateTime<Utc>,
) -> WhyThisNow {
    let primary = primary_factor(score);
    let sanitized_triggers = sanitize_trigger_refs(triggers, now);
    let text = format!(
        "Salience driven by {}: {}. Triggers: {}.",
        factor_label(primary.kind),
        rationale_summary(&primary.rationale),
        trigger_summary(&sanitized_triggers)
    );

    WhyThisNow {
        primary_factor: primary.kind,
        text,
        triggers: sanitized_triggers,
    }
}

pub fn sanitized_trigger_ref(
    trigger_kind: TriggerKind,
    source: &str,
    at: DateTime<Utc>,
) -> TriggerRef {
    TriggerRef {
        trigger_kind,
        at,
        source: sanitize_trigger_source(trigger_kind, source).to_string(),
    }
}

fn primary_factor(score: &SalienceScore) -> SalienceFactor {
    score
        .factors
        .iter()
        .filter(|factor| factor.value.is_some())
        .max_by(|left, right| weighted_value(left).total_cmp(&weighted_value(right)))
        .cloned()
        .or_else(|| score.factors.first().cloned())
        .unwrap_or(SalienceFactor {
            kind: SalienceFactorKind::Freshness,
            value: Some(0.0),
            weight: 1.0,
            rationale: FactorRationale::Freshness { decay_factor: 0.0 },
        })
}

fn weighted_value(factor: &SalienceFactor) -> f64 {
    factor.value.unwrap_or(0.0) * factor.weight
}

fn sanitize_trigger_refs(triggers: &[TriggerRef], now: DateTime<Utc>) -> Vec<TriggerRef> {
    if triggers.is_empty() {
        return vec![sanitized_trigger_ref(
            TriggerKind::ScheduledScan,
            "scheduled_scan",
            now,
        )];
    }

    triggers
        .iter()
        .map(|trigger| {
            sanitized_trigger_ref(trigger.trigger_kind, trigger.source.as_str(), trigger.at)
        })
        .collect()
}

fn sanitize_trigger_source(kind: TriggerKind, source: &str) -> &str {
    match source {
        "scheduled_scan" | "signal_event" | "entity_change" | "manual_refresh"
        | "feedback_echo" | "claim_change" | "source_change" | "open_loop_change"
        | "meeting_window" | "decision_window" => source,
        _ => match kind {
            TriggerKind::SignalArrival => "signal_event",
            TriggerKind::EntityChange => "entity_change",
            TriggerKind::ScheduledScan => "scheduled_scan",
            TriggerKind::FeedbackEcho => "feedback_echo",
        },
    }
}

fn trigger_summary(triggers: &[TriggerRef]) -> String {
    let mut sources = triggers
        .iter()
        .map(|trigger| trigger.source.as_str())
        .collect::<Vec<_>>();
    sources.sort_unstable();
    sources.dedup();

    if sources.is_empty() {
        return "scheduled_scan".to_string();
    }

    let mut summary = sources
        .iter()
        .take(MAX_TRIGGER_SUMMARY_ITEMS)
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    if sources.len() > MAX_TRIGGER_SUMMARY_ITEMS {
        summary.push_str(", additional_trigger");
    }
    summary
}

fn factor_label(kind: SalienceFactorKind) -> &'static str {
    match kind {
        SalienceFactorKind::Importance => "importance",
        SalienceFactorKind::Novelty => "novelty",
        SalienceFactorKind::Urgency => "urgency",
        SalienceFactorKind::Timing => "timing",
        SalienceFactorKind::UserFit => "user fit",
        SalienceFactorKind::Freshness => "freshness",
        SalienceFactorKind::Trust => "trust",
        SalienceFactorKind::Corroboration => "corroboration",
        SalienceFactorKind::Contradiction => "contradiction",
        SalienceFactorKind::OpenLoopRelevance => "open loop relevance",
    }
}

fn rationale_summary(rationale: &FactorRationale) -> &'static str {
    match rationale {
        FactorRationale::Importance { .. } => "trusted source strength",
        FactorRationale::Novelty { .. } => "new compared with nearby memory",
        FactorRationale::Urgency { .. } => "deadline or time pressure",
        FactorRationale::Timing { .. } => "recent signal timing",
        FactorRationale::UserFit { .. } => "prior feedback fit",
        FactorRationale::Freshness { .. } => "current source timing",
        FactorRationale::Trust { .. } => "trust band",
        FactorRationale::Corroboration { .. } => "supporting evidence count",
        FactorRationale::Contradiction { .. } => "contradiction state",
        FactorRationale::OpenLoopRelevance { .. } => "open loop relevance",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use abilities_runtime::abilities::trust::types::TrustBand;

    #[test]
    fn rationale_uses_closed_tokens_and_sanitized_trigger_sources() {
        let now = "2026-05-26T12:00:00Z"
            .parse::<DateTime<Utc>>()
            .expect("fixture time parses");
        let score = SalienceScore {
            total: 0.9,
            factors: vec![SalienceFactor {
                kind: SalienceFactorKind::Urgency,
                value: Some(0.95),
                weight: 0.15,
                rationale: FactorRationale::Urgency {
                    deadline: Some(now),
                    decay_factor: 0.95,
                },
            }],
        };
        let trigger = TriggerRef {
            trigger_kind: TriggerKind::SignalArrival,
            at: now,
            source: "/Users/example/raw/file.md".to_string(),
        };

        let why = why_this_now_for_score(&score, &[trigger], now);

        assert_eq!(why.primary_factor, SalienceFactorKind::Urgency);
        assert_eq!(why.triggers[0].source, "signal_event");
        assert!(why.text.contains("urgency"));
        assert!(!why.text.contains("/Users/"));
        assert!(!serde_json::to_string(&why).unwrap().contains("/Users/"));
    }

    #[test]
    fn empty_triggers_default_to_scheduled_scan() {
        let now = "2026-05-26T12:00:00Z"
            .parse::<DateTime<Utc>>()
            .expect("fixture time parses");
        let score = SalienceScore {
            total: 0.4,
            factors: vec![SalienceFactor {
                kind: SalienceFactorKind::Trust,
                value: Some(0.6),
                weight: 0.1,
                rationale: FactorRationale::Trust {
                    trust_band: TrustBand::UseWithCaution,
                },
            }],
        };

        let why = why_this_now_for_score(&score, &[], now);

        assert_eq!(why.triggers[0].source, "scheduled_scan");
        assert!(why.text.contains("scheduled_scan"));
    }
}

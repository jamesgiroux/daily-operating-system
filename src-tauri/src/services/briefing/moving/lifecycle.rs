use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};

use super::EntityId;
use crate::services::briefing_view_model::{
    CorrectionState, LifecycleMixin, MovingSignalViewModel, SignalDotKind, SignalUrgency,
    TrustBandWire, TrustMixin, WhatSegment,
};
use crate::types::DashboardLifecycleUpdate;

pub(super) fn collect_lifecycle_signals(
    updates: Option<&[DashboardLifecycleUpdate]>,
    correction_lookup: &dyn Fn(i64) -> Option<CorrectionState>,
) -> Vec<(EntityId, MovingSignalViewModel)> {
    updates
        .map(|updates| {
            updates
                .iter()
                .map(|update| {
                    (
                        EntityId(update.account_id.clone()),
                        build_signal(update, correction_lookup),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn build_signal(
    update: &DashboardLifecycleUpdate,
    correction_lookup: &dyn Fn(i64) -> Option<CorrectionState>,
) -> MovingSignalViewModel {
    MovingSignalViewModel {
        trust: TrustMixin {
            trust_band: classify_confidence(update.confidence),
            trust_field_path: Some(format!("lifecycle_change.{}", update.change_id)),
            trust_source_date: Some(Some(update.created_at.clone())),
            rendered_provenance: None,
        },
        lifecycle: LifecycleMixin {
            correction_state: correction_lookup(update.change_id),
        },
        kind: SignalDotKind::Lifecycle,
        when: format_when(update),
        what_segments: format_what_segments(update),
        urgency: SignalUrgency::Normal,
        thread_action: None,
    }
}

fn classify_confidence(confidence: f64) -> TrustBandWire {
    if confidence.is_nan() {
        TrustBandWire::Unscored
    } else if confidence >= 0.85 {
        TrustBandWire::LikelyCurrent
    } else if confidence >= 0.60 {
        TrustBandWire::UseWithCaution
    } else {
        TrustBandWire::NeedsVerification
    }
}

fn format_what_segments(update: &DashboardLifecycleUpdate) -> Vec<WhatSegment> {
    if lifecycle_unchanged(update) {
        if let Some(stage) = update.renewal_stage.as_deref() {
            let stage_text = match update.previous_renewal_stage.as_deref() {
                Some(previous) if !previous.trim().is_empty() && previous != stage => {
                    format!(
                        "{} \u{2192} {}",
                        display_value(previous),
                        display_value(stage)
                    )
                }
                _ => display_value(stage),
            };
            return vec![segment("Renewal stage: ", false), segment(stage_text, true)];
        }
    }

    if update.previous_lifecycle.as_deref().is_some() {
        vec![
            segment("Moved to ", false),
            segment(display_value(&update.new_lifecycle), true),
        ]
    } else {
        vec![
            segment("Classified as ", false),
            segment(display_value(&update.new_lifecycle), true),
        ]
    }
}

fn lifecycle_unchanged(update: &DashboardLifecycleUpdate) -> bool {
    update
        .previous_lifecycle
        .as_deref()
        .is_some_and(|previous| previous == update.new_lifecycle)
}

fn display_value(value: &str) -> String {
    value.trim().replace('_', " ")
}

fn segment(text: impl Into<String>, emphasized: bool) -> WhatSegment {
    WhatSegment {
        text: text.into(),
        emphasized: emphasized.then_some(true),
    }
}

fn format_when(update: &DashboardLifecycleUpdate) -> String {
    let Some(created_at) = parse_created_at(&update.created_at) else {
        return "today".to_string();
    };
    let created_date = created_at.with_timezone(&Local).date_naive();
    let today = Local::now().date_naive();
    if created_date == today {
        "today".to_string()
    } else if created_date == today - chrono::Duration::days(1) {
        "yesterday".to_string()
    } else {
        created_date.to_string()
    }
}

fn parse_created_at(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                .ok()
                .and_then(|dt| Utc.from_local_datetime(&dt).single())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn update(change_id: i64, confidence: f64) -> DashboardLifecycleUpdate {
        DashboardLifecycleUpdate {
            change_id,
            account_id: "acct-1".to_string(),
            account_name: "Acme".to_string(),
            previous_lifecycle: Some("active".to_string()),
            new_lifecycle: "renewing".to_string(),
            previous_renewal_stage: None,
            renewal_stage: None,
            source: "calendar_pattern".to_string(),
            confidence,
            evidence: Some("QBR language shifted".to_string()),
            health_score_before: Some(0.7),
            health_score_after: Some(0.8),
            action_state: "pending".to_string(),
            created_at: "not-a-date".to_string(),
        }
    }

    fn collect(
        updates: &[DashboardLifecycleUpdate],
        correction_lookup: &dyn Fn(i64) -> Option<CorrectionState>,
    ) -> Vec<(EntityId, MovingSignalViewModel)> {
        collect_lifecycle_signals(Some(updates), correction_lookup)
    }

    #[test]
    fn maps_each_update_to_a_lifecycle_signal() {
        let updates = vec![update(42, 0.91)];
        let signals = collect(&updates, &|_| None);

        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].0 .0, "acct-1");
        let signal = &signals[0].1;
        assert_eq!(signal.kind, SignalDotKind::Lifecycle);
        assert_eq!(signal.trust.trust_band, TrustBandWire::LikelyCurrent);
        assert_eq!(
            signal.trust.trust_field_path.as_deref(),
            Some("lifecycle_change.42")
        );
        assert_eq!(signal.lifecycle.correction_state, None);
        assert_eq!(signal.when, "today");
        assert_eq!(signal.what_segments[0].text, "Moved to ");
        assert_eq!(signal.what_segments[1].text, "renewing");
        assert_eq!(signal.what_segments[1].emphasized, Some(true));
    }

    #[test]
    fn classifies_confidence_boundaries() {
        assert_eq!(classify_confidence(0.85), TrustBandWire::LikelyCurrent);
        assert_eq!(classify_confidence(0.849), TrustBandWire::UseWithCaution);
        assert_eq!(classify_confidence(0.60), TrustBandWire::UseWithCaution);
        assert_eq!(classify_confidence(0.599), TrustBandWire::NeedsVerification);
        assert_eq!(classify_confidence(f64::NAN), TrustBandWire::Unscored);
    }

    #[test]
    fn picks_up_correction_state_from_lookup() {
        let updates = vec![update(7, 0.7)];
        let signals = collect(&updates, &|change_id| {
            (change_id == 7).then_some(CorrectionState::Corrected)
        });

        assert_eq!(
            signals[0].1.lifecycle.correction_state,
            Some(CorrectionState::Corrected)
        );
    }

    #[test]
    fn omits_correction_state_when_lookup_is_empty() {
        let updates = vec![update(7, 0.7)];
        let signals = collect(&updates, &|_| None);

        assert_eq!(signals[0].1.lifecycle.correction_state, None);
    }

    #[test]
    fn handles_empty_updates() {
        assert!(collect_lifecycle_signals(None, &|_| None).is_empty());
        assert!(collect_lifecycle_signals(Some(&[]), &|_| None).is_empty());
    }

    #[test]
    fn formats_initial_classification_and_stage_transition() {
        let mut initial = update(1, 0.9);
        initial.previous_lifecycle = None;
        let initial_signal = collect(&[initial], &|_| None).remove(0).1;
        assert_eq!(initial_signal.what_segments[0].text, "Classified as ");
        assert_eq!(initial_signal.what_segments[1].text, "renewing");

        let mut stage = update(2, 0.9);
        stage.previous_lifecycle = Some("renewing".to_string());
        stage.new_lifecycle = "renewing".to_string();
        stage.previous_renewal_stage = Some("prospecting".to_string());
        stage.renewal_stage = Some("engaged".to_string());
        let stage_signal = collect(&[stage], &|_| None).remove(0).1;
        assert_eq!(stage_signal.what_segments[0].text, "Renewal stage: ");
        assert_eq!(stage_signal.what_segments[1].text, "prospecting → engaged");
    }

    #[test]
    fn returns_multiple_updates_for_the_same_entity() {
        let updates = vec![update(1, 0.9), update(2, 0.7)];
        let signals = collect(&updates, &|_| None);

        assert_eq!(signals.len(), 2);
        assert!(signals.iter().all(|(entity_id, _)| entity_id.0 == "acct-1"));
    }

    #[test]
    fn serializes_camel_case_wire_shape() {
        let updates = vec![update(42, 0.91)];
        let signal = collect(&updates, &|_| Some(CorrectionState::Contested))
            .remove(0)
            .1;
        let parsed: Value = serde_json::from_str(&serde_json::to_string(&signal).unwrap()).unwrap();

        assert_eq!(parsed["kind"], "lifecycle");
        assert_eq!(parsed["trustBand"], "likely_current");
        assert_eq!(parsed["correctionState"], "contested");
        assert_eq!(parsed["whatSegments"][1]["emphasized"], true);
    }
}

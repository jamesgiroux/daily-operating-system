//! Email signal feeder for the briefing Moving slice.
//!
//! Trust source: Gmail metadata persisted in `emails`, local enrichment rows
//! in `email_signals`, and local relevance scores on `emails`. No Glean data
//! is read here. Missing or invalid data yields no candidate.

use std::collections::HashMap;

use serde_json::json;

use crate::db::{ActionDb, DbEmail, DbEmailSignal};
use crate::services::briefing_view_model::{
    LifecycleMixin, MovingSignalViewModel, RenderedProvenanceSummary, SignalDotKind, SignalUrgency,
    ThreadAction, TrustBandWire, TrustMixin, WhatSegment,
};
use crate::state::AppState;

const EMAIL_MOVING_SIGNAL_LIMIT: usize = 12;

#[derive(Debug, Clone)]
pub struct EmailMovingSignalCandidate {
    pub entity_id: String,
    pub entity_type: String,
    pub entity_name: Option<String>,
    pub movement_score: f64,
    pub occurred_at_iso: Option<String>,
    pub source_email_id: String,
    pub source_thread_id: Option<String>,
    pub signal: MovingSignalViewModel,
}

pub async fn collect_email_moving_signal_candidates(
    state: &AppState,
) -> Vec<EmailMovingSignalCandidate> {
    state
        .db_read(build_email_moving_signal_candidates)
        .await
        .unwrap_or_default()
}

pub fn build_email_moving_signal_candidates(
    db: &ActionDb,
) -> Result<Vec<EmailMovingSignalCandidate>, String> {
    let all_rows = db.get_all_active_emails().map_err(|e| e.to_string())?;
    if all_rows.is_empty() {
        return Ok(Vec::new());
    }

    let thread_rows = crate::services::emails::collapse_to_latest_thread_emails(&all_rows);
    let selected = crate::services::emails::select_briefing_email_rows(
        &thread_rows,
        EMAIL_MOVING_SIGNAL_LIMIT,
    );
    let email_ids: Vec<String> = selected
        .iter()
        .map(|email| email.email_id.clone())
        .collect();
    let mut signals_by_email: HashMap<String, Vec<DbEmailSignal>> = db
        .list_email_signals_by_email_ids(&email_ids)
        .map_err(|e| e.to_string())?
        .into_iter()
        .fold(HashMap::new(), |mut acc, signal| {
            acc.entry(signal.email_id.clone()).or_default().push(signal);
            acc
        });

    let mut candidates = Vec::new();
    for email in selected {
        let entity_name = email
            .entity_id
            .as_deref()
            .zip(email.entity_type.as_deref())
            .and_then(|(entity_id, entity_type)| resolve_entity_name(db, entity_id, entity_type));
        let signals = signals_by_email.remove(&email.email_id).unwrap_or_default();
        if let Some(candidate) = candidate_from_email(email, &signals, entity_name) {
            candidates.push(candidate);
        }
    }

    Ok(candidates)
}

pub fn candidate_from_email(
    email: &DbEmail,
    signals: &[DbEmailSignal],
    entity_name: Option<String>,
) -> Option<EmailMovingSignalCandidate> {
    let entity_id = clean(email.entity_id.as_deref())?;
    let entity_type = clean(email.entity_type.as_deref())?;
    if !is_supported_entity_type(&entity_type) {
        return None;
    }

    let preferred_signal = preferred_signal(email, signals);
    let (body_text, body_source) = signal_body(email, preferred_signal)?;
    let occurred_at_iso = preferred_signal
        .map(|signal| signal.detected_at.clone())
        .or_else(|| email.received_at.clone());
    let trust_source_date = preferred_signal
        .map(|signal| signal.detected_at.clone())
        .or_else(|| email.enriched_at.clone())
        .or_else(|| email.last_enrichment_at.clone())
        .or_else(|| email.received_at.clone())
        .or_else(|| Some(email.updated_at.clone()));

    let signal = MovingSignalViewModel {
        trust: email_trust(
            email,
            preferred_signal,
            trust_source_date.clone(),
            &body_source,
        ),
        lifecycle: LifecycleMixin {
            correction_state: None,
        },
        kind: SignalDotKind::Email,
        when: occurred_at_iso
            .clone()
            .unwrap_or_else(|| "Email".to_string()),
        what_segments: email_segments(email, &body_text),
        urgency: email_urgency(email, preferred_signal),
        thread_action: Some(ThreadAction {
            label: "Open email".to_string(),
            href: "/emails".to_string(),
        }),
    };

    Some(EmailMovingSignalCandidate {
        entity_id,
        entity_type,
        entity_name,
        movement_score: email.relevance_score.unwrap_or(0.0),
        occurred_at_iso,
        source_email_id: email.email_id.clone(),
        source_thread_id: email.thread_id.clone(),
        signal,
    })
}

fn preferred_signal<'a>(
    email: &DbEmail,
    signals: &'a [DbEmailSignal],
) -> Option<&'a DbEmailSignal> {
    signals
        .iter()
        .find(|signal| {
            email.entity_id.as_deref() == Some(signal.entity_id.as_str())
                && email.entity_type.as_deref() == Some(signal.entity_type.as_str())
        })
        .or_else(|| signals.first())
}

fn signal_body(email: &DbEmail, signal: Option<&DbEmailSignal>) -> Option<(String, String)> {
    if let Some(signal) = signal {
        if let Some(text) = clean(Some(signal.signal_text.as_str())) {
            return Some((text, signal.source.clone()));
        }
    }
    if let Some(summary) = clean(email.contextual_summary.as_deref()) {
        return Some((summary, "email_enrichment".to_string()));
    }
    clean(email.subject.as_deref()).map(|subject| (subject, "gmail".to_string()))
}

fn email_segments(email: &DbEmail, body_text: &str) -> Vec<WhatSegment> {
    let prefix = clean(email.sender_name.as_deref())
        .or_else(|| clean(email.sender_email.as_deref()))
        .map(|sender| format!("{sender}: "))
        .unwrap_or_else(|| "Email: ".to_string());
    vec![
        WhatSegment {
            text: prefix,
            emphasized: None,
        },
        WhatSegment {
            text: body_text.to_string(),
            emphasized: Some(true),
        },
    ]
}

fn email_urgency(email: &DbEmail, signal: Option<&DbEmailSignal>) -> SignalUrgency {
    let raw = signal
        .and_then(|signal| signal.urgency.as_deref())
        .or(email.urgency.as_deref())
        .unwrap_or_default()
        .to_lowercase();
    if matches!(raw.as_str(), "urgent" | "overdue" | "high") {
        SignalUrgency::Overdue
    } else {
        SignalUrgency::Normal
    }
}

fn email_trust(
    email: &DbEmail,
    signal: Option<&DbEmailSignal>,
    source_date: Option<String>,
    body_source: &str,
) -> TrustMixin {
    TrustMixin {
        trust_band: TrustBandWire::Unscored,
        trust_field_path: Some("moving.entities[].signals[].whatSegments".to_string()),
        trust_source_date: source_date.clone().map(Some),
        rendered_provenance: Some(RenderedProvenanceSummary {
            surface: Some("briefing.moving.email".to_string()),
            value: email_provenance(email, signal, source_date, body_source),
        }),
    }
}

fn email_provenance(
    email: &DbEmail,
    signal: Option<&DbEmailSignal>,
    source_date: Option<String>,
    body_source: &str,
) -> serde_json::Value {
    let mut sources = vec![json!({
        "system": "gmail",
        "email_id": &email.email_id,
        "thread_id": &email.thread_id,
        "source_asof": &email.received_at,
    })];

    if let Some(score) = email.relevance_score {
        sources.push(json!({
            "system": "email_scoring",
            "email_id": &email.email_id,
            "source_asof": &email.updated_at,
            "score": score,
            "reason": &email.score_reason,
        }));
    }

    if let Some(signal) = signal {
        sources.push(json!({
            "system": &signal.source,
            "email_signal_id": signal.id,
            "email_id": &signal.email_id,
            "signal_type": &signal.signal_type,
            "source_asof": &signal.detected_at,
        }));
    } else if body_source == "email_enrichment" {
        sources.push(json!({
            "system": "email_enrichment",
            "email_id": &email.email_id,
            "source_asof": email.enriched_at.as_ref().or(email.last_enrichment_at.as_ref()),
        }));
    }

    let source_refs = sources
        .iter()
        .filter_map(|source| source.get("system").cloned())
        .collect::<Vec<_>>();

    json!({
        "sources": sources,
        "field_attributions": {
            "whatSegments": {
                "subject": {
                    "kind": "email",
                    "id": &email.email_id,
                },
                "derivation": body_source,
                "source_refs": source_refs,
                "trust_band": "unscored",
                "explanation": "Moving email signal assembled from persisted mail metadata and local enrichment."
            }
        },
        "produced_at": source_date,
    })
}

fn resolve_entity_name(db: &ActionDb, entity_id: &str, entity_type: &str) -> Option<String> {
    match entity_type {
        "account" => db
            .get_account(entity_id)
            .ok()
            .flatten()
            .map(|entity| entity.name),
        "person" => db
            .get_person(entity_id)
            .ok()
            .flatten()
            .map(|entity| entity.name),
        "project" => db
            .get_project(entity_id)
            .ok()
            .flatten()
            .map(|entity| entity.name),
        _ => None,
    }
}

fn clean(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn is_supported_entity_type(value: &str) -> bool {
    matches!(value, "account" | "person" | "project")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn email(id: &str) -> DbEmail {
        DbEmail {
            email_id: id.to_string(),
            thread_id: Some("thread-1".to_string()),
            sender_email: Some("sender@example.com".to_string()),
            sender_name: Some("Pat Sender".to_string()),
            subject: Some("Renewal check-in".to_string()),
            snippet: Some("Snippet".to_string()),
            priority: Some("medium".to_string()),
            is_unread: true,
            received_at: Some("2026-05-07T10:00:00Z".to_string()),
            enrichment_state: "enriched".to_string(),
            enrichment_attempts: 1,
            last_enrichment_at: Some("2026-05-07T10:04:00Z".to_string()),
            enriched_at: Some("2026-05-07T10:05:00Z".to_string()),
            last_seen_at: Some("2026-05-07T10:06:00Z".to_string()),
            resolved_at: None,
            entity_id: Some("acc-1".to_string()),
            entity_type: Some("account".to_string()),
            contextual_summary: Some("Customer asked for renewal timing.".to_string()),
            sentiment: Some("neutral".to_string()),
            urgency: Some("normal".to_string()),
            user_is_last_sender: false,
            last_sender_email: Some("sender@example.com".to_string()),
            message_count: 1,
            created_at: "2026-05-07T10:00:00Z".to_string(),
            updated_at: "2026-05-07T10:07:00Z".to_string(),
            relevance_score: Some(0.42),
            score_reason: Some("Linked to customer".to_string()),
            pinned_at: None,
            commitments: None,
            questions: None,
            is_noise: false,
            to_recipients: None,
            cc_recipients: None,
        }
    }

    fn signal() -> DbEmailSignal {
        DbEmailSignal {
            id: 7,
            email_id: "email-1".to_string(),
            sender_email: Some("sender@example.com".to_string()),
            person_id: None,
            entity_id: "acc-1".to_string(),
            entity_type: "account".to_string(),
            signal_type: "risk".to_string(),
            signal_text: "Customer flagged implementation risk.".to_string(),
            confidence: Some(0.8),
            sentiment: Some("negative".to_string()),
            urgency: Some("urgent".to_string()),
            detected_at: "2026-05-07T10:08:00Z".to_string(),
            source: "email_enrichment".to_string(),
        }
    }

    #[test]
    fn candidate_uses_email_signal_text_and_trust_source() {
        let email = email("email-1");
        let signal = signal();

        let candidate =
            candidate_from_email(&email, &[signal], Some("Acme".to_string())).expect("candidate");

        assert_eq!(candidate.entity_id, "acc-1");
        assert_eq!(candidate.entity_type, "account");
        assert_eq!(candidate.entity_name.as_deref(), Some("Acme"));
        assert_eq!(candidate.source_email_id, "email-1");
        assert_eq!(candidate.source_thread_id.as_deref(), Some("thread-1"));
        assert_eq!(candidate.signal.kind, SignalDotKind::Email);
        assert_eq!(candidate.signal.urgency, SignalUrgency::Overdue);
        assert_eq!(
            candidate.signal.what_segments[1].text,
            "Customer flagged implementation risk."
        );
        assert_eq!(candidate.signal.trust.trust_band, TrustBandWire::Unscored);
        assert_eq!(
            candidate.signal.trust.trust_field_path.as_deref(),
            Some("moving.entities[].signals[].whatSegments")
        );
        assert_eq!(
            candidate
                .signal
                .trust
                .trust_source_date
                .as_ref()
                .and_then(|value| value.as_deref()),
            Some("2026-05-07T10:08:00Z")
        );
        assert!(candidate.signal.trust.rendered_provenance.is_some());
    }

    #[test]
    fn candidate_falls_back_to_summary_then_subject() {
        let with_summary = email("email-1");
        let candidate =
            candidate_from_email(&with_summary, &[], None).expect("summary candidate");
        assert_eq!(
            candidate.signal.what_segments[1].text,
            "Customer asked for renewal timing."
        );
        assert_eq!(
            candidate
                .signal
                .trust
                .trust_source_date
                .as_ref()
                .and_then(|value| value.as_deref()),
            Some("2026-05-07T10:05:00Z")
        );

        let mut subject_only = email("email-2");
        subject_only.contextual_summary = None;
        let candidate = candidate_from_email(&subject_only, &[], None).expect("subject candidate");
        assert_eq!(candidate.signal.what_segments[1].text, "Renewal check-in");
    }
}

//! Briefing callout generation (I308 — ADR-0080 Phase 4).
//!
//! Generates proactive intelligence callouts for the daily briefing by
//! querying recent high-confidence signals and optionally ranking them
//! by embedding similarity to today's meeting context.

use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::db::{ActionDb, DbError};
use crate::embeddings::EmbeddingModel;
use crate::helpers;

use super::bus::SignalEvent;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A callout to surface in the daily briefing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefingCallout {
    pub id: String,
    pub severity: String,
    pub headline: String,
    pub detail: String,
    pub entity_name: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub relevance_score: Option<f64>,
}

/// Signal types that produce callouts.
const CALLOUT_SIGNAL_TYPES: &[&str] = &[
    "stakeholder_change",
    "champion_risk",
    "renewal_risk_escalation",
    "renewal_at_risk",
    "engagement_warning",
    "project_health_warning",
    "post_meeting_followup",
    "proactive_renewal_gap",
    "proactive_relationship_drift",
    "proactive_email_spike",
    "proactive_meeting_load",
    "proactive_stale_champion",
    "proactive_action_cluster",
    "proactive_prep_gap",
    "proactive_no_contact",
    "cadence_anomaly",
];

// ---------------------------------------------------------------------------
// Callout generation
// ---------------------------------------------------------------------------

/// Generate briefing callouts from recent high-confidence signals.
///
/// Optionally ranks by embedding similarity to today's meetings if an
/// embedding model is provided.
pub fn generate_callouts(
    db: &ActionDb,
    model: Option<&EmbeddingModel>,
    todays_meetings: &[Value],
) -> Vec<BriefingCallout> {
    // Get recent signals (last 24h) of callout-worthy types
    let signals = match db.get_recent_callout_signals(24, CALLOUT_SIGNAL_TYPES) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("generate_callouts: failed to query signals: {}", e);
            return Vec::new();
        }
    };

    if signals.is_empty() {
        return Vec::new();
    }

    // Optionally rank by relevance to today's meetings
    let meeting_context = build_meeting_context_string(todays_meetings);
    let scored_signals: Vec<(SignalEvent, f64)> = if !meeting_context.is_empty() {
        if let Some(m) = model {
            super::relevance::rank_signals_by_relevance(m, &meeting_context, &signals)
        } else {
            signals.iter().map(|s| (s.clone(), 0.0)).collect()
        }
    } else {
        signals.iter().map(|s| (s.clone(), 0.0)).collect()
    };

    // Convert to callouts
    let mut callouts: Vec<BriefingCallout> = scored_signals
        .into_iter()
        .filter(|(s, _)| s.confidence >= 0.55)
        .map(|(signal, relevance)| {
            let severity = classify_severity(signal.confidence);
            let (headline, detail) = build_callout_text(&signal);
            let entity_name = Some(helpers::resolve_entity_name(db, &signal.entity_type, &signal.entity_id));

            let callout = BriefingCallout {
                id: format!("bc-{}", Uuid::new_v4()),
                severity,
                headline,
                detail,
                entity_name,
                entity_type: signal.entity_type.clone(),
                entity_id: signal.entity_id.clone(),
                relevance_score: if relevance > 0.0 {
                    Some(relevance)
                } else {
                    None
                },
            };

            // Persist callout
            let _ = db.upsert_briefing_callout(&callout, &signal.id);

            callout
        })
        .collect();

    // Sort: critical first, then by relevance score
    callouts.sort_by(|a, b| {
        let sev_ord = severity_order(&a.severity).cmp(&severity_order(&b.severity));
        if sev_ord != std::cmp::Ordering::Equal {
            return sev_ord;
        }
        b.relevance_score
            .unwrap_or(0.0)
            .partial_cmp(&a.relevance_score.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    callouts
}

fn severity_order(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "warning" => 1,
        "info" => 2,
        _ => 3,
    }
}

fn classify_severity(confidence: f64) -> String {
    if confidence >= 0.85 {
        "critical".to_string()
    } else if confidence >= 0.70 {
        "warning".to_string()
    } else {
        "info".to_string()
    }
}

fn build_callout_text(signal: &SignalEvent) -> (String, String) {
    let parsed: Value = signal
        .value
        .as_deref()
        .and_then(|v| serde_json::from_str(v).ok())
        .unwrap_or(Value::Null);

    match signal.signal_type.as_str() {
        "stakeholder_change" => {
            let detail = parsed
                .get("detail")
                .and_then(|v| v.as_str())
                .unwrap_or("Role or title change detected");
            ("Stakeholder change detected".to_string(), detail.to_string())
        }
        "champion_risk" => {
            let detail = parsed
                .get("detail")
                .and_then(|v| v.as_str())
                .unwrap_or("Negative sentiment from account champion");
            ("Champion risk signal".to_string(), detail.to_string())
        }
        "renewal_risk_escalation" => {
            let change_type = parsed
                .get("change_type")
                .and_then(|v| v.as_str())
                .unwrap_or("departure");
            (
                "Renewal risk: champion departure".to_string(),
                format!(
                    "Key contact {} detected near renewal window",
                    change_type
                ),
            )
        }
        "engagement_warning" => {
            let drop_pct = parsed
                .get("drop_percentage")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            (
                "Engagement declining".to_string(),
                format!("Meeting frequency dropped {}%", drop_pct as i64),
            )
        }
        "project_health_warning" => {
            let count = parsed
                .get("overdue_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            (
                "Project health concern".to_string(),
                format!("{} overdue actions", count),
            )
        }
        "post_meeting_followup" => {
            let sender = parsed
                .get("sender_email")
                .and_then(|v| v.as_str())
                .unwrap_or("attendee");
            (
                "Post-meeting follow-up received".to_string(),
                format!("Email from {} after recent meeting", sender),
            )
        }
        "proactive_renewal_gap" => {
            let name = parsed.get("account_name").and_then(|v| v.as_str()).unwrap_or("Account");
            let days = parsed.get("days_until_renewal").and_then(|v| v.as_i64()).unwrap_or(0);
            let gap = parsed.get("last_contact_days").and_then(|v| v.as_i64()).unwrap_or(0);
            (
                "Renewal approaching with no QBR".to_string(),
                format!("{} renews in {}d — no executive contact in {}d", name, days, gap),
            )
        }
        "proactive_relationship_drift" => {
            let name = parsed.get("person_name").and_then(|v| v.as_str()).unwrap_or("Contact");
            let drop = parsed.get("drop_pct").and_then(|v| v.as_i64()).unwrap_or(0);
            (
                "Meeting frequency declining".to_string(),
                format!("Down {}% with {} over last 30 days", drop, name),
            )
        }
        "proactive_email_spike" => {
            let name = parsed.get("entity_name").and_then(|v| v.as_str()).unwrap_or("Entity");
            let count = parsed.get("count_7d").and_then(|v| v.as_i64()).unwrap_or(0);
            let avg = parsed.get("avg_weekly").and_then(|v| v.as_f64()).unwrap_or(0.0);
            (
                "Email activity spike".to_string(),
                format!("{} emails from {} contacts this week (vs {:.1}/week)", count, name, avg),
            )
        }
        "proactive_meeting_load" => {
            let next = parsed.get("next_week_count").and_then(|v| v.as_i64()).unwrap_or(0);
            let this = parsed.get("this_week_count").and_then(|v| v.as_i64()).unwrap_or(0);
            (
                "Heavy week ahead".to_string(),
                format!("{} meetings next week vs {} this week", next, this),
            )
        }
        "proactive_stale_champion" => {
            let person = parsed.get("person_name").and_then(|v| v.as_str()).unwrap_or("Champion");
            let days = parsed.get("days_since_contact").and_then(|v| v.as_i64()).unwrap_or(0);
            let account = parsed.get("account_name").and_then(|v| v.as_str()).unwrap_or("Account");
            let renewal = parsed.get("renewal_days").and_then(|v| v.as_i64()).unwrap_or(0);
            (
                "Champion going cold".to_string(),
                format!("No contact with {} in {}d — {} renewal in {}d", person, days, account, renewal),
            )
        }
        "proactive_prep_gap" => {
            let total = parsed.get("total_external").and_then(|v| v.as_i64()).unwrap_or(0);
            let with_intel = parsed.get("with_intel").and_then(|v| v.as_i64()).unwrap_or(0);
            let unprepped = total - with_intel;
            (
                "Prep coverage gap".to_string(),
                format!("{}/{} external meetings tomorrow without intelligence", unprepped, total),
            )
        }
        "proactive_action_cluster" => {
            let name = parsed.get("entity_name").and_then(|v| v.as_str()).unwrap_or("Entity");
            let pending = parsed.get("pending_count").and_then(|v| v.as_i64()).unwrap_or(0);
            let overdue = parsed.get("overdue_count").and_then(|v| v.as_i64()).unwrap_or(0);
            (
                "Action overload".to_string(),
                format!("{} pending actions on {} ({} overdue)", pending, name, overdue),
            )
        }
        "proactive_no_contact" => {
            let name = parsed.get("account_name").and_then(|v| v.as_str()).unwrap_or("Account");
            (
                "Account going dark".to_string(),
                format!("No meeting or email with {} in 30+ days", name),
            )
        }
        "renewal_at_risk" => {
            let days = parsed
                .get("days_without_meeting")
                .and_then(|v| v.as_i64())
                .unwrap_or(30);
            (
                "Renewal at risk: no recent engagement".to_string(),
                format!("No meetings in {} days near renewal window", days),
            )
        }
        "cadence_anomaly" => {
            // I319: value is the anomaly type string ("gone_quiet" or "activity_spike")
            let anomaly_type = signal.value.as_deref().unwrap_or("unknown");
            match anomaly_type {
                "gone_quiet" => (
                    "Email activity dropped".to_string(),
                    format!("Significant decrease in email volume from {}", signal.entity_id),
                ),
                "activity_spike" => (
                    "Email activity surge".to_string(),
                    format!("Unusual spike in email volume from {}", signal.entity_id),
                ),
                _ => (
                    "Email cadence anomaly".to_string(),
                    format!("Unusual email pattern for {}", signal.entity_id),
                ),
            }
        }
        _ => (
            format!("Signal: {}", signal.signal_type),
            signal
                .value
                .clone()
                .unwrap_or_else(|| "No details".to_string()),
        ),
    }
}

fn build_meeting_context_string(meetings: &[Value]) -> String {
    meetings
        .iter()
        .filter_map(|m| {
            let title = m.get("title").or_else(|| m.get("summary"))?.as_str()?;
            Some(title.to_string())
        })
        .collect::<Vec<_>>()
        .join(". ")
}


// ---------------------------------------------------------------------------
// ActionDb methods
// ---------------------------------------------------------------------------

impl ActionDb {
    /// Get recent signals of specified types within the last N hours.
    pub fn get_recent_callout_signals(
        &self,
        hours: i32,
        signal_types: &[&str],
    ) -> Result<Vec<SignalEvent>, DbError> {
        if signal_types.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders: Vec<String> = signal_types
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 2))
            .collect();
        let sql = format!(
            "SELECT id, entity_type, entity_id, signal_type, source, value,
                    confidence, decay_half_life_days, created_at, superseded_by,
                    source_context
             FROM signal_events
             WHERE created_at >= datetime('now', ?1)
               AND superseded_by IS NULL
               AND signal_type IN ({})
             ORDER BY confidence DESC, created_at DESC",
            placeholders.join(", ")
        );

        let hours_param = format!("-{} hours", hours);
        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> =
            vec![Box::new(hours_param)];
        for st in signal_types {
            all_params.push(Box::new(st.to_string()));
        }
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            all_params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn_ref().prepare(&sql)?;
        let rows = stmt.query_map(param_refs.as_slice(), Self::map_signal_event_row)?;

        let mut signals = Vec::new();
        for row in rows {
            signals.push(row?);
        }
        Ok(signals)
    }

    /// Upsert a briefing callout (insert or ignore if signal already has callout).
    pub fn upsert_briefing_callout(
        &self,
        callout: &BriefingCallout,
        signal_id: &str,
    ) -> Result<(), DbError> {
        let context_json = serde_json::json!({
            "relevance_score": callout.relevance_score,
        })
        .to_string();

        self.conn_ref().execute(
            "INSERT OR IGNORE INTO briefing_callouts
                (id, signal_id, entity_type, entity_id, entity_name, severity, headline, detail, context_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                callout.id,
                signal_id,
                callout.entity_type,
                callout.entity_id,
                callout.entity_name,
                callout.severity,
                callout.headline,
                callout.detail,
                context_json,
            ],
        )?;
        Ok(())
    }

}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    #[test]
    fn test_classify_severity() {
        assert_eq!(classify_severity(0.90), "critical");
        assert_eq!(classify_severity(0.85), "critical");
        assert_eq!(classify_severity(0.75), "warning");
        assert_eq!(classify_severity(0.60), "info");
    }

    #[test]
    fn test_generate_callouts_empty() {
        let db = test_db();
        let callouts = generate_callouts(&db, None, &[]);
        assert!(callouts.is_empty());
    }

    #[test]
    fn test_generate_callouts_with_signal() {
        let db = test_db();

        // Insert a stakeholder_change signal
        super::super::bus::emit_signal(
            &db,
            "account",
            "a1",
            "stakeholder_change",
            "propagation",
            Some("{\"detail\": \"Alice promoted to CRO\"}"),
            0.85,
        )
        .expect("emit");

        // Create account so entity name resolves
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at) VALUES ('a1', 'Acme Corp', '2026-01-01')",
                [],
            )
            .unwrap();

        let callouts = generate_callouts(&db, None, &[]);
        assert_eq!(callouts.len(), 1);
        assert_eq!(callouts[0].severity, "critical");
        assert_eq!(callouts[0].entity_name.as_deref(), Some("Acme Corp"));
    }

}

//! Email-calendar correlation bridge (ADR-0080).
//!
//! For each meeting in the next 48 hours, find email_signals where the
//! sender_email overlaps with meeting attendee emails (last 7 days).
//! Emit `pre_meeting_context` signals linking relevant email threads
//! to upcoming meetings.

use serde::{Deserialize, Serialize};

use crate::db::ActionDb;
use crate::helpers;

use super::bus;

/// A correlation between an email signal and an upcoming meeting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeCorrelation {
    pub meeting_id: String,
    pub meeting_title: String,
    pub email_signal_id: String,
    pub sender_email: String,
    pub signal_text: String,
    pub signal_id: String,
}

/// Run the email-meeting bridge: correlate recent email signals with
/// upcoming meetings (next 48h) by attendee email overlap.
///
/// Uses existing infrastructure:
/// - `meetings_history` table for upcoming meetings + attendee CSV
/// - `email_signals` table for sender_email matching
/// - `emit_signal()` to write correlations to signal_events
pub fn run_email_meeting_bridge(db: &ActionDb) -> Result<Vec<BridgeCorrelation>, String> {
    let conn = db.conn_ref();
    let mut correlations = Vec::new();

    // Get meetings in next 48 hours with attendees
    let mut meeting_stmt = conn
        .prepare(
            "SELECT id, title, attendees, calendar_event_id
             FROM meetings_history
             WHERE start_time >= datetime('now')
               AND start_time <= datetime('now', '+48 hours')
               AND attendees IS NOT NULL AND attendees != ''",
        )
        .map_err(|e| format!("Failed to prepare meeting query: {}", e))?;

    let meetings: Vec<(String, String, String, Option<String>)> = meeting_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|e| format!("Failed to query meetings: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if meetings.is_empty() {
        return Ok(correlations);
    }

    // Get recent email signals (last 7 days)
    let mut email_stmt = conn
        .prepare(
            "SELECT CAST(id AS TEXT), COALESCE(sender_email, ''), entity_id, entity_type, signal_type, signal_text
             FROM email_signals
             WHERE detected_at >= datetime('now', '-7 days')
             ORDER BY detected_at DESC",
        )
        .map_err(|e| format!("Failed to prepare email query: {}", e))?;

    let email_signals: Vec<(String, String, String, String, String, String)> = email_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| format!("Failed to query email signals: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if email_signals.is_empty() {
        return Ok(correlations);
    }

    // For each meeting, check for attendee overlap with email senders
    for (meeting_id, meeting_title, attendees_csv, _event_id) in &meetings {
        let attendee_emails = helpers::parse_attendee_emails(attendees_csv);

        if attendee_emails.is_empty() {
            continue;
        }

        for (email_id, sender_email, entity_id, _entity_type, _signal_type, signal_text) in
            &email_signals
        {
            let sender_lower = sender_email.to_lowercase();
            if attendee_emails.contains(&sender_lower) {
                // Emit a pre_meeting_context signal
                let value = serde_json::json!({
                    "meeting_id": meeting_id,
                    "meeting_title": meeting_title,
                    "email_signal_id": email_id,
                    "sender_email": sender_email,
                    "signal_text": signal_text,
                })
                .to_string();

                let signal_id = bus::emit_signal(
                    db,
                    "meeting",
                    meeting_id,
                    "pre_meeting_context",
                    "email_thread",
                    Some(&value),
                    0.75,
                )
                .map_err(|e| format!("Failed to emit bridge signal: {}", e))?;

                correlations.push(BridgeCorrelation {
                    meeting_id: meeting_id.clone(),
                    meeting_title: meeting_title.clone(),
                    email_signal_id: email_id.clone(),
                    sender_email: sender_email.clone(),
                    signal_text: signal_text.clone(),
                    signal_id,
                });

                // Also emit to the entity for cross-reference
                if !entity_id.is_empty() {
                    let _ = bus::emit_signal(
                        db,
                        "account",
                        entity_id,
                        "pre_meeting_context",
                        "email_thread",
                        Some(&value),
                        0.75,
                    );
                }
            }
        }
    }

    if !correlations.is_empty() {
        log::info!(
            "Email-meeting bridge: {} correlations found for {} upcoming meetings",
            correlations.len(),
            meetings.len(),
        );
    }

    Ok(correlations)
}

/// I372: Emit entity signals from ALL enriched emails (not just meeting-linked).
///
/// For each recently enriched email with a resolved entity, emit:
/// - `email_sentiment` — the sentiment assessment (positive/negative/mixed)
/// - `email_urgency_high` — only for high-urgency emails
///
/// Source: `email_enrichment`. Signals compound with existing entity signals
/// via the propagation engine.
pub fn emit_enriched_email_signals(db: &ActionDb) -> usize {
    // Get enriched emails with resolved entities
    let mut stmt = match db.conn_ref().prepare(
        "SELECT email_id, entity_id, entity_type, sentiment, urgency, subject, sender_email
         FROM emails
         WHERE enrichment_state = 'enriched'
           AND entity_id IS NOT NULL
           AND entity_type IS NOT NULL
           AND last_enrichment_at >= datetime('now', '-1 hour')
         ORDER BY last_enrichment_at DESC
         LIMIT 50",
    ) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("I372: Failed to prepare enriched email query: {}", e);
            return 0;
        }
    };

    let rows: Vec<(String, String, String, Option<String>, Option<String>, Option<String>, Option<String>)> =
        match stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        }) {
            Ok(r) => r.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                log::warn!("I372: Failed to query enriched emails: {}", e);
                return 0;
            }
        };

    if rows.is_empty() {
        return 0;
    }

    // Build set of email IDs that already have signals to avoid duplicates
    let already_signaled: std::collections::HashSet<String> = db
        .conn_ref()
        .prepare(
            "SELECT DISTINCT json_extract(value, '$.email_id')
             FROM signal_events
             WHERE source = 'email_enrichment'
               AND superseded_by IS NULL
               AND json_extract(value, '$.email_id') IS NOT NULL",
        )
        .and_then(|mut stmt| {
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    let mut emitted = 0usize;

    for (email_id, entity_id, entity_type, sentiment, urgency, subject, sender) in &rows {
        // Skip emails that already have signals emitted
        if already_signaled.contains(email_id) {
            continue;
        }

        let source_context = format!(
            "email:{}:{}",
            sender.as_deref().unwrap_or(""),
            subject.as_deref().unwrap_or("")
        );
        // Truncate source_context to avoid oversized signal values
        let ctx = if source_context.len() > 200 {
            &source_context[..200]
        } else {
            &source_context
        };

        // Emit email_sentiment for non-neutral sentiments
        if let Some(ref s) = sentiment {
            if s != "neutral" {
                let value = serde_json::json!({
                    "email_id": email_id,
                    "sentiment": s,
                    "source_context": ctx,
                })
                .to_string();

                if bus::emit_signal(
                    db,
                    entity_type,
                    entity_id,
                    "email_sentiment",
                    "email_enrichment",
                    Some(&value),
                    0.7,
                )
                .is_ok()
                {
                    emitted += 1;
                }
            }
        }

        // Emit email_commitment when contextual summary contains commitment language (I372 AC2)
        {
            let summary: Option<String> = db.conn_ref()
                .prepare("SELECT contextual_summary FROM emails WHERE email_id = ?1")
                .and_then(|mut s| s.query_row([email_id.as_str()], |row| row.get(0)))
                .ok()
                .flatten();
            if let Some(ref text) = summary {
                let lower = text.to_lowercase();
                let has_commitment = lower.contains("will send")
                    || lower.contains("will provide")
                    || lower.contains("committed to")
                    || lower.contains("confirmed")
                    || lower.contains("agreed to")
                    || lower.contains("by friday")
                    || lower.contains("by monday")
                    || lower.contains("by end of")
                    || lower.contains("deadline")
                    || lower.contains("order form")
                    || lower.contains("contract");
                if has_commitment {
                    let value = serde_json::json!({
                        "email_id": email_id,
                        "source_context": ctx,
                    })
                    .to_string();
                    if bus::emit_signal(
                        db, entity_type, entity_id,
                        "email_commitment", "email_enrichment",
                        Some(&value), 0.65,
                    ).is_ok() {
                        emitted += 1;
                    }
                }
            }
        }

        // Emit email_urgency_high for high-urgency emails
        if urgency.as_deref() == Some("high") {
            let value = serde_json::json!({
                "email_id": email_id,
                "source_context": ctx,
            })
            .to_string();

            if bus::emit_signal(
                db,
                entity_type,
                entity_id,
                "email_urgency_high",
                "email_enrichment",
                Some(&value),
                0.8,
            )
            .is_ok()
            {
                emitted += 1;
            }
        }
    }

    if emitted > 0 {
        log::info!(
            "I372: emitted {} entity signals from {} enriched emails",
            emitted,
            rows.len()
        );
    }

    emitted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    #[test]
    fn test_bridge_no_meetings() {
        let db = test_db();
        let result = run_email_meeting_bridge(&db).expect("bridge");
        assert!(result.is_empty());
    }

    #[test]
    fn test_bridge_with_overlap() {
        let db = test_db();
        let conn = db.conn_ref();

        // Insert a meeting starting in 1 hour with attendees
        conn.execute(
            "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at, attendees)
             VALUES ('m1', 'Sync with Alice', 'customer', datetime('now', '+1 hour'), datetime('now'), 'alice@acme.com, bob@partner.com')",
            [],
        ).expect("insert meeting");

        // Insert an email signal from alice
        conn.execute(
            "INSERT INTO email_signals (email_id, sender_email, entity_id, entity_type, signal_type, signal_text)
             VALUES ('em-1', 'alice@acme.com', 'acc-acme', 'account', 'timeline', 'Q4 numbers look good')",
            [],
        ).expect("insert email signal");

        let correlations = run_email_meeting_bridge(&db).expect("bridge");
        assert_eq!(correlations.len(), 1);
        assert_eq!(correlations[0].meeting_id, "m1");
        assert_eq!(correlations[0].sender_email, "alice@acme.com");

        // Verify signal was emitted
        let signals = bus::get_active_signals(&db, "meeting", "m1").expect("get signals");
        assert!(!signals.is_empty());
        assert_eq!(signals[0].signal_type, "pre_meeting_context");
    }
}

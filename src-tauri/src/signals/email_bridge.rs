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

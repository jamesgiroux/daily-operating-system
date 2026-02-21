//! Pre-meeting email context gathering (ADR-0080).
//!
//! Queries the signal_events table for `pre_meeting_context` signals plus
//! direct email_signals for attendee overlap. Ranks by recency and returns
//! a JSON array suitable for injection into meeting prep context.

use serde_json::{json, Value};

use crate::db::ActionDb;

/// Gather email context relevant to an upcoming meeting.
///
/// Combines:
/// 1. `pre_meeting_context` signals from the email bridge (signal_events)
/// 2. Direct email_signals matching attendee emails
///
/// Returns a JSON array of `{from, snippet, date, relevance, source}`.
pub fn gather_email_context(
    db: &ActionDb,
    attendees: &[String],
    entity_id: &str,
    limit: usize,
) -> Value {
    let mut context_items: Vec<Value> = Vec::new();
    let conn = db.conn_ref();

    // 1. Bridge signals from signal_events
    if let Ok(mut stmt) = conn.prepare(
        "SELECT value, confidence, created_at
         FROM signal_events
         WHERE signal_type = 'pre_meeting_context'
           AND superseded_by IS NULL
           AND created_at >= datetime('now', '-7 days')
         ORDER BY created_at DESC
         LIMIT ?1",
    ) {
        if let Ok(rows) = stmt.query_map([limit as i64], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, String>(2)?,
            ))
        }) {
            for row in rows.flatten() {
                let (value_json, confidence, date) = row;
                if let Some(ref val_str) = value_json {
                    if let Ok(val) = serde_json::from_str::<Value>(val_str) {
                        context_items.push(json!({
                            "from": val.get("sender_email").and_then(|v| v.as_str()).unwrap_or(""),
                            "snippet": val.get("signal_text").and_then(|v| v.as_str()).unwrap_or(""),
                            "date": date,
                            "relevance": confidence,
                            "source": "signal_bus",
                        }));
                    }
                }
            }
        }
    }

    // 2. Direct email_signals for attendee overlap
    if !attendees.is_empty() || !entity_id.is_empty() {
        let email_signals = if !entity_id.is_empty() {
            db.list_recent_email_signals_for_entity(entity_id, limit)
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        for signal in &email_signals {
            let sender = signal.sender_email.as_deref().unwrap_or("");
            let text = &signal.signal_text;
            let date = &signal.detected_at;

            if sender.is_empty() {
                continue;
            }

            // Only include if sender is in attendees or if from entity
            let sender_lower = sender.to_lowercase();
            let relevant = attendees.iter().any(|a| a.to_lowercase() == sender_lower)
                || !entity_id.is_empty();

            if relevant {
                context_items.push(json!({
                    "from": sender,
                    "snippet": text,
                    "date": date,
                    "relevance": 0.7,
                    "source": "email_signals",
                }));
            }
        }
    }

    // Deduplicate by snippet similarity (simple exact match)
    context_items.dedup_by(|a, b| {
        a.get("snippet").and_then(|v| v.as_str()) == b.get("snippet").and_then(|v| v.as_str())
    });

    // Limit results
    context_items.truncate(limit);

    json!(context_items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    #[test]
    fn test_empty_context() {
        let db = test_db();
        let result = gather_email_context(&db, &[], "", 10);
        assert!(result.as_array().unwrap().is_empty());
    }
}

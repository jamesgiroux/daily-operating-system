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

    // 1. Bridge signals from signal_events — scoped to this meeting's entity.
    //
    // Before this filter existed, the query pulled the 8 globally most-recent
    // `pre_meeting_context` signals regardless of whose meeting was being
    // prepped. On a multi-account workspace the recent volume on one account
    // (e.g. a thread with heavy fan-out) drowned out every other meeting's
    // prep — a user with a 1:1 with Josiah would see "6 email threads from
    // randall.benson@coxautoinc.com" injected into the briefing because
    // Randall's AcmeCorp-scoped signals were the most recent rows in the table.
    //
    // Scoping by entity_id (and skipping the query entirely when the meeting
    // has no resolved primary entity) closes the leak.
    if entity_id.is_empty() {
        // No entity → no bridge signals. Skip step 1 entirely; fall through
        // to attendee-overlap email_signals, which has its own entity guard.
    } else if let Ok(mut stmt) = conn.prepare(
        "SELECT value, confidence, created_at
         FROM signal_events
         WHERE signal_type = 'pre_meeting_context'
           AND entity_id = ?1
           AND superseded_by IS NULL
           AND created_at >= datetime('now', '-7 days')
         ORDER BY created_at DESC
         LIMIT ?2",
    ) {
        if let Ok(rows) = stmt.query_map(
            rusqlite::params![entity_id, limit as i64],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        ) {
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

            // Only include if sender is an attendee of this meeting, or if no
            // attendee list was provided (entity-level context, not meeting-level)
            let sender_lower = sender.to_lowercase();
            let relevant =
                attendees.is_empty() || attendees.iter().any(|a| a.to_lowercase() == sender_lower);

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

    /// Regression: pre_meeting_context signals for entity A must not leak
    /// into a meeting prep built for entity B. Before the entity_id filter
    /// was added to the signal_events query, every meeting prep globally
    /// pulled the 8 most-recent pre_meeting_context rows from the last 7
    /// days, causing (e.g.) a 1:1 with Josiah to render Randall's
    /// AcmeCorp-scoped email signals in the briefing digest.
    #[test]
    fn pre_meeting_context_signals_are_entity_scoped() {
        let db = test_db();
        let now = chrono::Utc::now().to_rfc3339();

        // Seed two entities' worth of pre_meeting_context signals.
        let conn = db.conn_ref();
        for (eid, from, text) in [
            ("acmecorp", "randall.benson@example.com", "Randall: AcmeCorp thread"),
            ("acmecorp", "randall.benson@example.com", "Randall: another AcmeCorp thread"),
            ("josiah-wold", "josiah@example.com", "Josiah: 1:1 prep"),
        ] {
            let value = serde_json::json!({
                "sender_email": from,
                "signal_text": text,
            })
            .to_string();
            conn.execute(
                "INSERT INTO signal_events
                 (id, entity_type, entity_id, signal_type, source, value, confidence, decay_half_life_days, created_at)
                 VALUES (?1, 'account', ?2, 'pre_meeting_context', 'test', ?3, 1.0, 7.0, ?4)",
                rusqlite::params![format!("sig-{}-{}", eid, text), eid, value, now],
            )
            .expect("insert signal");
        }

        // Gather for Josiah — must only see Josiah's row.
        let result = gather_email_context(&db, &[], "josiah-wold", 8);
        let arr = result.as_array().expect("array");
        assert_eq!(
            arr.len(),
            1,
            "josiah meeting must not pull acmecorp signals; got {arr:?}"
        );
        assert_eq!(
            arr[0].get("from").and_then(|v| v.as_str()),
            Some("josiah@example.com"),
            "wrong sender leaked into josiah meeting: {arr:?}"
        );

        // Gather for AcmeCorp — must see both AcmeCorp rows.
        let result = gather_email_context(&db, &[], "acmecorp", 8);
        let arr = result.as_array().expect("array");
        assert_eq!(arr.len(), 2, "acmecorp should have 2 signals, got {arr:?}");
    }
}

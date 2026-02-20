//! Post-meeting email correlation (I308 — ADR-0080 Phase 4).
//!
//! Correlates emails received 1-48h after meetings with meeting attendees
//! to detect follow-up threads and extract action items.

use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{ActionDb, DbError};
use crate::helpers;

use super::bus;
use super::propagation::PropagationEngine;

/// A correlation between a post-meeting email and its meeting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMeetingCorrelation {
    pub meeting_id: String,
    pub meeting_title: String,
    pub email_signal_id: String,
    pub sender_email: String,
    pub thread_id: Option<String>,
}

/// Find emails from meeting attendees sent 1-48h after meeting end time.
/// Persists correlations and emits `post_meeting_followup` signals.
pub fn correlate_post_meeting_emails(db: &ActionDb) -> Result<Vec<PostMeetingCorrelation>, String> {
    correlate_post_meeting_emails_with_engine(db, None)
}

/// Like `correlate_post_meeting_emails` but optionally propagates signals via the engine.
pub fn correlate_post_meeting_emails_with_engine(
    db: &ActionDb,
    engine: Option<&PropagationEngine>,
) -> Result<Vec<PostMeetingCorrelation>, String> {
    let conn = db.conn_ref();
    let mut correlations = Vec::new();

    // Get meetings that ended 1-48h ago
    let meetings = db
        .get_recently_ended_meetings(48)
        .map_err(|e| format!("Failed to query recent meetings: {}", e))?;

    if meetings.is_empty() {
        return Ok(correlations);
    }

    // For each meeting, find email signals from attendees within 24h post-meeting
    for (meeting_id, title, end_time, attendees_csv, account_id) in &meetings {
        let attendee_emails = helpers::parse_attendee_emails(attendees_csv);

        if attendee_emails.is_empty() {
            continue;
        }

        // Find email signals from attendees after meeting end
        let mut stmt = conn
            .prepare(
                "SELECT CAST(id AS TEXT), sender_email, signal_text
                 FROM email_signals
                 WHERE detected_at > ?1
                   AND detected_at <= datetime(?1, '+24 hours')
                 ORDER BY detected_at ASC",
            )
            .map_err(|e| format!("Failed to prepare email query: {}", e))?;

        let emails: Vec<(String, String, String)> = stmt
            .query_map(params![end_time], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| format!("Failed to query emails: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        for (email_id, sender_email, _signal_text) in &emails {
            if !attendee_emails.contains(&sender_email.to_lowercase()) {
                continue;
            }

            // Check if already correlated
            let already_exists: bool = conn
                .prepare("SELECT 1 FROM post_meeting_emails WHERE meeting_id = ?1 AND email_signal_id = ?2")
                .and_then(|mut s| s.exists(params![meeting_id, email_id]))
                .unwrap_or(false);

            if already_exists {
                continue;
            }

            // Persist correlation
            let _pme_id = format!("pme-{}", Uuid::new_v4());
            db.insert_post_meeting_email(meeting_id, email_id, None, None)
                .map_err(|e| format!("Failed to insert post-meeting email: {}", e))?;

            // Emit post_meeting_followup signal
            let value = serde_json::json!({
                "meeting_id": meeting_id,
                "meeting_title": title,
                "email_signal_id": email_id,
                "sender_email": sender_email,
                "account_id": account_id,
            })
            .to_string();

            let entity_type = if account_id.is_some() { "account" } else { "meeting" };
            let entity_id = account_id.as_deref().unwrap_or(meeting_id);

            if let Some(eng) = engine {
                let _ = bus::emit_signal_and_propagate(
                    db,
                    eng,
                    entity_type,
                    entity_id,
                    "post_meeting_followup",
                    "email_thread",
                    Some(&value),
                    0.70,
                );
            } else {
                let _ = bus::emit_signal(
                    db,
                    entity_type,
                    entity_id,
                    "post_meeting_followup",
                    "email_thread",
                    Some(&value),
                    0.70,
                );
            }

            correlations.push(PostMeetingCorrelation {
                meeting_id: meeting_id.clone(),
                meeting_title: title.clone(),
                email_signal_id: email_id.clone(),
                sender_email: sender_email.clone(),
                thread_id: None,
            });
        }
    }

    if !correlations.is_empty() {
        log::info!(
            "Post-meeting correlation: {} emails matched to meetings",
            correlations.len(),
        );
    }

    Ok(correlations)
}

// ---------------------------------------------------------------------------
// ActionDb methods
// ---------------------------------------------------------------------------

impl ActionDb {
    /// Get meetings that ended within the last N hours.
    /// Returns (id, title, end_time, attendees, account_id).
    pub fn get_recently_ended_meetings(
        &self,
        hours_ago: i32,
    ) -> Result<Vec<(String, String, String, String, Option<String>)>, DbError> {
        let mut stmt = self.conn_ref().prepare(
            "SELECT mh.id, mh.title,
                    COALESCE(
                        -- Compute end_time from start_time if no explicit end
                        datetime(mh.start_time, '+1 hour'),
                        mh.start_time
                    ) as end_time,
                    COALESCE(mh.attendees, '') as attendees,
                    me.entity_id as account_id
             FROM meetings_history mh
             LEFT JOIN meeting_entities me ON me.meeting_id = mh.id AND me.entity_type = 'account'
             WHERE mh.start_time <= datetime('now', '-1 hour')
               AND mh.start_time >= datetime('now', ?1)
               AND mh.attendees IS NOT NULL AND mh.attendees != ''",
        )?;

        let hours_param = format!("-{} hours", hours_ago);
        let rows = stmt
            .query_map(params![hours_param], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })?;

        let mut meetings = Vec::new();
        for row in rows {
            meetings.push(row?);
        }
        Ok(meetings)
    }

    /// Insert a post-meeting email correlation record.
    pub fn insert_post_meeting_email(
        &self,
        meeting_id: &str,
        email_signal_id: &str,
        thread_id: Option<&str>,
        actions_json: Option<&str>,
    ) -> Result<(), DbError> {
        let id = format!("pme-{}", Uuid::new_v4());
        self.conn_ref().execute(
            "INSERT INTO post_meeting_emails (id, meeting_id, email_signal_id, thread_id, actions_extracted)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, meeting_id, email_signal_id, thread_id, actions_json],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    #[test]
    fn test_correlate_no_meetings() {
        let db = test_db();
        let result = correlate_post_meeting_emails(&db).expect("correlate");
        assert!(result.is_empty());
    }

    #[test]
    fn test_insert_post_meeting_email() {
        let db = test_db();
        db.insert_post_meeting_email("m1", "es-1", Some("thread-1"), None)
            .expect("insert");

        let count: i32 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM post_meeting_emails WHERE meeting_id = 'm1'",
                [],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(count, 1);
    }
}

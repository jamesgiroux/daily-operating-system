//! Email-specific relevance scoring adapter (I395).
//!
//! Maps DbEmail to ScoringContext, applies noise sender/subject penalties,
//! and builds today's meeting context for embedding similarity.

use crate::db::{ActionDb, DbEmail};
use crate::embeddings::EmbeddingModel;

use super::scoring::{score_item, ScoringContext};

/// Sender patterns that indicate automated/noise emails.
const NOISE_SENDERS: &[&str] = &[
    "noreply", "no-reply", "donotreply", "do-not-reply", "comment-reply",
    "notifications@", "mailer-daemon", "drive-shares", "calendar-notification", "notify@",
];

/// Subject prefixes that indicate calendar notifications (not conversations).
const NOISE_SUBJECT_PREFIXES: &[&str] = &[
    "Accepted:", "Declined:", "Tentatively accepted:",
    "Updated invitation:", "Canceled event:", "Invitation:",
];

/// Score a single email. Returns (score, reason).
pub fn score_single_email(
    db: &ActionDb,
    model: Option<&EmbeddingModel>,
    email: &DbEmail,
    meeting_context: &str,
) -> (f64, String) {
    // Check noise sender
    if let Some(sender) = &email.sender_email {
        let lower = sender.to_lowercase();
        if NOISE_SENDERS.iter().any(|pat| lower.contains(pat)) {
            return (0.01, "automated sender".to_string());
        }
    }

    // Check noise subject
    if let Some(subject) = &email.subject {
        if NOISE_SUBJECT_PREFIXES.iter().any(|prefix| subject.starts_with(prefix)) {
            return (0.02, "calendar notification".to_string());
        }
    }

    let content = email.contextual_summary.as_deref().unwrap_or(
        email.snippet.as_deref().unwrap_or("")
    );
    let created = email.received_at.as_deref().unwrap_or(&email.created_at);

    let ctx = ScoringContext {
        entity_id: email.entity_id.as_deref(),
        entity_type: email.entity_type.as_deref(),
        content_text: content,
        urgency: email.urgency.as_deref(),
        sentiment: email.sentiment.as_deref(),
        created_at: created,
    };

    let result = score_item(db, model, &ctx, meeting_context);
    (result.total, result.reason)
}

/// Score all enriched active emails. Returns (email_id, score, reason) tuples.
pub fn score_emails(
    db: &ActionDb,
    model: Option<&EmbeddingModel>,
    emails: &[DbEmail],
) -> Vec<(String, f64, String)> {
    let meeting_context = build_meeting_context(db);

    emails
        .iter()
        .filter(|e| e.enrichment_state == "enriched")
        .map(|email| {
            let (score, reason) = score_single_email(db, model, email, &meeting_context);
            (email.email_id.clone(), score, reason)
        })
        .collect()
}

/// Build today's meeting context string for embedding similarity.
///
/// Queries meetings_history for today's meetings, concatenates titles.
/// Same pattern as callouts.rs build_meeting_context_string.
pub fn build_meeting_context(db: &ActionDb) -> String {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let start = format!("{}T00:00:00", today);
    let end = format!("{}T23:59:59", today);

    let mut stmt = match db.conn_ref().prepare(
        "SELECT title FROM meetings_history WHERE start_time >= ?1 AND start_time <= ?2"
    ) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };

    let titles: Vec<String> = stmt
        .query_map(rusqlite::params![start, end], |row| row.get::<_, String>(0))
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    titles.join(". ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_email(sender: &str, subject: &str) -> DbEmail {
        DbEmail {
            email_id: "test-1".to_string(),
            thread_id: None,
            sender_email: Some(sender.to_string()),
            sender_name: Some("Test".to_string()),
            subject: Some(subject.to_string()),
            snippet: Some("test snippet".to_string()),
            priority: Some("medium".to_string()),
            is_unread: true,
            received_at: Some(chrono::Utc::now().to_rfc3339()),
            enrichment_state: "enriched".to_string(),
            enrichment_attempts: 1,
            last_enrichment_at: None,
            last_seen_at: None,
            resolved_at: None,
            entity_id: None,
            entity_type: None,
            contextual_summary: Some("Test summary about renewal discussion".to_string()),
            sentiment: None,
            urgency: None,
            user_is_last_sender: false,
            last_sender_email: None,
            message_count: 1,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            relevance_score: None,
            score_reason: None,
        }
    }

    #[test]
    fn test_noise_sender_gets_low_score() {
        let db = crate::db::test_utils::test_db();
        let email = make_test_email("noreply@company.com", "Your report is ready");
        let (score, reason) = score_single_email(&db, None, &email, "");
        assert!(score < 0.05, "noise sender should score near zero, got {}", score);
        assert_eq!(reason, "automated sender");
    }

    #[test]
    fn test_calendar_notification_gets_low_score() {
        let db = crate::db::test_utils::test_db();
        let email = make_test_email("alice@company.com", "Accepted: Weekly standup");
        let (score, reason) = score_single_email(&db, None, &email, "");
        assert!(score < 0.05, "calendar notification should score near zero, got {}", score);
        assert_eq!(reason, "calendar notification");
    }

    #[test]
    fn test_normal_email_scores_above_noise() {
        let db = crate::db::test_utils::test_db();
        let email = make_test_email("alice@customer.com", "Re: Contract renewal discussion");
        let (score, _reason) = score_single_email(&db, None, &email, "");
        assert!(score > 0.05, "normal email with keyword should score above noise, got {}", score);
    }
}

//! Email priority classification (ported from ops/email_fetch.py).
//!
//! 3-tier classification: high, medium, low.
//! Gmail fetch lives in google_api/gmail.rs — this module only classifies.

use std::collections::HashSet;

use super::constants::{
    BULK_SENDER_DOMAINS, HIGH_PRIORITY_SUBJECT_KEYWORDS, LOW_PRIORITY_SIGNALS, NOREPLY_LOCAL_PARTS,
};

/// Extract bare email from a "From" header like "Name <email@example.com>".
pub fn extract_email_address(from_field: &str) -> String {
    if let Some(start) = from_field.find('<') {
        if let Some(end) = from_field.find('>') {
            if end > start {
                return from_field[start + 1..end].to_lowercase();
            }
        }
    }
    from_field.trim().to_lowercase()
}

/// Extract the display name from a "From" header like "Jane Doe <jane@customer.com>".
///
/// Returns `Some("Jane Doe")` if a display name is present, `None` for bare emails.
/// Handles quoted display names: `"Jane Doe" <jane@customer.com>`.
pub fn extract_display_name(from_field: &str) -> Option<String> {
    let trimmed = from_field.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Must have angle brackets to have a display name
    let angle_start = trimmed.find('<')?;
    if angle_start == 0 {
        return None; // "<email>" with no name prefix
    }

    let name_part = trimmed[..angle_start].trim();
    if name_part.is_empty() {
        return None;
    }

    // Strip surrounding quotes
    let name = name_part.trim_matches('"').trim();
    if name.is_empty() {
        return None;
    }

    // Reject if the "name" is actually an email address
    if name.contains('@') {
        return None;
    }

    // Must have at least two words to be a real name (not "Jdoe")
    if !name.contains(' ') {
        return None;
    }

    Some(name.to_string())
}

/// Extract domain from an email address.
pub fn extract_domain(email_addr: &str) -> String {
    if let Some(at_pos) = email_addr.rfind('@') {
        email_addr[at_pos + 1..].to_lowercase()
    } else {
        String::new()
    }
}

/// Classify email priority: "high", "medium", or "low".
///
/// High: from customer domains, from known account domains, or subject
///       contains urgency keywords.
/// Medium: from internal colleagues, or meeting-related.
/// Low: newsletters, automated, GitHub notifications.
pub fn classify_email_priority(
    from_raw: &str,
    subject: &str,
    list_unsubscribe: &str,
    precedence: &str,
    customer_domains: &HashSet<String>,
    user_domain: &str,
    account_hints: &HashSet<String>,
) -> &'static str {
    classify_email_priority_with_extras(
        from_raw,
        subject,
        list_unsubscribe,
        precedence,
        customer_domains,
        user_domain,
        account_hints,
        &[],
    )
}

/// Classify email priority with optional role-preset keywords.
#[allow(clippy::too_many_arguments)]
pub fn classify_email_priority_with_extras(
    from_raw: &str,
    subject: &str,
    list_unsubscribe: &str,
    precedence: &str,
    customer_domains: &HashSet<String>,
    user_domain: &str,
    account_hints: &HashSet<String>,
    extra_high_keywords: &[String],
) -> &'static str {
    let from_addr = extract_email_address(from_raw);
    let domain = extract_domain(&from_addr);
    let subject_lower = subject.to_lowercase();

    // HIGH: Customer domains (from today's meeting attendees)
    if customer_domains.contains(&domain) {
        return "high";
    }

    // HIGH: Sender domain matches a known customer account
    if !account_hints.is_empty() && !domain.is_empty() {
        let domain_base = domain.split('.').next().unwrap_or("");
        for hint in account_hints {
            if hint == domain_base || (hint.len() >= 4 && domain_base.contains(hint.as_str())) {
                return "high";
            }
        }
    }

    // HIGH: Urgency + business keywords in subject (hardcoded base list)
    if HIGH_PRIORITY_SUBJECT_KEYWORDS
        .iter()
        .any(|kw| subject_lower.contains(kw))
    {
        return "high";
    }

    // HIGH: Role-preset keywords in subject (I313)
    if extra_high_keywords
        .iter()
        .any(|kw| subject_lower.contains(&kw.to_lowercase()))
    {
        return "high";
    }

    // LOW: Newsletters, automated, GitHub
    let from_lower = from_raw.to_lowercase();
    if LOW_PRIORITY_SIGNALS
        .iter()
        .any(|signal| from_lower.contains(signal) || subject_lower.contains(signal))
    {
        return "low";
    }
    if domain.contains("github.com") {
        return "low";
    }

    // LOW: List-Unsubscribe header present (bulk/marketing mail) — I21
    if !list_unsubscribe.is_empty() {
        return "low";
    }

    // LOW: Precedence: bulk or list — I21
    let precedence_lower = precedence.to_lowercase();
    if precedence_lower == "bulk" || precedence_lower == "list" {
        return "low";
    }

    // LOW: Sender domain is a known bulk/marketing sender — I21
    if BULK_SENDER_DOMAINS.contains(&domain.as_str()) {
        return "low";
    }

    // LOW: Noreply local-part (checked AFTER customer/account domain) — I21
    if let Some(at_pos) = from_addr.find('@') {
        let local_part = &from_addr[..at_pos];
        if NOREPLY_LOCAL_PARTS.contains(&local_part) {
            return "low";
        }
    }

    // MEDIUM: Internal colleagues
    if !user_domain.is_empty() && domain == user_domain {
        return "medium";
    }

    // MEDIUM: Meeting-related
    if ["meeting", "calendar", "invite"]
        .iter()
        .any(|kw| subject_lower.contains(kw))
    {
        return "medium";
    }

    "medium"
}

// ============================================================================
// I320: Signal-context boosting (Layer 1 of hybrid classification)
// ============================================================================

/// High-confidence signal types that warrant boosting a medium email to high.
const BOOST_SIGNAL_TYPES: &[&str] = &[
    "renewal_approaching",
    "engagement_warning",
    "champion_risk",
    "churn_risk",
    "escalation",
    "expansion_opportunity",
    "cadence_anomaly",
    "project_health_warning",
];

/// A boost result explaining why an email was elevated.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoostResult {
    pub entity_id: String,
    pub entity_type: String,
    pub signal_type: String,
    pub reason: String,
}

/// After mechanical classification, check if the email's sender resolves to
/// an entity with active high-confidence signals. If so, boost "medium" → "high".
///
/// Returns `Some(BoostResult)` if the email should be boosted, `None` otherwise.
/// Only operates on "medium" priority emails — high stays high, low stays low.
pub fn boost_with_entity_context(
    from_email: &str,
    current_priority: &str,
    db: &crate::db::ActionDb,
) -> Option<BoostResult> {
    if current_priority != "medium" {
        return None;
    }

    // Find entity via email_signals (sender_email → entity mapping)
    let conn = db.conn_ref();
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT entity_id, entity_type
             FROM email_signals
             WHERE sender_email = ?1
             ORDER BY detected_at DESC
             LIMIT 5",
        )
        .ok()?;

    let entity_matches: Vec<(String, String)> = stmt
        .query_map(rusqlite::params![from_email], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .ok()?
        .filter_map(|r| r.ok())
        .collect();

    // Check each matched entity for active high-confidence signals
    for (entity_id, entity_type) in &entity_matches {
        let mut sig_stmt = match conn.prepare(
            "SELECT signal_type, value, confidence
             FROM signal_events
             WHERE entity_id = ?1 AND entity_type = ?2
               AND created_at >= datetime('now', '-30 days')
               AND confidence >= 0.6
             ORDER BY created_at DESC
             LIMIT 20",
        ) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let signals: Vec<(String, Option<String>, f64)> = sig_stmt
            .query_map(rusqlite::params![entity_id, entity_type], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, f64>(2)?,
                ))
            })
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|r| r.ok())
            .collect();

        for (signal_type, value, confidence) in &signals {
            if BOOST_SIGNAL_TYPES.contains(&signal_type.as_str()) {
                let reason = match value {
                    Some(v) => format!(
                        "elevated: {} (confidence {:.0}%)",
                        v,
                        confidence * 100.0
                    ),
                    None => format!(
                        "elevated: {} (confidence {:.0}%)",
                        signal_type,
                        confidence * 100.0
                    ),
                };
                return Some(BoostResult {
                    entity_id: entity_id.clone(),
                    entity_type: entity_type.clone(),
                    signal_type: signal_type.clone(),
                    reason,
                });
            }
        }
    }

    None
}

// ============================================================================
// I357: Semantic email reclassification (opt-in AI re-scoring)
// ============================================================================

/// Result of AI reclassification for a single email.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReclassResult {
    pub email_id: String,
    pub original_priority: String,
    pub new_priority: String,
    pub reason: String,
}

/// AI-reclassify medium-priority emails that may be mis-classified.
///
/// Requires feature flag `semanticEmailReclass` (default false).
/// Uses the extraction-tier PtyManager with a 60s timeout.
/// Returns a list of reclassification results for emails whose priority changed.
pub fn reclassify_with_ai(
    emails: &[serde_json::Value],
    pty: &crate::pty::PtyManager,
    workspace: &std::path::Path,
) -> Vec<ReclassResult> {
    let medium_emails: Vec<&serde_json::Value> = emails
        .iter()
        .filter(|e| e.get("priority").and_then(|v| v.as_str()) == Some("medium"))
        .collect();

    if medium_emails.is_empty() {
        return Vec::new();
    }

    // Cap at 15 emails to keep prompt concise
    let batch: Vec<&&serde_json::Value> = medium_emails.iter().take(15).collect();

    let mut context = String::new();
    for email in &batch {
        let id = email.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let from = email.get("sender").and_then(|v| v.as_str()).unwrap_or("?");
        let subject = email.get("subject").and_then(|v| v.as_str()).unwrap_or("?");
        let snippet = email.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
        context.push_str(&format!(
            "ID: {}\nFrom: {}\nSubject: {}\nSnippet: {}\n\n",
            id, from, subject, snippet
        ));
    }

    let prompt = format!(
        "You are re-classifying email priority. Each email below is currently classified as \
         \"medium\" priority. Re-evaluate whether it should be \"high\", \"medium\", or \"low\" \
         based on business importance, urgency signals, and sender relevance.\n\n\
         Only output emails whose priority should CHANGE. Skip emails that should stay medium.\n\n\
         Format:\n\
         RECLASS:email-id\n\
         PRIORITY: high|low\n\
         REASON: <brief explanation>\n\
         END_RECLASS\n\n\
         {}",
        context
    );

    let output = match pty.spawn_claude(workspace, &prompt) {
        Ok(o) => o,
        Err(e) => {
            log::warn!("reclassify_with_ai: Claude invocation failed: {}", e);
            return Vec::new();
        }
    };

    parse_reclassification(&output.stdout)
}

/// Parse Claude's reclassification response.
fn parse_reclassification(response: &str) -> Vec<ReclassResult> {
    let mut results = Vec::new();
    let mut current_id: Option<String> = None;
    let mut priority: Option<String> = None;
    let mut reason: Option<String> = None;

    for line in response.lines() {
        let trimmed = line.trim();

        if let Some(id) = trimmed.strip_prefix("RECLASS:") {
            current_id = Some(id.trim().to_string());
            priority = None;
            reason = None;
        } else if trimmed == "END_RECLASS" {
            if let (Some(id), Some(pri)) = (current_id.take(), priority.take()) {
                if pri == "high" || pri == "low" {
                    results.push(ReclassResult {
                        email_id: id,
                        original_priority: "medium".to_string(),
                        new_priority: pri,
                        reason: reason.take().unwrap_or_default(),
                    });
                }
            }
            current_id = None;
            priority = None;
            reason = None;
        } else if current_id.is_some() {
            if let Some(val) = trimmed.strip_prefix("PRIORITY:") {
                priority = Some(val.trim().to_lowercase());
            } else if let Some(val) = trimmed.strip_prefix("REASON:") {
                reason = Some(val.trim().to_string());
            }
        }
    }

    results
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_email_address_angle_brackets() {
        assert_eq!(
            extract_email_address("Jane Doe <jane@customer.com>"),
            "jane@customer.com"
        );
    }

    #[test]
    fn test_extract_email_address_bare() {
        assert_eq!(
            extract_email_address("  JANE@CUSTOMER.COM  "),
            "jane@customer.com"
        );
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("jane@customer.com"), "customer.com");
        assert_eq!(extract_domain("nodomain"), "");
    }

    #[test]
    fn test_high_customer_domain() {
        let mut customer = HashSet::new();
        customer.insert("customer.com".to_string());

        assert_eq!(
            classify_email_priority(
                "Jane <jane@customer.com>",
                "Hello",
                "",
                "",
                &customer,
                "myco.com",
                &HashSet::new(),
            ),
            "high"
        );
    }

    #[test]
    fn test_high_account_hint() {
        let hints: HashSet<String> = ["acmecorp"].iter().map(|s| s.to_string()).collect();

        assert_eq!(
            classify_email_priority(
                "support@acmecorp.com",
                "Hello",
                "",
                "",
                &HashSet::new(),
                "myco.com",
                &hints,
            ),
            "high"
        );
    }

    #[test]
    fn test_high_urgency_keywords() {
        assert_eq!(
            classify_email_priority(
                "bob@random.com",
                "URGENT: please review",
                "",
                "",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
            ),
            "high"
        );
    }

    #[test]
    fn test_low_newsletter() {
        assert_eq!(
            classify_email_priority(
                "newsletter@news.com",
                "Weekly Digest",
                "",
                "",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
            ),
            "low"
        );
    }

    #[test]
    fn test_low_github() {
        assert_eq!(
            classify_email_priority(
                "notifications@github.com",
                "PR #42 merged",
                "",
                "",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
            ),
            "low"
        );
    }

    #[test]
    fn test_low_list_unsubscribe() {
        assert_eq!(
            classify_email_priority(
                "info@someshop.com",
                "Sale!",
                "<https://example.com/unsub>",
                "",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
            ),
            "low"
        );
    }

    #[test]
    fn test_low_precedence_bulk() {
        assert_eq!(
            classify_email_priority(
                "updates@someco.com",
                "Monthly update",
                "",
                "bulk",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
            ),
            "low"
        );
    }

    #[test]
    fn test_low_bulk_sender_domain() {
        assert_eq!(
            classify_email_priority(
                "campaign@mailchimp.com",
                "Your campaign",
                "",
                "",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
            ),
            "low"
        );
    }

    #[test]
    fn test_low_noreply() {
        assert_eq!(
            classify_email_priority(
                "noreply@someapp.com",
                "Your receipt",
                "",
                "",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
            ),
            "low"
        );
    }

    #[test]
    fn test_medium_internal() {
        assert_eq!(
            classify_email_priority(
                "colleague@myco.com",
                "Quick question",
                "",
                "",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
            ),
            "medium"
        );
    }

    #[test]
    fn test_medium_meeting_related() {
        assert_eq!(
            classify_email_priority(
                "stranger@external.com",
                "Meeting invite: Sync call",
                "",
                "",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
            ),
            "medium"
        );
    }

    #[test]
    fn test_medium_default() {
        assert_eq!(
            classify_email_priority(
                "someone@unknown.com",
                "Hello there",
                "",
                "",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
            ),
            "medium"
        );
    }

    // --- extract_display_name tests ---

    #[test]
    fn test_display_name_angle_brackets() {
        assert_eq!(
            extract_display_name("Jane Doe <jane@customer.com>"),
            Some("Jane Doe".to_string())
        );
    }

    #[test]
    fn test_display_name_quoted() {
        assert_eq!(
            extract_display_name("\"Jane Doe\" <jane@customer.com>"),
            Some("Jane Doe".to_string())
        );
    }

    #[test]
    fn test_display_name_bare_email() {
        assert_eq!(extract_display_name("jane@customer.com"), None);
    }

    #[test]
    fn test_display_name_empty() {
        assert_eq!(extract_display_name(""), None);
    }

    #[test]
    fn test_display_name_single_word() {
        // Single word names (e.g. "Jdoe <jdoe@co.com>") are not real names
        assert_eq!(extract_display_name("Jdoe <jdoe@co.com>"), None);
    }

    #[test]
    fn test_display_name_email_in_name() {
        assert_eq!(extract_display_name("jane@co.com <jane@co.com>"), None);
    }

    #[test]
    fn test_display_name_angle_only() {
        assert_eq!(extract_display_name("<jane@co.com>"), None);
    }

    #[test]
    fn test_parse_reclassification() {
        let response = "\
RECLASS:msg-1
PRIORITY: high
REASON: Contains urgent escalation from VP
END_RECLASS

RECLASS:msg-2
PRIORITY: low
REASON: Automated notification, no action needed
END_RECLASS
";
        let results = super::parse_reclassification(response);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].email_id, "msg-1");
        assert_eq!(results[0].new_priority, "high");
        assert!(results[0].reason.contains("escalation"));
        assert_eq!(results[1].email_id, "msg-2");
        assert_eq!(results[1].new_priority, "low");
    }

    #[test]
    fn test_parse_reclassification_skips_medium() {
        let response = "\
RECLASS:msg-1
PRIORITY: medium
REASON: Should stay medium
END_RECLASS
";
        let results = super::parse_reclassification(response);
        assert_eq!(results.len(), 0, "medium→medium should be filtered out");
    }

    #[test]
    fn test_parse_reclassification_empty() {
        let results = super::parse_reclassification("No reclassifications needed.");
        assert!(results.is_empty());
    }
}

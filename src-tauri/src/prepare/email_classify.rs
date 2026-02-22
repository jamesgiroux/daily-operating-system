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

/// Urgency keywords that override dismissal penalty (I374).
const URGENCY_OVERRIDE_KEYWORDS: &[&str] = &[
    "urgent", "asap", "critical", "deadline", "emergency", "immediately",
];

/// Classify email priority with optional role-preset keywords and dismissal penalty.
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
    classify_email_priority_full(
        from_raw,
        subject,
        list_unsubscribe,
        precedence,
        customer_domains,
        user_domain,
        account_hints,
        extra_high_keywords,
        &HashSet::new(),
    )
}

/// Full classifier with dismissal learning (I374).
///
/// `dismissed_domains` contains sender domains with >= threshold dismissals.
/// Emails from these domains are downgraded one priority tier unless the
/// subject contains urgency keywords.
#[allow(clippy::too_many_arguments)]
pub fn classify_email_priority_full(
    from_raw: &str,
    subject: &str,
    list_unsubscribe: &str,
    precedence: &str,
    customer_domains: &HashSet<String>,
    user_domain: &str,
    account_hints: &HashSet<String>,
    extra_high_keywords: &[String],
    dismissed_domains: &HashSet<String>,
) -> &'static str {
    let from_addr = extract_email_address(from_raw);
    let domain = extract_domain(&from_addr);
    let subject_lower = subject.to_lowercase();

    // HIGH: Customer domains (from today's meeting attendees)
    if customer_domains.contains(&domain) {
        return apply_dismissal_penalty("high", &domain, &subject_lower, dismissed_domains);
    }

    // HIGH: Sender domain matches a known customer account
    if !account_hints.is_empty() && !domain.is_empty() {
        let domain_base = domain.split('.').next().unwrap_or("");
        for hint in account_hints {
            if hint == domain_base || (hint.len() >= 4 && domain_base.contains(hint.as_str())) {
                return apply_dismissal_penalty("high", &domain, &subject_lower, dismissed_domains);
            }
        }
    }

    // HIGH: Urgency + business keywords in subject (hardcoded base list)
    if HIGH_PRIORITY_SUBJECT_KEYWORDS
        .iter()
        .any(|kw| subject_lower.contains(kw))
    {
        // Urgency keywords override dismissal penalty — return high directly
        return "high";
    }

    // HIGH: Role-preset keywords in subject (I313)
    if extra_high_keywords
        .iter()
        .any(|kw| subject_lower.contains(&kw.to_lowercase()))
    {
        return apply_dismissal_penalty("high", &domain, &subject_lower, dismissed_domains);
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
        return apply_dismissal_penalty("medium", &domain, &subject_lower, dismissed_domains);
    }

    // MEDIUM: Meeting-related
    if ["meeting", "calendar", "invite"]
        .iter()
        .any(|kw| subject_lower.contains(kw))
    {
        return apply_dismissal_penalty("medium", &domain, &subject_lower, dismissed_domains);
    }

    apply_dismissal_penalty("medium", &domain, &subject_lower, dismissed_domains)
}

/// Apply dismissal penalty: downgrade one tier if domain is in dismissed set,
/// unless subject contains urgency keywords (I374).
fn apply_dismissal_penalty(
    base_priority: &'static str,
    domain: &str,
    subject_lower: &str,
    dismissed_domains: &HashSet<String>,
) -> &'static str {
    if dismissed_domains.is_empty() || !dismissed_domains.contains(domain) {
        return base_priority;
    }

    // Urgency keywords override dismissal penalty
    if URGENCY_OVERRIDE_KEYWORDS
        .iter()
        .any(|kw| subject_lower.contains(kw))
    {
        return base_priority;
    }

    // Downgrade one tier
    match base_priority {
        "high" => "medium",
        "medium" => "low",
        _ => base_priority, // low stays low
    }
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

    // --- I374: Dismissal penalty tests ---

    #[test]
    fn test_dismissal_penalty_downgrades_medium_to_low() {
        let mut dismissed = HashSet::new();
        dismissed.insert("spammy.com".to_string());

        assert_eq!(
            classify_email_priority_full(
                "someone@spammy.com",
                "Hello there",
                "",
                "",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
                &[],
                &dismissed,
            ),
            "low"
        );
    }

    #[test]
    fn test_dismissal_penalty_downgrades_high_to_medium() {
        let mut customer = HashSet::new();
        customer.insert("spammy.com".to_string());
        let mut dismissed = HashSet::new();
        dismissed.insert("spammy.com".to_string());

        assert_eq!(
            classify_email_priority_full(
                "jane@spammy.com",
                "Hello",
                "",
                "",
                &customer,
                "myco.com",
                &HashSet::new(),
                &[],
                &dismissed,
            ),
            "medium"
        );
    }

    #[test]
    fn test_dismissal_penalty_skipped_for_urgency() {
        let mut dismissed = HashSet::new();
        dismissed.insert("spammy.com".to_string());

        // "urgent" in subject overrides dismissal penalty
        assert_eq!(
            classify_email_priority_full(
                "someone@spammy.com",
                "URGENT: please respond",
                "",
                "",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
                &[],
                &dismissed,
            ),
            "high"
        );
    }

    #[test]
    fn test_dismissal_penalty_no_effect_without_domain() {
        // Domain not in dismissed set — no penalty
        let mut dismissed = HashSet::new();
        dismissed.insert("other.com".to_string());

        assert_eq!(
            classify_email_priority_full(
                "someone@unknown.com",
                "Hello there",
                "",
                "",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
                &[],
                &dismissed,
            ),
            "medium"
        );
    }

    #[test]
    fn test_dismissal_penalty_empty_set_no_effect() {
        assert_eq!(
            classify_email_priority_full(
                "someone@unknown.com",
                "Hello there",
                "",
                "",
                &HashSet::new(),
                "myco.com",
                &HashSet::new(),
                &[],
                &HashSet::new(),
            ),
            "medium"
        );
    }
}

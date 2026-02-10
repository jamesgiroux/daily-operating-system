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

    // HIGH: Urgency keywords in subject
    if HIGH_PRIORITY_SUBJECT_KEYWORDS
        .iter()
        .any(|kw| subject_lower.contains(kw))
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
        assert_eq!(
            extract_display_name("jane@co.com <jane@co.com>"),
            None
        );
    }

    #[test]
    fn test_display_name_angle_only() {
        assert_eq!(extract_display_name("<jane@co.com>"), None);
    }
}

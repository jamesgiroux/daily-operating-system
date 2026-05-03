//! Email priority classification (ported from ops/email_fetch.py).
//!
//! 3-tier classification: high, medium, low.
//! Gmail fetch lives in google_api/gmail.rs — this module only classifies.

use std::collections::HashSet;

use super::constants::{
    BULK_SENDER_DOMAINS, HIGH_PRIORITY_SUBJECT_KEYWORDS, LOW_PRIORITY_SIGNALS,
    NOISE_SUBJECT_PATTERNS, NOREPLY_LOCAL_PARTS,
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

/// Urgency keywords that override dismissal penalty.
const URGENCY_OVERRIDE_KEYWORDS: &[&str] = &[
    "urgent",
    "asap",
    "critical",
    "deadline",
    "emergency",
    "immediately",
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

/// Full classifier with dismissal learning.
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

    // HIGH: Role-preset keywords in subject
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

    // LOW: List-Unsubscribe header present (bulk/marketing mail) —
    if !list_unsubscribe.is_empty() {
        return "low";
    }

    // LOW: Precedence: bulk or list —
    let precedence_lower = precedence.to_lowercase();
    if precedence_lower == "bulk" || precedence_lower == "list" {
        return "low";
    }

    // LOW: Sender domain is a known bulk/marketing sender —
    if BULK_SENDER_DOMAINS.contains(&domain.as_str()) {
        return "low";
    }

    // LOW: Noreply local-part (checked AFTER customer/account domain) —
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
/// unless subject contains urgency keywords.
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
// Signal-context boosting (Layer 1 of hybrid classification)
// ============================================================================

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
/// `merged_email_signal_types` is the pre-merged list of signal type strings from
/// `AppState::get_merged_signal_config.email_signal_types`. Callers
/// must pass this list — this function does not reach into global state.
///
/// Returns `Some(BoostResult)` if the email should be boosted, `None` otherwise.
/// Only operates on "medium" priority emails — high stays high, low stays low.
pub fn boost_with_entity_context(
    from_email: &str,
    current_priority: &str,
    db: &crate::db::ActionDb,
    merged_email_signal_types: &[String],
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
            if merged_email_signal_types
                .iter()
                .any(|t| t == signal_type)
            {
                let reason = match value {
                    Some(v) => format!("elevated: {} (confidence {:.0}%)", v, confidence * 100.0),
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
// Hard-drop noise suppression
// ============================================================================

/// Decide whether an inbound email should be suppressed entirely
/// (hidden from inbox, account Records, signal emission).
///
/// Suppress when ANY of the following hold:
/// 1. Sender domain is in `BULK_SENDER_DOMAINS` (LinkedIn, Slack, GitHub
///    notifications, etc.) — these are noise regardless of List-Unsubscribe.
/// 2. Subject matches any `NOISE_SUBJECT_PATTERNS` entry — automated
///    receipts, digests, verification codes, etc.
/// 3. `List-Unsubscribe` header present AND another automation marker is
///    also present (noreply local-part, "newsletter" in sender, or noisy
///    subject pattern). List-Unsubscribe alone over-fires —
///    legitimate 1:1 customer email sent via Salesforce/HubSpot/Outreach
///    or Google Groups carries this header. Require corroboration.
///
/// `account_domains` are lowercase customer domains from `account_domains`
/// table — domain-level match (any email from a customer's domain is spared).
/// `person_emails_full` are lowercase EXACT email addresses from
/// `person_emails` + `people.email`. matching on full address (not
/// domain) prevents internal-org bulk notifications (e.g.
/// `no-reply@gainsightapp.com`, `notifications@wordpress.com`) from getting
/// a free pass just because a colleague's address shares the domain.
pub fn should_suppress_email(
    sender: &str,
    subject: &str,
    list_unsubscribe: &str,
    account_domains: &HashSet<String>,
    person_emails_full: &HashSet<String>,
) -> bool {
    let from_addr = extract_email_address(sender);
    let domain = extract_domain(&from_addr);

    // Customer correspondence (domain in account_domains) and known
    // 1:1 contacts (exact email in person_emails) are never suppressed.
    // the previous check used domain-only match against
    // person_emails, which let any noreply@<colleague-domain> through.
    if !domain.is_empty() && account_domains.contains(&domain) {
        return false;
    }
    if !from_addr.is_empty() && person_emails_full.contains(&from_addr) {
        return false;
    }

    // Rule 1: bulk/SaaS-notification sender domain → suppress.
    if !domain.is_empty() && BULK_SENDER_DOMAINS.contains(&domain.as_str()) {
        return true;
    }

    // Rule 2: subject signals automation/transactional/digest mail.
    let subject_lower = subject.to_lowercase();
    let subject_noisy = NOISE_SUBJECT_PATTERNS
        .iter()
        .any(|pat| subject_lower.contains(pat));
    if subject_noisy {
        return true;
    }

    // Rule 2b: noreply local-part is bulk by definition.
    // Suppress unconditionally — these are never 1:1 correspondence.
    let local_part = from_addr.split('@').next().unwrap_or("");
    let local_part_noisy = NOREPLY_LOCAL_PARTS
        .iter()
        .any(|pat| local_part.contains(pat));
    if local_part_noisy {
        return true;
    }

    // List-Unsubscribe heuristic is intentionally dropped.
    // The AI enrichment pass (`prepare/email_enrich.rs`) judges noise
    // for everything that survives the deterministic rules above. The
    // LLM has the full body context and can distinguish a genuine
    // customer reply (List-Unsubscribe present, but real 1:1) from a
    // marketing blast much more reliably than substring matching on
    // the sender header.
    let _ = list_unsubscribe;

    false
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

    // --- Dismissal penalty tests ---

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

    // --- should_suppress_email tests ---

    #[test]
    fn test_suppress_bulk_sender_linkedin() {
        assert!(should_suppress_email(
            "LinkedIn <invitations@linkedin.com>",
            "You have a new connection",
            "",
            &HashSet::new(),
            &HashSet::new(),
        ));
    }

    #[test]
    fn test_suppress_subject_pattern_receipt() {
        assert!(should_suppress_email(
            "billing@somesaas.example",
            "Your receipt from SomeSaaS",
            "",
            &HashSet::new(),
            &HashSet::new(),
        ));
    }

    #[test]
    fn dos_249_no_deterministic_suppress_for_list_unsubscribe_alone() {
        // List-Unsubscribe heuristic dropped from the
        // deterministic pre-filter — LLM enrichment now classifies
        // these. should_suppress_email returns false; the email
        // enters the enrichment pipeline where Claude judges noise.
        assert!(!should_suppress_email(
            "Contact <hello@randomvendor.example>",
            "New product launch",
            "<https://example.com/unsub>",
            &HashSet::new(),
            &HashSet::new(),
        ));
    }

    #[test]
    fn test_no_suppress_account_domain_even_with_unsubscribe() {
        // Real customer correspondence with an inadvertent unsubscribe footer
        // (e.g. routed through a corporate ESP) must NOT be suppressed.
        let mut accounts = HashSet::new();
        accounts.insert("subsidiary.com".to_string());
        assert!(!should_suppress_email(
            "Jane <jane@subsidiary.com>",
            "Re: contract review",
            "<https://corp.example/unsub>",
            &accounts,
            &HashSet::new(),
        ));
    }

    #[test]
    fn test_no_suppress_person_exact_email_bulk_sender_overridden() {
        // matching is now exact-email (not domain). A tracked
        // contact at a bulk-sender domain still gets through.
        let mut persons = HashSet::new();
        persons.insert("alex@github.com".to_string());
        assert!(!should_suppress_email(
            "Alex <alex@github.com>",
            "Re: PR review",
            "",
            &HashSet::new(),
            &persons,
        ));
    }

    #[test]
    fn dos_248_suppress_internal_org_noreply_when_colleague_same_domain() {
        // Even though a colleague (alice@automattic.com) is tracked,
        // notifications@automattic.com is NOT spared. Domain-only match
        // (the pre- behavior) would have let this through.
        let mut persons = HashSet::new();
        persons.insert("alice@automattic.com".to_string());
        assert!(should_suppress_email(
            "Thursday Updates <noreply@automattic.com>",
            "[New post] Systems Update",
            "<https://automattic.com/unsub>",
            &HashSet::new(),
            &persons,
        ));
    }

    #[test]
    fn dos_248_suppress_gainsight_noreply() {
        // Realistic CSP-tool notification — should be suppressed even
        // though gainsightapp.com isn't on the bulk allow-list.
        assert!(should_suppress_email(
            "Gainsight <no-reply@gainsightapp.com>",
            "Renan Basteris added Activity",
            "",
            &HashSet::new(),
            &HashSet::new(),
        ));
    }

    #[test]
    fn dos_248_suppress_internal_blog_post_pattern() {
        // [New post] / [New mention] / [WPVIP] subject patterns should
        // catch internal-org distribution-list noise.
        assert!(should_suppress_email(
            "VIP Accounts Org <notifications@example.org>",
            "[New post] Customer Contact Lookup",
            "",
            &HashSet::new(),
            &HashSet::new(),
        ));
    }

    #[test]
    fn dos_248_suppress_registration_confirmation() {
        assert!(should_suppress_email(
            "Forrester Events <events@forrester.example>",
            "Registration Confirmed: B2B Summit North America 2026",
            "",
            &HashSet::new(),
            &HashSet::new(),
        ));
    }

    #[test]
    fn test_no_suppress_normal_external_email() {
        assert!(!should_suppress_email(
            "Bob <bob@unknown.example>",
            "Quick question",
            "",
            &HashSet::new(),
            &HashSet::new(),
        ));
    }

    #[test]
    fn dos_247_no_suppress_real_person_with_list_unsubscribe() {
        // Real 1:1 customer email sent via Salesforce / HubSpot / Outreach
        // / Google Groups carries List-Unsubscribe but is not noise.
        // Untracked domain (no account_domains/person_domains entry) — must
        // still not be suppressed when the sender looks like a real person.
        assert!(!should_suppress_email(
            "Jane Smith <jane.smith@prospect.example>",
            "Re: pricing question",
            "<https://prospect.example/unsubscribe?id=abc>",
            &HashSet::new(),
            &HashSet::new(),
        ));
    }

    #[test]
    fn dos_247_suppress_noreply_with_list_unsubscribe() {
        // noreply local-part + List-Unsubscribe = bona fide bulk mail, suppress.
        assert!(should_suppress_email(
            "Acme <noreply@acmeapp.example>",
            "Your weekly summary is ready",
            "<https://acmeapp.example/unsub>",
            &HashSet::new(),
            &HashSet::new(),
        ));
    }

    #[test]
    fn dos_247_no_suppress_internal_distribution_with_list_unsubscribe() {
        // Google Workspace adds List-Unsubscribe to internal group mail.
        // Untracked-domain real human sender must not be suppressed.
        assert!(!should_suppress_email(
            "Alex Patel <alex@partnercorp.example>",
            "Q3 planning sync — agenda attached",
            "<mailto:unsubscribe@partnercorp.example>",
            &HashSet::new(),
            &HashSet::new(),
        ));
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

    // =========================================================================
    // Preset-aware email signal type tests
    // =========================================================================

    fn merged_signal_types_for_preset(role: &str) -> Vec<String> {
        let preset = crate::presets::loader::load_preset(role)
            .unwrap_or_else(|e| panic!("Failed to load '{}' preset: {}", role, e));
        crate::state::build_merged_signal_config(&preset).email_signal_types
    }

    #[test]
    fn dos176_affiliates_merged_signal_types_include_campaign_deadline() {
        let types = merged_signal_types_for_preset("affiliates");
        assert!(
            types.iter().any(|t| t == "campaign_deadline_approaching"),
            "affiliates merged signal types must include 'campaign_deadline_approaching', got: {:?}",
            types
        );
    }

    #[test]
    fn dos176_affiliates_merged_signal_types_exclude_renewal_approaching() {
        // "renewal_approaching" is a CS-specific signal type and must NOT be in the
        // affiliates preset's own config (it may be in the base list — that is
        // acceptable — but the affiliates preset itself must not add it).
        let preset = crate::presets::loader::load_preset("affiliates")
            .expect("affiliates preset should load");
        assert!(
            !preset
                .intelligence
                .email_signal_types
                .iter()
                .any(|t| t == "renewal_approaching"),
            "affiliates preset config must NOT include 'renewal_approaching'"
        );
    }

    #[test]
    fn dos176_cs_merged_signal_types_include_churn_risk_and_renewal_approaching() {
        let types = merged_signal_types_for_preset("customer-success");
        assert!(
            types.iter().any(|t| t == "churn_risk"),
            "CS merged signal types must include 'churn_risk'"
        );
        assert!(
            types.iter().any(|t| t == "renewal_approaching"),
            "CS merged signal types must include 'renewal_approaching'"
        );
    }

    #[test]
    fn dos176_base_signal_types_are_generic() {
        // Base signal types must be role-neutral: no CS-specific types.
        for t in crate::state::BASE_EMAIL_SIGNAL_TYPES {
            assert!(
                *t != "churn_risk" && *t != "renewal_approaching" && *t != "champion_risk",
                "Base signal type '{}' is CS-specific — it must live in the CS preset",
                t
            );
        }
    }
}

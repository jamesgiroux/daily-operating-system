//! Shared constants for Phase 1 preparation (ported from ops/config.py).

/// Work-day start hour (24h) for gap analysis.
pub const WORK_DAY_START_HOUR: u32 = 9;

/// Work-day end hour (24h) for gap analysis.
pub const WORK_DAY_END_HOUR: u32 = 17;

/// Minimum gap length worth reporting (minutes).
pub const MIN_GAP_MINUTES: i64 = 30;

/// Personal email domains (not tied to any organization).
pub const PERSONAL_EMAIL_DOMAINS: &[&str] = &[
    "gmail.com",
    "googlemail.com",
    "outlook.com",
    "hotmail.com",
    "yahoo.com",
    "icloud.com",
    "me.com",
    "live.com",
];

/// High-priority email subject keywords (role-neutral base list).
///
/// CS-specific keywords (`churn`, `cancellation`, `cancel`) have been moved to the
/// CS preset's `intelligence.emailPriorityKeywords`. This base list
/// contains only terms that are high-signal across all roles.
pub const HIGH_PRIORITY_SUBJECT_KEYWORDS: &[&str] = &[
    // Urgency signals (universal)
    "urgent",
    "asap",
    "action required",
    "please respond",
    "deadline",
    "escalation",
    "critical",
    // Business/revenue signals (universal — not CS-specific)
    "renewal",
    "order form",
    "contract",
    "proposal",
    "invoice",
    "expansion",
    "sow",
    "msa",
    "amendment",
    "pricing",
    "budget",
    "signature required",
    "docusign",
];

/// Low-priority signals in from/subject fields.
pub const LOW_PRIORITY_SIGNALS: &[&str] = &[
    "newsletter",
    "digest",
    "notification",
    "automated",
    "noreply",
    "no-reply",
    "unsubscribe",
    "marketing",
    "promo",
    "promotions",
    "info@",
    "updates@",
    "news@",
    "do-not-reply",
    "donotreply",
    "notify",
    "mailer-daemon",
];

/// Bulk/marketing sender domains (FYI classification expansion).
///
/// Expanded with SaaS notification senders (LinkedIn, Slack, GitHub,
/// Notion, AWS, etc.) that should be hard-suppressed from inbox/Records,
/// not merely demoted to priority='low'.
pub const BULK_SENDER_DOMAINS: &[&str] = &[
    // ESP / marketing platforms
    "mailchimp.com",
    "sendgrid.net",
    "mandrillapp.com",
    "hubspot.com",
    "marketo.com",
    "pardot.com",
    "intercom.io",
    "customer.io",
    "mailgun.org",
    "postmarkapp.com",
    "amazonses.com",
    // SaaS notification senders
    "linkedin.com",
    "slack.com",
    "github.com",
    "notifications.github.com",
    "notion.so",
    "stripe.com",
    "amazonaws.com",
    "datadoghq.com",
    "atlassian.com",
    "calendly.com",
    "zoom.us",
    "loom.com",
    "docusign.net",
    "dropbox.com",
    "figma.com",
];

/// Subject substrings that signal automated/transactional/digest mail.
/// Matched case-insensitively against the email subject.
pub const NOISE_SUBJECT_PATTERNS: &[&str] = &[
    "your receipt",
    "your order",
    "weekly digest",
    "weekly summary",
    "security alert",
    "verification code",
    "[slack]",
    "[github]",
    "[wpvip]",
    "[new post]",
    "[new mention]",
    "[new comment]",
    "shipped",
    "confirmation #",
    "registration confirmed",
    "registration confirmation",
    "thank you for registering",
    "thursday updates",
    "spring cleaning",
    "monthly digest",
    "daily digest",
    "added activity",
    "your invitation",
];

/// Noreply local-part patterns.
pub const NOREPLY_LOCAL_PARTS: &[&str] = &[
    "noreply",
    "no-reply",
    "donotreply",
    "do-not-reply",
    "mailer-daemon",
];

/// Weekday names for weekly bucketing.
pub const DAY_NAMES: &[&str] = &["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_populated() {
        assert!(!PERSONAL_EMAIL_DOMAINS.is_empty());
        assert!(!HIGH_PRIORITY_SUBJECT_KEYWORDS.is_empty());
        assert!(!LOW_PRIORITY_SIGNALS.is_empty());
        assert!(!BULK_SENDER_DOMAINS.is_empty());
        assert!(!NOREPLY_LOCAL_PARTS.is_empty());
        assert_eq!(DAY_NAMES.len(), 5);
        assert_eq!(WORK_DAY_START_HOUR, 9);
        assert_eq!(WORK_DAY_END_HOUR, 17);
        assert_eq!(MIN_GAP_MINUTES, 30);
    }
}

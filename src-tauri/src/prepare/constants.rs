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

/// High-priority email subject keywords.
pub const HIGH_PRIORITY_SUBJECT_KEYWORDS: &[&str] = &[
    // Urgency signals
    "urgent",
    "asap",
    "action required",
    "please respond",
    "deadline",
    "escalation",
    "critical",
    // Business/revenue signals — renewals, contracts, commercial activity
    "renewal",
    "order form",
    "contract",
    "proposal",
    "invoice",
    "expansion",
    "churn",
    "cancellation",
    "cancel",
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

/// Bulk/marketing sender domains (I21: FYI classification expansion).
pub const BULK_SENDER_DOMAINS: &[&str] = &[
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
];

/// Noreply local-part patterns (I21).
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

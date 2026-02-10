//! Shared metadata parser for inline action tokens.
//!
//! Extracts priority (`P1`/`P2`/`P3`), account (`@Name`), due date (`due: YYYY-MM-DD`),
//! context (`#tag`), and waiting/blocked status from action text.
//! Returns a clean title with mechanical tokens stripped.

use std::sync::OnceLock;

use regex::Regex;

/// Parsed metadata from an action line.
#[derive(Debug, Clone, Default)]
pub struct ActionMetadata {
    /// Extracted priority (P1/P2/P3), or None if not specified.
    pub priority: Option<String>,
    /// Account name from `@AccountName`.
    pub account: Option<String>,
    /// Due date in YYYY-MM-DD format.
    pub due_date: Option<String>,
    /// Context tag from `#tag`.
    pub context: Option<String>,
    /// Whether waiting/blocked/pending keywords were found.
    pub is_waiting: bool,
    /// Title with metadata tokens stripped and whitespace normalized.
    pub clean_title: String,
}

// Compile-once regex patterns via OnceLock.
fn re_priority() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\b(P[123])\b").unwrap())
}

fn re_account() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"@(\S+)").unwrap())
}

fn re_due_date() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)due[:\s]+(\d{4}-\d{2}-\d{2})").unwrap())
}

fn re_context() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Match #"quoted multi-word context" or #single-word (backwards compatible)
    RE.get_or_init(|| Regex::new(r#"#"([^"]+)"|#(\S+)"#).unwrap())
}

fn re_waiting() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\b(waiting|blocked|pending)\b").unwrap())
}

/// Parse inline metadata tokens from action text.
///
/// Extracts priority, account, due date, context, and waiting status.
/// Returns a `clean_title` with mechanical tokens (`P1`, `@Acme`, `due: ...`, `#tag`)
/// stripped out. Waiting keywords are NOT stripped (they carry semantic meaning).
pub fn parse_action_metadata(text: &str) -> ActionMetadata {
    let mut meta = ActionMetadata::default();

    if text.is_empty() {
        return meta;
    }

    // Extract values (first match wins for each)
    if let Some(caps) = re_priority().captures(text) {
        meta.priority = Some(caps[1].to_uppercase());
    }

    if let Some(caps) = re_account().captures(text) {
        meta.account = Some(caps[1].to_string());
    }

    if let Some(caps) = re_due_date().captures(text) {
        meta.due_date = Some(caps[1].to_string());
    }

    if let Some(caps) = re_context().captures(text) {
        // Group 1 = quoted multi-word, Group 2 = single-word fallback
        meta.context = caps
            .get(1)
            .or_else(|| caps.get(2))
            .map(|m| m.as_str().to_string());
    }

    meta.is_waiting = re_waiting().is_match(text);

    // Build clean title: strip mechanical tokens, normalize whitespace.
    // Waiting keywords are NOT stripped — "Waiting on John" is meaningful.
    let mut clean = text.to_string();
    clean = re_priority().replace_all(&clean, "").to_string();
    clean = re_account().replace_all(&clean, "").to_string();
    clean = re_due_date().replace_all(&clean, "").to_string();
    clean = re_context().replace_all(&clean, "").to_string();

    // Normalize whitespace: collapse runs of spaces, trim
    meta.clean_title = clean.split_whitespace().collect::<Vec<_>>().join(" ");

    meta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_metadata() {
        let m = parse_action_metadata("P1 @Acme Follow up on renewal due: 2026-03-15 #billing");
        assert_eq!(m.priority.as_deref(), Some("P1"));
        assert_eq!(m.account.as_deref(), Some("Acme"));
        assert_eq!(m.due_date.as_deref(), Some("2026-03-15"));
        assert_eq!(m.context.as_deref(), Some("billing"));
        assert!(!m.is_waiting);
        assert_eq!(m.clean_title, "Follow up on renewal");
    }

    #[test]
    fn quoted_context() {
        let m = parse_action_metadata(
            r#"Follow up on renewal P1 @Acme due: 2026-03-15 #"CFO needs pricing comparison before Q2""#,
        );
        assert_eq!(m.priority.as_deref(), Some("P1"));
        assert_eq!(m.account.as_deref(), Some("Acme"));
        assert_eq!(m.due_date.as_deref(), Some("2026-03-15"));
        assert_eq!(
            m.context.as_deref(),
            Some("CFO needs pricing comparison before Q2")
        );
        assert_eq!(m.clean_title, "Follow up on renewal");
    }

    #[test]
    fn no_metadata() {
        let m = parse_action_metadata("Send weekly update email");
        assert!(m.priority.is_none());
        assert!(m.account.is_none());
        assert!(m.due_date.is_none());
        assert!(m.context.is_none());
        assert!(!m.is_waiting);
        assert_eq!(m.clean_title, "Send weekly update email");
    }

    #[test]
    fn priority_only() {
        let m = parse_action_metadata("Send weekly update P3");
        assert_eq!(m.priority.as_deref(), Some("P3"));
        assert_eq!(m.clean_title, "Send weekly update");
    }

    #[test]
    fn account_only() {
        let m = parse_action_metadata("Review contract @BigCorp");
        assert_eq!(m.account.as_deref(), Some("BigCorp"));
        assert_eq!(m.clean_title, "Review contract");
    }

    #[test]
    fn due_date_only() {
        let m = parse_action_metadata("Submit report due: 2026-01-31");
        assert_eq!(m.due_date.as_deref(), Some("2026-01-31"));
        assert_eq!(m.clean_title, "Submit report");
    }

    #[test]
    fn context_only() {
        let m = parse_action_metadata("Fix login bug #support");
        assert_eq!(m.context.as_deref(), Some("support"));
        assert_eq!(m.clean_title, "Fix login bug");
    }

    #[test]
    fn case_insensitivity() {
        let m = parse_action_metadata("p1 urgent task Due: 2026-06-01");
        assert_eq!(m.priority.as_deref(), Some("P1"));
        assert_eq!(m.due_date.as_deref(), Some("2026-06-01"));
        assert_eq!(m.clean_title, "urgent task");
    }

    #[test]
    fn waiting_keyword() {
        let m = parse_action_metadata("Waiting on John for contract review");
        assert!(m.is_waiting);
        // Waiting keywords are NOT stripped from clean_title
        assert_eq!(m.clean_title, "Waiting on John for contract review");
    }

    #[test]
    fn blocked_keyword() {
        let m = parse_action_metadata("Blocked by legal review @Acme");
        assert!(m.is_waiting);
        assert_eq!(m.account.as_deref(), Some("Acme"));
        assert_eq!(m.clean_title, "Blocked by legal review");
    }

    #[test]
    fn pending_keyword() {
        let m = parse_action_metadata("Pending approval from finance");
        assert!(m.is_waiting);
        assert_eq!(m.clean_title, "Pending approval from finance");
    }

    #[test]
    fn whitespace_normalization() {
        let m = parse_action_metadata("P2   @Acme   Follow up   #billing");
        assert_eq!(m.clean_title, "Follow up");
    }

    #[test]
    fn empty_string() {
        let m = parse_action_metadata("");
        assert!(m.priority.is_none());
        assert!(m.account.is_none());
        assert!(m.due_date.is_none());
        assert!(m.context.is_none());
        assert!(!m.is_waiting);
        assert_eq!(m.clean_title, "");
    }

    #[test]
    fn hyphenated_account() {
        let m = parse_action_metadata("Renew contract @Acme-Corp");
        assert_eq!(m.account.as_deref(), Some("Acme-Corp"));
        assert_eq!(m.clean_title, "Renew contract");
    }

    #[test]
    fn due_without_colon() {
        // "due 2026-03-15" should also work (regex allows due[:\s]+)
        let m = parse_action_metadata("Submit report due 2026-03-15");
        assert_eq!(m.due_date.as_deref(), Some("2026-03-15"));
        assert_eq!(m.clean_title, "Submit report");
    }
}

//! Three-gate email filtering and deduplication logic (I652 Phase 2).
//!
//! This module implements intelligent filtering to prevent over-enrichment of emails:
//!
//! - **Gate 0 (Primary Deduplication):** Skip already-enriched emails unless content changed.
//! - **Gate 1 (Selective Filtering):** Apply recency, priority, and newness checks.
//! - **Gate 2 (Bounding):** Limit to N emails with priority-tier-first sorting.
//!
//! The orchestrator `select_emails_for_enrichment()` runs all three gates in sequence,
//! returning only the emails that should be enriched in the current batch.

use chrono::{DateTime, Duration, Utc};
use std::collections::{HashMap, HashSet};

/// Prior email content snapshot for content-change detection in Gate 0.
/// Stores the state of an email when it was last enriched.
/// Used to determine if content has changed since last enrichment (e.g., new reply in thread).
#[derive(Debug, Clone)]
pub struct PriorEmailSnapshot {
    /// Optional snippet text from email body (used for content change detection).
    pub snippet: Option<String>,
    /// Subject line (used for content change detection).
    pub subject: Option<String>,
}

/// Gate 0: Skip already-enriched emails, unless content changed.
///
/// Returns `true` if the email should be SKIPPED (already enriched with no content change).
/// Returns `false` if the email should CONTINUE to Gate 1 (never enriched or content changed).
///
/// Logic:
/// - If `enriched_at` is None → email was never enriched → proceed to Gate 1 (return false)
/// - If `enriched_at` is older than 24 hours → re-enrichment eligible → proceed to Gate 1 (return false)
/// - If `enriched_at` is recent (within 24h) AND content unchanged → skip this email (return true)
/// - If `enriched_at` is recent AND content changed → allow re-enrichment → proceed to Gate 1 (return false)
pub fn gate_0_skip_enriched(
    email_id: &str,
    enriched_at: Option<DateTime<Utc>>,
    current_snippet: &Option<String>,
    current_subject: &Option<String>,
    prior_snapshots: &HashMap<String, PriorEmailSnapshot>,
    now: DateTime<Utc>,
) -> bool {
    // If never enriched, proceed to next gate
    let Some(enr_at) = enriched_at else {
        return false;
    };

    // If enriched more than 24 hours ago, re-enrichment is eligible
    if enr_at < now - Duration::hours(24) {
        return false;
    }

    // Enriched within last 24 hours: check if content changed
    match prior_snapshots.get(email_id) {
        Some(prior) => {
            if current_snippet != &prior.snippet || current_subject != &prior.subject {
                // Content changed (e.g., new reply added to thread), allow re-enrichment
                return false;
            }
            // Content unchanged and recently enriched: skip this email
            true
        }
        None => {
            // No prior snapshot to compare against: proceed conservatively
            // (can't determine if content changed, so allow re-enrichment)
            false
        }
    }
}

/// Gate 1: Selective filtering (priority + recency + newness).
///
/// Returns `true` if the email PASSES all gates and should continue to Gate 2.
/// Returns `false` if the email FAILS any gate and should be skipped entirely.
///
/// Checks (all must pass):
/// 1. Recency: email received within last 7 days
/// 2. Priority: either high OR (medium + known_domain)
/// 3. Newness: thread has response within last 24 hours
pub fn gate_1_selective(
    priority: Option<&str>,
    received_at: DateTime<Utc>,
    last_response_date: Option<DateTime<Utc>>,
    known_domains: &HashSet<String>,
    sender_email: Option<&str>,
    now: DateTime<Utc>,
) -> bool {
    // Check 1: Recency gate (7 days)
    if received_at < now - Duration::days(7) {
        return false;
    }

    // Check 2: Priority gate (high OR medium+known_domain)
    let is_high = priority.map(|p| p == "high").unwrap_or(false);
    let is_medium_known_domain = if priority.map(|p| p == "medium").unwrap_or(false) {
        let domain = sender_email
            .and_then(|e| e.split('@').nth(1))
            .unwrap_or("")
            .to_lowercase();
        !domain.is_empty() && known_domains.contains(&domain)
    } else {
        false
    };

    if !is_high && !is_medium_known_domain {
        return false;
    }

    // Check 3: Newness gate (response within 24 hours)
    if let Some(response_date) = last_response_date {
        if response_date >= now - Duration::days(1) {
            return true; // Pass all gates
        }
    }

    false // Fail newness gate
}

/// Represents an email with priority information for sorting in Gate 2.
/// This is a minimal view used only for sorting purposes.
#[derive(Debug, Clone)]
pub struct EmailForSort {
    pub email_id: String,
    pub priority: Option<String>,
    pub received_at: DateTime<Utc>,
}

/// Gate 2: Limit to N emails, priority-tier-first sort.
///
/// Sorts emails such that:
/// 1. All high-priority emails come first (sorted by received_at descending)
/// 2. Then medium-priority emails (sorted by received_at descending)
/// 3. Then low-priority emails (sorted by received_at descending)
/// 4. Takes only the first N emails.
///
/// Within the same priority tier, more recent emails come first (descending received_at).
/// If two emails have the same priority and received_at, secondary sort by email_id for determinism.
pub fn gate_2_limit_and_sort(emails: Vec<EmailForSort>, limit: usize) -> Vec<EmailForSort> {
    let mut sorted = emails;

    // Sort: priority tier first (0=high, 1=medium, 2=low)
    // Then within tier: received_at descending (most recent first)
    // Then email_id ascending (deterministic tie-breaker)
    sorted.sort_by(|a, b| {
        let priority_order = |p: Option<&str>| match p {
            Some("high") => 0,
            Some("medium") => 1,
            _ => 2,
        };

        let a_priority = priority_order(a.priority.as_deref());
        let b_priority = priority_order(b.priority.as_deref());

        match a_priority.cmp(&b_priority) {
            std::cmp::Ordering::Equal => {
                // Same priority tier: sort by received_at descending (most recent first)
                match b.received_at.cmp(&a.received_at) {
                    std::cmp::Ordering::Equal => {
                        // Tie-breaker: sort by email_id ascending
                        a.email_id.cmp(&b.email_id)
                    }
                    other => other,
                }
            }
            other => other,
        }
    });

    sorted.truncate(limit);
    sorted
}

/// Input configuration for email filtering across all three gates.
#[derive(Debug, Clone)]
pub struct EmailFilterInput<'a> {
    /// Map of email_id -> Option<DateTime<Utc>> for enriched_at from DB.
    pub enriched_at_map: &'a HashMap<String, Option<DateTime<Utc>>>,
    /// Map of email_id -> prior snapshot state (for content-change detection).
    pub snippets_map: &'a HashMap<String, PriorEmailSnapshot>,
    /// Set of known customer/contact domains (extracted from CRM entities).
    pub known_domains: &'a HashSet<String>,
    /// Map of email_id -> sender email address.
    pub sender_email_map: &'a HashMap<String, Option<String>>,
    /// Map of email_id -> last response date in thread.
    pub last_response_date_map: &'a HashMap<String, Option<DateTime<Utc>>>,
    /// Maximum number of emails to return (typically 5-7).
    pub limit: usize,
    /// Current timestamp (injected for testing).
    pub now: DateTime<Utc>,
}

/// Orchestrates all three gates, returning the final list of emails to enrich.
///
/// This is the main entry point for email filtering. It:
/// 1. Loads prior snapshots (for Gate 0 content-change detection)
/// 2. Applies Gate 0 (skip enriched with unchanged content)
/// 3. Applies Gate 1 (selective filtering: recency, priority, newness)
/// 4. Applies Gate 2 (limit to top N, sorted by priority-tier-first)
///
/// # Arguments
///
/// * `emails` - Emails from directive or DB, with id, priority, received_at, last_response_date, sender_email
/// * `input` - Filter configuration (maps and limits)
///
/// # Returns
///
/// Filtered and sorted vector of emails ready for enrichment.
pub fn select_emails_for_enrichment(
    emails: Vec<EmailForSort>,
    input: &EmailFilterInput,
) -> Vec<EmailForSort> {
    // Gate 0: Skip already-enriched emails (unless content changed)
    let gate_0_pass: Vec<EmailForSort> = emails
        .into_iter()
        .filter(|email| {
            let enriched_at = input
                .enriched_at_map
                .get(&email.email_id)
                .copied()
                .flatten();
            let current_snippet = input
                .snippets_map
                .get(&email.email_id)
                .and_then(|s| s.snippet.as_ref());
            let current_subject = input
                .snippets_map
                .get(&email.email_id)
                .and_then(|s| s.subject.as_ref());

            // Returns true if should SKIP, false if should CONTINUE
            // We want emails that DON'T skip (false)
            !gate_0_skip_enriched(
                &email.email_id,
                enriched_at,
                &current_snippet.cloned(),
                &current_subject.cloned(),
                input.snippets_map,
                input.now,
            )
        })
        .collect();

    // Gate 1: Selective filtering (priority + recency + newness)
    let gate_1_pass: Vec<EmailForSort> = gate_0_pass
        .into_iter()
        .filter(|email| {
            let sender_email = input
                .sender_email_map
                .get(&email.email_id)
                .and_then(|se| se.as_deref());
            let last_response_date = input
                .last_response_date_map
                .get(&email.email_id)
                .copied()
                .flatten();

            gate_1_selective(
                email.priority.as_deref(),
                email.received_at,
                last_response_date,
                input.known_domains,
                sender_email,
                input.now,
            )
        })
        .collect();

    // Gate 2: Limit and sort
    gate_2_limit_and_sort(gate_1_pass, input.limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Gate 0 Tests (Skip Already-Enriched)
    // =========================================================================

    #[test]
    fn gate_0_never_enriched_proceeds() {
        let now = Utc::now();
        let result = gate_0_skip_enriched(
            "email1",
            None, // Never enriched
            &Some("snippet".to_string()),
            &Some("subject".to_string()),
            &HashMap::new(),
            now,
        );
        assert!(!result, "Never-enriched email should proceed to next gate");
    }

    #[test]
    fn gate_0_recently_enriched_unchanged_skipped() {
        let now = Utc::now();
        let enriched_at = now - Duration::hours(12);
        let snapshot = PriorEmailSnapshot {
            snippet: Some("snippet".to_string()),
            subject: Some("subject".to_string()),
        };
        let mut snapshots = HashMap::new();
        snapshots.insert("email1".to_string(), snapshot);

        let result = gate_0_skip_enriched(
            "email1",
            Some(enriched_at),
            &Some("snippet".to_string()),
            &Some("subject".to_string()),
            &snapshots,
            now,
        );
        assert!(
            result,
            "Recently enriched email with no content change should be skipped"
        );
    }

    #[test]
    fn gate_0_recently_enriched_snippet_changed_proceeds() {
        let now = Utc::now();
        let enriched_at = now - Duration::hours(12);
        let snapshot = PriorEmailSnapshot {
            snippet: Some("old snippet".to_string()),
            subject: Some("subject".to_string()),
        };
        let mut snapshots = HashMap::new();
        snapshots.insert("email1".to_string(), snapshot);

        let result = gate_0_skip_enriched(
            "email1",
            Some(enriched_at),
            &Some("new snippet".to_string()),
            &Some("subject".to_string()),
            &snapshots,
            now,
        );
        assert!(
            !result,
            "Recently enriched email with snippet change should proceed to next gate"
        );
    }

    #[test]
    fn gate_0_recently_enriched_subject_changed_proceeds() {
        let now = Utc::now();
        let enriched_at = now - Duration::hours(12);
        let snapshot = PriorEmailSnapshot {
            snippet: Some("snippet".to_string()),
            subject: Some("old subject".to_string()),
        };
        let mut snapshots = HashMap::new();
        snapshots.insert("email1".to_string(), snapshot);

        let result = gate_0_skip_enriched(
            "email1",
            Some(enriched_at),
            &Some("snippet".to_string()),
            &Some("new subject".to_string()),
            &snapshots,
            now,
        );
        assert!(
            !result,
            "Recently enriched email with subject change should proceed to next gate"
        );
    }

    #[test]
    fn gate_0_expired_enrichment_proceeds() {
        let now = Utc::now();
        let enriched_at = now - Duration::hours(25); // Older than 24h
        let snapshot = PriorEmailSnapshot {
            snippet: Some("snippet".to_string()),
            subject: Some("subject".to_string()),
        };
        let mut snapshots = HashMap::new();
        snapshots.insert("email1".to_string(), snapshot);

        let result = gate_0_skip_enriched(
            "email1",
            Some(enriched_at),
            &Some("snippet".to_string()),
            &Some("subject".to_string()),
            &snapshots,
            now,
        );
        assert!(
            !result,
            "Expired enrichment (>24h) should proceed to next gate for re-enrichment"
        );
    }

    #[test]
    fn gate_0_boundary_exactly_24h_ago_skipped() {
        let now = Utc::now();
        let enriched_at = now - Duration::hours(24);
        let snapshot = PriorEmailSnapshot {
            snippet: Some("snippet".to_string()),
            subject: Some("subject".to_string()),
        };
        let mut snapshots = HashMap::new();
        snapshots.insert("email1".to_string(), snapshot);

        let result = gate_0_skip_enriched(
            "email1",
            Some(enriched_at),
            &Some("snippet".to_string()),
            &Some("subject".to_string()),
            &snapshots,
            now,
        );
        // At exactly 24h, it's still within the window, so it should be skipped
        // (enriched_at <= now - 24h is false when enriched_at == now - 24h)
        assert!(
            result,
            "Email enriched exactly 24h ago should still be skipped"
        );
    }

    #[test]
    fn gate_0_boundary_just_over_24h_proceeds() {
        let now = Utc::now();
        // Just barely over 24h: 24h + 1 second
        let enriched_at = now - Duration::hours(24) - Duration::seconds(1);
        let snapshot = PriorEmailSnapshot {
            snippet: Some("snippet".to_string()),
            subject: Some("subject".to_string()),
        };
        let mut snapshots = HashMap::new();
        snapshots.insert("email1".to_string(), snapshot);

        let result = gate_0_skip_enriched(
            "email1",
            Some(enriched_at),
            &Some("snippet".to_string()),
            &Some("subject".to_string()),
            &snapshots,
            now,
        );
        assert!(
            !result,
            "Email enriched >24h ago should proceed for re-enrichment"
        );
    }

    #[test]
    fn gate_0_no_prior_snapshot_proceeds() {
        let now = Utc::now();
        let enriched_at = now - Duration::hours(12);
        let snapshots = HashMap::new(); // No prior snapshot

        let result = gate_0_skip_enriched(
            "email1",
            Some(enriched_at),
            &Some("snippet".to_string()),
            &Some("subject".to_string()),
            &snapshots,
            now,
        );
        // No prior snapshot: we can't compare, so proceed conservatively
        assert!(
            !result,
            "Email with no prior snapshot should proceed (conservative approach)"
        );
    }

    // =========================================================================
    // Gate 1 Tests (Selective Filtering)
    // =========================================================================

    #[test]
    fn gate_1_high_priority_recent_recent_response_passes() {
        let now = Utc::now();
        let received_at = now - Duration::hours(2);
        let last_response = now - Duration::hours(1);
        let known_domains = HashSet::new();

        let result = gate_1_selective(
            Some("high"),
            received_at,
            Some(last_response),
            &known_domains,
            Some("sender@example.com"),
            now,
        );
        assert!(result, "High-priority with recent response should pass");
    }

    #[test]
    fn gate_1_high_priority_old_fails_recency() {
        let now = Utc::now();
        let received_at = now - Duration::days(8); // Older than 7 days
        let last_response = now - Duration::hours(1);
        let known_domains = HashSet::new();

        let result = gate_1_selective(
            Some("high"),
            received_at,
            Some(last_response),
            &known_domains,
            Some("sender@example.com"),
            now,
        );
        assert!(
            !result,
            "High-priority but old (>7 days) should fail recency check"
        );
    }

    #[test]
    fn gate_1_high_priority_no_recent_response_fails_newness() {
        let now = Utc::now();
        let received_at = now - Duration::hours(2);
        let last_response = now - Duration::days(2); // No response in last 24h
        let known_domains = HashSet::new();

        let result = gate_1_selective(
            Some("high"),
            received_at,
            Some(last_response),
            &known_domains,
            Some("sender@example.com"),
            now,
        );
        assert!(
            !result,
            "High-priority but no recent response should fail newness check"
        );
    }

    #[test]
    fn gate_1_medium_priority_known_domain_passes() {
        let now = Utc::now();
        let received_at = now - Duration::hours(2);
        let last_response = now - Duration::hours(1);
        let mut known_domains = HashSet::new();
        known_domains.insert("example.com".to_string());

        let result = gate_1_selective(
            Some("medium"),
            received_at,
            Some(last_response),
            &known_domains,
            Some("sender@example.com"),
            now,
        );
        assert!(
            result,
            "Medium-priority with known domain and recent response should pass"
        );
    }

    #[test]
    fn gate_1_medium_priority_unknown_domain_fails() {
        let now = Utc::now();
        let received_at = now - Duration::hours(2);
        let last_response = now - Duration::hours(1);
        let mut known_domains = HashSet::new();
        known_domains.insert("example.com".to_string());

        let result = gate_1_selective(
            Some("medium"),
            received_at,
            Some(last_response),
            &known_domains,
            Some("sender@unknown.com"),
            now,
        );
        assert!(
            !result,
            "Medium-priority with unknown domain should fail domain check"
        );
    }

    #[test]
    fn gate_1_low_priority_always_fails() {
        let now = Utc::now();
        let received_at = now - Duration::hours(2);
        let last_response = now - Duration::hours(1);
        let known_domains = HashSet::new();

        let result = gate_1_selective(
            Some("low"),
            received_at,
            Some(last_response),
            &known_domains,
            Some("sender@example.com"),
            now,
        );
        assert!(!result, "Low-priority should always fail");
    }

    #[test]
    fn gate_1_no_priority_fails() {
        let now = Utc::now();
        let received_at = now - Duration::hours(2);
        let last_response = now - Duration::hours(1);
        let known_domains = HashSet::new();

        let result = gate_1_selective(
            None,
            received_at,
            Some(last_response),
            &known_domains,
            Some("sender@example.com"),
            now,
        );
        assert!(!result, "No priority should fail");
    }

    #[test]
    fn gate_1_recency_boundary_exactly_7_days_passes() {
        let now = Utc::now();
        let received_at = now - Duration::days(7);
        let last_response = now - Duration::hours(1);
        let known_domains = HashSet::new();

        let result = gate_1_selective(
            Some("high"),
            received_at,
            Some(last_response),
            &known_domains,
            Some("sender@example.com"),
            now,
        );
        assert!(
            result,
            "Email received exactly 7 days ago should pass recency check"
        );
    }

    #[test]
    fn gate_1_recency_boundary_just_over_7_days_fails() {
        let now = Utc::now();
        let received_at = now - Duration::days(7) - Duration::seconds(1);
        let last_response = now - Duration::hours(1);
        let known_domains = HashSet::new();

        let result = gate_1_selective(
            Some("high"),
            received_at,
            Some(last_response),
            &known_domains,
            Some("sender@example.com"),
            now,
        );
        assert!(
            !result,
            "Email received >7 days ago should fail recency check"
        );
    }

    #[test]
    fn gate_1_newness_boundary_exactly_24h_ago_passes() {
        let now = Utc::now();
        let received_at = now - Duration::hours(2);
        let last_response = now - Duration::days(1);
        let known_domains = HashSet::new();

        let result = gate_1_selective(
            Some("high"),
            received_at,
            Some(last_response),
            &known_domains,
            Some("sender@example.com"),
            now,
        );
        // last_response >= now - 24h should be true when last_response == now - 24h
        assert!(
            result,
            "Response exactly 24h ago should pass newness check (boundary case)"
        );
    }

    #[test]
    fn gate_1_newness_boundary_just_over_24h_ago_fails() {
        let now = Utc::now();
        let received_at = now - Duration::hours(2);
        let last_response = now - Duration::days(1) - Duration::seconds(1);
        let known_domains = HashSet::new();

        let result = gate_1_selective(
            Some("high"),
            received_at,
            Some(last_response),
            &known_domains,
            Some("sender@example.com"),
            now,
        );
        assert!(!result, "Response >24h ago should fail newness check");
    }

    #[test]
    fn gate_1_no_response_date_fails_newness() {
        let now = Utc::now();
        let received_at = now - Duration::hours(2);
        let known_domains = HashSet::new();

        let result = gate_1_selective(
            Some("high"),
            received_at,
            None, // No response date
            &known_domains,
            Some("sender@example.com"),
            now,
        );
        assert!(!result, "No response date should fail newness check");
    }

    #[test]
    fn gate_1_medium_known_domain_case_insensitive() {
        let now = Utc::now();
        let received_at = now - Duration::hours(2);
        let last_response = now - Duration::hours(1);
        let mut known_domains = HashSet::new();
        known_domains.insert("example.com".to_string());

        let result = gate_1_selective(
            Some("medium"),
            received_at,
            Some(last_response),
            &known_domains,
            Some("sender@EXAMPLE.COM"),
            now,
        );
        assert!(
            result,
            "Domain matching should be case-insensitive (EXAMPLE.COM == example.com)"
        );
    }

    // =========================================================================
    // Gate 2 Tests (Limit & Sort)
    // =========================================================================

    #[test]
    fn gate_2_sorts_by_priority_tier_first() {
        let now = Utc::now();
        let emails = vec![
            EmailForSort {
                email_id: "medium1".to_string(),
                priority: Some("medium".to_string()),
                received_at: now - Duration::hours(1),
            },
            EmailForSort {
                email_id: "high1".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(2),
            },
            EmailForSort {
                email_id: "low1".to_string(),
                priority: Some("low".to_string()),
                received_at: now,
            },
        ];

        let result = gate_2_limit_and_sort(emails, 10);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].email_id, "high1", "High priority should be first");
        assert_eq!(
            result[1].email_id, "medium1",
            "Medium priority should be second"
        );
        assert_eq!(result[2].email_id, "low1", "Low priority should be last");
    }

    #[test]
    fn gate_2_sorts_within_tier_by_recency() {
        let now = Utc::now();
        let emails = vec![
            EmailForSort {
                email_id: "high_old".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(3),
            },
            EmailForSort {
                email_id: "high_new".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(1),
            },
            EmailForSort {
                email_id: "high_newest".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::minutes(30),
            },
        ];

        let result = gate_2_limit_and_sort(emails, 10);
        assert_eq!(result.len(), 3);
        assert_eq!(
            result[0].email_id, "high_newest",
            "Newest high-priority should be first"
        );
        assert_eq!(
            result[1].email_id, "high_new",
            "Middle high-priority should be second"
        );
        assert_eq!(
            result[2].email_id, "high_old",
            "Oldest high-priority should be last"
        );
    }

    #[test]
    fn gate_2_enforces_limit() {
        let now = Utc::now();
        let mut emails = Vec::new();
        for i in 0..10 {
            emails.push(EmailForSort {
                email_id: format!("email{}", i),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(i as i64),
            });
        }

        let result = gate_2_limit_and_sort(emails, 5);
        assert_eq!(result.len(), 5, "Should enforce limit of 5");
    }

    #[test]
    fn gate_2_returns_less_than_limit_when_input_smaller() {
        let now = Utc::now();
        let emails = vec![
            EmailForSort {
                email_id: "email1".to_string(),
                priority: Some("high".to_string()),
                received_at: now,
            },
            EmailForSort {
                email_id: "email2".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(1),
            },
        ];

        let result = gate_2_limit_and_sort(emails, 5);
        assert_eq!(
            result.len(),
            2,
            "Should return all input when less than limit"
        );
    }

    #[test]
    fn gate_2_complex_scenario_4h_2m_with_limit_5() {
        let now = Utc::now();
        let emails = vec![
            EmailForSort {
                email_id: "high1".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(1),
            },
            EmailForSort {
                email_id: "medium1".to_string(),
                priority: Some("medium".to_string()),
                received_at: now - Duration::minutes(30),
            },
            EmailForSort {
                email_id: "high2".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(2),
            },
            EmailForSort {
                email_id: "medium2".to_string(),
                priority: Some("medium".to_string()),
                received_at: now - Duration::hours(1),
            },
            EmailForSort {
                email_id: "high3".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(3),
            },
            EmailForSort {
                email_id: "medium3".to_string(),
                priority: Some("medium".to_string()),
                received_at: now - Duration::hours(2),
            },
        ];

        let result = gate_2_limit_and_sort(emails, 5);
        assert_eq!(result.len(), 5);
        // All 3 high-priority first (sorted by recency)
        assert_eq!(result[0].email_id, "high1", "Newest high should be first");
        assert_eq!(result[1].email_id, "high2", "Middle high should be second");
        assert_eq!(result[2].email_id, "high3", "Oldest high should be third");
        // Then medium-priority (sorted by recency)
        assert_eq!(
            result[3].email_id, "medium1",
            "Newest medium should be fourth"
        );
        assert_eq!(result[4].email_id, "medium2", "Next medium should be fifth");
    }

    #[test]
    fn gate_2_deterministic_tie_breaker() {
        let now = Utc::now();
        // Two emails with same priority and received_at
        let emails = vec![
            EmailForSort {
                email_id: "z_email".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(1),
            },
            EmailForSort {
                email_id: "a_email".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(1),
            },
        ];

        let result = gate_2_limit_and_sort(emails, 10);
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].email_id, "a_email",
            "With same priority and time, should sort by email_id ascending"
        );
        assert_eq!(result[1].email_id, "z_email");
    }

    // =========================================================================
    // Orchestrator Tests (All Three Gates Together)
    // =========================================================================

    #[test]
    fn orchestrator_applies_all_three_gates() {
        let now = Utc::now();

        // Set up 10 emails with varied characteristics
        let emails = vec![
            // Will pass all gates
            EmailForSort {
                email_id: "e1".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(2),
            },
            // Will fail Gate 0 (already enriched, no content change)
            EmailForSort {
                email_id: "e2".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(3),
            },
            // Will fail Gate 1 (old, >7 days)
            EmailForSort {
                email_id: "e3".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::days(8),
            },
            // Will fail Gate 1 (no recent response)
            EmailForSort {
                email_id: "e4".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(2),
            },
            // Will pass all gates
            EmailForSort {
                email_id: "e5".to_string(),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(1),
            },
        ];

        let mut enriched_at_map = HashMap::new();
        enriched_at_map.insert("e1".to_string(), None);
        enriched_at_map.insert("e2".to_string(), Some(now - Duration::hours(12))); // Already enriched
        enriched_at_map.insert("e3".to_string(), None);
        enriched_at_map.insert("e4".to_string(), None);
        enriched_at_map.insert("e5".to_string(), None);

        let mut snippets_map = HashMap::new();
        snippets_map.insert(
            "e2".to_string(),
            PriorEmailSnapshot {
                snippet: Some("unchanged".to_string()),
                subject: Some("unchanged".to_string()),
            },
        );

        let mut sender_email_map = HashMap::new();
        sender_email_map.insert("e1".to_string(), Some("sender@example.com".to_string()));
        sender_email_map.insert("e2".to_string(), Some("sender@example.com".to_string()));
        sender_email_map.insert("e3".to_string(), Some("sender@example.com".to_string()));
        sender_email_map.insert("e4".to_string(), Some("sender@example.com".to_string()));
        sender_email_map.insert("e5".to_string(), Some("sender@example.com".to_string()));

        let mut last_response_date_map = HashMap::new();
        last_response_date_map.insert("e1".to_string(), Some(now - Duration::hours(1)));
        last_response_date_map.insert("e2".to_string(), Some(now - Duration::hours(1)));
        last_response_date_map.insert("e3".to_string(), Some(now - Duration::hours(1)));
        last_response_date_map.insert("e4".to_string(), None); // No recent response
        last_response_date_map.insert("e5".to_string(), Some(now - Duration::hours(1)));

        let known_domains = HashSet::new();

        let input = EmailFilterInput {
            enriched_at_map: &enriched_at_map,
            snippets_map: &snippets_map,
            known_domains: &known_domains,
            sender_email_map: &sender_email_map,
            last_response_date_map: &last_response_date_map,
            limit: 5,
            now,
        };
        let result = select_emails_for_enrichment(emails, &input);

        // Should only have e1 and e5 (both pass all gates and are high-priority)
        assert_eq!(result.len(), 2, "Should have 2 emails passing all gates");
        assert_eq!(result[0].email_id, "e5", "e5 is newer, should be first");
        assert_eq!(result[1].email_id, "e1", "e1 is older, should be second");
    }

    #[test]
    fn orchestrator_empty_input() {
        let now = Utc::now();
        let input = EmailFilterInput {
            enriched_at_map: &HashMap::new(),
            snippets_map: &HashMap::new(),
            known_domains: &HashSet::new(),
            sender_email_map: &HashMap::new(),
            last_response_date_map: &HashMap::new(),
            limit: 5,
            now,
        };
        let result = select_emails_for_enrichment(vec![], &input);
        assert_eq!(result.len(), 0, "Empty input should produce empty output");
    }

    #[test]
    fn orchestrator_all_emails_filtered_out() {
        let now = Utc::now();
        let emails = vec![EmailForSort {
            email_id: "e1".to_string(),
            priority: Some("low".to_string()),
            received_at: now - Duration::hours(2),
        }];

        let enriched_at_map = HashMap::new();
        let snippets_map = HashMap::new();
        let sender_email_map = HashMap::new();
        let last_response_date_map = HashMap::new();

        let input = EmailFilterInput {
            enriched_at_map: &enriched_at_map,
            snippets_map: &snippets_map,
            known_domains: &HashSet::new(),
            sender_email_map: &sender_email_map,
            last_response_date_map: &last_response_date_map,
            limit: 5,
            now,
        };
        let result = select_emails_for_enrichment(emails, &input);
        assert_eq!(
            result.len(),
            0,
            "Low-priority should be filtered out by Gate 1"
        );
    }

    #[test]
    fn orchestrator_respects_limit() {
        let now = Utc::now();
        let mut emails = Vec::new();
        for i in 0..20 {
            emails.push(EmailForSort {
                email_id: format!("e{}", i),
                priority: Some("high".to_string()),
                received_at: now - Duration::hours(i as i64),
            });
        }

        let enriched_at_map: HashMap<String, Option<DateTime<Utc>>> =
            emails.iter().map(|e| (e.email_id.clone(), None)).collect();

        let mut last_response_date_map = HashMap::new();
        for email in &emails {
            last_response_date_map.insert(email.email_id.clone(), Some(now - Duration::hours(1)));
        }

        let mut sender_email_map = HashMap::new();
        for email in &emails {
            sender_email_map.insert(
                email.email_id.clone(),
                Some("sender@example.com".to_string()),
            );
        }

        let input = EmailFilterInput {
            enriched_at_map: &enriched_at_map,
            snippets_map: &HashMap::new(),
            known_domains: &HashSet::new(),
            sender_email_map: &sender_email_map,
            last_response_date_map: &last_response_date_map,
            limit: 7,
            now,
        };
        let result = select_emails_for_enrichment(emails, &input);
        assert_eq!(result.len(), 7, "Should respect limit of 7");
    }

    #[test]
    fn orchestrator_medium_priority_filtered_by_domain() {
        let now = Utc::now();
        let emails = vec![
            EmailForSort {
                email_id: "e1".to_string(),
                priority: Some("medium".to_string()),
                received_at: now - Duration::hours(2),
            },
            EmailForSort {
                email_id: "e2".to_string(),
                priority: Some("medium".to_string()),
                received_at: now - Duration::hours(3),
            },
        ];

        let enriched_at_map = HashMap::new();
        let snippets_map = HashMap::new();

        let mut sender_email_map = HashMap::new();
        sender_email_map.insert("e1".to_string(), Some("sender@known.com".to_string()));
        sender_email_map.insert("e2".to_string(), Some("sender@unknown.com".to_string()));

        let mut last_response_date_map = HashMap::new();
        last_response_date_map.insert("e1".to_string(), Some(now - Duration::hours(1)));
        last_response_date_map.insert("e2".to_string(), Some(now - Duration::hours(1)));

        let mut known_domains = HashSet::new();
        known_domains.insert("known.com".to_string());

        let input = EmailFilterInput {
            enriched_at_map: &enriched_at_map,
            snippets_map: &snippets_map,
            known_domains: &known_domains,
            sender_email_map: &sender_email_map,
            last_response_date_map: &last_response_date_map,
            limit: 5,
            now,
        };
        let result = select_emails_for_enrichment(emails, &input);
        assert_eq!(result.len(), 1, "Only known domain should pass");
        assert_eq!(result[0].email_id, "e1");
    }
}

//! Meeting classification algorithm (10-rule, per MEETING-TYPES.md).
//!
//! Replaces ops/calendar_fetch.py:classify_meeting().
//! Classification priority (first match wins):
//!   1. Personal: 0-1 attendees
//!   2. All-hands: 50+ attendees
//!   3. Title keywords: all hands, qbr, training, one_on_one
//!   4. Domain-based: internal vs external
//!   5. External path: personal email, customer, external
//!   6. Internal path: one_on_one, team_sync, internal

use std::collections::HashSet;

use super::calendar::GoogleCalendarEvent;

/// All-hands attendee threshold (per MEETING-TYPES.md).
pub const ALL_HANDS_THRESHOLD: usize = 50;

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

/// Result of meeting classification.
#[derive(Debug, Clone)]
pub struct ClassifiedMeeting {
    pub id: String,
    pub title: String,
    pub start: String,
    pub end: String,
    pub attendees: Vec<String>,
    pub organizer: String,
    pub is_recurring: bool,
    pub is_all_day: bool,
    pub meeting_type: String,
    /// Account name (if matched to a known account).
    pub account: Option<String>,
    /// External domains found in attendees.
    pub external_domains: Vec<String>,
    /// Calendar event description (I185).
    pub description: String,
}

/// Classify a calendar event using the multi-signal algorithm.
///
/// Arguments:
/// - `event`: The raw calendar event.
/// - `user_domain`: The user's primary email domain (e.g., "company.com").
///   For multi-domain support (I171), use `classify_meeting_multi`.
/// - `account_hints`: Lowercased slugs of known customer accounts (e.g., {"acme", "bigcorp"}).
pub fn classify_meeting(
    event: &GoogleCalendarEvent,
    user_domain: &str,
    account_hints: &HashSet<String>,
) -> ClassifiedMeeting {
    let domains: Vec<String> = if user_domain.is_empty() {
        Vec::new()
    } else {
        vec![user_domain.to_string()]
    };
    classify_meeting_multi(event, &domains, account_hints)
}

/// Multi-domain meeting classification (I171).
///
/// Classifies attendees as internal if their domain matches ANY of `user_domains`.
pub fn classify_meeting_multi(
    event: &GoogleCalendarEvent,
    user_domains: &[String],
    account_hints: &HashSet<String>,
) -> ClassifiedMeeting {
    let title_lower = event.summary.to_lowercase();
    let attendee_count = event.attendees.len();

    let mut result = ClassifiedMeeting {
        id: event.id.clone(),
        title: event.summary.clone(),
        start: event.start.clone(),
        end: event.end.clone(),
        attendees: event.attendees.clone(),
        organizer: event.organizer.clone(),
        is_recurring: event.is_recurring,
        is_all_day: event.is_all_day,
        meeting_type: "internal".to_string(),
        account: None,
        external_domains: Vec::new(),
        description: event.description.clone(),
    };

    // ---- Step 1: Personal (no attendees or only organizer) ----
    if attendee_count <= 1 {
        result.meeting_type = "personal".to_string();
        return result;
    }

    // ---- Step 2: Scale-based override (50+ attendees) ----
    if attendee_count >= ALL_HANDS_THRESHOLD {
        result.meeting_type = "all_hands".to_string();
        return result;
    }

    // ---- Step 3: Title keyword overrides (all-hands) ----
    if contains_any(&title_lower, &["all hands", "all-hands", "town hall"]) {
        result.meeting_type = "all_hands".to_string();
        return result;
    }

    // Track title overrides that still need domain matching for account
    let title_override = if contains_any(
        &title_lower,
        &["qbr", "business review", "quarterly review"],
    ) {
        Some("qbr")
    } else if contains_any(&title_lower, &["training", "enablement", "workshop"]) {
        Some("training")
    } else if contains_any(&title_lower, &["1:1", "1-1", "one on one", "1-on-1"]) {
        Some("one_on_one")
    } else {
        None
    };

    // ---- Step 4: Domain classification (I171: multi-domain) ----
    let (external, _internal): (Vec<&String>, Vec<&String>) = if !user_domains.is_empty() {
        event
            .attendees
            .iter()
            .filter(|a| a.contains('@'))
            .partition(|a| {
                let lower = a.to_lowercase();
                !user_domains
                    .iter()
                    .any(|d| !d.is_empty() && lower.ends_with(&format!("@{}", d)))
            })
    } else {
        // Without known domains, treat all as potentially external
        (event.attendees.iter().collect(), Vec::new())
    };

    let external_domains: HashSet<String> = external
        .iter()
        .filter_map(|a| a.split('@').nth(1))
        .map(|d| d.to_lowercase())
        .collect();

    let has_external = !external.is_empty();

    // ---- Step 5: All-internal path ----
    if !has_external {
        if title_override == Some("one_on_one") || attendee_count == 2 {
            result.meeting_type = title_override.unwrap_or("one_on_one").to_string();
            return result;
        }

        if let Some(override_type) = title_override {
            result.meeting_type = override_type.to_string();
            return result;
        }

        // Team sync signals
        let sync_signals = ["sync", "standup", "stand-up", "scrum", "daily", "weekly"];
        if contains_any(&title_lower, &sync_signals) && event.is_recurring {
            result.meeting_type = "team_sync".to_string();
            return result;
        }

        result.meeting_type = "internal".to_string();
        return result;
    }

    // ---- Step 6: External path ----
    // Personal email domains only → personal event
    if !external_domains.is_empty()
        && external_domains
            .iter()
            .all(|d| PERSONAL_EMAIL_DOMAINS.contains(&d.as_str()))
    {
        result.meeting_type = "personal".to_string();
        return result;
    }

    // Record external domains
    result.external_domains = external_domains.iter().cloned().collect();
    result.external_domains.sort();

    // Try to match external domains to known accounts
    for domain in &external_domains {
        let domain_base = domain.split('.').next().unwrap_or(domain);
        for hint in account_hints {
            if hint == domain_base || (hint.len() >= 4 && domain_base.contains(hint.as_str())) {
                result.account = Some(hint.clone());
                break;
            }
        }
        if result.account.is_some() {
            break;
        }
    }

    // Apply title override if set (e.g., QBR with external attendees)
    if let Some(override_type) = title_override {
        result.meeting_type = override_type.to_string();
    } else if attendee_count == 2 {
        result.meeting_type = "one_on_one".to_string();
    } else {
        result.meeting_type = "customer".to_string();
    }

    result
}

/// Check if a string contains any of the given substrings.
fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

// ============================================================================
// Convert ClassifiedMeeting → CalendarEvent (for AppState storage)
// ============================================================================

impl ClassifiedMeeting {
    /// Convert to the CalendarEvent type used by AppState.
    pub fn to_calendar_event(&self) -> crate::types::CalendarEvent {
        use crate::types::MeetingType;

        let meeting_type = match self.meeting_type.as_str() {
            "customer" => MeetingType::Customer,
            "qbr" => MeetingType::Qbr,
            "training" => MeetingType::Training,
            "internal" => MeetingType::Internal,
            "team_sync" => MeetingType::TeamSync,
            "one_on_one" => MeetingType::OneOnOne,
            "partnership" => MeetingType::Partnership,
            "all_hands" => MeetingType::AllHands,
            "external" => MeetingType::External,
            "personal" => MeetingType::Personal,
            _ => MeetingType::Internal,
        };

        let start =
            super::calendar::parse_event_datetime(&self.start).unwrap_or_else(chrono::Utc::now);
        let end = super::calendar::parse_event_datetime(&self.end).unwrap_or_else(chrono::Utc::now);

        crate::types::CalendarEvent {
            id: self.id.clone(),
            title: self.title.clone(),
            start,
            end,
            meeting_type,
            account: self.account.clone(),
            attendees: self.attendees.clone(),
            is_all_day: self.is_all_day,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(title: &str, attendees: Vec<&str>, is_recurring: bool) -> GoogleCalendarEvent {
        GoogleCalendarEvent {
            id: "test-id".to_string(),
            summary: title.to_string(),
            start: "2026-02-08T09:00:00-05:00".to_string(),
            end: "2026-02-08T10:00:00-05:00".to_string(),
            attendees: attendees.iter().map(|s| s.to_string()).collect(),
            organizer: "me@company.com".to_string(),
            description: String::new(),
            location: String::new(),
            is_recurring,
            is_all_day: false,
        }
    }

    fn empty_hints() -> HashSet<String> {
        HashSet::new()
    }

    fn hints(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    // Step 1: Personal
    #[test]
    fn test_classify_no_attendees_is_personal() {
        let event = make_event("Lunch break", vec![], false);
        let result = classify_meeting(&event, "company.com", &empty_hints());
        assert_eq!(result.meeting_type, "personal");
    }

    #[test]
    fn test_classify_single_attendee_is_personal() {
        let event = make_event("Focus time", vec!["me@company.com"], false);
        let result = classify_meeting(&event, "company.com", &empty_hints());
        assert_eq!(result.meeting_type, "personal");
    }

    // Step 2: All-hands (scale)
    #[test]
    fn test_classify_50_attendees_is_all_hands() {
        let attendees: Vec<&str> = (0..50).map(|_| "person@company.com").collect();
        let event = make_event("Something", attendees, false);
        let result = classify_meeting(&event, "company.com", &empty_hints());
        assert_eq!(result.meeting_type, "all_hands");
    }

    // Step 3: Title keywords — all hands
    #[test]
    fn test_classify_all_hands_title() {
        let event = make_event("Company All Hands", vec!["a@c.com", "b@c.com"], false);
        let result = classify_meeting(&event, "c.com", &empty_hints());
        assert_eq!(result.meeting_type, "all_hands");
    }

    #[test]
    fn test_classify_town_hall_title() {
        let event = make_event("Q1 Town Hall", vec!["a@c.com", "b@c.com"], false);
        let result = classify_meeting(&event, "c.com", &empty_hints());
        assert_eq!(result.meeting_type, "all_hands");
    }

    // Step 3: Title keywords — qbr
    #[test]
    fn test_classify_qbr_title() {
        let event = make_event("Acme QBR", vec!["me@co.com", "them@acme.com"], false);
        let result = classify_meeting(&event, "co.com", &hints(&["acme"]));
        assert_eq!(result.meeting_type, "qbr");
    }

    #[test]
    fn test_classify_business_review_title() {
        let event = make_event(
            "Acme Business Review",
            vec!["me@co.com", "them@acme.com", "other@acme.com"],
            false,
        );
        let result = classify_meeting(&event, "co.com", &empty_hints());
        assert_eq!(result.meeting_type, "qbr");
    }

    // Step 3: Title keywords — training
    #[test]
    fn test_classify_training_title() {
        let event = make_event("Product Training", vec!["a@co.com", "b@co.com"], false);
        let result = classify_meeting(&event, "co.com", &empty_hints());
        assert_eq!(result.meeting_type, "training");
    }

    // Step 3: Title keywords — 1:1
    #[test]
    fn test_classify_1on1_title() {
        let event = make_event("Sarah / Bob 1:1", vec!["a@co.com", "b@co.com"], false);
        let result = classify_meeting(&event, "co.com", &empty_hints());
        assert_eq!(result.meeting_type, "one_on_one");
    }

    // Step 5: All-internal — one_on_one (2 people)
    #[test]
    fn test_classify_internal_2_people() {
        let event = make_event("Quick chat", vec!["a@co.com", "b@co.com"], false);
        let result = classify_meeting(&event, "co.com", &empty_hints());
        assert_eq!(result.meeting_type, "one_on_one");
    }

    // Step 5: All-internal — team_sync
    #[test]
    fn test_classify_team_sync() {
        let event = make_event(
            "Engineering Weekly Sync",
            vec!["a@co.com", "b@co.com", "c@co.com"],
            true,
        );
        let result = classify_meeting(&event, "co.com", &empty_hints());
        assert_eq!(result.meeting_type, "team_sync");
    }

    // Step 5: All-internal — internal (non-sync, 3+ people)
    #[test]
    fn test_classify_internal_meeting() {
        let event = make_event(
            "Project planning",
            vec!["a@co.com", "b@co.com", "c@co.com"],
            false,
        );
        let result = classify_meeting(&event, "co.com", &empty_hints());
        assert_eq!(result.meeting_type, "internal");
    }

    // Step 6: External — personal email domains
    #[test]
    fn test_classify_personal_email_external() {
        let event = make_event("Catch up", vec!["me@co.com", "friend@gmail.com"], false);
        let result = classify_meeting(&event, "co.com", &empty_hints());
        assert_eq!(result.meeting_type, "personal");
    }

    // Step 6: External — customer (domain match via hints)
    #[test]
    fn test_classify_customer_with_hint() {
        let event = make_event(
            "Acme sync",
            vec!["me@co.com", "contact@acme.com", "other@acme.com"],
            false,
        );
        let result = classify_meeting(&event, "co.com", &hints(&["acme"]));
        assert_eq!(result.meeting_type, "customer");
        assert_eq!(result.account.as_deref(), Some("acme"));
    }

    // Step 6: External — customer (default, no hint but external non-personal)
    #[test]
    fn test_classify_external_defaults_to_customer() {
        let event = make_event(
            "Check-in",
            vec!["me@co.com", "them@bigcorp.com", "other@bigcorp.com"],
            false,
        );
        let result = classify_meeting(&event, "co.com", &empty_hints());
        assert_eq!(result.meeting_type, "customer");
    }

    // Step 6: External — 2 people one_on_one
    #[test]
    fn test_classify_external_2_people_is_one_on_one() {
        let event = make_event("Quick call", vec!["me@co.com", "them@other.com"], false);
        let result = classify_meeting(&event, "co.com", &empty_hints());
        assert_eq!(result.meeting_type, "one_on_one");
    }

    // No domain: treats all as external
    #[test]
    fn test_classify_no_user_domain() {
        let event = make_event(
            "Unknown",
            vec!["a@foo.com", "b@bar.com", "c@baz.com"],
            false,
        );
        let result = classify_meeting(&event, "", &empty_hints());
        assert_eq!(result.meeting_type, "customer");
    }

    // to_calendar_event conversion
    #[test]
    fn test_to_calendar_event() {
        let event = make_event("Acme sync", vec!["me@co.com", "them@acme.com"], false);
        let classified = classify_meeting(&event, "co.com", &hints(&["acme"]));
        let cal_event = classified.to_calendar_event();
        assert_eq!(cal_event.meeting_type, crate::types::MeetingType::OneOnOne);
        assert_eq!(cal_event.title, "Acme sync");
    }
}

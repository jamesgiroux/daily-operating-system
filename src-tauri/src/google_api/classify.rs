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

use serde::{Deserialize, Serialize};

use super::calendar::GoogleCalendarEvent;

/// All-hands attendee threshold (per MEETING-TYPES.md).
pub const ALL_HANDS_THRESHOLD: usize = 50;

/// Entity hint for classification (built from DB). I336.
#[derive(Debug, Clone)]
pub struct EntityHint {
    pub id: String,
    pub entity_type: crate::entity::EntityType,
    pub name: String,
    pub slugs: Vec<String>,
    pub domains: Vec<String>,
    pub keywords: Vec<String>,
    pub emails: Vec<String>,
}

/// Resolved entity from classification. I336.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedMeetingEntity {
    pub entity_id: String,
    pub entity_type: String,
    pub name: String,
    pub confidence: f64,
    pub source: String,
}

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
    /// Resolved entities from multi-entity classification (I336).
    pub resolved_entities: Vec<ResolvedMeetingEntity>,
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
/// - `entity_hints`: Entity hints from DB for multi-entity resolution (I336).
pub fn classify_meeting(
    event: &GoogleCalendarEvent,
    user_domain: &str,
    entity_hints: &[EntityHint],
) -> ClassifiedMeeting {
    let domains: Vec<String> = if user_domain.is_empty() {
        Vec::new()
    } else {
        vec![user_domain.to_string()]
    };
    classify_meeting_multi(event, &domains, entity_hints)
}

/// Multi-domain meeting classification with entity resolution (I171 + I336).
///
/// Classifies attendees as internal if their domain matches ANY of `user_domains`.
/// Resolves meetings to accounts, projects, and people via entity hints.
pub fn classify_meeting_multi(
    event: &GoogleCalendarEvent,
    user_domains: &[String],
    entity_hints: &[EntityHint],
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
        resolved_entities: Vec::new(),
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

    let external_domains: std::collections::HashSet<String> = external
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

    // ---- Entity resolution (I336) ----
    resolve_entities(&mut result, entity_hints, user_domains, &external_domains);

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

/// Resolve entities from hints against meeting signals. I336.
fn resolve_entities(
    result: &mut ClassifiedMeeting,
    entity_hints: &[EntityHint],
    user_domains: &[String],
    external_domains: &std::collections::HashSet<String>,
) {
    use crate::entity::EntityType;
    use std::collections::HashMap;

    // Track best resolution per entity to dedup
    let mut best: HashMap<String, ResolvedMeetingEntity> = HashMap::new();

    let title_lower = result.title.to_lowercase();
    let desc_lower = result.description.to_lowercase();

    // Collect attendee emails for person matching
    let attendee_emails: Vec<String> = result.attendees.iter()
        .filter(|a| a.contains('@'))
        .map(|a| a.to_lowercase())
        .collect();

    for hint in entity_hints {
        let mut confidence = 0.0_f64;
        let mut source = String::new();

        match hint.entity_type {
            EntityType::Account => {
                // A. Domain matching (from account_domains table)
                for ext_domain in external_domains {
                    if hint.domains.iter().any(|d| d == ext_domain) && confidence < 0.80 {
                        confidence = 0.80;
                        source = "domain".to_string();
                    }
                }
                // Domain base heuristic (existing behavior)
                if confidence < 0.65 {
                    for ext_domain in external_domains {
                        let domain_base = ext_domain.split('.').next().unwrap_or(ext_domain);
                        for slug in &hint.slugs {
                            if slug == domain_base || (slug.len() >= 4 && domain_base.contains(slug.as_str())) {
                                confidence = 0.65;
                                source = "domain".to_string();
                            }
                        }
                    }
                }
                // B. Keyword matching
                if confidence < 0.70 {
                    for kw in &hint.keywords {
                        let kw_lower = kw.to_lowercase();
                        if title_lower.contains(&kw_lower) || desc_lower.contains(&kw_lower) {
                            confidence = 0.70;
                            source = "keyword".to_string();
                            break;
                        }
                    }
                }
                // C. Title slug matching
                if confidence < 0.50 {
                    for slug in &hint.slugs {
                        if slug.len() >= 4 && title_lower.contains(slug.as_str()) {
                            confidence = 0.50;
                            source = "title".to_string();
                            break;
                        }
                    }
                }
            }
            EntityType::Project => {
                // B. Keyword matching
                for kw in &hint.keywords {
                    let kw_lower = kw.to_lowercase();
                    if title_lower.contains(&kw_lower) || desc_lower.contains(&kw_lower) {
                        if confidence < 0.70 {
                            confidence = 0.70;
                            source = "keyword".to_string();
                        }
                        break;
                    }
                }
                // C. Title slug matching
                if confidence < 0.50 {
                    for slug in &hint.slugs {
                        if slug.len() >= 4 && title_lower.contains(slug.as_str()) {
                            confidence = 0.50;
                            source = "title".to_string();
                            break;
                        }
                    }
                }
            }
            EntityType::Person => {
                // D. 1:1 person detection
                if result.attendees.len() == 2 {
                    let is_1on1_pattern = result.is_recurring
                        || contains_any(&title_lower, &["1:1", "1-1", "one on one", "1-on-1"]);
                    if is_1on1_pattern {
                        // Find non-user attendee
                        for email in &attendee_emails {
                            let email_domain = email.split('@').nth(1).unwrap_or("");
                            let is_user = user_domains.iter().any(|d| !d.is_empty() && email_domain == d);
                            if !is_user && hint.emails.iter().any(|e| e.to_lowercase() == *email) {
                                confidence = if result.is_recurring { 0.90 } else { 0.85 };
                                source = "1:1".to_string();
                                break;
                            }
                        }
                    }
                }
            }
            EntityType::Other => {}
        }

        if confidence > 0.0 {
            let key = hint.id.clone();
            let candidate = ResolvedMeetingEntity {
                entity_id: hint.id.clone(),
                entity_type: hint.entity_type.as_str().to_string(),
                name: hint.name.clone(),
                confidence,
                source,
            };
            best.entry(key)
                .and_modify(|existing| {
                    if candidate.confidence > existing.confidence {
                        *existing = candidate.clone();
                    }
                })
                .or_insert(candidate);
        }
    }

    let mut entities: Vec<ResolvedMeetingEntity> = best.into_values().collect();
    entities.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    result.resolved_entities = entities;
}

/// Check if a string contains any of the given substrings.
fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

// ============================================================================
// Convert ClassifiedMeeting → CalendarEvent (for AppState storage)
// ============================================================================

impl ClassifiedMeeting {
    /// Extract account name from resolved entities (backward compat helper). I336.
    pub fn account(&self) -> Option<&str> {
        self.resolved_entities.iter()
            .find(|e| e.entity_type == "account")
            .map(|e| e.name.as_str())
    }

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

        let account = self.resolved_entities.iter()
            .find(|e| e.entity_type == "account")
            .map(|e| e.name.clone());

        crate::types::CalendarEvent {
            id: self.id.clone(),
            title: self.title.clone(),
            start,
            end,
            meeting_type,
            account,
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
    use crate::entity::EntityType;

    fn make_event(title: &str, attendees: Vec<&str>, is_recurring: bool) -> GoogleCalendarEvent {
        GoogleCalendarEvent {
            id: "test-id".to_string(),
            summary: title.to_string(),
            start: "2026-02-08T09:00:00-05:00".to_string(),
            end: "2026-02-08T10:00:00-05:00".to_string(),
            attendees: attendees.iter().map(|s| s.to_string()).collect(),
            attendee_rsvp: std::collections::HashMap::new(),
            attendee_names: std::collections::HashMap::new(),
            organizer: "me@company.com".to_string(),
            description: String::new(),
            location: String::new(),
            is_recurring,
            is_all_day: false,
        }
    }

    fn empty_hints() -> Vec<EntityHint> {
        Vec::new()
    }

    /// Build account entity hints from slug names (backward compat with old tests).
    fn account_hints(names: &[&str]) -> Vec<EntityHint> {
        names.iter().map(|s| EntityHint {
            id: s.to_string(),
            entity_type: EntityType::Account,
            name: s.to_string(),
            slugs: vec![s.to_string()],
            domains: vec![],
            keywords: vec![],
            emails: vec![],
        }).collect()
    }

    fn project_hint(id: &str, name: &str, keywords: &[&str]) -> EntityHint {
        EntityHint {
            id: id.to_string(),
            entity_type: EntityType::Project,
            name: name.to_string(),
            slugs: vec![name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()],
            domains: vec![],
            keywords: keywords.iter().map(|s| s.to_string()).collect(),
            emails: vec![],
        }
    }

    fn person_hint(id: &str, name: &str, emails: &[&str]) -> EntityHint {
        EntityHint {
            id: id.to_string(),
            entity_type: EntityType::Person,
            name: name.to_string(),
            slugs: vec![],
            domains: vec![],
            keywords: vec![],
            emails: emails.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn account_hint_with_domain(id: &str, name: &str, domains: &[&str]) -> EntityHint {
        EntityHint {
            id: id.to_string(),
            entity_type: EntityType::Account,
            name: name.to_string(),
            slugs: vec![name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()],
            domains: domains.iter().map(|s| s.to_string()).collect(),
            keywords: vec![],
            emails: vec![],
        }
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
        let result = classify_meeting(&event, "co.com", &account_hints(&["acme"]));
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

    // Step 6: External — customer (domain match via slug hints)
    #[test]
    fn test_classify_customer_with_hint() {
        let event = make_event(
            "Acme sync",
            vec!["me@co.com", "contact@acme.com", "other@acme.com"],
            false,
        );
        let result = classify_meeting(&event, "co.com", &account_hints(&["acme"]));
        assert_eq!(result.meeting_type, "customer");
        assert_eq!(result.account(), Some("acme"));
        assert!(!result.resolved_entities.is_empty());
        assert_eq!(result.resolved_entities[0].entity_type, "account");
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
        let classified = classify_meeting(&event, "co.com", &account_hints(&["acme"]));
        let cal_event = classified.to_calendar_event();
        assert_eq!(cal_event.meeting_type, crate::types::MeetingType::OneOnOne);
        assert_eq!(cal_event.title, "Acme sync");
    }

    // ---- I336: Entity-generic classification tests ----

    #[test]
    fn test_classify_account_domain_match() {
        let hints = vec![account_hint_with_domain("acme-id", "Acme Corp", &["acme.com"])];
        let event = make_event(
            "Kickoff call",
            vec!["me@co.com", "them@acme.com", "other@acme.com"],
            false,
        );
        let result = classify_meeting(&event, "co.com", &hints);
        assert_eq!(result.meeting_type, "customer");
        assert_eq!(result.account(), Some("Acme Corp"));
        assert_eq!(result.resolved_entities[0].source, "domain");
        assert!(result.resolved_entities[0].confidence >= 0.80);
    }

    #[test]
    fn test_classify_project_keyword_match() {
        let hints = vec![project_hint("proj-1", "Agentforce", &["agentforce", "agent force"])];
        let event = make_event(
            "Agentforce Demo Prep",
            vec!["me@co.com", "them@client.com"],
            false,
        );
        let result = classify_meeting(&event, "co.com", &hints);
        let project_entity = result.resolved_entities.iter()
            .find(|e| e.entity_type == "project");
        assert!(project_entity.is_some(), "Should resolve to project entity");
        assert_eq!(project_entity.unwrap().name, "Agentforce");
        assert_eq!(project_entity.unwrap().source, "keyword");
    }

    #[test]
    fn test_classify_1on1_person_detection() {
        let hints = vec![person_hint("person-1", "Jane Smith", &["jane@other.com"])];
        let event = make_event(
            "Weekly check-in",
            vec!["me@co.com", "jane@other.com"],
            true,
        );
        let result = classify_meeting(&event, "co.com", &hints);
        let person_entity = result.resolved_entities.iter()
            .find(|e| e.entity_type == "person");
        assert!(person_entity.is_some(), "Should resolve to person entity");
        assert_eq!(person_entity.unwrap().name, "Jane Smith");
        assert_eq!(person_entity.unwrap().source, "1:1");
        assert!(person_entity.unwrap().confidence >= 0.90, "Recurring 1:1 should have high confidence");
    }

    #[test]
    fn test_classify_1on1_person_non_recurring_title_pattern() {
        let hints = vec![person_hint("person-1", "Jane Smith", &["jane@other.com"])];
        let event = make_event(
            "Jane / Me 1:1",
            vec!["me@co.com", "jane@other.com"],
            false,
        );
        let result = classify_meeting(&event, "co.com", &hints);
        let person_entity = result.resolved_entities.iter()
            .find(|e| e.entity_type == "person");
        assert!(person_entity.is_some());
        assert!(person_entity.unwrap().confidence >= 0.85);
    }

    #[test]
    fn test_classify_multiple_entities() {
        let hints = vec![
            account_hint_with_domain("acme-id", "Acme Corp", &["acme.com"]),
            project_hint("proj-1", "Agentforce", &["agentforce"]),
        ];
        let event = make_event(
            "Acme Agentforce Review",
            vec!["me@co.com", "them@acme.com"],
            false,
        );
        let result = classify_meeting(&event, "co.com", &hints);
        assert!(result.resolved_entities.len() >= 2, "Should resolve both account and project");
        let types: Vec<&str> = result.resolved_entities.iter().map(|e| e.entity_type.as_str()).collect();
        assert!(types.contains(&"account"));
        assert!(types.contains(&"project"));
    }

    #[test]
    fn test_classify_confidence_ordering() {
        let hints = vec![
            account_hint_with_domain("acme-id", "Acme Corp", &["acme.com"]),
            project_hint("proj-1", "Acme Project", &["review"]),
        ];
        let event = make_event(
            "Acme review",
            vec!["me@co.com", "them@acme.com"],
            false,
        );
        let result = classify_meeting(&event, "co.com", &hints);
        assert!(result.resolved_entities.len() >= 2);
        // Domain match (0.80) should come before keyword match (0.70)
        assert!(result.resolved_entities[0].confidence >= result.resolved_entities[1].confidence);
    }
}

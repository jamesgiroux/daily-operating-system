use std::collections::HashSet;

use chrono::{NaiveDate, TimeZone, Utc};
use rusqlite::params;

use crate::db::ActionDb;
use crate::entity::EntityType;
use crate::google_api::classify::EntityHint;

/// SQL WHERE clause and params for filtering meetings on a local calendar day.
///
/// Meeting `start_time` is stored in two formats depending on the write path:
/// - Google Calendar poller: RFC3339 UTC (`2026-03-06T15:00:00+00:00`)
/// - Pipeline reconcile: local format (`2026-03-06 10:00 AM`)
///
/// A single range comparison can't handle both because space < 'T' in ASCII,
/// so we use an OR clause: one branch for RFC3339 (UTC-adjusted boundaries),
/// one for local-format strings (bare date prefix match).
///
/// Returns `(sql_fragment, param1, param2, param3)` for use as:
/// `WHERE {sql_fragment}` with params `[param1, param2, param3]`.
pub fn today_meeting_filter(tz: &chrono_tz::Tz) -> TodayMeetingFilter {
    let local_today = chrono::Local::now().date_naive();
    today_meeting_filter_for_date(local_today, tz)
}

pub struct TodayMeetingFilter {
    /// The local date string (e.g. "2026-03-06")
    pub date: String,
    /// Next local date string (e.g. "2026-03-07")
    pub next_date: String,
    /// UTC start boundary as RFC3339 (e.g. "2026-03-06T05:00:00+00:00")
    pub utc_start: String,
    /// UTC end boundary as RFC3339 (e.g. "2026-03-07T05:00:00+00:00")
    pub utc_end: String,
}

impl TodayMeetingFilter {
    /// SQL fragment for WHERE clause. Use with `params()`.
    ///
    /// Matches both local-format times (`2026-03-06 10:00 AM`) and
    /// RFC3339 UTC times (`2026-03-06T15:00:00+00:00`).
    pub fn sql(&self) -> &'static str {
        "(m.start_time >= ?1 AND m.start_time < ?2)"
    }

    /// Returns (date_str, next_date_prefix) for SQL params.
    /// The bare date comparison works for BOTH formats:
    /// - "2026-03-06 10:00 AM" >= "2026-03-06" ✓ (space > end-of-string)
    /// - "2026-03-06T15:00:00+00:00" >= "2026-03-06" ✓ (T > end-of-string)
    /// - "2026-03-06 10:00 AM" < "2026-03-07" ✓
    /// - "2026-03-06T15:00:00+00:00" < "2026-03-07" ✓
    ///
    /// Edge case: UTC-stored meetings after local midnight (e.g. 9 PM ET =
    /// 2026-03-07T02:00:00+00:00) are technically the next UTC date but still
    /// today in local time. These are caught by the calendar_merge overlay
    /// from live_events, which always has the current day's events.
    pub fn params(&self) -> [&str; 2] {
        [&self.date, &self.next_date]
    }
}

/// Build a TodayMeetingFilter for an arbitrary local date.
pub fn today_meeting_filter_for_date(date: NaiveDate, tz: &chrono_tz::Tz) -> TodayMeetingFilter {
    let next = date + chrono::Duration::days(1);
    let day_start = date.and_hms_opt(0, 0, 0).unwrap();
    let day_end = next.and_hms_opt(0, 0, 0).unwrap();
    let utc_start = tz
        .from_local_datetime(&day_start)
        .earliest()
        .map(|dt| dt.with_timezone(&Utc).to_rfc3339())
        .unwrap_or_else(|| format!("{}T00:00:00+00:00", date));
    let utc_end = tz
        .from_local_datetime(&day_end)
        .earliest()
        .map(|dt| dt.with_timezone(&Utc).to_rfc3339())
        .unwrap_or_else(|| format!("{}T00:00:00+00:00", next));
    TodayMeetingFilter {
        date: date.to_string(),
        next_date: next.to_string(),
        utc_start,
        utc_end,
    }
}

/// Normalize a string for fuzzy matching: lowercase + ASCII alphanumeric only.
pub fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// Normalize a list of domain strings: trim, lowercase, dedupe, sort.
pub fn normalize_domains(domains: &[String]) -> Vec<String> {
    let mut out: Vec<String> = domains
        .iter()
        .map(|d| d.trim().to_lowercase())
        .filter(|d| !d.is_empty())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Build entity hints from DB for multi-entity meeting classification (I336).
pub fn build_entity_hints(db: &ActionDb) -> Vec<EntityHint> {
    let mut hints = Vec::new();

    // 1. Accounts: name slugs + domains (account_domains table) + keywords
    if let Ok(accounts) = db.get_all_accounts() {
        for acct in accounts.iter().filter(|a| !a.archived) {
            let domains = db.get_account_domains(&acct.id).unwrap_or_default();
            let keywords = acct
                .keywords
                .as_deref()
                .and_then(|k| serde_json::from_str::<Vec<String>>(k).ok())
                .unwrap_or_default();
            let slug = normalize_key(&acct.name);
            if slug.len() >= 3 || !domains.is_empty() || !keywords.is_empty() {
                hints.push(EntityHint {
                    id: acct.id.clone(),
                    entity_type: EntityType::Account,
                    name: acct.name.clone(),
                    slugs: if slug.len() >= 3 { vec![slug] } else { vec![] },
                    domains,
                    keywords,
                    emails: vec![],
                    account_type: Some(acct.account_type.as_db_str().to_string()),
                    linked_account_ids: vec![],
                });
            }
        }
    }

    // 2. Projects: name slugs + keywords
    if let Ok(projects) = db.get_all_projects() {
        for proj in projects.iter().filter(|p| !p.archived) {
            let keywords = proj
                .keywords
                .as_deref()
                .and_then(|k| serde_json::from_str::<Vec<String>>(k).ok())
                .unwrap_or_default();
            let slug = normalize_key(&proj.name);
            if slug.len() >= 3 || !keywords.is_empty() {
                hints.push(EntityHint {
                    id: proj.id.clone(),
                    entity_type: EntityType::Project,
                    name: proj.name.clone(),
                    slugs: if slug.len() >= 3 { vec![slug] } else { vec![] },
                    domains: vec![],
                    keywords,
                    emails: vec![],
                    account_type: None,
                    linked_account_ids: vec![],
                });
            }
        }
    }

    // 3. People: email for 1:1 attendee matching
    if let Ok(people) = db.get_people(None) {
        for person in people.iter().filter(|p| !p.archived) {
            let mut emails = vec![person.email.clone()];
            // Also include aliases
            if let Ok(aliases) = db.get_person_emails(&person.id) {
                for alias in aliases {
                    if alias != person.email {
                        emails.push(alias);
                    }
                }
            }
            // I653 FIX 8: Include linked account IDs for classification-time chaining.
            // When a known stakeholder attends a meeting, their linked account
            // gets a confidence boost in resolve_entities.
            let linked_account_ids: Vec<String> = db
                .get_entities_for_person(&person.id)
                .unwrap_or_default()
                .into_iter()
                .map(|e| e.id)
                .collect();

            hints.push(EntityHint {
                id: person.id.clone(),
                entity_type: EntityType::Person,
                name: person.name.clone(),
                slugs: vec![],
                domains: vec![],
                keywords: vec![],
                emails,
                account_type: None,
                linked_account_ids,
            });
        }
    }

    hints
}

/// Build account hint set for email classification (backward compat). I336.
/// Extracts account slugs from entity hints for use by email_classify.
pub fn account_hints_from_entity_hints(entity_hints: &[EntityHint]) -> HashSet<String> {
    entity_hints
        .iter()
        .filter(|h| matches!(h.entity_type, EntityType::Account))
        .flat_map(|h| h.slugs.iter().cloned())
        .collect()
}

/// Build account hint set for meeting classification (legacy — delegates to entity hints).
pub fn build_external_account_hints(db: &ActionDb) -> HashSet<String> {
    account_hints_from_entity_hints(&build_entity_hints(db))
}

// ---------------------------------------------------------------------------
// Text similarity (DOS-15: auto-link actions to objectives)
// ---------------------------------------------------------------------------

/// Jaccard word similarity: proportion of shared words between two strings.
/// Returns 0.0 for empty inputs, 1.0 for identical word sets.
pub fn jaccard_word_similarity(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let a_tokens: HashSet<&str> = a_lower.split_whitespace().collect();
    let b_tokens: HashSet<&str> = b_lower.split_whitespace().collect();
    let intersection = a_tokens.intersection(&b_tokens).count();
    let union = a_tokens.union(&b_tokens).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

// ---------------------------------------------------------------------------
// Entity name resolution (unified from signals/callouts + proactive/detectors)
// ---------------------------------------------------------------------------

/// Resolve a display name for an entity from accounts, projects, or people tables.
///
/// Returns the entity name if found, or falls back to `entity_id` as a string.
pub fn resolve_entity_name(db: &ActionDb, entity_type: &str, entity_id: &str) -> String {
    let (table, col) = match entity_type {
        "account" => ("accounts", "name"),
        "project" => ("projects", "name"),
        "person" => ("people", "name"),
        _ => return entity_id.to_string(),
    };
    let sql = format!("SELECT {} FROM {} WHERE id = ?1", col, table);
    db.conn_ref()
        .query_row(&sql, params![entity_id], |row| row.get::<_, String>(0))
        .unwrap_or_else(|_| entity_id.to_string())
}

// ---------------------------------------------------------------------------
// Attendee email parsing (unified from signals/patterns, email_bridge, post_meeting)
// ---------------------------------------------------------------------------

/// Parse attendee emails from a DB-stored string (comma-separated or JSON array).
///
/// Normalizes to lowercase and filters to valid-looking email addresses.
pub fn parse_attendee_emails(raw: &str) -> Vec<String> {
    // Try JSON array first
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(raw) {
        return arr
            .into_iter()
            .map(|e| e.trim().to_lowercase())
            .filter(|e| e.contains('@'))
            .collect();
    }
    // Fall back to comma-separated
    raw.split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| s.contains('@'))
        .collect()
}

/// Extract attendee emails from a meeting JSON value's "attendees" array field.
pub fn extract_attendee_emails(meeting: &serde_json::Value) -> Vec<String> {
    if let Some(arr) = meeting.get("attendees").and_then(|v| v.as_array()) {
        return arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.trim().to_lowercase())
            .filter(|s| s.contains('@'))
            .collect();
    }
    Vec::new()
}

// ---------------------------------------------------------------------------
// Calendar description cleaning (shared by deliver.rs + prompts.rs)
// ---------------------------------------------------------------------------

/// Strip video-conferencing noise from a calendar event description.
///
/// Removes Zoom, Teams, Google Meet, WebEx blocks and dial-in metadata,
/// returning only the meaningful meeting context (agenda, discussion points, etc.).
pub fn strip_conferencing_noise(description: &str) -> String {
    let mut out = Vec::new();
    let mut in_conf_block = false;

    for line in description.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();

        // Detect conferencing block headers — set skip flag
        if lower.contains("join zoom meeting")
            || lower.contains("join microsoft teams meeting")
            || lower.contains("join with a video conferencing device")
            || lower.contains("join with google meet")
            || lower.contains("join webex meeting")
            || lower.starts_with("microsoft teams meeting")
            || lower.starts_with("microsoft teams need help?")
        {
            in_conf_block = true;
            continue;
        }

        // Blank line ends a conferencing block
        if trimmed.is_empty() {
            if in_conf_block {
                in_conf_block = false;
            } else {
                out.push(String::new());
            }
            continue;
        }

        // Skip lines inside a conferencing block
        if in_conf_block {
            continue;
        }

        // Skip individual noise lines (URLs, dial-in metadata)
        if is_conferencing_noise_line(trimmed) {
            continue;
        }

        out.push(line.to_string());
    }

    // Trim trailing blank lines
    while out.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
        out.pop();
    }

    out.join("\n")
}

/// Returns true if a single line is conferencing noise (URL-only, dial-in info, etc.).
fn is_conferencing_noise_line(trimmed: &str) -> bool {
    let lower = trimmed.to_lowercase();

    // Pure URL lines
    if (lower.starts_with("http://") || lower.starts_with("https://")) && !trimmed.contains(' ') {
        return true;
    }

    // Conferencing-specific URL domains
    if lower.contains("zoom.us/")
        || lower.contains("meet.google.com/")
        || lower.contains(".webex.com/")
        || lower.contains("teams.microsoft.com/")
    {
        return true;
    }

    // Dial-in metadata patterns
    if lower.starts_with("meeting id:")
        || lower.starts_with("passcode:")
        || lower.starts_with("password:")
        || lower.starts_with("pin:")
        || lower.starts_with("dial-in:")
        || lower.starts_with("dial in:")
        || lower.starts_with("one tap mobile")
        || lower.starts_with("find your local number")
        || lower.starts_with("phone one-tap:")
        || lower.starts_with("tap to join from a mobile device")
    {
        return true;
    }

    // Phone numbers with access codes (e.g., "+1 555-1234,,12345#")
    if trimmed.starts_with('+')
        && trimmed.len() > 6
        && trimmed.chars().filter(|c| c.is_ascii_digit()).count() > 6
    {
        return true;
    }

    // Lines that are just separators (dashes, underscores, equals)
    if trimmed.len() >= 3 && trimmed.chars().all(|c| c == '-' || c == '_' || c == '=') {
        return true;
    }

    false
}

#[cfg(test)]
mod conferencing_noise_tests {
    use super::*;

    #[test]
    fn test_strip_zoom_block() {
        let desc = "Discuss Q1 goals\n\nJoin Zoom Meeting\nhttps://zoom.us/j/12345\nMeeting ID: 123 456\nPasscode: abc\n\nPlease come prepared.";
        let cleaned = strip_conferencing_noise(desc);
        assert!(cleaned.contains("Discuss Q1 goals"));
        assert!(cleaned.contains("Please come prepared"));
        assert!(!cleaned.contains("Zoom"));
        assert!(!cleaned.contains("Passcode"));
    }

    #[test]
    fn test_strip_teams_block() {
        let desc = "1. Review pipeline\n2. Budget update\n\nJoin Microsoft Teams Meeting\nhttps://teams.microsoft.com/l/meetup\n+1 234-567-8901,,12345#\n\nMicrosoft Teams Need help?";
        let cleaned = strip_conferencing_noise(desc);
        assert!(cleaned.contains("Review pipeline"));
        assert!(cleaned.contains("Budget update"));
        assert!(!cleaned.contains("Teams"));
    }

    #[test]
    fn test_strip_google_meet_url() {
        let desc = "Agenda: Review proposal\nhttps://meet.google.com/abc-defg-hij";
        let cleaned = strip_conferencing_noise(desc);
        assert!(cleaned.contains("Agenda: Review proposal"));
        assert!(!cleaned.contains("meet.google.com"));
    }

    #[test]
    fn test_pure_zoom_returns_empty() {
        let desc = "Join Zoom Meeting\nhttps://zoom.us/j/12345\nMeeting ID: 123 456\nPasscode: abc";
        let cleaned = strip_conferencing_noise(desc);
        assert!(cleaned.trim().is_empty());
    }

    #[test]
    fn test_no_noise_passes_through() {
        let desc = "Here's the planned agenda:\n1. Globex DMT overview\n2. Review Q1 pipeline\n3. Discuss expansion";
        let cleaned = strip_conferencing_noise(desc);
        assert_eq!(cleaned, desc);
    }

    #[test]
    fn test_phone_number_stripped() {
        let desc = "Discuss roadmap\n+1 555-123-4567,,99887766#\nDial-in: 555-1234";
        let cleaned = strip_conferencing_noise(desc);
        assert!(cleaned.contains("Discuss roadmap"));
        assert!(!cleaned.contains("+1 555"));
        assert!(!cleaned.contains("Dial-in"));
    }
}

#[cfg(test)]
mod jaccard_tests {
    use super::*;

    #[test]
    fn test_identical_strings() {
        let score = jaccard_word_similarity("onboard customer", "onboard customer");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_no_overlap() {
        let score = jaccard_word_similarity("alpha beta", "gamma delta");
        assert!(score.abs() < f64::EPSILON);
    }

    #[test]
    fn test_partial_overlap() {
        // "complete" and "onboarding" shared; "customer" only in a, "setup" only in b
        // a = {complete, customer, onboarding}, b = {complete, onboarding, setup}
        // intersection = 2, union = 4 => 0.5
        let score = jaccard_word_similarity("Complete customer onboarding", "Complete onboarding setup");
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_empty_strings() {
        assert!(jaccard_word_similarity("", "").abs() < f64::EPSILON);
        assert!(jaccard_word_similarity("hello", "").abs() < f64::EPSILON);
    }

    #[test]
    fn test_case_insensitive() {
        let score = jaccard_word_similarity("Hello World", "hello world");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }
}

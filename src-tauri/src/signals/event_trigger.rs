//! Entity-trigger helpers retained after the signal invalidation rewrite.
//!
//! The legacy event-driven entity resolution trigger (ADR-0080 Phase 4)
//! was removed here. Entity linking now runs on every calendar poll via
//! `services::entity_linking::calendar_adapter::evaluate_meeting`, which
//! writes to `linked_entities_raw` instead of the fuzzy/keyword resolver's
//! `meeting_entities` table.
//!
//! What remains in this module:
//! - `extract_domains_from_attendees`: domain extraction still used by
//!   `commands::planning_reports` and `workflow::recover`.
//! - `ActionDb::get_account_meetings_for_domain_backfill`: read path used
//!   by `commands::planning_reports` for the domain backfill command.
//! - `ActionDb::link_meeting_entity_if_absent`: helper still called from
//!   `services::meetings` for legacy `meeting_entities` writes that have
//!   not yet been cut over. Retained as a neutral DB helper.

/// Extract unique domains from attendee email addresses.
/// Handles both valid emails and malformed strings gracefully.
/// Extract unique domains from attendee email addresses, filtering out
/// the user's own company domains and personal email providers.
pub fn extract_domains_from_attendees(
    attendees: &[String],
    user_domains: &[String],
) -> Vec<String> {
    use crate::google_api::classify::PERSONAL_EMAIL_DOMAINS;

    let mut domains = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for email in attendees {
        if let Some(domain_part) = email.split('@').nth(1) {
            let domain = domain_part.to_lowercase();
            // Exclude the user's own company domains. Without this filter,
            // the CSM's domain gets stored as a domain for every customer
            // account, causing every meeting to resolve to every account.
            // Also exclude personal email providers (gmail, outlook, etc.)
            // which don't represent organizational domains.
            if !domain.is_empty()
                && !domain.contains(' ')
                && !user_domains.iter().any(|ud| ud == &domain)
                && !PERSONAL_EMAIL_DOMAINS.contains(&domain.as_str())
                && seen.insert(domain.clone())
            {
                domains.push(domain);
            }
        }
    }

    domains
}

// ---------------------------------------------------------------------------
// ActionDb methods
// ---------------------------------------------------------------------------

impl crate::db::ActionDb {
    /// Get all meetings linked to accounts, with attendees, for domain backfill.
    ///
    /// Returns (account_id, meeting_attendees) pairs for every meeting→account link.
    /// Used by the backfill command to populate account_domains from historical data.
    pub fn get_account_meetings_for_domain_backfill(
        &self,
    ) -> Result<Vec<(String, String)>, crate::db::DbError> {
        let mut stmt = self.conn_ref().prepare(
            "SELECT me.entity_id, m.attendees
             FROM meeting_entities me
             INNER JOIN meetings m ON m.id = me.meeting_id
             WHERE me.entity_type = 'account'
               AND m.attendees IS NOT NULL
               AND m.attendees != ''",
        )?;
        let rows = stmt.query_map(rusqlite::params![], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Link a meeting to an entity if not already linked.
    pub fn link_meeting_entity_if_absent(
        &self,
        meeting_id: &str,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<bool, crate::db::DbError> {
        let already: bool = self
            .conn_ref()
            .prepare("SELECT 1 FROM meeting_entities WHERE meeting_id = ?1 AND entity_id = ?2")
            .and_then(|mut s| s.exists(rusqlite::params![meeting_id, entity_id]))
            .unwrap_or(false);

        if already {
            return Ok(false);
        }

        self.conn_ref().execute(
            "INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![meeting_id, entity_id, entity_type],
        )?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::extract_domains_from_attendees;

    #[test]
    fn test_extract_domains_filters_personal_emails() {
        let attendees = vec![
            "me@company.com".to_string(),
            "contact@acme.com".to_string(),
            "friend@gmail.com".to_string(),
            "other@outlook.com".to_string(),
            "buyer@bigcorp.com".to_string(),
        ];
        let user_domains = vec!["company.com".to_string()];
        let result = extract_domains_from_attendees(&attendees, &user_domains);

        assert!(result.contains(&"acme.com".to_string()));
        assert!(result.contains(&"bigcorp.com".to_string()));
        assert!(!result.contains(&"company.com".to_string()), "user domain excluded");
        assert!(!result.contains(&"gmail.com".to_string()), "personal email excluded");
        assert!(!result.contains(&"outlook.com".to_string()), "personal email excluded");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_extract_domains_deduplicates() {
        let attendees = vec![
            "alice@acme.com".to_string(),
            "bob@acme.com".to_string(),
            "charlie@acme.com".to_string(),
        ];
        let user_domains = vec!["company.com".to_string()];
        let result = extract_domains_from_attendees(&attendees, &user_domains);

        assert_eq!(result, vec!["acme.com".to_string()]);
    }
}

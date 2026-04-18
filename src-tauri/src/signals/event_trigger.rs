//! Event-driven entity resolution trigger (I308 — ADR-0080 Phase 4).
//!
//! Background task that watches for newly-created meetings and triggers
//! entity resolution when they appear. Uses a Notify wake signal from
//! the calendar reconcile loop plus a 5-minute fallback poll.

use std::collections::HashMap;
use std::sync::Arc;

use crate::state::AppState;

/// Background task: waits for entity resolution wake signal or polls every 5 min.
/// Queries recently-created meetings without resolution signals and runs
/// entity resolution on them.
pub async fn run_entity_resolution_trigger(state: Arc<AppState>) {
    // Startup delay
    tokio::time::sleep(tokio::time::Duration::from_secs(45)).await;

    log::info!("Entity resolution trigger: started");

    loop {
        // Wait for wake signal or 5-minute timeout
        tokio::select! {
            _ = state.signals.entity_resolution_wake.notified() => {
                log::debug!("Entity resolution trigger: woken by reconcile signal");
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {
                log::debug!("Entity resolution trigger: periodic poll");
            }
        }

        // Dev mode isolation: pause background processing while dev sandbox is active
        if crate::db::is_dev_db_mode() {
            continue;
        }

        // Run entity resolution on meetings needing it
        if let Err(e) = resolve_new_meetings(&state) {
            log::warn!("Entity resolution trigger: {}", e);
        }
    }
}

/// Find meetings created in the last 30 minutes without entity resolution
/// signals and run resolution on them.
fn resolve_new_meetings(state: &AppState) -> Result<(), String> {
    let config = state.config.read().clone();
    let workspace = match config.as_ref() {
        Some(c) => std::path::PathBuf::from(&c.workspace_path),
        None => return Ok(()),
    };
    let accounts_dir = workspace.join("Accounts");
    let user_domains = config
        .as_ref()
        .map(|c| c.resolved_user_domains())
        .unwrap_or_default();

    let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

    // I653: Widened from 30 minutes to 7 days (10080 minutes) so meetings
    // created during app downtime still get entity resolution on restart.
    let meetings = db
        .get_meetings_needing_resolution(10080)
        .map_err(|e| format!("Failed to query meetings: {}", e))?;

    if meetings.is_empty() {
        return Ok(());
    }

    log::info!(
        "Entity resolution trigger: {} meetings need resolution",
        meetings.len()
    );

    let embedding_ref = state.embedding_model.as_ref();

    for meeting in &meetings {
        // Parse attendees from storage (JSON array string or comma-separated).
        // Attendees in meetings table are stored as Option<String>, either:
        //   - JSON array: ["alice@acme.com", "bob@partner.com"]
        //   - Comma-separated: alice@acme.com, bob@partner.com
        // We need to parse into a proper JSON array for the resolver.
        let attendees_array: Vec<String> = match &meeting.attendees {
            Some(attendees_str) => {
                // Try parsing as JSON array first
                if let Ok(arr) = serde_json::from_str::<Vec<String>>(attendees_str) {
                    arr
                } else {
                    // Fall back to comma-separated parsing
                    attendees_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                }
            }
            None => Vec::new(),
        };

        // Build a minimal meeting Value for the resolver
        let meeting_json = serde_json::json!({
            "id": meeting.id,
            "summary": meeting.title,
            "title": meeting.title,
            "attendees": attendees_array,
        });

        let outcomes = crate::prepare::entity_resolver::resolve_meeting_entities(
            &db,
            &meeting.id,
            &meeting_json,
            &accounts_dir,
            Some(embedding_ref),
        );

        // Auto-link only high-confidence `Resolved` outcomes and pick at most
        // one entity per type to avoid multi-account contamination.
        let selected = select_auto_link_candidates(&outcomes);

        // DOS-74: Gather primary IDs so suggestion candidates don't duplicate
        // entities we already auto-linked as primaries.
        let primary_ids: std::collections::HashSet<String> =
            selected.iter().map(|e| e.entity_id.clone()).collect();
        let suggestions = select_suggestion_candidates(&outcomes, &primary_ids);

        let linked_account_id: Option<String> = selected
            .iter()
            .find(|e| e.entity_type.as_str() == "account")
            .map(|e| e.entity_id.clone());

        // DOS-74: Persist scored candidates so junction rows carry confidence
        // + is_primary. Primaries auto-link at full confidence; suggestions
        // write at their signal confidence with is_primary = false.
        let mut candidates: Vec<crate::services::meetings::EntityLinkCandidate> = Vec::new();
        for entity in &selected {
            candidates.push(crate::services::meetings::EntityLinkCandidate {
                entity_id: entity.entity_id.clone(),
                entity_type: entity.entity_type.as_str().to_string(),
                confidence: entity.confidence.max(0.95),
                is_primary: true,
            });
        }
        for entity in &suggestions {
            candidates.push(crate::services::meetings::EntityLinkCandidate {
                entity_id: entity.entity_id.clone(),
                entity_type: entity.entity_type.as_str().to_string(),
                confidence: entity.confidence,
                is_primary: false,
            });
        }

        let linked = crate::services::meetings::persist_and_invalidate_entity_links_sync_scored(
            &db,
            &meeting.id,
            &candidates,
            &state.meeting_prep_queue,
            &state.integrations.prep_queue_wake,
        )
        .unwrap_or(0);

        if linked > 0 {
            log::debug!(
                "Entity resolution trigger: linked {} entities for meeting '{}'",
                linked,
                meeting.title,
            );

            // Path 2a: Extract domains from attendee emails and store in account_domains.
            // This ensures newly-linked accounts gain domain knowledge for future matching.
            // Uses merge (additive) rather than set (replace-all) so domains from
            // multiple meetings accumulate rather than clobber each other.
            if let Some(account_id) = linked_account_id {
                let discovered_domains =
                    extract_domains_from_attendees(&attendees_array, &user_domains);
                if !discovered_domains.is_empty() {
                    if let Err(e) = db.merge_account_domains(&account_id, &discovered_domains) {
                        log::warn!(
                            "Entity resolution trigger: failed to store domains for account {}: {}",
                            account_id,
                            e
                        );
                    } else {
                        log::debug!(
                            "Entity resolution trigger: stored {} domains for account '{}'",
                            discovered_domains.len(),
                            account_id
                        );
                        // Emit signal for audit trail
                        let _ = crate::signals::bus::emit_signal(
                            &db,
                            "account",
                            &account_id,
                            "account_domains_updated",
                            "entity_resolution",
                            Some(&format!("{} domains", discovered_domains.len())),
                            0.9,
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

/// Base threshold from ADR-0080 for auto-linking.
const AUTO_LINK_BASE_CONFIDENCE: f64 = 0.85;

fn source_rank(source: &str) -> i32 {
    match source {
        "junction" => 6,
        "keyword" => 5,
        "group_pattern" => 4,
        "attendee_vote" => 3,
        "embedding" => 2,
        "keyword_fuzzy" => 1,
        _ => 0,
    }
}

/// Additional guardrails by source to reduce false-positive auto-links.
fn source_min_confidence(source: &str) -> f64 {
    match source {
        "junction" | "keyword" => 0.85,
        "group_pattern" => 0.88,
        "attendee_vote" => 0.93,
        "embedding" => 0.95,
        "keyword_fuzzy" => 0.96,
        _ => 0.95,
    }
}

fn is_auto_link_candidate(entity: &crate::prepare::entity_resolver::ResolvedEntity) -> bool {
    entity.confidence >= AUTO_LINK_BASE_CONFIDENCE
        && entity.confidence >= source_min_confidence(&entity.source)
}

/// Select at most one high-confidence auto-link candidate per entity type.
fn select_auto_link_candidates(
    outcomes: &[crate::prepare::entity_resolver::ResolutionOutcome],
) -> Vec<crate::prepare::entity_resolver::ResolvedEntity> {
    let mut best_by_type: HashMap<String, crate::prepare::entity_resolver::ResolvedEntity> =
        HashMap::new();

    for outcome in outcomes {
        let entity = match outcome {
            crate::prepare::entity_resolver::ResolutionOutcome::Resolved(e) => e,
            _ => continue,
        };

        if !is_auto_link_candidate(entity) {
            continue;
        }

        let key = entity.entity_type.as_str().to_string();
        let replace = match best_by_type.get(&key) {
            None => true,
            Some(existing) => {
                entity.confidence > existing.confidence
                    || (entity.confidence == existing.confidence
                        && source_rank(&entity.source) > source_rank(&existing.source))
            }
        };
        if replace {
            best_by_type.insert(key, entity.clone());
        }
    }

    best_by_type.into_values().collect()
}

/// DOS-74: Select suggestion-tier outcomes that should be persisted as
/// non-primary junction rows. These render as muted "suggested" chips in the
/// UI rather than co-equal primary entities. Suggestions never cross to
/// auto-linked — they exist purely for disambiguation affordance.
fn select_suggestion_candidates(
    outcomes: &[crate::prepare::entity_resolver::ResolutionOutcome],
    primary_ids: &std::collections::HashSet<String>,
) -> Vec<crate::prepare::entity_resolver::ResolvedEntity> {
    outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            crate::prepare::entity_resolver::ResolutionOutcome::Suggestion(e) => {
                if primary_ids.contains(&e.entity_id) {
                    None
                } else {
                    Some(e.clone())
                }
            }
            _ => None,
        })
        .collect()
}

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

/// Minimal meeting info for resolution trigger.
pub struct MeetingForResolution {
    pub id: String,
    pub title: String,
    pub attendees: Option<String>,
}

// ---------------------------------------------------------------------------
// ActionDb methods
// ---------------------------------------------------------------------------

impl crate::db::ActionDb {
    /// Get meetings created in the last N minutes that have no entity resolution signal.
    pub fn get_meetings_needing_resolution(
        &self,
        since_minutes: i32,
    ) -> Result<Vec<MeetingForResolution>, crate::db::DbError> {
        let since_param = format!("-{} minutes", since_minutes);
        let mut stmt = self.conn_ref().prepare(
            "SELECT mh.id, mh.title, mh.attendees
             FROM meetings mh
             WHERE mh.created_at >= datetime('now', ?1)
               AND NOT EXISTS (
                   SELECT 1 FROM signal_events se
                   WHERE se.entity_id = mh.id
                     AND se.signal_type = 'entity_resolution'
               )
               AND NOT EXISTS (
                   SELECT 1 FROM meeting_entities me
                   WHERE me.meeting_id = mh.id
               )",
        )?;

        let rows = stmt.query_map(rusqlite::params![since_param], |row| {
            Ok(MeetingForResolution {
                id: row.get(0)?,
                title: row.get(1)?,
                attendees: row.get(2)?,
            })
        })?;

        let mut meetings = Vec::new();
        for row in rows {
            meetings.push(row?);
        }
        Ok(meetings)
    }

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
    use crate::entity::EntityType;
    use crate::prepare::entity_resolver::{ResolutionOutcome, ResolvedEntity};

    use super::{extract_domains_from_attendees, select_auto_link_candidates};

    #[test]
    fn auto_link_excludes_resolved_with_flag() {
        let outcomes = vec![ResolutionOutcome::ResolvedWithFlag(ResolvedEntity {
            entity_id: "acc1".to_string(),
            entity_type: EntityType::Account,
            confidence: 0.9,
            source: "keyword".to_string(),
        })];

        let selected = select_auto_link_candidates(&outcomes);
        assert!(selected.is_empty());
    }

    #[test]
    fn auto_link_picks_single_best_per_type() {
        let outcomes = vec![
            ResolutionOutcome::Resolved(ResolvedEntity {
                entity_id: "acc-a".to_string(),
                entity_type: EntityType::Account,
                confidence: 0.89,
                source: "group_pattern".to_string(),
            }),
            ResolutionOutcome::Resolved(ResolvedEntity {
                entity_id: "acc-b".to_string(),
                entity_type: EntityType::Account,
                confidence: 0.91,
                source: "keyword".to_string(),
            }),
            ResolutionOutcome::Resolved(ResolvedEntity {
                entity_id: "proj-a".to_string(),
                entity_type: EntityType::Project,
                confidence: 0.9,
                source: "keyword".to_string(),
            }),
        ];

        let mut selected = select_auto_link_candidates(&outcomes);
        selected.sort_by(|a, b| a.entity_type.as_str().cmp(b.entity_type.as_str()));
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].entity_id, "acc-b");
        assert_eq!(selected[1].entity_id, "proj-a");
    }

    #[test]
    fn auto_link_rejects_weak_source_with_low_confidence() {
        let outcomes = vec![ResolutionOutcome::Resolved(ResolvedEntity {
            entity_id: "acc1".to_string(),
            entity_type: EntityType::Account,
            confidence: 0.89,
            source: "attendee_vote".to_string(),
        })];

        let selected = select_auto_link_candidates(&outcomes);
        assert!(selected.is_empty());
    }

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

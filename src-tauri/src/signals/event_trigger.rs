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
    let config = state.config.read().ok().and_then(|g| g.clone());
    let workspace = match config.as_ref() {
        Some(c) => std::path::PathBuf::from(&c.workspace_path),
        None => return Ok(()),
    };
    let accounts_dir = workspace.join("Accounts");

    let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

    let meetings = db
        .get_meetings_needing_resolution(30)
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

        let mut linked = 0;
        for entity in &selected {
            let _ = db.link_meeting_entity_if_absent(
                &meeting.id,
                &entity.entity_id,
                entity.entity_type.as_str(),
            );
            linked += 1;
        }
        if linked > 0 {
            log::debug!(
                "Entity resolution trigger: linked {} entities for meeting '{}'",
                linked,
                meeting.title,
            );
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

    use super::select_auto_link_candidates;

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
}

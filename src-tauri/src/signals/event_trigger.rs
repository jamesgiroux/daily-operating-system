//! Event-driven entity resolution trigger (I308 — ADR-0080 Phase 4).
//!
//! Background task that watches for newly-created meetings and triggers
//! entity resolution when they appear. Uses a Notify wake signal from
//! the calendar reconcile loop plus a 5-minute fallback poll.

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
            _ = state.entity_resolution_wake.notified() => {
                log::debug!("Entity resolution trigger: woken by reconcile signal");
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {
                log::debug!("Entity resolution trigger: periodic poll");
            }
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
    let config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.clone());
    let workspace = match config.as_ref() {
        Some(c) => std::path::PathBuf::from(&c.workspace_path),
        None => return Ok(()),
    };
    let accounts_dir = workspace.join("Accounts");

    let guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
    let db = guard.as_ref().ok_or("Database unavailable")?;

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
        // Build a minimal meeting Value for the resolver
        let meeting_json = serde_json::json!({
            "id": meeting.id,
            "summary": meeting.title,
            "title": meeting.title,
            "attendees": meeting.attendees,
        });

        let outcomes = crate::prepare::entity_resolver::resolve_meeting_entities(
            db,
            &meeting.id,
            &meeting_json,
            &accounts_dir,
            Some(embedding_ref),
        );

        // Auto-link Resolved outcomes (confidence ≥0.85)
        let mut linked = 0;
        for outcome in &outcomes {
            if let crate::prepare::entity_resolver::ResolutionOutcome::Resolved(entity)
                | crate::prepare::entity_resolver::ResolutionOutcome::ResolvedWithFlag(entity) = outcome
            {
                let _ = db.link_meeting_entity_if_absent(
                    &meeting.id,
                    &entity.entity_id,
                    entity.entity_type.as_str(),
                );
                linked += 1;
            }
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
             FROM meetings_history mh
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

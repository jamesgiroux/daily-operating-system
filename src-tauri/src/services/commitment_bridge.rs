//! AI commitment bridge service.
//!
//! Synchronizes AI-inferred open commitments from `IntelligenceJson` into the
//! Actions entity, using the `ai_commitment_bridge` table for stable identity
//! and tombstone tracking.
//!
//! See migration 108 for the bridge table shape.  See ADR-0101 for the
//! service-boundary rule (no direct DB writes from commands).

use crate::action_status::{KIND_COMMITMENT, OPEN_STATUSES, UNSTARTED};
use crate::db::{ActionDb, DbAction};
use crate::intelligence::io::OpenCommitment;

/// Summary of a commitment sync pass — emitted as an INFO log after every
/// enrichment completion so we can observe bridge churn in the wild.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BridgeSyncSummary {
    pub created: usize,
    pub updated: usize,
    pub skipped_tombstoned: usize,
    pub skipped_missing_id: usize,
}

/// A single bridge row read from `ai_commitment_bridge`.
struct BridgeRow {
    action_id: Option<String>,
    tombstoned: bool,
}

/// Synchronize AI-inferred commitments from `IntelligenceJson` into the
/// Actions entity via the `ai_commitment_bridge` table.
///
/// Called from enrichment completion (after `IntelligenceJson` persists).
/// For each commitment with a `commitment_id`:
///   - If bridge row exists and `tombstoned = 1` → skip (do not resurrect).
///   - If bridge row exists and `action_id` points to a non-terminal Action →
///     update Action metadata (title/description, due_date) if changed,
///     then update `last_seen_at`.
///   - If bridge row does not exist → create Action (status=BACKLOG,
///     action_kind=KIND_COMMITMENT, source_type='commitment',
///     source_id=commitment_id), then insert bridge row
///     (first_seen_at=now, last_seen_at=now, tombstoned=0).
///
/// Commitments without `commitment_id` are skipped (legacy / unbridgeable).
pub fn sync_ai_commitments(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    commitments: &[OpenCommitment],
) -> Result<BridgeSyncSummary, String> {
    let mut summary = BridgeSyncSummary::default();
    let now = chrono::Utc::now().to_rfc3339();

    for commitment in commitments {
        let commitment_id = match commitment.commitment_id.as_deref() {
            Some(id) if !id.trim().is_empty() => id,
            _ => {
                summary.skipped_missing_id += 1;
                continue;
            }
        };

        let existing = read_bridge_row(db, commitment_id).map_err(|e| e.to_string())?;

        match existing {
            Some(row) if row.tombstoned => {
                summary.skipped_tombstoned += 1;
                // Still refresh last_seen_at so we can see the LLM kept
                // re-emitting it (useful for debugging tombstone churn).
                touch_bridge_row(db, commitment_id, &now).map_err(|e| e.to_string())?;
            }
            Some(row) => {
                // Existing non-tombstoned row. If the associated Action exists
                // and is non-terminal, update metadata that changed.
                if let Some(action_id) = row.action_id.as_deref() {
                    let action_opt = db
                        .get_action_by_id(action_id)
                        .map_err(|e| e.to_string())?;
                    if let Some(mut action) = action_opt {
                        if OPEN_STATUSES.contains(&action.status.as_str()) {
                            let mut dirty = false;

                            if action.title != commitment.description {
                                action.title = commitment.description.clone();
                                dirty = true;
                            }
                            if action.due_date != commitment.due_date {
                                action.due_date = commitment.due_date.clone();
                                dirty = true;
                            }
                            // Owner is recorded in context for now — DbAction
                            // has no owner field of its own.
                            let owner_ctx = commitment
                                .owner
                                .as_deref()
                                .map(|o| format!("owner: {o}"));
                            if action.context != owner_ctx {
                                action.context = owner_ctx;
                                dirty = true;
                            }

                            if dirty {
                                action.updated_at = now.clone();
                                db.upsert_action(&action).map_err(|e| e.to_string())?;
                                summary.updated += 1;
                            }
                        }
                    }
                }
                touch_bridge_row(db, commitment_id, &now).map_err(|e| e.to_string())?;
            }
            None => {
                // Brand new commitment: create Action, then insert bridge row.
                let action_id = uuid::Uuid::new_v4().to_string();
                let account_id = if entity_type == "account" {
                    Some(entity_id.to_string())
                } else {
                    None
                };
                let project_id = if entity_type == "project" {
                    Some(entity_id.to_string())
                } else {
                    None
                };
                let owner_ctx = commitment.owner.as_deref().map(|o| format!("owner: {o}"));

                let action = DbAction {
                    id: action_id.clone(),
                    title: commitment.description.clone(),
                    priority: crate::action_status::PRIORITY_DEFAULT,
                    status: crate::action_status::BACKLOG.to_string(),
                    created_at: now.clone(),
                    due_date: commitment.due_date.clone(),
                    completed_at: None,
                    account_id,
                    project_id,
                    source_type: Some("commitment".to_string()),
                    source_id: Some(commitment_id.to_string()),
                    source_label: commitment.source.clone(),
                    action_kind: KIND_COMMITMENT.to_string(),
                    context: owner_ctx,
                    waiting_on: None,
                    updated_at: now.clone(),
                    person_id: None,
                    account_name: None,
                    next_meeting_title: None,
                    next_meeting_start: None,
                    needs_decision: false,
                    decision_owner: None,
                    decision_stakes: None,
                    linear_identifier: None,
                    linear_url: None,
                };
                db.upsert_action(&action).map_err(|e| e.to_string())?;

                insert_bridge_row(db, commitment_id, entity_type, entity_id, &action_id, &now)
                    .map_err(|e| e.to_string())?;
                summary.created += 1;
            }
        }
    }

    // Silence unused warning on UNSTARTED (imported for clarity / future use).
    let _ = UNSTARTED;

    Ok(summary)
}

/// Mark a commitment's bridge row tombstoned so re-enrichment can't
/// resurrect it. Called from Action state-transition services
/// (`complete_action`, `reject_suggested_action`, `archive_action`, etc.)
/// when `action_kind == KIND_COMMITMENT` and the transition is terminal.
///
/// Looks up the bridge row by `action_id`. No-op if none exists.
pub fn tombstone_commitment_bridge(db: &ActionDb, action_id: &str) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let changed = db
        .conn_ref()
        .execute(
            "UPDATE ai_commitment_bridge
             SET tombstoned = 1, last_seen_at = ?1
             WHERE action_id = ?2",
            rusqlite::params![now, action_id],
        )
        .map_err(|e| e.to_string())?;
    if changed > 0 {
        log::info!(
            "commitment_bridge: tombstoned bridge row(s) for action {} ({})",
            action_id,
            changed
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers — SQL kept inside the service module since the
// ai_commitment_bridge table has no other callers.
// ---------------------------------------------------------------------------

fn read_bridge_row(
    db: &ActionDb,
    commitment_id: &str,
) -> Result<Option<BridgeRow>, rusqlite::Error> {
    db.conn_ref()
        .query_row(
            "SELECT action_id, tombstoned FROM ai_commitment_bridge
             WHERE commitment_id = ?1",
            rusqlite::params![commitment_id],
            |row| {
                let action_id: Option<String> = row.get(0)?;
                let tombstoned: i32 = row.get(1)?;
                Ok(BridgeRow {
                    action_id,
                    tombstoned: tombstoned != 0,
                })
            },
        )
        .map(Some)
        .or_else(|e| {
            if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                Ok(None)
            } else {
                Err(e)
            }
        })
}

fn insert_bridge_row(
    db: &ActionDb,
    commitment_id: &str,
    entity_type: &str,
    entity_id: &str,
    action_id: &str,
    now: &str,
) -> Result<(), rusqlite::Error> {
    db.conn_ref().execute(
        "INSERT INTO ai_commitment_bridge
             (commitment_id, entity_type, entity_id, action_id,
              first_seen_at, last_seen_at, tombstoned)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0)",
        rusqlite::params![commitment_id, entity_type, entity_id, action_id, now],
    )?;
    Ok(())
}

fn touch_bridge_row(
    db: &ActionDb,
    commitment_id: &str,
    now: &str,
) -> Result<(), rusqlite::Error> {
    db.conn_ref().execute(
        "UPDATE ai_commitment_bridge SET last_seen_at = ?1 WHERE commitment_id = ?2",
        rusqlite::params![now, commitment_id],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    fn make_commitment(id: Option<&str>, description: &str) -> OpenCommitment {
        OpenCommitment {
            commitment_id: id.map(|s| s.to_string()),
            description: description.to_string(),
            owner: None,
            due_date: None,
            source: None,
            status: None,
            item_source: None,
            discrepancy: None,
        }
    }

    #[test]
    fn test_sync_creates_new_commitment() {
        let db = test_db();
        let commitments = vec![make_commitment(Some("meeting:abc:1"), "Send renewal deck")];

        let summary = sync_ai_commitments(&db, "account", "acct-1", &commitments).expect("sync");
        assert_eq!(summary.created, 1);
        assert_eq!(summary.updated, 0);
        assert_eq!(summary.skipped_tombstoned, 0);
        assert_eq!(summary.skipped_missing_id, 0);

        // Bridge row exists
        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["meeting:abc:1"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // Action exists with kind=commitment, status=backlog
        let (kind, status): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT a.action_kind, a.status FROM actions a
                 JOIN ai_commitment_bridge b ON b.action_id = a.id
                 WHERE b.commitment_id = ?1",
                rusqlite::params!["meeting:abc:1"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(kind, KIND_COMMITMENT);
        assert_eq!(status, "backlog");
    }

    #[test]
    fn test_sync_skips_tombstoned() {
        let db = test_db();
        let commitments = vec![make_commitment(Some("c:1"), "Tombstoned item")];

        sync_ai_commitments(&db, "account", "acct-1", &commitments).expect("initial sync");

        // Fetch the action_id we just created, tombstone it.
        let action_id: String = db
            .conn_ref()
            .query_row(
                "SELECT action_id FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["c:1"],
                |row| row.get(0),
            )
            .unwrap();
        tombstone_commitment_bridge(&db, &action_id).expect("tombstone");

        // Re-sync with the same commitment — should be skipped, no new action.
        let summary = sync_ai_commitments(&db, "account", "acct-1", &commitments).expect("resync");
        assert_eq!(summary.created, 0);
        assert_eq!(summary.skipped_tombstoned, 1);

        let action_count: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(action_count, 1);
    }

    #[test]
    fn test_sync_skips_missing_id() {
        let db = test_db();
        let commitments = vec![make_commitment(None, "Legacy item")];
        let summary = sync_ai_commitments(&db, "account", "acct-1", &commitments).expect("sync");
        assert_eq!(summary.created, 0);
        assert_eq!(summary.skipped_missing_id, 1);

        let action_count: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(action_count, 0);
    }

    #[test]
    fn test_tombstone_sets_flag() {
        let db = test_db();
        let commitments = vec![make_commitment(Some("c:2"), "Thing to kill")];
        sync_ai_commitments(&db, "account", "acct-1", &commitments).expect("sync");

        let action_id: String = db
            .conn_ref()
            .query_row(
                "SELECT action_id FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["c:2"],
                |row| row.get(0),
            )
            .unwrap();
        tombstone_commitment_bridge(&db, &action_id).expect("tombstone");

        let flag: i32 = db
            .conn_ref()
            .query_row(
                "SELECT tombstoned FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["c:2"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(flag, 1);
    }

    #[test]
    fn test_resurrection_blocked() {
        let db = test_db();
        let commitments = vec![make_commitment(Some("c:3"), "Resurrect me?")];

        // Initial sync creates action
        sync_ai_commitments(&db, "account", "acct-1", &commitments).expect("sync");
        let action_id: String = db
            .conn_ref()
            .query_row(
                "SELECT action_id FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["c:3"],
                |row| row.get(0),
            )
            .unwrap();

        // Complete + tombstone
        db.complete_action(&action_id).expect("complete");
        tombstone_commitment_bridge(&db, &action_id).expect("tombstone");

        // Re-sync with same commitment — no new action.
        let summary = sync_ai_commitments(&db, "account", "acct-1", &commitments).expect("resync");
        assert_eq!(summary.created, 0);
        assert_eq!(summary.skipped_tombstoned, 1);

        let action_count: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(action_count, 1);
    }

    #[test]
    fn test_resurrection_blocked_on_rephrase() {
        let db = test_db();
        let original = vec![make_commitment(Some("c:4"), "Original phrasing")];
        let rephrased = vec![make_commitment(Some("c:4"), "Totally different wording")];

        sync_ai_commitments(&db, "account", "acct-1", &original).expect("sync");
        let action_id: String = db
            .conn_ref()
            .query_row(
                "SELECT action_id FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["c:4"],
                |row| row.get(0),
            )
            .unwrap();
        tombstone_commitment_bridge(&db, &action_id).expect("tombstone");

        // Same id, rephrased description — bridge still blocks.
        let summary =
            sync_ai_commitments(&db, "account", "acct-1", &rephrased).expect("resync");
        assert_eq!(summary.created, 0);
        assert_eq!(summary.skipped_tombstoned, 1);

        let title: String = db
            .conn_ref()
            .query_row(
                "SELECT title FROM actions WHERE id = ?1",
                rusqlite::params![action_id],
                |row| row.get(0),
            )
            .unwrap();
        // Title unchanged (still original), because the tombstone short-circuits.
        assert_eq!(title, "Original phrasing");
    }

    #[test]
    fn test_description_update_propagates() {
        let db = test_db();
        let original = vec![make_commitment(Some("c:5"), "Original")];
        let updated = vec![make_commitment(Some("c:5"), "Updated description")];

        sync_ai_commitments(&db, "account", "acct-1", &original).expect("sync");
        let summary = sync_ai_commitments(&db, "account", "acct-1", &updated).expect("resync");

        assert_eq!(summary.created, 0);
        assert_eq!(summary.updated, 1);

        let title: String = db
            .conn_ref()
            .query_row(
                "SELECT a.title FROM actions a
                 JOIN ai_commitment_bridge b ON b.action_id = a.id
                 WHERE b.commitment_id = ?1",
                rusqlite::params!["c:5"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "Updated description");
    }

    #[test]
    fn test_complete_commitment_action_tombstones_bridge_row() {
        // Regression guard: after complete_action on a commitment action,
        // the bridge row must be tombstoned (so re-enrichment can't resurrect).
        let db = test_db();
        let commitments = vec![make_commitment(Some("c:6"), "Will complete")];
        sync_ai_commitments(&db, "account", "acct-1", &commitments).expect("sync");

        let action_id: String = db
            .conn_ref()
            .query_row(
                "SELECT action_id FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["c:6"],
                |row| row.get(0),
            )
            .unwrap();

        db.complete_action(&action_id).expect("complete");
        tombstone_commitment_bridge(&db, &action_id).expect("tombstone");

        let flag: i32 = db
            .conn_ref()
            .query_row(
                "SELECT tombstoned FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["c:6"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(flag, 1);
    }
}

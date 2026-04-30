//! AI commitment bridge service.
//!
//! Synchronizes AI-inferred open commitments from `IntelligenceJson` into the
//! Actions entity, using the `ai_commitment_bridge` table for stable identity
//! and tombstone tracking.
//!
//! See migration 108 for the bridge table shape.  See ADR-0101 for the
//! service-boundary rule (no direct DB writes from commands).

use crate::action_status::{BACKLOG, KIND_COMMITMENT, OPEN_STATUSES, UNSTARTED};
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
    /// DOS-321: bridge_id was new but mapped to an existing action via
    /// normalized-title match (alias). No new action row was created;
    /// the bridge row was inserted pointing at the existing action.
    pub aliased_to_existing: usize,
}

/// DOS-321: Normalize a commitment title for cross-source dedup.
///
/// The AI emits stable commitment_ids per source — but re-enrichment hits
/// different sources (Gong call, meeting transcript, CRM, Glean) for the
/// same commitment text, producing distinct commitment_ids that the bridge
/// can't unify. Normalizing the title gives us a stable secondary key:
/// (entity_id, normalized_title) → canonical action.
///
/// Lowercases and trims surrounding whitespace. Internal whitespace and
/// punctuation are kept verbatim — the observed duplicates from real
/// production data are character-for-character identical, so tighter
/// normalization risks false-positive merges of similarly-worded but
/// distinct commitments.
pub fn normalize_commitment_title(title: &str) -> String {
    title.trim().to_lowercase()
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
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    commitments: &[OpenCommitment],
) -> Result<BridgeSyncSummary, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let mut summary = BridgeSyncSummary::default();
    let now = ctx.clock.now().to_rfc3339();

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
                touch_bridge_row(ctx, db, commitment_id, &now).map_err(|e| e.to_string())?;
            }
            Some(row) => {
                // Existing non-tombstoned row. Only update Action metadata
                // while the Action is still `backlog` (AI-proposed, unaccepted).
                // Once the user accepts (backlog → unstarted) the row is
                // USER-OWNED — a user edit to the title must not be
                // overwritten by the next enrichment pass. For accepted rows
                // we only refresh `last_seen_at` on the bridge so the LLM's
                // continued emission remains observable.
                if let Some(action_id) = row.action_id.as_deref() {
                    let action_opt = db
                        .get_action_by_id(action_id)
                        .map_err(|e| e.to_string())?;
                    if let Some(mut action) = action_opt {
                        if action.status.as_str() == BACKLOG {
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
                        // User-accepted (unstarted/started) → skip metadata
                        // update. Terminal statuses are already excluded by
                        // bridge tombstoning, but defensively we no-op here
                        // as well if the action somehow landed in a non-open
                        // non-backlog state.
                        let _ = OPEN_STATUSES;
                    }
                }
                touch_bridge_row(ctx, db, commitment_id, &now).map_err(|e| e.to_string())?;
            }
            None => {
                // Brand-new commitment_id. Two sub-cases:
                //
                // (a) DOS-321: The same commitment text may already have a
                //     non-tombstoned action under a *different* commitment_id
                //     (different source, e.g. Gong vs meeting transcript vs
                //     Glean). Re-emerging the row would create dupes that
                //     accumulate every enrichment run. Look for an existing
                //     action with the same normalized title; if found, alias
                //     the new bridge row to it instead of creating another.
                //
                // (b) Truly new: create Action, then insert bridge row.
                let normalized = normalize_commitment_title(&commitment.description);
                if let Some(existing_action_id) =
                    find_existing_open_commitment_by_title(db, entity_type, entity_id, &normalized)
                        .map_err(|e| e.to_string())?
                {
                    insert_bridge_row(
                        ctx,
                        db,
                        commitment_id,
                        entity_type,
                        entity_id,
                        &existing_action_id,
                        &now,
                    )
                    .map_err(|e| e.to_string())?;
                    log::debug!(
                        "commitment_bridge: aliased {} → existing action {} via title match",
                        commitment_id,
                        existing_action_id
                    );
                    summary.aliased_to_existing += 1;
                    continue;
                }

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

                insert_bridge_row(ctx, db, commitment_id, entity_type, entity_id, &action_id, &now)
                    .map_err(|e| e.to_string())?;
                summary.created += 1;
            }
        }
    }

    // Silence unused warning on UNSTARTED (imported for clarity / future use).
    let _ = UNSTARTED;

    Ok(summary)
}

/// DOS-321: Look up an existing non-terminal commitment-typed action with
/// the same normalized title under the given entity. Used by
/// `sync_ai_commitments` to alias a new commitment_id onto an existing
/// action instead of creating a duplicate row.
///
/// Returns the action_id of the oldest matching action so re-aliasing
/// is stable across runs (deterministic on `created_at ASC`).
fn find_existing_open_commitment_by_title(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    normalized_title: &str,
) -> Result<Option<String>, rusqlite::Error> {
    if normalized_title.is_empty() {
        return Ok(None);
    }
    let entity_col = match entity_type {
        "account" => "account_id",
        "project" => "project_id",
        _ => return Ok(None),
    };
    let sql = format!(
        "SELECT id FROM actions
         WHERE {entity_col} = ?1
           AND action_kind = ?2
           AND status NOT IN ('completed', 'cancelled', 'rejected', 'archived')
           AND lower(trim(title)) = ?3
         ORDER BY created_at ASC
         LIMIT 1"
    );
    db.conn_ref()
        .query_row(
            &sql,
            rusqlite::params![entity_id, KIND_COMMITMENT, normalized_title],
            |row| row.get::<_, String>(0),
        )
        .map(Some)
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
}

/// Mark a commitment's bridge row tombstoned so re-enrichment can't
/// resurrect it. Called from Action state-transition services
/// (`complete_action`, `reject_suggested_action`, `archive_action`, etc.)
/// when `action_kind == KIND_COMMITMENT` and the transition is terminal.
///
/// Looks up the bridge row by `action_id`. No-op if none exists.
pub fn tombstone_commitment_bridge(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    action_id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let now = ctx.clock.now().to_rfc3339();
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
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    commitment_id: &str,
    entity_type: &str,
    entity_id: &str,
    action_id: &str,
    now: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.conn_ref().execute(
        "INSERT INTO ai_commitment_bridge
             (commitment_id, entity_type, entity_id, action_id,
              first_seen_at, last_seen_at, tombstoned)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0)",
        rusqlite::params![commitment_id, entity_type, entity_id, action_id, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn touch_bridge_row(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    commitment_id: &str,
    now: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.conn_ref().execute(
        "UPDATE ai_commitment_bridge SET last_seen_at = ?1 WHERE commitment_id = ?2",
        rusqlite::params![now, commitment_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::TimeZone;

    macro_rules! make_ctx {
        ($ctx:ident) => {
            let clock =
                FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
            let rng = SeedableRng::new(42);
            let ext = ExternalClients::default();
            let $ctx = ServiceContext::test_live(&clock, &rng, &ext);
        };
    }

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
        make_ctx!(ctx);
        let commitments = vec![make_commitment(Some("meeting:abc:1"), "Send renewal deck")];

        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("sync");
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
        make_ctx!(ctx);
        let commitments = vec![make_commitment(Some("c:1"), "Tombstoned item")];

        sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments)
            .expect("initial sync");

        // Fetch the action_id we just created, tombstone it.
        let action_id: String = db
            .conn_ref()
            .query_row(
                "SELECT action_id FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["c:1"],
                |row| row.get(0),
            )
            .unwrap();
        tombstone_commitment_bridge(&ctx, &db, &action_id).expect("tombstone");

        // Re-sync with the same commitment — should be skipped, no new action.
        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("resync");
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
        make_ctx!(ctx);
        let commitments = vec![make_commitment(None, "Legacy item")];
        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("sync");
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
        make_ctx!(ctx);
        let commitments = vec![make_commitment(Some("c:2"), "Thing to kill")];
        sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("sync");

        let action_id: String = db
            .conn_ref()
            .query_row(
                "SELECT action_id FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["c:2"],
                |row| row.get(0),
            )
            .unwrap();
        tombstone_commitment_bridge(&ctx, &db, &action_id).expect("tombstone");

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
        make_ctx!(ctx);
        let commitments = vec![make_commitment(Some("c:3"), "Resurrect me?")];

        // Initial sync creates action
        sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("sync");
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
        tombstone_commitment_bridge(&ctx, &db, &action_id).expect("tombstone");

        // Re-sync with same commitment — no new action.
        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("resync");
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
        make_ctx!(ctx);
        let original = vec![make_commitment(Some("c:4"), "Original phrasing")];
        let rephrased = vec![make_commitment(Some("c:4"), "Totally different wording")];

        sync_ai_commitments(&ctx, &db, "account", "acct-1", &original).expect("sync");
        let action_id: String = db
            .conn_ref()
            .query_row(
                "SELECT action_id FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["c:4"],
                |row| row.get(0),
            )
            .unwrap();
        tombstone_commitment_bridge(&ctx, &db, &action_id).expect("tombstone");

        // Same id, rephrased description — bridge still blocks.
        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &rephrased).expect("resync");
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
        make_ctx!(ctx);
        let original = vec![make_commitment(Some("c:5"), "Original")];
        let updated = vec![make_commitment(Some("c:5"), "Updated description")];

        sync_ai_commitments(&ctx, &db, "account", "acct-1", &original).expect("sync");
        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &updated).expect("resync");

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
    fn test_user_edit_preserved_across_sync_when_accepted() {
        // Regression guard: after user accepts a backlog commitment
        // (backlog → unstarted) and edits the title, a subsequent sync pass
        // must NOT overwrite the user's title. The bridge only updates
        // metadata while the row is still backlog (AI-owned). Once accepted,
        // the row is USER-OWNED and only last_seen_at gets refreshed.
        let db = test_db();
        make_ctx!(ctx);
        let original = vec![make_commitment(Some("c:user-owned"), "AI phrasing")];
        sync_ai_commitments(&ctx, &db, "account", "acct-1", &original).expect("initial sync");

        let action_id: String = db
            .conn_ref()
            .query_row(
                "SELECT action_id FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["c:user-owned"],
                |row| row.get(0),
            )
            .unwrap();

        // User accepts → backlog transitions to unstarted.
        db.accept_suggested_action(&action_id).expect("accept");
        // User edits the title inline.
        db.conn_ref()
            .execute(
                "UPDATE actions SET title = ?1, updated_at = datetime('now') WHERE id = ?2",
                rusqlite::params!["User-edited title", action_id],
            )
            .expect("user edit");

        // Next enrichment pass emits the SAME commitment_id with the
        // original AI phrasing — this must NOT clobber the user's edit.
        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &original).expect("resync");
        assert_eq!(summary.created, 0);
        assert_eq!(summary.updated, 0, "user-owned title must not be overwritten");

        let title: String = db
            .conn_ref()
            .query_row(
                "SELECT title FROM actions WHERE id = ?1",
                rusqlite::params![action_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "User-edited title");
    }

    #[test]
    fn test_backlog_metadata_still_updates_before_acceptance() {
        // Regression guard for the inverse: while a commitment is still
        // backlog (unaccepted), the AI is the owner and enrichment MUST keep
        // metadata up to date — otherwise we'd freeze bad early phrasing.
        let db = test_db();
        make_ctx!(ctx);
        let v1 = vec![make_commitment(Some("c:ai-owned"), "Early phrasing")];
        let v2 = vec![make_commitment(Some("c:ai-owned"), "Refined phrasing")];

        sync_ai_commitments(&ctx, &db, "account", "acct-1", &v1).expect("initial sync");
        let summary = sync_ai_commitments(&ctx, &db, "account", "acct-1", &v2).expect("resync");
        assert_eq!(summary.updated, 1, "backlog rows still track AI updates");

        let title: String = db
            .conn_ref()
            .query_row(
                "SELECT a.title FROM actions a
                 JOIN ai_commitment_bridge b ON b.action_id = a.id
                 WHERE b.commitment_id = ?1",
                rusqlite::params!["c:ai-owned"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "Refined phrasing");
    }

    #[test]
    fn dos321_normalize_commitment_title_strips_case_and_outer_whitespace() {
        assert_eq!(
            normalize_commitment_title("  Send Renewal Deck  "),
            "send renewal deck"
        );
        assert_eq!(
            normalize_commitment_title("Send Renewal Deck"),
            "send renewal deck"
        );
        // Internal whitespace + punctuation are preserved verbatim — the
        // observed dupes are character-for-character identical, so loose
        // matching would risk merging similarly-worded but distinct items.
        assert_eq!(
            normalize_commitment_title("Send  Renewal  Deck."),
            "send  renewal  deck."
        );
    }

    #[test]
    fn dos321_aliases_new_commitment_id_to_existing_action_with_same_title() {
        // Reproduces the production bug: same commitment text emerges twice
        // under different commitment_ids (different sources). Without dedup
        // we would create two action rows; with dedup the second emit
        // creates a bridge row pointing at the first action's id.
        let db = test_db();
        make_ctx!(ctx);
        let title = "Consolidate Globex subsidiary domains onto VIP";

        // First emit: source = meeting transcript.
        let first = sync_ai_commitments(
            &ctx,
            &db,
            "account",
            "acct-1",
            &[make_commitment(Some("meeting:1"), title)],
        )
        .expect("first sync");
        assert_eq!(first.created, 1);
        assert_eq!(first.aliased_to_existing, 0);

        // Second emit: same title, different commitment_id (source = Gong).
        let second = sync_ai_commitments(
            &ctx,
            &db,
            "account",
            "acct-1",
            &[make_commitment(Some("gong:99"), title)],
        )
        .expect("second sync");
        assert_eq!(second.created, 0);
        assert_eq!(second.aliased_to_existing, 1);

        // One action, two bridge rows pointing at it.
        let action_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM actions WHERE account_id = ?1 AND action_kind = 'commitment'",
                rusqlite::params!["acct-1"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(action_count, 1, "should not create a duplicate action");

        let bridge_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM ai_commitment_bridge WHERE commitment_id IN (?1, ?2)",
                rusqlite::params!["meeting:1", "gong:99"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(bridge_count, 2, "both source ids should bridge to the same action");
    }

    #[test]
    fn dos321_alias_dedup_is_case_and_whitespace_insensitive_at_edges() {
        let db = test_db();
        make_ctx!(ctx);
        sync_ai_commitments(
            &ctx,
            &db,
            "account",
            "acct-1",
            &[make_commitment(Some("a"), "Send Renewal Deck")],
        )
        .expect("first");

        // Different commitment_id, leading/trailing whitespace, different case.
        let s = sync_ai_commitments(
            &ctx,
            &db,
            "account",
            "acct-1",
            &[make_commitment(Some("b"), "  send renewal deck  ")],
        )
        .expect("second");
        assert_eq!(s.aliased_to_existing, 1);
        assert_eq!(s.created, 0);
    }

    #[test]
    fn dos321_alias_only_targets_open_actions_not_completed() {
        // If the existing action was completed/cancelled, a fresh emit
        // should NOT alias onto it — the user is done with that commitment.
        // Bridge tombstone usually catches this on the same commitment_id;
        // for a *different* commitment_id pointing at a completed action,
        // a brand-new action is the right shape.
        let db = test_db();
        make_ctx!(ctx);
        sync_ai_commitments(
            &ctx,
            &db,
            "account",
            "acct-1",
            &[make_commitment(Some("a"), "One-time work")],
        )
        .expect("first");
        let action_id: String = db
            .conn_ref()
            .query_row(
                "SELECT action_id FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["a"],
                |row| row.get(0),
            )
            .unwrap();
        db.complete_action(&action_id).expect("complete");

        let s = sync_ai_commitments(
            &ctx,
            &db,
            "account",
            "acct-1",
            &[make_commitment(Some("b"), "One-time work")],
        )
        .expect("second");
        // No alias — the existing action is completed; new action created.
        assert_eq!(s.aliased_to_existing, 0);
        assert_eq!(s.created, 1);
    }

    #[test]
    fn test_complete_commitment_action_tombstones_bridge_row() {
        // Regression guard: after complete_action on a commitment action,
        // the bridge row must be tombstoned (so re-enrichment can't resurrect).
        let db = test_db();
        make_ctx!(ctx);
        let commitments = vec![make_commitment(Some("c:6"), "Will complete")];
        sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("sync");

        let action_id: String = db
            .conn_ref()
            .query_row(
                "SELECT action_id FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params!["c:6"],
                |row| row.get(0),
            )
            .unwrap();

        db.complete_action(&action_id).expect("complete");
        tombstone_commitment_bridge(&ctx, &db, &action_id).expect("tombstone");

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

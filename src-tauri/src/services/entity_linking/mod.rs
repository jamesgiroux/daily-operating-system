pub mod calendar_adapter;
pub mod cascade;
pub mod email_adapter;
pub mod evidence;
pub mod phases;
pub mod primitives;
pub mod repository;
pub mod rules;
pub mod types;

pub use types::{
    Candidate, EntityRef, LinkOutcome, LinkRole, LinkTier, LinkingContext, OwnerRef, OwnerType,
    Participant, ParticipantRole, RuleOutcome, Trigger,
};

use std::sync::Arc;

use crate::state::AppState;

// ---------------------------------------------------------------------------
// evaluate — four-phase engine entry point
//
// Uses the shared DbService writer connection (serialized via tokio-rusqlite).
// One write connection slot is held for the duration of the closure — no new
// connections opened per event.
// ---------------------------------------------------------------------------

pub async fn evaluate(
    state: Arc<AppState>,
    mut ctx: LinkingContext,
    trigger: Trigger,
) -> Result<LinkOutcome, String> {
    state
        .db_write(move |db| {
            let user_domains = ctx.user_domains.clone();
            // Phase 1 suppress check — run BEFORE Phase 2 so declined/self
            // meetings don't get person stubs created unnecessarily.
            if let Some(suppressed_outcome) = phases::phase1_suppress(&ctx, db, trigger)? {
                return Ok(suppressed_outcome);
            }
            // Phase 2: person stub creation (writes, but independent of the
            // phase-3 transaction — find_or_create_person has its own txn).
            phases::phase2_record_facts(&mut ctx, db, &user_domains);
            // Phases 3 + 4 inside a single write transaction.
            phases::run_phases(&ctx, db)
        })
        .await
}

// ---------------------------------------------------------------------------
// Manual overrides — all writes go through the shared writer connection
// ---------------------------------------------------------------------------

pub async fn manual_set_primary(
    state: Arc<AppState>,
    owner_type: OwnerType,
    owner_id: String,
    entity: Option<EntityRef>,
) -> Result<LinkOutcome, String> {
    state
        .db_write(move |db| {
            db.with_transaction(|_| {
                // Delete ALL prior primary rows for this owner (user and auto)
                // so idx_one_primary never blocks the new user-override insert.
                db.conn_ref()
                    .execute(
                        "DELETE FROM linked_entities_raw \
                         WHERE owner_type = ?1 AND owner_id = ?2 AND role = 'primary'",
                        rusqlite::params![owner_type.as_str(), owner_id],
                    )
                    .map_err(|e| format!("manual_set_primary clear: {e}"))?;

                if let Some(ref ent) = entity {
                    let now = chrono::Utc::now().to_rfc3339();
                    let version = db.get_entity_graph_version().unwrap_or(0);
                    db.conn_ref()
                        .execute(
                            "INSERT INTO linked_entities_raw \
                             (owner_type, owner_id, entity_id, entity_type, role, source, \
                              rule_id, confidence, graph_version, created_at) \
                             VALUES (?1, ?2, ?3, ?4, 'primary', 'user', 'P1', 1.0, ?5, ?6)",
                            rusqlite::params![
                                owner_type.as_str(),
                                owner_id,
                                ent.entity_id,
                                ent.entity_type,
                                version,
                                now,
                            ],
                        )
                        .map_err(|e| format!("manual_set_primary insert: {e}"))?;
                }
                Ok(())
            })?;

            let graph_version = db.get_entity_graph_version().unwrap_or(0);
            let ctx = LinkingContext {
                owner: OwnerRef { owner_type, owner_id },
                participants: vec![],
                title: None,
                attendee_count: 0,
                thread_id: None,
                series_id: None,
                graph_version,
                user_domains: vec![],
            };
            phases::run_phases(&ctx, db)
        })
        .await
}

pub async fn manual_dismiss(
    state: Arc<AppState>,
    owner_type: OwnerType,
    owner_id: String,
    entity: EntityRef,
) -> Result<LinkOutcome, String> {
    state
        .db_write(move |db| {
            db.with_transaction(|_| {
                // Write dismissal row first.
                db.upsert_linking_dismissal(
                    owner_type.as_str(),
                    &owner_id,
                    &entity.entity_id,
                    &entity.entity_type,
                    None,
                )?;
                // Mark the raw row user_dismissed so the view hides it
                // and delete_auto_links_for_owner preserves it on recompute.
                db.set_link_user_dismissed(
                    owner_type.as_str(),
                    &owner_id,
                    &entity.entity_id,
                    &entity.entity_type,
                )
            })?;

            let graph_version = db.get_entity_graph_version().unwrap_or(0);
            let ctx = LinkingContext {
                owner: OwnerRef { owner_type, owner_id },
                participants: vec![],
                title: None,
                attendee_count: 0,
                thread_id: None,
                series_id: None,
                graph_version,
                user_domains: vec![],
            };
            phases::run_phases(&ctx, db)
        })
        .await
}

pub async fn manual_undismiss(
    state: Arc<AppState>,
    owner_type: OwnerType,
    owner_id: String,
    entity: EntityRef,
) -> Result<LinkOutcome, String> {
    state
        .db_write(move |db| {
            db.delete_linking_dismissal(
                owner_type.as_str(),
                &owner_id,
                &entity.entity_id,
                &entity.entity_type,
            )?;
            // Restore the raw row to the rule-derived source so it becomes
            // visible in the linked_entities view again.
            db.conn_ref()
                .execute(
                    "UPDATE linked_entities_raw \
                     SET source = 'rule:restored' \
                     WHERE owner_type = ?1 AND owner_id = ?2 \
                       AND entity_id = ?3 AND entity_type = ?4 \
                       AND source = 'user_dismissed'",
                    rusqlite::params![
                        owner_type.as_str(),
                        owner_id,
                        entity.entity_id,
                        entity.entity_type,
                    ],
                )
                .map_err(|e| format!("manual_undismiss restore: {e}"))?;

            let graph_version = db.get_entity_graph_version().unwrap_or(0);
            let ctx = LinkingContext {
                owner: OwnerRef { owner_type, owner_id },
                participants: vec![],
                title: None,
                attendee_count: 0,
                thread_id: None,
                series_id: None,
                graph_version,
                user_domains: vec![],
            };
            phases::run_phases(&ctx, db)
        })
        .await
}

// ---------------------------------------------------------------------------
// Stakeholder queue — write path through shared writer
// ---------------------------------------------------------------------------

pub async fn confirm_stakeholder_suggestion(
    state: Arc<AppState>,
    account_id: String,
    person_id: String,
) -> Result<(), String> {
    state
        .db_write(move |db| db.confirm_stakeholder(&account_id, &person_id))
        .await
}

pub async fn dismiss_stakeholder_suggestion(
    state: Arc<AppState>,
    account_id: String,
    person_id: String,
) -> Result<(), String> {
    state
        .db_write(move |db| db.dismiss_stakeholder_suggestion(&account_id, &person_id))
        .await
}

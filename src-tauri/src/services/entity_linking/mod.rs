pub mod calendar_adapter;
pub mod cascade;
pub mod email_adapter;
pub mod evidence;
pub mod phases;
pub mod primitives;
pub mod repository;
pub mod rescan;
pub mod rules;
pub mod stakeholder_domains;
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
            // Phase 1: suppress check. Run BEFORE Phase 2 so S1 (self-meeting)
            // doesn't generate person stubs. S2 (broadcast) still writes facts.
            match phases::phase1_suppress(&ctx, db, trigger)? {
                phases::Phase1Result::Declined(outcome) => {
                    // S1: no facts, no primary.
                    return Ok(outcome);
                }
                phases::Phase1Result::Broadcast(outcome) => {
                    // S2: facts written per spec (AC#6), but no primary.
                    phases::phase2_record_facts(&mut ctx, db, &user_domains);
                    return Ok(outcome);
                }
                phases::Phase1Result::Continue => {}
            }
            // Phase 2: person stub creation (independent txn per stub).
            phases::phase2_record_facts(&mut ctx, db, &user_domains);
            // Phases 3 + 4 inside a single write transaction.
            phases::run_phases(&ctx, db)
        })
        .await
}

// ---------------------------------------------------------------------------
// Manual overrides — all writes go through the shared writer connection
// ---------------------------------------------------------------------------

/// DOS-258: Build a realistic `LinkingContext` for a manual override.
///
/// Previously all three `manual_*` functions passed `participants: vec![]`
/// into `run_phases`, which crippled any cascade rule that needs attendee
/// domains — in particular the downstream stakeholder-domain backfill and
/// the P4 evidence rules. Here we load the meeting's attendees or the
/// email's envelope so the same context that calendar_adapter /
/// email_adapter would build for a live event is available on user
/// relinks.
///
/// Returns a best-effort context — on DB errors we fall back to an empty
/// participant list so the manual override itself still succeeds.
fn build_manual_context(
    db: &crate::db::ActionDb,
    owner_type: OwnerType,
    owner_id: String,
) -> LinkingContext {
    let graph_version = db.get_entity_graph_version().unwrap_or(0);
    let user_domains = crate::state::load_config()
        .map(|c| c.resolved_user_domains())
        .unwrap_or_default();

    match owner_type {
        OwnerType::Meeting => {
            let (participants, title) = match db.get_meeting_by_id(&owner_id) {
                Ok(Some(meeting)) => {
                    let parts = meeting
                        .attendees
                        .as_deref()
                        .map(parse_meeting_attendees)
                        .unwrap_or_default();
                    (parts, Some(meeting.title))
                }
                _ => (Vec::new(), None),
            };
            let attendee_count = participants.len();
            LinkingContext {
                owner: OwnerRef { owner_type, owner_id },
                participants,
                title,
                attendee_count,
                thread_id: None,
                series_id: None,
                graph_version,
                user_domains,
            }
        }
        OwnerType::Email | OwnerType::EmailThread => {
            match db.get_email_by_id_for_linking(&owner_id) {
                Ok(Some(email)) => {
                    match email_adapter::build_context(&email, None, db) {
                        Ok(mut ctx) => {
                            // build_context sets OwnerType::Email — preserve the
                            // caller's owner_type (may be EmailThread).
                            ctx.owner.owner_type = owner_type;
                            ctx.owner.owner_id = owner_id;
                            ctx
                        }
                        Err(_) => LinkingContext {
                            owner: OwnerRef { owner_type, owner_id },
                            participants: vec![],
                            title: None,
                            attendee_count: 0,
                            thread_id: None,
                            series_id: None,
                            graph_version,
                            user_domains,
                        },
                    }
                }
                _ => LinkingContext {
                    owner: OwnerRef { owner_type, owner_id },
                    participants: vec![],
                    title: None,
                    attendee_count: 0,
                    thread_id: None,
                    series_id: None,
                    graph_version,
                    user_domains,
                },
            }
        }
    }
}

/// Parse the `meetings.attendees` column (JSON array or comma-separated) into
/// Participant rows. Mirrors calendar_adapter::build_context but reads from
/// the persisted meeting row rather than a live CalendarEvent.
fn parse_meeting_attendees(attendees_str: &str) -> Vec<Participant> {
    let emails: Vec<String> = serde_json::from_str::<Vec<String>>(attendees_str)
        .unwrap_or_else(|_| {
            attendees_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        });
    emails
        .into_iter()
        .filter_map(|email| {
            let email_lower = email.to_lowercase();
            if !email_lower.contains('@') {
                return None;
            }
            let domain = primitives::domain_from_email(&email_lower);
            Some(Participant {
                email: email_lower,
                name: None,
                role: ParticipantRole::Attendee,
                person_id: None,
                domain,
            })
        })
        .collect()
}

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

            // DOS-258: load the owner's actual attendees/envelope into ctx so
            // downstream cascade rules (including the stakeholder-domain
            // backfill in cascade::run_cascade) see real participants.
            let ctx = build_manual_context(db, owner_type, owner_id);
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

            let ctx = build_manual_context(db, owner_type, owner_id);
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

            let ctx = build_manual_context(db, owner_type, owner_id);
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
    // DOS-258: after confirming, sweep the whole account's stakeholder graph
    // for domains that aren't yet registered. Catches sibling stakeholders
    // whose domain wasn't registered by an earlier confirmation that
    // predated this code. Supersedes the per-person backfill — the
    // account-wide scan includes the just-confirmed person.
    let user_domains: Vec<String> = state
        .config
        .read()
        .as_ref()
        .map(|c| c.resolved_user_domains())
        .unwrap_or_default();
    state
        .db_write(move |db| {
            db.confirm_stakeholder(&account_id, &person_id)?;
            if let Err(e) = stakeholder_domains::backfill_domains_for_account(
                db,
                &account_id,
                &user_domains,
            ) {
                log::warn!(
                    "stakeholder_domains: confirm backfill for {account_id} failed: {e}"
                );
            }
            Ok(())
        })
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::types::DbMeeting;
    use chrono::Utc;

    /// DOS-258: regression — manual_set_primary used to call run_phases with
    /// `participants: vec![]`, which meant cascade C6 and the downstream
    /// stakeholder-domain backfill never saw real attendees on user relinks.
    /// This test asserts build_manual_context loads them from the DB.
    #[test]
    fn manual_set_primary_loads_meeting_attendees_into_cascade() {
        let db = crate::db::test_utils::test_db();
        let now = Utc::now().to_rfc3339();
        let meeting = DbMeeting {
            id: "mtg-dos258".to_string(),
            title: "Customer QBR".to_string(),
            meeting_type: "customer".to_string(),
            start_time: now.clone(),
            end_time: None,
            attendees: Some(
                r#"["alice@customer.com","bob@customer.com","carol@subsidiary.com"]"#
                    .to_string(),
            ),
            notes_path: None,
            summary: None,
            created_at: now,
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&meeting).expect("upsert meeting");

        let ctx = build_manual_context(
            &db,
            OwnerType::Meeting,
            "mtg-dos258".to_string(),
        );

        assert_eq!(ctx.participants.len(), 3, "should load all 3 attendees");
        assert_eq!(ctx.attendee_count, 3);
        assert_eq!(ctx.title.as_deref(), Some("Customer QBR"));
        let emails: Vec<_> = ctx.participants.iter().map(|p| p.email.as_str()).collect();
        assert!(emails.contains(&"alice@customer.com"));
        assert!(emails.contains(&"bob@customer.com"));
        assert!(emails.contains(&"carol@subsidiary.com"));
        // Each participant has a resolved domain so P4b/P4c/P4d can fire.
        assert!(ctx.participants.iter().all(|p| p.domain.is_some()));
    }

    #[test]
    fn manual_context_handles_unknown_owner_gracefully() {
        let db = crate::db::test_utils::test_db();
        let ctx = build_manual_context(
            &db,
            OwnerType::Meeting,
            "does-not-exist".to_string(),
        );
        assert!(ctx.participants.is_empty());
        assert_eq!(ctx.attendee_count, 0);
    }
}

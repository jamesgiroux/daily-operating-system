//! Email adapter — DbEmail → LinkingContext.

use std::sync::Arc;

use crate::db::ActionDb;
use crate::db::types::DbEmail;
use crate::state::AppState;

use super::primitives::domain_from_email;
use super::types::{
    LinkingContext, LinkOutcome, OwnerRef, OwnerType, Participant, ParticipantRole, Trigger,
};

/// Convert a DbEmail into a LinkingContext for evaluate().
///
/// DbEmail only stores sender_email (no to/cc columns persist in the schema).
/// Participants are therefore limited to the From sender only.
/// TODO(Lane-E): extend when recipient columns are added to DbEmail.
pub fn build_context(
    email: &DbEmail,
    _thread_primary_entity_id: Option<&str>,
    db: &ActionDb,
) -> Result<LinkingContext, String> {
    let mut participants = Vec::new();

    if let Some(ref sender) = email.sender_email {
        participants.push(Participant {
            email: sender.clone(),
            name: email.sender_name.clone(),
            role: ParticipantRole::From,
            person_id: None,
            domain: domain_from_email(sender),
        });
    }

    let graph_version = db.get_entity_graph_version().unwrap_or(0);

    let user_domains = crate::state::load_config()
        .ok()
        .map(|c| c.resolved_user_domains())
        .unwrap_or_default();

    Ok(LinkingContext {
        owner: OwnerRef {
            owner_type: OwnerType::Email,
            owner_id: email.email_id.clone(),
        },
        participants,
        title: email.subject.clone(),
        attendee_count: 1,
        thread_id: email.thread_id.clone(),
        series_id: None,
        graph_version,
        user_domains,
    })
}

/// Evaluate entity linking for an email, then flush any waiting thread children.
///
/// After a primary is resolved, child emails queued in `pending_thread_inheritance`
/// (because they arrived before their parent) have their auto links invalidated so
/// the next enrichment pass re-evaluates them with the parent's primary available.
pub async fn evaluate_email(
    state: Arc<AppState>,
    email: &DbEmail,
    trigger: Trigger,
) -> Result<LinkOutcome, String> {
    let thread_id = email.thread_id.clone();

    let ctx = state.with_db_read(|db| build_context(email, None, db))?;
    let outcome = super::evaluate(state.clone(), ctx, trigger).await?;

    // Flush thread inheritance queue when a primary was just set.
    if outcome.primary.is_some() {
        if let Some(tid) = thread_id {
            let children = state.with_db_read(move |db| {
                db.drain_thread_inheritance_queue(&tid)
            })?;

            if !children.is_empty() {
                log::info!(
                    "entity_linking: invalidating {} pending thread children for re-evaluation",
                    children.len()
                );
                // Invalidate auto links so the next enrichment pass re-evaluates each
                // child with the parent's primary now available for P2 inheritance.
                // TODO(Lane-E): fetch DbEmail by id and call evaluate_email recursively
                // once a get_email_by_id helper is added to ActionDb.
                for child_id in children {
                    let child = child_id.clone();
                    let _ = state
                        .db_write(move |db| db.delete_auto_links_for_owner("email", &child))
                        .await;
                }
            }
        }
    }

    Ok(outcome)
}

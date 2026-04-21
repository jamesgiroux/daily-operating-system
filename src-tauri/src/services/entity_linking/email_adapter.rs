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
/// Builds participants from From, To, and Cc so P4a/P4b/P4c domain evidence
/// rules can evaluate all participants, not just the sender.
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

    // Parse comma-separated To recipients stored at sync time.
    if let Some(ref to_str) = email.to_recipients {
        for addr in to_str.split(',').map(str::trim).filter(|s| s.contains('@')) {
            participants.push(Participant {
                email: addr.to_string(),
                name: None,
                role: ParticipantRole::To,
                person_id: None,
                domain: domain_from_email(addr),
            });
        }
    }

    // Parse comma-separated Cc recipients.
    if let Some(ref cc_str) = email.cc_recipients {
        for addr in cc_str.split(',').map(str::trim).filter(|s| s.contains('@')) {
            participants.push(Participant {
                email: addr.to_string(),
                name: None,
                role: ParticipantRole::Cc,
                person_id: None,
                domain: domain_from_email(addr),
            });
        }
    }

    let attendee_count = participants.len().max(1);
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
        attendee_count,
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
    // For each waiting child email: fetch it and re-evaluate so P2 can now
    // resolve correctly with the parent's primary available.
    if outcome.primary.is_some() {
        if let Some(tid) = thread_id {
            let children = state.with_db_read(move |db| {
                db.drain_thread_inheritance_queue(&tid)
            })?;

            if !children.is_empty() {
                log::info!(
                    "entity_linking: re-evaluating {} pending thread children",
                    children.len()
                );
                for child_id in children {
                    let cid = child_id.clone();
                    let child_email = state
                        .with_db_read(move |db| db.get_email_by_id_for_linking(&cid));
                    match child_email {
                        Ok(Some(child_email)) => {
                            // Box::pin required because evaluate_email calls itself
                            // (async recursion needs explicit boxing in Rust).
                            if let Err(e) = Box::pin(evaluate_email(
                                state.clone(),
                                &child_email,
                                Trigger::EmailThreadUpdate,
                            ))
                            .await
                            {
                                log::warn!(
                                    "entity_linking: thread child {} re-eval failed: {e}",
                                    child_email.email_id
                                );
                            }
                        }
                        Ok(None) => {
                            log::debug!(
                                "entity_linking: thread child {child_id} not found, skipping"
                            );
                        }
                        Err(e) => {
                            log::warn!(
                                "entity_linking: could not fetch thread child {child_id}: {e}"
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(outcome)
}

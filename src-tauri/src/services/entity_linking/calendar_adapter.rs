//! Calendar adapter — CalendarEvent → LinkingContext.

use std::sync::Arc;

use crate::db::ActionDb;
use crate::state::AppState;
use crate::types::CalendarEvent;

use super::primitives::domain_from_email;
use super::types::{
    LinkingContext, LinkOutcome, OwnerRef, OwnerType, Participant, ParticipantRole, Trigger,
};

/// Derive the meeting DB primary id from a CalendarEvent.
///
/// Mirrors the id derivation in the calendar workflow so that
/// `linked_entities_raw.owner_id` matches `meetings.id` everywhere.
fn meeting_db_id(event: &CalendarEvent) -> String {
    event.id.replace('@', "_at_")
}

/// Convert a CalendarEvent into a LinkingContext for evaluate().
///
/// Reads graph_version from entity_graph_version (O(1)).
/// NOTE: CalendarEvent has no `recurring_event_id` field, so `series_id`
/// is always None and rule P3 (series inheritance) is dormant for the
/// calendar surface. TODO(Lane-D): plumb recurring_event_id through
/// CalendarEvent to enable P3.
pub fn build_context(event: &CalendarEvent, db: &ActionDb) -> Result<LinkingContext, String> {
    let owner_id = meeting_db_id(event);
    let graph_version = db.get_entity_graph_version().unwrap_or(0);

    let user_domains = crate::state::load_config()
        .ok()
        .map(|c| c.resolved_user_domains())
        .unwrap_or_default();

    let participants: Vec<Participant> = event
        .attendees
        .iter()
        .map(|email| {
            let email_lower = email.to_lowercase();
            let domain = domain_from_email(&email_lower);
            Participant {
                email: email_lower,
                name: None,
                role: ParticipantRole::Attendee,
                person_id: None,
                domain,
            }
        })
        .collect();

    Ok(LinkingContext {
        owner: OwnerRef {
            owner_type: OwnerType::Meeting,
            owner_id,
        },
        participants,
        title: Some(event.title.clone()),
        attendee_count: event.attendees.len(),
        thread_id: None,
        series_id: None,
        graph_version,
        user_domains,
    })
}

/// Run the deterministic entity linking engine for a calendar event.
///
/// Adds the calendar owner as a participant if not already in the attendee
/// list (Google doesn't guarantee the organiser is included), then calls
/// the four-phase engine.
pub async fn evaluate_meeting(
    state: Arc<AppState>,
    event: &CalendarEvent,
    trigger: Trigger,
) -> Result<LinkOutcome, String> {
    let event_clone = event.clone();

    // Snapshot the authenticated calendar owner email (sync, no await).
    let self_email: Option<String> = {
        let g = state.calendar.google_auth.lock();
        match &*g {
            crate::types::GoogleAuthStatus::Authenticated { email } => {
                Some(email.to_lowercase())
            }
            _ => None,
        }
    };

    let mut ctx = state
        .db_read(move |db| build_context(&event_clone, db))
        .await?;

    // Add the calendar owner as an Attendee if absent.
    if let Some(ref owner_email) = self_email {
        let already_listed = ctx
            .participants
            .iter()
            .any(|p| p.email.eq_ignore_ascii_case(owner_email));
        if !already_listed {
            let domain = domain_from_email(owner_email);
            ctx.participants.push(Participant {
                email: owner_email.clone(),
                name: None,
                role: ParticipantRole::Attendee,
                person_id: None,
                domain,
            });
        }
    }

    super::evaluate(state, ctx, trigger).await
}

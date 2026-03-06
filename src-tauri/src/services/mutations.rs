use chrono::Utc;
use rusqlite::params;
use serde_json::Value;

use crate::db::person_relationships::UpsertRelationship;
use crate::db::{ActionDb, DbAction, DbChatSession, DbMeeting, DbProcessingLog};
use crate::signals::propagation::PropagationEngine;

pub fn set_meeting_prep_context(
    db: &ActionDb,
    meeting_id: &str,
    updated_json: &str,
) -> Result<(), String> {
    crate::services::meetings::set_meeting_prep_context(db, meeting_id, updated_json)
}

pub fn reset_email_dismissals(db: &ActionDb) -> Result<u64, String> {
    db.reset_email_dismissals().map_err(|e| e.to_string())
}

pub fn update_capture_content(
    db: &ActionDb,
    capture_id: &str,
    content: &str,
) -> Result<(), String> {
    crate::services::meetings::update_capture_content(db, capture_id, content)
}

pub fn upsert_account(
    db: &ActionDb,
    engine: &PropagationEngine,
    account: &crate::db::DbAccount,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.upsert_account(account).map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            engine,
            "account",
            &account.id,
            "entity_updated",
            "onboarding",
            None,
            0.8,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

pub fn upsert_project(
    db: &ActionDb,
    engine: &PropagationEngine,
    project: &crate::db::DbProject,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.upsert_project(project).map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            engine,
            "project",
            &project.id,
            "entity_updated",
            "onboarding",
            None,
            0.8,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

pub fn remove_project_keyword(
    db: &ActionDb,
    project_id: &str,
    keyword: &str,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.remove_project_keyword(project_id, keyword)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit(
            tx,
            "project",
            project_id,
            "keywords_updated",
            "user_edit",
            Some(&format!(
                "{{\"removed\":\"{}\"}}",
                keyword.replace('"', "\\\"")
            )),
            0.8,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

pub fn remove_account_keyword(
    db: &ActionDb,
    account_id: &str,
    keyword: &str,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.remove_account_keyword(account_id, keyword)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit(
            tx,
            "account",
            account_id,
            "keywords_updated",
            "user_edit",
            Some(&format!(
                "{{\"removed\":\"{}\"}}",
                keyword.replace('"', "\\\"")
            )),
            0.8,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

pub fn ensure_open_chat_session(
    db: &ActionDb,
    entity_id: Option<&str>,
    entity_type: Option<&str>,
) -> Result<DbChatSession, String> {
    if let Some(existing) = db
        .get_open_chat_session(entity_id, entity_type)
        .map_err(|e| e.to_string())?
    {
        return Ok(existing);
    }

    let now = Utc::now().to_rfc3339();
    let session_id = uuid::Uuid::new_v4().to_string();
    db.create_chat_session(&session_id, entity_id, entity_type, &now, &now)
        .map_err(|e| e.to_string())
}

pub fn append_chat_exchange(
    db: &ActionDb,
    session_id: &str,
    user_content: &str,
    assistant_json: &Value,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    let first_idx = db
        .get_next_chat_turn_index(session_id)
        .map_err(|e| e.to_string())?;

    db.append_chat_turn(
        &uuid::Uuid::new_v4().to_string(),
        session_id,
        first_idx,
        "user",
        user_content,
        &now,
    )
    .map_err(|e| e.to_string())?;

    let assistant_content =
        serde_json::to_string(assistant_json).map_err(|e| format!("serialize failed: {}", e))?;
    db.append_chat_turn(
        &uuid::Uuid::new_v4().to_string(),
        session_id,
        first_idx + 1,
        "assistant",
        &assistant_content,
        &now,
    )
    .map_err(|e| e.to_string())?;

    db.bump_chat_session_stats(session_id, 2, Some(user_content))
        .map_err(|e| e.to_string())
}

pub fn update_meeting_user_layer(
    db: &ActionDb,
    engine: &PropagationEngine,
    meeting_id: &str,
    agenda_json: Option<&str>,
    notes: Option<&str>,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.update_meeting_user_layer(meeting_id, agenda_json, notes)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            engine,
            "meeting",
            meeting_id,
            "meeting_user_layer_updated",
            "user_edit",
            None,
            0.85,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

pub fn queue_clay_sync_for_people(db: &ActionDb, person_ids: &[String]) -> Result<usize, String> {
    if person_ids.is_empty() {
        return Ok(0);
    }
    let now = Utc::now().to_rfc3339();
    for person_id in person_ids {
        let id = uuid::Uuid::new_v4().to_string();
        db.conn_ref()
            .execute(
                "INSERT OR IGNORE INTO clay_sync_state (id, entity_type, entity_id, state, created_at, updated_at)
                 VALUES (?1, 'person', ?2, 'pending', ?3, ?3)",
                params![id, person_id, now],
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(person_ids.len())
}

pub fn create_linear_entity_link(
    db: &ActionDb,
    linear_project_id: &str,
    entity_id: &str,
    entity_type: &str,
) -> Result<(), String> {
    create_linear_entity_link_with_confirmed(db, linear_project_id, entity_id, entity_type, true)
}

pub fn create_linear_entity_link_with_confirmed(
    db: &ActionDb,
    linear_project_id: &str,
    entity_id: &str,
    entity_type: &str,
    confirmed: bool,
) -> Result<(), String> {
    let confirmed_int: i32 = if confirmed { 1 } else { 0 };
    db.conn_ref()
        .execute(
            "INSERT OR IGNORE INTO linear_entity_links (id, linear_project_id, entity_id, entity_type, confirmed)
             VALUES (lower(hex(randomblob(16))), ?1, ?2, ?3, ?4)",
            params![linear_project_id, entity_id, entity_type, confirmed_int],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_linear_entity_link(db: &ActionDb, link_id: &str) -> Result<(), String> {
    db.conn_ref()
        .execute(
            "DELETE FROM linear_entity_links WHERE id = ?1",
            params![link_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn update_entity_metadata(
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    metadata: &str,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.update_entity_metadata(entity_type, entity_id, metadata)?;
        crate::services::signals::emit_and_propagate(
            tx,
            engine,
            entity_type,
            entity_id,
            "entity_metadata_updated",
            "user_edit",
            None,
            0.85,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

pub fn upsert_email_feedback_signal(
    db: &ActionDb,
    email_id: &str,
    corrected_priority: &str,
) -> Result<(), String> {
    let signal_text = format!(
        "User corrected auto-archived email to {}",
        corrected_priority
    );
    db.upsert_email_signal(
        email_id,
        None,
        None,
        "system",
        "account",
        "feedback",
        &signal_text,
        Some(1.0),
        None,
        None,
        None,
    )
    .map(|_| ())
    .map_err(|e| format!("Failed to record correction signal: {}", e))
}

pub fn upsert_timeline_meeting_with_entities(
    db: &ActionDb,
    meeting: &DbMeeting,
    links: &[(String, String)],
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.upsert_meeting(meeting).map_err(|e| e.to_string())?;
        for (entity_id, entity_type) in links {
            tx.link_meeting_entity(&meeting.id, entity_id, entity_type)
                .map_err(|e| e.to_string())?;
        }
        crate::services::signals::emit(
            tx,
            "meeting",
            &meeting.id,
            "meeting_upserted",
            "calendar_sync",
            None,
            0.7,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
pub fn upsert_person_relationship(
    db: &ActionDb,
    engine: &PropagationEngine,
    id: &str,
    from_person_id: &str,
    to_person_id: &str,
    relationship_type: &str,
    direction: &str,
    confidence: f64,
    context_entity_id: Option<&str>,
    context_entity_type: Option<&str>,
    source: &str,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.upsert_person_relationship(&UpsertRelationship {
            id,
            from_person_id,
            to_person_id,
            relationship_type,
            direction,
            confidence,
            context_entity_id,
            context_entity_type,
            source,
        })
        .map_err(|e| format!("Failed to upsert relationship: {}", e))?;

        crate::services::signals::emit_and_propagate(
            tx,
            engine,
            "person",
            from_person_id,
            "relationship_graph_changed",
            "user_action",
            Some(&format!(
                "{{\"relationship_id\":\"{}\",\"other_person_id\":\"{}\"}}",
                id, to_person_id
            )),
            0.9,
        )
        .map_err(|e| format!("signal emit failed (from): {e}"))?;

        crate::services::signals::emit_and_propagate(
            tx,
            engine,
            "person",
            to_person_id,
            "relationship_graph_changed",
            "user_action",
            Some(&format!(
                "{{\"relationship_id\":\"{}\",\"other_person_id\":\"{}\"}}",
                id, from_person_id
            )),
            0.9,
        )
        .map_err(|e| format!("signal emit failed (to): {e}"))?;

        Ok(())
    })
}

pub fn delete_person_relationship(
    db: &ActionDb,
    engine: &PropagationEngine,
    id: &str,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        let person_ids = tx
            .get_person_relationship_by_id(id)
            .map_err(|e| format!("Failed to look up relationship: {}", e))?;
        tx.delete_person_relationship(id)
            .map_err(|e| format!("Failed to delete relationship: {}", e))?;

        if let Some((from_id, to_id)) = person_ids {
            crate::services::signals::emit_and_propagate(
                tx,
                engine,
                "person",
                &from_id,
                "relationship_graph_changed",
                "user_action",
                Some(&format!("{{\"deleted_relationship_id\":\"{}\"}}", id)),
                0.7,
            )
            .map_err(|e| format!("signal emit failed (from): {e}"))?;
            crate::services::signals::emit_and_propagate(
                tx,
                engine,
                "person",
                &to_id,
                "relationship_graph_changed",
                "user_action",
                Some(&format!("{{\"deleted_relationship_id\":\"{}\"}}", id)),
                0.7,
            )
            .map_err(|e| format!("signal emit failed (to): {e}"))?;
        }

        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
pub fn persist_transcript_outcomes(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    meeting_id: &str,
    meeting_title: &str,
    account_id: Option<&str>,
    wins: &[String],
    risks: &[String],
    decisions: &[String],
) -> Result<(), String> {
    db.with_transaction(|tx| {
        for win in wins {
            tx.insert_capture(meeting_id, meeting_title, account_id, "win", win)
                .map_err(|e| format!("insert win capture failed: {e}"))?;
        }
        for risk in risks {
            tx.insert_capture(meeting_id, meeting_title, account_id, "risk", risk)
                .map_err(|e| format!("insert risk capture failed: {e}"))?;
        }
        for decision in decisions {
            tx.insert_capture(meeting_id, meeting_title, account_id, "decision", decision)
                .map_err(|e| format!("insert decision capture failed: {e}"))?;
        }

        let capture_count = wins.len() + risks.len() + decisions.len();
        if capture_count > 0 {
            crate::services::signals::emit(
                tx,
                entity_type,
                entity_id,
                "transcript_outcomes",
                "transcript",
                Some(&format!(
                    "{{\"meeting_id\":\"{}\",\"wins\":{},\"risks\":{},\"decisions\":{}}}",
                    meeting_id,
                    wins.len(),
                    risks.len(),
                    decisions.len()
                )),
                0.75,
            )
            .map_err(|e| format!("signal emit failed: {e}"))?;
        }

        Ok(())
    })
}

pub fn insert_processing_log(db: &ActionDb, log_entry: &DbProcessingLog) -> Result<(), String> {
    db.insert_processing_log(log_entry)
        .map_err(|e| e.to_string())
}

pub fn upsert_action_if_not_completed(db: &ActionDb, action: &DbAction) -> Result<(), String> {
    db.with_transaction(|tx| {
        let wrote = tx
            .upsert_action_if_not_completed_with_status(action)
            .map_err(|e| e.to_string())?;
        if !wrote {
            return Ok(());
        }

        let (entity_type, entity_id) = action_signal_target(action);
        crate::services::signals::emit(
            tx,
            entity_type,
            &entity_id,
            "action_created",
            action.source_type.as_deref().unwrap_or("unknown"),
            Some(&format!(
                "{{\"action_id\":\"{}\",\"title\":\"{}\"}}",
                action.id,
                action.title.replace('"', "\\\"")
            )),
            0.7,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

fn action_signal_target(action: &DbAction) -> (&'static str, String) {
    if let Some(account_id) = action.account_id.as_deref() {
        return ("account", account_id.to_string());
    }
    if let Some(project_id) = action.project_id.as_deref() {
        return ("project", project_id.to_string());
    }
    ("action", action.id.clone())
}

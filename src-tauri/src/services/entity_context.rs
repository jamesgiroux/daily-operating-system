//! Entity context entries CRUD service.
//!
//! Structured knowledge entries for accounts, people, and projects.
//! routes user-authored notes through `intelligence_claims`
//! with claim_type `user_note`; the legacy SQL table remains only as
//! backfill/rollback storage.

use crate::db::claims::{ClaimSensitivity, IntelligenceClaim, TemporalScope};
use crate::services::claims::{
    commit_claim, load_claim_by_id, load_entity_context_claims_active_for_surface,
    subject_ref_from_json, withdraw_claim, ClaimProposal, CommittedClaim,
};
use crate::services::sensitivity::{
    renderable_claim_text_with_value, ClaimDismissalSurface, RenderActor, RenderSurface,
};
use crate::state::AppState;
use crate::types::{EntityContextEntry, EntityContextText};

pub const USER_NOTE_CLAIM_TYPE: &str = "user_note";
const USER_NOTE_LEGACY_NAMESPACE: &str = "b9bd8742-3f99-5b5f-a732-94d1e4e77111";

#[derive(Debug, Clone)]
pub struct LegacyEntityContextEntry {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

pub fn legacy_user_note_claim_id(legacy_entry_id: &str) -> Result<String, String> {
    let namespace = uuid::Uuid::parse_str(USER_NOTE_LEGACY_NAMESPACE)
        .map_err(|error| format!("Invalid user_note namespace UUID: {error}"))?;
    Ok(uuid::Uuid::new_v5(&namespace, legacy_entry_id.as_bytes()).to_string())
}

/// Get all claim-backed context entries for an entity.
pub async fn get_entries(
    entity_type: &str,
    entity_id: &str,
    state: &AppState,
) -> Result<Vec<EntityContextEntry>, String> {
    let entity_type = entity_type.to_string();
    let entity_id = entity_id.to_string();
    state
        .db_read(move |db| {
            let claims = load_entity_context_claims_active_for_surface(
                db,
                &entity_type,
                &entity_id,
                1,
                ClaimDismissalSurface::TauriEntityDetail.as_str(),
            )
            .map_err(|error| format!("Failed to read entity context claims: {error}"))?;
            claims
                .into_iter()
                .map(entity_context_entry_for_claim)
                .collect()
        })
        .await
}

/// Create a new user-authored entity context note.
pub async fn create_entry(
    ctx: &crate::services::context::ServiceContext<'_>,
    entity_type: &str,
    entity_id: &str,
    title: &str,
    content: &str,
    state: &AppState,
) -> Result<EntityContextEntry, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;

    let entity_type = normalize_entity_type(entity_type)?.to_string();
    let entity_id = entity_id.to_string();
    let title = title.to_string();
    let content = content.to_string();
    let observed_at = ctx.clock.now().to_rfc3339();
    let engine = std::sync::Arc::clone(&state.signals.engine);

    state
        .db_write(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let write_ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext)
                .with_actor("user");
            let proposal = user_note_claim_proposal(UserNoteProposal {
                id: None,
                supersedes: None,
                entity_type: &entity_type,
                entity_id: &entity_id,
                title: &title,
                content: &content,
                actor: "user",
                observed_at: &observed_at,
                source_ref: None,
                provenance_json: user_note_provenance_json(None),
            })?;
            let committed = commit_claim(&write_ctx, db, proposal)
                .map_err(|error| format!("Failed to create entity context note claim: {error}"))?;
            let claim = inserted_claim(committed)?;

            crate::services::signals::emit_and_propagate_or_log(
                &write_ctx,
                db,
                &engine,
                &entity_type,
                &entity_id,
                "user_note_added",
                "user_note",
                Some(&title),
                0.85,
            );

            entity_context_entry_for_claim(claim)
        })
        .await
}

/// Update an existing user note by superseding the old immutable claim.
pub async fn update_entry(
    ctx: &crate::services::context::ServiceContext<'_>,
    id: &str,
    title: &str,
    content: &str,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;

    let id = id.to_string();
    let title = title.to_string();
    let content = content.to_string();
    let observed_at = ctx.clock.now().to_rfc3339();
    let engine = std::sync::Arc::clone(&state.signals.engine);

    state
        .db_write(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let write_ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext)
                .with_actor("user");
            let old = load_claim_by_id(db.conn_ref(), &id)
                .map_err(|error| format!("Failed to load entity context note claim: {error}"))?
                .ok_or_else(|| format!("Entity context note not found: {id}"))?;
            ensure_user_note_claim(&old)?;
            let old_entry = entity_context_entry_for_claim(old.clone())?;
            let proposal = user_note_claim_proposal(UserNoteProposal {
                id: None,
                supersedes: Some(&id),
                entity_type: &old_entry.entity_type,
                entity_id: &old_entry.entity_id,
                title: &title,
                content: &content,
                actor: "user",
                observed_at: &observed_at,
                source_ref: None,
                provenance_json: user_note_provenance_json(Some(&id)),
            })?;
            commit_claim(&write_ctx, db, proposal)
                .map_err(|error| format!("Failed to update entity context note claim: {error}"))?;

            crate::services::signals::emit_and_propagate_or_log(
                &write_ctx,
                db,
                &engine,
                &old_entry.entity_type,
                &old_entry.entity_id,
                "user_note_updated",
                "user_note",
                Some(&title),
                0.85,
            );

            Ok(())
        })
        .await
}

/// Delete an entity context note by withdrawing its claim.
pub async fn delete_entry(
    ctx: &crate::services::context::ServiceContext<'_>,
    id: &str,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let id = id.to_string();
    let engine = std::sync::Arc::clone(&state.signals.engine);

    state
        .db_write(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let write_ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext)
                .with_actor("user");
            let old = load_claim_by_id(db.conn_ref(), &id)
                .map_err(|error| format!("Failed to load entity context note claim: {error}"))?
                .ok_or_else(|| format!("Entity context note not found: {id}"))?;
            ensure_user_note_claim(&old)?;
            let old_entry = entity_context_entry_for_claim(old)?;
            withdraw_claim(&write_ctx, db, &id, "user_deleted")
                .map_err(|error| format!("Failed to delete entity context note claim: {error}"))?;

            crate::services::signals::emit_and_propagate_or_log(
                &write_ctx,
                db,
                &engine,
                &old_entry.entity_type,
                &old_entry.entity_id,
                "user_note_deleted",
                "user_note",
                None,
                0.85,
            );

            Ok(())
        })
        .await
}

/// Migrate legacy notes from the people table into user_note claims.
///
/// Called once at startup. The old path wrote `entity_context_entries`;
/// keeps this idempotent but routes new migration writes through `commit_claim`.
pub fn migrate_legacy_notes(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &crate::db::ActionDb,
) -> Result<usize, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let conn = db.conn_ref();
    if !table_has_column(conn, "people", "notes")? {
        return Ok(0);
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, notes FROM people
             WHERE notes IS NOT NULL AND trim(notes) != ''",
        )
        .map_err(|e| format!("Failed to query people notes: {e}"))?;

    let people = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("Failed to read people notes: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map people notes: {e}"))?;

    let mut count = 0usize;
    for (person_id, notes) in people {
        let source_ref = serde_json::json!({
            "kind": "legacy_people_notes",
            "id": person_id,
        })
        .to_string();
        let already_exists: bool = conn
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM intelligence_claims
                    WHERE claim_type = ?1
                      AND source_ref = ?2
                      AND claim_state IN ('active', 'dormant')
                 )",
                rusqlite::params![USER_NOTE_CLAIM_TYPE, &source_ref],
                |row| row.get::<_, i64>(0).map(|value| value != 0),
            )
            .map_err(|e| format!("Failed to check migrated person note: {e}"))?;
        if already_exists {
            continue;
        }

        let observed_at = ctx.clock.now().to_rfc3339();
        let proposal = user_note_claim_proposal(UserNoteProposal {
            id: None,
            supersedes: None,
            entity_type: "person",
            entity_id: &person_id,
            title: "Notes",
            content: &notes,
            actor: "user",
            observed_at: &observed_at,
            source_ref: Some(&source_ref),
            provenance_json: user_note_provenance_json(None),
        })?;
        commit_claim(ctx, db, proposal)
            .map_err(|error| format!("Failed to migrate person notes to claim: {error}"))?;
        count += 1;
    }

    if count > 0 {
        log::info!("Migrated {count} legacy people notes to user_note claims");
    }

    Ok(count)
}

pub fn legacy_entity_context_entries(
    db: &crate::db::ActionDb,
) -> Result<Vec<LegacyEntityContextEntry>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT id, entity_type, entity_id, title, content, created_at, updated_at
             FROM entity_context_entries
             ORDER BY created_at ASC, id ASC",
        )
        .map_err(|error| format!("Failed to prepare legacy user_note backfill query: {error}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(LegacyEntityContextEntry {
                id: row.get("id")?,
                entity_type: row.get("entity_type")?,
                entity_id: row.get("entity_id")?,
                title: row.get("title")?,
                content: row.get("content")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .map_err(|error| format!("Failed to query legacy user_note rows: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to map legacy user_note rows: {error}"))?;

    Ok(rows)
}

pub fn commit_backfilled_user_note(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &crate::db::ActionDb,
    legacy: &LegacyEntityContextEntry,
) -> Result<String, String> {
    let claim_id = legacy_user_note_claim_id(&legacy.id)?;
    let exists: bool = db
        .conn_ref()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM intelligence_claims WHERE id = ?1)",
            rusqlite::params![&claim_id],
            |row| row.get::<_, i64>(0).map(|value| value != 0),
        )
        .map_err(|error| format!("Failed to check existing user_note claim: {error}"))?;
    if !exists {
        let source_ref = serde_json::json!({
            "kind": "user_note",
            "id": legacy.id,
        })
        .to_string();
        let proposal = user_note_claim_proposal(UserNoteProposal {
            id: Some(&claim_id),
            supersedes: None,
            entity_type: &legacy.entity_type,
            entity_id: &legacy.entity_id,
            title: &legacy.title,
            content: &legacy.content,
            actor: "user",
            observed_at: &legacy.created_at,
            source_ref: Some(&source_ref),
            provenance_json: user_note_backfill_provenance_json(&legacy.id),
        })?;
        commit_claim(ctx, db, proposal)
            .map_err(|error| format!("Failed to backfill user_note claim: {error}"))?;
    }

    Ok(claim_id)
}

pub fn entity_context_entry_for_claim(
    claim: IntelligenceClaim,
) -> Result<EntityContextEntry, String> {
    let value: serde_json::Value = serde_json::from_str(&claim.subject_ref)
        .map_err(|error| format!("Invalid entity context claim subject_ref JSON: {error}"))?;
    let (entity_type, entity_id) = match subject_ref_from_json(&value)
        .map_err(|error| format!("Invalid entity context claim subject_ref: {error}"))?
    {
        crate::db::claim_invalidation::SubjectRef::Account { id } => ("account".to_string(), id),
        crate::db::claim_invalidation::SubjectRef::Person { id } => ("person".to_string(), id),
        crate::db::claim_invalidation::SubjectRef::Project { id } => ("project".to_string(), id),
        crate::db::claim_invalidation::SubjectRef::Meeting { id } => ("meeting".to_string(), id),
        crate::db::claim_invalidation::SubjectRef::Email { .. }
        | crate::db::claim_invalidation::SubjectRef::Multi(_)
        | crate::db::claim_invalidation::SubjectRef::Global => {
            return Err(format!(
                "Claim `{}` has unsupported entity context subject",
                claim.id
            ));
        }
    };

    let updated_at = claim
        .reactivated_at
        .clone()
        .unwrap_or_else(|| claim.created_at.clone());
    let title = title_for_entity_context_claim(&claim);
    let actor = RenderActor::user("user", Some("user"));
    let rendered_title = renderable_entity_context_text(&claim, &title, &actor)?;
    let rendered_content = renderable_entity_context_text(&claim, &claim.text, &actor)?;

    Ok(EntityContextEntry {
        id: claim.id,
        entity_type,
        entity_id,
        title: rendered_title,
        content: rendered_content,
        created_at: claim.created_at,
        updated_at,
    })
}

fn renderable_entity_context_text(
    claim: &IntelligenceClaim,
    value: &str,
    actor: &RenderActor,
) -> Result<EntityContextText, String> {
    renderable_claim_text_with_value(claim, value, RenderSurface::TauriEntityDetail, actor)
        .map(EntityContextText::Claim)
        .ok_or_else(|| format!("Claim `{}` cannot render for entity context", claim.id))
}

struct UserNoteProposal<'a> {
    id: Option<&'a str>,
    supersedes: Option<&'a str>,
    entity_type: &'a str,
    entity_id: &'a str,
    title: &'a str,
    content: &'a str,
    actor: &'a str,
    observed_at: &'a str,
    source_ref: Option<&'a str>,
    provenance_json: String,
}

fn user_note_claim_proposal(input: UserNoteProposal<'_>) -> Result<ClaimProposal, String> {
    let entity_type = normalize_entity_type(input.entity_type)?;
    Ok(ClaimProposal {
        id: input.id.map(str::to_string),
        subject_ref: serde_json::json!({
            "kind": entity_type,
            "id": input.entity_id,
        })
        .to_string(),
        claim_type: USER_NOTE_CLAIM_TYPE.to_string(),
        field_path: None,
        topic_key: None,
        text: input.content.to_string(),
        actor: input.actor.to_string(),
        data_source: "manual".to_string(),
        source_ref: input.source_ref.map(str::to_string),
        source_asof: Some(input.observed_at.to_string()),
        observed_at: input.observed_at.to_string(),
        provenance_json: input.provenance_json,
        metadata_json: Some(
            serde_json::json!({
                "title": input.title,
            })
            .to_string(),
        ),
        thread_id: None,
        temporal_scope: Some(TemporalScope::State),
        sensitivity: Some(ClaimSensitivity::Internal),
        supersedes: input.supersedes.map(str::to_string),
        tombstone: None,
    })
}

fn normalize_entity_type(entity_type: &str) -> Result<&'static str, String> {
    match entity_type
        .trim()
        .trim_end_matches('s')
        .to_ascii_lowercase()
        .as_str()
    {
        "account" => Ok("account"),
        "person" | "people" => Ok("person"),
        "project" => Ok("project"),
        other => Err(format!(
            "Unsupported entity context note subject kind: {other}"
        )),
    }
}

fn inserted_claim(committed: CommittedClaim) -> Result<IntelligenceClaim, String> {
    match committed {
        CommittedClaim::Inserted { claim } => Ok(claim),
        CommittedClaim::Reinforced { claim, .. } => Ok(claim),
        other => Err(format!("Expected inserted user_note claim, got {other:?}")),
    }
}

fn ensure_user_note_claim(claim: &IntelligenceClaim) -> Result<(), String> {
    if claim.claim_type == USER_NOTE_CLAIM_TYPE {
        Ok(())
    } else {
        Err(format!(
            "Entity context note mutation only supports user_note claims, got {}",
            claim.claim_type
        ))
    }
}

fn title_for_entity_context_claim(claim: &IntelligenceClaim) -> String {
    if claim.claim_type == USER_NOTE_CLAIM_TYPE {
        if let Some(title) = claim
            .metadata_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
            .and_then(|value| {
                value
                    .get("title")
                    .and_then(|title| title.as_str())
                    .map(str::trim)
                    .map(str::to_string)
            })
            .filter(|title| !title.is_empty())
        {
            return title;
        }
        return "Note".to_string();
    }

    match claim
        .field_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(field_path) => format!("{}: {field_path}", claim.claim_type),
        None => claim.claim_type.clone(),
    }
}

fn user_note_provenance_json(supersedes: Option<&str>) -> String {
    serde_json::json!({
        "actor": "user",
        "data_source": "manual",
        "source": "tauri_entity_context",
        "supersedes": supersedes,
    })
    .to_string()
}

fn user_note_backfill_provenance_json(legacy_id: &str) -> String {
    serde_json::json!({
        "actor": "user",
        "data_source": "manual",
        "source": {
            "kind": "user_note",
            "id": legacy_id,
        },
        "backfill": "DOS-411",
    })
    .to_string()
}

fn table_has_column(
    conn: &rusqlite::Connection,
    table: &str,
    column: &str,
) -> Result<bool, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|error| format!("Failed to inspect {table}: {error}"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("Failed to inspect {table} columns: {error}"))?;
    for name in columns {
        if name.map_err(|error| format!("Failed to read {table} column: {error}"))? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

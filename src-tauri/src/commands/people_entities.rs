use super::*;

#[tauri::command]
pub async fn get_people(
    relationship: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::PersonListItem>, String> {
    state
        .db_read(move |db| {
            db.get_people_with_signals(relationship.as_deref())
                .map_err(|e| e.to_string())
        })
        .await
}

/// Person detail result including signals, linked entities, and recent meetings.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonDetailResult {
    #[serde(flatten)]
    pub person: crate::db::DbPerson,
    pub signals: Option<crate::db::PersonSignals>,
    pub entities: Vec<EntitySummary>,
    pub recent_meetings: Vec<MeetingSummary>,
    pub recent_captures: Vec<crate::db::DbCapture>,
    pub recent_email_signals: Vec<crate::db::DbEmailSignal>,
    pub intelligence: Option<crate::intelligence::IntelligenceJson>,
    pub open_actions: Vec<crate::db::DbAction>,
    pub upcoming_meetings: Vec<MeetingSummary>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntitySummary {
    pub id: String,
    pub name: String,
    pub entity_type: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSummary {
    pub id: String,
    pub title: String,
    pub start_time: String,
    pub meeting_type: String,
}

/// Richer meeting summary with optional prep context (ADR-0063).
/// Used on account detail pages where prep preview is needed.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingPreview {
    pub id: String,
    pub title: String,
    pub start_time: String,
    pub meeting_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_context: Option<PrepContext>,
}

/// Get full detail for a person (person + signals + entities + recent meetings).
#[tauri::command]
pub async fn get_person_detail(
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<PersonDetailResult, String> {
    crate::services::people::get_person_detail(&person_id, &state).await
}

/// Search people by name, email, or organization.
#[tauri::command]
pub async fn search_people(
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbPerson>, String> {
    state
        .db_read(move |db| db.search_people(&query, 50).map_err(|e| e.to_string()))
        .await
}

/// Update a single field on a person (role, organization, notes, relationship).
/// Also updates the person's workspace files.
#[tauri::command]
pub async fn update_person(
    person_id: String,
    field: String,
    value: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::people::update_person_field(
                &ctx, db, &app_state, &person_id, &field, &value,
            )
        })
        .await
}

/// Link a person to an entity (account/project).
/// Regenerates person.json so the link persists in the filesystem (ADR-0048).
#[tauri::command]
pub async fn link_person_entity(
    person_id: String,
    entity_id: String,
    relationship_type: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::people::link_person_entity(
                &ctx,
                db,
                &app_state,
                &person_id,
                &entity_id,
                &relationship_type,
            )
        })
        .await
}

/// Unlink a person from an entity.
/// Regenerates person.json so the removal persists in the filesystem (ADR-0048).
#[tauri::command]
pub async fn unlink_person_entity(
    person_id: String,
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::people::unlink_person_entity(
                &ctx, db, &app_state, &person_id, &entity_id,
            )
        })
        .await
}

/// Get people linked to an entity.
#[tauri::command]
pub async fn get_people_for_entity(
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbPerson>, String> {
    state
        .db_read(move |db| {
            db.get_people_for_entity(&entity_id)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Get people who attended a specific meeting.
#[tauri::command]
pub async fn get_meeting_attendees(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbPerson>, String> {
    state
        .db_read(move |db| {
            db.get_meeting_attendees(&meeting_id)
                .map_err(|e| e.to_string())
        })
        .await
}

// =========================================================================
// Meeting-Entity M2M (I52)
// =========================================================================

/// Link a meeting to an entity (account/project) via the junction table.
/// ADR-0086: After relinking, clears prep_frozen_json and enqueues for
/// mechanical re-assembly from the new entity's intelligence.
#[tauri::command]
pub async fn link_meeting_entity(
    meeting_id: String,
    entity_id: String,
    entity_type: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::meetings::link_meeting_entity_with_prep_queue(
        &state,
        &meeting_id,
        &entity_id,
        &entity_type,
    )
    .await
}

/// Remove a meeting-entity link from the junction table.
/// ADR-0086: After unlinking, clears prep_frozen_json and enqueues for
/// mechanical re-assembly without the removed entity's intelligence.
#[tauri::command]
pub async fn unlink_meeting_entity(
    meeting_id: String,
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::meetings::unlink_meeting_entity_with_prep_queue(
        &state,
        &meeting_id,
        &entity_id,
    )
    .await
}

/// DOS-240: Dismiss an auto-resolved meeting entity. Unlinks it AND records
/// a persistent dismissal so future calendar-sync / resolver sweeps do not
/// re-link the same (meeting, entity, type) tuple.
#[tauri::command]
pub async fn dismiss_meeting_entity(
    meeting_id: String,
    entity_id: String,
    entity_type: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::meetings::dismiss_meeting_entity(
        &state,
        &meeting_id,
        &entity_id,
        &entity_type,
        None,
    )
    .await
}

/// DOS-240: Undo a previous dismissal. Removes the dismissal record so the
/// entity can auto-link again on the next calendar-sync or resolver pass.
#[tauri::command]
pub async fn restore_meeting_entity(
    meeting_id: String,
    entity_id: String,
    entity_type: String,
    state: State<'_, Arc<AppState>>,
) -> Result<bool, String> {
    crate::services::meetings::restore_meeting_entity(&state, &meeting_id, &entity_id, &entity_type)
        .await
}

// =========================================================================
// DOS-258: entity linking manual overrides
// =========================================================================

/// Set (or clear) the primary entity for a meeting or email.
/// Writes a source='user' row to linked_entities_raw (P1 override).
#[tauri::command]
pub async fn set_entity_link_primary(
    owner_type: String,
    owner_id: String,
    entity_id: Option<String>,
    entity_type: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let ot = crate::services::entity_linking::OwnerType::try_from(owner_type.as_str())
        .map_err(|e| format!("invalid owner_type: {e}"))?;
    let entity_ref = entity_id.map(|id| crate::services::entity_linking::EntityRef {
        entity_id: id,
        entity_type: entity_type.unwrap_or_else(|| "account".to_string()),
    });
    crate::services::entity_linking::manual_set_primary(
        state.inner().clone(),
        ot,
        owner_id,
        entity_ref,
    )
    .await
    .map(|_| ())
}

/// Dismiss a suggested entity link for a meeting or email.
/// Writes a linking_dismissals tombstone + marks raw row as user_dismissed.
#[tauri::command]
pub async fn dismiss_entity_link(
    owner_type: String,
    owner_id: String,
    entity_id: String,
    entity_type: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let ot = crate::services::entity_linking::OwnerType::try_from(owner_type.as_str())
        .map_err(|e| format!("invalid owner_type: {e}"))?;
    crate::services::entity_linking::manual_dismiss(
        state.inner().clone(),
        ot,
        owner_id,
        crate::services::entity_linking::EntityRef {
            entity_id,
            entity_type,
        },
    )
    .await
    .map(|_| ())
}

/// Undo a previous entity link dismissal.
#[tauri::command]
pub async fn restore_entity_link(
    owner_type: String,
    owner_id: String,
    entity_id: String,
    entity_type: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let ot = crate::services::entity_linking::OwnerType::try_from(owner_type.as_str())
        .map_err(|e| format!("invalid owner_type: {e}"))?;
    crate::services::entity_linking::manual_undismiss(
        state.inner().clone(),
        ot,
        owner_id,
        crate::services::entity_linking::EntityRef {
            entity_id,
            entity_type,
        },
    )
    .await
    .map(|_| ())
}

/// Get all entities linked to a meeting via the junction table.
#[tauri::command]
pub async fn get_meeting_entities(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::entity::DbEntity>, String> {
    state
        .db_read(move |db| {
            db.get_meeting_entities(&meeting_id)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Reassign a meeting's entity with full cascade to actions, captures, and intelligence.
/// Clears existing entity links, sets the new one, and cascades to related tables.
/// Emits `prep-ready` event on successful rebuild (I477).
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn update_meeting_entity(
    meeting_id: String,
    entity_id: Option<String>,
    entity_type: String,
    meeting_title: String,
    start_time: String,
    meeting_type_str: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let ctx = crate::services::meetings::MeetingMutationCtx {
        state: &state,
        meeting_id: &meeting_id,
        app_handle: Some(&app_handle),
    };
    crate::services::meetings::update_meeting_entity(
        ctx,
        entity_id.as_deref(),
        &entity_type,
        &meeting_title,
        &start_time,
        &meeting_type_str,
    )
    .await
}

// =========================================================================
// Additive Meeting-Entity Link/Unlink (I184 multi-entity)
// =========================================================================

/// Add an entity link to a meeting with full cascade (people, intelligence).
/// Unlike `update_meeting_entity` which clears-and-replaces, this is additive.
/// Emits `prep-ready` event on successful rebuild (I477).
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn add_meeting_entity(
    meeting_id: String,
    entity_id: String,
    entity_type: String,
    meeting_title: String,
    start_time: String,
    meeting_type_str: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let ctx = crate::services::meetings::MeetingMutationCtx {
        state: &state,
        meeting_id: &meeting_id,
        app_handle: Some(&app_handle),
    };
    crate::services::meetings::add_meeting_entity(
        ctx,
        &entity_id,
        &entity_type,
        &meeting_title,
        &start_time,
        &meeting_type_str,
    )
    .await
}

/// Remove an entity link from a meeting with cleanup (legacy account_id, intelligence).
/// Emits `prep-ready` event on successful rebuild (I477).
#[tauri::command]
pub async fn remove_meeting_entity(
    meeting_id: String,
    entity_id: String,
    entity_type: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let ctx = crate::services::meetings::MeetingMutationCtx {
        state: &state,
        meeting_id: &meeting_id,
        app_handle: Some(&app_handle),
    };
    crate::services::meetings::remove_meeting_entity(ctx, &entity_id, &entity_type).await
}

// =========================================================================
// Entity Keyword Management (I305)
// =========================================================================

/// Remove a keyword from a project's auto-extracted keyword list.
#[tauri::command]
pub async fn remove_project_keyword(
    project_id: String,
    keyword: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let state_for_ctx = state.inner().clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::mutations::remove_project_keyword(&ctx, db, &project_id, &keyword)
        })
        .await
}

/// Remove a keyword from an account's auto-extracted keyword list.
#[tauri::command]
pub async fn remove_account_keyword(
    account_id: String,
    keyword: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let state_for_ctx = state.inner().clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::mutations::remove_account_keyword(&ctx, db, &account_id, &keyword)
        })
        .await
}

// =========================================================================
// Person Creation (I129)
// =========================================================================

/// Create a new person manually. Returns the generated person ID.
#[tauri::command]
pub async fn create_person(
    email: String,
    name: String,
    organization: Option<String>,
    role: Option<String>,
    relationship: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let email = crate::util::validate_email(&email)?;
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::people::create_person(
                &ctx,
                db,
                &app_state,
                &email,
                &name,
                organization.as_deref(),
                role.as_deref(),
                relationship.as_deref(),
            )
        })
        .await
}

/// Merge two people: transfer all references from `remove_id` to `keep_id`, then delete the removed person.
/// Also cleans up filesystem directories and regenerates the kept person's files.
#[tauri::command]
pub async fn merge_people(
    keep_id: String,
    remove_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::people::merge_people(&ctx, db, &app_state, &keep_id, &remove_id)
        })
        .await
}

/// Delete a person and all their references. Also removes their filesystem directory.
#[tauri::command]
pub async fn delete_person(
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::people::delete_person(&ctx, db, &app_state, &person_id)
        })
        .await
}

/// Enrich a person with intelligence assessment (relationship intelligence).
/// Uses split-lock pattern (I173) — DB lock held only briefly during gather/write.
#[tauri::command]
pub async fn enrich_person(
    app_handle: tauri::AppHandle,
    person_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::intelligence::IntelligenceJson, String> {
    crate::services::intelligence::enrich_entity(
        person_id,
        "person".to_string(),
        &state,
        Some(&app_handle),
    )
    .await
}

// =========================================================================
// I529: Intelligence Quality Feedback
// =========================================================================

/// Submit feedback (positive/negative) on an intelligence field for an entity.
#[tauri::command]
pub async fn submit_intelligence_feedback(
    entity_id: String,
    entity_type: String,
    field: String,
    feedback_type: String,
    context: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // DOS-209 (W2-A): construct ServiceContext at the command boundary
    // and pass it into the service mutator. Arc-clone state so the
    // closure can build the context inside the db_write lane.
    let state_for_ctx = Arc::clone(&state);
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::feedback::submit_intelligence_feedback(
                &ctx,
                db,
                &entity_id,
                &entity_type,
                &field,
                &feedback_type,
                context.as_deref(),
            )
        })
        .await
}

/// DOS-41: Submit a consolidated intelligence correction.
///
/// Replaces the legacy thumbs up/down + separate "replaced" paths with a
/// single command that handles all supported user actions:
/// - `confirmed`  — user agrees with the AI output
/// - `rejected`   — user disagrees but keeps the content visible
/// - `annotated`  — user adds context without rejecting the output
/// - `corrected`  — user replaces the output with a new value
/// - `dismissed`  — user marks the output wrong and wants it hidden
///
/// Frontend integration: `useIntelligenceCorrection` hook. Component
/// placement (`IntelligenceCorrection.tsx`) lands in Wave 1.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitIntelligenceCorrectionRequest {
    pub entity_id: String,
    pub entity_type: String,
    pub field: String,
    pub action: String,
    pub corrected_value: Option<String>,
    pub annotation: Option<String>,
    pub item_key: Option<String>,
}

#[tauri::command]
pub async fn submit_intelligence_correction(
    request: SubmitIntelligenceCorrectionRequest,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let parsed = crate::db::feedback::CorrectionAction::parse(&request.action)?;
    // DOS-209 (W2-A): construct ServiceContext at the command boundary.
    let state_for_ctx = Arc::clone(&state);
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::feedback::submit_intelligence_correction(
                &ctx,
                db,
                crate::services::feedback::SubmitIntelligenceCorrectionInput {
                    entity_id: &request.entity_id,
                    entity_type: &request.entity_type,
                    field: &request.field,
                    action: parsed,
                    corrected_value: request.corrected_value.as_deref(),
                    annotation: request.annotation.as_deref(),
                    item_key: request.item_key.as_deref(),
                },
            )
        })
        .await
}

/// Get all feedback records for an entity.
#[tauri::command]
pub async fn get_entity_feedback(
    entity_id: String,
    entity_type: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::intelligence_feedback::FeedbackRow>, String> {
    state
        .db_read(move |db| db.get_entity_feedback(&entity_id, &entity_type))
        .await
}

// =========================================================================
// DOS-258: read linked entities from the new view
// =========================================================================

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedEntityDto {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub role: String, // "primary" | "related" | "auto_suggested"
    pub confidence: Option<f64>,
    pub applied_rule: Option<String>,
}

/// Read linked entities from the DOS-258 linked_entities view.
///
/// Returns primary + related entities from `linked_entities_raw` (excluding
/// user_dismissed rows). Falls back to empty when no entries exist yet.
/// The meeting chip uses the `role` field directly to render primary vs related.
#[tauri::command]
pub async fn get_linked_entities_for_owner(
    owner_type: String,
    owner_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<LinkedEntityDto>, String> {
    state
        .db_read(move |db| {
            let conn = db.conn_ref();
            let mut stmt = conn
                .prepare(
                    "SELECT lr.entity_id, lr.entity_type, lr.role,
                            lr.confidence, lr.rule_id,
                            COALESCE(acc.name, proj.name, p.name, lr.entity_id) as name
                     FROM linked_entities lr
                     LEFT JOIN accounts acc
                          ON lr.entity_type = 'account' AND acc.id = lr.entity_id
                     LEFT JOIN projects proj
                          ON lr.entity_type = 'project' AND proj.id = lr.entity_id
                     LEFT JOIN people p
                          ON lr.entity_type = 'person' AND p.id = lr.entity_id
                     WHERE lr.owner_type = ?1 AND lr.owner_id = ?2
                     ORDER BY
                       CASE lr.role WHEN 'primary' THEN 0
                                    WHEN 'related' THEN 1
                                    ELSE 2 END",
                )
                .map_err(|e| format!("prepare get_linked_entities_for_owner: {e}"))?;
            let mut rows = stmt
                .query(rusqlite::params![owner_type, owner_id])
                .map_err(|e| format!("get_linked_entities_for_owner query: {e}"))?;
            let mut results = Vec::new();
            while let Some(row) = rows
                .next()
                .map_err(|e| format!("get_linked_entities_for_owner row: {e}"))?
            {
                results.push(LinkedEntityDto {
                    id: row.get(0).unwrap_or_default(),
                    entity_type: row.get(1).unwrap_or_default(),
                    role: row.get(2).unwrap_or_default(),
                    confidence: row.get(3).ok(),
                    applied_rule: row.get(4).ok(),
                    name: row.get(5).unwrap_or_default(),
                });
            }
            Ok(results)
        })
        .await
}

/// Purge inferred account_domains before DOS-258 flag flip (admin/devtools).
///
/// Removes all rows where source='inferred'. User-entered ('user') and
/// enrichment-sourced ('enrichment') domains are preserved.
#[tauri::command]
pub async fn rebuild_account_domains(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    state
        .db_write(|db| {
            crate::services::entity_linking::repository::raw_rebuild_account_domains(db)
                .map(|_| "account_domains rebuild complete".to_string())
        })
        .await
}

/// Get all active suppression tombstones for an entity.
#[tauri::command]
pub async fn get_entity_suppressions(
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::SuppressionTombstone>, String> {
    state
        .db_read(move |db| {
            db.get_active_suppressions(&entity_id)
                .map_err(|e| e.to_string())
        })
        .await
}

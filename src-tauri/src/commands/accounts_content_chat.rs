use super::*;

/// Account list item with computed fields for the list page.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountListItem {
    pub id: String,
    pub name: String,
    pub lifecycle: Option<String>,
    pub arr: Option<f64>,
    pub health: Option<String>,
    pub nps: Option<i32>,
    pub renewal_date: Option<String>,
    pub open_action_count: usize,
    pub days_since_last_meeting: Option<i64>,
    pub parent_id: Option<String>,
    pub parent_name: Option<String>,
    pub child_count: usize,
    pub is_parent: bool,
    pub account_type: crate::db::AccountType,
    pub archived: bool,
    /// User health sentiment assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_health_sentiment: Option<String>,
    /// When the sentiment was last set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentiment_set_at: Option<String>,
}

/// Full account detail for the detail page.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDetailResult {
    pub id: String,
    pub name: String,
    pub lifecycle: Option<String>,
    pub arr: Option<f64>,
    pub health: Option<String>,
    pub nps: Option<i32>,
    pub renewal_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_stage: Option<String>,
    /// Contract: Separate commercial opportunity stage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commercial_stage: Option<String>,
    pub contract_start: Option<String>,
    pub company_overview: Option<crate::accounts::CompanyOverview>,
    pub strategic_programs: Vec<crate::accounts::StrategicProgram>,
    pub notes: Option<String>,
    pub open_actions: Vec<crate::db::DbAction>,
    pub upcoming_meetings: Vec<MeetingSummary>,
    pub recent_meetings: Vec<MeetingPreview>,
    /// total count of meetings linked to this account
    /// above the accepted-confidence floor (0.70). `recent_meetings` is
    /// capped at 10 for preview rendering; this field gives the About-
    /// dossier its true "N meetings on record" number.
    #[serde(default)]
    pub meeting_total_count: i64,
    /// total count of meetings with a transcript on
    /// record for this account (unbounded). The preview/manifest lists
    /// remain capped; this is the dossier count source of truth.
    #[serde(default)]
    pub transcript_total_count: i64,
    pub linked_people: Vec<crate::db::DbPerson>,
    pub account_team: Vec<crate::db::DbAccountTeamMember>,
    pub account_team_import_notes: Vec<crate::db::DbAccountTeamImportNote>,
    pub signals: Option<crate::db::StakeholderSignals>,
    pub recent_captures: Vec<crate::db::DbCapture>,
    pub recent_email_signals: Vec<crate::db::DbEmailSignal>,
    pub parent_id: Option<String>,
    pub parent_name: Option<String>,
    pub children: Vec<AccountChildSummary>,
    pub parent_aggregate: Option<crate::db::ParentAggregate>,
    pub account_type: crate::db::AccountType,
    pub archived: bool,
    #[serde(default)]
    pub objectives: Vec<crate::types::AccountObjective>,
    #[serde(default)]
    pub lifecycle_changes: Vec<crate::db::DbLifecycleChange>,
    #[serde(default)]
    pub products: Vec<crate::db::DbAccountProduct>,
    #[serde(default)]
    pub field_provenance: Vec<crate::db::DbAccountFieldProvenance>,
    #[serde(default)]
    pub field_conflicts: Vec<crate::types::AccountFieldConflictSuggestion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence: Option<crate::intelligence::IntelligenceJson>,
    /// Acceptance criterion: Recently auto-completed milestones for timeline display.
    #[serde(default)]
    pub auto_completed_milestones: Vec<crate::types::AccountMilestone>,
    /// Technical footprint, adoption, and service-delivery data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub technical_footprint: Option<crate::db::DbAccountTechnicalFootprint>,
    /// DB-first stakeholder read model: all stakeholders with provenance.
    #[serde(default)]
    pub stakeholders_full: Vec<crate::db::DbStakeholderFull>,
    /// Source references for promoted account facts.
    #[serde(default)]
    pub source_refs: Vec<crate::db::DbAccountSourceRef>,
    /// User health sentiment assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_health_sentiment: Option<String>,
    /// When the sentiment was last set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentiment_set_at: Option<String>,
    /// Most recent sentiment journal note for the current value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentiment_note: Option<String>,
    /// Sentiment journal entries — last 90 days.
    #[serde(default)]
    pub sentiment_history: Vec<crate::db::accounts::DbSentimentJournalEntry>,
    /// Daily computed-health sparkline — last 90 days.
    #[serde(default)]
    pub health_sparkline: Vec<crate::db::accounts::DbHealthSparklinePoint>,
    /// Glean leading-signal enrichment bundle (champion risk, usage
    /// trends, sentiment divergence, transcript extraction, commercial signals,
    /// advocacy, quote wall). Null when Glean is not configured or enrichment
    /// has not yet run for this account.
    #[serde(skip_serializing_if = "Option::is_none", rename = "gleanSignals")]
    pub glean_signals: Option<crate::intelligence::glean_leading_signals::HealthOutlookSignals>,
    /// Regression guard: Current risk-briefing generation job status.
    /// Present when a briefing has ever been enqueued for this account; the
    /// frontend uses this to render progress, surface failures, and expose a
    /// retry affordance. Replaces the old fire-and-forget behaviour where
    /// failures only appeared in the log stream.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_briefing_job: Option<crate::db::accounts::DbRiskBriefingJob>,
}

/// Compact child account summary for parent detail pages.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountChildSummary {
    pub id: String,
    pub name: String,
    pub health: Option<String>,
    pub arr: Option<f64>,
    pub open_action_count: usize,
    pub account_type: String,
}

#[tauri::command]
pub async fn get_accounts_list(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<AccountListItem>, String> {
    state
        .db_read(crate::services::accounts::get_accounts_list)
        .await
}

/// Lightweight list of ALL accounts (parents + children) for entity pickers.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickerAccount {
    pub id: String,
    pub name: String,
    pub parent_name: Option<String>,
    pub account_type: crate::db::AccountType,
}

#[tauri::command]
pub async fn get_accounts_for_picker(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<PickerAccount>, String> {
    state
        .db_read(crate::services::accounts::get_accounts_for_picker)
        .await
}

/// Get child accounts for a parent.
#[tauri::command]
pub async fn get_child_accounts_list(
    parent_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<AccountListItem>, String> {
    state
        .db_read(move |db| crate::services::accounts::get_child_accounts_list(db, &parent_id))
        .await
}

/// Get ancestor accounts for breadcrumb navigation.
#[tauri::command]
pub async fn get_account_ancestors(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAccount>, String> {
    state
        .db_read(move |db| {
            db.get_account_ancestors(&account_id)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Get all descendant accounts for a given ancestor.
#[tauri::command]
pub async fn get_descendant_accounts(
    ancestor_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAccount>, String> {
    state
        .db_read(move |db| {
            db.get_descendant_accounts(&ancestor_id)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Convert a DbAccount to an AccountListItem with computed signals.
fn account_to_list_item(
    a: &crate::db::DbAccount,
    db: &crate::db::ActionDb,
    child_count: usize,
) -> AccountListItem {
    crate::services::accounts::account_to_list_item(a, db, child_count)
}

/// Get full detail for an account (DB fields + narrative JSON + computed data).
#[tauri::command]
pub async fn get_account_detail(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<AccountDetailResult, String> {
    let app_state = state.inner().clone();
    let ctx = app_state.live_service_context();
    crate::services::accounts::get_account_detail(&ctx, &account_id, &app_state).await
}

/// Get account-team members.
#[tauri::command]
pub async fn get_account_team(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAccountTeamMember>, String> {
    state
        .db_read(move |db| db.get_account_team(&account_id).map_err(|e| e.to_string()))
        .await
}

/// Add a person-role pair to an account team.
#[tauri::command]
pub async fn add_account_team_member(
    account_id: String,
    person_id: String,
    role: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::add_account_team_member(
                &ctx,
                db,
                &app_state,
                &account_id,
                &person_id,
                &role,
            )
        })
        .await
}

/// Replace all roles for a team member (single-select role change).
#[tauri::command]
pub async fn set_team_member_role(
    account_id: String,
    person_id: String,
    new_role: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::set_team_member_role(
                &ctx,
                db,
                &app_state,
                &account_id,
                &person_id,
                &new_role,
            )
        })
        .await
}

/// Remove a person-role pair from an account team.
#[tauri::command]
pub async fn remove_account_team_member(
    account_id: String,
    person_id: String,
    role: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::remove_account_team_member(
                &ctx,
                db,
                &app_state,
                &account_id,
                &person_id,
                &role,
            )
        })
        .await
}

/// Update a single structured field on an account.
/// Writes to SQLite, then regenerates dashboard.json + dashboard.md.
#[tauri::command]
pub async fn update_account_field(
    account_id: String,
    field: String,
    value: String,
    state: State<'_, Arc<AppState>>,
) -> Result<AccountDetailResult, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::update_account_field(
                &ctx,
                db,
                &app_state,
                &account_id,
                &field,
                &value,
            )
        })
        .await
}

/// persist a single gap-row field on
/// `account_technical_footprint` and return the refreshed account detail so
/// the frontend can render the value without a follow-up fetch.
#[tauri::command]
pub async fn update_technical_footprint_field(
    account_id: String,
    field: String,
    value: String,
    state: State<'_, Arc<AppState>>,
) -> Result<AccountDetailResult, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::update_technical_footprint_field(
                &ctx,
                db,
                &app_state,
                &account_id,
                &field,
                &value,
            )
        })
        .await
}

//: Set the user's manual health sentiment on an account,
/// optionally attaching a journal note.
#[tauri::command]
pub async fn set_user_health_sentiment(
    account_id: String,
    sentiment: String,
    note: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<AccountDetailResult, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::set_user_health_sentiment(
                &ctx,
                db,
                &app_state,
                &account_id,
                &sentiment,
                note.as_deref(),
            )
        })
        .await
}

/// Update the note on the latest sentiment journal row for an
/// account rather than inserting a new history entry. This is the
/// "Add more detail" flow — the user is augmenting the existing journal
/// entry, not creating a new sentiment change. Falls back to insertion
/// when no matching history row exists.
#[tauri::command]
pub async fn update_latest_sentiment_note(
    account_id: String,
    note: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<AccountDetailResult, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::update_latest_sentiment_note(
                &ctx,
                db,
                &app_state,
                &account_id,
                note.as_deref(),
            )
        })
        .await
}

/// Persist a triage-card snooze. `triage_key` is the frontend's
/// stable card id; `days` is the snooze window (default 14 at the call
/// site). Resolves silently — the UI refreshes after the call.
#[tauri::command]
pub async fn snooze_triage_item(
    entity_type: String,
    entity_id: String,
    triage_key: String,
    days: Option<i64>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::snooze_triage_item(
                &ctx,
                db,
                &entity_type,
                &entity_id,
                &triage_key,
                days.unwrap_or(14),
            )
        })
        .await
}

/// Mark a triage card resolved. Permanent for the lifetime of the
/// card key — re-enrichment that emits a new key will re-surface.
#[tauri::command]
pub async fn resolve_triage_item(
    entity_type: String,
    entity_id: String,
    triage_key: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::resolve_triage_item(
                &ctx,
                db,
                &app_state,
                &entity_type,
                &entity_id,
                &triage_key,
            )
        })
        .await
}

/// Return the active snooze/resolution rows for an entity so the
/// frontend can hide matching triage cards.
#[tauri::command]
pub async fn list_triage_snoozes(
    entity_type: String,
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::services::accounts::TriageSnoozeRow>, String> {
    state
        .db_read(move |db| {
            crate::services::accounts::list_triage_snoozes(db, &entity_type, &entity_id)
        })
        .await
}

/// Regression guard: Retry a failed (or re-run a prior) risk-briefing job.
/// Returns immediately; the user should refetch `get_account_detail` to see
/// the `risk_briefing_job` row progress through enqueued → running → complete.
#[tauri::command]
pub async fn retry_risk_briefing(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let ctx = app_state.live_service_context();
    crate::services::accounts::retry_risk_briefing(&ctx, &app_state, &account_id).await
}

#[tauri::command]
pub async fn confirm_lifecycle_change(
    change_id: i64,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::confirm_lifecycle_change(
                &ctx,
                db,
                &app_state.signals.engine,
                change_id,
            )
        })
        .await
}

#[tauri::command]
pub async fn correct_account_product(
    account_id: String,
    product_id: i64,
    name: String,
    status: Option<String>,
    source_to_penalize: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::correct_account_product(
                &ctx,
                db,
                &app_state.signals.engine,
                &account_id,
                product_id,
                &name,
                status.as_deref(),
                &source_to_penalize,
            )
        })
        .await
}

#[tauri::command]
pub async fn correct_lifecycle_change(
    change_id: i64,
    corrected_lifecycle: String,
    corrected_stage: Option<String>,
    notes: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::correct_lifecycle_change(
                &ctx,
                db,
                &app_state.signals.engine,
                change_id,
                &corrected_lifecycle,
                corrected_stage.as_deref(),
                notes.as_deref(),
            )
        })
        .await
}

#[tauri::command]
pub async fn accept_account_field_conflict(
    account_id: String,
    field: String,
    suggested_value: String,
    source: String,
    signal_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<AccountDetailResult, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::accept_account_field_conflict(
                &ctx,
                db,
                &app_state,
                &account_id,
                &field,
                &suggested_value,
                &source,
                signal_id.as_deref(),
            )
        })
        .await
}

#[tauri::command]
pub async fn dismiss_account_field_conflict(
    account_id: String,
    field: String,
    signal_id: String,
    source: String,
    suggested_value: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<AccountDetailResult, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::dismiss_account_field_conflict(
                &ctx,
                db,
                &app_state,
                &account_id,
                &field,
                &signal_id,
                &source,
                suggested_value.as_deref(),
            )
        })
        .await
}

/// Update account notes (narrative field — JSON only, not SQLite).
/// Writes dashboard.json + regenerates dashboard.md.
#[tauri::command]
pub async fn update_account_notes(
    account_id: String,
    notes: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::update_account_notes(
                &ctx,
                db,
                &app_state,
                &account_id,
                &notes,
            )
        })
        .await
}

/// Update account strategic programs (narrative field — JSON only).
/// Writes dashboard.json + regenerates dashboard.md.
#[tauri::command]
pub async fn update_account_programs(
    account_id: String,
    programs_json: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::update_account_programs(
                &ctx,
                db,
                &app_state,
                &account_id,
                &programs_json,
            )
        })
        .await
}

/// Create a new account. Creates SQLite record + workspace files.
/// If `parent_id` is provided, creates a child (BU) account under that parent.
/// If `account_type` is provided, uses that type; otherwise defaults to `customer`
/// (or inherits from parent for child accounts).
#[tauri::command]
pub async fn create_account(
    name: String,
    parent_id: Option<String>,
    account_type: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let acct_type = account_type.map(|s| crate::db::AccountType::from_db(&s));
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::create_account(
                &ctx,
                db,
                &app_state,
                &name,
                parent_id.as_deref(),
                acct_type,
            )
        })
        .await
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamColleagueInput {
    pub name: String,
    pub email: String,
    pub title: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInternalOrganizationResult {
    pub root_account_id: String,
    pub initial_team_id: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalTeamSetupPrefill {
    pub company: Option<String>,
    pub domains: Vec<String>,
    pub title: Option<String>,
    pub suggested_team_name: String,
    pub suggested_colleagues: Vec<TeamColleagueInput>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalTeamSetupStatus {
    pub required: bool,
    pub prefill: InternalTeamSetupPrefill,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChildAccountResult {
    pub id: String,
}

#[tauri::command]
pub async fn get_internal_team_setup_status(
    state: State<'_, Arc<AppState>>,
) -> Result<InternalTeamSetupStatus, String> {
    let config = state.config.read().clone().ok_or("Config not loaded")?;

    let suggested_team_name = if let Some(title) = config.user_title.as_deref() {
        if title.to_lowercase().contains("manager") || title.to_lowercase().contains("director") {
            "Leadership Team".to_string()
        } else {
            "Core Team".to_string()
        }
    } else {
        "Core Team".to_string()
    };

    let suggested_colleagues = state
        .db_read(|db| {
            db.get_people(Some("internal"))
                .map_err(|e| e.to_string())
                .map(|people| {
                    people
                        .into_iter()
                        .take(5)
                        .map(|p| TeamColleagueInput {
                            name: p.name,
                            email: p.email,
                            title: p.role,
                        })
                        .collect::<Vec<_>>()
                })
        })
        .await?;

    Ok(InternalTeamSetupStatus {
        required: !config.internal_team_setup_completed,
        prefill: InternalTeamSetupPrefill {
            company: config.user_company.clone(),
            domains: config.resolved_user_domains(),
            title: config.user_title.clone(),
            suggested_team_name,
            suggested_colleagues,
        },
    })
}

#[tauri::command]
pub async fn create_internal_organization(
    company_name: String,
    domains: Vec<String>,
    team_name: String,
    colleagues: Vec<TeamColleagueInput>,
    existing_person_ids: Option<Vec<String>>,
    state: State<'_, Arc<AppState>>,
) -> Result<CreateInternalOrganizationResult, String> {
    let app_state = state.inner().clone();
    let ctx = app_state.live_service_context();
    crate::services::accounts::create_internal_organization(
        &ctx,
        &app_state,
        &company_name,
        &domains,
        &team_name,
        &colleagues,
        &existing_person_ids.unwrap_or_default(),
    )
    .await
}

#[tauri::command]
pub async fn create_child_account(
    parent_id: String,
    name: String,
    description: Option<String>,
    owner_person_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<CreateChildAccountResult, String> {
    let app_state = state.inner().clone();
    let ctx = app_state.live_service_context();
    crate::services::accounts::create_child_account_cmd(
        &ctx,
        &app_state,
        &parent_id,
        &name,
        description.as_deref(),
        owner_person_id.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn create_team(
    name: String,
    description: Option<String>,
    owner_person_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<CreateChildAccountResult, String> {
    let cfg = state.config.read().clone().ok_or("Config not loaded")?;

    let root_id = if let Some(id) = cfg.internal_org_account_id {
        id
    } else {
        state
            .db_read(|db| {
                db.get_internal_root_account()
                    .map_err(|e| e.to_string())?
                    .map(|a| a.id)
                    .ok_or("No internal organization configured".to_string())
            })
            .await?
    };

    create_child_account(root_id, name, description, owner_person_id, state).await
}

#[tauri::command]
pub async fn backfill_internal_meeting_associations(
    state: State<'_, Arc<AppState>>,
) -> Result<usize, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::backfill_internal_meeting_associations(&ctx, db)
        })
        .await
}

// =============================================================================
// Person-first stakeholder commands
// =============================================================================

/// Update engagement level for a stakeholder.
#[tauri::command]
pub async fn update_stakeholder_engagement(
    account_id: String,
    person_id: String,
    engagement: String,
    state: State<'_, Arc<AppState>>,
) -> Result<AccountDetailResult, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::update_stakeholder_engagement(
                &ctx,
                db,
                &app_state,
                &account_id,
                &person_id,
                &engagement,
            )
        })
        .await
}

/// Update assessment text for a stakeholder.
#[tauri::command]
pub async fn update_stakeholder_assessment(
    account_id: String,
    person_id: String,
    assessment: String,
    state: State<'_, Arc<AppState>>,
) -> Result<AccountDetailResult, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::update_stakeholder_assessment(
                &ctx,
                db,
                &app_state,
                &account_id,
                &person_id,
                &assessment,
            )
        })
        .await
}

/// Get all stakeholder roles for a person across all their linked accounts.
#[tauri::command]
pub async fn get_person_stakeholder_roles(
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::PersonAccountRole>, String> {
    state
        .db_read(move |db| {
            db.get_person_stakeholder_roles(&person_id)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Add a role to a stakeholder (multi-role).
#[tauri::command]
pub async fn add_stakeholder_role(
    account_id: String,
    person_id: String,
    role: String,
    state: State<'_, Arc<AppState>>,
) -> Result<AccountDetailResult, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::add_stakeholder_role(
                &ctx,
                db,
                &app_state,
                &account_id,
                &person_id,
                &role,
            )
        })
        .await
}

/// Remove a specific role from a stakeholder.
#[tauri::command]
pub async fn remove_stakeholder_role(
    account_id: String,
    person_id: String,
    role: String,
    state: State<'_, Arc<AppState>>,
) -> Result<AccountDetailResult, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::remove_stakeholder_role(
                &ctx,
                db,
                &app_state,
                &account_id,
                &person_id,
                &role,
            )
        })
        .await
}

/// Get pending stakeholder suggestions for an account.
#[tauri::command]
pub async fn get_stakeholder_suggestions(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::StakeholderSuggestionRow>, String> {
    state
        .db_read(move |db| {
            db.get_stakeholder_suggestions(&account_id)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Accept a stakeholder suggestion.
#[tauri::command]
pub async fn accept_stakeholder_suggestion(
    suggestion_id: i64,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::accept_stakeholder_suggestion(
                &ctx,
                db,
                &app_state,
                suggestion_id,
            )
        })
        .await
}

/// Dismiss a stakeholder suggestion.
#[tauri::command]
pub async fn dismiss_stakeholder_suggestion(
    suggestion_id: i64,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::accounts::dismiss_stakeholder_suggestion(
                &ctx,
                db,
                &app_state.signals.engine,
                suggestion_id,
            )
        })
        .await
}

// =============================================================================
// Pending stakeholder review queue
// =============================================================================

/// Row returned by get_pending_stakeholder_suggestions.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PendingStakeholderRow {
    pub person_id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub confidence: Option<f64>,
    pub data_source: Option<String>,
    /// AC#13 multi-BU: other accounts sharing this person's email domain.
    /// Each entry is (account_id, account_name). Rendered as "Also add to X?" hints.
    pub sibling_account_hints: Vec<(String, String)>,
}

/// Get pending stakeholder suggestions (status='pending_review') for an account.
#[tauri::command]
pub async fn get_pending_stakeholder_suggestions(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<PendingStakeholderRow>, String> {
    state
        .db_read(move |db| {
            // Delegate to the entity_linking DB helper which also computes sibling hints.
            let rows = db
                .get_pending_stakeholder_suggestions(&account_id)
                .map_err(|e| e.to_string())?;
            Ok(rows
                .into_iter()
                .map(|r| PendingStakeholderRow {
                    person_id: r.person_id,
                    name: Some(r.name),
                    email: Some(r.email),
                    confidence: r.confidence,
                    data_source: Some(r.data_source),
                    sibling_account_hints: r.sibling_account_hints,
                })
                .collect())
        })
        .await
}

/// Confirm a pending_review stakeholder: promotes status to 'active'.
#[tauri::command]
pub async fn confirm_pending_stakeholder(
    account_id: String,
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let ctx = app_state.live_service_context();
    crate::services::entity_linking::confirm_stakeholder_suggestion(
        &ctx,
        app_state.clone(),
        account_id,
        person_id,
    )
    .await
}

/// Dismiss a pending_review stakeholder: sets status to 'dismissed'.
#[tauri::command]
pub async fn dismiss_pending_stakeholder(
    account_id: String,
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    let ctx = app_state.live_service_context();
    crate::services::entity_linking::dismiss_stakeholder_suggestion(
        &ctx,
        app_state.clone(),
        account_id,
        person_id,
    )
    .await
}

// =============================================================================
// Content Index
// =============================================================================

/// Get indexed files for an entity.
#[tauri::command]
pub async fn get_entity_files(
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbContentFile>, String> {
    state
        .db_read(move |db| db.get_entity_files(&entity_id).map_err(|e| e.to_string()))
        .await
}

/// Re-scan an entity's directory and return the updated file list.
///
/// Supports accounts, projects, and people. The `entity_type` parameter
/// determines which lookup and sync path to use.
#[tauri::command]
pub async fn index_entity_files(
    entity_type: String,
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbContentFile>, String> {
    let workspace_path = state
        .config
        .read()
        .as_ref()
        .ok_or("Config not loaded")?
        .workspace_path
        .clone();

    let et = entity_type.clone();
    let eid = entity_id.clone();
    let wp = workspace_path.clone();
    let files = state
        .db_write(move |db| {
            let workspace = Path::new(&wp);
            match et.as_str() {
                "account" => {
                    let account = db
                        .get_account(&eid)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("Account not found: {}", eid))?;
                    crate::accounts::sync_content_index_for_account(workspace, db, &account)?;
                }
                "project" => {
                    let project = db
                        .get_project(&eid)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("Project not found: {}", eid))?;
                    crate::projects::sync_content_index_for_project(workspace, db, &project)?;
                }
                "person" => {
                    let person = db
                        .get_person(&eid)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("Person not found: {}", eid))?;
                    let dir = if let Some(ref tp) = person.tracker_path {
                        workspace.join(tp)
                    } else {
                        crate::people::person_dir(workspace, &person.name)
                    };
                    crate::entity_io::sync_content_index_for_entity(
                        db, workspace, &person.id, "person", &dir,
                    )?;
                }
                _ => return Err(format!("Unknown entity type: {}", et)),
            }
            db.get_entity_files(&eid).map_err(|e| e.to_string())
        })
        .await?;

    state
        .embedding_queue
        .enqueue(crate::processor::embeddings::EmbeddingRequest {
            entity_id: entity_id.clone(),
            entity_type: entity_type.clone(),
            requested_at: std::time::Instant::now(),
        });
    state.integrations.embedding_queue_wake.notify_one();
    let _ = state
        .intel_queue
        .enqueue(crate::intel_queue::IntelRequest::new(
            entity_id,
            entity_type,
            crate::intel_queue::IntelPriority::ContentChange,
        ));
    state.integrations.intel_queue_wake.notify_one();

    Ok(files)
}

/// Reveal a file in macOS Finder.
///
/// Path must resolve to within the workspace directory or ~.dailyos.
#[tauri::command]
pub fn reveal_in_finder(path: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let canonical = std::fs::canonicalize(&path).map_err(|e| format!("Invalid path: {}", e))?;
    let canonical_str = canonical.to_string_lossy();

    // Allow workspace directory
    let workspace_ok = state
        .config
        .read()
        .as_ref()
        .map(|cfg| cfg.workspace_path.clone())
        .map(|wp| {
            std::fs::canonicalize(&wp)
                .map(|cwp| canonical_str.starts_with(&*cwp.to_string_lossy()))
                .unwrap_or(false)
        })
        .unwrap_or(false);

    // Allow ~/.dailyos/
    let config_ok = dirs::home_dir()
        .map(|h| {
            let config_dir = h.join(".dailyos");
            std::fs::canonicalize(&config_dir)
                .map(|cd| canonical_str.starts_with(&*cd.to_string_lossy()))
                .unwrap_or(false)
        })
        .unwrap_or(false);

    if !workspace_ok && !config_ok {
        return Err("Path is outside the workspace directory".to_string());
    }

    std::process::Command::new("open")
        .arg("-R")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open Finder: {}", e))?;
    Ok(())
}

/// Export a meeting briefing as a styled HTML file and open in the default browser.
/// The user can then Print > Save as PDF from the browser.
#[tauri::command]
pub fn export_briefing_html(meeting_id: String, markdown: String) -> Result<(), String> {
    let tmp_dir = std::env::temp_dir().join("dailyos-export");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let safe_id = meeting_id
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>();
    let filename = format!(
        "briefing-{}.html",
        if safe_id.is_empty() {
            "export"
        } else {
            &safe_id
        }
    );
    let path = tmp_dir.join(&filename);

    // Convert markdown to simple HTML
    let body_html = markdown_to_simple_html(&markdown);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Intelligence Report</title>
<style>
  @import url('https://fonts.googleapis.com/css2?family=Newsreader:ital,opsz,wght@0,6..72,200..800;1,6..72,200..800&family=DM+Sans:wght@400;500&family=JetBrains+Mono:wght@400;500&display=swap');
  body {{ font-family: 'DM Sans', sans-serif; max-width: 700px; margin: 48px auto; padding: 0 24px; color: #2a2a2a; line-height: 1.65; font-size: 15px; }}
  h1 {{ font-family: 'Newsreader', serif; font-size: 36px; font-weight: 400; letter-spacing: -0.01em; margin: 0 0 8px; }}
  h2 {{ font-family: 'Newsreader', serif; font-size: 22px; font-weight: 400; margin: 48px 0 12px; border-top: 1px solid #e0ddd8; padding-top: 16px; }}
  p {{ margin: 0 0 12px; }}
  ul, ol {{ padding-left: 20px; margin: 0 0 12px; }}
  li {{ margin-bottom: 8px; }}
  code {{ font-family: 'JetBrains Mono', monospace; font-size: 13px; background: #f5f3ef; padding: 1px 4px; border-radius: 2px; }}
  blockquote {{ border-left: 3px solid #c9a227; padding-left: 20px; margin: 16px 0; font-style: italic; color: #555; }}
  hr {{ border: none; border-top: 1px solid #e0ddd8; margin: 32px 0; }}
  .meta {{ font-family: 'JetBrains Mono', monospace; font-size: 11px; color: #888; letter-spacing: 0.04em; margin-bottom: 32px; }}
  @media print {{ body {{ margin: 24px; }} }}
</style>
</head>
<body>
<p class="meta">DAILYOS INTELLIGENCE REPORT</p>
{}
</body>
</html>"#,
        body_html
    );

    std::fs::write(&path, &html).map_err(|e| format!("Failed to write HTML: {}", e))?;

    std::process::Command::new("open")
        .arg(path.to_str().unwrap_or(""))
        .spawn()
        .map_err(|e| format!("Failed to open browser: {}", e))?;

    Ok(())
}

/// Simple markdown to HTML converter for briefing export.
fn markdown_to_simple_html(md: &str) -> String {
    let mut html = String::new();
    let mut in_list = false;
    let mut list_type = "ul";

    for line in md.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if in_list {
                html.push_str(&format!("</{}>\n", list_type));
                in_list = false;
            }
            continue;
        }

        // Headings
        if let Some(rest) = trimmed.strip_prefix("# ") {
            if in_list {
                html.push_str(&format!("</{}>\n", list_type));
                in_list = false;
            }
            html.push_str(&format!("<h1>{}</h1>\n", rest));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            if in_list {
                html.push_str(&format!("</{}>\n", list_type));
                in_list = false;
            }
            html.push_str(&format!("<h2>{}</h2>\n", rest));
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            if in_list {
                html.push_str(&format!("</{}>\n", list_type));
                in_list = false;
            }
            html.push_str(&format!("<h3>{}</h3>\n", rest));
        }
        // Unordered list
        else if let Some(rest) = trimmed.strip_prefix("- ") {
            if !in_list {
                html.push_str("<ul>\n");
                in_list = true;
                list_type = "ul";
            }
            html.push_str(&format!("<li>{}</li>\n", rest));
        }
        // Ordered list
        else if trimmed.len() > 2
            && trimmed
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            && trimmed.contains(". ")
        {
            if let Some(pos) = trimmed.find(". ") {
                if !in_list {
                    html.push_str("<ol>\n");
                    in_list = true;
                    list_type = "ol";
                }
                html.push_str(&format!("<li>{}</li>\n", &trimmed[pos + 2..]));
            }
        }
        // Horizontal rule
        else if trimmed == "---" || trimmed == "***" {
            if in_list {
                html.push_str(&format!("</{}>\n", list_type));
                in_list = false;
            }
            html.push_str("<hr>\n");
        }
        // Paragraph
        else {
            if in_list {
                html.push_str(&format!("</{}>\n", list_type));
                in_list = false;
            }
            html.push_str(&format!("<p>{}</p>\n", trimmed));
        }
    }

    if in_list {
        html.push_str(&format!("</{}>\n", list_type));
    }

    html
}

// =============================================================================
// Sprint 26: Chat Tool Commands
// =============================================================================

use crate::types::{meetings_to_json, ChatEntityListItem};

fn ensure_open_chat_session(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &crate::db::ActionDb,
    entity_id: Option<&str>,
    entity_type: Option<&str>,
) -> Result<crate::db::DbChatSession, String> {
    crate::services::mutations::ensure_open_chat_session(ctx, db, entity_id, entity_type)
}

fn append_chat_exchange(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &crate::db::ActionDb,
    session_id: &str,
    user_content: &str,
    assistant_json: &serde_json::Value,
) -> Result<(), String> {
    crate::services::mutations::append_chat_exchange(
        ctx,
        db,
        session_id,
        user_content,
        assistant_json,
    )
}

#[tauri::command]
pub async fn chat_search_content(
    entity_id: String,
    query: String,
    top_k: Option<usize>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::queries::search::ContentMatch>, String> {
    let query_str = query.trim().to_string();
    if query_str.is_empty() {
        return Ok(Vec::new());
    }

    let embedding_model = state.embedding_model.clone();
    let k = top_k.unwrap_or(10).clamp(1, 50);
    let state_for_ctx = state.inner().clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            let matches = crate::queries::search::search_entity_content(
                db,
                Some(embedding_model.as_ref()),
                &entity_id,
                &query_str,
                k,
                0.7,
                0.3,
            )?;

            let session = ensure_open_chat_session(&ctx, db, Some(&entity_id), None)?;
            let response = serde_json::json!({
                "entityId": entity_id,
                "query": query_str,
                "matches": matches,
            });
            append_chat_exchange(&ctx, db, &session.id, &query_str, &response)?;

            Ok(matches)
        })
        .await
}

#[tauri::command]
pub async fn chat_query_entity(
    entity_id: String,
    question: String,
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let question_str = question.trim().to_string();
    if question_str.is_empty() {
        return Err("question is required".to_string());
    }

    let embedding_model = state.embedding_model.clone();
    let state_for_ctx = state.inner().clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            let question = question_str.as_str();

            let (entity_type, entity_name, facts, open_actions, recent_meetings) =
                if let Some(account) = db.get_account(&entity_id).map_err(|e| e.to_string())? {
                    let meetings = db
                        .get_meetings_for_account(&entity_id, 10)
                        .map_err(|e| e.to_string())?;
                    let meetings_json = meetings_to_json(&meetings);
                    (
                        "account",
                        account.name.clone(),
                        serde_json::json!({
                            "health": account.health,
                            "lifecycle": account.lifecycle,
                            "arr": account.arr,
                            "renewal": account.contract_end,
                            "nps": account.nps,
                        }),
                        db.get_account_actions(&entity_id).unwrap_or_default(),
                        meetings_json,
                    )
                } else if let Some(project) =
                    db.get_project(&entity_id).map_err(|e| e.to_string())?
                {
                    let meetings = db
                        .get_meetings_for_project(&entity_id, 10)
                        .map_err(|e| e.to_string())?;
                    let meetings_json = meetings_to_json(&meetings);
                    (
                        "project",
                        project.name.clone(),
                        serde_json::json!({
                            "status": project.status,
                            "milestone": project.milestone,
                            "owner": project.owner,
                            "targetDate": project.target_date,
                        }),
                        db.get_project_actions(&entity_id).unwrap_or_default(),
                        meetings_json,
                    )
                } else if let Some(person) = db.get_person(&entity_id).map_err(|e| e.to_string())? {
                    let meetings = db
                        .get_person_meetings(&entity_id, 10)
                        .map_err(|e| e.to_string())?;
                    let meetings_json = meetings_to_json(&meetings);
                    (
                        "person",
                        person.name.clone(),
                        serde_json::json!({
                            "organization": person.organization,
                            "role": person.role,
                            "relationship": person.relationship,
                            "meetingCount": person.meeting_count,
                            "lastSeen": person.last_seen,
                        }),
                        Vec::new(),
                        meetings_json,
                    )
                } else {
                    return Err(format!("Entity not found: {}", entity_id));
                };

            let semantic_matches = crate::queries::search::search_entity_content(
                db,
                Some(embedding_model.as_ref()),
                &entity_id,
                question,
                8,
                0.7,
                0.3,
            )?;
            let intelligence = db.get_entity_intelligence(&entity_id).ok().flatten();

            let session = ensure_open_chat_session(&ctx, db, Some(&entity_id), Some(entity_type))?;
            let response = serde_json::json!({
                "sessionId": session.id,
                "entityId": entity_id,
                "entityType": entity_type,
                "entityName": entity_name,
                "question": question,
                "facts": facts,
                "intelligence": intelligence,
                "openActions": open_actions,
                "recentMeetings": recent_meetings,
                "semanticMatches": semantic_matches,
            });
            append_chat_exchange(&ctx, db, &session.id, question, &response)?;

            Ok(response)
        })
        .await
}

#[tauri::command]
pub async fn chat_get_briefing(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let dashboard = crate::services::dashboard::get_dashboard_data(&state).await;

    let response = match dashboard {
        DashboardResult::Success {
            data, freshness, ..
        } => serde_json::json!({
            "status": "success",
            "data": data,
            "freshness": freshness,
        }),
        DashboardResult::Empty { message, .. } => serde_json::json!({
            "status": "empty",
            "message": message,
        }),
        DashboardResult::Error { message } => serde_json::json!({
            "status": "error",
            "message": message,
        }),
    };

    let state_for_ctx = state.inner().clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            let session = ensure_open_chat_session(&ctx, db, None, None)?;
            append_chat_exchange(&ctx, db, &session.id, "get briefing", &response)?;
            Ok(response)
        })
        .await
}

#[tauri::command]
pub async fn chat_list_entities(
    entity_type: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ChatEntityListItem>, String> {
    let requested = entity_type
        .as_deref()
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "all".to_string());

    let state_for_ctx = state.inner().clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            let mut items = Vec::new();
            if requested == "all" || requested == "account" || requested == "accounts" {
                let accounts = db.get_all_accounts().map_err(|e| e.to_string())?;
                for account in accounts.into_iter().filter(|a| !a.archived) {
                    let open_action_count = db
                        .get_account_actions(&account.id)
                        .map(|a| a.len())
                        .unwrap_or(0);
                    items.push(ChatEntityListItem {
                        id: account.id,
                        name: account.name,
                        entity_type: "account".to_string(),
                        status: account.lifecycle,
                        health: account.health,
                        open_action_count,
                    });
                }
            }

            if requested == "all" || requested == "project" || requested == "projects" {
                let projects = db.get_all_projects().map_err(|e| e.to_string())?;
                for project in projects.into_iter().filter(|p| !p.archived) {
                    let open_action_count = db
                        .get_project_actions(&project.id)
                        .map(|a| a.len())
                        .unwrap_or(0);
                    items.push(ChatEntityListItem {
                        id: project.id,
                        name: project.name,
                        entity_type: "project".to_string(),
                        status: Some(project.status),
                        health: None,
                        open_action_count,
                    });
                }
            }

            let session = ensure_open_chat_session(&ctx, db, None, None)?;
            let response = serde_json::json!({
                "entityType": requested,
                "count": items.len(),
                "items": items,
            });
            append_chat_exchange(&ctx, db, &session.id, "list entities", &response)?;

            Ok(items)
        })
        .await
}

// ──: Entity Intelligence Enrichment via Claude Code ────────

/// Uses split-lock pattern  — DB lock held only briefly during gather/write.
#[tauri::command]
pub async fn enrich_account(
    app_handle: tauri::AppHandle,
    account_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::intelligence::IntelligenceJson, String> {
    let app_state = state.inner().clone();
    let ctx = app_state.live_service_context();
    crate::services::intelligence::enrich_entity(
        &ctx,
        account_id,
        "account".to_string(),
        &app_state,
        Some(&app_handle),
    )
    .await
}

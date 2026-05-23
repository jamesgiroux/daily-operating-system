//! App-facing re-export of the ability runtime `ServiceContext` surface.
//!
//! The `abilities-runtime` crate owns the public context/capability types that
//! ability code can compile against. This module adds DailyOS-only live reader
//! adapters that reach SQLite from the app crate, keeping those raw handles out
//! of the ability runtime dependency graph.

use std::{collections::HashSet, sync::Arc};

pub use abilities_runtime::services::context::*;

use crate::abilities::temporal::{
    DetectRoleChangeInput, DetectRoleChangeResult, RefreshEngagementCurveInput,
    RefreshEngagementCurveResult, TemporalMaintenanceFuture, TemporalMaintenanceHandle,
    TrajectoryQueryDepth, TrajectoryReadFuture, TrajectoryReadHandle,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionSignatureEnforcementMode {
    #[default]
    Shadow,
    Enforce,
    Disabled,
}

impl ProjectionSignatureEnforcementMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shadow => "shadow",
            Self::Enforce => "enforce",
            Self::Disabled => "disabled",
        }
    }

    pub fn parse_config(value: &str) -> Option<Self> {
        Some(match value {
            "shadow" => Self::Shadow,
            "enforce" => Self::Enforce,
            "disabled" => Self::Disabled,
            _ => return None,
        })
    }
}

pub struct LiveEntityContextReader;
pub struct LiveListOpenLoopsReader;
pub struct LiveAccountListReader;
pub struct LivePersonListReader;
pub struct LiveProjectListReader;
pub struct LiveEntityContextClaimReader;
pub struct LivePrepareMeetingContextReader;
pub struct LiveDailyReadinessContextReader;
pub struct LiveTemporalWorkspaceReader;
pub struct LiveCompositionCommitter;
/// Live adapter projecting `services::meeting_prep_status::read`
/// into the abilities-runtime crate's narrow `MeetingPrepStatusReadHandle`.
pub struct LiveMeetingPrepStatusReader;
/// Live adapter projecting `services::claim_receipt::render::
/// render_receipt_for` into the abilities-runtime crate's narrow
/// `ClaimReceiptReadHandle`. Required for the WP block runtime client to
/// invoke `claim_receipt` (the existing `render_claim_receipt` Tauri command
/// remains as the React/Tauri invocation path).
pub struct LiveClaimReceiptReader;

pub fn attach_live_workspace_readers(ctx: ServiceContext<'_>) -> ServiceContext<'_> {
    ctx.with_entity_context_reader(Arc::new(LiveEntityContextReader))
        .with_list_open_loops_reader(Arc::new(LiveListOpenLoopsReader))
        .with_account_list_reader(Arc::new(LiveAccountListReader))
        .with_person_list_reader(Arc::new(LivePersonListReader))
        .with_project_list_reader(Arc::new(LiveProjectListReader))
        .with_entity_context_claim_reader(Arc::new(LiveEntityContextClaimReader))
        .with_prepare_meeting_context_reader(Arc::new(LivePrepareMeetingContextReader))
        .with_daily_readiness_context_reader(Arc::new(LiveDailyReadinessContextReader))
        .with_trajectory_reader(Arc::new(LiveTemporalWorkspaceReader))
        .with_temporal_maintenance(Arc::new(LiveTemporalWorkspaceReader))
        .with_composition_commit_handle(Arc::new(LiveCompositionCommitter))
        .with_entity_touchpoints_reader(Arc::new(
            crate::services::entity_intelligence::touchpoints::LiveEntityTouchpointsReader,
        ))
        .with_meeting_prep_status_reader(Arc::new(LiveMeetingPrepStatusReader))
        .with_claim_receipt_reader(Arc::new(LiveClaimReceiptReader))
        .with_workspace_intake(Arc::new(
            crate::services::workspace_ingestion::workspace_intake_impl::IngestPipelineWorkspaceIntake::from_config_or_empty(),
        ))
}

impl EntityContextReadHandle for LiveEntityContextReader {
    fn read_entity_context_entries<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
    ) -> EntityContextReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                        .map_err(|error| format!("Database unavailable: {error}"))?;
                read_entity_context_entries_from_db(&db, &entity_type, &entity_id)
            })
            .await
            .map_err(|error| format!("Entity context read task failed: {error}"))?
        })
    }
}

impl ListOpenLoopsReadHandle for LiveListOpenLoopsReader {
    fn read_open_loops<'a>(&'a self, query: ListOpenLoopsQuery) -> ListOpenLoopsReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = open_action_db().map_err(ListOpenLoopsReadError::ReadFailed)?;
                let actions = load_open_loop_actions(&db, &query)?;
                let claims = actions
                    .into_iter()
                    .filter(is_open_loop_action)
                    .filter_map(|action| open_loop_claim_for_action(action, &query))
                    .collect::<Vec<_>>();
                Ok(ListOpenLoopsSnapshot { claims })
            })
            .await
            .map_err(|error| {
                ListOpenLoopsReadError::ReadFailed(format!("open loop read task failed: {error}"))
            })?
        })
    }
}

impl AccountListReadHandle for LiveAccountListReader {
    fn read_accounts<'a>(&'a self, query: AccountListQuery) -> AccountListReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = open_action_db().map_err(AccountListReadError::ReadFailed)?;
                let mut rows = db
                    .get_all_accounts()
                    .map_err(|error| AccountListReadError::ReadFailed(error.to_string()))?
                    .into_iter()
                    .map(|account| AccountListSummary {
                        account_id: account.id,
                        name: account.name,
                        status: account.lifecycle.unwrap_or_else(|| "unknown".to_string()), // MVP default until lifecycle is mandatory.
                        health_band: health_band_for_account(account.health.as_deref()),
                        last_touchpoint_at: None, // MVP default until touchpoint rollups are available here.
                        open_loops_count: 0, // MVP default until a cheap per-account aggregate exists.
                    })
                    .collect::<Vec<_>>();

                if let Some(filter) = query.status.as_deref() {
                    rows.retain(|row| row.status == filter);
                }
                if let Some(filter) = query.health_band {
                    rows.retain(|row| row.health_band == filter);
                }
                if let Some(needle) = query.name_contains.as_deref() {
                    let needle_lc = needle.to_lowercase();
                    rows.retain(|row| row.name.to_lowercase().contains(&needle_lc));
                }

                Ok(AccountListSnapshot {
                    total_after_filter: rows.len() as u64,
                    items: paginate(rows, query.offset, query.page_size),
                    data_shifted_advisory: None,
                })
            })
            .await
            .map_err(|error| {
                AccountListReadError::ReadFailed(format!("account list read task failed: {error}"))
            })?
        })
    }
}

impl PersonListReadHandle for LivePersonListReader {
    fn read_people<'a>(&'a self, query: PersonListQuery) -> PersonListReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = open_action_db().map_err(PersonListReadError::ReadFailed)?;
                let mut rows = db
                    .get_people(None)
                    .map_err(|error| PersonListReadError::ReadFailed(error.to_string()))?
                    .into_iter()
                    .map(|person| PersonListSummary {
                        person_id: person.id,
                        display_name: if person.name.trim().is_empty() {
                            person.email // MVP fallback for legacy rows without a display name.
                        } else {
                            person.name
                        },
                        primary_account_id: None, // MVP default until primary-account resolution is exposed here.
                        role: person.role.unwrap_or_else(|| "unknown".to_string()), // MVP default until role is mandatory.
                        last_touchpoint_at: person.last_seen, // MVP uses people.last_seen as the safest existing proxy.
                    })
                    .collect::<Vec<_>>();

                if let Some(filter) = query.role.as_deref() {
                    rows.retain(|row| row.role == filter);
                }
                if let Some(filter) = query.primary_account_id.as_deref() {
                    rows.retain(|row| row.primary_account_id.as_deref() == Some(filter));
                }
                if let Some(needle) = query.name_contains.as_deref() {
                    let needle_lc = needle.to_lowercase();
                    rows.retain(|row| row.display_name.to_lowercase().contains(&needle_lc));
                }

                Ok(PersonListSnapshot {
                    total_after_filter: rows.len() as u64,
                    items: paginate(rows, query.offset, query.page_size),
                    data_shifted_advisory: None,
                })
            })
            .await
            .map_err(|error| {
                PersonListReadError::ReadFailed(format!("person list read task failed: {error}"))
            })?
        })
    }
}

impl ProjectListReadHandle for LiveProjectListReader {
    fn read_projects<'a>(&'a self, query: ProjectListQuery) -> ProjectListReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = open_action_db().map_err(ProjectListReadError::ReadFailed)?;
                let mut rows = db
                    .get_all_projects()
                    .map_err(|error| ProjectListReadError::ReadFailed(error.to_string()))?
                    .into_iter()
                    .map(|project| ProjectListSummary {
                        project_id: project.id,
                        name: project.name,
                        parent_account_id: project.parent_id,
                        status: project.status,
                        trajectory:
                            abilities_runtime::abilities::list_projects::ProjectTrajectory::Unknown, // MVP default until trajectory is persisted.
                        last_touchpoint_at: None, // MVP default until touchpoint rollups are available here.
                    })
                    .collect::<Vec<_>>();

                if let Some(filter) = query.status.as_deref() {
                    rows.retain(|row| row.status == filter);
                }
                if let Some(filter) = query.trajectory {
                    rows.retain(|row| row.trajectory == filter);
                }
                if let Some(filter) = query.parent_account_id.as_deref() {
                    rows.retain(|row| row.parent_account_id.as_deref() == Some(filter));
                }
                if let Some(needle) = query.name_contains.as_deref() {
                    let needle_lc = needle.to_lowercase();
                    rows.retain(|row| row.name.to_lowercase().contains(&needle_lc));
                }

                Ok(ProjectListSnapshot {
                    total_after_filter: rows.len() as u64,
                    items: paginate(rows, query.offset, query.page_size),
                    data_shifted_advisory: None,
                })
            })
            .await
            .map_err(|error| {
                ProjectListReadError::ReadFailed(format!("project list read task failed: {error}"))
            })?
        })
    }
}

fn open_action_db() -> Result<crate::db::ActionDb, String> {
    crate::db::ActionDb::open(Arc::new(crate::db::LocalKeychain::new()))
        .map_err(|error| format!("Database unavailable: {error}"))
}

fn paginate<T>(rows: Vec<T>, offset: u64, page_size: u32) -> Vec<T>
where
    T: Clone,
{
    let offset = offset as usize;
    let end = (offset + page_size as usize).min(rows.len());
    if offset >= rows.len() {
        Vec::new()
    } else {
        rows[offset..end].to_vec()
    }
}

fn health_band_for_account(
    health: Option<&str>,
) -> abilities_runtime::abilities::trust::types::TrustBand {
    use abilities_runtime::abilities::trust::types::TrustBand;

    // The accounts table stores health as color strings — `green` / `yellow`
    // / `red` — per the existing data layer, not the narrative strings the
    // producer first guessed. Map both vocabularies so existing rows surface
    // their actual band instead of silently defaulting to LikelyCurrent.
    // Unknown / missing maps to Unscored instead of an optimistic band.
    match health.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if value == "healthy" || value == "good" || value == "green" => {
            TrustBand::LikelyCurrent
        }
        Some(value) if value == "watch" || value == "neutral" || value == "yellow" => {
            TrustBand::UseWithCaution
        }
        Some(value)
            if value == "at-risk"
                || value == "at_risk"
                || value == "critical"
                || value == "red" =>
        {
            TrustBand::NeedsVerification
        }
        None | Some(_) => TrustBand::Unscored,
    }
}

fn load_open_loop_actions(
    db: &crate::db::ActionDb,
    query: &ListOpenLoopsQuery,
) -> Result<Vec<crate::db::DbAction>, ListOpenLoopsReadError> {
    match (query.entity_type.as_deref(), query.entity_id.as_deref()) {
        (None, None) => db
            .get_due_actions(36_500)
            .map_err(|error| ListOpenLoopsReadError::ReadFailed(error.to_string())),
        (Some("account"), Some(entity_id)) => {
            let mut seen = HashSet::new();
            let mut rows = Vec::new();
            for action in db
                .get_account_actions(entity_id)
                .map_err(|error| ListOpenLoopsReadError::ReadFailed(error.to_string()))?
                .into_iter()
                .chain(
                    db.get_account_commitments(entity_id)
                        .map_err(|error| ListOpenLoopsReadError::ReadFailed(error.to_string()))?,
                )
            {
                if seen.insert(action.id.clone()) {
                    rows.push(action);
                }
            }
            Ok(rows)
        }
        (Some("person"), Some(entity_id)) => db
            .get_person_actions(entity_id)
            .map_err(|error| ListOpenLoopsReadError::ReadFailed(error.to_string())),
        (Some("project"), Some(entity_id)) => db
            .get_project_actions(entity_id)
            .map_err(|error| ListOpenLoopsReadError::ReadFailed(error.to_string())),
        (Some("meeting"), Some(entity_id)) => db
            .get_actions_for_meeting(entity_id)
            .map_err(|error| ListOpenLoopsReadError::ReadFailed(error.to_string())),
        (Some(entity_type), Some(entity_id)) => Err(ListOpenLoopsReadError::SubjectNotOwned {
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
        }),
        (entity_type, entity_id) => Err(ListOpenLoopsReadError::ReadFailed(format!(
            "incomplete open loop subject filter: entity_type={entity_type:?}, entity_id={entity_id:?}"
        ))),
    }
}

fn is_open_loop_action(action: &crate::db::DbAction) -> bool {
    matches!(
        action.status.as_str(),
        crate::action_status::BACKLOG
            | crate::action_status::UNSTARTED
            | crate::action_status::STARTED
    )
}

fn open_loop_claim_for_action(
    action: crate::db::DbAction,
    query: &ListOpenLoopsQuery,
) -> Option<abilities_runtime::types::IntelligenceClaim> {
    let (entity_type, entity_id) = open_loop_subject_for_action(&action, query)?;
    let claim_type = if action.action_kind == crate::action_status::KIND_COMMITMENT {
        abilities_runtime::ClaimType::Commitment.as_str()
    } else {
        abilities_runtime::ClaimType::OpenLoop.as_str()
    };
    let timestamp = if action.updated_at.trim().is_empty() {
        action.created_at.clone()
    } else {
        action.updated_at.clone()
    };
    let metadata = serde_json::json!({
        "loop_kind": action.action_kind,
        "status": action.status,
        "owner": action.owner_raw.or(action.waiting_on),
        "due_date": action.due_date,
        "source_label": action.source_label,
        "surface": query.surface.as_str(),
    });

    Some(abilities_runtime::types::IntelligenceClaim {
        id: action.id.clone(),
        claim_version: 1,
        subject_ref: serde_json::json!({
            "kind": entity_type,
            "id": entity_id,
        })
        .to_string(),
        claim_type: claim_type.to_string(),
        field_path: Some("open_loop".to_string()),
        topic_key: None,
        text: action.title,
        dedup_key: format!("action:{}", action.id),
        item_hash: None,
        actor: "system".to_string(),
        data_source: "local_enrichment".to_string(),
        source_ref: action.source_id,
        source_asof: Some(timestamp.clone()),
        observed_at: timestamp.clone(),
        created_at: action.created_at,
        provenance_json: "{}".to_string(),
        metadata_json: Some(metadata.to_string()),
        claim_state: abilities_runtime::types::ClaimState::Active,
        surfacing_state: abilities_runtime::types::SurfacingState::Active,
        demotion_reason: None,
        reactivated_at: None,
        retraction_reason: None,
        expires_at: None,
        superseded_by: None,
        trust_score: action.trust_score,
        trust_computed_at: None,
        trust_version: None,
        thread_id: None,
        temporal_scope: abilities_runtime::types::TemporalScope::State,
        sensitivity: abilities_runtime::types::ClaimSensitivity::Public,
        verification_state: abilities_runtime::ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    })
}

fn open_loop_subject_for_action(
    action: &crate::db::DbAction,
    query: &ListOpenLoopsQuery,
) -> Option<(String, String)> {
    if let (Some(entity_type), Some(entity_id)) =
        (query.entity_type.as_deref(), query.entity_id.as_deref())
    {
        return Some((entity_type.to_string(), entity_id.to_string()));
    }
    if let Some(project_id) = action.project_id.as_ref().filter(|value| !value.is_empty()) {
        return Some(("project".to_string(), project_id.clone()));
    }
    if let Some(account_id) = action.account_id.as_ref().filter(|value| !value.is_empty()) {
        return Some(("account".to_string(), account_id.clone()));
    }
    if let Some(person_id) = action.person_id.as_ref().filter(|value| !value.is_empty()) {
        return Some(("person".to_string(), person_id.clone()));
    }
    match (action.source_type.as_deref(), action.source_id.as_ref()) {
        (Some("transcript" | "post_meeting"), Some(meeting_id)) if !meeting_id.is_empty() => {
            Some(("meeting".to_string(), meeting_id.clone()))
        }
        _ => None,
    }
}

impl EntityContextClaimReadHandle for LiveEntityContextClaimReader {
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        surface: ClaimDismissalSurface,
        depth: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                        .map_err(|error| format!("Database unavailable: {error}"))?;
                crate::services::claims::load_entity_context_claims_active_for_surface(
                    &db,
                    &entity_type,
                    &entity_id,
                    depth,
                    surface.as_str(),
                )
                .map_err(|error| format!("Entity context claim read failed: {error}"))
            })
            .await
            .map_err(|error| format!("Entity context claim read task failed: {error}"))?
        })
    }
}

impl CompositionCommitHandle for LiveCompositionCommitter {
    fn commit_composition<'a>(
        &'a self,
        request: CompositionCommitRequest,
    ) -> CompositionCommitFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                        .map_err(|error| {
                            CompositionCommitError::Transaction(format!(
                                "Database unavailable: {error}"
                            ))
                        })?;
                let clock = SystemClock;
                let rng = SystemRng;
                let external = ExternalClients::default();
                let mut ctx = ServiceContext::new_live(&clock, &rng, &external)
                    .with_actor(request.actor.as_str());
                if let Some(ability_id) = request.ability_id.as_deref() {
                    ctx = ctx.with_ability_id(ability_id);
                }
                let proposal = crate::services::compositions::CompositionProposal {
                    composition_id: request.proposal.composition_id,
                    expected_composition_version: request.proposal.expected_composition_version,
                    composition: request.proposal.composition,
                };
                crate::services::compositions::commit_composition(&ctx, &db, proposal)
                    .map(|committed| CommittedComposition {
                        composition_id: committed.composition_id,
                        composition_version: committed.composition_version,
                        composition: committed.composition,
                    })
                    .map_err(composition_commit_error)
            })
            .await
            .map_err(|error| {
                CompositionCommitError::Transaction(format!(
                    "composition commit task failed: {error}"
                ))
            })?
        })
    }
}

fn composition_commit_error(
    error: crate::services::compositions::CompositionError,
) -> CompositionCommitError {
    match error {
        crate::services::compositions::CompositionError::EmptyCompositionId => {
            CompositionCommitError::EmptyCompositionId
        }
        crate::services::compositions::CompositionError::StaleVersion {
            composition_id,
            expected,
            current,
        } => CompositionCommitError::StaleVersion {
            composition_id,
            expected,
            current,
        },
        crate::services::compositions::CompositionError::InflatedVersion {
            composition_id,
            expected,
            current,
        } => CompositionCommitError::InflatedVersion {
            composition_id,
            expected,
            current,
        },
        crate::services::compositions::CompositionError::Overflow { composition_id } => {
            CompositionCommitError::Overflow { composition_id }
        }
        crate::services::compositions::CompositionError::Transaction(message) => {
            CompositionCommitError::Transaction(message)
        }
        crate::services::compositions::CompositionError::Mode(message) => {
            CompositionCommitError::Mode(message)
        }
    }
}

impl PrepareMeetingContextReadHandle for LivePrepareMeetingContextReader {
    fn read_prepare_meeting_context<'a>(
        &'a self,
        meeting_id: String,
    ) -> PrepareMeetingContextReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                        .map_err(|error| format!("Database unavailable: {error}"))?;
                crate::services::meetings::load_prepare_meeting_context_snapshot(&db, &meeting_id)
            })
            .await
            .map_err(|error| format!("prepare_meeting context read task failed: {error}"))?
        })
    }
}

impl DailyReadinessContextReadHandle for LiveDailyReadinessContextReader {
    fn read_daily_readiness_context<'a>(
        &'a self,
        workspace_scope: String,
        date: String,
    ) -> DailyReadinessContextReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = open_action_db()?;
                project_daily_readiness_context_snapshot(&db, &workspace_scope, &date)
            })
            .await
            .map_err(|error| format!("daily readiness context read task failed: {error}"))?
        })
    }
}

fn project_daily_readiness_context_snapshot(
    db: &crate::db::ActionDb,
    workspace_scope: &str,
    date: &str,
) -> Result<DailyReadinessContextSnapshot, String> {
    let parsed_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|error| format!("invalid daily readiness date `{date}`: {error}"))?;
    let next_date = parsed_date
        .checked_add_days(chrono::Days::new(1))
        .ok_or_else(|| format!("invalid next-day range for daily readiness date `{date}`"))?;
    let start = parsed_date.format("%Y-%m-%d").to_string();
    let end = next_date.format("%Y-%m-%d").to_string();
    let conn = db.conn_ref();
    let mut stmt = conn
        .prepare(
            "SELECT id, title, start_time, end_time
             FROM meetings
             WHERE start_time >= ?1 AND start_time < ?2
             ORDER BY start_time ASC",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![start, end], |row| {
            Ok(DailyReadinessMeetingSnapshot {
                id: row.get(0)?,
                title: row.get(1)?,
                starts_at: row.get(2)?,
                ends_at: row.get(3)?,
                workspace_scope: workspace_scope.to_string(),
            })
        })
        .map_err(|error| error.to_string())?;
    let meetings: Vec<DailyReadinessMeetingSnapshot> = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let meeting_ids = meetings
        .iter()
        .map(|meeting| meeting.id.clone())
        .collect::<Vec<_>>();
    let mut coverage_warnings = Vec::new();
    let entity_map = match db.get_linked_entities_map_for_meetings(&meeting_ids) {
        Ok(entity_map) => entity_map,
        Err(_) => {
            coverage_warnings.push(DailyReadinessCoverageWarningSnapshot {
                kind: "linked_entities_read_failed".to_string(),
                message: "Linked meeting subjects could not be read for this briefing.".to_string(),
                count: meeting_ids.len() as u32,
                workspace_scope: workspace_scope.to_string(),
            });
            Default::default()
        }
    };
    let mut seen_subjects = HashSet::new();
    let mut tracked_subjects = Vec::new();
    for linked_entities in entity_map.values() {
        for entity in linked_entities {
            let key = format!("{}:{}", entity.entity_type, entity.id);
            if !seen_subjects.insert(key) {
                continue;
            }
            tracked_subjects.push(DailyReadinessSubjectSnapshot {
                kind: entity.entity_type.clone(),
                id: entity.id.clone(),
                display_name: entity.name.clone(),
                workspace_scope: workspace_scope.to_string(),
            });
        }
    }

    Ok(DailyReadinessContextSnapshot {
        workspace_scope: workspace_scope.to_string(),
        date: date.to_string(),
        meetings,
        tracked_subjects,
        overnight_changes: Vec::new(),
        risk_shifts: Vec::new(),
        open_loops: Vec::new(),
        coverage_warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    #[test]
    fn daily_readiness_context_warns_when_linked_subjects_cannot_be_read() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::ActionDb::open_at_unencrypted(
            tempdir.path().join("daily-readiness-context.db"),
        )
        .expect("open db");
        db.conn_ref()
            .execute(
                "INSERT INTO meetings (id, title, meeting_type, start_time, end_time, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    "meeting-1",
                    "Daily Review",
                    "customer",
                    "2026-05-23T09:00:00Z",
                    "2026-05-23T09:30:00Z",
                    "2026-05-23T08:00:00Z",
                ],
            )
            .expect("insert meeting");
        db.conn_ref()
            .execute_batch("DROP VIEW IF EXISTS linked_entities;")
            .expect("drop linked_entities view");

        let snapshot = project_daily_readiness_context_snapshot(&db, "local", "2026-05-23")
            .expect("read daily readiness context");

        assert_eq!(snapshot.meetings.len(), 1);
        assert!(snapshot.tracked_subjects.is_empty());
        assert_eq!(snapshot.coverage_warnings.len(), 1);
        assert_eq!(
            snapshot.coverage_warnings[0].kind,
            "linked_entities_read_failed"
        );
        assert_eq!(
            snapshot.coverage_warnings[0].message,
            "Linked meeting subjects could not be read for this briefing."
        );
        assert_eq!(snapshot.coverage_warnings[0].count, 1);
        assert_eq!(snapshot.coverage_warnings[0].workspace_scope, "local");
    }
}

impl MeetingPrepStatusReadHandle for LiveMeetingPrepStatusReader {
    fn read_meeting_prep_status<'a>(
        &'a self,
        meeting_id: String,
    ) -> MeetingPrepStatusReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                        .map_err(|error| {
                            MeetingPrepStatusReadError::ReadFailed(format!(
                                "Database unavailable: {error}"
                            ))
                        })?;
                project_meeting_prep_status_snapshot(&db, &meeting_id)
            })
            .await
            .map_err(|error| {
                MeetingPrepStatusReadError::ReadFailed(format!(
                    "meeting_prep_status read task failed: {error}"
                ))
            })?
        })
    }
}

fn project_meeting_prep_status_snapshot(
    db: &crate::db::ActionDb,
    meeting_id: &str,
) -> Result<MeetingPrepStatusSnapshot, MeetingPrepStatusReadError> {
    use crate::services::meeting_prep_status::{read::compute_status, PrepStatusError};
    match compute_status(meeting_id, db) {
        Ok(snapshot) => Ok(MeetingPrepStatusSnapshot {
            meeting_id: snapshot.meeting_id,
            event_id: snapshot.event_id,
            linked_entity_type: snapshot
                .linked_entity
                .as_ref()
                .map(|binding| binding.entity_type.clone()),
            linked_entity_id: snapshot
                .linked_entity
                .as_ref()
                .map(|binding| binding.entity_id.clone()),
            status: prep_status_to_str(snapshot.status).to_string(),
            blocking_reason: snapshot.blocking_reason.map(blocking_reason_to_str),
            stale_reason: snapshot.stale_reason.map(stale_reason_to_str),
            last_prepared_at: snapshot.last_prepared_at,
            source_asof_inputs: snapshot
                .source_asof_inputs
                .into_iter()
                .map(|input| MeetingPrepSourceAsofRef {
                    source: input.source,
                    as_of: input.as_of,
                })
                .collect(),
        }),
        Err(PrepStatusError::MeetingNotFound(id)) => {
            Err(MeetingPrepStatusReadError::MeetingNotFound(id))
        }
        Err(other) => Err(MeetingPrepStatusReadError::ReadFailed(other.to_string())),
    }
}

fn prep_status_to_str(status: crate::services::meeting_prep_status::PrepStatus) -> &'static str {
    use crate::services::meeting_prep_status::PrepStatus::*;
    match status {
        BlockedNoEntity => "blocked_no_entity",
        PrepNeeded => "prep_needed",
        Queued => "queued",
        Running => "running",
        Ready => "ready",
        Limited => "limited",
        Stale => "stale",
        Failed => "failed",
        UserSuppressed => "user_suppressed",
        UserDismissed => "user_dismissed",
    }
}

fn blocking_reason_to_str(reason: crate::services::meeting_prep_status::BlockingReason) -> String {
    use crate::services::meeting_prep_status::BlockingReason::*;
    match reason {
        NoLinkedEntity => "no_linked_entity",
        AmbiguousAttendeeMatch => "ambiguous_attendee_match",
        SourceRevoked => "source_revoked",
        PolicyForbidden => "policy_forbidden",
    }
    .to_string()
}

fn stale_reason_to_str(reason: crate::services::meeting_prep_status::StaleReason) -> String {
    use crate::services::meeting_prep_status::StaleReason::*;
    match reason {
        EntityContextStale => "entity_context_stale",
        RecentCorrection => "recent_correction",
        SourceAsofOlderThanThreshold => "source_asof_older_than_threshold",
        ContradictedClaimUpstream => "contradicted_claim_upstream",
    }
    .to_string()
}

impl TrajectoryReadHandle for LiveTemporalWorkspaceReader {
    fn read_trajectory_bundle<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        depth: TrajectoryQueryDepth,
        computed_at: chrono::DateTime<chrono::Utc>,
    ) -> TrajectoryReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                        .map_err(|error| format!("Database unavailable: {error}"))?;
                crate::services::temporal::read_trajectory_bundle_from_db(
                    &db,
                    &entity_type,
                    &entity_id,
                    depth,
                    computed_at,
                )
            })
            .await
            .map_err(|error| format!("trajectory read task failed: {error}"))?
        })
    }
}

impl TemporalMaintenanceHandle for LiveTemporalWorkspaceReader {
    fn refresh_engagement_curve<'a>(
        &'a self,
        input: RefreshEngagementCurveInput,
        computed_at: chrono::DateTime<chrono::Utc>,
    ) -> TemporalMaintenanceFuture<'a, RefreshEngagementCurveResult> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                        .map_err(|error| format!("Database unavailable: {error}"))?;
                crate::services::temporal::refresh_engagement_curve_in_db(&db, input, computed_at)
            })
            .await
            .map_err(|error| format!("refresh_engagement_curve task failed: {error}"))?
        })
    }

    fn detect_role_change<'a>(
        &'a self,
        input: DetectRoleChangeInput,
        computed_at: chrono::DateTime<chrono::Utc>,
    ) -> TemporalMaintenanceFuture<'a, DetectRoleChangeResult> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                        .map_err(|error| format!("Database unavailable: {error}"))?;
                crate::services::temporal::detect_role_change_in_db(&db, input, computed_at)
            })
            .await
            .map_err(|error| format!("detect_role_change task failed: {error}"))?
        })
    }
}

impl EntityContextReadHandle for crate::db_service::PooledConnection {
    fn read_entity_context_entries<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
    ) -> EntityContextReadFuture<'a> {
        let reader = self.clone();
        Box::pin(async move {
            let entries = reader
                .call(move |conn| {
                    let db = crate::db::ActionDb::from_conn(conn);
                    read_entity_context_entries_from_db(db, &entity_type, &entity_id)
                        .map_err(rusqlite::Error::InvalidParameterName)
                })
                .await
                .map_err(|error| format!("DB read error: {error}"))?;
            Ok(entries)
        })
    }
}

// ─── claim_receipt — live read adapter ─────────────────────────────────────

impl ClaimReceiptReadHandle for LiveClaimReceiptReader {
    fn read_claim_receipt<'a>(
        &'a self,
        target: ClaimReceiptTarget,
        surface: ClaimReceiptSurfaceContext,
    ) -> ClaimReceiptReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || live_render_claim_receipt(target, surface))
                .await
                .map_err(|error| {
                    ClaimReceiptReadError::ReadFailed(format!(
                        "claim_receipt blocking task failed: {error}"
                    ))
                })?
        })
    }
}

/// Open a connection and dispatch through the same privacy + render-policy
/// pipeline as the Tauri `render_claim_receipt` command, then translate the
/// app crate's `ClaimReceipt` into the ability-shaped `ClaimReceiptSnapshot`.
///
/// Mirrors `services::claim_receipt::render::render_receipt_for` — the two
/// entry points share the privacy filter (`build_receipt_for_audience`) +
/// surface-policy rendered-text projection so the Tauri command and the
/// ability invocation are byte-equivalent for the same target/surface.
fn live_render_claim_receipt(
    target: ClaimReceiptTarget,
    surface: ClaimReceiptSurfaceContext,
) -> Result<ClaimReceiptSnapshot, ClaimReceiptReadError> {
    use crate::services::claim_receipt::contracts as app;
    use crate::services::claim_receipt::privacy::{build_receipt_for_audience, PrivacyError};
    use crate::services::claim_receipt::render::audience_for_surface;
    use abilities_runtime::sensitivity::{
        renderable_claim_text_with_value, RenderActor, RenderSurface,
    };

    let app_target = ability_target_to_app(&target);
    let app_surface = ability_surface_to_app(surface);

    // Proposal / WorkItem deferral matches the Tauri command's behavior.
    match &app_target {
        app::ReceiptTarget::Claim { .. } => {}
        app::ReceiptTarget::Proposal { .. } | app::ReceiptTarget::WorkItem { .. } => {
            return Err(ClaimReceiptReadError::TargetNotFound);
        }
    }

    let db = crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
        .map_err(|error| {
            ClaimReceiptReadError::ReadFailed(format!("Database unavailable: {error}"))
        })?;
    let audience = audience_for_surface(app_surface);
    let mut receipt = match build_receipt_for_audience(&app_target, audience, db.conn_ref()) {
        Ok(receipt) => receipt,
        Err(PrivacyError::ClaimNotFound(_)) => {
            return Err(ClaimReceiptReadError::TargetNotFound);
        }
        Err(PrivacyError::NonDisclosureAudience)
        | Err(PrivacyError::ComposedClaimDropped)
        | Err(PrivacyError::SurfaceDrop) => {
            return Err(ClaimReceiptReadError::PrivacyDrop);
        }
        Err(PrivacyError::Storage(message)) => {
            return Err(ClaimReceiptReadError::ReadFailed(message.to_string()));
        }
        Err(PrivacyError::InvalidMetadata(message)) => {
            return Err(ClaimReceiptReadError::ReadFailed(format!(
                "invalid metadata: {message}"
            )));
        }
    };

    receipt.surface_context = app_surface;

    // UserTauri surfaces attach the policy-resolved rendered text exactly the
    // same way `render_receipt_for` does (see render.rs for the rationale).
    if matches!(
        audience,
        crate::services::claim_receipt::privacy::Audience::UserTauri
    ) {
        if let app::ReceiptTarget::Claim { claim_id, .. } = &app_target {
            let claim_opt = crate::services::claims::load_claim_by_id(db.conn_ref(), claim_id)
                .map_err(|error| ClaimReceiptReadError::ReadFailed(error.to_string()))?;
            let claim = claim_opt.ok_or(ClaimReceiptReadError::TargetNotFound)?;
            let render_surface = match app_surface {
                app::SurfaceContext::ActionsWork => RenderSurface::Action,
                app::SurfaceContext::EntityDetail => RenderSurface::TauriEntityDetail,
                app::SurfaceContext::DailyBriefing => RenderSurface::TauriBriefingPrep,
                app::SurfaceContext::MeetingDetail => RenderSurface::TauriMeetingDetail,
                app::SurfaceContext::Mcp => RenderSurface::McpTool,
            };
            let actor = RenderActor {
                actor: "user".to_string(),
                user_id: None,
            };
            if let Some(rendered_text) =
                renderable_claim_text_with_value(&claim, &claim.text, render_surface, &actor)
            {
                receipt.rendered_text = Some(rendered_text);
            }
        }
    }

    Ok(app_receipt_to_ability(receipt))
}

fn ability_target_to_app(
    target: &ClaimReceiptTarget,
) -> crate::services::claim_receipt::contracts::ReceiptTarget {
    use crate::services::claim_receipt::contracts as app;
    match target {
        ClaimReceiptTarget::Claim {
            claim_id,
            subject,
            field_path,
        } => app::ReceiptTarget::Claim {
            claim_id: claim_id.clone(),
            subject: subject.clone(),
            field_path: field_path.clone(),
        },
        ClaimReceiptTarget::Proposal {
            proposal_id,
            subject,
            field_path,
        } => app::ReceiptTarget::Proposal {
            proposal_id: proposal_id.clone(),
            subject: subject.clone(),
            field_path: field_path.clone(),
        },
        ClaimReceiptTarget::WorkItem {
            action_id,
            backing_claim_id,
            subject,
        } => app::ReceiptTarget::WorkItem {
            action_id: action_id.clone(),
            backing_claim_id: backing_claim_id.clone(),
            subject: subject.clone(),
        },
    }
}

fn app_target_to_ability(
    target: &crate::services::claim_receipt::contracts::ReceiptTarget,
) -> ClaimReceiptTarget {
    use crate::services::claim_receipt::contracts as app;
    match target {
        app::ReceiptTarget::Claim {
            claim_id,
            subject,
            field_path,
        } => ClaimReceiptTarget::Claim {
            claim_id: claim_id.clone(),
            subject: subject.clone(),
            field_path: field_path.clone(),
        },
        app::ReceiptTarget::Proposal {
            proposal_id,
            subject,
            field_path,
        } => ClaimReceiptTarget::Proposal {
            proposal_id: proposal_id.clone(),
            subject: subject.clone(),
            field_path: field_path.clone(),
        },
        app::ReceiptTarget::WorkItem {
            action_id,
            backing_claim_id,
            subject,
        } => ClaimReceiptTarget::WorkItem {
            action_id: action_id.clone(),
            backing_claim_id: backing_claim_id.clone(),
            subject: subject.clone(),
        },
    }
}

fn ability_surface_to_app(
    surface: ClaimReceiptSurfaceContext,
) -> crate::services::claim_receipt::contracts::SurfaceContext {
    use crate::services::claim_receipt::contracts as app;
    match surface {
        ClaimReceiptSurfaceContext::ActionsWork => app::SurfaceContext::ActionsWork,
        ClaimReceiptSurfaceContext::EntityDetail => app::SurfaceContext::EntityDetail,
        ClaimReceiptSurfaceContext::DailyBriefing => app::SurfaceContext::DailyBriefing,
        ClaimReceiptSurfaceContext::MeetingDetail => app::SurfaceContext::MeetingDetail,
        ClaimReceiptSurfaceContext::Mcp => app::SurfaceContext::Mcp,
    }
}

fn app_surface_to_ability(
    surface: crate::services::claim_receipt::contracts::SurfaceContext,
) -> ClaimReceiptSurfaceContext {
    use crate::services::claim_receipt::contracts as app;
    match surface {
        app::SurfaceContext::ActionsWork => ClaimReceiptSurfaceContext::ActionsWork,
        app::SurfaceContext::EntityDetail => ClaimReceiptSurfaceContext::EntityDetail,
        app::SurfaceContext::DailyBriefing => ClaimReceiptSurfaceContext::DailyBriefing,
        app::SurfaceContext::MeetingDetail => ClaimReceiptSurfaceContext::MeetingDetail,
        app::SurfaceContext::Mcp => ClaimReceiptSurfaceContext::Mcp,
    }
}

fn app_receipt_to_ability(
    receipt: crate::services::claim_receipt::contracts::ClaimReceipt,
) -> ClaimReceiptSnapshot {
    ClaimReceiptSnapshot {
        target: app_target_to_ability(&receipt.target),
        surface_context: app_surface_to_ability(receipt.surface_context),
        rendered_text: receipt.rendered_text,
        trust: ClaimReceiptTrust {
            band: receipt.trust.band,
            source_asof: receipt.trust.source_asof,
            freshness: match receipt.trust.freshness {
                crate::services::claim_receipt::contracts::Freshness::Current => {
                    ClaimReceiptFreshness::Current
                }
                crate::services::claim_receipt::contracts::Freshness::Aging => {
                    ClaimReceiptFreshness::Aging
                }
                crate::services::claim_receipt::contracts::Freshness::Stale => {
                    ClaimReceiptFreshness::Stale
                }
                crate::services::claim_receipt::contracts::Freshness::Unknown => {
                    ClaimReceiptFreshness::Unknown
                }
            },
            caveat: receipt.trust.caveat,
            rationale: receipt.trust.rationale,
        },
        lifecycle: ClaimReceiptLifecycle {
            claim_state: receipt.lifecycle.claim_state,
            surfacing_state: receipt.lifecycle.surfacing_state,
            verification_state: receipt.lifecycle.verification_state,
            updated_at: receipt.lifecycle.updated_at,
        },
        provenance: ClaimReceiptProvenance {
            sources: receipt
                .provenance
                .sources
                .into_iter()
                .map(|source| ClaimReceiptProvenanceSource {
                    label: source.label,
                    source_type: source.source_type,
                    as_of: source.as_of,
                    href: source.href,
                    redacted: source.redacted,
                })
                .collect(),
            field_path: receipt.provenance.field_path,
            evidence_summary: receipt.provenance.evidence_summary,
            redaction: match receipt.provenance.redaction {
                crate::services::claim_receipt::contracts::RedactionLevel::None => {
                    ClaimReceiptRedactionLevel::None
                }
                crate::services::claim_receipt::contracts::RedactionLevel::Partial => {
                    ClaimReceiptRedactionLevel::Partial
                }
                crate::services::claim_receipt::contracts::RedactionLevel::Full => {
                    ClaimReceiptRedactionLevel::Full
                }
            },
        },
        actions: receipt
            .actions
            .into_iter()
            .map(|action| ClaimReceiptAction {
                action: action.action,
                label: action.label,
                disabled_reason: action.disabled_reason,
            })
            .collect(),
    }
}

pub(crate) fn read_entity_context_entries_from_db(
    db: &crate::db::ActionDb,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<crate::types::EntityContextEntry>, String> {
    let conn = db.conn_ref();
    let mut stmt = conn
        .prepare(
            "SELECT id, entity_type, entity_id, title, content, created_at, updated_at
             FROM entity_context_entries
             WHERE entity_type = ?1 AND entity_id = ?2
             ORDER BY created_at DESC",
        )
        .map_err(|error| format!("Failed to prepare entity context query: {error}"))?;

    let entries = stmt
        .query_map(rusqlite::params![entity_type, entity_id], |row| {
            Ok(crate::types::EntityContextEntry {
                id: row.get("id")?,
                entity_type: row.get("entity_type")?,
                entity_id: row.get("entity_id")?,
                title: row.get::<_, String>("title")?.into(),
                content: row.get::<_, String>("content")?.into(),
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .map_err(|error| format!("Failed to query entity context entries: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to map entity context entries: {error}"))?;

    Ok(entries)
}

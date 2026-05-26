//! App-facing re-export of the ability runtime `ServiceContext` surface.
//!
//! The `abilities-runtime` crate owns the public context/capability types that
//! ability code can compile against. This module adds DailyOS-only live reader
//! adapters that reach SQLite from the app crate, keeping those raw handles out
//! of the ability runtime dependency graph.

use std::{collections::HashSet, sync::Arc};

use abilities_runtime::abilities::recommendations::contracts as runtime_salience;

pub use abilities_runtime::services::context::*;

use crate::abilities::temporal::{
    DetectRoleChangeInput, DetectRoleChangeResult, RefreshEngagementCurveInput,
    RefreshEngagementCurveResult, TemporalMaintenanceFuture, TemporalMaintenanceHandle,
    TrajectoryQueryDepth, TrajectoryReadFuture, TrajectoryReadHandle,
};
use crate::services::recommendations::{
    contracts as app_recommendations, salience as app_salience,
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
pub struct LiveMarkdownPreviewReader;
pub struct LiveWorkspaceGraphReader;
pub struct LiveSourceManagementLedgerReader;
pub struct LiveSalienceReader;
pub struct LiveSourceManagementActionHandler {
    signal_engine: Option<Arc<crate::signals::propagation::PropagationEngine>>,
}
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
    attach_live_workspace_readers_with_signal_engine(ctx, None)
}

pub fn attach_live_workspace_readers_with_signal_engine(
    ctx: ServiceContext<'_>,
    signal_engine: Option<Arc<crate::signals::propagation::PropagationEngine>>,
) -> ServiceContext<'_> {
    ctx.with_entity_context_reader(Arc::new(LiveEntityContextReader))
        .with_list_open_loops_reader(Arc::new(LiveListOpenLoopsReader))
        .with_account_list_reader(Arc::new(LiveAccountListReader))
        .with_person_list_reader(Arc::new(LivePersonListReader))
        .with_project_list_reader(Arc::new(LiveProjectListReader))
        .with_markdown_preview_reader(Arc::new(LiveMarkdownPreviewReader))
        .with_workspace_graph_reader(Arc::new(LiveWorkspaceGraphReader))
        .with_source_management_ledger_reader(Arc::new(LiveSourceManagementLedgerReader))
        .with_salience_reader(Arc::new(LiveSalienceReader))
        .with_source_management_action_handler(Arc::new(LiveSourceManagementActionHandler {
            signal_engine: signal_engine.clone(),
        }))
        .with_entity_context_claim_reader(Arc::new(LiveEntityContextClaimReader))
        .with_prepare_meeting_context_reader(Arc::new(LivePrepareMeetingContextReader))
        .with_daily_readiness_context_reader(Arc::new(LiveDailyReadinessContextReader))
        .with_trajectory_reader(Arc::new(LiveTemporalWorkspaceReader))
        .with_temporal_maintenance(Arc::new(LiveTemporalWorkspaceReader))
        .with_composition_commit_handle(Arc::new(LiveCompositionCommitter))
        .with_entity_touchpoints_reader(Arc::new(
            crate::services::entity_intelligence::touchpoints::LiveEntityTouchpointsReader,
        ))
        .with_entity_neighborhood_reader(Arc::new(
            crate::services::entity_intelligence::neighborhood::LiveEntityNeighborhoodReader,
        ))
        .with_meeting_prep_status_reader(Arc::new(LiveMeetingPrepStatusReader))
        .with_claim_receipt_reader(Arc::new(LiveClaimReceiptReader))
        .with_workspace_intake(Arc::new(
            crate::services::workspace_ingestion::workspace_intake_impl::IngestPipelineWorkspaceIntake::from_config_or_empty_with_signal_engine(signal_engine),
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
                let db = crate::db::ActionDb::open_readonly(std::sync::Arc::new(
                    crate::db::LocalKeychain::new(),
                ))
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
                read_open_loops_from_db(&db, &query)
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

impl WorkspaceGraphReadHandle for LiveWorkspaceGraphReader {
    fn read_workspace_graph<'a>(
        &'a self,
        request: WorkspaceGraphReadRequest,
    ) -> WorkspaceGraphReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = open_action_db().map_err(WorkspaceGraphReadError::ReadFailed)?;
                let diagnostic_key =
                    crate::services::workspace_ingestion::graph::local_install_diagnostic_key()
                        .map_err(WorkspaceGraphReadError::ReadFailed)?;
                crate::services::workspace_ingestion::graph::read_workspace_graph(
                    db.conn_ref(),
                    request,
                    &diagnostic_key,
                )
            })
            .await
            .map_err(|error| {
                WorkspaceGraphReadError::ReadFailed(format!(
                    "workspace graph read task failed: {error}"
                ))
            })?
        })
    }
}

impl SourceManagementLedgerReadHandle for LiveSourceManagementLedgerReader {
    fn read_source_management_ledger<'a>(
        &'a self,
        request: SourceManagementLedgerReadRequest,
    ) -> SourceManagementLedgerReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = open_action_db().map_err(SourceManagementLedgerReadError::ReadFailed)?;
                let diagnostic_key =
                    crate::services::workspace_ingestion::graph::local_install_diagnostic_key()
                        .map_err(SourceManagementLedgerReadError::ReadFailed)?;
                crate::services::source_management_ledger::read_source_management_ledger(
                    db.conn_ref(),
                    request,
                    &diagnostic_key,
                )
            })
            .await
            .map_err(|error| {
                SourceManagementLedgerReadError::ReadFailed(format!(
                    "source management ledger read task failed: {error}"
                ))
            })?
        })
    }
}

impl SalienceReadHandle for LiveSalienceReader {
    fn score_salience<'a>(
        &'a self,
        request: runtime_salience::ScoreSalienceReadRequest,
    ) -> SalienceReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db =
                    open_action_db().map_err(runtime_salience::SalienceReadError::ReadFailed)?;
                let clock = SystemClock;
                let rng = SystemRng;
                let external = ExternalClients::default();
                let actor = match request.actor {
                    abilities_runtime::abilities::registry::ActorKind::User => {
                        "user:score_salience"
                    }
                    abilities_runtime::abilities::registry::ActorKind::System => {
                        "system:score_salience"
                    }
                    _ => "system:score_salience",
                };
                let service_ctx = ServiceContext::new_live(&clock, &rng, &external)
                    .with_actor(actor)
                    .with_ability_id(runtime_salience::SCORE_SALIENCE_ABILITY_NAME);
                let result = app_salience::score_salience(
                    &service_ctx,
                    &db,
                    app_salience::ScoreSalienceRequest {
                        schema_version: request.schema_version,
                        claim_id: app_recommendations::ClaimId(request.claim_id.0),
                    },
                )
                .map_err(salience_error_to_read_error)?;

                Ok(salience_result_to_runtime(result))
            })
            .await
            .map_err(|error| {
                runtime_salience::SalienceReadError::ReadFailed(format!(
                    "salience read task failed: {error}"
                ))
            })?
        })
    }
}

impl SourceManagementActionHandle for LiveSourceManagementActionHandler {
    fn apply_source_management_action<'a>(
        &'a self,
        request: SourceManagementActionRequest,
    ) -> SourceManagementActionFuture<'a> {
        let signal_engine = self.signal_engine.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = open_action_db().map_err(SourceManagementActionError::ActionFailed)?;
                let workspace_root = crate::state::load_config()
                    .map(|config| std::path::PathBuf::from(config.workspace_path))
                    .map_err(|error| {
                        SourceManagementActionError::ActionFailed(error.to_string())
                    })?;
                let diagnostic_key =
                    crate::services::workspace_ingestion::graph::local_install_diagnostic_key()
                        .map_err(SourceManagementActionError::ActionFailed)?;
                let clock = SystemClock;
                let rng = SystemRng;
                let external = ExternalClients::default();
                let service_ctx = ServiceContext::new_live(&clock, &rng, &external)
                    .with_actor("system:source_management_action");
                crate::services::source_management_ledger::apply_source_management_action(
                    &service_ctx,
                    &db,
                    workspace_root,
                    signal_engine,
                    request,
                    &diagnostic_key,
                )
            })
            .await
            .map_err(|error| {
                SourceManagementActionError::ActionFailed(format!(
                    "source management action task failed: {error}"
                ))
            })?
        })
    }
}

impl MarkdownPreviewReadHandle for LiveMarkdownPreviewReader {
    fn read_markdown_preview<'a>(
        &'a self,
        request: MarkdownPreviewReadRequest,
    ) -> MarkdownPreviewReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = open_action_db().map_err(MarkdownPreviewReadError::SourceUnavailable)?;
                let config = crate::state::load_config()
                    .map_err(MarkdownPreviewReadError::SourceUnavailable)?;
                let workspace_root = std::path::PathBuf::from(config.workspace_path);
                crate::services::markdown_preview::read_markdown_preview(
                    db.conn_ref(),
                    &workspace_root,
                    request,
                )
            })
            .await
            .map_err(|error| {
                MarkdownPreviewReadError::SourceUnavailable(format!(
                    "markdown preview read task failed: {error}"
                ))
            })?
        })
    }
}

fn salience_error_to_read_error(
    error: app_salience::SalienceError,
) -> runtime_salience::SalienceReadError {
    match error {
        app_salience::SalienceError::UnsupportedSchemaVersion(schema_version) => {
            runtime_salience::SalienceReadError::UnsupportedSchemaVersion(schema_version)
        }
        app_salience::SalienceError::ClaimNotFound(claim_id) => {
            runtime_salience::SalienceReadError::ClaimNotFound(claim_id)
        }
        app_salience::SalienceError::ClaimNotVisible(claim_id) => {
            runtime_salience::SalienceReadError::ClaimNotVisible(claim_id)
        }
        other => runtime_salience::SalienceReadError::ReadFailed(other.to_string()),
    }
}

fn salience_result_to_runtime(
    result: app_salience::ScoreSalienceResult,
) -> runtime_salience::ScoreSalienceResponse {
    runtime_salience::ScoreSalienceResponse {
        schema_version: result.schema_version,
        claim_id: runtime_salience::ClaimId(result.claim_id.0),
        computed_at: salience_datetime_wire(result.computed_at),
        persistence: salience_persistence_to_runtime(result.persistence),
        salience: salience_score_to_runtime(result.salience),
    }
}

fn salience_persistence_to_runtime(
    persistence: app_salience::SaliencePersistence,
) -> runtime_salience::SaliencePersistence {
    match persistence {
        app_salience::SaliencePersistence::Preview => {
            runtime_salience::SaliencePersistence::Preview
        }
        app_salience::SaliencePersistence::Stored { evaluation_id } => {
            runtime_salience::SaliencePersistence::Stored { evaluation_id }
        }
    }
}

fn salience_score_to_runtime(
    score: app_recommendations::SalienceScore,
) -> runtime_salience::SalienceScore {
    runtime_salience::SalienceScore {
        total: score.total,
        factors: score
            .factors
            .into_iter()
            .map(salience_factor_to_runtime)
            .collect(),
    }
}

fn salience_factor_to_runtime(
    factor: app_recommendations::SalienceFactor,
) -> runtime_salience::SalienceFactor {
    runtime_salience::SalienceFactor {
        kind: salience_factor_kind_to_runtime(factor.kind),
        value: factor.value,
        weight: factor.weight,
        rationale: salience_rationale_to_runtime(factor.rationale),
    }
}

fn salience_factor_kind_to_runtime(
    kind: app_recommendations::SalienceFactorKind,
) -> runtime_salience::SalienceFactorKind {
    match kind {
        app_recommendations::SalienceFactorKind::Importance => {
            runtime_salience::SalienceFactorKind::Importance
        }
        app_recommendations::SalienceFactorKind::Novelty => {
            runtime_salience::SalienceFactorKind::Novelty
        }
        app_recommendations::SalienceFactorKind::Urgency => {
            runtime_salience::SalienceFactorKind::Urgency
        }
        app_recommendations::SalienceFactorKind::Timing => {
            runtime_salience::SalienceFactorKind::Timing
        }
        app_recommendations::SalienceFactorKind::UserFit => {
            runtime_salience::SalienceFactorKind::UserFit
        }
        app_recommendations::SalienceFactorKind::Freshness => {
            runtime_salience::SalienceFactorKind::Freshness
        }
        app_recommendations::SalienceFactorKind::Trust => {
            runtime_salience::SalienceFactorKind::Trust
        }
        app_recommendations::SalienceFactorKind::Corroboration => {
            runtime_salience::SalienceFactorKind::Corroboration
        }
        app_recommendations::SalienceFactorKind::Contradiction => {
            runtime_salience::SalienceFactorKind::Contradiction
        }
        app_recommendations::SalienceFactorKind::OpenLoopRelevance => {
            runtime_salience::SalienceFactorKind::OpenLoopRelevance
        }
    }
}

fn salience_rationale_to_runtime(
    rationale: app_recommendations::FactorRationale,
) -> runtime_salience::FactorRationale {
    match rationale {
        app_recommendations::FactorRationale::Importance {
            trust_band,
            source_authority,
        } => runtime_salience::FactorRationale::Importance {
            trust_band,
            source_authority,
        },
        app_recommendations::FactorRationale::Novelty {
            vector_distance,
            neighbor_count,
        } => runtime_salience::FactorRationale::Novelty {
            vector_distance,
            neighbor_count,
        },
        app_recommendations::FactorRationale::Urgency {
            deadline,
            decay_factor,
        } => runtime_salience::FactorRationale::Urgency {
            deadline: deadline.map(salience_datetime_wire),
            decay_factor,
        },
        app_recommendations::FactorRationale::Timing {
            signal_age_secs,
            calendar_proximity_secs,
        } => runtime_salience::FactorRationale::Timing {
            signal_age_secs,
            calendar_proximity_secs,
        },
        app_recommendations::FactorRationale::UserFit {
            feedback_history_score,
        } => runtime_salience::FactorRationale::UserFit {
            feedback_history_score,
        },
        app_recommendations::FactorRationale::Freshness { decay_factor } => {
            runtime_salience::FactorRationale::Freshness { decay_factor }
        }
        app_recommendations::FactorRationale::Trust { trust_band } => {
            runtime_salience::FactorRationale::Trust { trust_band }
        }
        app_recommendations::FactorRationale::Corroboration {
            corroboration_count,
        } => runtime_salience::FactorRationale::Corroboration {
            corroboration_count,
        },
        app_recommendations::FactorRationale::Contradiction {
            contradiction_count,
        } => runtime_salience::FactorRationale::Contradiction {
            contradiction_count,
        },
        app_recommendations::FactorRationale::OpenLoopRelevance {
            open_loop_count,
            has_action,
        } => runtime_salience::FactorRationale::OpenLoopRelevance {
            open_loop_count,
            has_action,
        },
    }
}

fn salience_datetime_wire(value: chrono::DateTime<chrono::Utc>) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
        .unwrap_or_else(|| value.to_rfc3339())
}

fn open_action_db() -> Result<crate::db::ActionDb, String> {
    crate::db::ActionDb::open_readonly(Arc::new(crate::db::LocalKeychain::new()))
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

pub(crate) fn read_open_loops_from_db(
    db: &crate::db::ActionDb,
    query: &ListOpenLoopsQuery,
) -> Result<ListOpenLoopsSnapshot, ListOpenLoopsReadError> {
    let claims = load_open_loop_claims_from_substrate(db, query)?;
    Ok(ListOpenLoopsSnapshot { claims })
}

fn load_open_loop_claims_from_substrate(
    db: &crate::db::ActionDb,
    query: &ListOpenLoopsQuery,
) -> Result<Vec<abilities_runtime::types::IntelligenceClaim>, ListOpenLoopsReadError> {
    const OPEN_LOOP_TYPES: &[&str] = &["open_loop", "commitment"];
    const OPEN_LOOP_LIMIT: usize = 500;

    let claims = match (query.entity_type.as_deref(), query.entity_id.as_deref()) {
        (None, None) => {
            crate::services::claims::load_prompt_claims_by_types_active_for_surface_limited(
                db,
                OPEN_LOOP_TYPES,
                query.surface.as_str(),
                OPEN_LOOP_LIMIT,
            )
            .map_err(|error| ListOpenLoopsReadError::ReadFailed(error.to_string()))?
        }
        (Some(entity_type), Some(entity_id)) => {
            let entity_type = entity_type.trim();
            if !matches!(entity_type, "account" | "project" | "person" | "meeting") {
                return Err(ListOpenLoopsReadError::SubjectNotOwned {
                    entity_type: entity_type.to_string(),
                    entity_id: entity_id.to_string(),
                });
            }
            crate::services::claims::load_entity_context_prompt_claims_active_for_surface_limited(
                db,
                entity_type,
                entity_id,
                1,
                query.surface.as_str(),
                OPEN_LOOP_LIMIT,
            )
            .map_err(|error| ListOpenLoopsReadError::ReadFailed(error.to_string()))?
            .into_iter()
            .filter(|claim| OPEN_LOOP_TYPES.contains(&claim.claim_type.as_str()))
            .collect()
        }
        (entity_type, entity_id) => {
            return Err(ListOpenLoopsReadError::ReadFailed(format!(
                "incomplete open loop subject filter: entity_type={entity_type:?}, entity_id={entity_id:?}"
            )));
        }
    };

    Ok(claims)
}

#[cfg(test)]
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
        "source_type": action.source_type,
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
        sensitivity: abilities_runtime::types::ClaimSensitivity::Internal,
        verification_state: abilities_runtime::ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    })
}

#[cfg(test)]
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
                let db = crate::db::ActionDb::open_readonly(std::sync::Arc::new(
                    crate::db::LocalKeychain::new(),
                ))
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

    fn read_entity_context_claims_limited<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        surface: ClaimDismissalSurface,
        depth: usize,
        limit: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = crate::db::ActionDb::open_readonly(std::sync::Arc::new(
                    crate::db::LocalKeychain::new(),
                ))
                .map_err(|error| format!("Database unavailable: {error}"))?;
                crate::services::claims::load_entity_context_claims_active_for_surface_limited(
                    &db,
                    &entity_type,
                    &entity_id,
                    depth,
                    surface.as_str(),
                    limit,
                )
                .map_err(|error| format!("Entity context claim read failed: {error}"))
            })
            .await
            .map_err(|error| format!("Entity context claim read task failed: {error}"))?
        })
    }

    fn read_entity_context_prompt_claims_limited<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        surface: ClaimDismissalSurface,
        depth: usize,
        limit: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = crate::db::ActionDb::open_readonly(std::sync::Arc::new(
                    crate::db::LocalKeychain::new(),
                ))
                .map_err(|error| format!("Database unavailable: {error}"))?;
                crate::services::claims::load_entity_context_prompt_claims_active_for_surface_limited(
                    &db,
                    &entity_type,
                    &entity_id,
                    depth,
                    surface.as_str(),
                    limit,
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
                let db = crate::db::ActionDb::open_readonly(std::sync::Arc::new(
                    crate::db::LocalKeychain::new(),
                ))
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
        intent: MeetingsViewIntent,
    ) -> DailyReadinessContextReadFuture<'a> {
        // Resolve the user's local-day boundaries in their configured TZ. Without
        // this the SQL query below would naively compare UTC-stored start_time
        // against bare date strings, so meetings between local-midnight and
        // UTC-midnight (e.g. an evening call on PDT yesterday stored as today
        // UTC) would leak into "today" and cross-day meetings on the user's
        // actual today would silently drop. Defaults match `dashboard.rs`.
        let tz: chrono_tz::Tz = crate::state::load_config()
            .ok()
            .map(|c| c.schedules.today.timezone)
            .and_then(|t| t.parse().ok())
            .unwrap_or(chrono_tz::America::New_York);
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = open_action_db()?;
                project_daily_readiness_context_snapshot(&db, &workspace_scope, &date, &tz, intent)
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
    tz: &chrono_tz::Tz,
    intent: MeetingsViewIntent,
) -> Result<DailyReadinessContextSnapshot, String> {
    let parsed_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|error| format!("invalid daily readiness date `{date}`: {error}"))?;
    // Meetings projection is service-owned (see services/meetings_view.rs).
    // TZ-aware window resolution, the transcript-archive JOIN, and the
    // per-intent type filter all live there so future consumers (dashboard,
    // executive intelligence) can share the same policy without re-deriving it.
    let meetings = crate::services::meetings_view::read_surface_meetings(
        db,
        workspace_scope,
        parsed_date,
        tz,
        intent,
    )?;
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
    use chrono::TimeZone;
    use rusqlite::params;

    fn fixture_action() -> crate::db::DbAction {
        crate::db::DbAction {
            id: "action-1".to_string(),
            title: "Follow up on renewal risk".to_string(),
            priority: 1,
            status: crate::action_status::UNSTARTED.to_string(),
            created_at: "2026-05-23T08:00:00Z".to_string(),
            due_date: None,
            completed_at: None,
            account_id: Some("acct-1".to_string()),
            project_id: None,
            source_type: Some("transcript".to_string()),
            source_id: Some("meeting-1".to_string()),
            source_label: Some("meeting".to_string()),
            action_kind: crate::action_status::KIND_TASK.to_string(),
            commitment_id: None,
            owner_raw: Some("Alex".to_string()),
            owner_entity_id: None,
            owner_confidence: None,
            owner_source: None,
            trust_score: Some(0.8),
            trust_band: Some("likely_current".to_string()),
            commitment_source_count: Some(1),
            context: None,
            waiting_on: None,
            updated_at: "2026-05-23T08:00:00Z".to_string(),
            person_id: None,
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
            needs_decision: false,
            decision_owner: None,
            decision_stakes: None,
            linear_identifier: None,
            linear_url: None,
        }
    }

    #[test]
    fn action_open_loop_synthesis_is_mcp_visible_as_internal_runtime_evidence() {
        let action = fixture_action();
        let mcp_query = ListOpenLoopsQuery {
            entity_type: Some("account".to_string()),
            entity_id: Some("acct-1".to_string()),
            surface: ClaimDismissalSurface::McpTool,
        };
        let mcp_claim = open_loop_claim_for_action(action.clone(), &mcp_query)
            .expect("action rows should become bounded MCP runtime evidence");
        assert_eq!(
            mcp_claim.sensitivity,
            abilities_runtime::types::ClaimSensitivity::Internal,
            "local MCP runs under the first-party OS/keychain boundary, so action evidence is internal"
        );
        let mcp_detail_query = ListOpenLoopsQuery {
            entity_type: Some("account".to_string()),
            entity_id: Some("acct-1".to_string()),
            surface: ClaimDismissalSurface::McpToolDetail,
        };
        let mcp_detail_claim = open_loop_claim_for_action(action.clone(), &mcp_detail_query)
            .expect("MCP detail should expose the same bounded runtime evidence");
        assert_eq!(
            mcp_detail_claim.sensitivity,
            abilities_runtime::types::ClaimSensitivity::Internal
        );

        let tauri_query = ListOpenLoopsQuery {
            entity_type: Some("account".to_string()),
            entity_id: Some("acct-1".to_string()),
            surface: ClaimDismissalSurface::TauriEntityDetail,
        };
        let tauri_claim = open_loop_claim_for_action(action, &tauri_query)
            .expect("first-party Tauri surfaces can still render local action open loops");
        assert_eq!(
            tauri_claim.sensitivity,
            abilities_runtime::types::ClaimSensitivity::Internal
        );
    }

    #[test]
    fn open_loop_reader_reads_claim_backed_action_evidence_not_raw_actions() {
        let db = crate::db::ActionDb::from_connection_for_tests(
            crate::migrations::migrated_in_memory_for_tests(),
        );
        db.upsert_account(&crate::db::DbAccount {
            id: "acct-1".to_string(),
            name: "Example Account".to_string(),
            account_type: crate::db::AccountType::Customer,
            updated_at: "2026-05-20T00:00:00Z".to_string(),
            ..Default::default()
        })
        .expect("seed account");
        let action = fixture_action();
        db.upsert_action(&action).expect("seed action");
        let query = ListOpenLoopsQuery {
            entity_type: Some("account".to_string()),
            entity_id: Some("acct-1".to_string()),
            surface: ClaimDismissalSurface::McpTool,
        };

        let before = read_open_loops_from_db(&db, &query).expect("read open loops before sync");
        assert!(
            before.claims.is_empty(),
            "open-loop reader must not synthesize raw action rows when no claim exists"
        );

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(44);
        let ext = ExternalClients::default();
        let ctx = ServiceContext::test_live(&clock, &rng, &ext);
        crate::services::action_claims::sync_action_open_loop_claim(&ctx, &db, &action)
            .expect("sync action claim");

        let after = read_open_loops_from_db(&db, &query).expect("read open loops after sync");
        assert_eq!(after.claims.len(), 1);
        assert_eq!(after.claims[0].claim_type, "open_loop");
        assert_eq!(
            after.claims[0].field_path.as_deref(),
            Some("actions.action-1")
        );
    }

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

        let tz: chrono_tz::Tz = "UTC".parse().expect("parse UTC tz");
        let snapshot = project_daily_readiness_context_snapshot(
            &db,
            "local",
            "2026-05-23",
            &tz,
            MeetingsViewIntent::Briefing,
        )
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

    /// DOS-771 regression: a calendar day with N personal blocks and zero
    /// customer meetings must yield zero rows under `Briefing` intent, so the
    /// briefing producer doesn't emit phantom "needs prep" / "link N meetings"
    /// advisories. `AllRows` keeps the rows for callers that want the raw set.
    #[test]
    fn personal_blocks_excluded_under_briefing_intent() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::ActionDb::open_at_unencrypted(
            tempdir.path().join("personal-block-projection.db"),
        )
        .expect("open db");

        // 4 personal blocks (matches the DOS-771 phantom-row shape) + 1 customer
        // meeting. The customer meeting is the only row Briefing should return.
        let rows = [
            ("meet-personal-1", "Lunch", "personal", "2026-05-23T12:00:00Z"),
            ("meet-personal-2", "Gym", "personal", "2026-05-23T07:00:00Z"),
            (
                "meet-personal-3",
                "School pickup",
                "personal",
                "2026-05-23T15:00:00Z",
            ),
            (
                "meet-personal-4",
                "Doctor",
                "personal",
                "2026-05-23T16:00:00Z",
            ),
            (
                "meet-customer-1",
                "Customer sync",
                "customer",
                "2026-05-23T10:00:00Z",
            ),
        ];
        for (id, title, meeting_type, start) in rows {
            db.conn_ref()
                .execute(
                    "INSERT INTO meetings (id, title, meeting_type, start_time, end_time, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        id,
                        title,
                        meeting_type,
                        start,
                        "2026-05-23T23:59:59Z",
                        "2026-05-23T00:00:00Z",
                    ],
                )
                .expect("insert meeting");
        }

        let tz: chrono_tz::Tz = "UTC".parse().expect("parse UTC tz");
        let date = chrono::NaiveDate::from_ymd_opt(2026, 5, 23).expect("date");

        let briefing = crate::services::meetings_view::read_surface_meetings(
            &db,
            "local",
            date,
            &tz,
            MeetingsViewIntent::Briefing,
        )
        .expect("briefing projection");
        assert_eq!(
            briefing.len(),
            1,
            "Briefing intent must exclude personal blocks; got {briefing:?}"
        );
        assert_eq!(briefing[0].id, "meet-customer-1");

        let all_rows = crate::services::meetings_view::read_surface_meetings(
            &db,
            "local",
            date,
            &tz,
            MeetingsViewIntent::AllRows,
        )
        .expect("all-rows projection");
        assert_eq!(
            all_rows.len(),
            5,
            "AllRows intent must include personal blocks alongside customer rows"
        );
    }

    #[test]
    fn salience_runtime_adapter_preserves_app_dto_wire_shape() {
        use abilities_runtime::abilities::trust::types::TrustBand;
        use app_recommendations::{FactorRationale, SalienceFactor, SalienceFactorKind};

        let result = app_salience::ScoreSalienceResult {
            schema_version: app_salience::SCORE_SALIENCE_SCHEMA_VERSION,
            claim_id: app_recommendations::ClaimId("claim-1".to_string()),
            computed_at: chrono::Utc.with_ymd_and_hms(2026, 5, 26, 12, 0, 0).unwrap(),
            persistence: app_salience::SaliencePersistence::Stored {
                evaluation_id: "salience-eval-1".to_string(),
            },
            salience: app_recommendations::SalienceScore {
                total: 0.72,
                factors: vec![
                    SalienceFactor {
                        kind: SalienceFactorKind::Importance,
                        value: Some(0.75),
                        weight: 0.2,
                        rationale: FactorRationale::Importance {
                            trust_band: TrustBand::LikelyCurrent,
                            source_authority: 0.8,
                        },
                    },
                    SalienceFactor {
                        kind: SalienceFactorKind::Novelty,
                        value: Some(0.5),
                        weight: 0.1,
                        rationale: FactorRationale::Novelty {
                            vector_distance: 0.5,
                            neighbor_count: 1,
                        },
                    },
                    SalienceFactor {
                        kind: SalienceFactorKind::Urgency,
                        value: Some(0.85),
                        weight: 0.15,
                        rationale: FactorRationale::Urgency {
                            deadline: Some(
                                chrono::Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
                            ),
                            decay_factor: 0.85,
                        },
                    },
                    SalienceFactor {
                        kind: SalienceFactorKind::Timing,
                        value: Some(0.7),
                        weight: 0.1,
                        rationale: FactorRationale::Timing {
                            signal_age_secs: 3600,
                            calendar_proximity_secs: None,
                        },
                    },
                    SalienceFactor {
                        kind: SalienceFactorKind::UserFit,
                        value: Some(0.9),
                        weight: 0.1,
                        rationale: FactorRationale::UserFit {
                            feedback_history_score: 0.8,
                        },
                    },
                    SalienceFactor {
                        kind: SalienceFactorKind::Freshness,
                        value: Some(0.95),
                        weight: 0.1,
                        rationale: FactorRationale::Freshness { decay_factor: 0.95 },
                    },
                    SalienceFactor {
                        kind: SalienceFactorKind::Trust,
                        value: Some(0.9),
                        weight: 0.1,
                        rationale: FactorRationale::Trust {
                            trust_band: TrustBand::LikelyCurrent,
                        },
                    },
                    SalienceFactor {
                        kind: SalienceFactorKind::Corroboration,
                        value: Some(0.25),
                        weight: 0.05,
                        rationale: FactorRationale::Corroboration {
                            corroboration_count: 2,
                        },
                    },
                    SalienceFactor {
                        kind: SalienceFactorKind::Contradiction,
                        value: Some(1.0),
                        weight: 0.05,
                        rationale: FactorRationale::Contradiction {
                            contradiction_count: 0,
                        },
                    },
                    SalienceFactor {
                        kind: SalienceFactorKind::OpenLoopRelevance,
                        value: Some(0.75),
                        weight: 0.05,
                        rationale: FactorRationale::OpenLoopRelevance {
                            open_loop_count: 1,
                            has_action: true,
                        },
                    },
                ],
            },
        };

        let app_json = serde_json::to_value(&result).expect("serialize app salience result");
        let runtime_json = serde_json::to_value(salience_result_to_runtime(result))
            .expect("serialize runtime salience result");

        assert_eq!(runtime_json, app_json);
    }
}

impl MeetingPrepStatusReadHandle for LiveMeetingPrepStatusReader {
    fn read_meeting_prep_status<'a>(
        &'a self,
        meeting_id: String,
    ) -> MeetingPrepStatusReadFuture<'a> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let db = crate::db::ActionDb::open_readonly(std::sync::Arc::new(
                    crate::db::LocalKeychain::new(),
                ))
                .map_err(|error| {
                    MeetingPrepStatusReadError::ReadFailed(format!("Database unavailable: {error}"))
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
                let db = crate::db::ActionDb::open_readonly(std::sync::Arc::new(
                    crate::db::LocalKeychain::new(),
                ))
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

    let db =
        crate::db::ActionDb::open_readonly(std::sync::Arc::new(crate::db::LocalKeychain::new()))
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

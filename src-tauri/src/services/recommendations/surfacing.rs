//! Surfacing policy.
//!
//! Decides which scored candidates surface to the user (tiered),
//! which defer, which suppress, and emits the
//! `SurfacingDecisionMade` audit signal for each decision. Enforces
//! the daily surfacing budget. No provider / LLM call from this
//! module.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use abilities_runtime::sensitivity::ClaimDismissalSurface;

use super::contracts::{
    ClaimId, DeferReason, RecommendationMetadataEnvelope, RecommendedAction, SalienceFactorKind,
    SalienceScore, SuppressReason, SurfacingDecision, SurfacingTier, TriggerRef,
};
use super::recommendation::action_key;
use super::salience::{
    recompute_salience_for_claim, score_salience, SalienceError, SaliencePersistence,
    ScoreSalienceRequest, SCORE_SALIENCE_SCHEMA_VERSION,
};
use super::why_this_now::why_this_now_for_score;
use crate::db::ActionDb;
use crate::services::context::{ExecutionMode, ServiceContext, ServiceError};
use crate::signals::propagation::PropagationEngine;

pub const SURFACING_POLICY_VERSION: &str = "recommendation_surfacing_v1";
pub const SURFACING_EVALUATION_SCHEMA_VERSION: u32 = 1;
pub const SURFACING_DECISION_SIGNAL: &str = "surfacing_decision_made";
const SURFACING_SIGNAL_SOURCE: &str = "recommendation_surfacing_policy";
const CLAIM_TYPE_RECOMMENDATION: &str = "recommendation";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceClass {
    Primary,
    Background,
    Quiet,
    Review,
}

impl SurfaceClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Background => "background",
            Self::Quiet => "quiet",
            Self::Review => "review",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfacingEvaluationInput {
    pub schema_version: u32,
    pub claim_id: ClaimId,
    pub render_surface: ClaimDismissalSurface,
    pub surface_class: SurfaceClass,
    pub trigger_refs: Vec<TriggerRef>,
    pub source_signal_id: Option<String>,
    pub trigger_source_asof: Option<DateTime<Utc>>,
    pub evidence_signature_changed: bool,
    pub subject_version_changed: bool,
}

impl SurfacingEvaluationInput {
    pub fn primary(claim_id: ClaimId, render_surface: ClaimDismissalSurface) -> Self {
        Self {
            schema_version: SURFACING_EVALUATION_SCHEMA_VERSION,
            claim_id,
            render_surface,
            surface_class: SurfaceClass::Primary,
            trigger_refs: Vec::new(),
            source_signal_id: None,
            trigger_source_asof: None,
            evidence_signature_changed: false,
            subject_version_changed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfacingSignalOutcome {
    pub signal_id: String,
    pub signal_coalesced: bool,
    pub derived_signal_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfacingEvaluation {
    pub schema_version: u32,
    pub policy_version: String,
    pub claim_id: ClaimId,
    pub salience_evaluation_id: Option<String>,
    pub surfacing_decision_id: Option<String>,
    pub budget_key: String,
    pub decision: SurfacingDecision,
    pub signal: Option<SurfacingSignalOutcome>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfacingPolicy {
    pub policy_version: String,
    pub critical_threshold: f64,
    pub urgent_factor_threshold: f64,
    pub critical_primary_daily_budget: u32,
    pub notable_threshold: f64,
    pub notable_primary_daily_budget: u32,
    pub background_threshold: f64,
    pub background_daily_budget: u32,
    pub claim_cooldown_days: i64,
    pub subject_action_cooldown_days: i64,
    pub feedback_suppression_days: i64,
}

impl Default for SurfacingPolicy {
    fn default() -> Self {
        Self {
            policy_version: SURFACING_POLICY_VERSION.to_string(),
            critical_threshold: 0.85,
            urgent_factor_threshold: 0.90,
            critical_primary_daily_budget: 1,
            notable_threshold: 0.68,
            notable_primary_daily_budget: 3,
            background_threshold: 0.45,
            background_daily_budget: 10,
            claim_cooldown_days: 7,
            subject_action_cooldown_days: 3,
            feedback_suppression_days: 14,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfacingCandidate {
    pub claim_id: ClaimId,
    pub claim_type: String,
    pub sensitivity: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub action_signature: String,
    pub source_asof: Option<DateTime<Utc>>,
    pub evidence_signature: Option<String>,
    pub salience: SalienceScore,
    pub trigger_refs: Vec<TriggerRef>,
    pub surface_class: SurfaceClass,
    pub material_new_evidence: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SurfacingPolicyState {
    pub claim_dismissed_recently: bool,
    pub feedback_suppressed_recently: bool,
    pub claim_cooldown_active: bool,
    pub claim_cooldown_until: Option<DateTime<Utc>>,
    pub subject_action_cooldown_active: bool,
    pub subject_action_cooldown_until: Option<DateTime<Utc>>,
    pub critical_primary_used: u32,
    pub notable_primary_used: u32,
    pub background_used: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum SurfacingError {
    #[error("unsupported schema_version `{0}` for evaluate_surfacing_for_claim")]
    UnsupportedSchemaVersion(u32),
    #[error("claim `{0}` not found")]
    ClaimNotFound(String),
    #[error("claim `{0}` is not a recommendation claim")]
    NotRecommendationClaim(String),
    #[error("claim `{0}` is not visible to surfacing policy")]
    ClaimNotVisible(String),
    #[error("salience recompute did not persist an evaluation for claim `{0}`")]
    MissingStoredSalience(String),
    #[error("surfacing database error: {0}")]
    Database(String),
    #[error(transparent)]
    Service(#[from] ServiceError),
    #[error(transparent)]
    Salience(#[from] SalienceError),
    #[error("surfacing signal emit failed: {0}")]
    Signal(String),
    #[error("surfacing serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<rusqlite::Error> for SurfacingError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error.to_string())
    }
}

#[derive(Debug, Clone)]
struct ClaimRow {
    id: ClaimId,
    subject_ref: String,
    claim_type: String,
    source_asof: Option<String>,
    metadata_json: Option<String>,
    claim_state: String,
    surfacing_state: String,
    sensitivity: String,
}

#[derive(Debug, Clone)]
struct LoadedClaim {
    row: ClaimRow,
    subject_kind: String,
    subject_id: String,
    action_signature: String,
    evidence_signature: Option<String>,
}

pub fn decide_surfacing(
    candidate: SurfacingCandidate,
    policy: &SurfacingPolicy,
    state: SurfacingPolicyState,
    now: DateTime<Utc>,
) -> SurfacingDecision {
    if (state.claim_dismissed_recently || state.feedback_suppressed_recently)
        && !candidate.material_new_evidence
    {
        return SurfacingDecision::Suppress {
            reason: SuppressReason::DismissedRecently,
        };
    }

    if (state.claim_cooldown_active || state.subject_action_cooldown_active)
        && !candidate.material_new_evidence
    {
        let until = max_datetime(
            state.claim_cooldown_until,
            state.subject_action_cooldown_until,
        )
        .unwrap_or_else(|| now + Duration::days(policy.claim_cooldown_days));
        return SurfacingDecision::Defer {
            until,
            reason: DeferReason::CooldownActive,
        };
    }

    let urgency = factor_value(&candidate.salience, SalienceFactorKind::Urgency);
    let tier = if candidate.salience.total >= policy.critical_threshold
        || urgency >= policy.urgent_factor_threshold
    {
        Some(SurfacingTier::Critical)
    } else if candidate.salience.total >= policy.notable_threshold {
        Some(SurfacingTier::Notable)
    } else if candidate.salience.total >= policy.background_threshold {
        Some(SurfacingTier::Background)
    } else {
        None
    };

    let Some(tier) = tier else {
        return SurfacingDecision::Suppress {
            reason: SuppressReason::BelowThreshold,
        };
    };

    if candidate.surface_class == SurfaceClass::Background
        && state.background_used >= policy.background_daily_budget
    {
        return SurfacingDecision::Defer {
            until: next_local_day(now),
            reason: DeferReason::BudgetExhausted,
        };
    }

    match tier {
        SurfacingTier::Critical if candidate.surface_class == SurfaceClass::Primary => {
            if state.critical_primary_used >= policy.critical_primary_daily_budget {
                return SurfacingDecision::Defer {
                    until: next_local_day(now),
                    reason: DeferReason::BudgetExhausted,
                };
            }
        }
        SurfacingTier::Notable if candidate.surface_class == SurfaceClass::Primary => {
            if state.notable_primary_used >= policy.notable_primary_daily_budget {
                return SurfacingDecision::Defer {
                    until: next_local_day(now),
                    reason: DeferReason::BudgetExhausted,
                };
            }
        }
        SurfacingTier::Background => {
            if candidate.surface_class == SurfaceClass::Primary {
                return SurfacingDecision::Defer {
                    until: now + Duration::hours(6),
                    reason: DeferReason::PendingTrigger,
                };
            }
        }
        SurfacingTier::Quiet | SurfacingTier::Critical | SurfacingTier::Notable => {}
    }

    SurfacingDecision::Render {
        tier,
        why_this_now: why_this_now_for_score(&candidate.salience, &candidate.trigger_refs, now),
    }
}

pub fn evaluate_surfacing_for_claim(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    input: SurfacingEvaluationInput,
) -> Result<SurfacingEvaluation, SurfacingError> {
    validate_schema_version(input.schema_version)?;
    let input = sanitize_surfacing_input(input);

    if !matches!(ctx.mode, ExecutionMode::Live) {
        let claim = load_recommendation_claim(db.conn_ref(), &input.claim_id)?;
        let policy = load_policy(db.conn_ref())?;
        let salience = score_salience(
            ctx,
            db,
            ScoreSalienceRequest {
                schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
                claim_id: input.claim_id.clone(),
            },
        )?;
        let budget_key = budget_key(
            ctx,
            &claim.row.claim_type,
            &claim.row.sensitivity,
            input.surface_class,
            ctx.clock.now().date_naive(),
        );
        let candidate = candidate_from_parts(
            &claim,
            salience.salience,
            input.surface_class,
            input.trigger_refs,
            false,
        );
        return Ok(SurfacingEvaluation {
            schema_version: SURFACING_EVALUATION_SCHEMA_VERSION,
            policy_version: policy.policy_version.clone(),
            claim_id: input.claim_id,
            salience_evaluation_id: None,
            surfacing_decision_id: None,
            budget_key,
            decision: decide_surfacing(
                candidate,
                &policy,
                SurfacingPolicyState::default(),
                ctx.clock.now(),
            ),
            signal: None,
        });
    }

    ctx.check_mutation_allowed()?;
    let now = ctx.clock.now();
    let claim = load_recommendation_claim(db.conn_ref(), &input.claim_id)?;
    let policy = load_policy(db.conn_ref())?;
    let salience = recompute_salience_for_claim(
        ctx,
        db,
        ScoreSalienceRequest {
            schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
            claim_id: input.claim_id.clone(),
        },
    )?;
    let SaliencePersistence::Stored {
        evaluation_id: salience_evaluation_id,
    } = salience.persistence.clone()
    else {
        return Err(SurfacingError::MissingStoredSalience(input.claim_id.0));
    };

    let local_day = now.date_naive();
    let budget_key = budget_key(
        ctx,
        &claim.row.claim_type,
        &claim.row.sensitivity,
        input.surface_class,
        local_day,
    );
    let decision_id = format!("surfacing-decision-{}", uuid::Uuid::new_v4());
    let idempotency_key = format!(
        "surfacing:{policy_version}:{claim_id}:{salience_evaluation_id}:{surface}",
        policy_version = policy.policy_version,
        claim_id = claim.row.id.0,
        surface = input.render_surface.as_str(),
    );
    let actor_kind = actor_kind(ctx);

    let (decision, signal_outcome, derived_signal_ids) = db
        .with_transaction(|tx| {
            let latest_suppression_at =
                latest_suppression_at(tx.conn_ref(), &claim, &input, now, &policy)
                    .map_err(|error| error.to_string())?;
            let latest_render_at = max_datetime(
                latest_render_at(tx.conn_ref(), &claim.row.id)
                    .map_err(|error| error.to_string())?,
                latest_subject_action_render_at(
                    tx.conn_ref(),
                    &claim.subject_kind,
                    &claim.subject_id,
                    &claim.action_signature,
                )
                .map_err(|error| error.to_string())?,
            );
            let material_change_key_fresh =
                material_change_key_fresh(tx.conn_ref(), &claim, input.source_signal_id.as_deref())
                    .map_err(|error| error.to_string())?;
            let material_source_asof = max_datetime(
                claim.row.source_asof.as_deref().and_then(parse_datetime),
                input.trigger_source_asof.as_ref().cloned(),
            );
            let material_new_evidence = has_material_new_evidence(
                latest_suppression_at,
                latest_render_at,
                material_source_asof,
                input.evidence_signature_changed,
                input.subject_version_changed,
                material_change_key_fresh,
            );
            let candidate = candidate_from_parts(
                &claim,
                salience.salience.clone(),
                input.surface_class,
                input.trigger_refs.clone(),
                material_new_evidence,
            );
            let policy_state = load_policy_state(
                tx.conn_ref(),
                &policy,
                &claim,
                &candidate,
                &budget_key,
                input.render_surface,
                now,
            )
            .map_err(|error| error.to_string())?;
            let decision = decide_surfacing(candidate.clone(), &policy, policy_state, now);
            let trigger_refs_json = serde_json::to_string(&sanitized_trigger_refs_for_storage(
                &decision, &candidate, now,
            ))
            .map_err(|error| error.to_string())?;
            let why_this_now_json = match &decision {
                SurfacingDecision::Render { why_this_now, .. } => {
                    Some(serde_json::to_string(why_this_now).map_err(|error| error.to_string())?)
                }
                SurfacingDecision::Defer { .. } | SurfacingDecision::Suppress { .. } => None,
            };
            let decision_payload = decision_storage(&decision);
            let signal_value = surfacing_signal_value(
                &decision_id,
                &candidate,
                &budget_key,
                &policy.policy_version,
                &salience_evaluation_id,
                &decision_payload,
            )
            .map_err(|error| error.to_string())?;

            insert_surfacing_decision(
                tx,
                InsertSurfacingDecision {
                    decision_id: &decision_id,
                    idempotency_key: &idempotency_key,
                    candidate: &candidate,
                    policy: &policy,
                    input: &input,
                    local_day,
                    budget_key: &budget_key,
                    actor_kind: &actor_kind,
                    salience_evaluation_id: &salience_evaluation_id,
                    decision_payload: &decision_payload,
                    salience_total: salience.salience.total,
                    why_this_now_json: why_this_now_json.as_deref(),
                    trigger_refs_json: &trigger_refs_json,
                    created_at: now,
                },
            )
            .map_err(|error| error.to_string())?;

            crate::services::signals::emit_once_for_key_and_propagate(
                ctx,
                tx,
                engine,
                &idempotency_key,
                "claim",
                &candidate.claim_id.0,
                SURFACING_DECISION_SIGNAL,
                SURFACING_SIGNAL_SOURCE,
                Some(&signal_value),
                1.0,
            )
            .map_err(SurfacingError::Signal)
            .map_err(|error| error.to_string())
            .map(|(signal_outcome, derived_signal_ids)| {
                (decision, signal_outcome, derived_signal_ids)
            })
        })
        .map_err(SurfacingError::Database)?;

    Ok(SurfacingEvaluation {
        schema_version: SURFACING_EVALUATION_SCHEMA_VERSION,
        policy_version: policy.policy_version,
        claim_id: input.claim_id,
        salience_evaluation_id: Some(salience_evaluation_id),
        surfacing_decision_id: Some(decision_id),
        budget_key,
        decision,
        signal: Some(SurfacingSignalOutcome {
            signal_id: signal_outcome.id,
            signal_coalesced: signal_outcome.coalesced,
            derived_signal_ids,
        }),
    })
}

fn validate_schema_version(schema_version: u32) -> Result<(), SurfacingError> {
    if schema_version == SURFACING_EVALUATION_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(SurfacingError::UnsupportedSchemaVersion(schema_version))
    }
}

fn candidate_from_parts(
    claim: &LoadedClaim,
    salience: SalienceScore,
    surface_class: SurfaceClass,
    trigger_refs: Vec<TriggerRef>,
    material_new_evidence: bool,
) -> SurfacingCandidate {
    SurfacingCandidate {
        claim_id: claim.row.id.clone(),
        claim_type: claim.row.claim_type.clone(),
        sensitivity: claim.row.sensitivity.clone(),
        subject_kind: claim.subject_kind.clone(),
        subject_id: claim.subject_id.clone(),
        action_signature: claim.action_signature.clone(),
        source_asof: claim.row.source_asof.as_deref().and_then(parse_datetime),
        evidence_signature: claim.evidence_signature.clone(),
        salience,
        trigger_refs,
        surface_class,
        material_new_evidence,
    }
}

fn load_recommendation_claim(
    conn: &rusqlite::Connection,
    claim_id: &ClaimId,
) -> Result<LoadedClaim, SurfacingError> {
    let row = conn
        .query_row(
            "SELECT id, subject_ref, claim_type, source_asof, metadata_json,
                    claim_state, surfacing_state, sensitivity
               FROM intelligence_claims
              WHERE id = ?1",
            [&claim_id.0],
            |row| {
                Ok(ClaimRow {
                    id: ClaimId(row.get(0)?),
                    subject_ref: row.get(1)?,
                    claim_type: row.get(2)?,
                    source_asof: row.get(3)?,
                    metadata_json: row.get(4)?,
                    claim_state: row.get(5)?,
                    surfacing_state: row.get(6)?,
                    sensitivity: row.get(7)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| SurfacingError::ClaimNotFound(claim_id.0.clone()))?;

    if row.claim_type != CLAIM_TYPE_RECOMMENDATION {
        return Err(SurfacingError::NotRecommendationClaim(claim_id.0.clone()));
    }
    if row.claim_state != "active" || row.surfacing_state != "active" {
        return Err(SurfacingError::ClaimNotVisible(claim_id.0.clone()));
    }

    let (subject_kind, subject_id) = subject_scope(&row.subject_ref).ok_or_else(|| {
        SurfacingError::Database("claim subject_ref is not typed JSON".to_string())
    })?;
    let action_signature = action_signature(&row.metadata_json);
    let evidence_signature = evidence_signature(&row.metadata_json);

    Ok(LoadedClaim {
        row,
        subject_kind,
        subject_id,
        action_signature,
        evidence_signature,
    })
}

fn load_policy(conn: &rusqlite::Connection) -> Result<SurfacingPolicy, SurfacingError> {
    if !table_exists(conn, "recommendation_surfacing_policy")? {
        return Ok(SurfacingPolicy::default());
    }

    conn.query_row(
        "SELECT policy_version, critical_threshold, urgent_factor_threshold,
                critical_primary_daily_budget, notable_threshold,
                notable_primary_daily_budget, background_threshold,
                background_daily_budget, claim_cooldown_days,
                subject_action_cooldown_days, feedback_suppression_days
           FROM recommendation_surfacing_policy
          WHERE policy_version = ?1 AND claim_type = 'recommendation'",
        [SURFACING_POLICY_VERSION],
        |row| {
            Ok(SurfacingPolicy {
                policy_version: row.get(0)?,
                critical_threshold: row.get(1)?,
                urgent_factor_threshold: row.get(2)?,
                critical_primary_daily_budget: row.get::<_, i64>(3)?.max(0) as u32,
                notable_threshold: row.get(4)?,
                notable_primary_daily_budget: row.get::<_, i64>(5)?.max(0) as u32,
                background_threshold: row.get(6)?,
                background_daily_budget: row.get::<_, i64>(7)?.max(0) as u32,
                claim_cooldown_days: row.get(8)?,
                subject_action_cooldown_days: row.get(9)?,
                feedback_suppression_days: row.get(10)?,
            })
        },
    )
    .optional()?
    .map(Ok)
    .unwrap_or_else(|| Ok(SurfacingPolicy::default()))
}

fn latest_suppression_at(
    conn: &rusqlite::Connection,
    claim: &LoadedClaim,
    input: &SurfacingEvaluationInput,
    now: DateTime<Utc>,
    policy: &SurfacingPolicy,
) -> Result<Option<DateTime<Utc>>, SurfacingError> {
    let dismissal = latest_surface_dismissal(conn, &claim.row.id, input.render_surface)?;
    let feedback = latest_feedback_suppression(conn, &claim.row.id, now, policy)?;
    Ok(match (dismissal, feedback) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    })
}

fn load_policy_state(
    conn: &rusqlite::Connection,
    policy: &SurfacingPolicy,
    claim: &LoadedClaim,
    candidate: &SurfacingCandidate,
    budget_key: &str,
    render_surface: ClaimDismissalSurface,
    now: DateTime<Utc>,
) -> Result<SurfacingPolicyState, SurfacingError> {
    let claim_dismissed_recently =
        latest_surface_dismissal(conn, &claim.row.id, render_surface)?.is_some();
    let feedback_suppressed_recently =
        latest_feedback_suppression(conn, &claim.row.id, now, policy)?.is_some();
    let claim_cooldown_until =
        claim_render_cooldown_until(conn, &claim.row.id, now, policy.claim_cooldown_days)?;
    let subject_action_cooldown_until = subject_action_render_cooldown_until(
        conn,
        &candidate.subject_kind,
        &candidate.subject_id,
        &candidate.action_signature,
        now,
        policy.subject_action_cooldown_days,
    )?;

    Ok(SurfacingPolicyState {
        claim_dismissed_recently,
        feedback_suppressed_recently,
        claim_cooldown_active: claim_cooldown_until.is_some(),
        claim_cooldown_until,
        subject_action_cooldown_active: subject_action_cooldown_until.is_some(),
        subject_action_cooldown_until,
        critical_primary_used: rendered_budget_count(
            conn,
            budget_key,
            Some(SurfacingTier::Critical),
        )?,
        notable_primary_used: rendered_budget_count(
            conn,
            budget_key,
            Some(SurfacingTier::Notable),
        )?,
        background_used: rendered_background_count(conn, budget_key)?,
    })
}

fn latest_surface_dismissal(
    conn: &rusqlite::Connection,
    claim_id: &ClaimId,
    render_surface: ClaimDismissalSurface,
) -> Result<Option<DateTime<Utc>>, SurfacingError> {
    if !table_exists(conn, "claim_surface_dismissals")? {
        return Ok(None);
    }

    let raw = conn
        .query_row(
            "SELECT dismissed_at
               FROM claim_surface_dismissals
              WHERE claim_id = ?1 AND surface = ?2
              ORDER BY dismissed_at DESC
              LIMIT 1",
            params![&claim_id.0, render_surface.as_str()],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(raw.as_deref().and_then(parse_datetime))
}

fn latest_feedback_suppression(
    conn: &rusqlite::Connection,
    claim_id: &ClaimId,
    now: DateTime<Utc>,
    policy: &SurfacingPolicy,
) -> Result<Option<DateTime<Utc>>, SurfacingError> {
    if !table_exists(conn, "claim_feedback")? {
        return Ok(None);
    }

    let threshold = (now - Duration::days(policy.feedback_suppression_days)).to_rfc3339();
    let raw = conn
        .query_row(
            "SELECT submitted_at
               FROM claim_feedback
              WHERE claim_id = ?1
                AND feedback_type IN (
                    'surface_inappropriate',
                    'not_relevant_here',
                    'mark_false',
                    'wrong_subject'
                )
                AND submitted_at >= ?2
              ORDER BY submitted_at DESC
              LIMIT 1",
            params![&claim_id.0, threshold],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(raw.as_deref().and_then(parse_datetime))
}

fn latest_render_at(
    conn: &rusqlite::Connection,
    claim_id: &ClaimId,
) -> Result<Option<DateTime<Utc>>, SurfacingError> {
    if !table_exists(conn, "surfacing_decisions")? {
        return Ok(None);
    }
    let raw: Option<String> = conn
        .query_row(
            "SELECT created_at
               FROM surfacing_decisions
              WHERE claim_id = ?1
                AND policy_version = ?2
                AND decision_kind = 'render'
              ORDER BY created_at DESC
              LIMIT 1",
            params![&claim_id.0, SURFACING_POLICY_VERSION],
            |row| row.get(0),
        )
        .optional()?;
    Ok(raw.as_deref().and_then(parse_datetime))
}

fn latest_subject_action_render_at(
    conn: &rusqlite::Connection,
    subject_kind: &str,
    subject_id: &str,
    action_signature: &str,
) -> Result<Option<DateTime<Utc>>, SurfacingError> {
    if !table_exists(conn, "surfacing_decisions")? {
        return Ok(None);
    }
    let raw: Option<String> = conn
        .query_row(
            "SELECT created_at
               FROM surfacing_decisions
              WHERE policy_version = ?1
                AND decision_kind = 'render'
                AND surfacing_tier IN ('critical', 'notable')
                AND surface_class = 'primary'
                AND subject_kind = ?2
                AND subject_id = ?3
                AND action_signature = ?4
              ORDER BY created_at DESC
              LIMIT 1",
            params![
                SURFACING_POLICY_VERSION,
                subject_kind,
                subject_id,
                action_signature
            ],
            |row| row.get(0),
        )
        .optional()?;
    Ok(raw.as_deref().and_then(parse_datetime))
}

fn material_change_key_fresh(
    conn: &rusqlite::Connection,
    claim: &LoadedClaim,
    source_signal_id: Option<&str>,
) -> Result<bool, SurfacingError> {
    let Some(source_signal_id) = source_signal_id else {
        return Ok(false);
    };
    if !table_exists(conn, "surfacing_decisions")? {
        return Ok(true);
    }
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM surfacing_decisions
          WHERE policy_version = ?1
            AND decision_kind = 'render'
            AND source_signal_id = ?2
            AND (
                claim_id = ?3
                OR (
                    subject_kind = ?4
                    AND subject_id = ?5
                    AND action_signature = ?6
                )
            )",
        params![
            SURFACING_POLICY_VERSION,
            source_signal_id,
            &claim.row.id.0,
            &claim.subject_kind,
            &claim.subject_id,
            &claim.action_signature,
        ],
        |row| row.get(0),
    )?;
    Ok(count == 0)
}

fn claim_render_cooldown_until(
    conn: &rusqlite::Connection,
    claim_id: &ClaimId,
    now: DateTime<Utc>,
    cooldown_days: i64,
) -> Result<Option<DateTime<Utc>>, SurfacingError> {
    if !table_exists(conn, "surfacing_decisions")? {
        return Ok(None);
    }
    let threshold = (now - Duration::days(cooldown_days)).to_rfc3339();
    let rendered_at: Option<String> = conn
        .query_row(
            "SELECT created_at
           FROM surfacing_decisions
          WHERE claim_id = ?1
            AND policy_version = ?2
            AND decision_kind = 'render'
            AND surfacing_tier IN ('critical', 'notable')
            AND surface_class = 'primary'
            AND created_at >= ?3
          ORDER BY created_at DESC
          LIMIT 1",
            params![&claim_id.0, SURFACING_POLICY_VERSION, threshold],
            |row| row.get(0),
        )
        .optional()?;
    Ok(rendered_at
        .as_deref()
        .and_then(parse_datetime)
        .map(|rendered_at| rendered_at + Duration::days(cooldown_days)))
}

fn subject_action_render_cooldown_until(
    conn: &rusqlite::Connection,
    subject_kind: &str,
    subject_id: &str,
    action_signature: &str,
    now: DateTime<Utc>,
    cooldown_days: i64,
) -> Result<Option<DateTime<Utc>>, SurfacingError> {
    if !table_exists(conn, "surfacing_decisions")? {
        return Ok(None);
    }
    let threshold = (now - Duration::days(cooldown_days)).to_rfc3339();
    let rendered_at: Option<String> = conn
        .query_row(
            "SELECT created_at
           FROM surfacing_decisions
          WHERE policy_version = ?1
            AND decision_kind = 'render'
            AND surfacing_tier IN ('critical', 'notable')
            AND surface_class = 'primary'
            AND subject_kind = ?2
            AND subject_id = ?3
            AND action_signature = ?4
            AND created_at >= ?5
          ORDER BY created_at DESC
          LIMIT 1",
            params![
                SURFACING_POLICY_VERSION,
                subject_kind,
                subject_id,
                action_signature,
                threshold
            ],
            |row| row.get(0),
        )
        .optional()?;
    Ok(rendered_at
        .as_deref()
        .and_then(parse_datetime)
        .map(|rendered_at| rendered_at + Duration::days(cooldown_days)))
}

fn rendered_budget_count(
    conn: &rusqlite::Connection,
    budget_key: &str,
    tier: Option<SurfacingTier>,
) -> Result<u32, SurfacingError> {
    if !table_exists(conn, "surfacing_decisions")? {
        return Ok(0);
    }

    let count: i64 = if let Some(tier) = tier {
        conn.query_row(
            "SELECT COUNT(*)
               FROM surfacing_decisions
              WHERE budget_key = ?1
                AND decision_kind = 'render'
                AND surface_class = 'primary'
                AND surfacing_tier = ?2",
            params![budget_key, tier_storage(tier)],
            |row| row.get(0),
        )?
    } else {
        conn.query_row(
            "SELECT COUNT(*)
               FROM surfacing_decisions
              WHERE budget_key = ?1
                AND decision_kind = 'render'
                AND surface_class = 'primary'
                AND surfacing_tier IN ('critical', 'notable')",
            params![budget_key],
            |row| row.get(0),
        )?
    };
    Ok(count.clamp(0, i64::from(u32::MAX)) as u32)
}

fn rendered_background_count(
    conn: &rusqlite::Connection,
    budget_key: &str,
) -> Result<u32, SurfacingError> {
    if !table_exists(conn, "surfacing_decisions")? {
        return Ok(0);
    }
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM surfacing_decisions
          WHERE budget_key = ?1
            AND decision_kind = 'render'
            AND surface_class = 'background'",
        params![budget_key],
        |row| row.get(0),
    )?;
    Ok(count.clamp(0, i64::from(u32::MAX)) as u32)
}

struct InsertSurfacingDecision<'a> {
    decision_id: &'a str,
    idempotency_key: &'a str,
    candidate: &'a SurfacingCandidate,
    policy: &'a SurfacingPolicy,
    input: &'a SurfacingEvaluationInput,
    local_day: NaiveDate,
    budget_key: &'a str,
    actor_kind: &'a str,
    salience_evaluation_id: &'a str,
    decision_payload: &'a DecisionStorage,
    salience_total: f64,
    why_this_now_json: Option<&'a str>,
    trigger_refs_json: &'a str,
    created_at: DateTime<Utc>,
}

fn insert_surfacing_decision(
    db: &ActionDb,
    input: InsertSurfacingDecision<'_>,
) -> Result<(), rusqlite::Error> {
    db.conn_ref().execute(
        "INSERT INTO surfacing_decisions (
             id, idempotency_key, policy_version, claim_id, decision_kind,
             surfacing_tier, defer_reason, defer_until, suppress_reason, budget_key,
             actor_kind, local_day, claim_type, sensitivity, surface_class,
             render_surface, subject_kind, subject_id, action_signature,
             salience_total, salience_evaluation_id, why_this_now_json,
             trigger_refs_json, evidence_signature, source_asof,
             source_signal_id, created_at
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5,
             ?6, ?7, ?8, ?9, ?10,
             ?11, ?12, ?13, ?14, ?15,
             ?16, ?17, ?18, ?19,
             ?20, ?21, ?22,
             ?23, ?24, ?25,
             ?26, ?27
         )",
        params![
            input.decision_id,
            input.idempotency_key,
            &input.policy.policy_version,
            &input.candidate.claim_id.0,
            input.decision_payload.kind,
            input.decision_payload.tier,
            input.decision_payload.defer_reason,
            input.decision_payload.defer_until.as_deref(),
            input.decision_payload.suppress_reason,
            input.budget_key,
            input.actor_kind,
            input.local_day.to_string(),
            &input.candidate.claim_type,
            &input.candidate.sensitivity,
            input.candidate.surface_class.as_str(),
            input.input.render_surface.as_str(),
            &input.candidate.subject_kind,
            &input.candidate.subject_id,
            &input.candidate.action_signature,
            input.salience_total,
            input.salience_evaluation_id,
            input.why_this_now_json,
            input.trigger_refs_json,
            input.candidate.evidence_signature.as_deref(),
            input
                .candidate
                .source_asof
                .as_ref()
                .map(|dt| dt.to_rfc3339()),
            input.input.source_signal_id.as_deref(),
            input.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

fn surfacing_signal_value(
    decision_id: &str,
    candidate: &SurfacingCandidate,
    budget_key: &str,
    policy_version: &str,
    salience_evaluation_id: &str,
    decision: &DecisionStorage,
) -> Result<String, SurfacingError> {
    serde_json::to_string(&json!({
        "surfacingDecisionId": decision_id,
        "claimId": candidate.claim_id.0,
        "policyVersion": policy_version,
        "decisionKind": decision.kind,
        "tier": decision.tier,
        "deferReason": decision.defer_reason,
        "deferUntil": decision.defer_until.as_deref(),
        "suppressReason": decision.suppress_reason,
        "budgetKey": budget_key,
        "salienceEvaluationId": salience_evaluation_id,
        "subject": {
            "kind": candidate.subject_kind,
            "id": candidate.subject_id,
        }
    }))
    .map_err(SurfacingError::from)
}

#[derive(Debug, Clone)]
struct DecisionStorage {
    kind: &'static str,
    tier: Option<&'static str>,
    defer_reason: Option<&'static str>,
    defer_until: Option<String>,
    suppress_reason: Option<&'static str>,
}

fn decision_storage(decision: &SurfacingDecision) -> DecisionStorage {
    match decision {
        SurfacingDecision::Render { tier, .. } => DecisionStorage {
            kind: "render",
            tier: Some(tier_storage(*tier)),
            defer_reason: None,
            defer_until: None,
            suppress_reason: None,
        },
        SurfacingDecision::Defer { reason, until } => DecisionStorage {
            kind: "defer",
            tier: None,
            defer_reason: Some(defer_reason_storage(*reason)),
            defer_until: Some(until.to_rfc3339()),
            suppress_reason: None,
        },
        SurfacingDecision::Suppress { reason } => DecisionStorage {
            kind: "suppress",
            tier: None,
            defer_reason: None,
            defer_until: None,
            suppress_reason: Some(suppress_reason_storage(*reason)),
        },
    }
}

fn sanitized_trigger_refs_for_storage(
    decision: &SurfacingDecision,
    candidate: &SurfacingCandidate,
    now: DateTime<Utc>,
) -> Vec<TriggerRef> {
    match decision {
        SurfacingDecision::Render { why_this_now, .. } => why_this_now.triggers.clone(),
        SurfacingDecision::Defer { .. } | SurfacingDecision::Suppress { .. } => {
            why_this_now_for_score(&candidate.salience, &candidate.trigger_refs, now).triggers
        }
    }
}

fn has_material_new_evidence(
    suppression_at: Option<DateTime<Utc>>,
    latest_render_at: Option<DateTime<Utc>>,
    source_asof: Option<DateTime<Utc>>,
    evidence_signature_changed: bool,
    subject_version_changed: bool,
    material_change_key_fresh: bool,
) -> bool {
    let has_changed_marker = evidence_signature_changed || subject_version_changed;
    if has_changed_marker && material_change_key_fresh {
        return true;
    }
    if let Some(rendered_at) = latest_render_at.filter(|rendered_at| {
        suppression_at.is_none_or(|suppression_at| *rendered_at > suppression_at)
    }) {
        return source_asof.is_some_and(|source_asof| source_asof > rendered_at);
    }
    let Some(suppression_at) = suppression_at else {
        return source_asof.is_some() || has_changed_marker;
    };
    source_asof.is_some_and(|source_asof| source_asof > suppression_at) || has_changed_marker
}

fn max_datetime(
    left: Option<DateTime<Utc>>,
    right: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn subject_scope(subject_ref: &str) -> Option<(String, String)> {
    let value = serde_json::from_str::<serde_json::Value>(subject_ref).ok()?;
    let kind = value.get("kind")?.as_str()?.to_string();
    let id = value.get("id")?.as_str()?.to_string();
    Some((kind, id))
}

fn action_signature(metadata_json: &Option<String>) -> String {
    let Some(action) = metadata_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<RecommendationMetadataEnvelope>(raw).ok())
        .map(|envelope| envelope.recommendation.recommended_action)
    else {
        return "unknown".to_string();
    };

    action_key(&action).unwrap_or_else(|_| fallback_action_signature(&action))
}

fn fallback_action_signature(action: &RecommendedAction) -> String {
    match action {
        RecommendedAction::ScheduleMeeting { .. } => "scheduleMeeting".to_string(),
        RecommendedAction::SendMessage { .. } => "sendMessage".to_string(),
        RecommendedAction::ReviewClaim { .. } => "reviewClaim".to_string(),
        RecommendedAction::UpdateRecord { .. } => "updateRecord".to_string(),
        RecommendedAction::InvestigateChange { .. } => "investigateChange".to_string(),
        RecommendedAction::Custom { .. } => "custom.invalid".to_string(),
    }
}

pub(super) fn evidence_signature(metadata_json: &Option<String>) -> Option<String> {
    let evidence = metadata_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|value| value.pointer("/recommendation/evidence").cloned())?;
    let mut hasher = Sha256::new();
    hasher.update(evidence.to_string().as_bytes());
    Some(format!("evsig_{}", hex::encode(&hasher.finalize()[..16])))
}

fn budget_key(
    ctx: &ServiceContext<'_>,
    claim_type: &str,
    sensitivity: &str,
    surface_class: SurfaceClass,
    local_day: NaiveDate,
) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        actor_kind(ctx),
        local_day,
        claim_type,
        sensitivity,
        surface_class.as_str()
    )
}

fn actor_kind(ctx: &ServiceContext<'_>) -> String {
    ctx.actor.split(':').next().unwrap_or("system").to_string()
}

fn factor_value(score: &SalienceScore, kind: SalienceFactorKind) -> f64 {
    score
        .factors
        .iter()
        .find(|factor| factor.kind == kind)
        .and_then(|factor| factor.value)
        .unwrap_or(0.0)
}

fn next_local_day(now: DateTime<Utc>) -> DateTime<Utc> {
    now.date_naive()
        .succ_opt()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|naive| naive.and_utc())
        .unwrap_or(now + Duration::days(1))
}

fn table_exists(conn: &rusqlite::Connection, table: &str) -> Result<bool, SurfacingError> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM sqlite_master
          WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get::<_, i64>(0).map(|count| count > 0),
    )
    .map_err(SurfacingError::from)
}

fn sanitize_surfacing_input(mut input: SurfacingEvaluationInput) -> SurfacingEvaluationInput {
    input.source_signal_id =
        safe_optional_storage_ref("signal_ref", input.source_signal_id.as_deref());
    input
}

fn safe_optional_storage_ref(prefix: &str, value: Option<&str>) -> Option<String> {
    value.map(|value| safe_storage_ref(prefix, value))
}

fn safe_storage_ref(prefix: &str, value: &str) -> String {
    let value = value.trim();
    if is_safe_storage_ref(value) {
        value.to_string()
    } else {
        hashed_storage_ref(prefix, value)
    }
}

fn is_safe_storage_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':' | b'.'))
}

fn hashed_storage_ref(prefix: &str, value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    hasher.update(b":");
    hasher.update(value.as_bytes());
    format!("{prefix}_{}", hex::encode(&hasher.finalize()[..16]))
}

fn parse_datetime(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                .map(|value| value.and_utc())
        })
        .ok()
}

fn tier_storage(tier: SurfacingTier) -> &'static str {
    match tier {
        SurfacingTier::Critical => "critical",
        SurfacingTier::Notable => "notable",
        SurfacingTier::Background => "background",
        SurfacingTier::Quiet => "quiet",
    }
}

fn defer_reason_storage(reason: DeferReason) -> &'static str {
    match reason {
        DeferReason::CooldownActive => "cooldown_active",
        DeferReason::BudgetExhausted => "budget_exhausted",
        DeferReason::AwaitingCorroboration => "awaiting_corroboration",
        DeferReason::PendingTrigger => "pending_trigger",
    }
}

fn suppress_reason_storage(reason: SuppressReason) -> &'static str {
    match reason {
        SuppressReason::BelowThreshold => "below_threshold",
        SuppressReason::UserMutedSubject => "user_muted_subject",
        SuppressReason::DismissedRecently => "dismissed_recently",
        SuppressReason::ContradictedWithStrongerEvidence => "contradicted_with_stronger_evidence",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::TimeZone;

    use abilities_runtime::abilities::trust::types::TrustBand;
    use abilities_runtime::types::{ClaimSensitivity, TemporalScope};

    use crate::abilities::claims::ClaimType;
    use crate::services::claims::{
        commit_claim, update_claim_trust, ClaimProposal, DeterministicInsertProposal, TrustScore,
    };
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};
    use crate::services::recommendations::contracts::{
        EvidenceRef, FactorRationale, RecommendationDraft, SalienceFactor, TriggerKind,
    };
    use crate::services::recommendations::recommendation::metadata_envelope;

    fn test_db() -> ActionDb {
        ActionDb::from_connection_for_tests(crate::migrations::migrated_in_memory_for_tests())
    }

    fn test_ctx() -> (FixedClock, SeedableRng, ExternalClients) {
        (
            FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 26, 12, 0, 0).unwrap()),
            SeedableRng::new(17),
            ExternalClients::default(),
        )
    }

    fn live_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        external: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, external).with_actor("system:test")
    }

    fn insert_recommendation_claim(db: &ActionDb, id: &str, trust_score: f64, source_asof: &str) {
        insert_recommendation_claim_with_metadata_source(db, id, id, trust_score, source_asof);
    }

    fn insert_recommendation_claim_with_metadata_source(
        db: &ActionDb,
        id: &str,
        metadata_source_id: &str,
        trust_score: f64,
        source_asof: &str,
    ) {
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let action_kind = default_action_kind(metadata_source_id);
        let metadata = metadata_envelope(&super_recommendation_draft_with_action_kind(
            metadata_source_id,
            source_asof,
            &action_kind,
        ));
        let proposal = ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: r#"{"kind":"account","id":"acct-example"}"#.to_string(),
            claim_type: ClaimType::Recommendation.as_str().to_string(),
            field_path: Some(format!("recommendation.{id}")),
            topic_key: Some(id.to_string()),
            text: "Review the account before the next customer conversation.".to_string(),
            actor: "agent".to_string(),
            data_source: "recommendation".to_string(),
            source_ref: Some("run:test".to_string()),
            source_asof: Some(source_asof.to_string()),
            observed_at: "2026-05-26T10:00:00Z".to_string(),
            provenance_json: r#"{"sources":[]}"#.to_string(),
            metadata_json: Some(serde_json::to_string(&metadata).unwrap()),
            thread_id: None,
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(ClaimSensitivity::Internal),
            supersedes: None,
            tombstone: None,
        };
        commit_claim(
            &ctx,
            db,
            DeterministicInsertProposal::new(id.to_string(), proposal),
        )
        .expect("insert recommendation claim");
        update_claim_trust(db, id, TrustScore(trust_score), 1, &ctx).expect("seed trust score");
    }

    fn default_action_kind(id: &str) -> String {
        format!("action_{id}").replace([':', '-'], "_")
    }

    fn super_recommendation_draft(id: &str, source_asof: &str) -> RecommendationDraft {
        let action_kind = default_action_kind(id);
        super_recommendation_draft_with_action_kind(id, source_asof, &action_kind)
    }

    fn super_recommendation_draft_with_action_kind(
        _id: &str,
        source_asof: &str,
        action_kind: &str,
    ) -> RecommendationDraft {
        RecommendationDraft {
            subject: abilities_runtime::abilities::provenance::subject::SubjectRef::Account(
                "acct-example".to_string(),
            ),
            recommended_action: RecommendedAction::Custom {
                action_kind: action_kind.to_string(),
                payload: serde_json::json!({ "opaque": true }),
            },
            evidence: vec![EvidenceRef {
                source: "claim:source-1".to_string(),
                chunk: None,
            }],
            provenance_json: r#"{"sources":[]}"#.to_string(),
            source_ref: Some("run:test".to_string()),
            source_asof: Some(source_asof.parse::<DateTime<Utc>>().unwrap()),
            observed_at: "2026-05-26T10:00:00Z".parse::<DateTime<Utc>>().unwrap(),
            text: "Review the account before the next customer conversation.".to_string(),
            salience: SalienceScore {
                total: 0.75,
                factors: vec![SalienceFactor {
                    kind: SalienceFactorKind::Trust,
                    value: Some(0.9),
                    weight: 0.1,
                    rationale: FactorRationale::Trust {
                        trust_band: TrustBand::LikelyCurrent,
                    },
                }],
            },
        }
    }

    fn input(claim_id: &str) -> SurfacingEvaluationInput {
        SurfacingEvaluationInput {
            schema_version: SURFACING_EVALUATION_SCHEMA_VERSION,
            claim_id: ClaimId(claim_id.to_string()),
            render_surface: ClaimDismissalSurface::Briefing,
            surface_class: SurfaceClass::Primary,
            trigger_refs: vec![TriggerRef {
                trigger_kind: TriggerKind::SignalArrival,
                at: "2026-05-26T12:00:00Z".parse::<DateTime<Utc>>().unwrap(),
                source: "/Users/example/raw.md".to_string(),
            }],
            source_signal_id: Some("signal-1".to_string()),
            trigger_source_asof: None,
            evidence_signature_changed: false,
            subject_version_changed: false,
        }
    }

    #[test]
    fn high_salience_candidate_renders_notable_and_persists_signal() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-surface-1", 0.95, "2026-05-26T10:00:00Z");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();

        let result = evaluate_surfacing_for_claim(&ctx, &db, &engine, input("claim-surface-1"))
            .expect("surface evaluation succeeds");

        assert!(matches!(
            result.decision,
            SurfacingDecision::Render {
                tier: SurfacingTier::Notable,
                ..
            }
        ));
        assert!(result.salience_evaluation_id.is_some());
        assert!(result.surfacing_decision_id.is_some());
        assert!(result.signal.is_some());
        let row_count: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM surfacing_decisions", [], |row| {
                row.get(0)
            })
            .expect("count surfacing decisions");
        assert_eq!(row_count, 1);
        let stored: String = db
            .conn_ref()
            .query_row(
                "SELECT trigger_refs_json FROM surfacing_decisions",
                [],
                |row| row.get(0),
            )
            .expect("read trigger refs");
        assert!(!stored.contains("/Users/"));
        assert!(stored.contains("signal_event"));
    }

    #[test]
    fn direct_surfacing_hashes_untrusted_source_signal_id() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-surface-privacy", 0.95, "2026-05-26T10:00:00Z");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();
        let mut request = input("claim-surface-privacy");
        request.source_signal_id = Some("/Users/example/raw-workspace-note.md".to_string());

        let result = evaluate_surfacing_for_claim(&ctx, &db, &engine, request)
            .expect("surface evaluation succeeds");

        let stored: String = db
            .conn_ref()
            .query_row(
                "SELECT source_signal_id
                   FROM surfacing_decisions
                  WHERE id = ?1",
                [result.surfacing_decision_id.as_deref().unwrap()],
                |row| row.get(0),
            )
            .expect("read stored source signal id");
        assert!(stored.starts_with("signal_ref_"));
        assert!(!stored.contains("/Users/"));
    }

    #[test]
    fn primary_budget_overflow_defers_fourth_notable() {
        let db = test_db();
        for id in [
            "claim-budget-1",
            "claim-budget-2",
            "claim-budget-3",
            "claim-budget-4",
        ] {
            insert_recommendation_claim(&db, id, 0.95, "2026-05-26T10:00:00Z");
        }
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();

        for id in ["claim-budget-1", "claim-budget-2", "claim-budget-3"] {
            let result =
                evaluate_surfacing_for_claim(&ctx, &db, &engine, input(id)).expect("render");
            assert!(matches!(result.decision, SurfacingDecision::Render { .. }));
        }
        let result = evaluate_surfacing_for_claim(&ctx, &db, &engine, input("claim-budget-4"))
            .expect("overflow decision");

        assert!(matches!(
            result.decision,
            SurfacingDecision::Defer {
                reason: DeferReason::BudgetExhausted,
                ..
            }
        ));
        let stored: (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT defer_reason, defer_until
                   FROM surfacing_decisions
                  WHERE id = ?1",
                [result.surfacing_decision_id.as_deref().unwrap()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read persisted deferral");
        assert_eq!(stored.0, "budget_exhausted");
        assert_eq!(
            parse_datetime(&stored.1),
            Some(Utc.with_ymd_and_hms(2026, 5, 27, 0, 0, 0).unwrap())
        );
    }

    #[test]
    fn background_budget_caps_high_salience_background_candidates() {
        let db = test_db();
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();

        for index in 0..11 {
            let id = format!("claim-background-budget-{index}");
            insert_recommendation_claim(&db, &id, 0.95, "2026-05-26T10:00:00Z");
            let mut request = input(&id);
            request.surface_class = SurfaceClass::Background;
            let result =
                evaluate_surfacing_for_claim(&ctx, &db, &engine, request).expect("evaluate");

            if index < 10 {
                assert!(matches!(result.decision, SurfacingDecision::Render { .. }));
            } else {
                assert!(matches!(
                    result.decision,
                    SurfacingDecision::Defer {
                        reason: DeferReason::BudgetExhausted,
                        ..
                    }
                ));
            }
        }
    }

    #[test]
    fn critical_primary_render_does_not_spend_notable_budget() {
        let db = test_db();
        for id in [
            "claim-budget-critical",
            "claim-budget-notable-1",
            "claim-budget-notable-2",
            "claim-budget-notable-3",
        ] {
            insert_recommendation_claim(&db, id, 0.95, "2026-05-26T10:00:00Z");
        }
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();

        let critical =
            evaluate_surfacing_for_claim(&ctx, &db, &engine, input("claim-budget-critical"))
                .expect("render critical seed");
        db.conn_ref()
            .execute(
                "UPDATE surfacing_decisions
                    SET surfacing_tier = 'critical'
                  WHERE id = ?1",
                [critical.surfacing_decision_id.as_deref().unwrap()],
            )
            .expect("mark seed as critical");

        for id in [
            "claim-budget-notable-1",
            "claim-budget-notable-2",
            "claim-budget-notable-3",
        ] {
            let result =
                evaluate_surfacing_for_claim(&ctx, &db, &engine, input(id)).expect("render");
            assert!(matches!(
                result.decision,
                SurfacingDecision::Render {
                    tier: SurfacingTier::Notable,
                    ..
                }
            ));
        }
    }

    #[test]
    fn cooldown_defer_until_uses_original_render_expiry() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-cooldown-expiry", 0.95, "2026-05-20T10:00:00Z");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();

        let initial =
            evaluate_surfacing_for_claim(&ctx, &db, &engine, input("claim-cooldown-expiry"))
                .expect("initial render");
        db.conn_ref()
            .execute(
                "UPDATE surfacing_decisions
                    SET created_at = '2026-05-20T12:00:00+00:00'
                  WHERE id = ?1",
                [initial.surfacing_decision_id.as_deref().unwrap()],
            )
            .expect("age render");

        let result =
            evaluate_surfacing_for_claim(&ctx, &db, &engine, input("claim-cooldown-expiry"))
                .expect("cooldown decision");

        assert!(matches!(
            result.decision,
            SurfacingDecision::Defer {
                reason: DeferReason::CooldownActive,
                until,
            } if until == Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap()
        ));
    }

    #[test]
    fn recent_surface_dismissal_suppresses_until_material_evidence_changes() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-dismissed", 0.95, "2026-05-26T10:00:00Z");
        db.conn_ref()
            .execute(
                "INSERT INTO claim_surface_dismissals (
                     claim_id, surface, actor, dismissed_at
                 ) VALUES ('claim-dismissed', 'briefing', 'user', '2026-05-26T11:00:00Z')",
                [],
            )
            .expect("insert dismissal");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();

        let suppressed = evaluate_surfacing_for_claim(&ctx, &db, &engine, input("claim-dismissed"))
            .expect("suppression decision");
        assert!(matches!(
            suppressed.decision,
            SurfacingDecision::Suppress {
                reason: SuppressReason::DismissedRecently
            }
        ));

        let mut changed = input("claim-dismissed");
        changed.evidence_signature_changed = true;
        let resurfaced = evaluate_surfacing_for_claim(&ctx, &db, &engine, changed)
            .expect("material evidence can resurface");
        assert!(matches!(
            resurfaced.decision,
            SurfacingDecision::Render { .. }
        ));

        let mut same_change = input("claim-dismissed");
        same_change.evidence_signature_changed = true;
        let repeated = evaluate_surfacing_for_claim(&ctx, &db, &engine, same_change)
            .expect("same material evidence is consumed");
        assert!(matches!(
            repeated.decision,
            SurfacingDecision::Suppress {
                reason: SuppressReason::DismissedRecently,
            }
        ));
    }

    #[test]
    fn old_surface_dismissal_remains_suppressed_until_material_evidence_changes() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-old-dismissed", 0.95, "2026-04-30T10:00:00Z");
        db.conn_ref()
            .execute(
                "INSERT INTO claim_surface_dismissals (
                     claim_id, surface, actor, dismissed_at
                 ) VALUES ('claim-old-dismissed', 'briefing', 'user', '2026-05-01T11:00:00Z')",
                [],
            )
            .expect("insert old dismissal");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();

        let suppressed =
            evaluate_surfacing_for_claim(&ctx, &db, &engine, input("claim-old-dismissed"))
                .expect("old dismissal still suppresses");
        assert!(matches!(
            suppressed.decision,
            SurfacingDecision::Suppress {
                reason: SuppressReason::DismissedRecently
            }
        ));

        let mut changed = input("claim-old-dismissed");
        changed.evidence_signature_changed = true;
        let resurfaced = evaluate_surfacing_for_claim(&ctx, &db, &engine, changed)
            .expect("material evidence can override durable dismissal");
        assert!(matches!(
            resurfaced.decision,
            SurfacingDecision::Render { .. }
        ));
    }

    #[test]
    fn material_source_after_recent_render_bypasses_cooldown_once() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-material-cooldown", 0.95, "2026-05-26T10:00:00Z");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();

        let initial =
            evaluate_surfacing_for_claim(&ctx, &db, &engine, input("claim-material-cooldown"))
                .expect("initial render");
        db.conn_ref()
            .execute(
                "UPDATE surfacing_decisions
                    SET created_at = '2026-05-26T11:00:00+00:00'
                  WHERE id = ?1",
                [initial.surfacing_decision_id.as_deref().unwrap()],
            )
            .expect("age render");

        let mut changed = input("claim-material-cooldown");
        changed.trigger_source_asof = Some(Utc.with_ymd_and_hms(2026, 5, 26, 11, 30, 0).unwrap());
        changed.evidence_signature_changed = true;
        let resurfaced = evaluate_surfacing_for_claim(&ctx, &db, &engine, changed.clone())
            .expect("material evidence bypasses cooldown");
        assert!(matches!(
            resurfaced.decision,
            SurfacingDecision::Render { .. }
        ));

        let repeated = evaluate_surfacing_for_claim(&ctx, &db, &engine, changed)
            .expect("same material evidence is consumed");
        assert!(matches!(
            repeated.decision,
            SurfacingDecision::Defer {
                reason: DeferReason::CooldownActive,
                ..
            }
        ));
    }

    #[test]
    fn material_baseline_uses_subject_action_render_for_duplicate_claims() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-subject-action-1", 0.95, "2026-05-26T10:00:00Z");
        insert_recommendation_claim_with_metadata_source(
            &db,
            "claim-subject-action-2",
            "claim-subject-action-1",
            0.95,
            "2026-05-26T10:00:00Z",
        );
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();

        let first =
            evaluate_surfacing_for_claim(&ctx, &db, &engine, input("claim-subject-action-1"))
                .expect("first render");
        assert!(matches!(first.decision, SurfacingDecision::Render { .. }));
        let second =
            evaluate_surfacing_for_claim(&ctx, &db, &engine, input("claim-subject-action-2"))
                .expect("second same action");

        assert!(matches!(
            second.decision,
            SurfacingDecision::Defer {
                reason: DeferReason::CooldownActive,
                ..
            }
        ));
    }

    #[test]
    fn evidence_change_signal_after_render_bypasses_cooldown_once() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-evidence-signal", 0.95, "2026-05-26T10:00:00Z");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();

        let initial =
            evaluate_surfacing_for_claim(&ctx, &db, &engine, input("claim-evidence-signal"))
                .expect("initial render");
        assert!(matches!(initial.decision, SurfacingDecision::Render { .. }));

        let mut changed = input("claim-evidence-signal");
        changed.evidence_signature_changed = true;
        changed.source_signal_id = Some("signal-evidence-change-1".to_string());
        let resurfaced = evaluate_surfacing_for_claim(&ctx, &db, &engine, changed.clone())
            .expect("evidence signal bypasses cooldown");
        assert!(matches!(
            resurfaced.decision,
            SurfacingDecision::Render { .. }
        ));

        let repeated = evaluate_surfacing_for_claim(&ctx, &db, &engine, changed)
            .expect("same evidence signal is consumed");
        assert!(matches!(
            repeated.decision,
            SurfacingDecision::Defer {
                reason: DeferReason::CooldownActive,
                ..
            }
        ));
    }

    #[test]
    fn source_signal_id_alone_does_not_override_recent_feedback() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-feedback", 0.95, "2026-05-26T10:00:00Z");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        db.conn_ref()
            .execute(
                "INSERT INTO claim_feedback (
                     id, claim_id, feedback_type, actor, actor_id, submitted_at
                 ) VALUES (
                     'feedback-suppress-1', 'claim-feedback', 'not_relevant_here',
                     'user', 'user-1', '2026-05-26T11:00:00Z'
                 )",
                [],
            )
            .expect("insert feedback");
        let engine = PropagationEngine::new();
        let mut request = input("claim-feedback");
        request.source_signal_id = Some("new-source-signal-id".to_string());

        let result = evaluate_surfacing_for_claim(&ctx, &db, &engine, request)
            .expect("feedback suppression");

        assert!(matches!(
            result.decision,
            SurfacingDecision::Suppress {
                reason: SuppressReason::DismissedRecently
            }
        ));
    }

    #[test]
    fn evaluate_mode_does_not_write_or_emit() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-evaluate", 0.95, "2026-05-26T10:00:00Z");
        let (clock, rng, _external) = test_ctx();
        let ctx = ServiceContext::new_evaluate_default(&clock, &rng).with_actor("system:test");
        let engine = PropagationEngine::new();

        let result = evaluate_surfacing_for_claim(&ctx, &db, &engine, input("claim-evaluate"))
            .expect("evaluate mode returns preview decision");

        assert!(result.surfacing_decision_id.is_none());
        assert!(result.signal.is_none());
        let decision_rows: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM surfacing_decisions", [], |row| {
                row.get(0)
            })
            .expect("count decisions");
        let signal_rows: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE signal_type = 'surfacing_decision_made'",
                [],
                |row| row.get(0),
            )
            .expect("count signals");
        assert_eq!(decision_rows, 0);
        assert_eq!(signal_rows, 0);
    }

    #[test]
    fn pure_decision_bounds_critical_primary_to_one_per_day() {
        let now = "2026-05-26T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let candidate = SurfacingCandidate {
            claim_id: ClaimId("claim-critical".to_string()),
            claim_type: "recommendation".to_string(),
            sensitivity: "internal".to_string(),
            subject_kind: "account".to_string(),
            subject_id: "acct-example".to_string(),
            action_signature: "reviewClaim".to_string(),
            source_asof: Some(now),
            evidence_signature: Some("evsig_test".to_string()),
            salience: SalienceScore {
                total: 0.97,
                factors: vec![SalienceFactor {
                    kind: SalienceFactorKind::Urgency,
                    value: Some(0.97),
                    weight: 0.15,
                    rationale: FactorRationale::Urgency {
                        deadline: None,
                        decay_factor: 0.97,
                    },
                }],
            },
            trigger_refs: Vec::new(),
            surface_class: SurfaceClass::Primary,
            material_new_evidence: true,
        };

        let decision = decide_surfacing(
            candidate,
            &SurfacingPolicy::default(),
            SurfacingPolicyState {
                critical_primary_used: 1,
                ..SurfacingPolicyState::default()
            },
            now,
        );

        assert!(matches!(
            decision,
            SurfacingDecision::Defer {
                reason: DeferReason::BudgetExhausted,
                ..
            }
        ));
    }
}

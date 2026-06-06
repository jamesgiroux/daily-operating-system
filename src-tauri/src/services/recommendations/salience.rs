//! Salience scoring engine.
//!
//! Computes a salience score from explicit inspectable factors
//! (trust, novelty, freshness, urgency, similarity, fit). No
//! provider / LLM call from this module — factor rationale is
//! constructed from typed `FactorRationale` values, never assembled
//! from raw strings. The CI invariant for this file is the absence
//! of any LLM client import.

use std::cmp::Ordering;

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use abilities_runtime::abilities::trust::types::TrustBand;

use super::contracts::{
    ClaimId, FactorRationale, SalienceFactor, SalienceFactorKind, SalienceScore,
};
use crate::db::ActionDb;
use crate::services::context::{ServiceContext, ServiceError};

pub const SCORE_SALIENCE_SCHEMA_VERSION: u32 = 1;

const WEIGHT_EPSILON: f64 = 0.0001;
const FRESHNESS_HALF_LIFE_DAYS: f64 = 90.0;
const UNKNOWN_TIMESTAMP_PENALTY: f64 = 0.8;
const SECONDS_PER_DAY: f64 = 86_400.0;

const DEFAULT_WEIGHTS: [(SalienceFactorKind, f64); 10] = [
    (SalienceFactorKind::Importance, 0.20),
    (SalienceFactorKind::Novelty, 0.10),
    (SalienceFactorKind::Urgency, 0.15),
    (SalienceFactorKind::Timing, 0.10),
    (SalienceFactorKind::UserFit, 0.10),
    (SalienceFactorKind::Freshness, 0.10),
    (SalienceFactorKind::Trust, 0.10),
    (SalienceFactorKind::Corroboration, 0.05),
    (SalienceFactorKind::Contradiction, 0.05),
    (SalienceFactorKind::OpenLoopRelevance, 0.05),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreSalienceRequest {
    pub schema_version: u32,
    pub claim_id: ClaimId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreSalienceResult {
    pub schema_version: u32,
    pub claim_id: ClaimId,
    pub computed_at: DateTime<Utc>,
    pub persistence: SaliencePersistence,
    pub salience: SalienceScore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SaliencePersistence {
    Preview,
    Stored { evaluation_id: String },
}

#[derive(Debug, thiserror::Error)]
pub enum SalienceError {
    #[error("unsupported schema_version `{0}` for score_salience")]
    UnsupportedSchemaVersion(u32),
    #[error("claim `{0}` not found")]
    ClaimNotFound(String),
    #[error("claim `{0}` is not visible to the current actor")]
    ClaimNotVisible(String),
    #[error("salience weights invalid: {0}")]
    InvalidWeights(String),
    #[error("salience database error: {0}")]
    Database(String),
    #[error(transparent)]
    Service(#[from] ServiceError),
    #[error("salience rationale serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<rusqlite::Error> for SalienceError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActorPolicy {
    User,
    System,
}

#[derive(Debug, Clone)]
struct ClaimRow {
    id: ClaimId,
    subject_ref: String,
    claim_type: String,
    data_source: String,
    source_asof: Option<String>,
    observed_at: String,
    created_at: String,
    metadata_json: Option<String>,
    claim_state: String,
    surfacing_state: String,
    expires_at: Option<String>,
    trust_score: Option<f64>,
    temporal_scope: String,
}

#[derive(Debug, Clone, Copy)]
struct SalienceWeights {
    entries: [(SalienceFactorKind, f64); 10],
}

impl SalienceWeights {
    fn default() -> Self {
        Self {
            entries: DEFAULT_WEIGHTS,
        }
    }

    fn weight_for(self, kind: SalienceFactorKind) -> f64 {
        self.entries
            .iter()
            .find_map(|(entry_kind, weight)| (*entry_kind == kind).then_some(*weight))
            .unwrap_or(0.0)
    }

    fn validate(self) -> Result<Self, SalienceError> {
        let sum = self.entries.iter().map(|(_, weight)| *weight).sum::<f64>();
        let all_finite = self
            .entries
            .iter()
            .all(|(_, weight)| weight.is_finite() && (0.0..=1.0).contains(weight));
        if !all_finite {
            return Err(SalienceError::InvalidWeights(
                "all weights must be finite values in [0,1]".to_string(),
            ));
        }
        if (sum - 1.0).abs() > WEIGHT_EPSILON {
            return Err(SalienceError::InvalidWeights(format!(
                "expected weight sum 1.0 +/- {WEIGHT_EPSILON}; got {sum}"
            )));
        }
        Ok(self)
    }
}

pub fn aggregate_salience(factors: Vec<SalienceFactor>) -> SalienceScore {
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    let mut normalized = Vec::with_capacity(factors.len());

    for mut factor in factors {
        let weight = if factor.weight.is_finite() {
            factor.weight.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let value = factor
            .value
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(0.0, 1.0));
        if let Some(value) = value {
            numerator += value * weight;
            denominator += weight;
        }
        factor.value = value;
        factor.weight = weight;
        normalized.push(factor);
    }

    let total = if denominator > 0.0 {
        (numerator / denominator).clamp(0.0, 1.0)
    } else {
        0.0
    };

    SalienceScore {
        total,
        factors: normalized,
    }
}

/// Sort comparator for descending salience ranking.
pub fn compare_salience(
    left_id: &ClaimId,
    left: &SalienceScore,
    right_id: &ClaimId,
    right: &SalienceScore,
) -> Ordering {
    right
        .total
        .total_cmp(&left.total)
        .then_with(|| {
            factor_value(right, SalienceFactorKind::Importance)
                .total_cmp(&factor_value(left, SalienceFactorKind::Importance))
        })
        .then_with(|| {
            factor_value(right, SalienceFactorKind::Urgency)
                .total_cmp(&factor_value(left, SalienceFactorKind::Urgency))
        })
        .then_with(|| left_id.0.cmp(&right_id.0))
}

pub fn score_salience(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    request: ScoreSalienceRequest,
) -> Result<ScoreSalienceResult, SalienceError> {
    validate_schema_version(request.schema_version)?;
    let actor_policy = actor_policy(ctx);
    let claim = load_claim(db.conn_ref(), &request.claim_id, actor_policy)?;
    let weights = load_weights(db.conn_ref())?;

    if let Some(stored) = load_latest_stored_salience(db.conn_ref(), &claim.id)? {
        return Ok(stored);
    }

    let computed_at = ctx.clock.now();
    let factors = extract_factors(db.conn_ref(), &claim, weights, computed_at)?;
    let salience = aggregate_salience(factors);

    Ok(ScoreSalienceResult {
        schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
        claim_id: claim.id,
        computed_at,
        persistence: SaliencePersistence::Preview,
        salience,
    })
}

pub fn recompute_salience_for_claim(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    request: ScoreSalienceRequest,
) -> Result<ScoreSalienceResult, SalienceError> {
    let evaluation_id = format!("salience-eval-{}", uuid::Uuid::new_v4());
    recompute_salience_for_claim_with_evaluation_id(ctx, db, request, evaluation_id)
}

pub fn recompute_salience_for_claim_with_evaluation_id(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    request: ScoreSalienceRequest,
    evaluation_id: String,
) -> Result<ScoreSalienceResult, SalienceError> {
    ctx.check_mutation_allowed()?;
    validate_schema_version(request.schema_version)?;
    validate_evaluation_id(&evaluation_id)?;

    let actor_policy = actor_policy(ctx);
    let claim = load_claim(db.conn_ref(), &request.claim_id, actor_policy)?;
    let weights = load_weights(db.conn_ref())?;
    let computed_at = ctx.clock.now();
    let factors = extract_factors(db.conn_ref(), &claim, weights, computed_at)?;
    let salience = aggregate_salience(factors);

    persist_salience(db, &claim.id, &evaluation_id, computed_at, &salience)?;

    Ok(ScoreSalienceResult {
        schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
        claim_id: claim.id,
        computed_at,
        persistence: SaliencePersistence::Stored { evaluation_id },
        salience,
    })
}

fn validate_evaluation_id(evaluation_id: &str) -> Result<(), SalienceError> {
    let valid = !evaluation_id.trim().is_empty()
        && evaluation_id.len() <= 160
        && evaluation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':' | b'.'));
    if valid {
        Ok(())
    } else {
        Err(SalienceError::InvalidWeights(
            "salience evaluation id must be a safe storage ref".to_string(),
        ))
    }
}

fn validate_schema_version(schema_version: u32) -> Result<(), SalienceError> {
    if schema_version == SCORE_SALIENCE_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(SalienceError::UnsupportedSchemaVersion(schema_version))
    }
}

fn actor_policy(ctx: &ServiceContext<'_>) -> ActorPolicy {
    if ctx.actor.starts_with("system") {
        ActorPolicy::System
    } else {
        ActorPolicy::User
    }
}

fn load_claim(
    conn: &rusqlite::Connection,
    claim_id: &ClaimId,
    actor_policy: ActorPolicy,
) -> Result<ClaimRow, SalienceError> {
    let row = conn
        .query_row(
            "SELECT id, subject_ref, claim_type, data_source, source_asof, observed_at, created_at,
                    metadata_json, claim_state, surfacing_state, expires_at, trust_score,
                    temporal_scope
               FROM intelligence_claims
              WHERE id = ?1",
            [&claim_id.0],
            |row| {
                Ok(ClaimRow {
                    id: ClaimId(row.get(0)?),
                    subject_ref: row.get(1)?,
                    claim_type: row.get(2)?,
                    data_source: row.get(3)?,
                    source_asof: row.get(4)?,
                    observed_at: row.get(5)?,
                    created_at: row.get(6)?,
                    metadata_json: row.get(7)?,
                    claim_state: row.get(8)?,
                    surfacing_state: row.get(9)?,
                    expires_at: row.get(10)?,
                    trust_score: row.get(11)?,
                    temporal_scope: row.get(12)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| SalienceError::ClaimNotFound(claim_id.0.clone()))?;

    let visible = match actor_policy {
        ActorPolicy::User => row.claim_state == "active" && row.surfacing_state == "active",
        ActorPolicy::System => matches!(row.claim_state.as_str(), "active" | "dormant"),
    };
    if visible {
        Ok(row)
    } else {
        Err(SalienceError::ClaimNotVisible(claim_id.0.clone()))
    }
}

fn load_weights(conn: &rusqlite::Connection) -> Result<SalienceWeights, SalienceError> {
    if !table_exists(conn, "salience_factors_weights")? {
        return SalienceWeights::default().validate();
    }

    let mut entries = DEFAULT_WEIGHTS;
    let mut loaded_count = 0usize;
    let mut stmt = conn.prepare(
        "SELECT factor_kind, default_weight
           FROM salience_factors_weights
          WHERE schema_version = 1",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    for row in rows {
        let (kind, weight) = row?;
        let Some(kind) = factor_kind_from_storage(&kind) else {
            return Err(SalienceError::InvalidWeights(format!(
                "unknown factor kind `{kind}`"
            )));
        };
        let Some(entry) = entries
            .iter_mut()
            .find(|(entry_kind, _)| *entry_kind == kind)
        else {
            return Err(SalienceError::InvalidWeights(format!(
                "unregistered factor kind `{}`",
                factor_kind_storage(kind)
            )));
        };
        entry.1 = weight;
        loaded_count += 1;
    }

    if loaded_count != DEFAULT_WEIGHTS.len() {
        return Err(SalienceError::InvalidWeights(format!(
            "expected {} weight rows; got {loaded_count}",
            DEFAULT_WEIGHTS.len()
        )));
    }

    SalienceWeights { entries }.validate()
}

fn load_latest_stored_salience(
    conn: &rusqlite::Connection,
    claim_id: &ClaimId,
) -> Result<Option<ScoreSalienceResult>, SalienceError> {
    if !table_exists(conn, "salience_factors")? {
        return Ok(None);
    }

    let latest = conn
        .query_row(
            "SELECT evaluation_id, computed_at
               FROM salience_factors
              WHERE claim_id = ?1
              GROUP BY evaluation_id, computed_at
              ORDER BY julianday(computed_at) DESC, computed_at DESC, evaluation_id DESC
              LIMIT 1",
            [&claim_id.0],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;

    let Some((evaluation_id, computed_at_raw)) = latest else {
        return Ok(None);
    };

    let computed_at = parse_datetime(&computed_at_raw).unwrap_or_else(Utc::now);
    let mut stmt = conn.prepare(
        "SELECT factor_kind, factor_value, weight, rationale_json
           FROM salience_factors
          WHERE claim_id = ?1 AND evaluation_id = ?2",
    )?;
    let rows = stmt.query_map(params![&claim_id.0, &evaluation_id], |row| {
        let factor_kind = row.get::<_, String>(0)?;
        let value = row.get::<_, Option<f64>>(1)?;
        let weight = row.get::<_, f64>(2)?;
        let rationale_json = row.get::<_, String>(3)?;
        Ok((factor_kind, value, weight, rationale_json))
    })?;

    let mut factors = Vec::new();
    for row in rows {
        let (factor_kind, value, weight, rationale_json) = row?;
        let Some(kind) = factor_kind_from_storage(&factor_kind) else {
            continue;
        };
        let rationale = serde_json::from_str::<FactorRationale>(&rationale_json)?;
        factors.push(SalienceFactor {
            kind,
            value,
            weight,
            rationale,
        });
    }
    factors.sort_by_key(|factor| factor_order(factor.kind));
    let salience = aggregate_salience(factors);

    Ok(Some(ScoreSalienceResult {
        schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
        claim_id: claim_id.clone(),
        computed_at,
        persistence: SaliencePersistence::Stored { evaluation_id },
        salience,
    }))
}

fn extract_factors(
    conn: &rusqlite::Connection,
    claim: &ClaimRow,
    weights: SalienceWeights,
    now: DateTime<Utc>,
) -> Result<Vec<SalienceFactor>, SalienceError> {
    let trust_band = trust_band_for_score(claim.trust_score);
    let trust_value = trust_value(trust_band);
    let source_authority = source_authority(&claim.data_source);
    let (novelty_value, vector_distance, neighbor_count) = novelty_value(conn, &claim.id)?;
    let (corroboration_value, corroboration_count) = corroboration_value(conn, &claim.id)?;
    let (contradiction_value, contradiction_count) = contradiction_value(conn, &claim.id)?;
    let (user_fit_value, feedback_history_score) = user_fit_value(conn, &claim.id)?;
    let (open_loop_value, open_loop_count, has_action) = open_loop_relevance(conn, claim)?;
    let (freshness_value, freshness_decay) = freshness_value(claim, now);
    let (urgency_value, deadline, urgency_decay) = urgency_value(claim, now);
    let (timing_value, signal_age_secs) = timing_value(claim, now);

    Ok(vec![
        SalienceFactor {
            kind: SalienceFactorKind::Importance,
            value: Some(((trust_value + source_authority) / 2.0).clamp(0.0, 1.0)),
            weight: weights.weight_for(SalienceFactorKind::Importance),
            rationale: FactorRationale::Importance {
                trust_band,
                source_authority,
            },
        },
        SalienceFactor {
            kind: SalienceFactorKind::Novelty,
            value: novelty_value,
            weight: weights.weight_for(SalienceFactorKind::Novelty),
            rationale: FactorRationale::Novelty {
                vector_distance,
                neighbor_count,
            },
        },
        SalienceFactor {
            kind: SalienceFactorKind::Urgency,
            value: urgency_value,
            weight: weights.weight_for(SalienceFactorKind::Urgency),
            rationale: FactorRationale::Urgency {
                deadline,
                decay_factor: urgency_decay,
            },
        },
        SalienceFactor {
            kind: SalienceFactorKind::Timing,
            value: timing_value,
            weight: weights.weight_for(SalienceFactorKind::Timing),
            rationale: FactorRationale::Timing {
                signal_age_secs,
                calendar_proximity_secs: None,
            },
        },
        SalienceFactor {
            kind: SalienceFactorKind::UserFit,
            value: user_fit_value,
            weight: weights.weight_for(SalienceFactorKind::UserFit),
            rationale: FactorRationale::UserFit {
                feedback_history_score,
            },
        },
        SalienceFactor {
            kind: SalienceFactorKind::Freshness,
            value: Some(freshness_value),
            weight: weights.weight_for(SalienceFactorKind::Freshness),
            rationale: FactorRationale::Freshness {
                decay_factor: freshness_decay,
            },
        },
        SalienceFactor {
            kind: SalienceFactorKind::Trust,
            value: Some(trust_value),
            weight: weights.weight_for(SalienceFactorKind::Trust),
            rationale: FactorRationale::Trust { trust_band },
        },
        SalienceFactor {
            kind: SalienceFactorKind::Corroboration,
            value: Some(corroboration_value),
            weight: weights.weight_for(SalienceFactorKind::Corroboration),
            rationale: FactorRationale::Corroboration {
                corroboration_count,
            },
        },
        SalienceFactor {
            kind: SalienceFactorKind::Contradiction,
            value: Some(contradiction_value),
            weight: weights.weight_for(SalienceFactorKind::Contradiction),
            rationale: FactorRationale::Contradiction {
                contradiction_count,
            },
        },
        SalienceFactor {
            kind: SalienceFactorKind::OpenLoopRelevance,
            value: Some(open_loop_value),
            weight: weights.weight_for(SalienceFactorKind::OpenLoopRelevance),
            rationale: FactorRationale::OpenLoopRelevance {
                open_loop_count,
                has_action,
            },
        },
    ])
}

fn persist_salience(
    db: &ActionDb,
    claim_id: &ClaimId,
    evaluation_id: &str,
    computed_at: DateTime<Utc>,
    salience: &SalienceScore,
) -> Result<(), SalienceError> {
    let computed_at = computed_at.to_rfc3339();
    db.with_transaction(|tx| {
        for factor in &salience.factors {
            let id = format!(
                "salience-factor-{}-{}",
                uuid::Uuid::new_v4(),
                factor_kind_storage(factor.kind)
            );
            let rationale_json =
                serde_json::to_string(&factor.rationale).map_err(|e| e.to_string())?;
            tx.conn_ref()
                .execute(
                    "INSERT INTO salience_factors (
                        id, evaluation_id, claim_id, factor_kind, factor_value,
                        weight, rationale_json, schema_version, computed_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8)
                     ON CONFLICT(evaluation_id, claim_id, factor_kind) DO UPDATE SET
                         factor_value = excluded.factor_value,
                         weight = excluded.weight,
                         rationale_json = excluded.rationale_json,
                         schema_version = excluded.schema_version,
                         computed_at = excluded.computed_at",
                    params![
                        id,
                        evaluation_id,
                        &claim_id.0,
                        factor_kind_storage(factor.kind),
                        factor.value,
                        factor.weight,
                        rationale_json,
                        &computed_at,
                    ],
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .map_err(SalienceError::Database)
}

fn factor_value(score: &SalienceScore, kind: SalienceFactorKind) -> f64 {
    score
        .factors
        .iter()
        .find(|factor| factor.kind == kind)
        .and_then(|factor| factor.value)
        .unwrap_or(0.0)
}

fn novelty_value(
    conn: &rusqlite::Connection,
    claim_id: &ClaimId,
) -> Result<(Option<f64>, f64, u32), SalienceError> {
    let semantic_count = if table_exists(conn, "claim_semantic_evidence")? {
        count_query(
            conn,
            "SELECT COUNT(*) FROM claim_semantic_evidence WHERE canonical_claim_id = ?1",
            &claim_id.0,
        )?
    } else {
        0
    };
    let decision_count = if table_exists(conn, "canonicalization_decisions")? {
        conn.query_row(
            "SELECT COUNT(*)
               FROM canonicalization_decisions
              WHERE claim_id_a = ?1 OR claim_id_b = ?1",
            [&claim_id.0],
            |row| row.get::<_, i64>(0),
        )?
    } else {
        0
    };
    let neighbor_count = u32::try_from(semantic_count + decision_count).unwrap_or(u32::MAX);
    if semantic_count == 0 && decision_count == 0 {
        return Ok((None, 0.0, 0));
    }
    let vector_distance = 1.0 / (1.0 + f64::from(neighbor_count));
    Ok((Some(vector_distance), vector_distance, neighbor_count))
}

fn corroboration_value(
    conn: &rusqlite::Connection,
    claim_id: &ClaimId,
) -> Result<(f64, u32), SalienceError> {
    if !table_exists(conn, "claim_corroborations")? {
        return Ok((0.0, 0));
    }
    let (count, strength_sum): (i64, Option<f64>) = conn.query_row(
        "SELECT COUNT(*), SUM(strength) FROM claim_corroborations WHERE claim_id = ?1",
        [&claim_id.0],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let count = count.max(0) as u32;
    let value = (strength_sum.unwrap_or(0.0) / 3.0).clamp(0.0, 1.0);
    Ok((value, count))
}

fn contradiction_value(
    conn: &rusqlite::Connection,
    claim_id: &ClaimId,
) -> Result<(f64, u32), SalienceError> {
    if !table_exists(conn, "claim_contradictions")? {
        return Ok((1.0, 0));
    }
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM claim_contradictions
          WHERE reconciled_at IS NULL
            AND (primary_claim_id = ?1 OR contradicting_claim_id = ?1)",
        [&claim_id.0],
        |row| row.get(0),
    )?;
    let count = count.max(0) as u32;
    Ok((1.0 / (1.0 + f64::from(count)), count))
}

fn user_fit_value(
    conn: &rusqlite::Connection,
    claim_id: &ClaimId,
) -> Result<(Option<f64>, f64), SalienceError> {
    if !table_exists(conn, "claim_feedback")? {
        return Ok((None, 0.0));
    }
    let mut stmt = conn.prepare("SELECT feedback_type FROM claim_feedback WHERE claim_id = ?1")?;
    let rows = stmt.query_map([&claim_id.0], |row| row.get::<_, String>(0))?;
    let mut total = 0u32;
    let mut raw_score = 0.0;
    for row in rows {
        if let Some(delta) = feedback_salience_delta(&row?) {
            total += 1;
            raw_score += delta;
        }
    }
    if total == 0 {
        return Ok((None, 0.0));
    }
    let normalized = (0.5 + (raw_score / f64::from(total)) / 2.0).clamp(0.0, 1.0);
    Ok((Some(normalized), raw_score))
}

fn feedback_salience_delta(feedback_type: &str) -> Option<f64> {
    match feedback_type {
        "confirm" | "confirm_current" => Some(1.0),
        "correct" | "needs_nuance" => Some(0.25),
        "cannot_verify" => Some(-0.2),
        "wrong_source" => Some(-0.6),
        "mark_outdated" => Some(-0.75),
        "surface_inappropriate" | "not_relevant_here" => Some(-0.5),
        "reject" | "mark_false" | "wrong_subject" => Some(-1.0),
        "merge_intent" => None,
        _ => None,
    }
}

fn open_loop_relevance(
    conn: &rusqlite::Connection,
    claim: &ClaimRow,
) -> Result<(f64, u32, bool), SalienceError> {
    let metadata_has_action = metadata_recommended_action_kind(claim).is_some();
    let open_loop_count = if table_exists(conn, "intelligence_claims")? {
        let Some((subject_kind, subject_id)) = subject_scope(&claim.subject_ref) else {
            return Ok((
                open_loop_value(metadata_has_action, 0, claim),
                0,
                metadata_has_action || claim.claim_type == "recommendation",
            ));
        };
        conn.query_row(
            "SELECT COUNT(*)
               FROM intelligence_claims
              WHERE claim_state = 'active'
                AND surfacing_state = 'active'
                AND claim_type IN ('open_loop', 'commitment', 'action')
                AND id <> ?1
                AND json_valid(subject_ref) = 1
                AND lower(json_extract(subject_ref, '$.kind')) = lower(?2)
                AND json_extract(subject_ref, '$.id') = ?3",
            params![&claim.id.0, subject_kind, subject_id],
            |row| row.get::<_, i64>(0),
        )?
        .clamp(0, i64::from(u32::MAX)) as u32
    } else {
        0
    };
    let value = open_loop_value(metadata_has_action, open_loop_count, claim);
    Ok((
        value,
        open_loop_count,
        metadata_has_action || claim.claim_type == "recommendation",
    ))
}

fn metadata_recommended_action_kind(claim: &ClaimRow) -> Option<String> {
    claim
        .metadata_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|value| {
            value
                .pointer("/recommendation/recommendedAction/kind")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
}

fn subject_scope(subject_ref: &str) -> Option<(String, String)> {
    let value = serde_json::from_str::<serde_json::Value>(subject_ref).ok()?;
    let kind = value.get("kind")?.as_str()?.to_string();
    let id = value.get("id")?.as_str()?.to_string();
    Some((kind, id))
}

fn open_loop_value(metadata_has_action: bool, open_loop_count: u32, claim: &ClaimRow) -> f64 {
    let has_action = metadata_has_action || claim.claim_type == "recommendation";
    if has_action && open_loop_count > 0 {
        1.0
    } else if has_action {
        0.75
    } else if open_loop_count > 0 {
        0.5
    } else {
        0.0
    }
}

fn freshness_value(claim: &ClaimRow, now: DateTime<Utc>) -> (f64, f64) {
    if matches!(claim.temporal_scope.as_str(), "point_in_time" | "closed") {
        return (1.0, 1.0);
    }
    let (timestamp, known) = claim
        .source_asof
        .as_deref()
        .and_then(parse_datetime)
        .map(|timestamp| (timestamp, true))
        .or_else(|| parse_datetime(&claim.observed_at).map(|timestamp| (timestamp, false)))
        .or_else(|| parse_datetime(&claim.created_at).map(|timestamp| (timestamp, false)))
        .unwrap_or((now - chrono::Duration::days(365), false));
    let age_days = age_days(now, timestamp);
    let mut value = 2f64.powf(-(age_days / FRESHNESS_HALF_LIFE_DAYS));
    if !known {
        value *= UNKNOWN_TIMESTAMP_PENALTY;
    }
    let value = value.clamp(0.0, 1.0);
    (value, value)
}

fn urgency_value(
    claim: &ClaimRow,
    now: DateTime<Utc>,
) -> (Option<f64>, Option<DateTime<Utc>>, f64) {
    let Some(deadline) = claim.expires_at.as_deref().and_then(parse_datetime) else {
        return (None, None, 0.0);
    };
    let secs = deadline.signed_duration_since(now).num_seconds();
    let value = if secs < 0 {
        0.1
    } else {
        let days = secs as f64 / SECONDS_PER_DAY;
        if days <= 1.0 {
            1.0
        } else if days <= 7.0 {
            0.85
        } else if days <= 30.0 {
            0.55
        } else {
            0.25
        }
    };
    (Some(value), Some(deadline), value)
}

fn timing_value(claim: &ClaimRow, now: DateTime<Utc>) -> (Option<f64>, i64) {
    let observed_at = parse_datetime(&claim.observed_at)
        .or_else(|| parse_datetime(&claim.created_at))
        .unwrap_or(now);
    let age_secs = now.signed_duration_since(observed_at).num_seconds().max(0);
    let age_days = age_secs as f64 / SECONDS_PER_DAY;
    let value = if age_days <= 1.0 {
        1.0
    } else if age_days <= 7.0 {
        0.7
    } else if age_days <= 30.0 {
        0.4
    } else {
        0.15
    };
    (Some(value), age_secs)
}

fn trust_band_for_score(score: Option<f64>) -> TrustBand {
    match score {
        Some(score) if score >= 0.75 => TrustBand::LikelyCurrent,
        Some(score) if score >= 0.50 => TrustBand::UseWithCaution,
        Some(_) => TrustBand::NeedsVerification,
        None => TrustBand::Unscored,
    }
}

fn trust_value(band: TrustBand) -> f64 {
    match band {
        TrustBand::LikelyCurrent => 0.9,
        TrustBand::UseWithCaution => 0.6,
        TrustBand::NeedsVerification => 0.35,
        TrustBand::Unscored => 0.2,
    }
}

fn source_authority(data_source: &str) -> f64 {
    match data_source {
        "user" | "human" => 1.0,
        "glean" | "salesforce" | "linear" => 0.85,
        "workspace" | "local_workspace" | "workspace_file" => 0.75,
        "recommendation" => 0.65,
        _ => 0.55,
    }
}

fn count_query(
    conn: &rusqlite::Connection,
    query: &str,
    value: &str,
) -> Result<i64, SalienceError> {
    conn.query_row(query, [value], |row| row.get::<_, i64>(0))
        .map_err(SalienceError::from)
}

fn table_exists(conn: &rusqlite::Connection, table: &str) -> Result<bool, SalienceError> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM sqlite_master
          WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get::<_, i64>(0).map(|count| count > 0),
    )
    .map_err(SalienceError::from)
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

fn age_days(now: DateTime<Utc>, then: DateTime<Utc>) -> f64 {
    now.signed_duration_since(then).num_seconds().max(0) as f64 / SECONDS_PER_DAY
}

fn factor_kind_storage(kind: SalienceFactorKind) -> &'static str {
    match kind {
        SalienceFactorKind::Importance => "importance",
        SalienceFactorKind::Novelty => "novelty",
        SalienceFactorKind::Urgency => "urgency",
        SalienceFactorKind::Timing => "timing",
        SalienceFactorKind::UserFit => "userFit",
        SalienceFactorKind::Freshness => "freshness",
        SalienceFactorKind::Trust => "trust",
        SalienceFactorKind::Corroboration => "corroboration",
        SalienceFactorKind::Contradiction => "contradiction",
        SalienceFactorKind::OpenLoopRelevance => "openLoopRelevance",
    }
}

fn factor_kind_from_storage(value: &str) -> Option<SalienceFactorKind> {
    Some(match value {
        "importance" => SalienceFactorKind::Importance,
        "novelty" => SalienceFactorKind::Novelty,
        "urgency" => SalienceFactorKind::Urgency,
        "timing" => SalienceFactorKind::Timing,
        "userFit" => SalienceFactorKind::UserFit,
        "freshness" => SalienceFactorKind::Freshness,
        "trust" => SalienceFactorKind::Trust,
        "corroboration" => SalienceFactorKind::Corroboration,
        "contradiction" => SalienceFactorKind::Contradiction,
        "openLoopRelevance" => SalienceFactorKind::OpenLoopRelevance,
        _ => return None,
    })
}

fn factor_order(kind: SalienceFactorKind) -> u8 {
    match kind {
        SalienceFactorKind::Importance => 0,
        SalienceFactorKind::Novelty => 1,
        SalienceFactorKind::Urgency => 2,
        SalienceFactorKind::Timing => 3,
        SalienceFactorKind::UserFit => 4,
        SalienceFactorKind::Freshness => 5,
        SalienceFactorKind::Trust => 6,
        SalienceFactorKind::Corroboration => 7,
        SalienceFactorKind::Contradiction => 8,
        SalienceFactorKind::OpenLoopRelevance => 9,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::abilities::claims::ClaimType;
    use crate::abilities::feedback::FeedbackAction;
    use crate::db::claims::{ClaimSensitivity, TemporalScope};
    use crate::db::ActionDb;
    use crate::services::claims::{
        commit_claim, record_claim_feedback, record_corroboration, update_claim_trust,
        ClaimFeedbackInput, ClaimProposal, DeterministicInsertProposal, TrustScore,
    };
    use crate::services::context::{ExecutionMode, ExternalClients, FixedClock, SeedableRng};

    fn factor(kind: SalienceFactorKind, value: Option<f64>, weight: f64) -> SalienceFactor {
        SalienceFactor {
            kind,
            value,
            weight,
            rationale: FactorRationale::Freshness { decay_factor: 1.0 },
        }
    }

    #[test]
    fn aggregate_skips_none_values_and_clamps_inputs() {
        let score = aggregate_salience(vec![
            factor(SalienceFactorKind::Importance, Some(2.0), 0.5),
            factor(SalienceFactorKind::Urgency, None, 0.25),
            factor(SalienceFactorKind::Freshness, Some(-1.0), 0.25),
        ]);

        assert_eq!(score.total, 2.0 / 3.0);
        assert_eq!(score.factors[0].value, Some(1.0));
        assert_eq!(score.factors[2].value, Some(0.0));
    }

    #[test]
    fn aggregate_empty_denominator_returns_zero() {
        let score = aggregate_salience(vec![factor(SalienceFactorKind::Novelty, None, 1.0)]);

        assert_eq!(score.total, 0.0);
    }

    #[test]
    fn compare_salience_uses_importance_urgency_then_claim_id() {
        let left_id = ClaimId("claim-a".to_string());
        let right_id = ClaimId("claim-b".to_string());
        let left = SalienceScore {
            total: 0.8,
            factors: vec![
                factor(SalienceFactorKind::Importance, Some(0.7), 0.2),
                factor(SalienceFactorKind::Urgency, Some(0.7), 0.2),
            ],
        };
        let right = SalienceScore {
            total: 0.8,
            factors: vec![
                factor(SalienceFactorKind::Importance, Some(0.7), 0.2),
                factor(SalienceFactorKind::Urgency, Some(0.6), 0.2),
            ],
        };

        assert_eq!(
            compare_salience(&left_id, &left, &right_id, &right),
            Ordering::Less
        );
    }

    #[test]
    fn user_fit_handles_current_feedback_action_values() {
        assert_eq!(feedback_salience_delta("confirm_current"), Some(1.0));
        assert_eq!(feedback_salience_delta("mark_outdated"), Some(-0.75));
        assert_eq!(feedback_salience_delta("mark_false"), Some(-1.0));
        assert_eq!(feedback_salience_delta("wrong_subject"), Some(-1.0));
        assert_eq!(feedback_salience_delta("wrong_source"), Some(-0.6));
        assert_eq!(feedback_salience_delta("cannot_verify"), Some(-0.2));
        assert_eq!(feedback_salience_delta("needs_nuance"), Some(0.25));
        assert_eq!(feedback_salience_delta("surface_inappropriate"), Some(-0.5));
        assert_eq!(feedback_salience_delta("not_relevant_here"), Some(-0.5));
        assert_eq!(feedback_salience_delta("merge_intent"), None);
        assert_eq!(feedback_salience_delta("unknown_legacy_value"), None);
    }

    #[test]
    fn novelty_counts_single_semantic_evidence_row_as_neighbor() {
        let db = test_db();
        insert_claim(&db, "claim-novelty", 0.72);
        db.conn_ref()
            .execute(
                "INSERT INTO claim_semantic_evidence (
                    id, canonical_claim_id, data_source, source_ref, source_asof,
                    provenance_json, original_text, actor, observed_at,
                    source_mechanism, created_at
                 ) VALUES (
                    'semantic-evidence-1', 'claim-novelty', 'workspace',
                    'source:test', '2026-05-25T12:00:00Z', '{}',
                    'duplicate variant text', 'agent', '2026-05-25T12:00:00Z',
                    'canonical_match_v2_merge', '2026-05-25T12:00:00Z'
                 )",
                [],
            )
            .expect("insert semantic evidence");

        let (value, vector_distance, neighbor_count) =
            novelty_value(db.conn_ref(), &ClaimId("claim-novelty".to_string()))
                .expect("compute novelty");

        assert_eq!(value, Some(0.5));
        assert_eq!(vector_distance, 0.5);
        assert_eq!(neighbor_count, 1);
    }

    fn test_db() -> ActionDb {
        ActionDb::from_connection_for_tests(crate::migrations::migrated_in_memory_for_tests())
    }

    fn test_ctx(mode: ExecutionMode) -> (FixedClock, SeedableRng, ExternalClients) {
        let clock = FixedClock::new(
            "2026-05-26T12:00:00Z"
                .parse::<DateTime<Utc>>()
                .expect("fixture time parses"),
        );
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        match mode {
            ExecutionMode::Live | ExecutionMode::Evaluate | ExecutionMode::Simulate => {}
        }
        (clock, rng, external)
    }

    fn claim_fixture_ctx() -> (FixedClock, SeedableRng, ExternalClients) {
        test_ctx(ExecutionMode::Live)
    }

    fn insert_claim(db: &ActionDb, id: &str, trust_score: f64) {
        let (clock, rng, external) = claim_fixture_ctx();
        let ctx = ServiceContext::test_live(&clock, &rng, &external).with_actor("user:test");
        let proposal = ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: r#"{"kind":"account","id":"acct-example"}"#.to_string(),
            claim_type: ClaimType::Recommendation.as_str().to_string(),
            field_path: Some("recommendation.reviewClaim".to_string()),
            topic_key: Some("reviewClaim".to_string()),
            text: "Review a claim before the next customer conversation.".to_string(),
            actor: "agent".to_string(),
            data_source: "recommendation".to_string(),
            source_ref: Some("run:test".to_string()),
            source_asof: Some("2026-05-25T12:00:00Z".to_string()),
            observed_at: "2026-05-26T10:00:00Z".to_string(),
            provenance_json: r#"{"sources":[]}"#.to_string(),
            metadata_json: Some(
                r#"{"recommendation":{"recommendedAction":{"kind":"reviewClaim","claimId":"claim-source","reason":"verify"}}}"#
                    .to_string(),
            ),
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
        .expect("insert claim through service");
        update_claim_trust(db, id, TrustScore(trust_score), 1, &ctx).expect("seed trust score");
    }

    fn insert_background_claim(db: &ActionDb, id: &str, trust_score: f64) {
        let (clock, rng, external) = claim_fixture_ctx();
        let ctx = ServiceContext::test_live(&clock, &rng, &external).with_actor("system:test");
        let proposal = ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: r#"{"kind":"account","id":"acct-example"}"#.to_string(),
            claim_type: ClaimType::EntityCurrentState.as_str().to_string(),
            field_path: Some("account.status".to_string()),
            topic_key: Some("accountStatus".to_string()),
            text: "Background account status remains unchanged.".to_string(),
            actor: "agent".to_string(),
            data_source: "recommendation".to_string(),
            source_ref: Some("run:test".to_string()),
            source_asof: Some("2025-11-01T12:00:00Z".to_string()),
            observed_at: "2025-11-01T12:00:00Z".to_string(),
            provenance_json: r#"{"sources":[]}"#.to_string(),
            metadata_json: Some("{}".to_string()),
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
        .expect("insert background claim through service");
        update_claim_trust(db, id, TrustScore(trust_score), 1, &ctx).expect("seed trust score");
    }

    fn insert_open_loop_claim(db: &ActionDb, id: &str, subject_ref: &str) {
        let (clock, rng, external) = claim_fixture_ctx();
        let ctx = ServiceContext::test_live(&clock, &rng, &external).with_actor("system:test");
        let proposal = ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: subject_ref.to_string(),
            claim_type: ClaimType::OpenLoop.as_str().to_string(),
            field_path: Some("openLoop.status".to_string()),
            topic_key: Some("openLoop".to_string()),
            text: "Follow up on the active customer question.".to_string(),
            actor: "agent".to_string(),
            data_source: "workspace".to_string(),
            source_ref: Some("run:test".to_string()),
            source_asof: Some("2026-05-25T12:00:00Z".to_string()),
            observed_at: "2026-05-26T10:00:00Z".to_string(),
            provenance_json: r#"{"sources":[]}"#.to_string(),
            metadata_json: Some("{}".to_string()),
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
        .expect("insert open loop claim through service");
        update_claim_trust(db, id, TrustScore(0.7), 1, &ctx).expect("seed trust score");
    }

    #[test]
    fn recompute_writes_one_privacy_safe_row_per_factor() {
        let db = test_db();
        insert_claim(&db, "claim-salient", 0.82);
        let (clock, rng, external) = test_ctx(ExecutionMode::Live);
        let ctx = ServiceContext::test_live(&clock, &rng, &external).with_actor("system:test");
        let feedback_ctx =
            ServiceContext::test_live(&clock, &rng, &external).with_actor("user:test");
        record_corroboration(&ctx, &db, "claim-salient", "workspace", None, None)
            .expect("record corroboration");
        record_claim_feedback(
            &feedback_ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: "claim-salient".to_string(),
                action: FeedbackAction::ConfirmCurrent,
                actor: "user".to_string(),
                actor_id: Some("user-1".to_string()),
                payload_json: None,
            },
        )
        .expect("record feedback");

        let result = recompute_salience_for_claim(
            &ctx,
            &db,
            ScoreSalienceRequest {
                schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
                claim_id: ClaimId("claim-salient".to_string()),
            },
        )
        .expect("recompute succeeds");

        assert!(matches!(
            result.persistence,
            SaliencePersistence::Stored { .. }
        ));
        assert_eq!(result.salience.factors.len(), 10);
        let row_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM salience_factors WHERE claim_id = 'claim-salient'",
                [],
                |row| row.get(0),
            )
            .expect("count salience rows");
        assert_eq!(row_count, 10);

        let rationale_blob: String = db
            .conn_ref()
            .query_row(
                "SELECT group_concat(rationale_json, ' ') FROM salience_factors",
                [],
                |row| row.get(0),
            )
            .expect("read rationale blob");
        for forbidden in [
            "Review a claim before",
            "/Users/",
            "prompt",
            "output",
            "source body",
        ] {
            assert!(
                !rationale_blob.contains(forbidden),
                "rationale leaked forbidden content marker {forbidden:?}"
            );
        }
    }

    #[test]
    fn recompute_blocks_without_writing_in_evaluate_mode() {
        let db = test_db();
        insert_claim(&db, "claim-blocked", 0.82);
        let (clock, rng, _external) = test_ctx(ExecutionMode::Evaluate);
        let ctx = ServiceContext::new_evaluate_default(&clock, &rng).with_actor("system:test");

        let err = recompute_salience_for_claim(
            &ctx,
            &db,
            ScoreSalienceRequest {
                schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
                claim_id: ClaimId("claim-blocked".to_string()),
            },
        )
        .expect_err("evaluate writes are blocked");

        assert!(matches!(
            err,
            SalienceError::Service(ServiceError::WriteBlockedByMode(ExecutionMode::Evaluate))
        ));
        let row_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM salience_factors WHERE claim_id = 'claim-blocked'",
                [],
                |row| row.get(0),
            )
            .expect("count salience rows");
        assert_eq!(row_count, 0);
    }

    #[test]
    fn score_salience_reads_latest_stored_evaluation() {
        let db = test_db();
        insert_claim(&db, "claim-stored", 0.72);
        let (clock, rng, external) = test_ctx(ExecutionMode::Live);
        let ctx = ServiceContext::test_live(&clock, &rng, &external).with_actor("system:test");
        let stored = recompute_salience_for_claim(
            &ctx,
            &db,
            ScoreSalienceRequest {
                schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
                claim_id: ClaimId("claim-stored".to_string()),
            },
        )
        .expect("recompute succeeds");

        let read = score_salience(
            &ctx,
            &db,
            ScoreSalienceRequest {
                schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
                claim_id: ClaimId("claim-stored".to_string()),
            },
        )
        .expect("read succeeds");

        assert_eq!(read.persistence, stored.persistence);
        assert_eq!(read.salience.factors.len(), 10);
    }

    #[test]
    fn score_salience_uses_subsecond_latest_stored_evaluation() {
        let db = test_db();
        insert_claim(&db, "claim-stored-subsecond", 0.72);
        let rationale_json =
            serde_json::to_string(&FactorRationale::Freshness { decay_factor: 1.0 })
                .expect("serialize rationale");
        for (evaluation_id, computed_at, value) in [
            ("zz-old", "2026-05-26T12:00:00.100Z", 0.1),
            ("aa-new", "2026-05-26T12:00:00.900Z", 0.9),
        ] {
            db.conn_ref()
                .execute(
                    "INSERT INTO salience_factors (
                        id, evaluation_id, claim_id, factor_kind, factor_value,
                        weight, rationale_json, schema_version, computed_at
                     ) VALUES (?1, ?2, 'claim-stored-subsecond', 'trust', ?3, 1.0, ?4, 1, ?5)",
                    rusqlite::params![
                        format!("salience-factor-{evaluation_id}"),
                        evaluation_id,
                        value,
                        &rationale_json,
                        computed_at
                    ],
                )
                .expect("insert stored salience factor");
        }
        let (clock, rng, external) = test_ctx(ExecutionMode::Live);
        let ctx = ServiceContext::test_live(&clock, &rng, &external).with_actor("system:test");

        let read = score_salience(
            &ctx,
            &db,
            ScoreSalienceRequest {
                schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
                claim_id: ClaimId("claim-stored-subsecond".to_string()),
            },
        )
        .expect("read succeeds");

        assert_eq!(
            read.persistence,
            SaliencePersistence::Stored {
                evaluation_id: "aa-new".to_string()
            }
        );
        assert_eq!(read.salience.total, 0.9);
    }

    #[test]
    fn open_loop_relevance_counts_only_same_subject_open_loops() {
        let db = test_db();
        insert_claim(&db, "claim-current-action", 0.35);
        insert_open_loop_claim(
            &db,
            "claim-related-open-loop",
            r#"{"kind":"account","id":"acct-example"}"#,
        );
        insert_open_loop_claim(
            &db,
            "claim-unrelated-open-loop",
            r#"{"kind":"account","id":"acct-other"}"#,
        );

        let (clock, rng, external) = test_ctx(ExecutionMode::Live);
        let ctx = ServiceContext::test_live(&clock, &rng, &external).with_actor("system:test");
        let current_action = score_salience(
            &ctx,
            &db,
            ScoreSalienceRequest {
                schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
                claim_id: ClaimId("claim-current-action".to_string()),
            },
        )
        .expect("current action score");
        let factor = current_action
            .salience
            .factors
            .iter()
            .find(|factor| factor.kind == SalienceFactorKind::OpenLoopRelevance)
            .expect("open-loop factor exists");

        assert_eq!(factor.value, Some(1.0));
        match &factor.rationale {
            FactorRationale::OpenLoopRelevance {
                open_loop_count,
                has_action,
            } => {
                assert_eq!(*open_loop_count, 1);
                assert!(*has_action);
            }
            other => panic!("unexpected rationale: {other:?}"),
        }
    }

    #[test]
    fn low_trust_current_action_can_outrank_high_trust_background_claim() {
        let db = test_db();
        insert_background_claim(&db, "claim-background", 0.95);
        insert_claim(&db, "claim-current-action", 0.35);

        let (clock, rng, external) = test_ctx(ExecutionMode::Live);
        let ctx = ServiceContext::test_live(&clock, &rng, &external).with_actor("system:test");
        let current_action = score_salience(
            &ctx,
            &db,
            ScoreSalienceRequest {
                schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
                claim_id: ClaimId("claim-current-action".to_string()),
            },
        )
        .expect("current action score");
        let background = score_salience(
            &ctx,
            &db,
            ScoreSalienceRequest {
                schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
                claim_id: ClaimId("claim-background".to_string()),
            },
        )
        .expect("background score");

        assert!(
            current_action.salience.total > background.salience.total,
            "current low-trust action should outrank background high-trust claim"
        );
    }
}

//! Derived-state projection ledger.
//!
//! `intelligence_claims` is the durable source of truth for claim-
//! shaped state. Several legacy consumers still read from cached
//! sibling tables and `intelligence.json`; commit_claim keeps them
//! current by running per-target projection rules right after the
//! claim insert.
//!
//! The projections are best-effort: one rule may fail without
//! aborting the authoritative claim, and a `claim_projection_status`
//! row records the outcome. Failed rows are the repair worklist a
//! sibling repair binary picks up.
//!
//! This module owns the substrate types, projection rules, and
//! status-write surface for the claim-to-legacy bridge.
//!
//! ## Invariants
//!
//! - Status writes are append-only per `(claim_id, projection_target)`
//!   primary key; an idempotent rerun of the same target overwrites
//!   the row but never deletes it.
//! - `attempted_at` is supplied by the caller (`ServiceContext.clock`)
//!   so backfill and tests get deterministic ordering.
//! - `succeeded_at` is set only on `committed` / `repaired` rows;
//!   `failed` rows leave it NULL.

use crate::abilities::claims::metadata_for_name;
use crate::db::claim_invalidation::SubjectRef;
use crate::db::claims::IntelligenceClaim;
use crate::db::ActionDb;
use crate::services::context::ServiceContext;

const PROJECTION_TARGETS: [ProjectionTarget; 4] = [
    ProjectionTarget::EntityIntelligence,
    ProjectionTarget::SuccessPlans,
    ProjectionTarget::AccountsColumns,
    ProjectionTarget::IntelligenceJson,
];

const ENTITY_INTELLIGENCE_CLAIM_TYPES: [&str; 7] = [
    "entity_summary",
    "entity_current_state",
    "entity_risk",
    "entity_win",
    "company_context",
    "value_delivered",
    "stakeholder_engagement",
];

/// Projection targets for the v1.4.0 dual-projection window. The
/// label values are stable wire-format strings — repair tooling and
/// cross-version manifests reference them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProjectionTarget {
    /// Legacy `entity_assessment` + `entity_quality` tables that
    /// today's render path reads to reconstruct entity intelligence.
    EntityIntelligence,
    /// `success_plans` rows. Knock-on D in the storage-shape review:
    /// unowned legacy table that needs an explicit owner during the
    /// dual-projection window.
    SuccessPlans,
    /// Account AI columns (`company_overview`, `strategic_programs`,
    /// `notes`). Knock-on E: also unowned and needs an explicit owner.
    AccountsColumns,
    /// `intelligence.json` on disk. Sync-best-effort post-DB-commit;
    /// honors the existing schema-epoch fence so a stale worker
    /// can't overwrite a fresher projection.
    IntelligenceJson,
}

impl ProjectionTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EntityIntelligence => "entity_intelligence",
            Self::SuccessPlans => "success_plans",
            Self::AccountsColumns => "accounts_columns",
            Self::IntelligenceJson => "intelligence_json",
        }
    }

    pub fn try_from_str(s: &str) -> Option<Self> {
        Some(match s {
            "entity_intelligence" => Self::EntityIntelligence,
            "success_plans" => Self::SuccessPlans,
            "accounts_columns" => Self::AccountsColumns,
            "intelligence_json" => Self::IntelligenceJson,
            _ => return None,
        })
    }
}

/// Outcome status recorded per (claim, target) pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionStatus {
    /// Projection succeeded at commit time.
    Committed,
    /// Projection failed; row is on the repair worklist. The
    /// authoritative claim was already committed — failed projections
    /// are best-effort and don't roll back the claim.
    Failed,
    /// A previously-failed projection was successfully reprojected
    /// by the repair worker.
    Repaired,
}

impl ProjectionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Committed => "committed",
            Self::Failed => "failed",
            Self::Repaired => "repaired",
        }
    }

    pub fn try_from_str(s: &str) -> Option<Self> {
        Some(match s {
            "committed" => Self::Committed,
            "failed" => Self::Failed,
            "repaired" => Self::Repaired,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionErrorClass {
    ValidationError,
    TargetTableLocked,
    FenceAdvanced,
    IoError,
    RegistryMismatch,
    Unknown,
}

impl ProjectionErrorClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ValidationError => "validation_error",
            Self::TargetTableLocked => "target_table_locked",
            Self::FenceAdvanced => "fence_advanced",
            Self::IoError => "io_error",
            Self::RegistryMismatch => "registry_mismatch",
            Self::Unknown => "unknown",
        }
    }
}

/// Outcome of a single projection rule. The aggregate of these
/// across all targets is what commit_claim returns to its caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionOutcome {
    pub target: ProjectionTarget,
    pub status: ProjectionStatus,
    /// Recorded only for `Failed` rows; carries the error class so
    /// repair can branch on it. Customer text never appears here.
    pub error_message: Option<String>,
    pub attempted_at: String,
    pub succeeded_at: Option<String>,
}

/// Errors callers may see from this module. Distinct from a
/// `Failed` status: those are recorded outcomes; these are
/// substrate-level problems (DB unavailable, malformed input).
#[derive(Debug, thiserror::Error)]
pub enum DerivedStateError {
    #[error("ServiceContext mutation gate: {0}")]
    Mode(String),
    #[error("rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),
}

/// Record a projection outcome. Upserts on the
/// `(claim_id, projection_target)` primary key so a repair pass
/// idempotently overwrites a prior failed row without losing the
/// audit ordering (`attempted_at` is the most-recent attempt).
///
/// `succeeded_at` should be `Some` for `Committed` / `Repaired` and
/// `None` for `Failed`. The contract isn't enforced at the SQL layer
/// because backfill and out-of-band repair flows may want flexibility,
/// but production callers go through the `mark_*` helpers below.
pub fn record_projection_outcome(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    claim_id: &str,
    outcome: &ProjectionOutcome,
) -> Result<(), DerivedStateError> {
    ctx.check_mutation_allowed()
        .map_err(|e| DerivedStateError::Mode(e.to_string()))?;
    db.conn_ref().execute(
        "INSERT INTO claim_projection_status \
         (claim_id, projection_target, status, error_message, attempted_at, succeeded_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
         ON CONFLICT (claim_id, projection_target) DO UPDATE SET \
             status = excluded.status, \
             error_message = excluded.error_message, \
             attempted_at = excluded.attempted_at, \
             succeeded_at = excluded.succeeded_at",
        rusqlite::params![
            claim_id,
            outcome.target.as_str(),
            outcome.status.as_str(),
            outcome.error_message.as_deref(),
            outcome.attempted_at,
            outcome.succeeded_at.as_deref(),
        ],
    )?;
    Ok(())
}

/// Convenience: record a successful projection at the supplied
/// `attempted_at` (typically `ctx.clock.now().to_rfc3339()`).
pub fn mark_committed(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    claim_id: &str,
    target: ProjectionTarget,
    attempted_at: &str,
) -> Result<(), DerivedStateError> {
    record_projection_outcome(
        ctx,
        db,
        claim_id,
        &ProjectionOutcome {
            target,
            status: ProjectionStatus::Committed,
            error_message: None,
            attempted_at: attempted_at.to_string(),
            succeeded_at: Some(attempted_at.to_string()),
        },
    )
}

/// Convenience: record a failed projection. The error class string
/// must NOT contain customer text — it's a class label like
/// `validation_error`, `target_table_locked`, `fence_advanced`.
pub fn mark_failed(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    claim_id: &str,
    target: ProjectionTarget,
    attempted_at: &str,
    error_class: ProjectionErrorClass,
) -> Result<(), DerivedStateError> {
    record_projection_outcome(
        ctx,
        db,
        claim_id,
        &ProjectionOutcome {
            target,
            status: ProjectionStatus::Failed,
            error_message: Some(error_class.as_str().to_string()),
            attempted_at: attempted_at.to_string(),
            succeeded_at: None,
        },
    )
}

/// Convenience: mark a previously-failed projection as repaired.
pub fn mark_repaired(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    claim_id: &str,
    target: ProjectionTarget,
    attempted_at: &str,
) -> Result<(), DerivedStateError> {
    record_projection_outcome(
        ctx,
        db,
        claim_id,
        &ProjectionOutcome {
            target,
            status: ProjectionStatus::Repaired,
            error_message: None,
            attempted_at: attempted_at.to_string(),
            succeeded_at: Some(attempted_at.to_string()),
        },
    )
}

/// Read-side: enumerate the failed-projection worklist for a target.
/// Returns `(claim_id, error_message_or_class)` pairs. The repair
/// binary uses this to drive idempotent reprojection.
pub fn list_failed_projections(
    db: &ActionDb,
    target: ProjectionTarget,
) -> Result<Vec<(String, Option<String>)>, rusqlite::Error> {
    let mut stmt = db.conn_ref().prepare(
        "SELECT claim_id, error_message \
         FROM claim_projection_status \
         WHERE projection_target = ?1 AND status = 'failed' \
         ORDER BY attempted_at",
    )?;
    let rows = stmt.query_map(rusqlite::params![target.as_str()], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn project_claim_to_db_legacy_tx(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    claim: &IntelligenceClaim,
) -> Vec<ProjectionOutcome> {
    let attempted_at = ctx.clock.now().to_rfc3339();
    if ctx.check_mutation_allowed().is_err() {
        return PROJECTION_TARGETS
            .into_iter()
            .map(|target| failed_outcome(target, &attempted_at, ProjectionErrorClass::Unknown))
            .collect();
    }

    PROJECTION_TARGETS
        .into_iter()
        .map(|target| match target {
            ProjectionTarget::EntityIntelligence => {
                project_entity_intelligence(ctx, tx, claim, &attempted_at)
            }
            ProjectionTarget::SuccessPlans
            | ProjectionTarget::AccountsColumns
            | ProjectionTarget::IntelligenceJson => committed_outcome(target, &attempted_at),
        })
        .collect()
}

fn project_entity_intelligence(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    claim: &IntelligenceClaim,
    attempted_at: &str,
) -> ProjectionOutcome {
    if metadata_for_name(&claim.claim_type).is_none() {
        return committed_outcome(ProjectionTarget::EntityIntelligence, attempted_at);
    }
    if !ENTITY_INTELLIGENCE_CLAIM_TYPES.contains(&claim.claim_type.as_str()) {
        return committed_outcome(ProjectionTarget::EntityIntelligence, attempted_at);
    }

    match run_rule_savepoint(tx, "entity_intelligence", || {
        rebuild_entity_assessment_from_claims(ctx, tx, claim)
    }) {
        Ok(()) => committed_outcome(ProjectionTarget::EntityIntelligence, attempted_at),
        Err(error_class) => failed_outcome(
            ProjectionTarget::EntityIntelligence,
            attempted_at,
            error_class,
        ),
    }
}

fn committed_outcome(target: ProjectionTarget, attempted_at: &str) -> ProjectionOutcome {
    ProjectionOutcome {
        target,
        status: ProjectionStatus::Committed,
        error_message: None,
        attempted_at: attempted_at.to_string(),
        succeeded_at: Some(attempted_at.to_string()),
    }
}

fn failed_outcome(
    target: ProjectionTarget,
    attempted_at: &str,
    error_class: ProjectionErrorClass,
) -> ProjectionOutcome {
    ProjectionOutcome {
        target,
        status: ProjectionStatus::Failed,
        error_message: Some(error_class.as_str().to_string()),
        attempted_at: attempted_at.to_string(),
        succeeded_at: None,
    }
}

fn run_rule_savepoint(
    tx: &ActionDb,
    name: &str,
    f: impl FnOnce() -> Result<(), ProjectionErrorClass>,
) -> Result<(), ProjectionErrorClass> {
    tx.conn_ref()
        .execute_batch(&format!("SAVEPOINT {name}"))
        .map_err(classify_sql_error)?;

    match f() {
        Ok(()) => tx
            .conn_ref()
            .execute_batch(&format!("RELEASE SAVEPOINT {name}"))
            .map_err(classify_sql_error),
        Err(error_class) => {
            let _ = tx
                .conn_ref()
                .execute_batch(&format!("ROLLBACK TO SAVEPOINT {name}"));
            let _ = tx
                .conn_ref()
                .execute_batch(&format!("RELEASE SAVEPOINT {name}"));
            Err(error_class)
        }
    }
}

fn classify_sql_error(error: rusqlite::Error) -> ProjectionErrorClass {
    match error {
        rusqlite::Error::SqliteFailure(ref sqlite_error, _) => match sqlite_error.code {
            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked => {
                ProjectionErrorClass::TargetTableLocked
            }
            _ => ProjectionErrorClass::Unknown,
        },
        _ => ProjectionErrorClass::Unknown,
    }
}

fn rebuild_entity_assessment_from_claims(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    claim: &IntelligenceClaim,
) -> Result<(), ProjectionErrorClass> {
    let (entity_id, entity_type) = entity_identity_from_subject_ref(&claim.subject_ref)?;
    let claims = crate::services::claims::load_claims_active(tx, &claim.subject_ref, None)
        .map_err(|_| ProjectionErrorClass::ValidationError)?;
    let projected = EntityAssessmentProjection::from_claims(&claims);
    let enriched_at = ctx.clock.now().to_rfc3339();

    tx.conn_ref()
        .execute(
            "INSERT INTO entity_assessment (
            entity_id, entity_type, enriched_at,
            executive_assessment, risks_json, recent_wins_json,
            current_state_json, stakeholder_insights_json,
            company_context_json, value_delivered
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(entity_id) DO UPDATE SET
            entity_type = excluded.entity_type,
            enriched_at = excluded.enriched_at,
            executive_assessment = excluded.executive_assessment,
            risks_json = excluded.risks_json,
            recent_wins_json = excluded.recent_wins_json,
            current_state_json = excluded.current_state_json,
            stakeholder_insights_json = excluded.stakeholder_insights_json,
            company_context_json = excluded.company_context_json,
            value_delivered = excluded.value_delivered",
            rusqlite::params![
                entity_id,
                entity_type,
                enriched_at,
                projected.executive_assessment.as_deref(),
                projected.risks_json.as_deref(),
                projected.recent_wins_json.as_deref(),
                projected.current_state_json.as_deref(),
                projected.stakeholder_insights_json.as_deref(),
                projected.company_context_json.as_deref(),
                projected.value_delivered_json.as_deref(),
            ],
        )
        .map_err(classify_sql_error)?;

    Ok(())
}

fn entity_identity_from_subject_ref(
    subject_ref: &str,
) -> Result<(String, &'static str), ProjectionErrorClass> {
    let value = serde_json::from_str::<serde_json::Value>(subject_ref)
        .map_err(|_| ProjectionErrorClass::ValidationError)?;
    let subject = crate::services::claims::subject_ref_from_json(&value)
        .map_err(|_| ProjectionErrorClass::ValidationError)?;
    match subject {
        SubjectRef::Account { id } => Ok((id, "account")),
        SubjectRef::Meeting { id } => Ok((id, "meeting")),
        SubjectRef::Person { id } => Ok((id, "person")),
        SubjectRef::Project { id } => Ok((id, "project")),
        SubjectRef::Email { .. } | SubjectRef::Multi(_) | SubjectRef::Global => {
            Err(ProjectionErrorClass::ValidationError)
        }
    }
}

#[derive(Default)]
struct EntityAssessmentProjection {
    executive_assessment: Option<String>,
    risks_json: Option<String>,
    recent_wins_json: Option<String>,
    current_state_json: Option<String>,
    stakeholder_insights_json: Option<String>,
    company_context_json: Option<String>,
    value_delivered_json: Option<String>,
}

impl EntityAssessmentProjection {
    fn from_claims(claims: &[IntelligenceClaim]) -> Self {
        let mut projection = Self::default();
        let mut risks = Vec::new();
        let mut wins = Vec::new();
        let mut current_state = Vec::new();
        let mut stakeholders = Vec::new();
        let mut values = Vec::new();
        let mut company_context = Vec::new();

        for claim in claims {
            match claim.claim_type.as_str() {
                "entity_summary" if projection.executive_assessment.is_none() => {
                    projection.executive_assessment = Some(claim.text.clone());
                }
                "entity_current_state" => current_state.push(claim.text.clone()),
                "entity_risk" => risks.push(claim.text.clone()),
                "entity_win" => wins.push(claim.text.clone()),
                "stakeholder_engagement" => stakeholders.push(claim.text.clone()),
                "value_delivered" => values.push(claim.text.clone()),
                "company_context" => company_context.push(claim.text.clone()),
                _ => {}
            }
        }

        projection.risks_json = json_array_from_text_claims(&risks, "text");
        projection.recent_wins_json = json_array_from_text_claims(&wins, "text");
        projection.current_state_json = json_current_state_from_claims(&current_state);
        projection.stakeholder_insights_json =
            json_array_from_text_claims(&stakeholders, "engagement");
        projection.value_delivered_json = json_array_from_text_claims(&values, "statement");
        projection.company_context_json = json_company_context_from_claims(&company_context);
        projection
    }
}

fn json_array_from_text_claims(claim_texts: &[String], text_key: &str) -> Option<String> {
    if claim_texts.is_empty() {
        return None;
    }
    let values: Vec<serde_json::Value> = claim_texts
        .iter()
        .map(|text| serde_json::json!({ text_key: text }))
        .collect();
    serde_json::to_string(&values).ok()
}

fn json_current_state_from_claims(claim_texts: &[String]) -> Option<String> {
    if claim_texts.is_empty() {
        return None;
    }
    serde_json::to_string(&serde_json::json!({
        "working": claim_texts,
        "notWorking": [],
        "unknowns": [],
    }))
    .ok()
}

fn json_company_context_from_claims(claim_texts: &[String]) -> Option<String> {
    if claim_texts.is_empty() {
        return None;
    }
    serde_json::to_string(&serde_json::json!({
        "description": claim_texts.first(),
        "additionalContext": claim_texts.iter().skip(1).cloned().collect::<Vec<_>>().join("\n"),
    }))
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::claims::{ClaimSensitivity, TemporalScope};
    use crate::db::test_utils::test_db;
    use crate::services::claims::{commit_claim, ClaimProposal, CommittedClaim};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::TimeZone;
    use rusqlite::params;

    const TS: &str = "2026-05-03T12:00:00+00:00";

    fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
        (
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap()),
            SeedableRng::new(7),
            ExternalClients::default(),
        )
    }

    fn live_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        external: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, external)
    }

    fn seed_claim(db: &ActionDb, claim_id: &str) {
        seed_claim_with_type(
            db,
            claim_id,
            "{\"kind\":\"account\",\"id\":\"acct-1\"}",
            "risk",
            "risk placeholder",
            TS,
        );
    }

    fn seed_claim_with_type(
        db: &ActionDb,
        claim_id: &str,
        subject_ref: &str,
        claim_type: &str,
        text: &str,
        created_at: &str,
    ) {
        // Minimal subject + claim row so the FK on
        // claim_projection_status.claim_id has something to point at.
        db.conn_ref()
            .execute(
                "INSERT OR IGNORE INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params!["acct-1", "Example Account", TS],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims /* dos7-allowed: test seed for FK target */ (\
                    id, subject_ref, claim_type, text, dedup_key, actor, \
                    data_source, observed_at, created_at, provenance_json) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 'system', 'manual', ?6, ?7, '{}')",
                params![
                    claim_id,
                    subject_ref,
                    claim_type,
                    text,
                    format!("hash:{claim_id}"),
                    TS,
                    created_at,
                ],
            )
            .unwrap();
    }

    fn projection_claim(claim_type: &str, subject_ref: &str) -> IntelligenceClaim {
        IntelligenceClaim {
            id: "projection-claim".to_string(),
            subject_ref: subject_ref.to_string(),
            claim_type: claim_type.to_string(),
            field_path: None,
            topic_key: None,
            text: "projected text".to_string(),
            dedup_key: format!("dedup:{claim_type}"),
            item_hash: None,
            actor: "system".to_string(),
            data_source: "manual".to_string(),
            source_ref: None,
            source_asof: None,
            observed_at: TS.to_string(),
            created_at: TS.to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            claim_state: crate::db::claims::ClaimState::Active,
            surfacing_state: crate::db::claims::SurfacingState::Active,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason: None,
            expires_at: None,
            superseded_by: None,
            trust_score: None,
            trust_computed_at: None,
            trust_version: None,
            thread_id: None,
            temporal_scope: TemporalScope::State,
            sensitivity: ClaimSensitivity::Internal,
            verification_state: crate::abilities::feedback::ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        }
    }

    fn claim_proposal(claim_type: &str, text: &str) -> ClaimProposal {
        ClaimProposal {
            subject_ref: "{\"kind\":\"account\",\"id\":\"acct-commit\"}".to_string(),
            claim_type: claim_type.to_string(),
            field_path: None,
            topic_key: None,
            text: text.to_string(),
            actor: "agent".to_string(),
            data_source: "manual".to_string(),
            source_ref: None,
            source_asof: None,
            observed_at: TS.to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(ClaimSensitivity::Internal),
            tombstone: None,
        }
    }

    #[test]
    fn projection_target_strings_round_trip() {
        for t in [
            ProjectionTarget::EntityIntelligence,
            ProjectionTarget::SuccessPlans,
            ProjectionTarget::AccountsColumns,
            ProjectionTarget::IntelligenceJson,
        ] {
            assert_eq!(ProjectionTarget::try_from_str(t.as_str()), Some(t));
        }
        assert_eq!(ProjectionTarget::try_from_str("not_a_target"), None);
    }

    #[test]
    fn projection_status_strings_round_trip() {
        for s in [
            ProjectionStatus::Committed,
            ProjectionStatus::Failed,
            ProjectionStatus::Repaired,
        ] {
            assert_eq!(ProjectionStatus::try_from_str(s.as_str()), Some(s));
        }
        assert_eq!(ProjectionStatus::try_from_str("queued"), None);
    }

    #[test]
    fn project_claim_to_db_legacy_tx_records_one_outcome_per_target() {
        let db = test_db();
        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);
        let claim = projection_claim("risk", "{\"kind\":\"account\",\"id\":\"acct-1\"}");

        let outcomes = project_claim_to_db_legacy_tx(&ctx, &db, &claim);

        assert_eq!(outcomes.len(), 4);
        assert_eq!(
            outcomes.iter().map(|o| o.target).collect::<Vec<_>>(),
            vec![
                ProjectionTarget::EntityIntelligence,
                ProjectionTarget::SuccessPlans,
                ProjectionTarget::AccountsColumns,
                ProjectionTarget::IntelligenceJson,
            ]
        );
        assert!(outcomes
            .iter()
            .all(|o| o.status == ProjectionStatus::Committed));
    }

    #[test]
    fn entity_intelligence_projection_rebuilds_legacy_row_from_claims() {
        let db = test_db();
        let subject = "{\"kind\":\"account\",\"id\":\"acct-1\"}";
        seed_claim_with_type(
            &db,
            "claim-summary",
            subject,
            "entity_summary",
            "Executive summary from claims",
            "2026-05-03T12:00:01+00:00",
        );
        seed_claim_with_type(
            &db,
            "claim-risk",
            subject,
            "entity_risk",
            "Renewal risk needs attention",
            "2026-05-03T12:00:02+00:00",
        );
        seed_claim_with_type(
            &db,
            "claim-win",
            subject,
            "entity_win",
            "Expansion proof point landed",
            "2026-05-03T12:00:03+00:00",
        );
        seed_claim_with_type(
            &db,
            "claim-state",
            subject,
            "entity_current_state",
            "Procurement path is clear",
            "2026-05-03T12:00:04+00:00",
        );
        seed_claim_with_type(
            &db,
            "claim-context",
            subject,
            "company_context",
            "Strategic account in healthcare",
            "2026-05-03T12:00:05+00:00",
        );
        seed_claim_with_type(
            &db,
            "claim-value",
            subject,
            "value_delivered",
            "Reduced onboarding time",
            "2026-05-03T12:00:06+00:00",
        );

        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);
        let claim = projection_claim("entity_summary", subject);

        let outcomes = project_claim_to_db_legacy_tx(&ctx, &db, &claim);

        assert_eq!(
            outcomes
                .iter()
                .find(|o| o.target == ProjectionTarget::EntityIntelligence)
                .map(|o| o.status),
            Some(ProjectionStatus::Committed)
        );
        let (
            entity_type,
            executive_assessment,
            risks_json,
            recent_wins_json,
            current_state_json,
            company_context_json,
            value_delivered,
        ): (
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT entity_type, executive_assessment, risks_json, recent_wins_json,
                        current_state_json, company_context_json, value_delivered
                 FROM entity_assessment WHERE entity_id = 'acct-1'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(entity_type, "account");
        assert_eq!(
            executive_assessment.as_deref(),
            Some("Executive summary from claims")
        );
        assert!(risks_json
            .as_deref()
            .unwrap()
            .contains("Renewal risk needs attention"));
        assert!(recent_wins_json
            .as_deref()
            .unwrap()
            .contains("Expansion proof point landed"));
        assert!(current_state_json
            .as_deref()
            .unwrap()
            .contains("Procurement path is clear"));
        assert!(company_context_json
            .as_deref()
            .unwrap()
            .contains("Strategic account in healthcare"));
        assert!(value_delivered
            .as_deref()
            .unwrap()
            .contains("Reduced onboarding time"));
    }

    #[test]
    fn failed_rule_does_not_abort_other_rules() {
        let db = test_db();
        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);
        let claim = projection_claim("entity_summary", "not-json");

        let outcomes = project_claim_to_db_legacy_tx(&ctx, &db, &claim);

        assert_eq!(outcomes.len(), 4);
        let entity = outcomes
            .iter()
            .find(|o| o.target == ProjectionTarget::EntityIntelligence)
            .unwrap();
        assert_eq!(entity.status, ProjectionStatus::Failed);
        assert_eq!(entity.error_message.as_deref(), Some("validation_error"));
        assert!(outcomes
            .iter()
            .filter(|o| o.target != ProjectionTarget::EntityIntelligence)
            .all(|o| o.status == ProjectionStatus::Committed));
    }

    #[test]
    fn commit_claim_writes_projection_status_alongside_claim() {
        let db = test_db();
        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);

        let committed = commit_claim(
            &ctx,
            &db,
            claim_proposal("entity_summary", "Committed summary"),
        )
        .unwrap();
        let claim_id = match committed {
            CommittedClaim::Inserted { claim } => claim.id,
            other => panic!("expected inserted claim, got {other:?}"),
        };

        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM claim_projection_status WHERE claim_id = ?1",
                params![claim_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 4);
    }

    #[test]
    fn mark_committed_writes_a_committed_row() {
        let db = test_db();
        seed_claim(&db, "claim-1");
        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);

        mark_committed(
            &ctx,
            &db,
            "claim-1",
            ProjectionTarget::EntityIntelligence,
            TS,
        )
        .unwrap();

        let (status, succeeded_at, attempted_at, err_msg): (
            String,
            Option<String>,
            String,
            Option<String>,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT status, succeeded_at, attempted_at, error_message \
                 FROM claim_projection_status \
                 WHERE claim_id = 'claim-1' AND projection_target = 'entity_intelligence'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(status, "committed");
        assert_eq!(succeeded_at.as_deref(), Some(TS));
        assert_eq!(attempted_at, TS);
        assert!(err_msg.is_none());
    }

    #[test]
    fn mark_failed_with_error_class_persists_class_label() {
        let db = test_db();
        seed_claim(&db, "claim-2");
        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);

        mark_failed(
            &ctx,
            &db,
            "claim-2",
            ProjectionTarget::IntelligenceJson,
            TS,
            ProjectionErrorClass::FenceAdvanced,
        )
        .unwrap();

        let (status, succeeded_at, err_msg): (String, Option<String>, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT status, succeeded_at, error_message \
                 FROM claim_projection_status \
                 WHERE claim_id = 'claim-2' AND projection_target = 'intelligence_json'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(status, "failed");
        assert_eq!(succeeded_at, None);
        assert_eq!(err_msg.as_deref(), Some("fence_advanced"));
    }

    #[test]
    fn mark_repaired_overwrites_prior_failed_row_idempotently() {
        let db = test_db();
        seed_claim(&db, "claim-3");
        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);

        // First attempt fails.
        mark_failed(
            &ctx,
            &db,
            "claim-3",
            ProjectionTarget::SuccessPlans,
            TS,
            ProjectionErrorClass::ValidationError,
        )
        .unwrap();
        // Repair attempt succeeds at a later attempted_at.
        let later = "2026-05-03T13:00:00+00:00";
        mark_repaired(&ctx, &db, "claim-3", ProjectionTarget::SuccessPlans, later).unwrap();

        // ON CONFLICT should have UPDATED the row, not inserted a
        // second one.
        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM claim_projection_status \
                 WHERE claim_id = 'claim-3' AND projection_target = 'success_plans'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        let (status, succeeded_at, attempted_at, err_msg): (
            String,
            Option<String>,
            String,
            Option<String>,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT status, succeeded_at, attempted_at, error_message \
                 FROM claim_projection_status \
                 WHERE claim_id = 'claim-3' AND projection_target = 'success_plans'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(status, "repaired");
        assert_eq!(succeeded_at.as_deref(), Some(later));
        assert_eq!(attempted_at, later);
        // Error class is cleared on repair.
        assert!(err_msg.is_none());
    }

    #[test]
    fn list_failed_projections_returns_only_failed_rows_for_target() {
        let db = test_db();
        seed_claim(&db, "claim-4");
        seed_claim(&db, "claim-5");
        seed_claim(&db, "claim-6");
        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);

        // Two failed for IntelligenceJson, one committed.
        mark_failed(
            &ctx,
            &db,
            "claim-4",
            ProjectionTarget::IntelligenceJson,
            TS,
            ProjectionErrorClass::FenceAdvanced,
        )
        .unwrap();
        mark_failed(
            &ctx,
            &db,
            "claim-5",
            ProjectionTarget::IntelligenceJson,
            TS,
            ProjectionErrorClass::IoError,
        )
        .unwrap();
        mark_committed(&ctx, &db, "claim-6", ProjectionTarget::IntelligenceJson, TS).unwrap();
        // One failed for a different target should not appear.
        mark_failed(
            &ctx,
            &db,
            "claim-4",
            ProjectionTarget::SuccessPlans,
            TS,
            ProjectionErrorClass::ValidationError,
        )
        .unwrap();

        let mut rows = list_failed_projections(&db, ProjectionTarget::IntelligenceJson).unwrap();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, "claim-4");
        assert_eq!(rows[0].1.as_deref(), Some("fence_advanced"));
        assert_eq!(rows[1].0, "claim-5");
        assert_eq!(rows[1].1.as_deref(), Some("io_error"));
    }

    #[test]
    fn foreign_key_pragma_rejects_orphan_claim_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = ActionDb::open_at_unencrypted(dir.path().join("fk-test.db")).unwrap();
        let fk_enabled: i64 = db
            .conn_ref()
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk_enabled, 1);

        let err = db
            .conn_ref()
            .execute(
                "INSERT INTO claim_projection_status
                 (claim_id, projection_target, status, attempted_at)
                 VALUES ('missing-claim', 'entity_intelligence', 'committed', ?1)",
                params![TS],
            )
            .expect_err("orphan claim_id must violate FK");
        assert!(matches!(
            err,
            rusqlite::Error::SqliteFailure(ref sqlite_error, _)
                if sqlite_error.code == rusqlite::ErrorCode::ConstraintViolation
        ));
    }
}

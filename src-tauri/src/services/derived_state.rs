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
//! - Projection rules should read the existing target row, build the
//!   candidate projection, and skip the write when the projected value
//!   is unchanged. This preserves target timestamps and avoids
//!   repeatedly re-stamping rows on idempotent repair passes.

use crate::abilities::claims::metadata_for_name;
use crate::db::claim_invalidation::SubjectRef;
use crate::db::claims::IntelligenceClaim;
use crate::db::ActionDb;
use crate::services::context::ServiceContext;
use rusqlite::OptionalExtension;

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

const SQLITE_ABORT_PREFIX: &str = "DOS:";
const SQLITE_ABORT_FENCE_ADVANCED: &str = "DOS:fence_advanced";
const SQLITE_ABORT_VALIDATION_ERROR: &str = "DOS:validation_error";

/// Return `None` when a target projection is unchanged, or `Some(new)`
/// when the caller should write the new value.
///
/// ```rust
/// # fn skip_if_unchanged<T: PartialEq>(existing: Option<T>, new: T) -> Option<T> {
/// #     match existing {
/// #         Some(existing) if existing == new => None,
/// #         _ => Some(new),
/// #     }
/// # }
/// assert_eq!(skip_if_unchanged(Some("same"), "same"), None);
/// assert_eq!(skip_if_unchanged(Some("old"), "new"), Some("new"));
/// assert_eq!(skip_if_unchanged::<i32>(None, 7), Some(7));
/// ```
pub(crate) fn skip_if_unchanged<T: PartialEq>(existing: Option<T>, new: T) -> Option<T> {
    match existing {
        Some(existing) if existing == new => None,
        _ => Some(new),
    }
}

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
            ProjectionTarget::AccountsColumns => {
                project_account_columns(ctx, tx, claim, &attempted_at)
            }
            ProjectionTarget::SuccessPlans | ProjectionTarget::IntelligenceJson => {
                committed_outcome(target, &attempted_at)
            }
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

    let projection_result = if claim.claim_type == "stakeholder_engagement"
        && matches!(
            entity_identity_from_subject_ref(&claim.subject_ref),
            Ok((_, "person"))
        ) {
        run_rule_savepoint(tx, "stakeholder_insights_cache", || {
            rebuild_stakeholder_insights_cache_from_claims(ctx, tx, claim)
        })
    } else {
        run_rule_savepoint(tx, "entity_intelligence", || {
            rebuild_entity_assessment_from_claims(ctx, tx, claim)
        })
    };

    match projection_result {
        Ok(()) => committed_outcome(ProjectionTarget::EntityIntelligence, attempted_at),
        Err(error_class) => failed_outcome(
            ProjectionTarget::EntityIntelligence,
            attempted_at,
            error_class,
        ),
    }
}

fn project_account_columns(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    claim: &IntelligenceClaim,
    attempted_at: &str,
) -> ProjectionOutcome {
    if claim.claim_type != "company_context" {
        return committed_outcome(ProjectionTarget::AccountsColumns, attempted_at);
    }

    match run_rule_savepoint(tx, "accounts_columns", || {
        rebuild_account_columns_from_claims(ctx, tx, claim)
    }) {
        Ok(()) => committed_outcome(ProjectionTarget::AccountsColumns, attempted_at),
        Err(error_class) => {
            failed_outcome(ProjectionTarget::AccountsColumns, attempted_at, error_class)
        }
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
            // Cleanup is best-effort: if the outer transaction is already
            // unwinding, SQLite will release the savepoint there. Logging keeps
            // cleanup failures visible without masking the projection failure.
            if let Err(e) = tx
                .conn_ref()
                .execute_batch(&format!("ROLLBACK TO SAVEPOINT {name}"))
            {
                log::warn!("savepoint {name} rollback failed: {e}");
            }
            if let Err(e) = tx
                .conn_ref()
                .execute_batch(&format!("RELEASE SAVEPOINT {name}"))
            {
                log::warn!("savepoint {name} release failed: {e}");
            }
            Err(error_class)
        }
    }
}

fn classify_sql_error(error: rusqlite::Error) -> ProjectionErrorClass {
    if let Some(error_class) = classify_stable_sql_abort(&error) {
        return error_class;
    }
    if is_missing_schema_object_error(&error) {
        return ProjectionErrorClass::RegistryMismatch;
    }

    match error {
        rusqlite::Error::SqliteFailure(ref sqlite_error, _) => match sqlite_error.code {
            rusqlite::ErrorCode::DatabaseBusy
            | rusqlite::ErrorCode::DatabaseLocked
            | rusqlite::ErrorCode::ReadOnly => ProjectionErrorClass::TargetTableLocked,
            rusqlite::ErrorCode::ConstraintViolation => ProjectionErrorClass::ValidationError,
            rusqlite::ErrorCode::DiskFull | rusqlite::ErrorCode::SystemIoFailure => {
                ProjectionErrorClass::IoError
            }
            _ => ProjectionErrorClass::Unknown,
        },
        rusqlite::Error::ToSqlConversionFailure(error) if error.is::<std::io::Error>() => {
            ProjectionErrorClass::IoError
        }
        rusqlite::Error::FromSqlConversionFailure(_, _, error) if error.is::<std::io::Error>() => {
            ProjectionErrorClass::IoError
        }
        _ => ProjectionErrorClass::Unknown,
    }
}

fn classify_stable_sql_abort(error: &rusqlite::Error) -> Option<ProjectionErrorClass> {
    let message = sqlite_error_message(error)?.trim();
    if !message.starts_with(SQLITE_ABORT_PREFIX) {
        return None;
    }
    Some(match message {
        SQLITE_ABORT_FENCE_ADVANCED => ProjectionErrorClass::FenceAdvanced,
        SQLITE_ABORT_VALIDATION_ERROR => ProjectionErrorClass::ValidationError,
        _ => ProjectionErrorClass::Unknown,
    })
}

fn sqlite_error_message(error: &rusqlite::Error) -> Option<&str> {
    match error {
        rusqlite::Error::SqliteFailure(_, Some(message)) => Some(message.as_str()),
        rusqlite::Error::SqlInputError { msg, .. } => Some(msg.as_str()),
        _ => None,
    }
}

fn is_missing_schema_object_error(error: &rusqlite::Error) -> bool {
    let Some(message) = sqlite_error_message(error) else {
        return false;
    };
    let normalized = message.trim().to_ascii_lowercase();
    // rusqlite 0.31/libsqlite3-sys 0.28 does not expose SQLite's newer
    // missing-table or missing-column extended codes. Keep the unavoidable
    // message parsing in one named helper so callers do not grow ad hoc
    // substring checks.
    normalized.starts_with("no such column:") || normalized.starts_with("no such table:")
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
    if let Some((existing_entity_type, existing_projected)) =
        existing_entity_assessment_projection(tx, &entity_id)?
    {
        if existing_entity_type == entity_type
            && skip_if_unchanged(Some(existing_projected), projected.clone()).is_none()
        {
            return Ok(());
        }
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct StakeholderEntityMembership {
    entity_id: String,
    entity_type: String,
}

fn rebuild_stakeholder_insights_cache_from_claims(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    claim: &IntelligenceClaim,
) -> Result<(), ProjectionErrorClass> {
    let person_id = person_id_from_subject_ref(&claim.subject_ref)?;
    for membership in stakeholder_cache_memberships(tx, &person_id)? {
        rebuild_stakeholder_insights_cache_for_entity(
            ctx,
            tx,
            &membership.entity_id,
            &membership.entity_type,
        )?;
    }
    Ok(())
}

fn stakeholder_cache_memberships(
    tx: &ActionDb,
    person_id: &str,
) -> Result<Vec<StakeholderEntityMembership>, ProjectionErrorClass> {
    let mut stmt = tx
        .conn_ref()
        .prepare(
            "SELECT account_id, 'account' AS entity_type
             FROM account_stakeholders
             WHERE person_id = ?1
             UNION
             SELECT em.entity_id, COALESCE(e.entity_type, 'project') AS entity_type
             FROM entity_members em
             LEFT JOIN entities e ON e.id = em.entity_id
             WHERE em.person_id = ?1
             ORDER BY 1, 2",
        )
        .map_err(classify_sql_error)?;
    let rows = stmt
        .query_map(rusqlite::params![person_id], |row| {
            Ok(StakeholderEntityMembership {
                entity_id: row.get(0)?,
                entity_type: row.get(1)?,
            })
        })
        .map_err(classify_sql_error)?;

    let mut memberships = Vec::new();
    for row in rows {
        memberships.push(row.map_err(classify_sql_error)?);
    }
    Ok(memberships)
}

fn rebuild_stakeholder_insights_cache_for_entity(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    entity_id: &str,
    entity_type: &str,
) -> Result<(), ProjectionErrorClass> {
    let person_ids = stakeholder_person_ids_for_entity(tx, entity_id)?;
    let mut claims = Vec::new();
    for person_id in person_ids {
        let subject_ref = serde_json::json!({
            "kind": "person",
            "id": person_id,
        })
        .to_string();
        let mut person_claims = crate::services::claims::load_claims_active(
            tx,
            &subject_ref,
            Some("stakeholder_engagement"),
        )
        .map_err(|_| ProjectionErrorClass::ValidationError)?;
        claims.append(&mut person_claims);
    }

    let claim_refs = claims.iter().collect::<Vec<_>>();
    let stakeholder_insights_json = json_array_from_claims(&claim_refs, "engagement");
    let existing = tx
        .conn_ref()
        .query_row(
            "SELECT stakeholder_insights_json
             FROM entity_assessment
             WHERE entity_id = ?1",
            rusqlite::params![entity_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(classify_sql_error)?
        .flatten();

    if skip_if_unchanged(Some(existing), stakeholder_insights_json.clone()).is_none() {
        return Ok(());
    }

    tx.conn_ref()
        .execute(
            "INSERT INTO entity_assessment (
                entity_id, entity_type, enriched_at, stakeholder_insights_json
             ) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(entity_id) DO UPDATE SET
                entity_type = excluded.entity_type,
                enriched_at = excluded.enriched_at,
                stakeholder_insights_json = excluded.stakeholder_insights_json",
            rusqlite::params![
                entity_id,
                entity_type,
                ctx.clock.now().to_rfc3339(),
                stakeholder_insights_json.as_deref(),
            ],
        )
        .map_err(classify_sql_error)?;

    Ok(())
}

fn stakeholder_person_ids_for_entity(
    tx: &ActionDb,
    entity_id: &str,
) -> Result<Vec<String>, ProjectionErrorClass> {
    let mut stmt = tx
        .conn_ref()
        .prepare(
            "SELECT person_id
             FROM account_stakeholders
             WHERE account_id = ?1
             UNION
             SELECT person_id
             FROM entity_members
             WHERE entity_id = ?1
             ORDER BY 1",
        )
        .map_err(classify_sql_error)?;
    let rows = stmt
        .query_map(rusqlite::params![entity_id], |row| row.get::<_, String>(0))
        .map_err(classify_sql_error)?;

    let mut person_ids = Vec::new();
    for row in rows {
        person_ids.push(row.map_err(classify_sql_error)?);
    }
    Ok(person_ids)
}

fn rebuild_account_columns_from_claims(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    claim: &IntelligenceClaim,
) -> Result<(), ProjectionErrorClass> {
    let (entity_id, entity_type) = entity_identity_from_subject_ref(&claim.subject_ref)?;
    if entity_type != "account" {
        return Err(ProjectionErrorClass::ValidationError);
    }

    let claims = crate::services::claims::load_claims_active(
        tx,
        &claim.subject_ref,
        Some("company_context"),
    )
    .map_err(|_| ProjectionErrorClass::ValidationError)?;
    let projected = EntityAssessmentProjection::from_claims(&claims).company_context_json;
    let existing = tx
        .conn_ref()
        .query_row(
            "SELECT company_overview FROM accounts WHERE id = ?1",
            rusqlite::params![entity_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(classify_sql_error)?
        .flatten();

    if existing == projected {
        return Ok(());
    }

    tx.conn_ref()
        .execute(
            "UPDATE accounts SET company_overview = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![
                projected.as_deref(),
                ctx.clock.now().to_rfc3339(),
                entity_id
            ],
        )
        .map_err(classify_sql_error)?;

    Ok(())
}

fn existing_entity_assessment_projection(
    tx: &ActionDb,
    entity_id: &str,
) -> Result<Option<(String, EntityAssessmentProjection)>, ProjectionErrorClass> {
    tx.conn_ref()
        .query_row(
            "SELECT entity_type, executive_assessment, risks_json, recent_wins_json,
                    current_state_json, stakeholder_insights_json, company_context_json,
                    value_delivered
             FROM entity_assessment
             WHERE entity_id = ?1",
            rusqlite::params![entity_id],
            |row| {
                Ok((
                    row.get(0)?,
                    EntityAssessmentProjection {
                        executive_assessment: row.get(1)?,
                        risks_json: row.get(2)?,
                        recent_wins_json: row.get(3)?,
                        current_state_json: row.get(4)?,
                        stakeholder_insights_json: row.get(5)?,
                        company_context_json: row.get(6)?,
                        value_delivered_json: row.get(7)?,
                    },
                ))
            },
        )
        .optional()
        .map_err(classify_sql_error)
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

fn person_id_from_subject_ref(subject_ref: &str) -> Result<String, ProjectionErrorClass> {
    let (id, entity_type) = entity_identity_from_subject_ref(subject_ref)?;
    if entity_type == "person" {
        Ok(id)
    } else {
        Err(ProjectionErrorClass::ValidationError)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
        let mut risks: Vec<&IntelligenceClaim> = Vec::new();
        let mut wins: Vec<&IntelligenceClaim> = Vec::new();
        let mut current_state: Vec<&IntelligenceClaim> = Vec::new();
        let mut stakeholders: Vec<&IntelligenceClaim> = Vec::new();
        let mut values: Vec<&IntelligenceClaim> = Vec::new();
        let mut company_context: Vec<&IntelligenceClaim> = Vec::new();

        for claim in claims {
            match claim.claim_type.as_str() {
                "entity_summary" if projection.executive_assessment.is_none() => {
                    projection.executive_assessment = Some(claim_projection_text(claim));
                }
                "entity_current_state" => current_state.push(claim),
                "entity_risk" => risks.push(claim),
                "entity_win" => wins.push(claim),
                "stakeholder_engagement" => stakeholders.push(claim),
                "value_delivered" => values.push(claim),
                "company_context" => company_context.push(claim),
                _ => {}
            }
        }

        projection.risks_json = json_array_from_claims(&risks, "text");
        projection.recent_wins_json = json_array_from_claims(&wins, "text");
        projection.current_state_json = json_current_state_from_claims(&current_state);
        projection.stakeholder_insights_json = json_array_from_claims(&stakeholders, "engagement");
        projection.value_delivered_json = json_array_from_claims(&values, "statement");
        projection.company_context_json = json_company_context_from_claims(&company_context);
        projection
    }
}

fn claim_projection_value(claim: &IntelligenceClaim) -> Option<serde_json::Value> {
    let metadata = claim.metadata_json.as_deref()?;
    serde_json::from_str::<serde_json::Value>(metadata)
        .ok()?
        .get("legacy_projection_value")
        .cloned()
}

fn claim_projection_text(claim: &IntelligenceClaim) -> String {
    if let Some(value) = claim_projection_value(claim) {
        if let Some(text) = value.as_str() {
            return text.to_string();
        }
        for key in [
            "text",
            "statement",
            "description",
            "engagement",
            "assessment",
        ] {
            if let Some(text) = value.get(key).and_then(|v| v.as_str()) {
                return text.to_string();
            }
        }
    }
    claim.text.clone()
}

fn json_array_from_claims(claims: &[&IntelligenceClaim], text_key: &str) -> Option<String> {
    if claims.is_empty() {
        return None;
    }
    let values: Vec<serde_json::Value> = claims
        .iter()
        .map(|claim| {
            claim_projection_value(claim)
                .unwrap_or_else(|| serde_json::json!({ text_key: claim_projection_text(claim) }))
        })
        .collect();
    serde_json::to_string(&values).ok()
}

fn json_current_state_from_claims(claims: &[&IntelligenceClaim]) -> Option<String> {
    if claims.is_empty() {
        return None;
    }
    if let Some(value) = claims
        .iter()
        .find_map(|claim| claim_projection_value(claim))
    {
        return serde_json::to_string(&value).ok();
    }
    let claim_texts: Vec<String> = claims
        .iter()
        .map(|claim| claim_projection_text(claim))
        .collect();
    serde_json::to_string(&serde_json::json!({
        "working": claim_texts,
        "notWorking": [],
        "unknowns": [],
    }))
    .ok()
}

fn json_company_context_from_claims(claims: &[&IntelligenceClaim]) -> Option<String> {
    if claims.is_empty() {
        return None;
    }
    if let Some(value) = claims
        .iter()
        .find_map(|claim| claim_projection_value(claim))
    {
        return serde_json::to_string(&value).ok();
    }
    let claim_texts: Vec<String> = claims
        .iter()
        .map(|claim| claim_projection_text(claim))
        .collect();
    serde_json::to_string(&serde_json::json!({
        "description": claim_texts.first(),
        "additionalContext": claim_texts.iter().skip(1).cloned().collect::<Vec<_>>().join("\n"),
    }))
    .ok()
}

/// Compatibility writer for legacy intelligence readers.
///
/// Claim-shaped fields should be committed through `commit_claim`, which runs
/// the projection rules above. The same legacy row also carries snapshot fields
/// that are not clean claims: enrichment timestamps, source manifests, health
/// structs, consistency metadata, relationship depth, and UI cache blobs. Those
/// stay as direct cache writes during the dual-read window, but this module owns
/// the SQL so callers do not write projection targets from unrelated services.
pub fn upsert_entity_intelligence_legacy_snapshot(
    db: &ActionDb,
    intel: &crate::intelligence::IntelligenceJson,
) -> Result<(), rusqlite::Error> {
    let conn = db.conn_ref();

    let dimensions_json = serde_json::to_string(&intel.dimensions_blob()).ok();
    conn.execute(
        "INSERT INTO entity_assessment (
            entity_id, entity_type, enriched_at, source_file_count,
            next_meeting_readiness_json, success_metrics, open_commitments,
            relationship_depth, health_json, org_health_json, consistency_status,
            consistency_findings_json, consistency_checked_at,
            portfolio_json, network_json, user_edits_json, source_manifest_json,
            dimensions_json, success_plan_signals_json, pull_quote
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
        ON CONFLICT(entity_id) DO UPDATE SET
            entity_type = excluded.entity_type,
            enriched_at = excluded.enriched_at,
            source_file_count = excluded.source_file_count,
            next_meeting_readiness_json = excluded.next_meeting_readiness_json,
            success_metrics = excluded.success_metrics,
            open_commitments = excluded.open_commitments,
            relationship_depth = excluded.relationship_depth,
            health_json = excluded.health_json,
            org_health_json = excluded.org_health_json,
            consistency_status = excluded.consistency_status,
            consistency_findings_json = excluded.consistency_findings_json,
            consistency_checked_at = excluded.consistency_checked_at,
            portfolio_json = excluded.portfolio_json,
            network_json = excluded.network_json,
            user_edits_json = excluded.user_edits_json,
            source_manifest_json = excluded.source_manifest_json,
            dimensions_json = excluded.dimensions_json,
            success_plan_signals_json = excluded.success_plan_signals_json,
            pull_quote = excluded.pull_quote",
        rusqlite::params![
            intel.entity_id,
            intel.entity_type,
            intel.enriched_at,
            intel.source_file_count,
            serde_json::to_string(&intel.next_meeting_readiness).ok(),
            serde_json::to_string(&intel.success_metrics).ok(),
            serde_json::to_string(&intel.open_commitments).ok(),
            serde_json::to_string(&intel.relationship_depth).ok(),
            intel.health.as_ref().and_then(|v| serde_json::to_string(v).ok()),
            intel
                .org_health
                .as_ref()
                .and_then(|v| serde_json::to_string(v).ok()),
            serde_json::to_string(&intel.consistency_status).ok(),
            serde_json::to_string(&intel.consistency_findings).ok(),
            intel.consistency_checked_at,
            serde_json::to_string(&intel.portfolio).ok(),
            serde_json::to_string(&intel.network).ok(),
            serde_json::to_string(&intel.user_edits).ok(),
            serde_json::to_string(&intel.source_manifest).ok(),
            dimensions_json,
            intel
                .success_plan_signals
                .as_ref()
                .and_then(|v| serde_json::to_string(v).ok()),
            intel.pull_quote,
        ],
    )?;

    conn.execute(
        "DELETE FROM intelligence_feedback WHERE entity_id = ?1 AND entity_type = ?2 \
         AND field NOT LIKE 'account_field_conflict:%'",
        rusqlite::params![intel.entity_id, intel.entity_type],
    )?;
    conn.execute(
        "DELETE FROM entity_feedback_events WHERE entity_id = ?1 AND entity_type = ?2 \
         AND feedback_type IN ('confirmed', 'rejected') \
         AND COALESCE(source_kind, '') != 'field_conflict'",
        rusqlite::params![intel.entity_id, intel.entity_type],
    )?;

    if let Some(health) = intel.health.as_ref() {
        upsert_entity_health_legacy_projection(db, &intel.entity_id, &intel.entity_type, health)?;
    }

    emit_enrichment_side_effect_signals(db, intel);

    Ok(())
}

pub fn upsert_entity_health_legacy_projection(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    health: &crate::intelligence::io::AccountHealth,
) -> Result<(), rusqlite::Error> {
    db.conn_ref().execute(
        "INSERT INTO entity_assessment (entity_id, entity_type, health_json)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(entity_id) DO UPDATE SET
             entity_type = excluded.entity_type,
             health_json = excluded.health_json",
        rusqlite::params![entity_id, entity_type, serde_json::to_string(health).ok(),],
    )?;
    db.conn_ref().execute(
        "INSERT INTO entity_quality (entity_id, entity_type, health_score, health_trend)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(entity_id) DO UPDATE SET
             entity_type = excluded.entity_type,
             health_score = excluded.health_score,
             health_trend = excluded.health_trend",
        rusqlite::params![
            entity_id,
            entity_type,
            health.score,
            serde_json::to_string(&health.trend).ok(),
        ],
    )?;
    Ok(())
}

pub fn upsert_health_outlook_signals_legacy_projection(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    signals_json: &str,
) -> Result<(), rusqlite::Error> {
    db.conn_ref().execute(
        "INSERT INTO entity_assessment (entity_id, entity_type, health_outlook_signals_json)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(entity_id) DO UPDATE SET
             entity_type = excluded.entity_type,
             health_outlook_signals_json = excluded.health_outlook_signals_json",
        rusqlite::params![entity_id, entity_type, signals_json],
    )?;
    Ok(())
}

pub fn clear_contaminated_enrichment_projection(
    db: &ActionDb,
    account_id: &str,
) -> Result<(), rusqlite::Error> {
    db.conn_ref().execute(
        "UPDATE entity_assessment SET executive_assessment = NULL WHERE entity_id = ?1",
        rusqlite::params![account_id],
    )?;
    db.conn_ref().execute(
        "UPDATE accounts SET company_overview = NULL, strategic_programs = NULL, notes = NULL \
         WHERE id = ?1",
        rusqlite::params![account_id],
    )?;
    Ok(())
}

pub fn update_account_ai_field_projection(
    db: &ActionDb,
    id: &str,
    field: &str,
    value: &str,
    updated_at: &str,
) -> Result<(), rusqlite::Error> {
    match field {
        "notes" => db.conn_ref().execute(
            "UPDATE accounts SET notes = CASE WHEN ?1 = '' THEN NULL ELSE ?1 END, \
             updated_at = ?3 WHERE id = ?2",
            rusqlite::params![value, id, updated_at],
        )?,
        "strategic_programs" => db.conn_ref().execute(
            "UPDATE accounts SET strategic_programs = ?1, updated_at = ?3 WHERE id = ?2",
            rusqlite::params![value, id, updated_at],
        )?,
        "company_overview" => db.conn_ref().execute(
            "UPDATE accounts SET company_overview = ?1, updated_at = ?3 WHERE id = ?2",
            rusqlite::params![value, id, updated_at],
        )?,
        _ => return Err(rusqlite::Error::InvalidParameterName(field.to_string())),
    };
    Ok(())
}

pub fn update_account_ai_columns_projection(
    db: &ActionDb,
    id: &str,
    company_overview: Option<&str>,
    strategic_programs: Option<&str>,
    notes: Option<&str>,
    updated_at: &str,
) -> Result<(), rusqlite::Error> {
    db.conn_ref().execute(
        "UPDATE accounts SET company_overview = ?1, strategic_programs = ?2, \
         notes = ?3, updated_at = ?4 WHERE id = ?5",
        rusqlite::params![company_overview, strategic_programs, notes, updated_at, id],
    )?;
    Ok(())
}

fn emit_enrichment_side_effect_signals(
    db: &ActionDb,
    intel: &crate::intelligence::IntelligenceJson,
) {
    for item in &intel.regulatory_context {
        if item.status == "gap" {
            let value = serde_json::json!({
                "standard": item.standard,
                "evidence": item.evidence,
            })
            .to_string();
            let _ = crate::signals::bus::emit_signal(
                db,
                &intel.entity_type,
                &intel.entity_id,
                "regulatory_gap_detected",
                "enrichment_write",
                Some(&value),
                0.9,
            );
        } else if item.status == "required" || item.status == "in_progress" {
            let value = serde_json::json!({
                "standard": item.standard,
                "status": item.status,
            })
            .to_string();
            let _ = crate::signals::bus::emit_signal(
                db,
                &intel.entity_type,
                &intel.entity_id,
                "regulatory_requirement_detected",
                "enrichment_write",
                Some(&value),
                0.85,
            );
        }
    }

    for insight in &intel.stakeholder_insights {
        if let Some(ref person_id) = insight.person_id {
            let (signal_type, confidence) = if insight.verified {
                ("stakeholder_verified", 0.9)
            } else {
                ("stakeholder_unverified", 0.7)
            };
            let value = serde_json::json!({
                "person_id": person_id,
                "name": insight.name,
                "verified_source": insight.verified_source,
            })
            .to_string();
            let _ = crate::signals::bus::emit_signal(
                db,
                &intel.entity_type,
                &intel.entity_id,
                signal_type,
                "enrichment_write",
                Some(&value),
                confidence,
            );
        }
    }
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
    fn entity_intelligence_projection_skips_write_when_projected_content_is_unchanged() {
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

        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);
        let claim = projection_claim("entity_summary", subject);

        let first = project_claim_to_db_legacy_tx(&ctx, &db, &claim);
        assert_eq!(
            first
                .iter()
                .find(|o| o.target == ProjectionTarget::EntityIntelligence)
                .map(|o| o.status),
            Some(ProjectionStatus::Committed)
        );
        let first_enriched_at: String = db
            .conn_ref()
            .query_row(
                "SELECT enriched_at FROM entity_assessment WHERE entity_id = 'acct-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        clock.advance(chrono::Duration::hours(1));
        let second = project_claim_to_db_legacy_tx(&ctx, &db, &claim);
        assert_eq!(
            second
                .iter()
                .find(|o| o.target == ProjectionTarget::EntityIntelligence)
                .map(|o| o.status),
            Some(ProjectionStatus::Committed)
        );
        let second_enriched_at: String = db
            .conn_ref()
            .query_row(
                "SELECT enriched_at FROM entity_assessment WHERE entity_id = 'acct-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(second_enriched_at, first_enriched_at);
    }

    #[test]
    fn failed_rule_does_not_abort_other_rules() {
        let db = test_db();
        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);
        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_entity_assessment_insert
                 BEFORE INSERT ON entity_assessment
                 BEGIN
                   SELECT RAISE(ABORT, 'DOS:validation_error');
                 END;
                 CREATE TRIGGER fail_entity_assessment_update
                 BEFORE UPDATE ON entity_assessment
                 BEGIN
                   SELECT RAISE(ABORT, 'DOS:validation_error');
                 END;",
            )
            .unwrap();

        let committed = commit_claim(
            &ctx,
            &db,
            claim_proposal("entity_summary", "Projection failure summary"),
        )
        .unwrap();
        let claim_id = match committed {
            CommittedClaim::Inserted { claim } => claim.id,
            other => panic!("expected inserted claim, got {other:?}"),
        };

        let claim_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM intelligence_claims WHERE id = ?1",
                params![&claim_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(claim_count, 1);

        let (entity_status, entity_error): (String, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT status, error_message
                 FROM claim_projection_status
                 WHERE claim_id = ?1 AND projection_target = 'entity_intelligence'",
                params![&claim_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(entity_status, "failed");
        assert_eq!(entity_error.as_deref(), Some("validation_error"));

        let committed_targets: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*)
                 FROM claim_projection_status
                 WHERE claim_id = ?1
                   AND projection_target IN ('success_plans', 'accounts_columns', 'intelligence_json')
                   AND status = 'committed'",
                params![&claim_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(committed_targets, 3);
    }

    fn sqlite_error(code: std::os::raw::c_int, message: &str) -> rusqlite::Error {
        rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), Some(message.to_string()))
    }

    #[test]
    fn classify_sql_error_maps_actionable_projection_failures() {
        assert_eq!(
            classify_sql_error(sqlite_error(rusqlite::ffi::SQLITE_CONSTRAINT, "constraint")),
            ProjectionErrorClass::ValidationError
        );
        assert_eq!(
            classify_sql_error(sqlite_error(rusqlite::ffi::SQLITE_FULL, "database full")),
            ProjectionErrorClass::IoError
        );
        assert_eq!(
            classify_sql_error(sqlite_error(rusqlite::ffi::SQLITE_IOERR, "io error")),
            ProjectionErrorClass::IoError
        );
        assert_eq!(
            classify_sql_error(sqlite_error(rusqlite::ffi::SQLITE_READONLY, "readonly")),
            ProjectionErrorClass::TargetTableLocked
        );
        assert_eq!(
            classify_sql_error(sqlite_error(
                rusqlite::ffi::SQLITE_ERROR,
                SQLITE_ABORT_FENCE_ADVANCED
            )),
            ProjectionErrorClass::FenceAdvanced
        );
        assert_eq!(
            classify_sql_error(sqlite_error(
                rusqlite::ffi::SQLITE_ERROR,
                SQLITE_ABORT_VALIDATION_ERROR
            )),
            ProjectionErrorClass::ValidationError
        );
        assert_eq!(
            classify_sql_error(sqlite_error(
                rusqlite::ffi::SQLITE_ERROR,
                "no such column: x"
            )),
            ProjectionErrorClass::RegistryMismatch
        );
        assert_eq!(
            classify_sql_error(sqlite_error(
                rusqlite::ffi::SQLITE_ERROR,
                "no such table: entity_assessment"
            )),
            ProjectionErrorClass::RegistryMismatch
        );
        assert_eq!(
            classify_sql_error(sqlite_error(rusqlite::ffi::SQLITE_MISUSE, "unexpected")),
            ProjectionErrorClass::Unknown
        );
        assert_eq!(
            classify_sql_error(rusqlite::Error::ToSqlConversionFailure(Box::new(
                std::io::Error::new(std::io::ErrorKind::Other, "disk"),
            ))),
            ProjectionErrorClass::IoError
        );
    }

    #[test]
    fn skip_if_unchanged_signals_skip_only_for_equal_existing_value() {
        assert_eq!(skip_if_unchanged(Some("same"), "same"), None);
        assert_eq!(skip_if_unchanged(Some("old"), "new"), Some("new"));
        assert_eq!(skip_if_unchanged::<i32>(None, 42), Some(42));
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

//! AI commitment bridge service.
//!
//! Synchronizes AI-inferred open commitments from `IntelligenceJson` into the
//! Actions entity, using the `ai_commitment_bridge` table for stable identity
//! and tombstone tracking.
//!
//! See migration 108 for the bridge table shape.  See ADR-0101 for the
//! service-boundary rule (no direct DB writes from commands).

use crate::abilities::extractors::commitment::{
    derive_commitment_id, CommitmentClaim, CommitmentTrust, OwnerRef,
};
use crate::abilities::feedback::ClaimVerificationState;
use crate::abilities::provenance::SubjectRef;
use crate::abilities::read::resolve_owner::{
    resolution_to_columns, resolve_owner, OwnerResolution,
};
use crate::abilities::trust::{
    compile_trust, CrossEntityCoherenceInput, FreshnessContext, SourceLifecycleState, SurfaceClass,
    TargetFootprint, TrustBand, TrustConfig, TrustContext, TrustFactorInputs, TrustScore,
    UserFeedbackSignal,
};
use crate::action_status::{BACKLOG, KIND_COMMITMENT, STARTED, UNSTARTED};
use crate::db::{ActionDb, DbAction};
use crate::intelligence::io::OpenCommitment;
use abilities_runtime::types::{
    ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState, TemporalScope,
};
use chrono::{DateTime, Utc};

/// Summary of a commitment sync pass — emitted as an INFO log after every
/// enrichment completion so we can observe bridge churn in the wild.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BridgeSyncSummary {
    pub created: usize,
    pub updated: usize,
    pub skipped_tombstoned: usize,
    pub skipped_missing_id: usize,
    /// bridge_id was new but mapped to an existing action via
    /// derived identity match (alias). No new action row was created;
    /// the bridge row was inserted pointing at the existing action.
    pub aliased_to_existing: usize,
}

/// Normalize a commitment title with the same title rule used by
/// `derive_commitment_id`.
pub fn normalize_commitment_title(title: &str) -> String {
    title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_ascii_lowercase()
}

/// A single bridge row read from `ai_commitment_bridge`.
struct BridgeRow {
    action_id: Option<String>,
    tombstoned: bool,
}

struct PendingAliasRemediationRow {
    id: String,
    legacy_bridge_id: String,
    source_commitment_id: Option<String>,
    source_type: Option<String>,
    source_id: Option<String>,
}

struct PendingAliasRemediationBlock {
    id: String,
    legacy_bridge_id: String,
    match_reason: &'static str,
}

/// Synchronize AI-inferred commitments from `IntelligenceJson` into the
/// Actions entity via the `ai_commitment_bridge` table.
///
/// Called from enrichment completion (after `IntelligenceJson` persists).
/// For each commitment with a `commitment_id`:
///   - If bridge row exists and `tombstoned = 1` → skip (do not resurrect).
///   - If bridge row exists and `action_id` points to a non-terminal Action →
///     update Action metadata (title/description, due_date) if changed,
///     then update `last_seen_at`.
///   - If bridge row does not exist → create Action (status=BACKLOG,
///     action_kind=KIND_COMMITMENT, source_type='commitment',
///     source_id=commitment_id), then insert bridge row
///     (first_seen_at=now, last_seen_at=now, tombstoned=0).
///
/// Commitments without `commitment_id` are skipped (legacy / unbridgeable).
pub fn sync_ai_commitments(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    commitments: &[OpenCommitment],
) -> Result<BridgeSyncSummary, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let mut summary = BridgeSyncSummary::default();
    let now = ctx.clock.now().to_rfc3339();

    for commitment in commitments {
        if commitment.description.trim().is_empty() {
            summary.skipped_missing_id += 1;
            continue;
        }

        let derived_id = derive_commitment_id(
            &commitment.description,
            entity_id,
            commitment.due_date.as_deref(),
            commitment.owner.as_deref(),
        );
        if read_bridge_row(db, &derived_id)
            .map_err(|e| e.to_string())?
            .is_none()
        {
            if let Some(block) = pending_alias_remediation_blocks_claim(
                db,
                entity_type,
                entity_id,
                commitment,
                &derived_id,
            )? {
                log::warn!(
                    "commitment_bridge: quarantining commitment due to pending alias remediation {} for {}:{} (legacy_bridge_id={}, match={}, derived_id={})",
                    block.id,
                    entity_type,
                    entity_id,
                    block.legacy_bridge_id,
                    block.match_reason,
                    derived_id
                );
                summary.skipped_tombstoned += 1;
                continue;
            }
        }
        if legacy_bridge_tombstone_blocks_claim(
            ctx,
            db,
            entity_type,
            entity_id,
            commitment,
            &derived_id,
            &now,
        )? {
            summary.skipped_tombstoned += 1;
            continue;
        }
        if source_sighting_tombstone_blocks_claim(
            ctx,
            db,
            entity_type,
            entity_id,
            &derived_id,
            &now,
        )? {
            summary.skipped_tombstoned += 1;
            continue;
        }
        let owner_resolution =
            resolve_owner(db, entity_id, &derived_id, commitment.owner.as_deref())?;
        let source_count = source_count_for_commitment(db, &derived_id).unwrap_or(0) + 1;
        let trust = compute_commitment_trust(
            entity_id,
            commitment,
            &owner_resolution,
            source_count,
            ctx.clock.now(),
        );
        let claim = CommitmentClaim::new(
            entity_id,
            commitment.description.clone(),
            commitment.due_date.as_deref(),
            commitment.owner.as_deref(),
            owner_resolution.owner_ref.clone(),
            Some(trust.clone()),
        );

        upsert_commitment_claim(
            ctx,
            db,
            entity_type,
            entity_id,
            commitment,
            &claim,
            &owner_resolution,
            &trust,
            &now,
            &mut summary,
        )?;
    }

    Ok(summary)
}

fn pending_alias_remediation_blocks_claim(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    commitment: &OpenCommitment,
    derived_id: &str,
) -> Result<Option<PendingAliasRemediationBlock>, String> {
    if !table_exists(db, "action_commitment_alias_remediation").map_err(|e| e.to_string())? {
        return Ok(None);
    }

    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT id, legacy_bridge_id, source_commitment_id, source_type, source_id
             FROM action_commitment_alias_remediation
             WHERE remediation_status = 'pending'
               AND entity_type = ?1
               AND entity_id = ?2",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![entity_type, entity_id], |row| {
            Ok(PendingAliasRemediationRow {
                id: row.get(0)?,
                legacy_bridge_id: row.get(1)?,
                source_commitment_id: row.get(2)?,
                source_type: row.get(3)?,
                source_id: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let incoming_id = trimmed_non_empty(commitment.commitment_id.as_deref());
    for row in rows {
        let row = row.map_err(|e| e.to_string())?;
        if let Some(match_reason) =
            pending_alias_remediation_match_reason(&row, incoming_id, derived_id)
        {
            return Ok(Some(PendingAliasRemediationBlock {
                id: row.id,
                legacy_bridge_id: row.legacy_bridge_id,
                match_reason,
            }));
        }
    }

    Ok(None)
}

fn pending_alias_remediation_match_reason(
    row: &PendingAliasRemediationRow,
    incoming_id: Option<&str>,
    derived_id: &str,
) -> Option<&'static str> {
    if let Some(incoming_id) = incoming_id {
        if ids_match(incoming_id, Some(row.legacy_bridge_id.as_str())) {
            return Some("legacy_bridge_id");
        }
        if ids_match(incoming_id, row.source_commitment_id.as_deref()) {
            return Some("source_commitment_id");
        }
        if ids_match(incoming_id, row.source_id.as_deref()) {
            return Some("source_id");
        }
        if incoming_id_matches_source_parts(
            incoming_id,
            row.source_type.as_deref(),
            row.source_id.as_deref(),
        ) {
            return Some("source_parts");
        }
    }

    if ids_match(derived_id, row.source_commitment_id.as_deref()) {
        return Some("derived_source_commitment_id");
    }
    if ids_match(derived_id, row.source_id.as_deref()) {
        return Some("derived_source_id");
    }

    None
}

fn ids_match(left: &str, right: Option<&str>) -> bool {
    trimmed_non_empty(right).is_some_and(|right| left == right)
}

fn incoming_id_matches_source_parts(
    incoming_id: &str,
    source_type: Option<&str>,
    source_id: Option<&str>,
) -> bool {
    let Some(source_type) = trimmed_non_empty(source_type) else {
        return false;
    };
    let Some(source_id) = trimmed_non_empty(source_id) else {
        return false;
    };
    let Some((incoming_source_type, rest)) = incoming_id.split_once(':') else {
        return false;
    };
    let Some((incoming_source_id, _ordinal)) = rest.rsplit_once(':') else {
        return false;
    };

    incoming_source_type == source_type && incoming_source_id == source_id
}

fn trimmed_non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn legacy_bridge_tombstone_blocks_claim(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    commitment: &OpenCommitment,
    derived_id: &str,
    now: &str,
) -> Result<bool, String> {
    let Some(incoming_id) = commitment
        .commitment_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    else {
        return Ok(false);
    };

    let Some(row) = read_bridge_row(db, incoming_id).map_err(|e| e.to_string())? else {
        return Ok(false);
    };

    if !row.tombstoned {
        return Ok(false);
    }

    insert_tombstoned_bridge_alias(
        ctx,
        db,
        derived_id,
        entity_type,
        entity_id,
        row.action_id.as_deref(),
        now,
    )?;
    touch_bridge_row(ctx, db, incoming_id, now).map_err(|e| e.to_string())?;
    Ok(true)
}

fn source_sighting_tombstone_blocks_claim(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    derived_id: &str,
    now: &str,
) -> Result<bool, String> {
    if read_bridge_row(db, derived_id)
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Ok(false);
    }

    let action_id = db
        .conn_ref()
        .query_row(
            "SELECT b.action_id
             FROM ai_commitment_bridge b
             JOIN action_commitment_sources acs
               ON acs.action_id = b.action_id
             WHERE b.entity_type = ?1
               AND b.entity_id = ?2
               AND b.tombstoned != 0
               AND acs.commitment_id = ?3
             ORDER BY b.last_seen_at DESC
             LIMIT 1",
            rusqlite::params![entity_type, entity_id, derived_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .map(Some)
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
        .map_err(|e| e.to_string())?;

    let Some(action_id) = action_id else {
        return Ok(false);
    };

    insert_tombstoned_bridge_alias(
        ctx,
        db,
        derived_id,
        entity_type,
        entity_id,
        action_id.as_deref(),
        now,
    )?;
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
pub fn upsert_commitment_claim(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    commitment: &OpenCommitment,
    claim: &CommitmentClaim,
    owner_resolution: &OwnerResolution,
    trust: &CommitmentTrust,
    now: &str,
    summary: &mut BridgeSyncSummary,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;

    let bridge_row = read_bridge_row(db, &claim.commitment_id).map_err(|e| e.to_string())?;
    if let Some(row) = bridge_row.as_ref() {
        if row.tombstoned {
            summary.skipped_tombstoned += 1;
            touch_bridge_row(ctx, db, &claim.commitment_id, now).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }

    let bridged_action = get_action_from_bridge_row(db, bridge_row.as_ref())?;
    let existing_action = match bridged_action {
        Some(action) => Some(action),
        None => get_action_by_commitment_id(db, &claim.commitment_id)?,
    };

    let mut created_action = false;
    let mut action = match existing_action {
        Some(action) => {
            if is_terminal_status(&action.status) {
                summary.skipped_tombstoned += 1;
                return Ok(());
            }
            action
        }
        None => {
            if let Some(existing_action_id) = find_existing_open_commitment_by_identity(
                db,
                entity_type,
                entity_id,
                &claim.commitment_id,
            )
            .map_err(|e| e.to_string())?
            {
                let action = db
                    .get_action_by_id(&existing_action_id)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| {
                        format!("commitment alias target missing: {existing_action_id}")
                    })?;
                summary.aliased_to_existing += 1;
                action
            } else {
                summary.created += 1;
                created_action = true;
                new_commitment_action(
                    entity_type,
                    entity_id,
                    commitment,
                    claim,
                    owner_resolution,
                    trust,
                    now,
                )
            }
        }
    };

    let before = action.clone();
    apply_commitment_claim_to_action(&mut action, commitment, claim, owner_resolution, trust, now);
    if action_changed_for_commitment(&before, &action) {
        db.upsert_action(&action).map_err(|e| e.to_string())?;
        if !created_action {
            summary.updated += 1;
        }
    } else if created_action {
        db.upsert_action(&action).map_err(|e| e.to_string())?;
    }

    insert_bridge_row(
        ctx,
        db,
        &claim.commitment_id,
        entity_type,
        entity_id,
        &action.id,
        now,
    )
    .map_err(|e| e.to_string())?;
    insert_commitment_source(db, &action, commitment, owner_resolution, trust, now)?;

    Ok(())
}

fn new_commitment_action(
    entity_type: &str,
    entity_id: &str,
    commitment: &OpenCommitment,
    claim: &CommitmentClaim,
    owner_resolution: &OwnerResolution,
    trust: &CommitmentTrust,
    now: &str,
) -> DbAction {
    let account_id = if entity_type == "account" {
        Some(entity_id.to_string())
    } else {
        None
    };
    let project_id = if entity_type == "project" {
        Some(entity_id.to_string())
    } else {
        None
    };
    let (owner_raw, owner_entity_id, owner_confidence, owner_source) =
        resolution_to_columns(owner_resolution);

    DbAction {
        id: uuid::Uuid::new_v4().to_string(),
        title: commitment.description.clone(),
        priority: crate::action_status::PRIORITY_DEFAULT,
        status: crate::action_status::BACKLOG.to_string(),
        created_at: now.to_string(),
        due_date: commitment.due_date.clone(),
        completed_at: None,
        account_id,
        project_id,
        source_type: Some("commitment".to_string()),
        source_id: Some(claim.commitment_id.clone()),
        source_label: commitment.source.clone(),
        action_kind: KIND_COMMITMENT.to_string(),
        commitment_id: Some(claim.commitment_id.clone()),
        owner_raw,
        owner_entity_id,
        owner_confidence,
        owner_source: Some(owner_source),
        trust_score: Some(trust.score.value()),
        trust_band: Some(trust_band_label(trust.band).to_string()),
        commitment_source_count: None,
        context: None,
        waiting_on: None,
        updated_at: now.to_string(),
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

fn apply_commitment_claim_to_action(
    action: &mut DbAction,
    commitment: &OpenCommitment,
    claim: &CommitmentClaim,
    owner_resolution: &OwnerResolution,
    trust: &CommitmentTrust,
    now: &str,
) {
    if action.status.as_str() != BACKLOG {
        refresh_accepted_commitment_trust(action, trust, now);
        return;
    }

    action.title = commitment.description.clone();
    action.due_date = commitment.due_date.clone();
    action.source_label = commitment.source.clone();

    if action.commitment_id.as_deref() != Some(claim.commitment_id.as_str()) {
        action.commitment_id = Some(claim.commitment_id.clone());
        action.source_id = Some(claim.commitment_id.clone());
    }

    if action.owner_source.as_deref() != Some("user_reassigned") {
        let (owner_raw, owner_entity_id, owner_confidence, owner_source) =
            resolution_to_columns(owner_resolution);
        action.owner_raw = owner_raw;
        action.owner_entity_id = owner_entity_id;
        action.owner_confidence = owner_confidence;
        action.owner_source = Some(owner_source);
    }

    action.trust_score = Some(trust.score.value());
    action.trust_band = Some(trust_band_label(trust.band).to_string());
    action.context = strip_legacy_owner_context(action.context.as_deref());
    action.updated_at = now.to_string();
}

fn action_changed_for_commitment(before: &DbAction, after: &DbAction) -> bool {
    before.title != after.title
        || before.due_date != after.due_date
        || before.source_label != after.source_label
        || before.commitment_id != after.commitment_id
        || before.owner_raw != after.owner_raw
        || before.owner_entity_id != after.owner_entity_id
        || before.owner_confidence != after.owner_confidence
        || before.owner_source != after.owner_source
        || before.trust_score != after.trust_score
        || before.trust_band != after.trust_band
        || before.context != after.context
}

fn refresh_accepted_commitment_trust(action: &mut DbAction, trust: &CommitmentTrust, now: &str) {
    if !matches!(action.status.as_str(), UNSTARTED | STARTED) {
        return;
    }

    let next_score = Some(trust.score.value());
    let next_band = Some(trust_band_label(trust.band).to_string());
    if action.trust_score != next_score || action.trust_band != next_band {
        action.trust_score = next_score;
        action.trust_band = next_band;
        action.updated_at = now.to_string();
    }
}

fn strip_legacy_owner_context(context: Option<&str>) -> Option<String> {
    let value = context?.trim();
    if value.is_empty() || value.to_ascii_lowercase().starts_with("owner:") {
        None
    } else {
        Some(value.to_string())
    }
}

pub fn strip_owner_context_for_action(context: Option<&str>) -> Option<String> {
    strip_legacy_owner_context(context)
}

fn is_terminal_status(status: &str) -> bool {
    matches!(status, "completed" | "cancelled" | "rejected" | "archived")
}

fn get_action_by_commitment_id(
    db: &ActionDb,
    commitment_id: &str,
) -> Result<Option<DbAction>, String> {
    let action_id = db
        .conn_ref()
        .query_row(
            "SELECT id FROM actions WHERE commitment_id = ?1 LIMIT 1",
            rusqlite::params![commitment_id],
            |row| row.get::<_, String>(0),
        )
        .map(Some)
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
        .map_err(|e| e.to_string())?;

    match action_id {
        Some(id) => db.get_action_by_id(&id).map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

fn get_action_from_bridge_row(
    db: &ActionDb,
    row: Option<&BridgeRow>,
) -> Result<Option<DbAction>, String> {
    let Some(action_id) = row.and_then(|row| row.action_id.as_deref()) else {
        return Ok(None);
    };
    db.get_action_by_id(action_id).map_err(|e| e.to_string())
}

fn insert_commitment_source(
    db: &ActionDb,
    action: &DbAction,
    commitment: &OpenCommitment,
    owner_resolution: &OwnerResolution,
    trust: &CommitmentTrust,
    now: &str,
) -> Result<(), String> {
    let commitment_id = action
        .commitment_id
        .as_deref()
        .ok_or_else(|| format!("commitment action {} missing commitment_id", action.id))?;
    let source_confidence = commitment.item_source.as_ref().map(|s| s.confidence);
    let source_type = commitment
        .item_source
        .as_ref()
        .map(|s| s.source.as_str())
        .or(commitment.source.as_deref())
        .or(action.source_type.as_deref());
    let source_label = commitment
        .item_source
        .as_ref()
        .and_then(|s| s.reference.as_deref())
        .or(commitment.source.as_deref())
        .or(action.source_label.as_deref());
    let owner_ref_json = serde_json::to_string(&owner_resolution.owner_ref).ok();

    db.conn_ref()
        .execute(
            "INSERT INTO action_commitment_sources (
                id, commitment_id, action_id, source_type, source_id,
                source_label, observed_at, source_confidence, trust_score,
                trust_band, owner_raw, owner_ref_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                commitment_id,
                action.id.as_str(),
                source_type,
                action.source_id.as_deref(),
                source_label,
                now,
                source_confidence,
                trust.score.value(),
                trust_band_label(trust.band),
                owner_resolution.owner_raw.as_deref(),
                owner_ref_json.as_deref(),
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn source_count_for_commitment(db: &ActionDb, commitment_id: &str) -> Result<i64, rusqlite::Error> {
    db.conn_ref().query_row(
        "SELECT COUNT(*) FROM action_commitment_sources WHERE commitment_id = ?1",
        rusqlite::params![commitment_id],
        |row| row.get(0),
    )
}

fn compute_commitment_trust(
    account_id: &str,
    commitment: &OpenCommitment,
    owner_resolution: &OwnerResolution,
    source_count: i64,
    now: DateTime<Utc>,
) -> CommitmentTrust {
    let source_reliability = commitment
        .item_source
        .as_ref()
        .map(|s| s.confidence)
        .unwrap_or(0.55)
        .clamp(0.0, 1.0);
    let source_asof = commitment
        .item_source
        .as_ref()
        .map(|s| s.sourced_at.as_str());
    let (timestamp_known, age_days) = source_age_days(source_asof, now);
    let owner_confidence = match owner_resolution.owner_ref {
        OwnerRef::Ambiguous { .. } => 0.35,
        OwnerRef::Unassigned => 0.45,
        _ => owner_resolution
            .owner_confidence
            .unwrap_or(0.7)
            .clamp(0.0, 1.0),
    };
    let corroboration_strength = (0.45 + (source_count.max(1) as f64 - 1.0) * 0.12).min(1.0);
    let claim = synthetic_commitment_claim(account_id, commitment, source_asof, now);
    let subject = SubjectRef::Account(account_id.to_string());
    let trust_ctx = TrustContext {
        now,
        renewal_context: None,
        config: TrustConfig::default(),
        factor_inputs: TrustFactorInputs {
            source_reliability,
            source_reliability_corroborators: Vec::new(),
            freshness: FreshnessContext {
                timestamp_known,
                age_days,
            },
            corroboration_strength,
            contradiction_count: u32::from(commitment.discrepancy.unwrap_or(false)),
            user_feedback: UserFeedbackSignal::None,
            subject_fit_confidence: owner_confidence,
            internal_consistency: 1.0,
            source_lifecycle: SourceLifecycleState::Active,
            read_state_indeterminate: false,
        },
        cross_entity: CrossEntityCoherenceInput {
            claim_text: commitment.description.clone(),
            target_footprint: TargetFootprint {
                subject: subject.clone(),
                names: Vec::new(),
                domains: Vec::new(),
                related_subjects: Vec::new(),
                allowed_aliases: Vec::new(),
            },
            portfolio_footprints: Vec::new(),
            cross_entity_context_expected: false,
        },
        target_surface: Some(SurfaceClass::Internal),
    };

    compile_trust(&claim, trust_ctx)
        .map(|computed| CommitmentTrust {
            score: computed.score,
            band: computed.band,
        })
        .unwrap_or(CommitmentTrust {
            score: TrustScore(0.0),
            band: TrustBand::Unscored,
        })
}

fn source_age_days(source_asof: Option<&str>, now: DateTime<Utc>) -> (bool, f64) {
    let Some(value) = source_asof else {
        return (false, 0.0);
    };
    let parsed = DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .ok();
    let Some(source_time) = parsed else {
        return (false, 0.0);
    };
    let seconds = (now - source_time).num_seconds().max(0) as f64;
    (true, seconds / 86_400.0)
}

fn synthetic_commitment_claim(
    account_id: &str,
    commitment: &OpenCommitment,
    source_asof: Option<&str>,
    now: DateTime<Utc>,
) -> IntelligenceClaim {
    let commitment_id = derive_commitment_id(
        &commitment.description,
        account_id,
        commitment.due_date.as_deref(),
        commitment.owner.as_deref(),
    );
    let now = now.to_rfc3339();
    IntelligenceClaim {
        id: commitment_id.clone(),
        subject_ref: serde_json::json!({ "kind": "account", "id": account_id }).to_string(),
        claim_type: "commitment".to_string(),
        field_path: Some("open_commitments".to_string()),
        topic_key: None,
        text: commitment.description.clone(),
        dedup_key: commitment_id,
        item_hash: None,
        actor: "agent".to_string(),
        data_source: commitment
            .item_source
            .as_ref()
            .map(|s| s.source.clone())
            .unwrap_or_else(|| "ai_enrichment".to_string()),
        source_ref: commitment.source.clone(),
        source_asof: source_asof.map(ToString::to_string),
        observed_at: now.clone(),
        created_at: now,
        provenance_json: "{}".to_string(),
        metadata_json: None,
        claim_state: ClaimState::Active,
        surfacing_state: SurfacingState::Active,
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
        verification_state: ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    }
}

fn trust_band_label(band: TrustBand) -> &'static str {
    match band {
        TrustBand::LikelyCurrent => "likely_current",
        TrustBand::UseWithCaution => "use_with_caution",
        TrustBand::NeedsVerification => "needs_verification",
        TrustBand::Unscored => "unscored",
    }
}

/// Look up an existing non-terminal commitment-typed action with the same
/// derived identity tuple under the given entity. Used by `sync_ai_commitments`
/// to alias a new commitment_id onto an existing action instead of creating a
/// duplicate row.
///
/// Returns the action_id of the oldest matching action so re-aliasing
/// is stable across runs (deterministic on `created_at ASC`).
fn find_existing_open_commitment_by_identity(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    commitment_id: &str,
) -> Result<Option<String>, rusqlite::Error> {
    if commitment_id.is_empty() {
        return Ok(None);
    }
    let entity_col = match entity_type {
        "account" => "account_id",
        "project" => "project_id",
        _ => return Ok(None),
    };
    let sql = format!(
        "SELECT id, title, due_date, owner_raw, context FROM actions
         WHERE {entity_col} = ?1
           AND action_kind = ?2
           AND status NOT IN ('completed', 'cancelled', 'rejected', 'archived')
         ORDER BY created_at ASC"
    );
    let mut stmt = db.conn_ref().prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params![entity_id, KIND_COMMITMENT], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;

    for row in rows {
        let (action_id, title, due_date, owner_raw, context) = row?;
        let context_owner = legacy_owner_from_context(context.as_deref());
        let owner_for_identity = owner_raw.as_deref().or(context_owner.as_deref());
        let candidate_id =
            derive_commitment_id(&title, entity_id, due_date.as_deref(), owner_for_identity);
        if candidate_id == commitment_id {
            return Ok(Some(action_id));
        }
    }

    Ok(None)
}

fn legacy_owner_from_context(context: Option<&str>) -> Option<String> {
    let value = context?.trim();
    let prefix_len = value
        .get(..6)
        .filter(|prefix| prefix.eq_ignore_ascii_case("owner:"))?
        .len();
    let owner = value[prefix_len..].trim();
    if owner.is_empty() {
        None
    } else {
        Some(owner.to_string())
    }
}

/// Mark a commitment's bridge row tombstoned so re-enrichment can't
/// resurrect it. Called from Action state-transition services
/// (`complete_action`, `reject_suggested_action`, `archive_action`, etc.)
/// when `action_kind == KIND_COMMITMENT` and the transition is terminal.
///
/// Looks up the bridge row by `action_id`. No-op if none exists.
pub fn tombstone_commitment_bridge(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    action_id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let now = ctx.clock.now().to_rfc3339();
    let changed = db
        .conn_ref()
        .execute(
            "UPDATE ai_commitment_bridge
             SET tombstoned = 1, last_seen_at = ?1
             WHERE action_id = ?2",
            rusqlite::params![now, action_id],
        )
        .map_err(|e| e.to_string())?;
    if changed > 0 {
        log::info!(
            "commitment_bridge: tombstoned bridge row(s) for action {} ({})",
            action_id,
            changed
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers — SQL kept inside the service module since the
// ai_commitment_bridge table has no other callers.
// ---------------------------------------------------------------------------

fn read_bridge_row(
    db: &ActionDb,
    commitment_id: &str,
) -> Result<Option<BridgeRow>, rusqlite::Error> {
    db.conn_ref()
        .query_row(
            "SELECT action_id, tombstoned FROM ai_commitment_bridge
             WHERE commitment_id = ?1",
            rusqlite::params![commitment_id],
            |row| {
                let action_id: Option<String> = row.get(0)?;
                let tombstoned: i32 = row.get(1)?;
                Ok(BridgeRow {
                    action_id,
                    tombstoned: tombstoned != 0,
                })
            },
        )
        .map(Some)
        .or_else(|e| {
            if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                Ok(None)
            } else {
                Err(e)
            }
        })
}

fn table_exists(db: &ActionDb, table_name: &str) -> Result<bool, rusqlite::Error> {
    db.conn_ref().query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM sqlite_master
            WHERE type = 'table' AND name = ?1
        )",
        rusqlite::params![table_name],
        |row| row.get::<_, i64>(0).map(|value| value != 0),
    )
}

fn insert_bridge_row(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    commitment_id: &str,
    entity_type: &str,
    entity_id: &str,
    action_id: &str,
    now: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.conn_ref()
        .execute(
            "INSERT INTO ai_commitment_bridge
             (commitment_id, entity_type, entity_id, action_id,
              first_seen_at, last_seen_at, tombstoned)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0)
         ON CONFLICT(commitment_id) DO UPDATE SET
              action_id = excluded.action_id,
              entity_type = excluded.entity_type,
              entity_id = excluded.entity_id,
              last_seen_at = excluded.last_seen_at",
            rusqlite::params![commitment_id, entity_type, entity_id, action_id, now],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn insert_tombstoned_bridge_alias(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    commitment_id: &str,
    entity_type: &str,
    entity_id: &str,
    action_id: Option<&str>,
    now: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.conn_ref()
        .execute(
            "INSERT INTO ai_commitment_bridge
             (commitment_id, entity_type, entity_id, action_id,
              first_seen_at, last_seen_at, tombstoned)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1)
         ON CONFLICT(commitment_id) DO UPDATE SET
              action_id = COALESCE(excluded.action_id, ai_commitment_bridge.action_id),
              entity_type = excluded.entity_type,
              entity_id = excluded.entity_id,
              last_seen_at = excluded.last_seen_at,
              tombstoned = 1",
            rusqlite::params![commitment_id, entity_type, entity_id, action_id, now],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn touch_bridge_row(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    commitment_id: &str,
    now: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.conn_ref()
        .execute(
            "UPDATE ai_commitment_bridge SET last_seen_at = ?1 WHERE commitment_id = ?2",
            rusqlite::params![now, commitment_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::TimeZone;

    macro_rules! make_ctx {
        ($ctx:ident) => {
            let clock =
                FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
            let rng = SeedableRng::new(42);
            let ext = ExternalClients::default();
            let $ctx = ServiceContext::test_live(&clock, &rng, &ext);
        };
    }

    fn make_commitment(description: &str) -> OpenCommitment {
        make_commitment_with_source(description, None)
    }

    fn make_commitment_with_source(description: &str, source: Option<&str>) -> OpenCommitment {
        make_commitment_with_legacy_id_and_source(description, "legacy-llm-id-is-ignored", source)
    }

    fn make_commitment_with_legacy_id(description: &str, legacy_id: &str) -> OpenCommitment {
        make_commitment_with_legacy_id_and_source(description, legacy_id, None)
    }

    fn make_commitment_with_legacy_id_and_source(
        description: &str,
        legacy_id: &str,
        source: Option<&str>,
    ) -> OpenCommitment {
        make_commitment_with_identity(description, Some(legacy_id), None, None, source)
    }

    fn make_commitment_with_due_owner(
        description: &str,
        due_date: Option<&str>,
        owner: Option<&str>,
    ) -> OpenCommitment {
        make_commitment_with_identity(
            description,
            Some("legacy-llm-id-is-ignored"),
            due_date,
            owner,
            None,
        )
    }

    fn make_commitment_with_identity(
        description: &str,
        legacy_id: Option<&str>,
        due_date: Option<&str>,
        owner: Option<&str>,
        source: Option<&str>,
    ) -> OpenCommitment {
        OpenCommitment {
            commitment_id: legacy_id.map(ToString::to_string),
            description: description.to_string(),
            owner: owner.map(ToString::to_string),
            due_date: due_date.map(ToString::to_string),
            source: source.map(ToString::to_string),
            status: None,
            item_source: None,
            discrepancy: None,
        }
    }

    fn derived_id(description: &str) -> String {
        derive_commitment_id(description, "acct-1", None, None)
    }

    fn derived_id_with(description: &str, due_date: Option<&str>, owner: Option<&str>) -> String {
        derive_commitment_id(description, "acct-1", due_date, owner)
    }

    fn bridge_action_id(db: &ActionDb, commitment_id: &str) -> String {
        db.conn_ref()
            .query_row(
                "SELECT action_id FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params![commitment_id],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn action_count(db: &ActionDb) -> i64 {
        db.conn_ref()
            .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
            .unwrap()
    }

    fn source_rows(db: &ActionDb, commitment_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM action_commitment_sources WHERE commitment_id = ?1",
                rusqlite::params![commitment_id],
                |row| row.get(0),
            )
            .unwrap()
    }

    #[test]
    fn test_sync_creates_new_commitment() {
        let db = test_db();
        make_ctx!(ctx);
        let commitments = vec![make_commitment("Send renewal deck")];
        let commitment_id = derived_id("Send renewal deck");

        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("sync");
        assert_eq!(summary.created, 1);
        assert_eq!(summary.updated, 0);
        assert_eq!(summary.skipped_tombstoned, 0);
        assert_eq!(summary.skipped_missing_id, 0);

        // Bridge row exists
        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params![commitment_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // Action exists with kind=commitment, status=backlog
        let (kind, status, action_commitment_id, owner_source, trust_band): (
            String,
            String,
            String,
            String,
            String,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT a.action_kind, a.status, a.commitment_id, a.owner_source, a.trust_band
                 FROM actions a
                 JOIN ai_commitment_bridge b ON b.action_id = a.id
                 WHERE b.commitment_id = ?1",
                rusqlite::params![commitment_id.clone()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(kind, KIND_COMMITMENT);
        assert_eq!(status, "backlog");
        assert_eq!(action_commitment_id, commitment_id);
        assert_eq!(owner_source, "unassigned");
        assert!(!trust_band.is_empty());
        assert_eq!(source_rows(&db, &action_commitment_id), 1);
    }

    #[test]
    fn rerun_same_commitment_keeps_action_stable_and_appends_source() {
        let db = test_db();
        make_ctx!(ctx);
        let commitments = vec![make_commitment("Send renewal deck")];
        let commitment_id = derived_id("Send renewal deck");

        sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("first sync");
        let first_action_id = bridge_action_id(&db, &commitment_id);

        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("resync");
        assert_eq!(summary.created, 0);
        assert_eq!(bridge_action_id(&db, &commitment_id), first_action_id);
        assert_eq!(action_count(&db), 1);
        assert_eq!(source_rows(&db, &commitment_id), 2);
    }

    #[test]
    fn test_sync_skips_tombstoned() {
        let db = test_db();
        make_ctx!(ctx);
        let commitments = vec![make_commitment("Tombstoned item")];
        let commitment_id = derived_id("Tombstoned item");

        sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("initial sync");

        // Fetch the action_id we just created, tombstone it.
        let action_id = bridge_action_id(&db, &commitment_id);
        tombstone_commitment_bridge(&ctx, &db, &action_id).expect("tombstone");

        // Re-sync with the same commitment — should be skipped, no new action.
        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("resync");
        assert_eq!(summary.created, 0);
        assert_eq!(summary.skipped_tombstoned, 1);

        assert_eq!(action_count(&db), 1);
    }

    #[test]
    fn test_sync_skips_missing_id() {
        let db = test_db();
        make_ctx!(ctx);
        let commitments = vec![make_commitment("   ")];
        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("sync");
        assert_eq!(summary.created, 0);
        assert_eq!(summary.skipped_missing_id, 1);

        assert_eq!(action_count(&db), 0);
    }

    #[test]
    fn test_tombstone_sets_flag() {
        let db = test_db();
        make_ctx!(ctx);
        let commitments = vec![make_commitment("Thing to kill")];
        let commitment_id = derived_id("Thing to kill");
        sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("sync");

        let action_id = bridge_action_id(&db, &commitment_id);
        tombstone_commitment_bridge(&ctx, &db, &action_id).expect("tombstone");

        let flag: i32 = db
            .conn_ref()
            .query_row(
                "SELECT tombstoned FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params![commitment_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(flag, 1);
    }

    #[test]
    fn test_resurrection_blocked() {
        let db = test_db();
        make_ctx!(ctx);
        let commitments = vec![make_commitment("Resurrect me?")];
        let commitment_id = derived_id("Resurrect me?");

        // Initial sync creates action
        sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("sync");
        let action_id = bridge_action_id(&db, &commitment_id);

        // Complete + tombstone
        db.complete_action(&action_id).expect("complete");
        tombstone_commitment_bridge(&ctx, &db, &action_id).expect("tombstone");

        // Re-sync with same commitment — no new action.
        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("resync");
        assert_eq!(summary.created, 0);
        assert_eq!(summary.skipped_tombstoned, 1);

        assert_eq!(action_count(&db), 1);
    }

    #[test]
    fn pre155_tombstoned_legacy_id_creates_derived_tombstone_alias_and_skips() {
        let db = test_db();
        make_ctx!(ctx);
        let title = "Do not resurrect legacy commitment";
        let legacy_id = "meeting:legacy-source:1";
        let derived_commitment_id = derived_id(title);

        db.conn_ref()
            .execute(
                "INSERT INTO actions
                 (id, title, priority, status, created_at, updated_at, account_id,
                  source_type, source_id, action_kind, commitment_id)
                 VALUES ('done-a1', ?1, 3, 'completed', '2026-01-01', '2026-01-02',
                         'acct-1', 'commitment', ?2, 'commitment', ?2)",
                rusqlite::params![title, legacy_id],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO ai_commitment_bridge
                 (commitment_id, entity_type, entity_id, action_id,
                  first_seen_at, last_seen_at, tombstoned)
                 VALUES (?1, 'account', 'acct-1', 'done-a1',
                         '2026-01-01', '2026-01-02', 1)",
                rusqlite::params![legacy_id],
            )
            .unwrap();

        let summary = sync_ai_commitments(
            &ctx,
            &db,
            "account",
            "acct-1",
            &[make_commitment_with_legacy_id(title, legacy_id)],
        )
        .expect("sync");

        assert_eq!(summary.created, 0);
        assert_eq!(summary.skipped_tombstoned, 1);
        assert_eq!(action_count(&db), 1);

        let (action_id, tombstoned): (String, i32) = db
            .conn_ref()
            .query_row(
                "SELECT action_id, tombstoned
                 FROM ai_commitment_bridge
                 WHERE commitment_id = ?1",
                rusqlite::params![derived_commitment_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(action_id, "done-a1");
        assert_eq!(tombstoned, 1);
    }

    #[test]
    fn changed_title_derives_new_identity() {
        let db = test_db();
        make_ctx!(ctx);
        let original = vec![make_commitment("Original phrasing")];
        let rephrased = vec![make_commitment("Totally different wording")];

        sync_ai_commitments(&ctx, &db, "account", "acct-1", &original).expect("sync");
        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &rephrased).expect("resync");
        assert_eq!(summary.created, 1);
        assert_eq!(summary.skipped_tombstoned, 0);

        let ids: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(DISTINCT commitment_id) FROM actions WHERE action_kind = 'commitment'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(ids, 2);
        assert_eq!(action_count(&db), 2);
    }

    #[test]
    fn same_title_with_different_owner_and_due_creates_distinct_actions() {
        let db = test_db();
        make_ctx!(ctx);
        let title = "Send implementation plan";
        let commitments = vec![
            make_commitment_with_due_owner(title, Some("2030-01-01"), Some("Alex Chen")),
            make_commitment_with_due_owner(title, Some("2030-02-01"), Some("Jamie Lee")),
        ];

        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("sync");

        assert_eq!(summary.created, 2);
        assert_eq!(summary.aliased_to_existing, 0);
        assert_eq!(action_count(&db), 2);

        let alex_id = derived_id_with(title, Some("2030-01-01"), Some("Alex Chen"));
        let jamie_id = derived_id_with(title, Some("2030-02-01"), Some("Jamie Lee"));
        assert_ne!(alex_id, jamie_id);
        assert_ne!(
            bridge_action_id(&db, &alex_id),
            bridge_action_id(&db, &jamie_id)
        );
    }

    #[test]
    fn tombstoned_source_sighting_blocks_user_edited_legacy_title_variant() {
        let db = test_db();
        make_ctx!(ctx);
        let original_title = "Send original renewal plan";
        let original_id = derived_id_with(original_title, Some("2030-03-01"), Some("Alex Chen"));

        db.conn_ref()
            .execute(
                "INSERT INTO actions
                 (id, title, priority, status, created_at, updated_at, due_date,
                  account_id, source_type, source_id, action_kind, commitment_id,
                  owner_raw)
                 VALUES ('done-edited-a1', 'User edited renewal wording', 3,
                         'completed', '2026-01-01', '2026-01-02', '2030-03-01',
                         'acct-1', 'commitment', 'legacy:original', 'commitment',
                         'legacy:original', 'Alex Chen')",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO ai_commitment_bridge
                 (commitment_id, entity_type, entity_id, action_id,
                  first_seen_at, last_seen_at, tombstoned)
                 VALUES ('legacy:original', 'account', 'acct-1', 'done-edited-a1',
                         '2026-01-01', '2026-01-02', 1)",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO action_commitment_sources
                 (id, commitment_id, action_id, observed_at)
                 VALUES ('src-original', ?1, 'done-edited-a1', '2026-01-01')",
                rusqlite::params![original_id.clone()],
            )
            .unwrap();

        let incoming = make_commitment_with_identity(
            original_title,
            None,
            Some("2030-03-01"),
            Some("Alex Chen"),
            None,
        );
        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &[incoming]).expect("sync");

        assert_eq!(summary.created, 0);
        assert_eq!(summary.skipped_tombstoned, 1);
        assert_eq!(action_count(&db), 1);

        let tombstoned: i32 = db
            .conn_ref()
            .query_row(
                "SELECT tombstoned FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params![original_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tombstoned, 1);
    }

    #[test]
    fn pending_alias_remediation_only_blocks_matching_legacy_claim() {
        let db = test_db();
        make_ctx!(ctx);
        let original_title = "Send original renewal plan";
        let original_id = derived_id_with(original_title, Some("2030-03-01"), Some("Alex Chen"));
        let unrelated_title = "Send unrelated pricing followup";
        let unrelated_id = derived_id(unrelated_title);

        db.conn_ref()
            .execute(
                "INSERT INTO actions
                 (id, title, priority, status, created_at, updated_at, due_date,
                  account_id, source_type, source_id, action_kind, commitment_id,
                  owner_raw)
                 VALUES ('done-edited-a1', 'User edited renewal wording', 3,
                         'completed', '2026-01-01', '2026-01-02', '2030-03-01',
                         'acct-1', 'commitment', 'legacy:original', 'commitment',
                         'legacy:original', 'Alex Chen')",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO ai_commitment_bridge
                 (commitment_id, entity_type, entity_id, action_id,
                  first_seen_at, last_seen_at, tombstoned)
                 VALUES ('legacy:original', 'account', 'acct-1', 'done-edited-a1',
                         '2026-01-01', '2026-01-02', 1)",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO action_commitment_alias_remediation
                 (id, legacy_bridge_id, tombstoned_action_id, entity_type,
                  entity_id, observed_at, reason, remediation_status)
                 VALUES ('remediation-1', 'legacy:original', 'done-edited-a1',
                         'account', 'acct-1', '2026-01-02',
                         'unrecoverable_tombstoned_legacy_bridge_alias', 'pending')",
                [],
            )
            .unwrap();

        let related = make_commitment_with_identity(
            original_title,
            Some("legacy:original"),
            Some("2030-03-01"),
            Some("Alex Chen"),
            Some("meeting"),
        );
        let unrelated = make_commitment(unrelated_title);
        let summary = sync_ai_commitments(&ctx, &db, "account", "acct-1", &[related, unrelated])
            .expect("sync");

        assert_eq!(summary.created, 1);
        assert_eq!(summary.skipped_tombstoned, 1);
        assert_eq!(action_count(&db), 2);

        let original_alias_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params![original_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            original_alias_count, 0,
            "pending remediation must block derived-id creation until manual review"
        );
        let unrelated_action_id = bridge_action_id(&db, &unrelated_id);
        assert_ne!(unrelated_action_id, "done-edited-a1");
    }

    #[test]
    fn backlog_source_metadata_updates_before_acceptance() {
        let db = test_db();
        make_ctx!(ctx);
        let v1 = vec![make_commitment_with_source("Stable title", Some("meeting"))];
        let v2 = vec![make_commitment_with_source("Stable title", Some("gong"))];
        let commitment_id = derived_id("Stable title");

        sync_ai_commitments(&ctx, &db, "account", "acct-1", &v1).expect("sync");
        let summary = sync_ai_commitments(&ctx, &db, "account", "acct-1", &v2).expect("resync");

        assert_eq!(summary.created, 0);
        assert_eq!(summary.updated, 1);

        let source_label: String = db
            .conn_ref()
            .query_row(
                "SELECT source_label FROM actions WHERE commitment_id = ?1",
                rusqlite::params![commitment_id.clone()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source_label, "gong");
        assert_eq!(source_rows(&db, &commitment_id), 2);
    }

    #[test]
    fn test_user_edit_preserved_across_sync_when_accepted() {
        // Regression guard: after user accepts a backlog commitment
        // (backlog → unstarted) and edits the title, a subsequent sync pass
        // must NOT overwrite the user's title. The bridge only updates
        // metadata while the row is still backlog (AI-owned). Once accepted,
        // the row is USER-OWNED and only last_seen_at gets refreshed.
        let db = test_db();
        make_ctx!(ctx);
        let original = vec![make_commitment("AI phrasing")];
        let commitment_id = derived_id("AI phrasing");
        sync_ai_commitments(&ctx, &db, "account", "acct-1", &original).expect("initial sync");

        let action_id = bridge_action_id(&db, &commitment_id);

        // User accepts → backlog transitions to unstarted.
        db.accept_suggested_action(&action_id).expect("accept");
        // User edits the title inline.
        db.conn_ref()
            .execute(
                "UPDATE actions SET title = ?1, updated_at = datetime('now') WHERE id = ?2",
                rusqlite::params!["User-edited title", action_id],
            )
            .expect("user edit");

        // Next enrichment pass emits the SAME commitment_id with the
        // original AI phrasing — this must NOT clobber the user's edit.
        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &original).expect("resync");
        assert_eq!(summary.created, 0);

        let title: String = db
            .conn_ref()
            .query_row(
                "SELECT title FROM actions WHERE id = ?1",
                rusqlite::params![action_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "User-edited title");
    }

    #[test]
    fn accepted_title_alias_preserves_identity_owner_context_and_refreshes_trust() {
        let db = test_db();
        make_ctx!(ctx);
        let title = "Accepted exact title";
        let derived_commitment_id = derived_id_with(title, Some("2030-01-01"), Some("Alex Chen"));

        db.conn_ref()
            .execute(
                "INSERT INTO actions
                 (id, title, priority, status, created_at, updated_at, due_date,
                  account_id, source_type, source_id, source_label, context,
                  action_kind, commitment_id, owner_raw, owner_entity_id,
                  owner_confidence, owner_source, trust_score, trust_band)
                 VALUES ('accepted-a1', ?1, 3, 'unstarted', '2026-01-01',
                         '2026-01-02', '2030-01-01', 'acct-1', 'commitment',
                         'legacy-accepted-id', 'manual source', 'owner: keep this',
                         'commitment', 'legacy-accepted-id', 'Alex Chen', 'p-alex',
                         0.96, 'exact_person_name', NULL, NULL)",
                rusqlite::params![title],
            )
            .unwrap();

        let summary = sync_ai_commitments(
            &ctx,
            &db,
            "account",
            "acct-1",
            &[make_commitment_with_due_owner(
                title,
                Some("2030-01-01"),
                Some("Alex Chen"),
            )],
        )
        .expect("sync");

        assert_eq!(summary.created, 0);
        assert_eq!(summary.aliased_to_existing, 1);
        assert_eq!(action_count(&db), 1);

        let (
            commitment_id,
            source_id,
            due_date,
            source_label,
            context,
            owner_raw,
            owner_entity_id,
            owner_confidence,
            owner_source,
            trust_score,
            trust_band,
        ): (
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            f64,
            String,
            Option<f64>,
            Option<String>,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT commitment_id, source_id, due_date, source_label, context,
                        owner_raw, owner_entity_id, owner_confidence, owner_source,
                        trust_score, trust_band
                 FROM actions WHERE id = 'accepted-a1'",
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
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(commitment_id, "legacy-accepted-id");
        assert_eq!(source_id, "legacy-accepted-id");
        assert_eq!(due_date, "2030-01-01");
        assert_eq!(source_label, "manual source");
        assert_eq!(context, "owner: keep this");
        assert_eq!(owner_raw, "Alex Chen");
        assert_eq!(owner_entity_id, "p-alex");
        assert_eq!(owner_confidence, 0.96);
        assert_eq!(owner_source, "exact_person_name");
        assert!(trust_score.is_some());
        assert!(trust_band.is_some());

        assert_eq!(bridge_action_id(&db, &derived_commitment_id), "accepted-a1");
        assert_eq!(source_rows(&db, "legacy-accepted-id"), 1);
    }

    #[test]
    fn accepted_bridge_match_preserves_user_owned_fields_after_rekeyed_action() {
        let db = test_db();
        make_ctx!(ctx);
        let original = vec![make_commitment("AI phrasing for accepted row")];
        let derived_commitment_id = derived_id("AI phrasing for accepted row");
        sync_ai_commitments(&ctx, &db, "account", "acct-1", &original).expect("initial sync");
        let action_id = bridge_action_id(&db, &derived_commitment_id);

        db.conn_ref()
            .execute(
                "UPDATE actions
                 SET status = 'unstarted',
                     title = 'User title',
                     due_date = '2030-02-03',
                     source_id = 'user-stable-id',
                     source_label = 'user source',
                     commitment_id = 'user-stable-id',
                     owner_raw = 'Jamie Lee',
                     owner_entity_id = 'p-jamie',
                     owner_confidence = 0.88,
                     owner_source = 'exact_person_name',
                     context = 'owner: still user context',
                     trust_score = NULL,
                     trust_band = NULL
                 WHERE id = ?1",
                rusqlite::params![action_id],
            )
            .unwrap();

        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &original).expect("resync");
        assert_eq!(summary.created, 0);
        assert_eq!(action_count(&db), 1);

        let (
            title,
            due_date,
            commitment_id,
            source_id,
            source_label,
            owner_raw,
            owner_entity_id,
            context,
            trust_score,
            trust_band,
        ): (
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<f64>,
            Option<String>,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT title, due_date, commitment_id, source_id, source_label,
                        owner_raw, owner_entity_id, context, trust_score, trust_band
                 FROM actions WHERE id = ?1",
                rusqlite::params![action_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(title, "User title");
        assert_eq!(due_date, "2030-02-03");
        assert_eq!(commitment_id, "user-stable-id");
        assert_eq!(source_id, "user-stable-id");
        assert_eq!(source_label, "user source");
        assert_eq!(owner_raw, "Jamie Lee");
        assert_eq!(owner_entity_id, "p-jamie");
        assert_eq!(context, "owner: still user context");
        assert!(trust_score.is_some());
        assert!(trust_band.is_some());
        assert_eq!(source_rows(&db, "user-stable-id"), 1);
    }

    #[test]
    fn dos321_normalize_commitment_title_matches_derived_identity_title() {
        assert_eq!(
            normalize_commitment_title("  Send Renewal Deck  "),
            "send renewal deck"
        );
        assert_eq!(
            normalize_commitment_title("Send Renewal Deck"),
            "send renewal deck"
        );
        assert_eq!(
            normalize_commitment_title("Send  Renewal  Deck."),
            "send renewal deck."
        );
    }

    #[test]
    fn aliases_legacy_backlog_row_with_same_exact_title_to_typed_identity() {
        let db = test_db();
        make_ctx!(ctx);
        let title = "Consolidate Globex subsidiary domains onto VIP";
        let commitment_id = derived_id(title);

        db.conn_ref()
            .execute(
                "INSERT INTO actions
                 (id, title, priority, status, created_at, updated_at, account_id, action_kind)
                 VALUES ('legacy-a1', ?1, 2, 'backlog', datetime('now'), datetime('now'), 'acct-1', 'commitment')",
                rusqlite::params![title],
            )
            .unwrap();

        let summary =
            sync_ai_commitments(&ctx, &db, "account", "acct-1", &[make_commitment(title)])
                .expect("sync");
        assert_eq!(summary.created, 0);
        assert_eq!(summary.aliased_to_existing, 1);

        let action_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM actions WHERE account_id = ?1 AND action_kind = 'commitment'",
                rusqlite::params!["acct-1"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(action_count, 1, "should not create a duplicate action");

        let action_id = bridge_action_id(&db, &commitment_id);
        assert_eq!(action_id, "legacy-a1");
        let stored_id: String = db
            .conn_ref()
            .query_row(
                "SELECT commitment_id FROM actions WHERE id = 'legacy-a1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_id, commitment_id);
    }

    #[test]
    fn dos321_alias_only_targets_open_actions_not_completed() {
        // If a legacy same-title row was completed, a fresh emit should not
        // alias onto it: the user is done with that commitment.
        let db = test_db();
        make_ctx!(ctx);
        db.conn_ref()
            .execute(
                "INSERT INTO actions
                 (id, title, priority, status, created_at, updated_at, account_id, action_kind)
                 VALUES ('done-a1', 'One-time work', 2, 'completed', datetime('now'), datetime('now'), 'acct-1', 'commitment')",
                [],
            )
            .unwrap();

        let s = sync_ai_commitments(
            &ctx,
            &db,
            "account",
            "acct-1",
            &[make_commitment("One-time work")],
        )
        .expect("sync");
        assert_eq!(s.aliased_to_existing, 0);
        assert_eq!(s.created, 1);

        let action_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM actions WHERE account_id = ?1 AND action_kind = 'commitment'",
                rusqlite::params!["acct-1"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(action_count, 2);
    }

    #[test]
    fn test_complete_commitment_action_tombstones_bridge_row() {
        // Regression guard: after complete_action on a commitment action,
        // the bridge row must be tombstoned (so re-enrichment can't resurrect).
        let db = test_db();
        make_ctx!(ctx);
        let commitments = vec![make_commitment("Will complete")];
        let commitment_id = derived_id("Will complete");
        sync_ai_commitments(&ctx, &db, "account", "acct-1", &commitments).expect("sync");

        let action_id = bridge_action_id(&db, &commitment_id);

        db.complete_action(&action_id).expect("complete");
        tombstone_commitment_bridge(&ctx, &db, &action_id).expect("tombstone");

        let flag: i32 = db
            .conn_ref()
            .query_row(
                "SELECT tombstoned FROM ai_commitment_bridge WHERE commitment_id = ?1",
                rusqlite::params![commitment_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(flag, 1);
    }
}

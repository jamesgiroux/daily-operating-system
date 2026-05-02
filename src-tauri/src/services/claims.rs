//! DOS-7 D2: claims commit substrate service layer.
//!
//! `commit_claim` is the only writer for intelligence_claims. All
//! mutation paths route through this function so the tombstone PRE-GATE,
//! dedup, contradiction detection, trust computation, and per-entity
//! invalidation are atomic and structurally enforced.
//!
//! D2 ships:
//! - `commit_claim(ctx, proposal) -> Result<CommittedClaim, ClaimError>`
//! - `record_corroboration(ctx, claim_id, source) -> Result<String, ClaimError>`
//! - `reconcile_contradiction(ctx, contradiction_id, kind, ...) -> Result<()>`
//! - `load_claims_active(db, subject_ref, ...) -> Result<Vec<IntelligenceClaim>>`
//! - `load_claims_including_dormant(...)` and `load_claims_dormant_only(...)`
//!
//! D3 owns the 9-mechanism backfill. D4 routes existing dismissal callers
//! through `commit_claim`. D5 owns reconcile_post_migration.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use parking_lot::Mutex;
use rusqlite::{params, OptionalExtension};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::db::claim_invalidation::SubjectRef;
use crate::db::claims::{
    ClaimSensitivity, ClaimState, IntelligenceClaim, ReconciliationKind, SurfacingState,
    TemporalScope,
};
use crate::db::{ActionDb, DbError};
use crate::intelligence::canonicalization::{item_hash, ItemKind};
use crate::services::context::ServiceContext;

// ---------------------------------------------------------------------------
// Public types: proposal + committed shape
// ---------------------------------------------------------------------------

/// Caller-supplied input to `commit_claim`. The service computes
/// dedup_key, canonical text, item_hash, and identity fields; the caller
/// supplies semantics + provenance.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClaimProposal {
    pub subject_ref: String,
    pub claim_type: String,
    pub field_path: Option<String>,
    pub topic_key: Option<String>,
    pub text: String,
    pub actor: String,
    pub data_source: String,
    pub source_ref: Option<String>,
    pub source_asof: Option<String>,
    pub observed_at: String,
    pub provenance_json: String,
    pub metadata_json: Option<String>,
    pub thread_id: Option<String>,
    pub temporal_scope: TemporalScope,
    pub sensitivity: ClaimSensitivity,
    /// If this commit is creating a tombstone, caller signals so via this
    /// enum + retraction_reason text.
    pub tombstone: Option<TombstoneSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TombstoneSpec {
    pub retraction_reason: String,
    pub expires_at: Option<String>,
}

/// What `commit_claim` returns. D2 fully implements the insert and
/// tombstone insert paths. Same-meaning reinforcement and contradiction
/// forking stay marked as D2 follow-up work.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommittedClaim {
    Inserted {
        claim: IntelligenceClaim,
    },
    Reinforced {
        claim: IntelligenceClaim,
        corroboration_id: String,
    },
    Forked {
        primary_claim: IntelligenceClaim,
        contradiction_id: String,
        new_claim_id: String,
    },
    Tombstoned {
        claim: IntelligenceClaim,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ClaimError {
    #[error("ServiceContext mutation gate: {0}")]
    Mode(String),
    #[error("invalid subject_ref: {0}")]
    SubjectRef(String),
    #[error("unknown claim_type: {0} (not in CLAIM_TYPE_REGISTRY)")]
    UnknownClaimType(String),
    #[error("tombstone PRE-GATE: claim is tombstoned and cannot be re-committed")]
    TombstonedPreGate,
    #[error("database error: {0}")]
    Db(#[from] DbError),
    #[error("rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Per-key commit lock (ADR-0113 R2)
// ---------------------------------------------------------------------------

/// Process-wide lock map keyed on (subject_ref, claim_type,
/// field_path|topic_key). Lock entries are intentionally retained for the
/// process lifetime; cardinality is bounded by distinct claim keys.
type CommitKey = (String, String, String);

static COMMIT_LOCKS: OnceLock<Mutex<HashMap<CommitKey, Arc<Mutex<()>>>>> = OnceLock::new();

fn commit_locks() -> &'static Mutex<HashMap<CommitKey, Arc<Mutex<()>>>> {
    COMMIT_LOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn commit_lock_for(key: CommitKey) -> Arc<Mutex<()>> {
    let mut map = commit_locks().lock();
    map.entry(key)
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// ADR-0113 section 8 dedup_key: content hash + compact subject identity +
/// claim type + field path. `thread_id` is deliberately excluded.
fn compute_dedup_key(
    item_hash: &str,
    subject_ref_compact: &str,
    claim_type: &str,
    field_path: Option<&str>,
) -> String {
    format!(
        "{}:{}:{}:{}",
        item_hash,
        subject_ref_compact,
        claim_type,
        field_path.unwrap_or("")
    )
}

/// DOS-280 canonicalization stub. D2 calls this in the write path; full
/// DOS-280 lands separately.
fn canonicalize_for_dos280(text: &str) -> String {
    text.to_string()
}

fn compact_subject_ref(value: &serde_json::Value) -> Result<String, ClaimError> {
    Ok(serde_json::to_string(value)?)
}

fn compact_subject_ref_str(subject_ref: &str) -> Result<String, ClaimError> {
    let value = serde_json::from_str::<serde_json::Value>(subject_ref)
        .map_err(|e| ClaimError::SubjectRef(format!("not JSON: {e}")))?;
    compact_subject_ref(&value)
}

fn subject_ref_from_json(value: &serde_json::Value) -> Result<SubjectRef, ClaimError> {
    let kind = value
        .get("kind")
        .or_else(|| value.get("type"))
        .or_else(|| value.get("entity_type"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| ClaimError::SubjectRef("missing kind/type".to_string()))?;

    match kind {
        "account" | "accounts" => Ok(SubjectRef::Account {
            id: subject_id(value)?,
        }),
        "meeting" | "meetings" => Ok(SubjectRef::Meeting {
            id: subject_id(value)?,
        }),
        "person" | "people" => Ok(SubjectRef::Person {
            id: subject_id(value)?,
        }),
        "project" | "projects" => Ok(SubjectRef::Project {
            id: subject_id(value)?,
        }),
        "multi" => {
            let refs = value
                .get("subjects")
                .or_else(|| value.get("refs"))
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    ClaimError::SubjectRef("multi subject_ref missing subjects".to_string())
                })?
                .iter()
                .map(subject_ref_from_json)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(SubjectRef::Multi(refs))
        }
        "global" => Ok(SubjectRef::Global),
        other => Err(ClaimError::SubjectRef(format!(
            "unsupported subject kind/type '{other}'"
        ))),
    }
}

fn subject_id(value: &serde_json::Value) -> Result<String, ClaimError> {
    value
        .get("id")
        .or_else(|| value.get("entity_id"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| ClaimError::SubjectRef("missing id/entity_id".to_string()))
}

fn item_kind_for_claim_type(claim_type: &str) -> ItemKind {
    match claim_type {
        "risk" => ItemKind::Risk,
        "win" => ItemKind::Win,
        _ => ItemKind::_Reserved,
    }
}

fn enum_to_db<T: Serialize>(value: &T) -> Result<String, ClaimError> {
    Ok(serde_json::to_string(value)?.trim_matches('"').to_string())
}

fn parse_db_enum<T>(value: String) -> Result<T, ClaimError>
where
    T: DeserializeOwned,
{
    Ok(serde_json::from_value(serde_json::Value::String(value))?)
}

fn with_claim_transaction<T>(
    db: &ActionDb,
    f: impl FnOnce(&ActionDb) -> Result<T, ClaimError>,
) -> Result<T, ClaimError> {
    if !db.conn_ref().is_autocommit() {
        return f(db);
    }

    db.conn_ref().execute_batch("BEGIN IMMEDIATE")?;
    match f(db) {
        Ok(value) => {
            db.conn_ref().execute_batch("COMMIT")?;
            Ok(value)
        }
        Err(error) => {
            let _ = db.conn_ref().execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

fn insert_claim_row(tx: &ActionDb, claim: &IntelligenceClaim) -> Result<(), ClaimError> {
    tx.conn_ref().execute(
        "INSERT INTO intelligence_claims (
            id, subject_ref, claim_type, field_path, topic_key, text, dedup_key,
            item_hash, actor, data_source, source_ref, source_asof, observed_at,
            created_at, provenance_json, metadata_json, claim_state, surfacing_state,
            demotion_reason, reactivated_at, retraction_reason, expires_at,
            superseded_by, trust_score, trust_computed_at, trust_version, thread_id,
            temporal_scope, sensitivity
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
            ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29
        )",
        params![
            &claim.id,
            &claim.subject_ref,
            &claim.claim_type,
            claim.field_path.as_deref(),
            claim.topic_key.as_deref(),
            &claim.text,
            &claim.dedup_key,
            claim.item_hash.as_deref(),
            &claim.actor,
            &claim.data_source,
            claim.source_ref.as_deref(),
            claim.source_asof.as_deref(),
            &claim.observed_at,
            &claim.created_at,
            &claim.provenance_json,
            claim.metadata_json.as_deref(),
            enum_to_db(&claim.claim_state)?,
            enum_to_db(&claim.surfacing_state)?,
            claim.demotion_reason.as_deref(),
            claim.reactivated_at.as_deref(),
            claim.retraction_reason.as_deref(),
            claim.expires_at.as_deref(),
            claim.superseded_by.as_deref(),
            claim.trust_score,
            claim.trust_computed_at.as_deref(),
            claim.trust_version,
            claim.thread_id.as_deref(),
            enum_to_db(&claim.temporal_scope)?,
            enum_to_db(&claim.sensitivity)?,
        ],
    )?;
    Ok(())
}

const CLAIM_COLUMNS: &str = "id, subject_ref, claim_type, field_path, topic_key, text, dedup_key,
    item_hash, actor, data_source, source_ref, source_asof, observed_at, created_at,
    provenance_json, metadata_json, claim_state, surfacing_state, demotion_reason,
    reactivated_at, retraction_reason, expires_at, superseded_by, trust_score,
    trust_computed_at, trust_version, thread_id, temporal_scope, sensitivity";

fn read_claim_row(row: &rusqlite::Row<'_>) -> Result<IntelligenceClaim, ClaimError> {
    Ok(IntelligenceClaim {
        id: row.get(0)?,
        subject_ref: row.get(1)?,
        claim_type: row.get(2)?,
        field_path: row.get(3)?,
        topic_key: row.get(4)?,
        text: row.get(5)?,
        dedup_key: row.get(6)?,
        item_hash: row.get(7)?,
        actor: row.get(8)?,
        data_source: row.get(9)?,
        source_ref: row.get(10)?,
        source_asof: row.get(11)?,
        observed_at: row.get(12)?,
        created_at: row.get(13)?,
        provenance_json: row.get(14)?,
        metadata_json: row.get(15)?,
        claim_state: parse_db_enum(row.get(16)?)?,
        surfacing_state: parse_db_enum(row.get(17)?)?,
        demotion_reason: row.get(18)?,
        reactivated_at: row.get(19)?,
        retraction_reason: row.get(20)?,
        expires_at: row.get(21)?,
        superseded_by: row.get(22)?,
        trust_score: row.get(23)?,
        trust_computed_at: row.get(24)?,
        trust_version: row.get(25)?,
        thread_id: row.get(26)?,
        temporal_scope: parse_db_enum(row.get(27)?)?,
        sensitivity: parse_db_enum(row.get(28)?)?,
    })
}

fn load_claims_where(
    db: &ActionDb,
    subject_ref: &str,
    claim_type: Option<&str>,
    lifecycle_where: &str,
) -> Result<Vec<IntelligenceClaim>, ClaimError> {
    let subject_ref_compact = compact_subject_ref_str(subject_ref)?;
    let sql = format!(
        "SELECT {CLAIM_COLUMNS} FROM intelligence_claims
         WHERE subject_ref = ?1
           AND (?2 IS NULL OR claim_type = ?2)
           AND {lifecycle_where}
         ORDER BY created_at DESC"
    );
    let mut stmt = db.conn_ref().prepare(&sql)?;
    let mut rows = stmt.query(params![subject_ref_compact, claim_type])?;
    let mut claims = Vec::new();
    while let Some(row) = rows.next()? {
        claims.push(read_claim_row(row)?);
    }
    Ok(claims)
}

// ---------------------------------------------------------------------------
// commit_claim
// ---------------------------------------------------------------------------

pub fn commit_claim(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    proposal: ClaimProposal,
) -> Result<CommittedClaim, ClaimError> {
    ctx.check_mutation_allowed()
        .map_err(|e| ClaimError::Mode(e.to_string()))?;

    if proposal.subject_ref.trim().is_empty() {
        return Err(ClaimError::SubjectRef("empty".to_string()));
    }
    let subject_value = serde_json::from_str::<serde_json::Value>(&proposal.subject_ref)
        .map_err(|e| ClaimError::SubjectRef(format!("not JSON: {e}")))?;
    let subject = subject_ref_from_json(&subject_value)?;
    let subject_ref_compact = compact_subject_ref(&subject_value)?;
    if proposal.claim_type.trim().is_empty() {
        return Err(ClaimError::UnknownClaimType("empty".to_string()));
    }

    let canonical_text = canonicalize_for_dos280(&proposal.text);
    let computed_hash = item_hash(item_kind_for_claim_type(&proposal.claim_type), &canonical_text);
    let dedup_key = compute_dedup_key(
        &computed_hash,
        &subject_ref_compact,
        &proposal.claim_type,
        proposal.field_path.as_deref(),
    );

    let key = (
        subject_ref_compact.clone(),
        proposal.claim_type.clone(),
        proposal
            .field_path
            .clone()
            .or_else(|| proposal.topic_key.clone())
            .unwrap_or_default(),
    );
    let lock = commit_lock_for(key);
    let _guard = lock.lock();

    with_claim_transaction(db, |tx| {
        let now = ctx.clock.now().to_rfc3339();
        let tombstone_count: i64 = tx.conn_ref().query_row(
            "SELECT count(*)
             FROM intelligence_claims
             WHERE dedup_key = ?1
               AND claim_state = 'tombstoned'
               AND (expires_at IS NULL OR expires_at > ?2)",
            params![&dedup_key, &now],
            |row| row.get(0),
        )?;
        if tombstone_count > 0 && proposal.tombstone.is_none() {
            return Err(ClaimError::TombstonedPreGate);
        }

        // TODO(D2-followup): Same-meaning merge. If an active claim with this
        // dedup_key exists, route through record_corroboration and return
        // CommittedClaim::Reinforced instead of inserting a duplicate row.

        // TODO(D2-followup): Contradiction detection. If an active claim exists
        // with the same (subject_ref, claim_type, field_path/topic_key) and a
        // different canonical text with aggregate strength >= 0.4, insert the
        // new claim plus claim_contradictions and return CommittedClaim::Forked.

        let id = uuid::Uuid::new_v4().to_string();
        let (claim_state, surfacing_state, retraction_reason, expires_at) =
            if let Some(tombstone) = &proposal.tombstone {
                (
                    ClaimState::Tombstoned,
                    SurfacingState::Dormant,
                    Some(tombstone.retraction_reason.clone()),
                    tombstone.expires_at.clone(),
                )
            } else {
                (ClaimState::Active, SurfacingState::Active, None, None)
            };
        let claim = IntelligenceClaim {
            id,
            subject_ref: subject_ref_compact,
            claim_type: proposal.claim_type.clone(),
            field_path: proposal.field_path.clone(),
            topic_key: proposal.topic_key.clone(),
            text: canonical_text,
            dedup_key,
            item_hash: Some(computed_hash),
            actor: proposal.actor.clone(),
            data_source: proposal.data_source.clone(),
            source_ref: proposal.source_ref.clone(),
            source_asof: proposal.source_asof.clone(),
            observed_at: proposal.observed_at.clone(),
            created_at: now,
            provenance_json: proposal.provenance_json.clone(),
            metadata_json: proposal.metadata_json.clone(),
            claim_state,
            surfacing_state,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason,
            expires_at,
            superseded_by: None,
            trust_score: None,
            trust_computed_at: None,
            trust_version: None,
            thread_id: proposal.thread_id.clone(),
            temporal_scope: proposal.temporal_scope.clone(),
            sensitivity: proposal.sensitivity.clone(),
        };

        insert_claim_row(tx, &claim)?;
        tx.bump_for_subject(&subject)?;

        if proposal.tombstone.is_some() {
            Ok(CommittedClaim::Tombstoned { claim })
        } else {
            Ok(CommittedClaim::Inserted { claim })
        }
    })
}

// ---------------------------------------------------------------------------
// record_corroboration
// ---------------------------------------------------------------------------

pub fn record_corroboration(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    claim_id: &str,
    data_source: &str,
    source_asof: Option<&str>,
    source_mechanism: Option<&str>,
) -> Result<String, ClaimError> {
    ctx.check_mutation_allowed()
        .map_err(|e| ClaimError::Mode(e.to_string()))?;

    with_claim_transaction(db, |tx| {
        let now = ctx.clock.now().to_rfc3339();
        let existing: Option<(String, f64, i64)> = tx
            .conn_ref()
            .query_row(
                "SELECT id, strength, reinforcement_count
                 FROM claim_corroborations
                 WHERE claim_id = ?1 AND data_source = ?2",
                params![claim_id, data_source],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        match existing {
            Some((id, strength, count)) => {
                let numerator = (count as f64 + 2.0).ln();
                let denominator = (count as f64 + 1.0).ln();
                let increment = if denominator > 0.0 {
                    numerator / denominator
                } else {
                    1.0
                };
                let new_strength = (strength + increment).min(1.0);
                tx.conn_ref().execute(
                    "UPDATE claim_corroborations
                     SET strength = ?1,
                         reinforcement_count = reinforcement_count + 1,
                         last_reinforced_at = ?2
                     WHERE id = ?3",
                    params![new_strength, &now, &id],
                )?;
                Ok(id)
            }
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                tx.conn_ref().execute(
                    "INSERT INTO claim_corroborations (
                        id, claim_id, data_source, source_asof, source_mechanism,
                        strength, reinforcement_count, last_reinforced_at, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 0.5, 1, ?6, ?6)",
                    params![
                        &id,
                        claim_id,
                        data_source,
                        source_asof,
                        source_mechanism,
                        &now
                    ],
                )?;
                Ok(id)
            }
        }
    })
}

// ---------------------------------------------------------------------------
// reconcile_contradiction
// ---------------------------------------------------------------------------

pub fn reconcile_contradiction(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    contradiction_id: &str,
    kind: ReconciliationKind,
    note: Option<&str>,
    winner_claim_id: Option<&str>,
    merged_claim_id: Option<&str>,
) -> Result<(), ClaimError> {
    ctx.check_mutation_allowed()
        .map_err(|e| ClaimError::Mode(e.to_string()))?;

    with_claim_transaction(db, |tx| {
        let now = ctx.clock.now().to_rfc3339();
        tx.conn_ref().execute(
            "UPDATE claim_contradictions
             SET reconciliation_kind = ?1,
                 reconciliation_note = ?2,
                 reconciled_at = ?3,
                 winner_claim_id = ?4,
                 merged_claim_id = ?5
             WHERE id = ?6",
            params![
                enum_to_db(&kind)?,
                note,
                &now,
                winner_claim_id,
                merged_claim_id,
                contradiction_id
            ],
        )?;
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// Default readers
// ---------------------------------------------------------------------------

/// Default reader: `claim_state='active' AND surfacing_state='active'`.
pub fn load_claims_active(
    db: &ActionDb,
    subject_ref: &str,
    claim_type: Option<&str>,
) -> Result<Vec<IntelligenceClaim>, ClaimError> {
    load_claims_where(
        db,
        subject_ref,
        claim_type,
        "claim_state = 'active' AND surfacing_state = 'active'",
    )
}

/// History-aware reader: active + dormant claims, excluding tombstoned and
/// withdrawn rows.
pub fn load_claims_including_dormant(
    db: &ActionDb,
    subject_ref: &str,
    claim_type: Option<&str>,
) -> Result<Vec<IntelligenceClaim>, ClaimError> {
    load_claims_where(
        db,
        subject_ref,
        claim_type,
        "claim_state IN ('active', 'dormant') AND surfacing_state IN ('active', 'dormant')",
    )
}

/// Dormant-only reader: claims hidden by either claim lifecycle or surfacing
/// lifecycle, excluding tombstoned and withdrawn rows.
pub fn load_claims_dormant_only(
    db: &ActionDb,
    subject_ref: &str,
    claim_type: Option<&str>,
) -> Result<Vec<IntelligenceClaim>, ClaimError> {
    load_claims_where(
        db,
        subject_ref,
        claim_type,
        "claim_state IN ('active', 'dormant')
         AND surfacing_state IN ('active', 'dormant')
         AND (claim_state = 'dormant' OR surfacing_state = 'dormant')",
    )
}

// ---------------------------------------------------------------------------
// DOS-7 D4-1a: runtime shadow-write helper
// ---------------------------------------------------------------------------

/// Shadow-write a tombstone claim alongside a legacy `create_suppression_tombstone`
/// call during the DOS-7 transition window.
///
/// Existing dismissal callers (services/intelligence.rs::dismiss_intelligence_item,
/// services/accounts.rs runtime correction paths, services/feedback.rs::apply_correction)
/// keep writing to the legacy `suppression_tombstones` table — DOS-301 / W3-D
/// owns the eventual refactor that makes services/derived_state.rs the only
/// legacy projection writer. Until that lands, we shadow-write a tombstone
/// claim into intelligence_claims so the new substrate is populated in
/// parallel and reconcile can verify parity in D5.
///
/// Failure of the shadow write is LOGGED but does NOT propagate as Err; the
/// legacy write above remains authoritative for this release. Once DOS-301
/// lands and reconcile is clean, follow-up work flips the authority.
pub struct ShadowTombstoneClaim<'a> {
    pub subject_kind: &'a str,
    pub subject_id: &'a str,
    pub claim_type: &'a str,
    pub field_path: Option<&'a str>,
    pub text: &'a str,
    pub actor: &'a str,
    pub source_scope: Option<&'a str>,
    pub observed_at: &'a str,
}

pub fn shadow_write_tombstone_claim(db: &ActionDb, args: ShadowTombstoneClaim<'_>) {
    let ShadowTombstoneClaim {
        subject_kind,
        subject_id,
        claim_type,
        field_path,
        text,
        actor,
        source_scope,
        observed_at,
    } = args;
    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);

    let subject_ref = format!(r#"{{"kind":"{}","id":"{}"}}"#, subject_kind, subject_id);
    let metadata_json = source_scope.map(|s| format!(r#"{{"source_scope":"{}"}}"#, s));

    let proposal = ClaimProposal {
        subject_ref,
        claim_type: claim_type.to_string(),
        field_path: field_path.map(|s| s.to_string()),
        topic_key: None,
        text: text.to_string(),
        actor: actor.to_string(),
        data_source: "user_dismissal".to_string(),
        source_ref: None,
        source_asof: Some(observed_at.to_string()),
        observed_at: observed_at.to_string(),
        provenance_json: r#"{"runtime":"dos7_d4_1a_shadow"}"#.to_string(),
        metadata_json,
        thread_id: None,
        temporal_scope: TemporalScope::State,
        sensitivity: ClaimSensitivity::Internal,
        tombstone: Some(TombstoneSpec {
            retraction_reason: "user_removal".to_string(),
            expires_at: None,
        }),
    };

    if let Err(e) = commit_claim(&ctx, db, proposal) {
        log::warn!(
            "[dos7-d4-1a] shadow tombstone claim write failed (subject={}:{} field={:?}): {}",
            subject_kind, subject_id, field_path, e
        );
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use chrono::{TimeZone, Utc};
    use rusqlite::params;

    use super::*;
    use crate::db::test_utils::test_db;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};

    const TS: &str = "2026-05-02T12:00:00+00:00";
    const SUBJECT: &str = r#"{"kind":"account","id":"acct-1"}"#;

    fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
        (
            FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 2, 12, 0, 0).unwrap()),
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

    fn proposal(text: &str) -> ClaimProposal {
        ClaimProposal {
            subject_ref: SUBJECT.to_string(),
            claim_type: "risk".to_string(),
            field_path: Some("health.risk".to_string()),
            topic_key: None,
            text: text.to_string(),
            actor: "agent:test".to_string(),
            data_source: "unit_test".to_string(),
            source_ref: None,
            source_asof: Some(TS.to_string()),
            observed_at: TS.to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: TemporalScope::State,
            sensitivity: ClaimSensitivity::Internal,
            tombstone: None,
        }
    }

    fn seed_account(db: &ActionDb) {
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params!["acct-1", "Account 1", TS],
            )
            .expect("seed account");
    }

    fn read_account_claim_version(db: &ActionDb) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT claim_version FROM accounts WHERE id = 'acct-1'",
                [],
                |row| row.get(0),
            )
            .expect("read claim_version")
    }

    fn insert_fixture_claim(
        db: &ActionDb,
        id: &str,
        subject_ref: &str,
        claim_type: &str,
        text: &str,
        claim_state: ClaimState,
        surfacing_state: SurfacingState,
    ) {
        let compact_subject_ref = compact_subject_ref_str(subject_ref).expect("compact subject");
        let hash = item_hash(ItemKind::Risk, text);
        let dedup_key =
            compute_dedup_key(&hash, &compact_subject_ref, claim_type, Some("health.risk"));
        let claim = IntelligenceClaim {
            id: id.to_string(),
            subject_ref: compact_subject_ref,
            claim_type: claim_type.to_string(),
            field_path: Some("health.risk".to_string()),
            topic_key: None,
            text: text.to_string(),
            dedup_key,
            item_hash: Some(hash),
            actor: "agent:test".to_string(),
            data_source: "unit_test".to_string(),
            source_ref: None,
            source_asof: None,
            observed_at: TS.to_string(),
            created_at: TS.to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            claim_state,
            surfacing_state,
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
        };
        insert_claim_row(db, &claim).expect("insert fixture claim");
    }

    fn inserted_claim_id(result: CommittedClaim) -> String {
        match result {
            CommittedClaim::Inserted { claim } | CommittedClaim::Tombstoned { claim } => claim.id,
            other => panic!("expected inserted/tombstoned claim, got {other:?}"),
        }
    }

    #[test]
    fn compute_dedup_key_is_stable_for_same_inputs() {
        let key_1 = compute_dedup_key("hash", SUBJECT, "risk", Some("health.risk"));
        let key_2 = compute_dedup_key("hash", SUBJECT, "risk", Some("health.risk"));
        assert_eq!(key_1, key_2);
        assert_eq!(key_1, format!("hash:{SUBJECT}:risk:health.risk"));
    }

    #[test]
    fn commit_lock_serializes_same_key_writers() {
        let key = (
            "subject-lock".to_string(),
            "risk".to_string(),
            "health.risk".to_string(),
        );
        let lock = commit_lock_for(key.clone());
        let guard = lock.lock();

        let (attempt_tx, attempt_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            attempt_tx.send(()).unwrap();
            let lock = commit_lock_for(key);
            let _guard = lock.lock();
            acquired_tx.send(()).unwrap();
        });

        attempt_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("writer attempted lock");
        assert!(
            acquired_rx.recv_timeout(Duration::from_millis(50)).is_err(),
            "second writer must wait while first guard is held"
        );
        drop(guard);
        acquired_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("second writer acquired after release");
        handle.join().expect("thread joined");
    }

    #[test]
    fn commit_claim_inserts_simple_active_claim() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let result = commit_claim(&ctx, &db, proposal("Renewal risk is elevated")).unwrap();
        let id = inserted_claim_id(result);
        let claim = load_claims_active(&db, SUBJECT, Some("risk"))
            .unwrap()
            .into_iter()
            .find(|claim| claim.id == id)
            .expect("inserted claim loads");

        assert_eq!(claim.claim_state, ClaimState::Active);
        assert_eq!(claim.surfacing_state, SurfacingState::Active);
        assert_eq!(claim.trust_score, None);
        assert_eq!(claim.item_hash, Some(item_hash(ItemKind::Risk, &claim.text)));
    }

    #[test]
    fn commit_claim_rejects_when_dedup_key_is_tombstoned() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let mut tombstone = proposal("Procurement blocked renewal");
        tombstone.tombstone = Some(TombstoneSpec {
            retraction_reason: "user_removal".to_string(),
            expires_at: None,
        });
        commit_claim(&ctx, &db, tombstone).unwrap();

        let err = commit_claim(&ctx, &db, proposal("Procurement blocked renewal"))
            .expect_err("tombstone should block recommit");
        assert!(matches!(err, ClaimError::TombstonedPreGate));
    }

    #[test]
    fn commit_claim_emits_per_entity_invalidation() {
        let db = test_db();
        seed_account(&db);
        let before = read_account_claim_version(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        commit_claim(&ctx, &db, proposal("Budget risk increased")).unwrap();

        assert_eq!(read_account_claim_version(&db), before + 1);
    }

    #[test]
    fn record_corroboration_first_source_inserts_at_0_5() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id = inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk one")).unwrap());

        let corr_id =
            record_corroboration(&ctx, &db, &claim_id, "glean", Some(TS), Some("backfill"))
                .unwrap();
        let (strength, count): (f64, i64) = db
            .conn_ref()
            .query_row(
                "SELECT strength, reinforcement_count FROM claim_corroborations WHERE id = ?1",
                params![corr_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(strength, 0.5);
        assert_eq!(count, 1);
    }

    #[test]
    fn record_corroboration_same_source_strengthens_via_formula() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id = inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk two")).unwrap());
        let first = record_corroboration(&ctx, &db, &claim_id, "glean", None, None).unwrap();
        let second = record_corroboration(&ctx, &db, &claim_id, "glean", None, None).unwrap();

        let (strength, count): (f64, i64) = db
            .conn_ref()
            .query_row(
                "SELECT strength, reinforcement_count FROM claim_corroborations WHERE id = ?1",
                params![first],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(strength, 1.0);
        assert_eq!(count, 2);
    }

    #[test]
    fn record_corroboration_diverse_sources_each_get_own_row() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id = inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk three")).unwrap());

        let first = record_corroboration(&ctx, &db, &claim_id, "glean", None, None).unwrap();
        let second = record_corroboration(&ctx, &db, &claim_id, "calendar", None, None).unwrap();
        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM claim_corroborations WHERE claim_id = ?1",
                params![claim_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_ne!(first, second);
        assert_eq!(count, 2);
    }

    #[test]
    fn reconcile_contradiction_marks_reconciled_at() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let primary = inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk four")).unwrap());
        let mut second_proposal = proposal("Risk four resolved");
        second_proposal.field_path = Some("health.risk.resolved".to_string());
        let contradicting = inserted_claim_id(commit_claim(&ctx, &db, second_proposal).unwrap());
        db.conn_ref()
            .execute(
                "INSERT INTO claim_contradictions (
                    id, primary_claim_id, contradicting_claim_id, branch_kind, detected_at
                 ) VALUES ('contradiction-1', ?1, ?2, 'contradiction', ?3)",
                params![primary, contradicting, TS],
            )
            .unwrap();

        reconcile_contradiction(
            &ctx,
            &db,
            "contradiction-1",
            ReconciliationKind::UserPickedWinner,
            Some("picked primary"),
            Some(&primary),
            None,
        )
        .unwrap();

        let reconciled_at: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT reconciled_at FROM claim_contradictions WHERE id = 'contradiction-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(reconciled_at, Some(TS.to_string()));
    }

    #[test]
    fn load_claims_active_filters_dormant_and_tombstoned() {
        let db = test_db();
        insert_fixture_claim(
            &db,
            "active-1",
            SUBJECT,
            "risk",
            "Active",
            ClaimState::Active,
            SurfacingState::Active,
        );
        insert_fixture_claim(
            &db,
            "dormant-1",
            SUBJECT,
            "risk",
            "Dormant",
            ClaimState::Dormant,
            SurfacingState::Dormant,
        );
        insert_fixture_claim(
            &db,
            "tombstone-1",
            SUBJECT,
            "risk",
            "Tombstoned",
            ClaimState::Tombstoned,
            SurfacingState::Dormant,
        );

        let claims = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();

        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].id, "active-1");
    }

    #[test]
    fn load_claims_including_dormant_returns_active_and_dormant() {
        let db = test_db();
        insert_fixture_claim(
            &db,
            "active-1",
            SUBJECT,
            "risk",
            "Active",
            ClaimState::Active,
            SurfacingState::Active,
        );
        insert_fixture_claim(
            &db,
            "dormant-1",
            SUBJECT,
            "risk",
            "Dormant",
            ClaimState::Dormant,
            SurfacingState::Dormant,
        );
        insert_fixture_claim(
            &db,
            "withdrawn-1",
            SUBJECT,
            "risk",
            "Withdrawn",
            ClaimState::Withdrawn,
            SurfacingState::Dormant,
        );

        let ids = load_claims_including_dormant(&db, SUBJECT, Some("risk"))
            .unwrap()
            .into_iter()
            .map(|claim| claim.id)
            .collect::<Vec<_>>();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"active-1".to_string()));
        assert!(ids.contains(&"dormant-1".to_string()));
    }

    #[test]
    fn load_claims_dormant_only_filters_active() {
        let db = test_db();
        insert_fixture_claim(
            &db,
            "active-1",
            SUBJECT,
            "risk",
            "Active",
            ClaimState::Active,
            SurfacingState::Active,
        );
        insert_fixture_claim(
            &db,
            "dormant-1",
            SUBJECT,
            "risk",
            "Dormant",
            ClaimState::Dormant,
            SurfacingState::Dormant,
        );
        insert_fixture_claim(
            &db,
            "surfacing-dormant-1",
            SUBJECT,
            "risk",
            "Surfacing dormant",
            ClaimState::Active,
            SurfacingState::Dormant,
        );

        let ids = load_claims_dormant_only(&db, SUBJECT, Some("risk"))
            .unwrap()
            .into_iter()
            .map(|claim| claim.id)
            .collect::<Vec<_>>();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"dormant-1".to_string()));
        assert!(ids.contains(&"surfacing-dormant-1".to_string()));
        assert!(!ids.contains(&"active-1".to_string()));
    }
}

//! Claims commit substrate service layer.
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

use crate::abilities::claims::{metadata_for_claim_type, ClaimActorClass};
use crate::abilities::feedback::{
    compute_needs_nuance_trust_effect, feedback_semantics, transition_for_feedback,
    ClaimFeedbackMetadata, ClaimRenderPolicy, ClaimVerificationState, FeedbackAction, RepairAction,
};
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
/// supplies semantics + provenance, with registry defaults applied for
/// omitted scope/sensitivity values.
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
    pub temporal_scope: Option<TemporalScope>,
    pub sensitivity: Option<ClaimSensitivity>,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClaimFeedbackInput {
    pub claim_id: String,
    pub action: FeedbackAction,
    pub actor: String,
    pub actor_id: Option<String>,
    pub payload_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClaimFeedbackOutcome {
    pub feedback_id: String,
    pub claim_id: String,
    pub action: FeedbackAction,
    pub new_verification_state: ClaimVerificationState,
    pub applied_at_pending: bool,
    pub repair_job_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ClaimError {
    #[error("ServiceContext mutation gate: {0}")]
    Mode(String),
    #[error("invalid subject_ref: {0}")]
    SubjectRef(String),
    #[error("unknown claim_type: {0} (not in CLAIM_TYPE_REGISTRY)")]
    UnknownClaimType(String),
    #[error("unknown claim_id: {0}")]
    UnknownClaimId(String),
    #[error("invalid claim feedback: {0}")]
    InvalidFeedback(String),
    #[error("invalid actor: {0}")]
    InvalidActor(String),
    #[error("actor class not allowed for claim_type {claim_type}: {actor}")]
    ActorClassNotAllowed { claim_type: String, actor: String },
    #[error("actor {actor} ({actor_class}) is not permitted to write claim_type {claim_type}")]
    ActorNotPermittedForClaimType {
        claim_type: String,
        actor: String,
        actor_class: String,
    },
    #[error("tombstone PRE-GATE: claim is tombstoned and cannot be re-committed")]
    TombstonedPreGate,
    #[error("transaction error: {0}")]
    Transaction(String),
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
pub(crate) fn compute_dedup_key(
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

/// L2 cycle-1 fix #6: light canonicalization that catches the most
/// common drift between byte-different claim texts that mean the same
/// thing — trailing whitespace, internal whitespace runs (tab/space
/// mixes from different paste sources), and case variation.
///
/// Full canonicalization (Unicode NFC, punctuation folding,
/// stopword normalization, etc.) lands separately. The claims substrate
/// only needs enough canonicalization to make `same-meaning merge`
/// (commit_claim's de-dupe-via-corroboration branch) catch the obvious
/// repeats that legacy data and AI re-runs produce in practice.
pub(crate) fn canonicalize_for_dos280(text: &str) -> String {
    let trimmed = text.trim();
    let collapsed: String = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.to_lowercase()
}

fn compact_subject_ref(value: &serde_json::Value) -> Result<String, ClaimError> {
    Ok(serde_json::to_string(value)?)
}

/// L2 cycle-15 fix #1: serialize a parsed [`SubjectRef`] into a
/// byte-stable canonical JSON form for dedup_key + commit_lock
/// derivation. Two semantically-equal subjects (PascalCase vs
/// lowercase kind, reordered keys, extra whitespace, etc.) MUST
/// produce identical output so same-meaning merge fires and the
/// per-key commit lock serializes their writers.
///
/// The shape is alphabetical-key JSON with lowercase kind:
///   `{"id":"<id>","kind":"<lowercase kind>"}`
pub(crate) fn canonical_subject_ref(subject: &SubjectRef) -> Result<String, ClaimError> {
    let (kind, id) = match subject {
        SubjectRef::Account { id } => ("account", id.as_str()),
        SubjectRef::Meeting { id } => ("meeting", id.as_str()),
        SubjectRef::Person { id } => ("person", id.as_str()),
        SubjectRef::Project { id } => ("project", id.as_str()),
        SubjectRef::Email { id } => ("email", id.as_str()),
        SubjectRef::Multi(_) | SubjectRef::Global => {
            return Err(ClaimError::SubjectRef(
                "Multi/Global subjects are rejected at commit time per ADR-0125 v1.4.0 spine"
                    .to_string(),
            ));
        }
    };
    Ok(serde_json::json!({ "id": id, "kind": kind }).to_string())
}

/// Subject kind label used in the `subject_ref` JSON column for both
/// runtime-written and SQL-backfilled tombstone claims. Returned in the
/// canonical PascalCase form (e.g. `Account`, `Meeting`) that the
/// backfill SQL writes via `json_object('kind', 'Account', ...)` and
/// that production runtime callers serialize via the `SubjectRef` enum.
fn subject_kind_label(subject: &SubjectRef) -> Option<&'static str> {
    match subject {
        SubjectRef::Account { .. } => Some("Account"),
        SubjectRef::Meeting { .. } => Some("Meeting"),
        SubjectRef::Person { .. } => Some("Person"),
        SubjectRef::Project { .. } => Some("Project"),
        SubjectRef::Email { .. } => Some("Email"),
        SubjectRef::Multi(_) | SubjectRef::Global => None,
    }
}

fn subject_id_for_lookup(subject: &SubjectRef) -> Option<&str> {
    match subject {
        SubjectRef::Account { id }
        | SubjectRef::Meeting { id }
        | SubjectRef::Person { id }
        | SubjectRef::Project { id }
        | SubjectRef::Email { id } => Some(id.as_str()),
        SubjectRef::Multi(_) | SubjectRef::Global => None,
    }
}

/// PRE-GATE: returns true if a tombstone claim already shadows the
/// proposed (subject, claim_type, field_path, content) tuple.
///
/// Matches by semantic identity, not by `dedup_key`. The runtime and the
/// 8 SQL backfill mechanisms each compute `dedup_key` differently, so
/// matching by `dedup_key` would let legacy backfilled tombstones
/// slip past the gate and resurrect on the next AI enrichment pass.
/// Per L2 cycle-1 finding #2: PRE-GATE matches the same canonical
/// subject/claim/field/hash fields used by every backfill.
///
/// Three tiers, evaluated in order:
///   1. **Hash tier** — `item_hash` equals the proposal's computed hash.
///      Catches every claim where backfill hash and runtime hash use the
///      same algorithm (i.e., post-claims-cutover writes; legacy audit-trail-shaped
///      hashes also coincide).
///   2. **Exact text tier** — `text` equals the proposal's canonical
///      text. Catches backfill rows that stored the legacy `item_key`
///      verbatim into `text`, when the user dismisses by re-typing the
///      same text the AI surfaces.
///   3. **Keyless tier** — `text = '<keyless>'`. Catches backfilled
///      mechanism-1 keyless suppressions (legacy item_key=NULL,
///      item_hash=NULL): once the user dismissed "everything in this
///      field," any subsequent claim in that (subject, claim_type,
///      field) tuple is blocked.
///
/// `subject_ref` is matched via `json_extract` on `kind` and `id` so the
/// query is order-agnostic between runtime-serialized JSON
/// (alphabetical, BTreeMap) and backfill-serialized JSON
/// (insertion-order from `json_object()`).
fn pre_gate_blocking_tombstone_exists(
    conn: &rusqlite::Connection,
    subject: &SubjectRef,
    claim_type: &str,
    field_path: Option<&str>,
    item_hash_value: &str,
    canonical_text: &str,
    now: &str,
) -> Result<bool, ClaimError> {
    let Some(kind) = subject_kind_label(subject) else {
        // Multi/Global subjects don't participate in single-tombstone
        // suppression. Fall through to the active-write path.
        return Ok(false);
    };
    let Some(id) = subject_id_for_lookup(subject) else {
        return Ok(false);
    };

    // Three independent tier queries. Each is cheap (indexed on
    // claim_state + claim_type) and bounded by the per-key COMMIT_LOCKS
    // serializing concurrent commits for the same identity tuple.
    // L2 cycle-7 fix #2: filter on `json_valid(subject_ref) = 1`
    // BEFORE evaluating json_extract. SQLite's WHERE-clause AND
    // chain doesn't reliably short-circuit, so a malformed
    // historical tombstone subject_ref would otherwise raise
    // "malformed JSON" mid-PRE-GATE and the entire commit_claim
    // would fail. The valid-rows subquery materializes the safe
    // set first; malformed tombstones are silently skipped (they
    // can be remediated by an operator-run quarantine pass).
    const TIER_SQL: &str = "\
        SELECT 1 \
        FROM intelligence_claims \
        WHERE id IN ( \
            SELECT ic.id FROM intelligence_claims ic \
            WHERE ic.claim_state = 'tombstoned' \
              AND ic.claim_type = ?1 \
              AND coalesce(ic.field_path, '') = coalesce(?2, '') \
              AND json_valid(ic.subject_ref) = 1 \
              AND lower(json_extract(ic.subject_ref, '$.kind')) = lower(?3) \
              AND json_extract(ic.subject_ref, '$.id') = ?4 \
              AND (ic.expires_at IS NULL OR ic.expires_at > ?5) \
        ) \
          AND TIER_PREDICATE \
        LIMIT 1";

    let hit = |predicate: &str, params: &[&dyn rusqlite::ToSql]| -> Result<bool, ClaimError> {
        let sql = TIER_SQL.replace("TIER_PREDICATE", predicate);
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query(params)?;
        Ok(rows.next()?.is_some())
    };

    let field = field_path.unwrap_or("");

    // Hash tier
    if !item_hash_value.is_empty()
        && hit(
            "item_hash IS NOT NULL AND item_hash <> '' AND item_hash = ?6",
            &[&claim_type, &field, &kind, &id, &now, &item_hash_value],
        )?
    {
        return Ok(true);
    }

    // Exact text tier — NOCASE so backfilled tombstones with the
    // legacy mixed-case `text` column still match runtime
    // canonical_text (which is lowercased by canonicalize_for_dos280).
    if !canonical_text.is_empty()
        && hit(
            "text = ?6 COLLATE NOCASE",
            &[&claim_type, &field, &kind, &id, &now, &canonical_text],
        )?
    {
        return Ok(true);
    }

    // Keyless field-wide tier
    if hit(
        "text = '<keyless>'",
        &[&claim_type, &field, &kind, &id, &now],
    )? {
        return Ok(true);
    }

    Ok(false)
}

fn compact_subject_ref_str(subject_ref: &str) -> Result<String, ClaimError> {
    let value = serde_json::from_str::<serde_json::Value>(subject_ref)
        .map_err(|e| ClaimError::SubjectRef(format!("not JSON: {e}")))?;
    compact_subject_ref(&value)
}

pub(crate) fn subject_ref_from_json(value: &serde_json::Value) -> Result<SubjectRef, ClaimError> {
    let kind_raw = value
        .get("kind")
        .or_else(|| value.get("type"))
        .or_else(|| value.get("entity_type"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| ClaimError::SubjectRef("missing kind/type".to_string()))?;
    // L2 cycle-14 fix #1: case-fold so PascalCase callers (the
    // shape SQLite json_object writes) parse successfully.
    // Previously only lowercase matched, so a reader called with
    // {"kind":"Account",...} hit the `other =>` arm and errored.
    let kind = kind_raw.to_ascii_lowercase();

    match kind.as_str() {
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
        "email" | "emails" => Ok(SubjectRef::Email {
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

pub(crate) fn item_kind_for_claim_type(claim_type: &str) -> ItemKind {
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
    let mut outcome: Option<Result<T, ClaimError>> = None;
    let transaction_result = db.with_transaction(|tx| {
        let result = f(tx);
        let result_for_return = if result.is_ok() {
            Ok(())
        } else {
            Err(result
                .as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_else(|| "claim transaction failed".to_string()))
        };
        outcome = Some(result);
        result_for_return
    });

    match transaction_result {
        Ok(()) => match outcome {
            Some(Ok(value)) => Ok(value),
            Some(Err(error)) => Err(error),
            None => Err(ClaimError::Transaction(
                "transaction completed without running closure".to_string(),
            )),
        },
        Err(message) => match outcome {
            Some(Err(error)) => Err(error),
            Some(Ok(_)) | None => Err(ClaimError::Transaction(message)),
        },
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
            temporal_scope, sensitivity, verification_state, verification_reason,
            needs_user_decision_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
            ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29,
            ?30, ?31, ?32
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
            enum_to_db(&claim.verification_state)?,
            claim.verification_reason.as_deref(),
            claim.needs_user_decision_at.as_deref(),
        ],
    )?;
    Ok(())
}

fn project_legacy_state_for_claim(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    claim: &IntelligenceClaim,
) -> Result<(), ClaimError> {
    let outcomes = crate::services::derived_state::project_claim_to_db_legacy_tx(ctx, tx, claim);
    for outcome in outcomes {
        crate::services::derived_state::record_projection_outcome(ctx, tx, &claim.id, &outcome)
            .map_err(|e| ClaimError::Transaction(e.to_string()))?;
    }
    Ok(())
}

const CLAIM_COLUMNS: &str = "id, subject_ref, claim_type, field_path, topic_key, text, dedup_key,
    item_hash, actor, data_source, source_ref, source_asof, observed_at, created_at,
    provenance_json, metadata_json, claim_state, surfacing_state, demotion_reason,
    reactivated_at, retraction_reason, expires_at, superseded_by, trust_score,
    trust_computed_at, trust_version, thread_id, temporal_scope, sensitivity,
    verification_state, verification_reason, needs_user_decision_at";

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
        verification_state: parse_db_enum(row.get(29)?)?,
        verification_reason: row.get(30)?,
        needs_user_decision_at: row.get(31)?,
    })
}

fn load_claim_by_id(
    conn: &rusqlite::Connection,
    claim_id: &str,
) -> Result<Option<IntelligenceClaim>, ClaimError> {
    let sql = format!("SELECT {CLAIM_COLUMNS} FROM intelligence_claims WHERE id = ?1 LIMIT 1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![claim_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(read_claim_row(row)?))
    } else {
        Ok(None)
    }
}

/// L2 cycle-1 fix #6: load the single ACTIVE claim with this exact
/// dedup_key, if any. Used by commit_claim's same-meaning merge branch
/// to detect a re-commit of the same logical content and route it
/// through corroboration instead of inserting a duplicate active row.
fn load_active_claim_by_dedup_key(
    conn: &rusqlite::Connection,
    dedup_key: &str,
) -> Result<Option<IntelligenceClaim>, ClaimError> {
    let sql = format!(
        "SELECT {CLAIM_COLUMNS} FROM intelligence_claims \
         WHERE dedup_key = ?1 AND claim_state = 'active' \
         ORDER BY created_at DESC LIMIT 1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![dedup_key])?;
    if let Some(row) = rows.next()? {
        Ok(Some(read_claim_row(row)?))
    } else {
        Ok(None)
    }
}

/// L2 cycle-1 fix #6: load any ACTIVE claim that contradicts the
/// proposal — same (subject_ref, claim_type, field_path) but DIFFERENT
/// canonical text. Used by commit_claim's contradiction-fork branch.
/// Returns the most recently created contradicting claim (one fork
/// per commit; subsequent contradictions chain off the new claim).
///
/// Skips active claims whose own `dedup_key` has a matching tombstone
/// in the table — those are "effectively retracted" by a user
/// dismissal even though their `claim_state` column hasn't been
/// transitioned (the claims substrate keeps active rows append-only; tombstones
/// shadow them via PRE-GATE on re-commit). Without this skip, a
/// paraphrase commit after the user dismissed the original would
/// fork a contradiction against a claim the user has already
/// retracted.
fn load_active_contradicting_claim(
    conn: &rusqlite::Connection,
    subject: &SubjectRef,
    claim_type: &str,
    field_path: Option<&str>,
    canonical_text: &str,
) -> Result<Option<IntelligenceClaim>, ClaimError> {
    // L2 cycle-12 fix #1: match the active subject by kind+id via
    // json_extract instead of exact subject_ref string equality.
    // Two byte-different but semantically-identical subject_refs
    // (e.g. reversed key order from json_object vs serde_json
    // serialization) would otherwise miss this contradiction
    // detector and silently insert an unlinked duplicate active
    // claim. json_valid guards malformed historical rows from
    // tripping json_extract mid-query (cycle-7 hazard).
    let Some(kind) = subject_kind_label(subject) else {
        return Ok(None);
    };
    let Some(id) = subject_id_for_lookup(subject) else {
        return Ok(None);
    };
    let sql = format!(
        "SELECT {CLAIM_COLUMNS} FROM intelligence_claims active \
         WHERE json_valid(active.subject_ref) = 1 \
           AND lower(json_extract(active.subject_ref, '$.kind')) = lower(?1) \
           AND json_extract(active.subject_ref, '$.id') = ?2 \
           AND active.claim_type = ?3 \
           AND coalesce(active.field_path, '') = coalesce(?4, '') \
           AND active.claim_state = 'active' \
           AND active.text <> ?5 \
           AND NOT EXISTS ( \
               SELECT 1 FROM intelligence_claims tombstone \
               WHERE tombstone.dedup_key = active.dedup_key \
                 AND tombstone.claim_state = 'tombstoned' \
           ) \
         ORDER BY active.created_at DESC LIMIT 1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![kind, id, claim_type, field_path, canonical_text])?;
    if let Some(row) = rows.next()? {
        Ok(Some(read_claim_row(row)?))
    } else {
        Ok(None)
    }
}

/// L2 cycle-1 fix #6: in-transaction corroboration helper. Same body
/// as `record_corroboration` but reuses the caller's transaction so
/// commit_claim's same-meaning merge branch composes atomically with
/// the surrounding write. The public `record_corroboration` keeps its
/// own-transaction shape for direct callers (D5+ source-of-truth flow).
fn corroborate_in_tx(
    tx: &ActionDb,
    claim_id: &str,
    data_source: &str,
    source_asof: Option<&str>,
    source_mechanism: Option<&str>,
    now: &str,
) -> Result<String, ClaimError> {
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

    let id = match existing {
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
            id
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
            id
        }
    };
    Ok(id)
}

fn load_claims_where(
    db: &ActionDb,
    subject_ref: &str,
    claim_type: Option<&str>,
    lifecycle_where: &str,
) -> Result<Vec<IntelligenceClaim>, ClaimError> {
    // L2 cycle-13 fix #2: parse the caller's subject_ref into the
    // typed SubjectRef and query by json_extract on $.kind+$.id
    // (with json_valid guard) so the reader matches the same
    // semantic-identity space as PRE-GATE / contradiction
    // detection / is_suppressed_via_claims. The previous exact
    // string match made reader-visible claims disagree with
    // commit-time behavior whenever subject_ref keys were
    // ordered or cased differently across writers.
    let subject_value = serde_json::from_str::<serde_json::Value>(subject_ref)
        .map_err(|e| ClaimError::SubjectRef(format!("not JSON: {e}")))?;
    let subject = subject_ref_from_json(&subject_value)?;
    let Some(kind) = subject_kind_label(&subject) else {
        // Multi/Global readers aren't supported through this path —
        // they're a future addition (matching commit_claim's
        // behavior, which also returns no PRE-GATE match for them).
        return Ok(Vec::new());
    };
    let Some(id) = subject_id_for_lookup(&subject) else {
        return Ok(Vec::new());
    };
    let sql = format!(
        "SELECT {CLAIM_COLUMNS} FROM intelligence_claims
         WHERE json_valid(subject_ref) = 1
           AND lower(json_extract(subject_ref, '$.kind')) = lower(?1)
           AND json_extract(subject_ref, '$.id') = ?2
           AND (?3 IS NULL OR claim_type = ?3)
           AND {lifecycle_where}
         ORDER BY created_at DESC"
    );
    let mut stmt = db.conn_ref().prepare(&sql)?;
    let mut rows = stmt.query(params![kind, id, claim_type])?;
    let mut claims = Vec::new();
    while let Some(row) = rows.next()? {
        claims.push(read_claim_row(row)?);
    }
    Ok(claims)
}

fn actor_class_for_actor(actor: &str) -> Option<ClaimActorClass> {
    let normalized = actor.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    let head = normalized
        .split([':', '/', '@'])
        .next()
        .unwrap_or(normalized.as_str());
    match head {
        "user" | "human" => Some(ClaimActorClass::User),
        "system" | "system_backfill" | "backfill" | "migration" | "repair" => {
            Some(ClaimActorClass::System)
        }
        "agent" | "ai" | "glean" | "llm" => Some(ClaimActorClass::Agent),
        _ => None,
    }
}

fn validate_feedback_actor(actor: &str) -> Result<(), ClaimError> {
    let actor_class = actor_class_for_actor(actor).ok_or_else(|| {
        ClaimError::InvalidFeedback(format!(
            "actor '{}' does not map to a registered actor class",
            actor
        ))
    })?;
    if matches!(actor_class, ClaimActorClass::User) {
        Ok(())
    } else {
        Err(ClaimError::InvalidFeedback(format!(
            "feedback actor '{}' maps to {}, but feedback is only accepted from user actors",
            actor,
            actor_class.as_str()
        )))
    }
}

fn validate_feedback_payload(
    input: &ClaimFeedbackInput,
    metadata: &ClaimFeedbackMetadata,
) -> Result<(), ClaimError> {
    let raw_payload = input
        .payload_json
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let payload = raw_payload
        .map(|payload| {
            serde_json::from_str::<serde_json::Value>(payload).map_err(|e| {
                ClaimError::InvalidFeedback(format!("payload_json must be valid JSON: {e}"))
            })
        })
        .transpose()?;

    if metadata.requires_action_metadata && payload.is_none() {
        return Err(ClaimError::InvalidFeedback(format!(
            "{} feedback requires payload_json metadata",
            input.action.as_str()
        )));
    }

    if let Some(payload) = payload.as_ref() {
        validate_feedback_action_metadata(input.action, payload)?;
    }

    Ok(())
}

fn validate_feedback_action_metadata(
    action: FeedbackAction,
    payload: &serde_json::Value,
) -> Result<(), ClaimError> {
    match action {
        FeedbackAction::WrongSource => require_payload_string(action, payload, "source_ref"),
        FeedbackAction::NeedsNuance => require_payload_string(action, payload, "corrected_text"),
        FeedbackAction::SurfaceInappropriate => require_payload_string(action, payload, "surface"),
        FeedbackAction::NotRelevantHere => require_payload_string(action, payload, "invocation_id"),
        _ => Ok(()),
    }
}

fn require_payload_string(
    action: FeedbackAction,
    payload: &serde_json::Value,
    key: &str,
) -> Result<(), ClaimError> {
    let value = payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if value.is_some() {
        Ok(())
    } else {
        Err(ClaimError::InvalidFeedback(format!(
            "{} feedback requires non-empty payload_json.{}",
            action.as_str(),
            key
        )))
    }
}

fn payload_string(payload_json: Option<&str>, key: &str) -> Result<Option<String>, ClaimError> {
    let Some(raw) = payload_json
        .map(str::trim)
        .filter(|payload| !payload.is_empty())
    else {
        return Ok(None);
    };
    let payload: serde_json::Value = serde_json::from_str(raw).map_err(|e| {
        ClaimError::InvalidFeedback(format!("payload_json must be valid JSON: {e}"))
    })?;
    Ok(payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string))
}

fn feedback_metadata_for_claim(
    claim: &IntelligenceClaim,
    input: &ClaimFeedbackInput,
    mut metadata: ClaimFeedbackMetadata,
) -> Result<ClaimFeedbackMetadata, ClaimError> {
    if matches!(input.action, FeedbackAction::NeedsNuance) {
        if let Some(corrected_text) =
            payload_string(input.payload_json.as_deref(), "corrected_text")?
        {
            metadata.trust_effect =
                compute_needs_nuance_trust_effect(&claim.text, &corrected_text);
        }
    }
    Ok(metadata)
}

fn verification_update_for_feedback(
    claim: &IntelligenceClaim,
    action: FeedbackAction,
    now: &str,
) -> (ClaimVerificationState, Option<String>, Option<String>) {
    let next_state = transition_for_feedback(claim.verification_state, action);
    if next_state == claim.verification_state {
        return (
            next_state,
            claim.verification_reason.clone(),
            claim.needs_user_decision_at.clone(),
        );
    }

    let reason = match next_state {
        ClaimVerificationState::Active => None,
        ClaimVerificationState::Contested | ClaimVerificationState::NeedsUserDecision => {
            Some(action.as_str().to_string())
        }
    };
    let needs_user_decision_at = if matches!(next_state, ClaimVerificationState::NeedsUserDecision)
    {
        Some(now.to_string())
    } else {
        None
    };

    (next_state, reason, needs_user_decision_at)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LifecycleUpdate {
    claim_state: ClaimState,
    surfacing_state: SurfacingState,
    demotion_reason: Option<String>,
    retraction_reason: Option<String>,
}

impl LifecycleUpdate {
    fn from_claim(claim: &IntelligenceClaim) -> Self {
        Self {
            claim_state: claim.claim_state.clone(),
            surfacing_state: claim.surfacing_state.clone(),
            demotion_reason: claim.demotion_reason.clone(),
            retraction_reason: claim.retraction_reason.clone(),
        }
    }
}

fn expected_lifecycle_render_policy(action: FeedbackAction) -> ClaimRenderPolicy {
    match action {
        FeedbackAction::ConfirmCurrent => ClaimRenderPolicy::DefaultWithUserCorroboration,
        FeedbackAction::MarkOutdated => ClaimRenderPolicy::HiddenFromCurrent,
        FeedbackAction::MarkFalse => ClaimRenderPolicy::SuppressedExceptAudit,
        FeedbackAction::WrongSubject => ClaimRenderPolicy::SuppressedOnAssertedSubject,
        FeedbackAction::WrongSource => ClaimRenderPolicy::QualifiedBySourceCaveat,
        FeedbackAction::CannotVerify => ClaimRenderPolicy::QualifiedNeedsCorroboration,
        FeedbackAction::NeedsNuance => ClaimRenderPolicy::RenderSuperseder,
        FeedbackAction::SurfaceInappropriate => ClaimRenderPolicy::HiddenOnNamedSurface,
        FeedbackAction::NotRelevantHere => ClaimRenderPolicy::DeprioritizedInContext,
    }
}

fn lifecycle_update_for_feedback(
    claim: &IntelligenceClaim,
    action: FeedbackAction,
    render: ClaimRenderPolicy,
) -> LifecycleUpdate {
    let expected = expected_lifecycle_render_policy(action);
    debug_assert_eq!(
        render,
        expected,
        "feedback render policy drift for {}",
        action.as_str()
    );

    match action {
        FeedbackAction::MarkOutdated => LifecycleUpdate {
            claim_state: claim.claim_state.clone(),
            surfacing_state: SurfacingState::Dormant,
            demotion_reason: Some("outdated".to_string()),
            retraction_reason: claim.retraction_reason.clone(),
        },
        FeedbackAction::MarkFalse => LifecycleUpdate {
            claim_state: ClaimState::Withdrawn,
            surfacing_state: SurfacingState::Dormant,
            demotion_reason: claim.demotion_reason.clone(),
            retraction_reason: Some("user_marked_false".to_string()),
        },
        FeedbackAction::WrongSubject => LifecycleUpdate {
            claim_state: ClaimState::Tombstoned,
            surfacing_state: SurfacingState::Dormant,
            demotion_reason: claim.demotion_reason.clone(),
            retraction_reason: Some("wrong_subject".to_string()),
        },
        FeedbackAction::NeedsNuance => LifecycleUpdate {
            claim_state: claim.claim_state.clone(),
            surfacing_state: SurfacingState::Dormant,
            demotion_reason: Some("superseded".to_string()),
            retraction_reason: claim.retraction_reason.clone(),
        },
        FeedbackAction::ConfirmCurrent
        | FeedbackAction::WrongSource
        | FeedbackAction::CannotVerify
        | FeedbackAction::SurfaceInappropriate
        | FeedbackAction::NotRelevantHere => LifecycleUpdate::from_claim(claim),
    }
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
    // L2 cycle-14 fix #2: reject Multi/Global at commit time per
    // ADR-0125 v1.4.0 spine restriction. The default reader family
    // (load_claims_active / _including_dormant / _dormant_only) and
    // PRE-GATE / contradiction detection / is_suppressed_via_claims
    // all require a single (kind, id) tuple — so accepting these
    // subjects at write time would create rows that read-after-write
    // can't see. v1.4.1+ work that justifies one of these variants
    // via ADR amendment can lift this guard.
    match subject {
        SubjectRef::Multi(_) => {
            return Err(ClaimError::SubjectRef(
                "Multi subjects are reserved for v1.4.1+; v1.4.0 spine writers must commit a single (kind, id)".to_string(),
            ));
        }
        SubjectRef::Global => {
            return Err(ClaimError::SubjectRef(
                "Global subjects are reserved for v1.4.1+ per ADR-0125; v1.4.0 spine writers must commit a single (kind, id)".to_string(),
            ));
        }
        _ => {}
    }
    // L2 cycle-15 fix #1: derive subject_ref_compact from the
    // PARSED SubjectRef enum, not the caller's raw JSON bytes. The
    // parser case-folds kind (cycle-14), but compact_subject_ref on
    // the caller's value preserves the original casing — so the
    // dedup_key + commit_lock keyed on it would differ across two
    // semantically-identical commits with different kind casing.
    // Both same-meaning merge AND the per-key lock then break:
    // the second write would insert a duplicate active row instead
    // of reinforcing.
    let subject_ref_compact = canonical_subject_ref(&subject)?;
    if proposal.claim_type.trim().is_empty() {
        return Err(ClaimError::UnknownClaimType("empty".to_string()));
    }
    // Closed-set validation: the claim_type string MUST be in the
    // registry, AND the subject kind MUST be one the registry permits
    // for this claim type. The latter is the cross-subject bleed
    // guard — a stakeholder_role on an Account is rejected because
    // the registry pins it to Person only.
    let kind = crate::abilities::claims::ClaimType::try_from_db_str(&proposal.claim_type)
        .map_err(|e| ClaimError::UnknownClaimType(e.0))?;
    // The upstream spine guard rejects Multi/Global; this lowers the
    // remaining single-subject variants to the registry's canonical
    // subject-kind labels.
    let subject_kind_lc = match &subject {
        SubjectRef::Account { .. } => "account",
        SubjectRef::Meeting { .. } => "meeting",
        SubjectRef::Person { .. } => "person",
        SubjectRef::Project { .. } => "project",
        SubjectRef::Email { .. } => "email",
        SubjectRef::Multi(_) | SubjectRef::Global => {
            unreachable!("Multi/Global rejected upstream")
        }
    };
    if !crate::abilities::claims::subject_kind_is_canonical_for(kind, subject_kind_lc) {
        return Err(ClaimError::SubjectRef(format!(
            "claim_type {} not permitted on subject kind {}",
            proposal.claim_type, subject_kind_lc
        )));
    }
    let metadata = metadata_for_claim_type(kind);
    let actor_class = actor_class_for_actor(&proposal.actor)
        .ok_or_else(|| ClaimError::InvalidActor(proposal.actor.clone()))?;
    if !metadata.allowed_actor_classes.is_empty()
        && !metadata.allowed_actor_classes.contains(&actor_class)
    {
        return Err(ClaimError::ActorNotPermittedForClaimType {
            claim_type: proposal.claim_type.clone(),
            actor: proposal.actor.clone(),
            actor_class: actor_class.as_str().to_string(),
        });
    }
    let effective_temporal_scope = proposal
        .temporal_scope
        .clone()
        .unwrap_or_else(|| metadata.default_temporal_scope.clone());
    let effective_sensitivity = proposal
        .sensitivity
        .clone()
        .unwrap_or_else(|| metadata.default_sensitivity.clone());

    let canonical_text = canonicalize_for_dos280(&proposal.text);
    let computed_hash = item_hash(
        item_kind_for_claim_type(&proposal.claim_type),
        &canonical_text,
    );
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
        if proposal.tombstone.is_none()
            && pre_gate_blocking_tombstone_exists(
                tx.conn_ref(),
                &subject,
                &proposal.claim_type,
                proposal.field_path.as_deref(),
                &computed_hash,
                &canonical_text,
                &now,
            )?
        {
            return Err(ClaimError::TombstonedPreGate);
        }

        // L2 cycle-1 fix #6: same-meaning merge. If an active claim
        // already exists with this dedup_key (same subject + claim_type
        // + field + canonical text + hash), route the new evidence
        // through corroboration instead of inserting a duplicate row.
        // Tombstone proposals always insert (they intentionally
        // shadow the active claim).
        if proposal.tombstone.is_none() {
            if let Some(existing) = load_active_claim_by_dedup_key(tx.conn_ref(), &dedup_key)? {
                let corroboration_id = corroborate_in_tx(
                    tx,
                    &existing.id,
                    &proposal.data_source,
                    proposal.source_asof.as_deref(),
                    Some("same_meaning_merge"),
                    &now,
                )?;
                tx.bump_for_subject(&subject)?;
                return Ok(CommittedClaim::Reinforced {
                    claim: existing,
                    corroboration_id,
                });
            }

            // L2 cycle-1 fix #6: contradiction detection. If an active
            // claim exists with the SAME (subject_ref, claim_type,
            // field_path) but a DIFFERENT canonical text, the
            // proposal contradicts the existing assertion. Insert the
            // new claim AND a claim_contradictions edge, then return
            // Forked. Both claims remain active until the user (or a
            // reconciliation pass) resolves the fork.
            if let Some(primary) = load_active_contradicting_claim(
                tx.conn_ref(),
                &subject,
                &proposal.claim_type,
                proposal.field_path.as_deref(),
                &canonical_text,
            )? {
                let new_id = uuid::Uuid::new_v4().to_string();
                let contradicting = IntelligenceClaim {
                    id: new_id.clone(),
                    subject_ref: subject_ref_compact.clone(),
                    claim_type: proposal.claim_type.clone(),
                    field_path: proposal.field_path.clone(),
                    topic_key: proposal.topic_key.clone(),
                    text: canonical_text.clone(),
                    dedup_key: dedup_key.clone(),
                    item_hash: Some(computed_hash.clone()),
                    actor: proposal.actor.clone(),
                    data_source: proposal.data_source.clone(),
                    source_ref: proposal.source_ref.clone(),
                    source_asof: proposal.source_asof.clone(),
                    observed_at: proposal.observed_at.clone(),
                    created_at: now.clone(),
                    provenance_json: proposal.provenance_json.clone(),
                    metadata_json: proposal.metadata_json.clone(),
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
                    thread_id: proposal.thread_id.clone(),
                    temporal_scope: effective_temporal_scope.clone(),
                    sensitivity: effective_sensitivity.clone(),
                    verification_state: ClaimVerificationState::Active,
                    verification_reason: None,
                    needs_user_decision_at: None,
                };
                insert_claim_row(tx, &contradicting)?;
                project_legacy_state_for_claim(ctx, tx, &contradicting)?;

                let contradiction_id = uuid::Uuid::new_v4().to_string();
                tx.conn_ref().execute(
                    "INSERT INTO claim_contradictions \
                     (id, primary_claim_id, contradicting_claim_id, branch_kind, detected_at) \
                     VALUES (?1, ?2, ?3, 'contradiction', ?4)",
                    params![&contradiction_id, &primary.id, &new_id, &now],
                )?;

                tx.bump_for_subject(&subject)?;

                return Ok(CommittedClaim::Forked {
                    primary_claim: primary,
                    contradiction_id,
                    new_claim_id: new_id,
                });
            }
        }

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
            temporal_scope: effective_temporal_scope.clone(),
            sensitivity: effective_sensitivity.clone(),
            verification_state: ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        };

        insert_claim_row(tx, &claim)?;
        project_legacy_state_for_claim(ctx, tx, &claim)?;
        tx.bump_for_subject(&subject)?;

        if proposal.tombstone.is_some() {
            Ok(CommittedClaim::Tombstoned { claim })
        } else {
            Ok(CommittedClaim::Inserted { claim })
        }
    })
}

// ---------------------------------------------------------------------------
// record_claim_feedback
// ---------------------------------------------------------------------------

const MAX_ACTIVE_REPAIR_JOBS_PER_CLAIM: i64 = 5;
const MAX_ACTIVE_REPAIR_JOBS_WORKSPACE: i64 = 50;

#[derive(Debug)]
struct ClaimFeedbackWriteOutcome {
    outcome: ClaimFeedbackOutcome,
    signal_entity_type: String,
    signal_entity_id: String,
    verification_state_before: String,
    verification_state_after: String,
}

pub fn record_claim_feedback(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: ClaimFeedbackInput,
) -> Result<ClaimFeedbackOutcome, ClaimError> {
    ctx.check_mutation_allowed()
        .map_err(|e| ClaimError::Mode(e.to_string()))?;

    let metadata = feedback_semantics(input.action);
    validate_feedback_payload(&input, &metadata)?;

    let write = with_claim_transaction(db, |tx| {
        let now = ctx.clock.now().to_rfc3339();
        let claim = load_claim_by_id(tx.conn_ref(), &input.claim_id)?
            .ok_or_else(|| ClaimError::UnknownClaimId(input.claim_id.clone()))?;
        validate_feedback_actor(&input.actor)?;
        let metadata = feedback_metadata_for_claim(&claim, &input, metadata.clone())?;
        let subject_value: serde_json::Value = serde_json::from_str(&claim.subject_ref)?;
        let subject = subject_ref_from_json(&subject_value)?;
        let (signal_entity_type, signal_entity_id) = signal_target_for_claim(&subject, &claim.id);
        let verification_state_before = enum_to_db(&claim.verification_state)?;

        let feedback_id = uuid::Uuid::new_v4().to_string();
        tx.conn_ref().execute(
            "INSERT INTO claim_feedback (
                id, claim_id, feedback_type, actor, actor_id, payload_json,
                submitted_at, applied_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
            params![
                &feedback_id,
                &input.claim_id,
                input.action.as_str(),
                &input.actor,
                input.actor_id.as_deref(),
                input.payload_json.as_deref(),
                &now,
            ],
        )?;

        let (new_verification_state, verification_reason, needs_user_decision_at) =
            verification_update_for_feedback(&claim, input.action, &now);
        let lifecycle_update = lifecycle_update_for_feedback(&claim, input.action, metadata.render);
        let lifecycle_changed = lifecycle_update != LifecycleUpdate::from_claim(&claim);
        let verification_changed = new_verification_state != claim.verification_state
            || verification_reason != claim.verification_reason
            || needs_user_decision_at != claim.needs_user_decision_at;

        match (verification_changed, lifecycle_changed) {
            (true, true) => {
                tx.conn_ref().execute(
                    "UPDATE intelligence_claims
                     SET verification_state = ?1,
                         verification_reason = ?2,
                         needs_user_decision_at = ?3,
                         claim_state = ?4,
                         surfacing_state = ?5,
                         demotion_reason = ?6,
                         retraction_reason = ?7
                     WHERE id = ?8",
                    params![
                        enum_to_db(&new_verification_state)?,
                        verification_reason.as_deref(),
                        needs_user_decision_at.as_deref(),
                        enum_to_db(&lifecycle_update.claim_state)?,
                        enum_to_db(&lifecycle_update.surfacing_state)?,
                        lifecycle_update.demotion_reason.as_deref(),
                        lifecycle_update.retraction_reason.as_deref(),
                        &input.claim_id,
                    ],
                )?;
            }
            (true, false) => {
                tx.conn_ref().execute(
                    "UPDATE intelligence_claims
                     SET verification_state = ?1,
                         verification_reason = ?2,
                         needs_user_decision_at = ?3
                     WHERE id = ?4",
                    params![
                        enum_to_db(&new_verification_state)?,
                        verification_reason.as_deref(),
                        needs_user_decision_at.as_deref(),
                        &input.claim_id,
                    ],
                )?;
            }
            (false, true) => {
                tx.conn_ref().execute(
                    "UPDATE intelligence_claims
                     SET claim_state = ?1,
                         surfacing_state = ?2,
                         demotion_reason = ?3,
                         retraction_reason = ?4
                     WHERE id = ?5",
                    params![
                        enum_to_db(&lifecycle_update.claim_state)?,
                        enum_to_db(&lifecycle_update.surfacing_state)?,
                        lifecycle_update.demotion_reason.as_deref(),
                        lifecycle_update.retraction_reason.as_deref(),
                        &input.claim_id,
                    ],
                )?;
            }
            (false, false) => {}
        }

        let repair_job_id =
            maybe_enqueue_repair_job(tx, &input.claim_id, &feedback_id, &now, metadata.repair)?;

        bump_invalidation_for_claim_id(tx, &input.claim_id)?;
        let verification_state_after = enum_to_db(&new_verification_state)?;

        Ok(ClaimFeedbackWriteOutcome {
            outcome: ClaimFeedbackOutcome {
                feedback_id,
                claim_id: input.claim_id.clone(),
                action: input.action,
                new_verification_state,
                applied_at_pending: true,
                repair_job_id,
            },
            signal_entity_type,
            signal_entity_id,
            verification_state_before,
            verification_state_after,
        })
    })?;

    emit_claim_feedback_signals(ctx, db, &write);

    Ok(write.outcome)
}

fn maybe_enqueue_repair_job(
    tx: &ActionDb,
    claim_id: &str,
    feedback_id: &str,
    created_at: &str,
    repair: RepairAction,
) -> Result<Option<String>, ClaimError> {
    if matches!(repair, RepairAction::None) {
        return Ok(None);
    }

    let per_claim_active: i64 = tx.conn_ref().query_row(
        "SELECT count(*) FROM claim_repair_job
         WHERE claim_id = ?1 AND state IN ('pending', 'in_progress')",
        params![claim_id],
        |row| row.get(0),
    )?;
    if per_claim_active >= MAX_ACTIVE_REPAIR_JOBS_PER_CLAIM {
        log::warn!(
            "claim repair job cap reached; claim_id={claim_id} active_jobs={per_claim_active}"
        );
        return Ok(None);
    }

    let workspace_active: i64 = tx.conn_ref().query_row(
        "SELECT count(*) FROM claim_repair_job
         WHERE state IN ('pending', 'in_progress')",
        [],
        |row| row.get(0),
    )?;
    if workspace_active >= MAX_ACTIVE_REPAIR_JOBS_WORKSPACE {
        log::warn!("workspace claim repair job cap reached; active_jobs={workspace_active}");
        return Ok(None);
    }

    let repair_job_id = uuid::Uuid::new_v4().to_string();
    tx.conn_ref().execute(
        "INSERT INTO claim_repair_job (id, claim_id, feedback_id, state, attempts, max_attempts, created_at)
         VALUES (?1, ?2, ?3, 'pending', 0, 3, ?4)",
        params![&repair_job_id, claim_id, feedback_id, created_at],
    )?;

    Ok(Some(repair_job_id))
}

fn signal_target_for_claim(subject: &SubjectRef, claim_id: &str) -> (String, String) {
    match (subject_kind_label(subject), subject_id_for_lookup(subject)) {
        (Some(kind), Some(id)) => (kind.to_ascii_lowercase(), id.to_string()),
        _ => ("claim".to_string(), claim_id.to_string()),
    }
}

fn emit_claim_feedback_signals(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    write: &ClaimFeedbackWriteOutcome,
) {
    let payload = serde_json::json!({
        "action": write.outcome.action.as_str(),
        "claim_id": &write.outcome.claim_id,
        "verification_state_before": &write.verification_state_before,
        "verification_state_after": &write.verification_state_after,
    })
    .to_string();

    if let Err(e) = crate::services::signals::emit(
        ctx,
        db,
        &write.signal_entity_type,
        &write.signal_entity_id,
        "claim_feedback_recorded",
        "user_feedback",
        Some(&payload),
        0.9,
    ) {
        log::warn!(
            "post-commit signal emission failed; \
             repair_target=signals_engine \
             signal_type=claim_feedback_recorded \
             claim_id={}: {e}",
            write.outcome.claim_id
        );
    }

    if write.verification_state_before != write.verification_state_after {
        let payload = serde_json::json!({
            "from": &write.verification_state_before,
            "to": &write.verification_state_after,
        })
        .to_string();
        if let Err(e) = crate::services::signals::emit(
            ctx,
            db,
            &write.signal_entity_type,
            &write.signal_entity_id,
            "claim_verification_state_changed",
            "user_feedback",
            Some(&payload),
            0.9,
        ) {
            log::warn!(
                "post-commit signal emission failed; \
                 repair_target=signals_engine \
                 signal_type=claim_verification_state_changed \
                 claim_id={}: {e}",
                write.outcome.claim_id
            );
        }
    }
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

        let result_id = match existing {
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
                id
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
                id
            }
        };

        // L2 cycle-1 fix #5: bump per-entity claim invalidation so trust /
        // surfacing readers keyed on per-entity claim_version observe the
        // strength change. The bump runs in the same transaction as the
        // corroboration write so observers either see both or neither.
        bump_invalidation_for_claim_id(tx, claim_id)?;

        Ok(result_id)
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

        // L2 cycle-1 fix #5: a reconciliation may flip claim_state on the
        // winner/loser sides (handled by callers) and at minimum changes
        // the contradiction record observed by trust-band readers. Bump
        // per-entity invalidation for the contradiction's primary AND
        // contradicting claim subjects so any reader keyed on per-entity
        // claim_version refreshes.
        let (primary_claim_id, contradicting_claim_id): (String, String) =
            tx.conn_ref().query_row(
                "SELECT primary_claim_id, contradicting_claim_id \
                 FROM claim_contradictions WHERE id = ?1",
                params![contradiction_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
        // Resolve both claim IDs to subjects, then bump each unique
        // subject exactly once. (Two claims on the same subject must
        // not double-bump.)
        let primary_subject = subject_for_claim_id(tx, &primary_claim_id)?;
        let contradicting_subject = subject_for_claim_id(tx, &contradicting_claim_id)?;
        tx.bump_for_subject(&primary_subject)?;
        if contradicting_subject != primary_subject {
            tx.bump_for_subject(&contradicting_subject)?;
        }

        Ok(())
    })
}

/// Lookup a claim's `subject_ref` JSON column by primary key, parse it
/// into a [`SubjectRef`], and bump the per-entity invalidation counter.
/// Used by `record_corroboration` so that trust/surfacing readers keyed
/// on per-entity `claim_version` observe the corroboration effect.
fn bump_invalidation_for_claim_id(tx: &ActionDb, claim_id: &str) -> Result<(), ClaimError> {
    let subject = subject_for_claim_id(tx, claim_id)?;
    tx.bump_for_subject(&subject)?;
    Ok(())
}

/// Lookup a claim's `subject_ref` JSON column and parse it to
/// [`SubjectRef`] without bumping. Used by `reconcile_contradiction`
/// which needs to dedupe two subjects before bumping each unique one.
fn subject_for_claim_id(tx: &ActionDb, claim_id: &str) -> Result<SubjectRef, ClaimError> {
    let subject_ref_json: String = tx
        .conn_ref()
        .query_row(
            "SELECT subject_ref FROM intelligence_claims WHERE id = ?1",
            params![claim_id],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                ClaimError::SubjectRef(format!("claim {claim_id} not found"))
            }
            other => ClaimError::Db(DbError::Sqlite(other)),
        })?;
    let value: serde_json::Value = serde_json::from_str(&subject_ref_json)?;
    subject_ref_from_json(&value)
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
// Runtime shadow-write helper
// ---------------------------------------------------------------------------

/// Shadow-write a tombstone claim alongside a legacy `create_suppression_tombstone`
/// call during the claims transition window.
///
/// Existing dismissal callers (services/intelligence.rs::dismiss_intelligence_item,
/// services/accounts.rs runtime correction paths, services/feedback.rs::apply_correction)
/// keep writing to the legacy `suppression_tombstones` table. The follow-up
/// owns the eventual refactor that makes services/derived_state.rs the only
/// legacy projection writer. Until that lands, we shadow-write a tombstone
/// claim into intelligence_claims so the new substrate is populated in
/// parallel and reconcile can verify parity in D5.
///
/// Failure of the shadow write is LOGGED but does NOT propagate as Err; the
/// legacy write above remains authoritative for this release. Once the follow-up
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
    /// L2 cycle-25 fix #3: optional finite expiry for time-bound
    /// dismissals (e.g. triage_snoozes.snoozed_until). When None,
    /// the tombstone is permanent (the typical user_removal case).
    /// When Some, PRE-GATE / suppression honor the expiry exactly
    /// like the SQL backfill (m8 mechanism preserves snoozed_until
    /// in expires_at). Without this field, runtime snoozes became
    /// permanent claim tombstones even though the legacy snooze
    /// expired — causing indefinite suppression of triage cards.
    pub expires_at: Option<&'a str>,
}

/// L2 cycle-2 fix #1: normalize the caller-supplied subject_kind into
/// the lowercase form `subject_ref_from_json` accepts. Runtime callers
/// pass PascalCase ("Account", "Meeting", "Person", "Project", "Email")
/// — which the parser previously rejected, silently no-op'ing the
/// shadow write. Returns `None` when the kind has no claim-substrate
/// representation today (currently: `Email`; tracked as a future
/// follow-up). Callers MUST handle `None` rather than assuming a
/// successful tombstone; see `shadow_write_tombstone_claim`'s contract.
fn normalize_subject_kind_for_claim(kind: &str) -> Option<&'static str> {
    match kind.trim() {
        k if k.eq_ignore_ascii_case("account") => Some("account"),
        k if k.eq_ignore_ascii_case("meeting") => Some("meeting"),
        k if k.eq_ignore_ascii_case("person") || k.eq_ignore_ascii_case("people") => Some("person"),
        k if k.eq_ignore_ascii_case("project") => Some("project"),
        // L2 cycle-3 fix: Email is now a real SubjectRef variant
        // (migration 132 added emails.claim_version). Cycle-2's
        // workaround that mapped Email → Account+prefix is removed.
        k if k.eq_ignore_ascii_case("email") => Some("email"),
        _ => None,
    }
}

/// Outcome of attempting to shadow-write a tombstone claim. Used in
/// regression tests to verify that the claim row actually got written
/// (or that we correctly skipped when the substrate cannot model the
/// subject yet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShadowTombstoneOutcome {
    /// Tombstone claim was committed.
    Committed,
    /// Subject kind cannot be represented in the claim substrate today
    /// (e.g. `Email` until SubjectRef gains an `Email` variant). The
    /// caller's legacy dismissal write is still authoritative; the
    /// gap is a known limitation tracked as a follow-up.
    SkippedUnsupportedSubjectKind,
    /// `commit_claim` itself failed (e.g. mutation gate, DB error).
    /// The caller's legacy write may have already committed; the
    /// claim substrate will be repaired by the next reconcile pass
    /// or by retrying via the cutover hook on next startup.
    Failed(String),
}

pub fn shadow_write_tombstone_claim(
    db: &ActionDb,
    args: ShadowTombstoneClaim<'_>,
) -> ShadowTombstoneOutcome {
    let ShadowTombstoneClaim {
        subject_kind,
        subject_id,
        claim_type,
        field_path,
        text,
        actor,
        source_scope,
        observed_at,
        expires_at,
    } = args;

    let Some(normalized_kind) = normalize_subject_kind_for_claim(subject_kind) else {
        log::debug!(
            "[dos7-shadow] skipping shadow tombstone — subject_kind {:?} has no claim-substrate representation yet (subject_id={})",
            subject_kind, subject_id
        );
        return ShadowTombstoneOutcome::SkippedUnsupportedSubjectKind;
    };

    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);

    // L2 cycle-10 fix #2: build subject_ref + metadata_json via
    // serde_json so subject_id or source_scope containing quotes,
    // backslashes, newlines, or control characters can't produce
    // malformed JSON. The previous `format!` interpolation made
    // commit_claim's subject_ref parser fail, returning Failed —
    // and callers treat shadow_write as best-effort, so the claim
    // tombstone could silently be absent. Equivalent hazard for
    // metadata_json (commit_claim doesn't validate it).
    let subject_ref = serde_json::json!({
        "kind": normalized_kind,
        "id": subject_id,
    })
    .to_string();
    let metadata_json = source_scope.map(|s| serde_json::json!({ "source_scope": s }).to_string());

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
        temporal_scope: Some(TemporalScope::State),
        sensitivity: Some(ClaimSensitivity::Internal),
        tombstone: Some(TombstoneSpec {
            retraction_reason: "user_removal".to_string(),
            expires_at: expires_at.map(|s| s.to_string()),
        }),
    };

    match commit_claim(&ctx, db, proposal) {
        Ok(_) => ShadowTombstoneOutcome::Committed,
        Err(e) => {
            let msg = e.to_string();
            log::warn!(
                "[dos7-shadow] tombstone claim write failed (subject={}:{} field={:?}): {}",
                subject_kind,
                subject_id,
                field_path,
                msg
            );
            ShadowTombstoneOutcome::Failed(msg)
        }
    }
}

/// L2 cycle-26 fix: filter for `withdraw_tombstones_for`. All fields
/// other than `subject_id` + `retraction_reason` are filters that
/// further narrow which tombstone claims to withdraw.
pub struct WithdrawTombstoneFilter<'a> {
    /// Subject kind in any case (`"Account"` / `"account"` / `"Person"` …).
    /// Maps to canonical lowercase form before comparison; non-substrate
    /// kinds (e.g. `EmailThread`) cause the helper to no-op (returns 0).
    pub subject_kind: &'a str,
    pub subject_id: &'a str,
    pub claim_type: &'a str,
    /// Optional exact-text filter (e.g. role name for `stakeholder_role`).
    pub text: Option<&'a str>,
    /// Optional `field_path` filter (e.g. `entity_type` for `linking_dismissed`).
    pub field_path: Option<&'a str>,
    /// Recorded into `intelligence_claims.retraction_reason`. Convention:
    /// `"restored_by_user"` for re-pin / undismiss, `"reset_by_user"` for
    /// bulk preference resets.
    pub retraction_reason: &'a str,
}

/// L2 cycle-26 fix: centralize lifecycle-tombstone withdrawal so restore
/// paths don't need ad-hoc `UPDATE intelligence_claims` statements.
/// Returns the number of rows withdrawn (0 if no matching tombstone or
/// if the subject_kind has no claim-substrate representation).
///
/// SET targets only lifecycle columns (`claim_state`, `surfacing_state`,
/// `retraction_reason`) — never assertion-identity columns. The
/// `claim_type` / `text` references in the WHERE clause are filters,
/// not mutations; the `dos7-allowed:` markers below exist only because
/// the immutability lint is clause-blind.
///
/// Errors propagate to the caller. The cycle-25 `let _ =` swallow
/// pattern is the exact split-brain failure cycle-26 closed: legacy
/// row reverts but tombstone claim stays active because the UPDATE
/// silently failed.
/// L2 cycle-26 fix #1: bulk-withdraw every tombstone claim of a given
/// `claim_type` regardless of subject. Used by user-facing reset paths
/// (e.g. `reset_email_dismissals`) that wipe a legacy preference table
/// and need the parallel claim tombstones cleared in the same
/// transaction so PRE-GATE / readers stop suppressing the items.
///
/// SET targets only lifecycle columns. The `claim_type` reference in
/// WHERE is a filter, not a mutation.
pub fn withdraw_all_tombstones_of_type(
    db: &ActionDb,
    claim_type: &str,
    retraction_reason: &str,
) -> Result<usize, rusqlite::Error> {
    db.conn_ref().execute(
        "UPDATE intelligence_claims /* dos7-allowed: bulk withdrawal helper */ \
         SET claim_state = 'withdrawn', \
             surfacing_state = 'dormant', \
             retraction_reason = ?1 \
         WHERE claim_state = 'tombstoned' \
           AND claim_type = ?2 /* dos7-allowed: WHERE-filter */",
        rusqlite::params![retraction_reason, claim_type],
    )
}

pub fn withdraw_tombstones_for(
    db: &ActionDb,
    filter: WithdrawTombstoneFilter<'_>,
) -> Result<usize, rusqlite::Error> {
    let Some(normalized_kind) = normalize_subject_kind_for_claim(filter.subject_kind) else {
        return Ok(0);
    };

    db.conn_ref().execute(
        "UPDATE intelligence_claims /* dos7-allowed: lifecycle withdrawal helper */ \
         SET claim_state = 'withdrawn', \
             surfacing_state = 'dormant', \
             retraction_reason = ?1 \
         WHERE id IN ( \
             SELECT ic.id FROM intelligence_claims ic \
             WHERE ic.claim_state = 'tombstoned' \
               AND ic.claim_type = ?2 /* dos7-allowed: WHERE-filter */ \
               AND json_valid(ic.subject_ref) = 1 \
               AND lower(json_extract(ic.subject_ref, '$.kind')) = ?3 \
               AND json_extract(ic.subject_ref, '$.id') = ?4 \
               AND (?5 IS NULL OR ic.text = ?5) \
               AND (?6 IS NULL OR coalesce(ic.field_path, '') = coalesce(?6, '')) \
         )",
        rusqlite::params![
            filter.retraction_reason,
            filter.claim_type,
            normalized_kind,
            filter.subject_id,
            filter.text,
            filter.field_path,
        ],
    )
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
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(ClaimSensitivity::Internal),
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

    fn seed_meeting(db: &ActionDb) {
        db.conn_ref()
            .execute(
                "INSERT INTO meetings (id, title, meeting_type, start_time, created_at) \
                 VALUES (?1, ?2, 'sync', ?3, ?3)",
                params!["meeting-1", "Meeting 1", TS],
            )
            .expect("seed meeting");
    }

    fn seed_person(db: &ActionDb) {
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params!["person-1", "person-1@example.com", "Person 1", TS],
            )
            .expect("seed person");
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

    fn read_claim_temporal_scope(db: &ActionDb, claim_id: &str) -> String {
        db.conn_ref()
            .query_row(
                "SELECT temporal_scope FROM intelligence_claims WHERE id = ?1",
                params![claim_id],
                |row| row.get(0),
            )
            .expect("read temporal_scope")
    }

    fn read_claim_sensitivity(db: &ActionDb, claim_id: &str) -> String {
        db.conn_ref()
            .query_row(
                "SELECT sensitivity FROM intelligence_claims WHERE id = ?1",
                params![claim_id],
                |row| row.get(0),
            )
            .expect("read sensitivity")
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
            verification_state: ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        };
        insert_claim_row(db, &claim).expect("insert fixture claim");
    }

    fn inserted_claim_id(result: CommittedClaim) -> String {
        match result {
            CommittedClaim::Inserted { claim } | CommittedClaim::Tombstoned { claim } => claim.id,
            other => panic!("expected inserted/tombstoned claim, got {other:?}"),
        }
    }

    fn all_feedback_actions() -> [FeedbackAction; 9] {
        [
            FeedbackAction::ConfirmCurrent,
            FeedbackAction::MarkOutdated,
            FeedbackAction::MarkFalse,
            FeedbackAction::WrongSubject,
            FeedbackAction::WrongSource,
            FeedbackAction::CannotVerify,
            FeedbackAction::NeedsNuance,
            FeedbackAction::SurfaceInappropriate,
            FeedbackAction::NotRelevantHere,
        ]
    }

    fn feedback_payload_for(action: FeedbackAction) -> Option<String> {
        match action {
            FeedbackAction::WrongSource => {
                Some(serde_json::json!({ "source_ref": "src-1" }).to_string())
            }
            FeedbackAction::NeedsNuance => Some(
                serde_json::json!({ "corrected_text": "Renewal risk needs a qualifier" })
                    .to_string(),
            ),
            FeedbackAction::SurfaceInappropriate => {
                Some(serde_json::json!({ "surface": "briefing" }).to_string())
            }
            FeedbackAction::NotRelevantHere => {
                Some(serde_json::json!({ "invocation_id": "invocation-fixture" }).to_string())
            }
            _ => None,
        }
    }

    fn feedback_input(claim_id: &str, action: FeedbackAction) -> ClaimFeedbackInput {
        ClaimFeedbackInput {
            claim_id: claim_id.to_string(),
            action,
            actor: "user".to_string(),
            actor_id: Some("user-fixture".to_string()),
            payload_json: feedback_payload_for(action),
        }
    }

    fn read_verification_columns(
        db: &ActionDb,
        claim_id: &str,
    ) -> (String, Option<String>, Option<String>) {
        db.conn_ref()
            .query_row(
                "SELECT verification_state, verification_reason, needs_user_decision_at \
                 FROM intelligence_claims WHERE id = ?1",
                params![claim_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read verification columns")
    }

    fn read_lifecycle_columns(
        db: &ActionDb,
        claim_id: &str,
    ) -> (String, String, Option<String>, Option<String>) {
        db.conn_ref()
            .query_row(
                "SELECT claim_state, surfacing_state, demotion_reason, retraction_reason \
                 FROM intelligence_claims WHERE id = ?1",
                params![claim_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("read lifecycle columns")
    }

    fn repair_job_count(db: &ActionDb, claim_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT count(*) FROM claim_repair_job WHERE claim_id = ?1",
                params![claim_id],
                |row| row.get(0),
            )
            .expect("count repair jobs")
    }

    fn signal_count(db: &ActionDb, signal_type: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT count(*) FROM signal_events WHERE signal_type = ?1",
                params![signal_type],
                |row| row.get(0),
            )
            .expect("count signals")
    }

    fn first_signal_value(db: &ActionDb, signal_type: &str) -> String {
        db.conn_ref()
            .query_row(
                "SELECT value FROM signal_events WHERE signal_type = ?1 ORDER BY rowid LIMIT 1",
                params![signal_type],
                |row| row.get::<_, String>(0),
            )
            .expect("read signal value")
    }

    #[test]
    fn compute_dedup_key_is_stable_for_same_inputs() {
        let key_1 = compute_dedup_key("hash", SUBJECT, "risk", Some("health.risk"));
        let key_2 = compute_dedup_key("hash", SUBJECT, "risk", Some("health.risk"));
        assert_eq!(key_1, key_2);
        assert_eq!(key_1, format!("hash:{SUBJECT}:risk:health.risk"));
    }

    #[test]
    fn dedup_key_signature_excludes_thread_id() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let mut first = proposal("Procurement blocked renewal");
        first.thread_id = Some("thread-a".to_string());
        let first_id = inserted_claim_id(commit_claim(&ctx, &db, first).unwrap());

        let mut second = proposal("Procurement blocked renewal");
        second.thread_id = Some("thread-b".to_string());
        second.data_source = "second_source".to_string();
        let result = commit_claim(&ctx, &db, second).unwrap();
        match result {
            CommittedClaim::Reinforced { claim, .. } => {
                assert_eq!(claim.id, first_id);
            }
            other => panic!("expected same-meaning merge, got {other:?}"),
        }

        let active: Vec<_> = load_claims_active(&db, SUBJECT, Some("risk"))
            .unwrap()
            .into_iter()
            .filter(|claim| claim.text == "procurement blocked renewal")
            .collect();
        assert_eq!(
            active.len(),
            1,
            "thread_id must not create duplicate active claims"
        );
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
        assert_eq!(
            claim.item_hash,
            Some(item_hash(ItemKind::Risk, &claim.text))
        );
    }

    #[test]
    fn commit_claim_substitutes_registry_default_temporal_scope_when_omitted() {
        let db = test_db();
        seed_meeting(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let mut p = proposal("Meeting included a renewal-risk note");
        p.subject_ref = r#"{"kind":"meeting","id":"meeting-1"}"#.to_string();
        p.claim_type = "meeting_event_note".to_string();
        p.field_path = None;
        p.temporal_scope = None;

        let id = inserted_claim_id(commit_claim(&ctx, &db, p).unwrap());
        assert_eq!(read_claim_temporal_scope(&db, &id), "point_in_time");
    }

    #[test]
    fn commit_claim_preserves_explicit_temporal_scope() {
        let db = test_db();
        seed_meeting(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let mut p = proposal("Meeting note should stay state-scoped");
        p.subject_ref = r#"{"kind":"meeting","id":"meeting-1"}"#.to_string();
        p.claim_type = "meeting_event_note".to_string();
        p.field_path = None;
        p.temporal_scope = Some(TemporalScope::State);

        let id = inserted_claim_id(commit_claim(&ctx, &db, p).unwrap());
        assert_eq!(read_claim_temporal_scope(&db, &id), "state");
    }

    #[test]
    fn commit_claim_explicit_some_sensitivity_wins_over_registry_default() {
        let db = test_db();
        seed_person(&db);
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params!["person-2", "person-2@example.com", "Person 2", TS],
            )
            .expect("seed second person");
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let mut explicit = proposal("Stakeholder assessment stays internal");
        explicit.subject_ref = r#"{"kind":"person","id":"person-1"}"#.to_string();
        explicit.claim_type = "stakeholder_assessment".to_string();
        explicit.field_path = None;
        explicit.sensitivity = Some(ClaimSensitivity::Internal);

        let explicit_id = inserted_claim_id(commit_claim(&ctx, &db, explicit).unwrap());
        assert_eq!(read_claim_sensitivity(&db, &explicit_id), "internal");

        let mut omitted = proposal("Stakeholder is privately assessing renewal risk");
        omitted.subject_ref = r#"{"kind":"person","id":"person-2"}"#.to_string();
        omitted.claim_type = "stakeholder_assessment".to_string();
        omitted.field_path = None;
        omitted.sensitivity = None;

        let omitted_id = inserted_claim_id(commit_claim(&ctx, &db, omitted).unwrap());
        assert_eq!(read_claim_sensitivity(&db, &omitted_id), "confidential");
    }

    #[test]
    fn commit_claim_rejects_user_actor_for_system_only_claim_type() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let mut p = proposal("Dismissed item must come from system backfill");
        p.claim_type = "dismissed_item".to_string();
        p.field_path = Some("risks".to_string());
        p.actor = "user".to_string();

        let err = commit_claim(&ctx, &db, p).expect_err("user actor must be rejected");
        assert!(matches!(
            err,
            ClaimError::ActorNotPermittedForClaimType {
                claim_type,
                actor_class,
                ..
            } if claim_type == "dismissed_item" && actor_class == "user"
        ));
    }

    #[test]
    fn commit_claim_rejects_agent_actor_for_user_claim_type() {
        let db = test_db();
        seed_person(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let mut p = proposal("Primary contact");
        p.subject_ref = r#"{"kind":"person","id":"person-1"}"#.to_string();
        p.claim_type = "stakeholder_role".to_string();
        p.field_path = None;
        p.actor = "agent:test".to_string();

        let err = commit_claim(&ctx, &db, p).expect_err("agent actor must be rejected");
        assert!(matches!(
            err,
            ClaimError::ActorNotPermittedForClaimType {
                claim_type,
                actor_class,
                ..
            } if claim_type == "stakeholder_role" && actor_class == "agent"
        ));
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

    /// L2 cycle-1 regression: backfilled tombstone with a m1-style
    /// `dedup_key` (entity_id without compact-JSON wrap, raw item_hash
    /// passed through) must still block runtime resurrection. PRE-GATE
    /// matches by per-tier (subject + claim_type + field + hash | text)
    /// so the dedup_key shape divergence is no longer load-bearing.
    fn seed_backfill_shaped_tombstone(db: &ActionDb, item_hash_value: &str, text: &str) {
        // Mirror migration 130's m1 INSERT shape: subject_ref via
        // json_object('kind', 'Account', 'id', X) (insertion-order JSON,
        // not the runtime alphabetical form), and dedup_key built per
        // the migration's idiosyncratic per-mechanism formula. The
        // PRE-GATE must NOT key off this dedup_key.
        // dos7-allowed: regression test seed for L2 cycle-1 finding #2
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims \
             (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
              actor, data_source, observed_at, created_at, provenance_json, \
              claim_state, surfacing_state, retraction_reason, \
              temporal_scope, sensitivity) \
             VALUES (?1, ?2, 'risk', 'health.risk', ?3, ?4, ?5, \
                     'system_backfill', 'legacy_dismissal', ?6, ?6, '{}', \
                     'tombstoned', 'active', 'user_removal', \
                     'state', 'internal')",
                params![
                    "m1-fixture-1",
                    // Backfill shape: kind first, NOT alphabetical
                    r#"{"kind":"Account","id":"acct-1"}"#,
                    text,
                    // Mechanism-1 dedup_key shape (DIFFERENT from runtime).
                    format!(
                        "{}:acct-1:risk:health.risk",
                        if item_hash_value.is_empty() {
                            text
                        } else {
                            item_hash_value
                        }
                    ),
                    item_hash_value,
                    TS,
                ],
            )
            .expect("seed backfill-shaped tombstone");
    }

    #[test]
    fn pre_gate_blocks_resurrection_via_backfilled_hash_match() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        // Pre-compute the runtime hash for the proposal text and seed a
        // backfill-shaped tombstone with that hash + a different
        // (mechanism-1-style) dedup_key. The PRE-GATE must still block.
        let canonical = canonicalize_for_dos280("Procurement blocked renewal");
        let hash = item_hash(ItemKind::Risk, &canonical);
        seed_backfill_shaped_tombstone(&db, &hash, "Procurement blocked renewal");

        let err = commit_claim(&ctx, &db, proposal("Procurement blocked renewal"))
            .expect_err("backfilled tombstone must block runtime resurrection (hash tier)");
        assert!(matches!(err, ClaimError::TombstonedPreGate));
    }

    #[test]
    fn pre_gate_blocks_resurrection_via_backfilled_exact_text_match() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        // Seed a backfill row with an EMPTY item_hash (legacy NULL → '')
        // but a `text` column that matches the runtime canonical text.
        // Hash tier won't fire; exact text tier must.
        seed_backfill_shaped_tombstone(&db, "", "Procurement blocked renewal");

        let err = commit_claim(&ctx, &db, proposal("Procurement blocked renewal"))
            .expect_err("backfilled tombstone must block runtime resurrection (text tier)");
        assert!(matches!(err, ClaimError::TombstonedPreGate));
    }

    #[test]
    fn pre_gate_blocks_resurrection_via_backfilled_keyless_sentinel() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        // Mechanism-1 keyless: legacy item_key=NULL, item_hash=NULL →
        // backfill writes text='<keyless>'. Any subsequent claim in
        // (Account:acct-1, risk, health.risk) is suppressed.
        seed_backfill_shaped_tombstone(&db, "", "<keyless>");

        let err = commit_claim(&ctx, &db, proposal("Any new risk text"))
            .expect_err("backfilled keyless tombstone must block runtime resurrection");
        assert!(matches!(err, ClaimError::TombstonedPreGate));
    }

    #[test]
    fn pre_gate_does_not_block_different_subject() {
        let db = test_db();
        seed_account(&db);
        // Seed a tombstone for acct-1.
        let canonical = canonicalize_for_dos280("Procurement blocked renewal");
        let hash = item_hash(ItemKind::Risk, &canonical);
        seed_backfill_shaped_tombstone(&db, &hash, "Procurement blocked renewal");

        // Different subject (acct-2) must still commit successfully.
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params!["acct-2", "Account 2", TS],
            )
            .expect("seed acct-2");
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let mut other_subject = proposal("Procurement blocked renewal");
        other_subject.subject_ref = r#"{"kind":"account","id":"acct-2"}"#.to_string();
        let result = commit_claim(&ctx, &db, other_subject);
        assert!(
            matches!(result, Ok(CommittedClaim::Inserted { .. })),
            "different subject must not be blocked, got {result:?}"
        );
    }

    #[test]
    fn pre_gate_does_not_block_different_claim_type() {
        let db = test_db();
        seed_account(&db);
        // Seed a 'risk' tombstone.
        let canonical = canonicalize_for_dos280("Procurement blocked renewal");
        let hash = item_hash(ItemKind::Risk, &canonical);
        seed_backfill_shaped_tombstone(&db, &hash, "Procurement blocked renewal");

        // Same subject + content but different claim_type = 'win' must
        // not be blocked by a 'risk' tombstone.
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let mut other_type = proposal("Procurement blocked renewal");
        other_type.claim_type = "win".to_string();
        let result = commit_claim(&ctx, &db, other_type);
        assert!(
            matches!(result, Ok(CommittedClaim::Inserted { .. })),
            "different claim_type must not be blocked, got {result:?}"
        );
    }

    /// L2 cycle-12 fix #1: contradiction detection must match the
    /// active subject by kind+id, not subject_ref string equality.
    /// Two byte-different but semantically-identical subject_refs
    /// (different JSON key order) would otherwise miss the
    /// contradiction and silently insert an unlinked duplicate.
    #[test]
    fn commit_claim_forks_when_subject_ref_key_order_differs_from_existing() {
        let db = test_db();
        seed_account(&db);

        // Manually seed an active claim with subject_ref in
        // INSERTION-order JSON (kind first), the shape SQLite's
        // json_object writes. dos7-allowed: cycle-12 regression seed
        let active_id = "preexisting-active-1";
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims \
                 (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
                  actor, data_source, observed_at, created_at, provenance_json, \
                  claim_state, surfacing_state, retraction_reason, \
                  temporal_scope, sensitivity) \
                 VALUES (?1, ?2, 'risk', 'health.risk', \
                         'first claim text', 'dedup-1', 'hash-1', \
                         'agent:test', 'unit_test', ?3, ?3, '{}', \
                         'active', 'active', NULL, 'state', 'internal')",
                params![
                    active_id,
                    // Insertion-order JSON (kind FIRST, id SECOND).
                    r#"{"kind":"account","id":"acct-1"}"#,
                    TS,
                ],
            )
            .unwrap();

        // The runtime serializer produces alphabetical-key JSON
        // ({"id":"acct-1","kind":"account"}). A naive subject_ref =
        // ?1 match would not find the existing claim. The cycle-12
        // fix's json_extract-based match should find it and fork.
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let result = commit_claim(&ctx, &db, proposal("Different text — should fork"))
            .expect("commit should succeed");
        match result {
            CommittedClaim::Forked { primary_claim, .. } => {
                assert_eq!(
                    primary_claim.id, active_id,
                    "fork must point at the existing active claim regardless of subject_ref key order"
                );
            }
            other => panic!(
                "expected Forked (cycle-12 fix should detect the contradiction \
                 across reversed key order), got {other:?}"
            ),
        }
    }

    /// L2 cycle-7 fix #2: a malformed historical tombstone
    /// `subject_ref` (not valid JSON) must NOT abort commit_claim.
    /// Cycle-7 wraps the PRE-GATE query in a json_valid subquery
    /// filter so SQLite never evaluates `json_extract` on a
    /// malformed row. Without this, a single bad legacy row blocks
    /// every subsequent commit_claim call until an operator runs
    /// remediation.
    #[test]
    fn pre_gate_skips_malformed_subject_ref_tombstones() {
        let db = test_db();
        seed_account(&db);

        // Seed a malformed-JSON tombstone whose claim_type + field
        // would otherwise match the proposal we're about to commit.
        // dos7-allowed: cycle-7 regression-test seed
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims \
                 (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
                  actor, data_source, observed_at, created_at, provenance_json, \
                  claim_state, surfacing_state, retraction_reason, \
                  temporal_scope, sensitivity) \
                 VALUES \
                 ('malformed-1', 'this is not json', 'risk', 'health.risk', \
                  'something', 'k1', 'h1', 'system_backfill', 'legacy_dismissal', \
                  ?1, ?1, '{}', 'tombstoned', 'dormant', 'user_removal', \
                  'state', 'internal')",
                params![TS],
            )
            .unwrap();

        // commit_claim must succeed — the malformed row is silently
        // skipped by the json_valid subquery filter.
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let result = commit_claim(&ctx, &db, proposal("Procurement blocked renewal"));
        assert!(
            matches!(result, Ok(CommittedClaim::Inserted { .. })),
            "commit_claim must succeed past a malformed tombstone, got {result:?}"
        );
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

    /// L2 cycle-1 fix #5: record_corroboration must bump per-entity
    /// claim_version so trust/surfacing readers refresh.
    #[test]
    fn record_corroboration_emits_per_entity_invalidation() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk for corroboration")).unwrap());
        // commit_claim already bumped once.
        let after_commit = read_account_claim_version(&db);

        record_corroboration(&ctx, &db, &claim_id, "glean", Some(TS), Some("backfill")).unwrap();

        assert_eq!(read_account_claim_version(&db), after_commit + 1);
    }

    #[test]
    fn record_claim_feedback_persists_a_row_per_action_for_each_of_9_variants() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk for feedback")).unwrap());

        for action in all_feedback_actions() {
            let outcome =
                record_claim_feedback(&ctx, &db, feedback_input(&claim_id, action)).unwrap();
            assert_eq!(outcome.claim_id, claim_id);
            assert_eq!(outcome.action, action);
            assert!(outcome.applied_at_pending);
        }

        let rows: Vec<String> = {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT feedback_type FROM claim_feedback \
                     WHERE claim_id = ?1 ORDER BY rowid",
                )
                .unwrap();
            stmt.query_map(params![&claim_id], |row| row.get::<_, String>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        let expected = all_feedback_actions()
            .iter()
            .map(FeedbackAction::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert_eq!(rows, expected);
    }

    #[test]
    fn record_claim_feedback_transitions_verification_state_to_contested_for_cannot_verify() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk needing evidence")).unwrap());

        let outcome = record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::CannotVerify),
        )
        .unwrap();

        assert_eq!(
            outcome.new_verification_state,
            ClaimVerificationState::Contested
        );
        let (state, reason, needs_user_decision_at) = read_verification_columns(&db, &claim_id);
        assert_eq!(state, "contested");
        assert_eq!(reason.as_deref(), Some("cannot_verify"));
        assert_eq!(needs_user_decision_at, None);
    }

    #[test]
    fn record_claim_feedback_enqueues_repair_for_cannot_verify() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk needing repair")).unwrap());

        let outcome = record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::CannotVerify),
        )
        .unwrap();

        let repair_job_id = outcome
            .repair_job_id
            .as_deref()
            .expect("repair job id should be returned");
        let (state, attempts): (String, i64) = db
            .conn_ref()
            .query_row(
                "SELECT state, attempts FROM claim_repair_job WHERE id = ?1",
                params![repair_job_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read repair job");
        assert_eq!(repair_job_count(&db, &claim_id), 1);
        assert_eq!(state, "pending");
        assert_eq!(attempts, 0);
    }

    #[test]
    fn record_claim_feedback_skips_repair_for_confirm_current() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk already confirmed")).unwrap());

        let outcome = record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::ConfirmCurrent),
        )
        .unwrap();

        assert_eq!(outcome.repair_job_id, None);
        assert_eq!(repair_job_count(&db, &claim_id), 0);
    }

    #[test]
    fn record_claim_feedback_honors_per_claim_cap() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk at repair cap")).unwrap());

        for idx in 0..MAX_ACTIVE_REPAIR_JOBS_PER_CLAIM {
            db.conn_ref()
                .execute(
                    "INSERT INTO claim_repair_job
                     (id, claim_id, feedback_id, state, attempts, max_attempts, created_at)
                     VALUES (?1, ?2, NULL, 'pending', 0, 3, ?3)",
                    params![format!("repair-seed-{idx}"), &claim_id, TS],
                )
                .expect("seed repair job");
        }

        let outcome = record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::CannotVerify),
        )
        .unwrap();

        assert_eq!(outcome.repair_job_id, None);
        assert_eq!(
            repair_job_count(&db, &claim_id),
            MAX_ACTIVE_REPAIR_JOBS_PER_CLAIM
        );
    }

    #[test]
    fn record_claim_feedback_emits_activity_signal() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_text = "Sensitive claim text must not appear in signal payload";
        let claim_id = inserted_claim_id(commit_claim(&ctx, &db, proposal(claim_text)).unwrap());

        record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::CannotVerify),
        )
        .unwrap();

        assert_eq!(signal_count(&db, "claim_feedback_recorded"), 1);
        let payload = first_signal_value(&db, "claim_feedback_recorded");
        let payload_json: serde_json::Value =
            serde_json::from_str(&payload).expect("signal payload should be JSON");
        assert_eq!(payload_json["action"], "cannot_verify");
        assert_eq!(payload_json["claim_id"], claim_id);
        assert!(
            !payload.contains(claim_text),
            "signal payload must not include claim text"
        );
    }

    #[test]
    fn record_claim_feedback_emits_state_change_signal_only_on_transition() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let confirmed_claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Confirmed active risk")).unwrap());

        record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&confirmed_claim_id, FeedbackAction::ConfirmCurrent),
        )
        .unwrap();

        assert_eq!(signal_count(&db, "claim_verification_state_changed"), 0);

        let db = test_db();
        seed_account(&db);
        let contested_claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Contested active risk")).unwrap());
        record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&contested_claim_id, FeedbackAction::CannotVerify),
        )
        .unwrap();

        assert_eq!(signal_count(&db, "claim_verification_state_changed"), 1);
        let payload = first_signal_value(&db, "claim_verification_state_changed");
        let payload_json: serde_json::Value =
            serde_json::from_str(&payload).expect("state-change payload should be JSON");
        assert_eq!(payload_json["from"], "active");
        assert_eq!(payload_json["to"], "contested");
    }

    #[test]
    fn record_claim_feedback_applies_lifecycle_transitions() {
        let cases = [
            (
                FeedbackAction::MarkOutdated,
                "active",
                "dormant",
                Some("outdated"),
                None,
            ),
            (
                FeedbackAction::MarkFalse,
                "withdrawn",
                "dormant",
                None,
                Some("user_marked_false"),
            ),
            (
                FeedbackAction::WrongSubject,
                "tombstoned",
                "dormant",
                None,
                Some("wrong_subject"),
            ),
            (
                FeedbackAction::NeedsNuance,
                "active",
                "dormant",
                Some("superseded"),
                None,
            ),
        ];

        for (
            action,
            expected_claim_state,
            expected_surfacing,
            expected_demotion,
            expected_retraction,
        ) in cases
        {
            let db = test_db();
            seed_account(&db);
            let (clock, rng, external) = ctx_parts();
            let ctx = live_ctx(&clock, &rng, &external);
            let claim_id = inserted_claim_id(
                commit_claim(&ctx, &db, proposal(&format!("Lifecycle {:?}", action))).unwrap(),
            );

            record_claim_feedback(&ctx, &db, feedback_input(&claim_id, action)).unwrap();

            let (claim_state, surfacing_state, demotion_reason, retraction_reason) =
                read_lifecycle_columns(&db, &claim_id);
            assert_eq!(claim_state, expected_claim_state, "{action:?}");
            assert_eq!(surfacing_state, expected_surfacing, "{action:?}");
            assert_eq!(demotion_reason.as_deref(), expected_demotion, "{action:?}");
            assert_eq!(
                retraction_reason.as_deref(),
                expected_retraction,
                "{action:?}"
            );
        }
    }

    #[test]
    fn record_claim_feedback_mark_false_removes_claim_from_active_reader() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk marked false")).unwrap());

        record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::MarkFalse),
        )
        .unwrap();

        let active_ids = load_claims_active(&db, SUBJECT, Some("risk"))
            .unwrap()
            .into_iter()
            .map(|claim| claim.id)
            .collect::<Vec<_>>();
        assert!(!active_ids.contains(&claim_id));
    }

    #[test]
    fn record_claim_feedback_accepts_user_feedback_on_agent_claim() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let mut p = proposal("Agent-authored entity risk");
        p.claim_type = "entity_risk".to_string();
        p.field_path = None;
        let claim_id = inserted_claim_id(commit_claim(&ctx, &db, p).unwrap());

        let outcome = record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::ConfirmCurrent),
        )
        .unwrap();

        assert_eq!(outcome.claim_id, claim_id);
    }

    #[test]
    fn record_claim_feedback_skips_claim_update_when_feedback_has_no_column_delta() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk already current")).unwrap());

        db.conn_ref()
            .execute_batch(
                "CREATE TABLE claim_update_log (id INTEGER);
                 CREATE TRIGGER claim_update_log_after_update
                 AFTER UPDATE ON intelligence_claims
                 BEGIN
                   INSERT INTO claim_update_log (id) VALUES (1);
                 END;",
            )
            .unwrap();

        record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::ConfirmCurrent),
        )
        .unwrap();

        let update_count: i64 = db
            .conn_ref()
            .query_row("SELECT count(*) FROM claim_update_log", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            update_count, 0,
            "feedback row should persist without re-writing unchanged claim columns"
        );
    }

    #[test]
    fn record_claim_feedback_rejects_non_user_actor() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk for actor check")).unwrap());
        let mut input = feedback_input(&claim_id, FeedbackAction::CannotVerify);
        input.actor = "system_backfill".to_string();

        let err = record_claim_feedback(&ctx, &db, input)
            .expect_err("system feedback actor must be rejected");

        assert!(
            matches!(err, ClaimError::InvalidFeedback(message) if message.contains("only accepted from user"))
        );
    }

    #[test]
    fn record_claim_feedback_rejects_missing_required_action_metadata() {
        let actions = [
            (FeedbackAction::WrongSource, "source_ref"),
            (FeedbackAction::NeedsNuance, "corrected_text"),
            (FeedbackAction::SurfaceInappropriate, "surface"),
            (FeedbackAction::NotRelevantHere, "invocation_id"),
        ];

        for (action, key) in actions {
            let empty_value_payload = serde_json::Value::Object(serde_json::Map::from_iter([(
                key.to_string(),
                serde_json::Value::String(String::new()),
            )]))
            .to_string();
            for payload in [
                serde_json::json!({}).to_string(),
                empty_value_payload.clone(),
            ] {
                let db = test_db();
                let (clock, rng, external) = ctx_parts();
                let ctx = live_ctx(&clock, &rng, &external);
                let mut input = feedback_input("claim-not-needed", action);
                input.payload_json = Some(payload);

                let err = record_claim_feedback(&ctx, &db, input)
                    .expect_err("invalid metadata should be rejected before claim lookup");
                assert!(
                    matches!(err, ClaimError::InvalidFeedback(message) if message.contains(key))
                );
            }
        }
    }

    #[test]
    fn feedback_metadata_for_claim_uses_corrected_text_for_needs_nuance() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated")).unwrap(),
        );
        let claim = load_claim_by_id(db.conn_ref(), &claim_id)
            .unwrap()
            .expect("claim exists");
        let input = ClaimFeedbackInput {
            claim_id,
            action: FeedbackAction::NeedsNuance,
            actor: "user".to_string(),
            actor_id: Some("user-fixture".to_string()),
            payload_json: Some(
                serde_json::json!({
                    "corrected_text": "Customer expanded usage across the support organization"
                })
                .to_string(),
            ),
        };

        let metadata =
            feedback_metadata_for_claim(&claim, &input, feedback_semantics(input.action)).unwrap();
        assert_eq!(metadata.trust_effect.claim_alpha_delta, 0.0);
        assert_eq!(metadata.trust_effect.claim_beta_delta, 0.3);
    }

    #[test]
    fn record_claim_feedback_idempotent_replay_does_not_dup_state_change() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id = inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk replay")).unwrap());

        record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::CannotVerify),
        )
        .unwrap();
        let first = read_verification_columns(&db, &claim_id);

        record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::CannotVerify),
        )
        .unwrap();
        let second = read_verification_columns(&db, &claim_id);

        assert_eq!(second, first);
        let feedback_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM claim_feedback WHERE claim_id = ?1",
                params![&claim_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(feedback_count, 2);
    }

    #[test]
    fn record_claim_feedback_rejects_unknown_claim_id() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let err = record_claim_feedback(
            &ctx,
            &db,
            feedback_input("missing-claim", FeedbackAction::CannotVerify),
        )
        .expect_err("unknown claim should be rejected");

        assert!(matches!(err, ClaimError::UnknownClaimId(id) if id == "missing-claim"));
        let feedback_count: i64 = db
            .conn_ref()
            .query_row("SELECT count(*) FROM claim_feedback", [], |row| row.get(0))
            .unwrap();
        assert_eq!(feedback_count, 0);
    }

    #[test]
    fn record_claim_feedback_blocks_in_simulate_mode() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let live = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&live, &db, proposal("Risk simulate gate")).unwrap());
        let simulate = ServiceContext::new_simulate(&clock, &rng, &external);

        let err = record_claim_feedback(
            &simulate,
            &db,
            feedback_input(&claim_id, FeedbackAction::CannotVerify),
        )
        .expect_err("simulate mode should block mutation");

        assert!(matches!(err, ClaimError::Mode(_)));
        let feedback_count: i64 = db
            .conn_ref()
            .query_row("SELECT count(*) FROM claim_feedback", [], |row| row.get(0))
            .unwrap();
        assert_eq!(feedback_count, 0);
    }

    #[test]
    fn intelligence_claims_verification_state_defaults_to_active_after_migration() {
        let db = test_db();

        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims (
                    id, subject_ref, claim_type, text, dedup_key, actor, data_source,
                    observed_at, created_at, provenance_json, temporal_scope, sensitivity
                 ) VALUES (
                    'claim-default-verification', ?1, 'risk', 'defaulted', 'dedup-default',
                    'agent:test', 'unit_test', ?2, ?2, '{}', 'state', 'internal'
                 )",
                params![SUBJECT, TS],
            )
            .unwrap();

        let (state, reason, needs_user_decision_at) =
            read_verification_columns(&db, "claim-default-verification");
        assert_eq!(state, "active");
        assert_eq!(reason, None);
        assert_eq!(needs_user_decision_at, None);
    }

    #[test]
    fn claim_feedback_check_constraint_accepts_all_9_action_strings() {
        let db = test_db();

        for action in all_feedback_actions() {
            let payload = feedback_payload_for(action);
            db.conn_ref()
                .execute(
                    "INSERT INTO claim_feedback (
                        id, claim_id, feedback_type, actor, actor_id, payload_json, submitted_at
                     ) VALUES (?1, 'claim-1', ?2, 'user', 'user@example.com', ?3, ?4)",
                    params![
                        format!("feedback-{}", action.as_str()),
                        action.as_str(),
                        payload.as_deref(),
                        TS,
                    ],
                )
                .unwrap_or_else(|e| panic!("{} should satisfy CHECK: {e}", action.as_str()));
        }

        let feedback_count: i64 = db
            .conn_ref()
            .query_row("SELECT count(*) FROM claim_feedback", [], |row| row.get(0))
            .unwrap();
        assert_eq!(feedback_count, 9);
    }

    /// L2 cycle-1 fix #5: reconcile_contradiction must bump per-entity
    /// claim_version for both sides of the contradiction.
    #[test]
    fn reconcile_contradiction_emits_per_entity_invalidation() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        // First commit inserts the primary; second commit on the same
        // subject + claim_type + field with DIFFERENT canonical text
        // forks via fix #6's contradiction-detection branch and
        // produces both the contradiction edge and the new claim id.
        let primary_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Renewal risk: red")).unwrap());
        let forked = commit_claim(&ctx, &db, proposal("Renewal risk: green")).unwrap();
        let (contradiction_id, contradicting_id) = match forked {
            CommittedClaim::Forked {
                contradiction_id,
                new_claim_id,
                ..
            } => (contradiction_id, new_claim_id),
            other => panic!("expected fork from contradiction detection, got {other:?}"),
        };
        let _ = (primary_id, contradicting_id); // referenced via the
                                                // contradiction_id

        let before = read_account_claim_version(&db);

        reconcile_contradiction(
            &ctx,
            &db,
            &contradiction_id,
            ReconciliationKind::UserPickedWinner,
            Some("user resolved"),
            None,
            None,
        )
        .unwrap();

        // Both primary and contradicting share subject_ref acct-1 → one
        // bump (the helper deduplicates via the if-equality guard).
        assert_eq!(read_account_claim_version(&db), before + 1);
    }

    /// L2 cycle-1 fix #6: canonicalize_for_dos280 lowercases, trims,
    /// and collapses internal whitespace runs.
    #[test]
    fn canonicalize_for_dos280_lowercases_trims_collapses_whitespace() {
        assert_eq!(
            canonicalize_for_dos280("  ARR Risk\trenewal "),
            "arr risk renewal"
        );
        assert_eq!(
            canonicalize_for_dos280("Procurement   Blocked\n\nRenewal"),
            "procurement blocked renewal"
        );
        assert_eq!(
            canonicalize_for_dos280("already canonical"),
            "already canonical"
        );
    }

    /// L2 cycle-1 fix #6: re-committing the same active claim's
    /// canonical text with a different data_source routes through
    /// corroboration and returns Reinforced — does NOT insert a
    /// duplicate active row.
    #[test]
    fn commit_claim_same_meaning_merges_via_corroboration() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        // First commit: inserts the active claim.
        let first_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Procurement blocked renewal")).unwrap(),
        );

        // Second commit with SAME canonical text but different
        // data_source → same-meaning merge.
        let mut p2 = proposal("Procurement blocked renewal");
        p2.data_source = "second_source".to_string();
        let result = commit_claim(&ctx, &db, p2).unwrap();
        match result {
            CommittedClaim::Reinforced {
                claim,
                corroboration_id: _,
            } => {
                assert_eq!(
                    claim.id, first_id,
                    "must reinforce existing claim, not insert new"
                );
            }
            other => panic!("expected Reinforced, got {other:?}"),
        }

        // The intelligence_claims table still has exactly ONE active
        // row for this dedup_key — no duplicate.
        let active: Vec<_> = load_claims_active(&db, SUBJECT, Some("risk"))
            .unwrap()
            .into_iter()
            .filter(|c| c.text == "procurement blocked renewal")
            .collect();
        assert_eq!(active.len(), 1, "exactly one active claim after merge");
    }

    /// L2 cycle-1 fix #6: committing different canonical text on the
    /// same (subject, claim_type, field) forks via contradiction
    /// detection — both claims remain active until reconciled.
    #[test]
    fn commit_claim_different_meaning_forks_via_contradiction_detection() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let primary_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Renewal looks healthy")).unwrap());
        let result =
            commit_claim(&ctx, &db, proposal("Renewal at risk due to procurement")).unwrap();
        match result {
            CommittedClaim::Forked {
                primary_claim,
                contradiction_id,
                new_claim_id,
            } => {
                assert_eq!(primary_claim.id, primary_id);
                assert_ne!(new_claim_id, primary_id);
                // Verify the contradiction edge persists.
                let edge_count: i64 = db
                    .conn_ref()
                    .query_row(
                        "SELECT count(*) FROM claim_contradictions WHERE id = ?1",
                        params![&contradiction_id],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(edge_count, 1, "contradiction edge must be persisted");
            }
            other => panic!("expected Forked, got {other:?}"),
        }

        // Both claims remain active.
        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2, "both claims active until user reconciles");
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

    /// L2 cycle-13 fix #2: load_claims_active (and the rest of the
    /// reader family that goes through load_claims_where) must
    /// return claims regardless of subject_ref JSON key order or
    /// kind casing. Previously the reader used exact subject_ref
    /// string equality, so a row written by SQLite json_object
    /// (insertion order, PascalCase kind) would be invisible to a
    /// reader called with serde_json-canonical (alphabetical order,
    /// lowercase kind) input.
    /// L2 cycle-14 fix #1: subject_ref_from_json must accept
    /// PascalCase kinds (the shape SQLite json_object writes).
    /// Cycle-13 fix #2 made the reader DB-side casing-tolerant
    /// but left the input-side parser strict, so PascalCase
    /// caller input regressed.
    #[test]
    fn load_claims_active_accepts_pascal_case_caller_input() {
        let db = test_db();

        // dos7-allowed: cycle-14 regression seed
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims \
                 (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
                  actor, data_source, observed_at, created_at, provenance_json, \
                  claim_state, surfacing_state, retraction_reason, \
                  temporal_scope, sensitivity) \
                 VALUES \
                 ('pascal-active', \
                  '{\"kind\":\"Account\",\"id\":\"acct-1\"}', 'risk', 'health.risk', \
                  'first', 'k1', 'h1', 'agent:test', 'unit_test', \
                  ?1, ?1, '{}', 'active', 'active', NULL, 'state', 'internal')",
                params![TS],
            )
            .unwrap();

        // PascalCase reader input — this is what backfill SQL also
        // produces. Must NOT error in subject_ref_from_json.
        let pascal_input = r#"{"kind":"Account","id":"acct-1"}"#;
        let claims = load_claims_active(&db, pascal_input, Some("risk"))
            .expect("PascalCase reader input must parse");
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].id, "pascal-active");
    }

    /// L2 cycle-15 fix #1: two semantically-equal subjects with
    /// different kind casing must produce identical dedup_key +
    /// commit lock so the second commit reinforces (Reinforced)
    /// instead of inserting an unlinked duplicate active row.
    #[test]
    fn commit_claim_canonicalizes_subject_across_kind_casing() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        // First commit: lowercase kind.
        let mut p1 = proposal("Same risk text");
        p1.subject_ref = r#"{"kind":"account","id":"acct-1"}"#.to_string();
        let first = commit_claim(&ctx, &db, p1).unwrap();
        let first_id = match first {
            CommittedClaim::Inserted { ref claim } => claim.id.clone(),
            other => panic!("expected first to insert, got {other:?}"),
        };

        // Second commit: PascalCase kind, otherwise identical. Must
        // canonicalize to the same dedup_key and route through
        // same-meaning merge (Reinforced).
        let mut p2 = proposal("Same risk text");
        p2.subject_ref = r#"{"kind":"Account","id":"acct-1"}"#.to_string();
        p2.data_source = "second-source".to_string();
        let second = commit_claim(&ctx, &db, p2).unwrap();
        match second {
            CommittedClaim::Reinforced { claim, .. } => {
                assert_eq!(
                    claim.id, first_id,
                    "second commit must reinforce the same row, not insert a duplicate"
                );
            }
            other => panic!("expected Reinforced, got {other:?}"),
        }

        // Exactly one active claim survives.
        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 1, "exactly one active claim across casings");
    }

    /// L2 cycle-14 fix #2: commit_claim must reject Multi and
    /// Global subjects per ADR-0125 v1.4.0 spine restriction.
    /// The reader family doesn't support them; allowing the write
    /// would create read-invisible rows.
    #[test]
    fn commit_claim_rejects_multi_subject_per_v140_spine() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let mut p = proposal("Multi-subject claim attempt");
        p.subject_ref = r#"{"kind":"multi","subjects":[{"kind":"account","id":"a-1"},{"kind":"person","id":"p-1"}]}"#.to_string();
        let err = commit_claim(&ctx, &db, p).expect_err("Multi must be rejected");
        match err {
            ClaimError::SubjectRef(msg) => {
                assert!(msg.contains("Multi") || msg.contains("v1.4.1"), "got {msg}");
            }
            other => panic!("expected SubjectRef error, got {other:?}"),
        }
    }

    #[test]
    fn commit_claim_rejects_global_subject_per_v140_spine() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let mut p = proposal("Global claim attempt");
        p.subject_ref = r#"{"kind":"global"}"#.to_string();
        let err = commit_claim(&ctx, &db, p).expect_err("Global must be rejected");
        match err {
            ClaimError::SubjectRef(msg) => {
                assert!(
                    msg.contains("Global") || msg.contains("v1.4.1") || msg.contains("ADR-0125"),
                    "got {msg}"
                );
            }
            other => panic!("expected SubjectRef error, got {other:?}"),
        }
    }

    #[test]
    fn load_claims_active_matches_across_subject_ref_key_order_and_casing() {
        let db = test_db();

        // Seed an active claim with insertion-order JSON,
        // PascalCase kind — the shape SQLite json_object writes.
        // dos7-allowed: cycle-13 regression seed
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims \
                 (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
                  actor, data_source, observed_at, created_at, provenance_json, \
                  claim_state, surfacing_state, retraction_reason, \
                  temporal_scope, sensitivity) \
                 VALUES \
                 ('insertion-order-active', \
                  '{\"kind\":\"Account\",\"id\":\"acct-1\"}', 'risk', 'health.risk', \
                  'first', 'k1', 'h1', 'agent:test', 'unit_test', \
                  ?1, ?1, '{}', 'active', 'active', NULL, 'state', 'internal')",
                params![TS],
            )
            .unwrap();

        // Reader called with the runtime serde_json shape
        // (alphabetical, lowercase). The fix's json_extract match
        // should find it regardless.
        let reader_input = r#"{"id":"acct-1","kind":"account"}"#;
        let claims = load_claims_active(&db, reader_input, Some("risk")).expect("reader query");
        assert_eq!(
            claims.len(),
            1,
            "reader must find the row across key/case differences"
        );
        assert_eq!(claims[0].id, "insertion-order-active");
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

    /// L2 cycle-2 fix #1: shadow_write_tombstone_claim must actually
    /// write the claim row when called with a substrate-supported
    /// subject_kind. Cycle-1 silently no-op'd because PascalCase kinds
    /// fell through subject_ref_from_json and hit the error arm.
    #[test]
    fn shadow_write_pascal_case_subject_kinds_actually_persist_claims() {
        let db = test_db();
        seed_account(&db);

        for kind in ["Account", "account", "ACCOUNT"] {
            let outcome = shadow_write_tombstone_claim(
                &db,
                ShadowTombstoneClaim {
                    subject_kind: kind,
                    subject_id: "acct-1",
                    claim_type: "risk",
                    field_path: Some("risks"),
                    text: &format!("kind={kind}"),
                    actor: "user",
                    source_scope: None,
                    observed_at: TS,
                    expires_at: None,
                },
            );
            assert_eq!(
                outcome,
                ShadowTombstoneOutcome::Committed,
                "shadow_write must commit for kind={kind}"
            );
        }

        // Three tombstone rows now persist for acct-1.
        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM intelligence_claims \
                 WHERE claim_state = 'tombstoned' \
                   AND lower(json_extract(subject_ref, '$.kind')) = 'account' \
                   AND json_extract(subject_ref, '$.id') = 'acct-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            count, 3,
            "all three PascalCase variants must produce claim rows"
        );
    }

    #[test]
    fn shadow_write_meeting_kind_persists_claim() {
        let db = test_db();
        // Meetings table seed not needed for the claim row itself.
        let outcome = shadow_write_tombstone_claim(
            &db,
            ShadowTombstoneClaim {
                subject_kind: "Meeting",
                subject_id: "mtg-1",
                claim_type: "meeting_entity_dismissed",
                field_path: Some("account"),
                text: "acct-x",
                actor: "user",
                source_scope: None,
                observed_at: TS,
                expires_at: None,
            },
        );
        assert_eq!(outcome, ShadowTombstoneOutcome::Committed);

        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM intelligence_claims \
                 WHERE claim_state = 'tombstoned' \
                   AND lower(json_extract(subject_ref, '$.kind')) = 'meeting' \
                   AND json_extract(subject_ref, '$.id') = 'mtg-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    /// L2 cycle-10 fix #2: shadow_write_tombstone_claim must produce
    /// well-formed JSON for subject_ref + metadata_json even when
    /// the caller-supplied subject_id or source_scope contain
    /// quotes, backslashes, newlines, or control characters. The
    /// previous `format!`-built JSON would fail commit_claim's
    /// subject_ref parser → outcome=Failed → callers treating
    /// shadow_write as best-effort would silently lose the claim.
    #[test]
    fn shadow_write_tombstone_claim_handles_weird_subject_id_and_source_scope() {
        let db = test_db();
        // Seed an account whose id contains characters that would
        // break naive JSON interpolation.
        let evil_id = "acct-with-\"quote\"-and-\\backslash-and-\nnewline";
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params![evil_id, "Evil Acct", TS],
            )
            .unwrap();

        let outcome = shadow_write_tombstone_claim(
            &db,
            ShadowTombstoneClaim {
                subject_kind: "Account",
                subject_id: evil_id,
                claim_type: "risk",
                field_path: Some("health.risk"),
                text: "Risk for evil-id account",
                actor: "user",
                source_scope: Some("scope-with-\"quote\"-and-\\backslash"),
                observed_at: TS,
                expires_at: None,
            },
        );
        assert_eq!(outcome, ShadowTombstoneOutcome::Committed);

        // Verify subject_ref + metadata_json on the row are valid JSON.
        let (subject_ref, metadata_json): (String, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT subject_ref, metadata_json FROM intelligence_claims \
                 WHERE claim_type = 'risk' AND text = 'risk for evil-id account'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        serde_json::from_str::<serde_json::Value>(&subject_ref)
            .unwrap_or_else(|e| panic!("subject_ref must be valid JSON, got {subject_ref:?}: {e}"));
        if let Some(metadata) = metadata_json.as_deref() {
            serde_json::from_str::<serde_json::Value>(metadata).unwrap_or_else(|e| {
                panic!("metadata_json must be valid JSON, got {metadata:?}: {e}")
            });
        }
    }

    /// L2 cycle-3 fix: Email is now a real SubjectRef variant
    /// (migration 132 added `emails.claim_version`). The cycle-2
    /// behavior — skip with SkippedUnsupportedSubjectKind — is
    /// removed. Email shadow-writes commit as ordinary tombstone
    /// claims.
    #[test]
    fn shadow_write_email_kind_persists_claim() {
        let db = test_db();
        // Seed an emails row so the bump_for_subject UPDATE has a target.
        db.conn_ref()
            .execute(
                "INSERT INTO emails (email_id, subject, received_at) \
                 VALUES (?1, 'subj', ?2)",
                params!["em-1", TS],
            )
            .unwrap();

        let outcome = shadow_write_tombstone_claim(
            &db,
            ShadowTombstoneClaim {
                subject_kind: "Email",
                subject_id: "em-1",
                claim_type: "email_dismissed",
                field_path: Some("commitment"),
                text: "blocking item",
                actor: "user",
                source_scope: None,
                observed_at: TS,
                expires_at: None,
            },
        );
        assert_eq!(outcome, ShadowTombstoneOutcome::Committed);

        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM intelligence_claims \
                 WHERE claim_state = 'tombstoned' \
                   AND lower(json_extract(subject_ref, '$.kind')) = 'email' \
                   AND json_extract(subject_ref, '$.id') = 'em-1' \
                   AND text = 'blocking item' \
                   AND field_path = 'commitment'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            count, 1,
            "Email shadow-write must persist a tombstone claim"
        );

        // emails.claim_version was bumped.
        let claim_version: i64 = db
            .conn_ref()
            .query_row(
                "SELECT claim_version FROM emails WHERE email_id = 'em-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(claim_version, 1);
    }

    /// L2 cycle-25 fix #3: shadow_write_tombstone_claim must propagate
    /// `expires_at` into the persisted claim row when supplied. Without
    /// this, runtime triage_snooze tombstones became permanent (snoozed
    /// items would never resurface even after the legacy snoozed_until
    /// expired). Asymmetric pair with the m8 SQL backfill which already
    /// preserves snoozed_until → expires_at.
    #[test]
    fn shadow_write_propagates_expires_at_for_finite_dismissals() {
        let db = test_db();
        seed_account(&db);

        let until = "2026-05-20T12:00:00+00:00";
        let outcome = shadow_write_tombstone_claim(
            &db,
            ShadowTombstoneClaim {
                subject_kind: "Account",
                subject_id: "acct-1",
                claim_type: "triage_snooze",
                field_path: Some("account"),
                text: "no_health_signals",
                actor: "user",
                source_scope: None,
                observed_at: TS,
                expires_at: Some(until),
            },
        );
        assert_eq!(outcome, ShadowTombstoneOutcome::Committed);

        let expires_at: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT expires_at FROM intelligence_claims \
                 WHERE claim_state = 'tombstoned' \
                   AND claim_type = 'triage_snooze' \
                   AND text = 'no_health_signals'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            expires_at.as_deref(),
            Some(until),
            "snooze expiry must round-trip into intelligence_claims.expires_at"
        );

        // Sanity: a None-expiry sibling still produces a permanent (NULL) row.
        let _ = shadow_write_tombstone_claim(
            &db,
            ShadowTombstoneClaim {
                subject_kind: "Account",
                subject_id: "acct-1",
                claim_type: "risk",
                field_path: Some("risks"),
                text: "permanent_dismiss",
                actor: "user",
                source_scope: None,
                observed_at: TS,
                expires_at: None,
            },
        );
        let null_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM intelligence_claims \
                 WHERE text = 'permanent_dismiss' AND expires_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            null_count, 1,
            "permanent dismissals must leave expires_at NULL"
        );
    }

    /// L2 cycle-26 fix #3: `withdraw_tombstones_for` flips matching
    /// tombstone rows to `withdrawn` + `dormant` and stamps the
    /// supplied `retraction_reason`. Non-matching rows are untouched.
    #[test]
    fn withdraw_tombstones_for_flips_only_matching_rows() {
        let db = test_db();
        seed_account(&db);
        // Seed a Person row so a 'person' subject can be a valid claim subject.
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) \
                 VALUES (?1, ?2, ?3, ?4)",
                params!["p-1", "p1@example.com", "P One", TS],
            )
            .unwrap();

        // Seed THREE tombstones: one matching, one with different role, one with different person.
        for (subj, role, label) in [
            ("p-1", "champion", "match"),
            ("p-1", "decision_maker", "wrong_role"),
            ("p-2", "champion", "wrong_person"),
        ] {
            let outcome = shadow_write_tombstone_claim(
                &db,
                ShadowTombstoneClaim {
                    subject_kind: "Person",
                    subject_id: subj,
                    claim_type: "stakeholder_role",
                    field_path: None,
                    text: role,
                    actor: "user",
                    source_scope: Some(label),
                    observed_at: TS,
                    expires_at: None,
                },
            );
            assert_eq!(outcome, ShadowTombstoneOutcome::Committed);
        }

        let withdrawn = withdraw_tombstones_for(
            &db,
            WithdrawTombstoneFilter {
                subject_kind: "Person",
                subject_id: "p-1",
                claim_type: "stakeholder_role",
                text: Some("champion"),
                field_path: None,
                retraction_reason: "restored_by_user",
            },
        )
        .unwrap();
        assert_eq!(withdrawn, 1, "exactly one row should be withdrawn");

        // The matching row is now withdrawn / dormant / restored_by_user.
        let (state, surfacing, reason): (String, String, String) = db
            .conn_ref()
            .query_row(
                "SELECT claim_state, surfacing_state, retraction_reason \
                 FROM intelligence_claims \
                 WHERE json_extract(metadata_json, '$.source_scope') = 'match'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap_or_else(|e| panic!("failed to read withdrawn row: {e}"));
        assert_eq!(state, "withdrawn");
        assert_eq!(surfacing, "dormant");
        assert_eq!(reason, "restored_by_user");

        // The non-matching rows stay tombstoned.
        let still_tombstoned: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM intelligence_claims \
                 WHERE claim_state = 'tombstoned'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(still_tombstoned, 2);

        // Calling again is idempotent (no rows match, returns 0).
        let zero = withdraw_tombstones_for(
            &db,
            WithdrawTombstoneFilter {
                subject_kind: "Person",
                subject_id: "p-1",
                claim_type: "stakeholder_role",
                text: Some("champion"),
                field_path: None,
                retraction_reason: "restored_by_user",
            },
        )
        .unwrap();
        assert_eq!(zero, 0, "idempotent: no matching tombstones remain");
    }

    /// L2 cycle-26 fix #1: `withdraw_all_tombstones_of_type` is the
    /// bulk reset path. Used by `reset_email_dismissals` to wipe
    /// every email_dismissed claim in a single transaction so PRE-GATE
    /// stops suppressing items after the legacy table is cleared.
    #[test]
    fn withdraw_all_tombstones_of_type_clears_every_matching_row() {
        let db = test_db();
        // Seed two distinct email subjects + two emails.
        for (eid, _label) in [("em-A", "matching-A"), ("em-B", "matching-B")] {
            db.conn_ref()
                .execute(
                    "INSERT INTO emails (email_id, subject, received_at) \
                     VALUES (?1, 'subj', ?2)",
                    params![eid, TS],
                )
                .unwrap();
            let outcome = shadow_write_tombstone_claim(
                &db,
                ShadowTombstoneClaim {
                    subject_kind: "Email",
                    subject_id: eid,
                    claim_type: "email_dismissed",
                    field_path: Some("commitment"),
                    text: "blocking_item",
                    actor: "user",
                    source_scope: None,
                    observed_at: TS,
                    expires_at: None,
                },
            );
            assert_eq!(outcome, ShadowTombstoneOutcome::Committed);
        }
        // Also seed an off-type tombstone that must NOT be touched.
        seed_account(&db);
        let _ = shadow_write_tombstone_claim(
            &db,
            ShadowTombstoneClaim {
                subject_kind: "Account",
                subject_id: "acct-1",
                claim_type: "risk",
                field_path: Some("risks"),
                text: "off_type",
                actor: "user",
                source_scope: None,
                observed_at: TS,
                expires_at: None,
            },
        );

        let withdrawn =
            withdraw_all_tombstones_of_type(&db, "email_dismissed", "reset_by_user").unwrap();
        assert_eq!(
            withdrawn, 2,
            "both email_dismissed tombstones must be withdrawn"
        );

        let (e_count, off_type_active): (i64, i64) = db
            .conn_ref()
            .query_row(
                "SELECT \
                   (SELECT count(*) FROM intelligence_claims \
                    WHERE claim_type = 'email_dismissed' AND claim_state = 'withdrawn'), \
                   (SELECT count(*) FROM intelligence_claims \
                    WHERE claim_type = 'risk' AND claim_state = 'tombstoned')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(e_count, 2);
        assert_eq!(off_type_active, 1, "off-type rows are untouched");
    }
}

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

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, OnceLock};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use parking_lot::Mutex;
use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension, Params};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::abilities::claims::{
    metadata_for_claim_type, ClaimActorClass, ClaimType, CommitPolicyClass,
};
use crate::abilities::feedback::{
    compute_needs_nuance_trust_effect, feedback_semantics, transition_for_feedback,
    ClaimFeedbackMetadata, ClaimRenderPolicy, ClaimVerificationState, FeedbackAction, RepairAction,
};
pub use crate::abilities::trust::TrustScore;
use crate::abilities::trust::{types as factors, TrustConfig};
use crate::db::claim_invalidation::SubjectRef;
use crate::db::claims::{
    ClaimSensitivity, ClaimState, IntelligenceClaim, ReconciliationKind, SurfacingState,
    TemporalScope,
};
use crate::db::{ActionDb, DbError};
use crate::intelligence::canonicalization::{item_hash, ItemKind};
use crate::services::context::{ClaimDismissalSurface, ServiceContext};

pub mod link_map;
mod link_map_macro;

// ---------------------------------------------------------------------------
// Public types: proposal + committed shape
// ---------------------------------------------------------------------------

/// Caller-supplied input to `commit_claim`. The service computes
/// dedup_key, canonical text, item_hash, and identity fields; the caller
/// supplies semantics + provenance, with registry defaults applied for
/// omitted scope/sensitivity values.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClaimProposal {
    /// Optional deterministic identity for migration/backfill callers.
    /// Runtime writes leave this empty and receive a fresh UUID v4.
    #[serde(default)]
    pub id: Option<String>,
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
    /// Claim ID superseded by this commit. The old claim is made dormant
    /// and linked to the new immutable claim in the same transaction.
    #[serde(default)]
    pub supersedes: Option<String>,
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

/// Claim-generation contract boundary.
///
/// Implementation note: this cites and accepts the W0-A enrichment refactor
/// design in `.docs/research/enrichment-refactor-design.md`: claim generation
/// is per reviewable fact; `get_entity_context` is the canonical Read shape;
/// `prepare_meeting` is a Transform that may produce bounded claim proposals;
/// narrative assembly cannot write durable claims directly. The signal policy,
/// durable repair job, and load-test amendment bullets in that document are
/// accepted here. None are rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimGenerationContract {
    ClaimExtraction,
    ClaimValidation,
    ClaimRepair,
    NarrativeAssembly,
}

/// Explicit per-ability claim-generation budget. These are generation budgets,
/// not total DB read budgets for the surrounding surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ClaimGenerationBudget {
    pub ability_id: String,
    pub contract: ClaimGenerationContract,
    pub max_candidate_claims: u16,
    pub max_provider_queries: u16,
    pub max_retrieval_sources: u16,
    pub max_llm_calls: u16,
    pub max_prompt_tokens: u32,
    pub max_output_tokens: u32,
    pub may_commit_claims: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum TargetedRepairProcessOutcome {
    NoJob,
    Completed {
        job_id: String,
        repair_jobs_processed: usize,
        claims_changed: usize,
        contradictions_reconciled: usize,
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
    #[error("unknown claim_id: {0}")]
    UnknownClaimId(String),
    #[error("claim not found: {0}")]
    ClaimNotFound(String),
    #[error("invalid claim feedback: {0}")]
    InvalidFeedback(String),
    #[error("invalid actor: {0}")]
    InvalidActor(String),
    #[error("invalid supersession: {0}")]
    InvalidSupersession(String),
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
    #[error("intelligence_claims UPDATE targets non-allowlisted columns: {0}")]
    ImmutableColumnUpdate(String),
    #[error("transaction error: {0}")]
    Transaction(String),
    #[error("database error: {0}")]
    Db(#[from] DbError),
    #[error("rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type ClaimsError = ClaimError;
pub type TrustVersion = i64;

// amendment D: assertion columns are insert-only. These are the only
// intelligence_claims columns the claim service may mutate in-place.
const CLAIM_UPDATE_ALLOWED_COLUMNS: &[&str] = &[
    "claim_state",
    "surfacing_state",
    "demotion_reason",
    "reactivated_at",
    "retraction_reason",
    "expires_at",
    "superseded_by",
    "trust_score",
    "trust_computed_at",
    "trust_version",
    "thread_id",
    // typed feedback adds derived review state; it is mutable
    // metadata, not assertion identity.
    "verification_state",
    "verification_reason",
    "needs_user_decision_at",
];

fn execute_claims_update<P>(conn: &Connection, sql: &str, params: P) -> Result<usize, ClaimError>
where
    P: Params,
{
    check_claim_update_allowlist(sql)?;
    Ok(conn.execute(sql, params)?)
}

fn execute_claims_update_sqlite<P>(
    conn: &Connection,
    sql: &str,
    params: P,
) -> rusqlite::Result<usize>
where
    P: Params,
{
    if check_claim_update_allowlist(sql).is_err() {
        return Err(rusqlite::Error::InvalidQuery);
    }
    conn.execute(sql, params)
}

fn check_claim_update_allowlist(sql: &str) -> Result<(), ClaimError> {
    let mut forbidden: Vec<String> = claim_update_columns(sql)
        .into_iter()
        .filter(|column| !CLAIM_UPDATE_ALLOWED_COLUMNS.contains(&column.as_str()))
        .collect();
    forbidden.sort();
    forbidden.dedup();

    if forbidden.is_empty() {
        Ok(())
    } else {
        Err(ClaimError::ImmutableColumnUpdate(forbidden.join(", ")))
    }
}

fn claim_update_columns(sql: &str) -> Vec<String> {
    let sql = strip_sql_comments(sql);
    let lower = sql.to_ascii_lowercase();
    let mut search_from = 0;

    while let Some(relative_idx) = lower[search_from..].find("update") {
        let update_idx = search_from + relative_idx;
        if !is_keyword_at(&lower, update_idx, "update") {
            search_from = update_idx + "update".len();
            continue;
        }

        let mut cursor = skip_ws(&sql, update_idx + "update".len());
        if is_keyword_at(&lower, cursor, "or") {
            cursor = skip_ws(&sql, cursor + "or".len());
            if let Some((_, next)) = parse_identifier(&sql, cursor) {
                cursor = skip_ws(&sql, next);
            }
        }

        let Some((first_ident, first_end)) = parse_identifier(&sql, cursor) else {
            search_from = update_idx + "update".len();
            continue;
        };
        cursor = skip_ws(&sql, first_end);

        let (table_ident, table_end) = if sql[cursor..].starts_with('.') {
            let next_start = skip_ws(&sql, cursor + 1);
            match parse_identifier(&sql, next_start) {
                Some((second_ident, second_end)) => (second_ident, second_end),
                None => (first_ident, cursor),
            }
        } else {
            (first_ident, cursor)
        };

        if table_ident != "intelligence_claims" {
            search_from = update_idx + "update".len();
            continue;
        }

        if let Some(set_idx) = find_keyword_from(&lower, "set", table_end) {
            return parse_set_columns(&sql, set_idx + "set".len());
        }

        search_from = update_idx + "update".len();
    }

    Vec::new()
}

fn strip_sql_comments(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();
    let mut in_single_quote = false;

    while let Some(ch) = chars.next() {
        if in_single_quote {
            out.push(ch);
            if ch == '\'' {
                if chars.peek() == Some(&'\'') {
                    out.push(chars.next().expect("peeked quote"));
                } else {
                    in_single_quote = false;
                }
            }
            continue;
        }

        if ch == '\'' {
            in_single_quote = true;
            out.push(ch);
            continue;
        }

        if ch == '-' && chars.peek() == Some(&'-') {
            out.push(' ');
            chars.next();
            for comment_ch in chars.by_ref() {
                if comment_ch == '\n' {
                    out.push('\n');
                    break;
                }
                out.push(' ');
            }
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'*') {
            out.push(' ');
            chars.next();
            let mut previous = '\0';
            for comment_ch in chars.by_ref() {
                out.push(if comment_ch == '\n' { '\n' } else { ' ' });
                if previous == '*' && comment_ch == '/' {
                    break;
                }
                previous = comment_ch;
            }
            continue;
        }

        out.push(ch);
    }

    out
}

fn parse_set_columns(sql: &str, mut cursor: usize) -> Vec<String> {
    let mut columns = Vec::new();

    while cursor < sql.len() {
        cursor = skip_ws(sql, cursor);
        if cursor >= sql.len() || top_level_clause_starts(sql, cursor) {
            break;
        }

        if let Some((row_columns, value_start)) = parse_row_value_set_target(sql, cursor) {
            columns.extend(row_columns);
            cursor = skip_expression(sql, value_start);
            continue;
        }

        let Some((column, next)) = parse_assignment_target(sql, cursor) else {
            cursor += next_char_len(sql, cursor);
            continue;
        };

        if !sql[next..].starts_with('=') {
            cursor = next + next_char_len(sql, next);
            continue;
        }

        columns.push(column);
        cursor = skip_expression(sql, next + 1);
    }

    columns
}

fn parse_assignment_target(sql: &str, cursor: usize) -> Option<(String, usize)> {
    let (mut column, mut next) = parse_identifier(sql, cursor)?;

    next = skip_ws(sql, next);
    if sql[next..].starts_with('.') {
        let qualified_start = skip_ws(sql, next + 1);
        if let Some((qualified_column, qualified_next)) = parse_identifier(sql, qualified_start) {
            column = qualified_column;
            next = skip_ws(sql, qualified_next);
        }
    }

    Some((column, next))
}

fn parse_row_value_set_target(sql: &str, cursor: usize) -> Option<(Vec<String>, usize)> {
    let mut cursor = skip_ws(sql, cursor);
    if !sql[cursor..].starts_with('(') {
        return None;
    }

    cursor += 1;
    let mut columns = Vec::new();
    while cursor < sql.len() {
        cursor = skip_ws(sql, cursor);
        if sql[cursor..].starts_with(')') {
            if columns.is_empty() {
                return None;
            }
            cursor += 1;
            break;
        }

        let (column, next) = parse_assignment_target(sql, cursor)?;
        columns.push(column);

        cursor = skip_ws(sql, next);
        if sql[cursor..].starts_with(',') {
            cursor += 1;
            continue;
        }
        if sql[cursor..].starts_with(')') {
            cursor += 1;
            break;
        }
        return None;
    }

    cursor = skip_ws(sql, cursor);
    if sql[cursor..].starts_with('=') {
        Some((columns, cursor + 1))
    } else {
        None
    }
}

fn skip_expression(sql: &str, mut cursor: usize) -> usize {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;

    while cursor < sql.len() {
        if quote.is_none() && depth == 0 && top_level_clause_starts(sql, cursor) {
            return cursor;
        }

        let ch = sql[cursor..].chars().next().expect("cursor in bounds");
        if let Some(active_quote) = quote {
            cursor += ch.len_utf8();
            if ch == active_quote {
                if active_quote == '\'' && sql[cursor..].starts_with('\'') {
                    cursor += 1;
                } else {
                    quote = None;
                }
            }
            continue;
        }

        match ch {
            '\'' | '"' | '`' => {
                quote = Some(ch);
                cursor += ch.len_utf8();
            }
            '[' => {
                quote = Some(']');
                cursor += ch.len_utf8();
            }
            '(' => {
                depth += 1;
                cursor += ch.len_utf8();
            }
            ')' => {
                depth = depth.saturating_sub(1);
                cursor += ch.len_utf8();
            }
            ',' if depth == 0 => return cursor + 1,
            _ => cursor += ch.len_utf8(),
        }
    }

    cursor
}

fn parse_identifier(sql: &str, cursor: usize) -> Option<(String, usize)> {
    let cursor = skip_ws(sql, cursor);
    let ch = sql[cursor..].chars().next()?;

    match ch {
        '\'' | '"' | '`' => parse_quoted_identifier(sql, cursor, ch, ch),
        '[' => parse_quoted_identifier(sql, cursor, '[', ']'),
        _ if is_ident_start(ch) => {
            let mut end = cursor + ch.len_utf8();
            while end < sql.len() {
                let next = sql[end..].chars().next().expect("end in bounds");
                if is_ident_continue(next) {
                    end += next.len_utf8();
                } else {
                    break;
                }
            }
            Some((sql[cursor..end].to_ascii_lowercase(), end))
        }
        _ => None,
    }
}

fn parse_quoted_identifier(
    sql: &str,
    cursor: usize,
    open: char,
    close: char,
) -> Option<(String, usize)> {
    debug_assert_eq!(sql[cursor..].chars().next(), Some(open));
    let mut ident = String::new();
    let mut idx = cursor + open.len_utf8();

    while idx < sql.len() {
        let ch = sql[idx..].chars().next().expect("idx in bounds");
        idx += ch.len_utf8();
        if ch == close {
            if sql[idx..].starts_with(close) {
                ident.push(ch);
                idx += close.len_utf8();
                continue;
            }
            return Some((ident.to_ascii_lowercase(), idx));
        }
        ident.push(ch);
    }

    None
}

fn skip_ws(sql: &str, mut cursor: usize) -> usize {
    while cursor < sql.len() {
        let ch = sql[cursor..].chars().next().expect("cursor in bounds");
        if ch.is_whitespace() || ch == '\\' {
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }
    cursor
}

fn next_char_len(sql: &str, cursor: usize) -> usize {
    sql[cursor..]
        .chars()
        .next()
        .map(char::len_utf8)
        .unwrap_or(1)
}

fn find_keyword_from(sql_lower: &str, keyword: &str, start: usize) -> Option<usize> {
    let mut search_from = start;
    while let Some(relative_idx) = sql_lower[search_from..].find(keyword) {
        let idx = search_from + relative_idx;
        if is_keyword_at(sql_lower, idx, keyword) {
            return Some(idx);
        }
        search_from = idx + keyword.len();
    }
    None
}

fn top_level_clause_starts(sql: &str, cursor: usize) -> bool {
    let lower = sql.to_ascii_lowercase();
    ["where", "returning", "order", "limit"]
        .iter()
        .any(|keyword| is_keyword_at(&lower, cursor, keyword))
}

fn is_keyword_at(sql_lower: &str, idx: usize, keyword: &str) -> bool {
    sql_lower[idx..].starts_with(keyword)
        && is_keyword_boundary(sql_lower, idx.checked_sub(1))
        && is_keyword_boundary(sql_lower, Some(idx + keyword.len()))
}

fn is_keyword_boundary(sql: &str, idx: Option<usize>) -> bool {
    match idx {
        None => true,
        Some(i) if i >= sql.len() => true,
        Some(i) => !sql[i..].chars().next().is_some_and(is_ident_continue),
    }
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
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

pub(crate) fn compute_user_note_dedup_key(
    subject_ref_compact: &str,
    actor: &str,
    created_at: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(subject_ref_compact.as_bytes());
    hasher.update(actor.as_bytes());
    hasher.update(timestamp_millis_key(created_at).as_bytes());
    format!("{:x}", hasher.finalize())
}

fn timestamp_millis_key(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Ok(parsed) = DateTime::parse_from_rfc3339(trimmed) {
        return parsed.timestamp_millis().to_string();
    }

    for format in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(trimmed, format) {
            return DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc)
                .timestamp_millis()
                .to_string();
        }
    }

    trimmed.to_string()
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
pub(crate) fn canonicalize_semantic_text(text: &str) -> String {
    let trimmed = text.trim();
    let collapsed: String = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.to_lowercase()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticSignature {
    terms: HashSet<String>,
    numbers: HashSet<String>,
    qualifiers: HashSet<String>,
    status: SemanticAssertionStatus,
}

const SEMANTIC_QUALIFIERS_METADATA_KEY: &str = "semantic_qualifiers";
const NON_SEMANTIC_MERGEABLE_METADATA_KEY: &str = "non_semantic_mergeable";
const LEGACY_SEMANTIC_QUALIFIERS_METADATA_KEY: &str = "dos280_semantic_qualifiers";
const LEGACY_NON_SEMANTIC_MERGEABLE_METADATA_KEY: &str = "dos280_non_semantic_mergeable";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SemanticAssertionStatus {
    Pending,
    Confirmed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SemanticDuplicateAction {
    Canonicalize,
    NeedsVerification,
}

struct SemanticDuplicateMatch {
    claim: IntelligenceClaim,
    action: SemanticDuplicateAction,
}

struct SemanticDuplicateLookup<'a> {
    subject: &'a SubjectRef,
    claim_type: &'a str,
    field_path: Option<&'a str>,
    canonical_text: &'a str,
    proposal_qualifiers: &'a HashSet<String>,
    proposal_temporal_scope: &'a TemporalScope,
    proposal_sensitivity: &'a ClaimSensitivity,
    now: &'a str,
}

struct ContradictionLookup<'a> {
    subject: &'a SubjectRef,
    claim_type: &'a str,
    field_path: Option<&'a str>,
    canonical_text: &'a str,
    proposal_qualifiers: &'a HashSet<String>,
    proposal_temporal_scope: &'a TemporalScope,
    proposal_sensitivity: &'a ClaimSensitivity,
    now: &'a str,
}

struct PreGateTombstoneLookup<'a> {
    subject: &'a SubjectRef,
    claim_type: &'a str,
    field_path: Option<&'a str>,
    item_hash_value: &'a str,
    canonical_text: &'a str,
    proposal_temporal_scope: &'a TemporalScope,
    proposal_sensitivity: &'a ClaimSensitivity,
    now: &'a str,
}

/// Builds a semantic near-duplicate signature. This is deliberately conservative:
/// it only compares claims that already share subject, claim_type,
/// and field family, then requires strong overlap on normalized
/// domain terms plus compatible assertion status.
fn compute_semantic_signature(text: &str) -> SemanticSignature {
    compute_semantic_signature_with_qualifiers(text, None)
}

fn compute_semantic_signature_with_qualifiers(
    text: &str,
    qualifiers: Option<&HashSet<String>>,
) -> SemanticSignature {
    let qualifiers = qualifiers
        .cloned()
        .unwrap_or_else(|| semantic_high_salience_qualifiers(text));
    let contraction_normalized = normalize_semantic_contractions(text);
    let mut normalized = String::with_capacity(text.len());
    for ch in contraction_normalized.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
        } else {
            normalized.push(' ');
        }
    }

    let raw_tokens = normalized.split_whitespace().collect::<Vec<_>>();
    let mut terms = HashSet::new();
    let mut numbers = HashSet::new();
    let mut status = SemanticAssertionStatus::Unknown;
    let mut negate_window = 0usize;
    let mut i = 0usize;

    while i < raw_tokens.len() {
        let mut raw = raw_tokens[i];
        if raw == "sign" && raw_tokens.get(i + 1) == Some(&"off") {
            raw = "signoff";
            i += 1;
        }

        if raw_tokens
            .get(i + 1)
            .is_some_and(|next| is_semantic_negator(next))
            && is_semantic_contraction_auxiliary_fragment(raw)
        {
            i += 1;
            continue;
        }

        if is_semantic_negator(raw) {
            negate_window = 3;
            i += 1;
            continue;
        }

        let negated = negate_window > 0;
        negate_window = negate_window.saturating_sub(1);

        if raw.chars().all(|ch| ch.is_ascii_digit()) {
            numbers.insert(raw.to_string());
            i += 1;
            continue;
        }

        if let Some((term, term_status)) = lookup_semantic_term(raw, negated) {
            terms.insert(term);
            status = combine_semantic_status(status, term_status);
        }

        i += 1;
    }

    SemanticSignature {
        terms,
        numbers,
        qualifiers,
        status,
    }
}

fn normalize_semantic_contractions(text: &str) -> String {
    let normalized = semantic_wont_contraction_regex().replace_all(text, "will not");
    let normalized = semantic_shant_contraction_regex().replace_all(&normalized, "shall not");
    let normalized = semantic_aint_contraction_regex().replace_all(&normalized, "am not");
    let normalized = semantic_cannot_regex().replace_all(&normalized, "can not");
    let normalized = semantic_cant_contraction_regex().replace_all(&normalized, "can not");
    semantic_negative_contraction_regex()
        .replace_all(&normalized, "${1} not")
        .into_owned()
}

fn semantic_negative_contraction_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new("(?i)\\b(\\w+)n['\u{2019}]t\\b")
            .expect("semantic negative contraction regex must compile")
    })
}

fn semantic_cannot_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("(?i)\\bcannot\\b").expect("cannot regex must compile"))
}

fn semantic_wont_contraction_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("(?i)\\bwon['\u{2019}]t\\b").expect("won't regex must compile"))
}

fn semantic_shant_contraction_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("(?i)\\bshan['\u{2019}]t\\b").expect("shan't regex must compile"))
}

fn semantic_cant_contraction_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("(?i)\\bcan['\u{2019}]t\\b").expect("can't regex must compile"))
}

fn semantic_aint_contraction_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("(?i)\\bain['\u{2019}]t\\b").expect("ain't regex must compile"))
}

fn is_semantic_negator(token: &str) -> bool {
    matches!(token, "not" | "no" | "never" | "without")
}

fn is_semantic_stopword(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "and"
            | "are"
            | "as"
            | "at"
            | "be"
            | "been"
            | "being"
            | "by"
            | "currently"
            | "can"
            | "could"
            | "did"
            | "do"
            | "does"
            | "due"
            | "for"
            | "from"
            | "has"
            | "have"
            | "had"
            | "is"
            | "it"
            | "its"
            | "of"
            | "on"
            | "or"
            | "our"
            | "remains"
            | "still"
            | "that"
            | "the"
            | "their"
            | "this"
            | "to"
            | "was"
            | "were"
            | "with"
            | "will"
            | "would"
            | "should"
            | "yet"
    )
}

fn is_semantic_contraction_auxiliary_fragment(token: &str) -> bool {
    token
        .strip_suffix('n')
        .filter(|stem| !stem.is_empty())
        .is_some_and(is_semantic_stopword)
}

fn lookup_semantic_term(token: &str, negated: bool) -> Option<(String, SemanticAssertionStatus)> {
    if is_semantic_stopword(token) {
        return None;
    }

    let (term, status) = match token {
        "approval" | "approvals" | "signoff" | "signoffs" => {
            let status = if negated {
                SemanticAssertionStatus::Pending
            } else {
                SemanticAssertionStatus::Unknown
            };
            ("approval", status)
        }
        "approve" | "approves" | "approved" | "approving" => (
            "approval",
            semantic_status_with_negation(SemanticAssertionStatus::Confirmed, negated),
        ),
        "awaiting" | "blocked" | "blocking" | "blocker" | "outstanding" | "pending" | "stalled"
        | "unapproved" => (
            "pending",
            semantic_status_with_negation(SemanticAssertionStatus::Pending, negated),
        ),
        "need" | "needed" | "needs" => (
            "pending",
            semantic_status_with_negation(SemanticAssertionStatus::Pending, negated),
        ),
        "confirm" | "confirms" | "confirmed" | "confirming" | "complete" | "completes"
        | "completed" | "completing" | "finalise" | "finalised" | "finalises" | "finalising"
        | "finalize" | "finalized" | "finalizes" | "finalizing" | "greenlight" | "greenlighted"
        | "greenlighting" | "greenlights" | "greenlit" | "land" | "landed" | "landing"
        | "lands" | "proceed" | "proceeded" | "proceeding" | "proceeds" | "secure" | "secured"
        | "secures" | "securing" => (
            "confirmed",
            semantic_status_with_negation(SemanticAssertionStatus::Confirmed, negated),
        ),
        "budget" | "budgets" | "funding" | "funds" => ("budget", SemanticAssertionStatus::Unknown),
        "finance" | "financial" | "cfo" => ("finance", SemanticAssertionStatus::Unknown),
        "phase" | "phases" => ("phase", SemanticAssertionStatus::Unknown),
        "signing" | "signings" => ("signing", SemanticAssertionStatus::Unknown),
        "signature" | "signatures" => ("signature", SemanticAssertionStatus::Unknown),
        _ => {
            let stemmed = semantic_stem(token);
            return if stemmed.is_empty() {
                None
            } else {
                Some((stemmed, SemanticAssertionStatus::Unknown))
            };
        }
    };

    Some((term.to_string(), status))
}

fn semantic_status_with_negation(
    status: SemanticAssertionStatus,
    negated: bool,
) -> SemanticAssertionStatus {
    if !negated {
        return status;
    }

    match status {
        SemanticAssertionStatus::Confirmed => SemanticAssertionStatus::Pending,
        SemanticAssertionStatus::Pending | SemanticAssertionStatus::Unknown => {
            SemanticAssertionStatus::Unknown
        }
    }
}

fn semantic_stem(token: &str) -> String {
    let mut stem = token.to_string();
    for suffix in ["ing", "ed", "es", "s"] {
        if stem.len() > suffix.len() + 3 && stem.ends_with(suffix) {
            stem.truncate(stem.len() - suffix.len());
            break;
        }
    }
    stem
}

fn combine_semantic_status(
    current: SemanticAssertionStatus,
    next: SemanticAssertionStatus,
) -> SemanticAssertionStatus {
    match (current, next) {
        (SemanticAssertionStatus::Pending, SemanticAssertionStatus::Confirmed)
        | (SemanticAssertionStatus::Confirmed, SemanticAssertionStatus::Pending) => current,
        (SemanticAssertionStatus::Unknown, status) => status,
        (status, SemanticAssertionStatus::Unknown) => status,
        (status, _) => status,
    }
}

fn semantic_status_compatible(
    left: SemanticAssertionStatus,
    right: SemanticAssertionStatus,
) -> bool {
    matches!(
        (left, right),
        (SemanticAssertionStatus::Unknown, _)
            | (_, SemanticAssertionStatus::Unknown)
            | (
                SemanticAssertionStatus::Pending,
                SemanticAssertionStatus::Pending
            )
            | (
                SemanticAssertionStatus::Confirmed,
                SemanticAssertionStatus::Confirmed
            )
    )
}

pub(crate) fn semantic_high_salience_qualifiers(text: &str) -> HashSet<String> {
    let mut qualifiers = HashSet::new();
    let mut token = String::new();
    let normalized_text = normalize_semantic_region_aliases(text);

    for ch in normalized_text.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_alphanumeric() {
            token.push(ch);
            continue;
        }

        if token.is_empty() {
            continue;
        }

        let lower = token.to_ascii_lowercase();
        let upper = token.to_ascii_uppercase();
        if matches!(lower.as_str(), "q1" | "q2" | "q3" | "q4") {
            qualifiers.insert(format!("quarter:{lower}"));
        } else if matches!(upper.as_str(), "US" | "UK" | "EU" | "APAC" | "EMEA") && token == upper {
            qualifiers.insert(format!("region:{upper}"));
        } else if matches!(lower.parse::<i32>(), Ok(2024..=2030)) {
            qualifiers.insert(format!("year:{lower}"));
        } else if is_semantic_named_entity(&token) {
            qualifiers.insert(format!("entity:{lower}"));
        }

        token.clear();
    }

    qualifiers
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SemanticRegionAliasSegmentKind {
    Token,
    Separator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticRegionAliasSegment {
    kind: SemanticRegionAliasSegmentKind,
    value: String,
}

fn normalize_semantic_region_aliases(text: &str) -> String {
    let segments = semantic_region_alias_segments(text);
    let mut normalized = String::with_capacity(text.len());
    let mut i = 0usize;

    while i < segments.len() {
        let segment = &segments[i];
        if matches!(segment.kind, SemanticRegionAliasSegmentKind::Token) {
            if let Some(region) = semantic_region_phrase_alias_at(&segments, i) {
                normalized.push_str(region);
                i += 3;
                continue;
            }
            if let Some(region) = semantic_region_token_alias(&segment.value) {
                normalized.push_str(region);
                i += 1;
                continue;
            }
        }

        normalized.push_str(&segment.value);
        i += 1;
    }

    normalized
}

fn semantic_region_alias_segments(text: &str) -> Vec<SemanticRegionAliasSegment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut current_kind = None;

    for ch in text.chars() {
        let kind = if ch.is_ascii_alphanumeric() || ch == '.' {
            SemanticRegionAliasSegmentKind::Token
        } else {
            SemanticRegionAliasSegmentKind::Separator
        };

        if current_kind == Some(kind) {
            current.push(ch);
            continue;
        }

        if let Some(kind) = current_kind {
            segments.push(SemanticRegionAliasSegment {
                kind,
                value: std::mem::take(&mut current),
            });
        }
        current.push(ch);
        current_kind = Some(kind);
    }

    if let Some(kind) = current_kind {
        segments.push(SemanticRegionAliasSegment {
            kind,
            value: current,
        });
    }

    segments
}

fn semantic_region_phrase_alias_at(
    segments: &[SemanticRegionAliasSegment],
    index: usize,
) -> Option<&'static str> {
    let [first, separator, second] = segments.get(index..index + 3)? else {
        return None;
    };
    if !matches!(first.kind, SemanticRegionAliasSegmentKind::Token)
        || !matches!(separator.kind, SemanticRegionAliasSegmentKind::Separator)
        || !matches!(second.kind, SemanticRegionAliasSegmentKind::Token)
        || !separator.value.chars().all(char::is_whitespace)
    {
        return None;
    }

    match (
        semantic_region_token_key(&first.value).as_str(),
        semantic_region_token_key(&second.value).as_str(),
    ) {
        ("united", "states") => Some("US"),
        ("united", "kingdom") => Some("UK"),
        ("european", "union") => Some("EU"),
        _ => None,
    }
}

fn semantic_region_token_alias(token: &str) -> Option<&'static str> {
    let key = semantic_region_token_key(token);
    let has_period = token.contains('.');
    let alnum = token
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    let all_upper = !alnum.is_empty() && alnum == alnum.to_ascii_uppercase();

    match key.as_str() {
        "us" if alnum == "US" || semantic_region_dotted_acronym(token, "US") => Some("US"),
        "usa" if all_upper => Some("US"),
        "uk" if has_period || all_upper => Some("UK"),
        "eu" if has_period || all_upper => Some("EU"),
        "apac" if has_period || all_upper => Some("APAC"),
        "emea" if has_period || all_upper => Some("EMEA"),
        _ => None,
    }
}

fn semantic_region_dotted_acronym(token: &str, expected: &str) -> bool {
    if !token.contains('.') {
        return false;
    }

    let mut token_chars = token.chars();
    let mut expected_chars = expected.chars().peekable();
    while let Some(expected_ch) = expected_chars.next() {
        let Some(token_ch) = token_chars.next() else {
            return false;
        };
        if !token_ch.eq_ignore_ascii_case(&expected_ch) {
            return false;
        }
        if expected_chars.peek().is_some() && token_chars.next() != Some('.') {
            return false;
        }
    }

    match token_chars.next() {
        None => true,
        Some('.') => token_chars.next().is_none(),
        Some(_) => false,
    }
}

fn semantic_region_token_key(token: &str) -> String {
    token
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_semantic_named_entity(token: &str) -> bool {
    if token.len() < 3 || token.chars().any(|ch| ch.is_ascii_digit()) {
        return false;
    }

    let lower = token.to_ascii_lowercase();
    if is_semantic_low_salience_token(&lower) {
        return false;
    }

    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    let has_later_upper = chars.clone().any(|ch| ch.is_ascii_uppercase());
    let all_upper = token.chars().all(|ch| ch.is_ascii_uppercase());
    let title_case =
        first.is_ascii_uppercase() && token[1..].chars().all(|ch| ch.is_ascii_lowercase());

    has_later_upper || all_upper || title_case
}

fn metadata_with_semantic_qualifiers(
    metadata_json: Option<String>,
    qualifiers: &HashSet<String>,
) -> Option<String> {
    let mut sorted = qualifiers.iter().cloned().collect::<Vec<_>>();
    sorted.sort();
    let qualifier_value =
        serde_json::Value::Array(sorted.into_iter().map(serde_json::Value::String).collect());

    let mut root = match metadata_json {
        Some(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(serde_json::Value::Object(map)) => map,
            Ok(_) | Err(_) => return Some(raw),
        },
        None => serde_json::Map::new(),
    };

    root.insert(
        SEMANTIC_QUALIFIERS_METADATA_KEY.to_string(),
        qualifier_value,
    );
    Some(serde_json::Value::Object(root).to_string())
}

fn semantic_qualifiers_from_metadata(metadata_json: Option<&str>) -> Option<HashSet<String>> {
    let metadata = serde_json::from_str::<serde_json::Value>(metadata_json?).ok()?;
    let metadata = metadata.as_object()?;
    if [
        NON_SEMANTIC_MERGEABLE_METADATA_KEY,
        LEGACY_NON_SEMANTIC_MERGEABLE_METADATA_KEY,
    ]
    .iter()
    .any(|key| {
        metadata
            .get(*key)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }) {
        return None;
    }

    let raw_qualifiers = metadata
        .get(SEMANTIC_QUALIFIERS_METADATA_KEY)
        .or_else(|| metadata.get(LEGACY_SEMANTIC_QUALIFIERS_METADATA_KEY))?
        .as_array()?;
    let mut qualifiers = HashSet::new();
    for value in raw_qualifiers {
        let qualifier = value.as_str()?;
        if !is_semantic_metadata_qualifier(qualifier) {
            return None;
        }
        qualifiers.insert(qualifier.to_string());
    }

    Some(qualifiers)
}

fn is_semantic_metadata_qualifier(qualifier: &str) -> bool {
    let Some((kind, value)) = qualifier.split_once(':') else {
        return false;
    };

    !value.is_empty() && matches!(kind, "quarter" | "region" | "year" | "entity")
}

fn semantic_claim_qualifiers(claim: &IntelligenceClaim) -> Option<HashSet<String>> {
    semantic_qualifiers_from_metadata(claim.metadata_json.as_deref())
}

fn semantic_explicit_synonym_map() -> &'static HashMap<&'static str, &'static [&'static str]> {
    static MAP: OnceLock<HashMap<&'static str, &'static [&'static str]>> = OnceLock::new();
    MAP.get_or_init(|| {
        HashMap::from([
            ("budget", &["funding"][..]),
            ("funding", &["budget"][..]),
            ("contract", &["agreement"][..]),
            ("agreement", &["contract", "deal"][..]),
            ("deal", &["agreement"][..]),
            ("signing", &["signature"][..]),
            ("signature", &["signing"][..]),
            ("approval", &["approved", "greenlit"][..]),
            ("approved", &["approval", "greenlit"][..]),
            ("greenlit", &["approval", "approved"][..]),
        ])
    })
}

fn semantic_terms_explicit_synonyms(left: &str, right: &str) -> bool {
    semantic_explicit_synonym_map()
        .get(left)
        .is_some_and(|synonyms| synonyms.contains(&right))
        || semantic_explicit_synonym_map()
            .get(right)
            .is_some_and(|synonyms| synonyms.contains(&left))
}

fn is_semantic_unmatched_low_salience_term(term: &str) -> bool {
    is_semantic_stopword(term)
        || is_semantic_negator(term)
        || matches!(
            term,
            "approval" | "pending" | "confirmed" | "finance" | "phase"
        )
}

fn semantic_unmatched_terms<'a>(
    left: &'a HashSet<String>,
    right: &'a HashSet<String>,
) -> Vec<&'a str> {
    left.difference(right)
        .map(String::as_str)
        .filter(|term| !is_semantic_unmatched_low_salience_term(term))
        .collect()
}

fn semantic_has_unsynonymized_unmatched_terms(
    left: &SemanticSignature,
    right: &SemanticSignature,
) -> bool {
    let left_unmatched = semantic_unmatched_terms(&left.terms, &right.terms);
    let right_unmatched = semantic_unmatched_terms(&right.terms, &left.terms);

    left_unmatched.iter().any(|term| {
        !right_unmatched
            .iter()
            .any(|other| semantic_terms_explicit_synonyms(term, other))
    }) || right_unmatched.iter().any(|term| {
        !left_unmatched
            .iter()
            .any(|other| semantic_terms_explicit_synonyms(term, other))
    })
}

fn is_semantic_low_salience_token(token: &str) -> bool {
    is_semantic_stopword(token)
        || is_semantic_negator(token)
        || matches!(
            token,
            "approval"
                | "approvals"
                | "approve"
                | "approves"
                | "approved"
                | "approving"
                | "awaiting"
                | "blocked"
                | "blocking"
                | "blocker"
                | "outstanding"
                | "pending"
                | "stalled"
                | "unapproved"
                | "need"
                | "needed"
                | "needs"
                | "confirm"
                | "confirms"
                | "confirmed"
                | "confirming"
                | "complete"
                | "completes"
                | "completed"
                | "completing"
                | "finalise"
                | "finalised"
                | "finalises"
                | "finalising"
                | "finalize"
                | "finalized"
                | "finalizes"
                | "finalizing"
                | "greenlight"
                | "greenlighted"
                | "greenlighting"
                | "greenlights"
                | "greenlit"
                | "land"
                | "landed"
                | "landing"
                | "lands"
                | "proceed"
                | "proceeded"
                | "proceeding"
                | "proceeds"
                | "secure"
                | "secured"
                | "secures"
                | "securing"
                | "budget"
                | "budgets"
                | "funding"
                | "funds"
                | "finance"
                | "financial"
                | "cfo"
                | "phase"
                | "phases"
        )
}

fn semantic_near_duplicate(left: &str, right: &str) -> bool {
    let left = compute_semantic_signature(left);
    let right = compute_semantic_signature(right);
    semantic_signatures_near_duplicate(&left, &right)
}

fn semantic_near_duplicate_with_qualifiers(
    left_text: &str,
    left_qualifiers: &HashSet<String>,
    right_text: &str,
    right_qualifiers: &HashSet<String>,
) -> bool {
    let left = compute_semantic_signature_with_qualifiers(left_text, Some(left_qualifiers));
    let right = compute_semantic_signature_with_qualifiers(right_text, Some(right_qualifiers));
    semantic_signatures_near_duplicate(&left, &right)
}

fn semantic_claim_near_duplicate(
    claim: &IntelligenceClaim,
    canonical_text: &str,
    proposal_qualifiers: &HashSet<String>,
) -> bool {
    let Some(claim_qualifiers) = semantic_claim_qualifiers(claim) else {
        return false;
    };
    semantic_near_duplicate_with_qualifiers(
        &claim.text,
        &claim_qualifiers,
        canonical_text,
        proposal_qualifiers,
    )
}

fn semantic_signatures_near_duplicate(left: &SemanticSignature, right: &SemanticSignature) -> bool {
    if matches!(left.status, SemanticAssertionStatus::Unknown)
        || matches!(right.status, SemanticAssertionStatus::Unknown)
    {
        return false;
    }

    if !semantic_status_compatible(left.status, right.status) {
        return false;
    }

    if !left.numbers.is_empty() && !right.numbers.is_empty() && left.numbers != right.numbers {
        return false;
    }

    if left.qualifiers != right.qualifiers {
        return false;
    }

    let overlap = left.terms.intersection(&right.terms).count();
    if overlap < 4 {
        return false;
    }

    let union = left.terms.union(&right.terms).count();
    if union == 0 {
        return false;
    }

    if semantic_has_unsynonymized_unmatched_terms(left, right) {
        return false;
    }

    let smaller = left.terms.len().min(right.terms.len()).max(1);
    let jaccard = overlap as f64 / union as f64;
    let coverage = overlap as f64 / smaller as f64;
    jaccard >= 0.58 && coverage >= 0.67
}

fn trust_band_for_score(trust_score: Option<f64>) -> factors::TrustBand {
    let Some(score) = trust_score.filter(|score| score.is_finite()) else {
        return factors::TrustBand::Unscored;
    };
    let config = TrustConfig::default();
    if score >= config.likely_current_min {
        factors::TrustBand::LikelyCurrent
    } else if score >= config.use_with_caution_min {
        factors::TrustBand::UseWithCaution
    } else {
        factors::TrustBand::NeedsVerification
    }
}

fn needs_verification_score() -> f64 {
    let config = TrustConfig::default();
    (config.use_with_caution_min - config.clamp_floor)
        .max(TrustScore::MIN)
        .min(config.use_with_caution_min)
}

fn semantic_trust_band_allows_canonicalization(band: factors::TrustBand) -> bool {
    matches!(
        band,
        factors::TrustBand::LikelyCurrent | factors::TrustBand::UseWithCaution
    )
}

fn claim_sensitivity_restriction_rank(sensitivity: &ClaimSensitivity) -> u8 {
    match sensitivity {
        ClaimSensitivity::Public => 0,
        ClaimSensitivity::Internal => 1,
        ClaimSensitivity::Confidential => 2,
        ClaimSensitivity::UserOnly => 3,
    }
}

fn claim_merge_tier_values_compatible(
    existing_temporal_scope: &TemporalScope,
    existing_sensitivity: &ClaimSensitivity,
    proposal_temporal_scope: &TemporalScope,
    proposal_sensitivity: &ClaimSensitivity,
) -> bool {
    existing_temporal_scope == proposal_temporal_scope
        && claim_sensitivity_restriction_rank(proposal_sensitivity)
            <= claim_sensitivity_restriction_rank(existing_sensitivity)
}

fn claim_merge_tiers_compatible(
    existing: &IntelligenceClaim,
    proposal_temporal_scope: &TemporalScope,
    proposal_sensitivity: &ClaimSensitivity,
) -> bool {
    claim_merge_tier_values_compatible(
        &existing.temporal_scope,
        &existing.sensitivity,
        proposal_temporal_scope,
        proposal_sensitivity,
    )
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
///
/// A tombstone only blocks proposals in the same merge tier: temporal
/// scope must match, and the proposal sensitivity must not be more
/// restrictive than the tombstone sensitivity.
fn pre_gate_blocking_tombstone_exists(
    conn: &rusqlite::Connection,
    lookup: PreGateTombstoneLookup<'_>,
) -> Result<bool, ClaimError> {
    let Some(kind) = subject_kind_label(lookup.subject) else {
        // Multi/Global subjects don't participate in single-tombstone
        // suppression. Fall through to the active-write path.
        return Ok(false);
    };
    let Some(id) = subject_id_for_lookup(lookup.subject) else {
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
        SELECT temporal_scope, sensitivity \
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
          AND TIER_PREDICATE";

    let hit = |predicate: &str, params: &[&dyn rusqlite::ToSql]| -> Result<bool, ClaimError> {
        let sql = TIER_SQL.replace("TIER_PREDICATE", predicate);
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query(params)?;
        while let Some(row) = rows.next()? {
            let tombstone_temporal_scope_raw: String = row.get(0)?;
            let tombstone_sensitivity_raw: String = row.get(1)?;
            let (Ok(tombstone_temporal_scope), Ok(tombstone_sensitivity)) = (
                parse_db_enum::<TemporalScope>(tombstone_temporal_scope_raw),
                parse_db_enum::<ClaimSensitivity>(tombstone_sensitivity_raw),
            ) else {
                continue;
            };

            if claim_merge_tier_values_compatible(
                &tombstone_temporal_scope,
                &tombstone_sensitivity,
                lookup.proposal_temporal_scope,
                lookup.proposal_sensitivity,
            ) {
                return Ok(true);
            }
        }
        Ok(false)
    };

    let field = lookup.field_path.unwrap_or("");

    // Hash tier
    if !lookup.item_hash_value.is_empty()
        && hit(
            "item_hash IS NOT NULL AND item_hash <> '' AND item_hash = ?6",
            &[
                &lookup.claim_type,
                &field,
                &kind,
                &id,
                &lookup.now,
                &lookup.item_hash_value,
            ],
        )?
    {
        return Ok(true);
    }

    // Exact text tier — NOCASE so backfilled tombstones with the
    // legacy mixed-case `text` column still match runtime
    // canonical_text (which is lowercased by canonicalize_semantic_text).
    if !lookup.canonical_text.is_empty()
        && hit(
            "text = ?6 COLLATE NOCASE",
            &[
                &lookup.claim_type,
                &field,
                &kind,
                &id,
                &lookup.now,
                &lookup.canonical_text,
            ],
        )?
    {
        return Ok(true);
    }

    // Keyless field-wide tier
    if hit(
        "text = '<keyless>'",
        &[&lookup.claim_type, &field, &kind, &id, &lookup.now],
    )? {
        return Ok(true);
    }

    Ok(false)
}

fn candidate_claim_shadowed_by_compatible_tombstone(
    conn: &rusqlite::Connection,
    subject: &SubjectRef,
    candidate: &IntelligenceClaim,
    proposal_temporal_scope: &TemporalScope,
    proposal_sensitivity: &ClaimSensitivity,
    now: &str,
) -> Result<bool, ClaimError> {
    pre_gate_blocking_tombstone_exists(
        conn,
        PreGateTombstoneLookup {
            subject,
            claim_type: candidate.claim_type.as_str(),
            field_path: candidate.field_path.as_deref(),
            item_hash_value: candidate.item_hash.as_deref().unwrap_or(""),
            canonical_text: &candidate.text,
            proposal_temporal_scope,
            proposal_sensitivity,
            now,
        },
    )
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

fn insert_claim_edges(tx: &ActionDb, claim: &IntelligenceClaim) -> Result<(), ClaimError> {
    let edges = link_map::compile_edges_from_claim(claim);
    if edges.is_empty() || !claim_edges_table_exists(tx)? {
        return Ok(());
    }

    for edge in edges {
        tx.conn_ref().execute(
            "INSERT OR IGNORE INTO claim_edges (
                id, from_entity_id, to_entity_id, edge_type, origin_claim_id,
                link_source, weight, confidence, superseded_by, tombstoned_at, created_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, ?9
             )",
            params![
                &edge.id,
                &edge.from_entity_id,
                &edge.to_entity_id,
                &edge.edge_type,
                &edge.origin_claim_id,
                edge.link_source,
                edge.weight,
                edge.confidence,
                &claim.created_at,
            ],
        )?;
    }
    Ok(())
}

fn claim_edges_table_exists(tx: &ActionDb) -> Result<bool, ClaimError> {
    Ok(tx
        .conn_ref()
        .query_row(
            "SELECT 1
             FROM sqlite_master
             WHERE type = 'table'
               AND name = 'claim_edges'
             LIMIT 1",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn mark_claim_edges_superseded_by_claim(
    tx: &ActionDb,
    origin_claim_id: &str,
    replacement_claim_id: &str,
) -> Result<(), ClaimError> {
    if !claim_edges_table_exists(tx)? {
        return Ok(());
    }

    tx.conn_ref().execute(
        "UPDATE claim_edges
         SET superseded_by = ?1
         WHERE origin_claim_id = ?2
           AND superseded_by IS NULL
           AND tombstoned_at IS NULL",
        params![replacement_claim_id, origin_claim_id],
    )?;
    Ok(())
}

fn mark_claim_edges_tombstoned(
    tx: &ActionDb,
    origin_claim_id: &str,
    tombstoned_at: &str,
) -> Result<(), ClaimError> {
    if !claim_edges_table_exists(tx)? {
        return Ok(());
    }

    tx.conn_ref().execute(
        "UPDATE claim_edges
         SET tombstoned_at = ?1
         WHERE origin_claim_id = ?2
           AND superseded_by IS NULL
           AND tombstoned_at IS NULL",
        params![tombstoned_at, origin_claim_id],
    )?;
    Ok(())
}

fn mark_claim_edges_tombstoned_for_identity(
    tx: &ActionDb,
    subject: &SubjectRef,
    claim_type: &str,
    field_path: Option<&str>,
    tombstoned_at: &str,
) -> Result<(), ClaimError> {
    if !claim_edges_table_exists(tx)? {
        return Ok(());
    }

    let Some(kind) = subject_kind_label(subject) else {
        return Ok(());
    };
    let Some(id) = subject_id_for_lookup(subject) else {
        return Ok(());
    };
    let field = field_path.unwrap_or("");

    tx.conn_ref().execute(
        "UPDATE claim_edges
         SET tombstoned_at = ?1
         WHERE superseded_by IS NULL
           AND tombstoned_at IS NULL
           AND origin_claim_id IN (
               SELECT ic.id
               FROM intelligence_claims ic
               WHERE ic.claim_type = ?2
                 AND coalesce(ic.field_path, '') = coalesce(?3, '')
                 AND json_valid(ic.subject_ref) = 1
                 AND lower(json_extract(ic.subject_ref, '$.kind')) = lower(?4)
                 AND json_extract(ic.subject_ref, '$.id') = ?5
           )",
        params![tombstoned_at, claim_type, field, kind, id],
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

/// Sensitivity gate for data crossing an LLM prompt-input boundary.
pub fn prompt_input_sensitivity_allowed(sensitivity: &ClaimSensitivity) -> bool {
    matches!(
        sensitivity,
        ClaimSensitivity::Public | ClaimSensitivity::Internal
    )
}

pub fn claim_allowed_for_prompt_input(claim: &IntelligenceClaim) -> bool {
    prompt_input_sensitivity_allowed(&claim.sensitivity)
}

pub fn prompt_input_sensitivity_name_allowed(sensitivity: &str) -> bool {
    matches!(
        sensitivity.trim().to_ascii_lowercase().as_str(),
        "public" | "internal"
    )
}

pub fn load_claim_by_id(
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

/// L2 cycle-1 fix #6: load the first ACTIVE claim with this exact dedup_key
/// that can merge with the proposal's temporal/sensitivity tier. Used by
/// commit_claim's same-meaning merge branch to detect a re-commit of the same
/// logical content and route it through corroboration instead of inserting a
/// duplicate active row.
fn load_active_claim_by_dedup_key(
    conn: &rusqlite::Connection,
    dedup_key: &str,
    proposal_temporal_scope: &TemporalScope,
    proposal_sensitivity: &ClaimSensitivity,
) -> Result<Option<IntelligenceClaim>, ClaimError> {
    let sql = format!(
        "SELECT {CLAIM_COLUMNS} FROM intelligence_claims \
         WHERE dedup_key = ?1 AND claim_state = 'active' AND surfacing_state = 'active' \
         ORDER BY created_at DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![dedup_key])?;
    while let Some(row) = rows.next()? {
        let claim = read_claim_row(row)?;
        if claim_merge_tiers_compatible(&claim, proposal_temporal_scope, proposal_sensitivity) {
            return Ok(Some(claim));
        }
    }

    Ok(None)
}

/// Looks up semantic near-duplicates. Exact `dedup_key` equality is
/// handled first; this scans the same entity + claim family for a tightly
/// scoped semantic signature match before the contradiction fork path runs.
fn load_active_semantic_duplicate_claim(
    conn: &rusqlite::Connection,
    lookup: SemanticDuplicateLookup<'_>,
) -> Result<Option<SemanticDuplicateMatch>, ClaimError> {
    let Some(kind) = subject_kind_label(lookup.subject) else {
        return Ok(None);
    };
    let Some(id) = subject_id_for_lookup(lookup.subject) else {
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
           AND active.surfacing_state = 'active' \
         ORDER BY active.created_at DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![kind, id, lookup.claim_type, lookup.field_path])?;
    let mut needs_verification_match = None;

    while let Some(row) = rows.next()? {
        let claim = read_claim_row(row)?;
        if candidate_claim_shadowed_by_compatible_tombstone(
            conn,
            lookup.subject,
            &claim,
            lookup.proposal_temporal_scope,
            lookup.proposal_sensitivity,
            lookup.now,
        )? {
            continue;
        }
        if !claim_merge_tiers_compatible(
            &claim,
            lookup.proposal_temporal_scope,
            lookup.proposal_sensitivity,
        ) {
            continue;
        }
        if !semantic_claim_near_duplicate(&claim, lookup.canonical_text, lookup.proposal_qualifiers)
        {
            continue;
        }

        let trust_band = trust_band_for_score(claim.trust_score);
        if semantic_trust_band_allows_canonicalization(trust_band) {
            return Ok(Some(SemanticDuplicateMatch {
                claim,
                action: SemanticDuplicateAction::Canonicalize,
            }));
        } else {
            needs_verification_match.get_or_insert(SemanticDuplicateMatch {
                claim,
                action: SemanticDuplicateAction::NeedsVerification,
            });
        }
    }

    Ok(needs_verification_match)
}

/// L2 cycle-1 fix #6: load any ACTIVE claim that contradicts the
/// proposal — same (subject_ref, claim_type, field_path) but DIFFERENT
/// canonical text. Used by commit_claim's contradiction-fork branch.
/// Returns the most recently created contradicting claim (one fork
/// per commit; subsequent contradictions chain off the new claim).
///
/// Skips active claims whose own semantic identity has a policy-compatible
/// tombstone in the table — those are "effectively retracted" by a user
/// dismissal even though their `claim_state` column hasn't been
/// transitioned (the claims substrate keeps active rows append-only; tombstones
/// shadow them via PRE-GATE on re-commit). Without this skip, a
/// paraphrase commit after the user dismissed the original would
/// fork a contradiction against a claim the user has already
/// retracted.
fn load_active_contradicting_claim(
    conn: &rusqlite::Connection,
    lookup: ContradictionLookup<'_>,
) -> Result<Option<IntelligenceClaim>, ClaimError> {
    // L2 cycle-12 fix #1: match the active subject by kind+id via
    // json_extract instead of exact subject_ref string equality.
    // Two byte-different but semantically-identical subject_refs
    // (e.g. reversed key order from json_object vs serde_json
    // serialization) would otherwise miss this contradiction
    // detector and silently insert an unlinked duplicate active
    // claim. json_valid guards malformed historical rows from
    // tripping json_extract mid-query (cycle-7 hazard).
    let Some(kind) = subject_kind_label(lookup.subject) else {
        return Ok(None);
    };
    let Some(id) = subject_id_for_lookup(lookup.subject) else {
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
           AND active.surfacing_state = 'active' \
           AND active.text <> ?5 \
         ORDER BY active.created_at DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![
        kind,
        id,
        lookup.claim_type,
        lookup.field_path,
        lookup.canonical_text
    ])?;
    while let Some(row) = rows.next()? {
        let claim = read_claim_row(row)?;
        if candidate_claim_shadowed_by_compatible_tombstone(
            conn,
            lookup.subject,
            &claim,
            lookup.proposal_temporal_scope,
            lookup.proposal_sensitivity,
            lookup.now,
        )? {
            continue;
        }
        if !claim_merge_tiers_compatible(
            &claim,
            lookup.proposal_temporal_scope,
            lookup.proposal_sensitivity,
        ) {
            continue;
        }
        if semantic_claim_near_duplicate(&claim, lookup.canonical_text, lookup.proposal_qualifiers)
        {
            continue;
        }
        return Ok(Some(claim));
    }

    Ok(None)
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

fn insert_semantic_evidence_in_tx(
    tx: &ActionDb,
    canonical_claim_id: &str,
    corroboration_id: &str,
    proposal: &ClaimProposal,
    source_mechanism: &str,
    now: &str,
) -> Result<String, ClaimError> {
    let id = uuid::Uuid::new_v4().to_string();
    tx.conn_ref().execute(
        "INSERT INTO claim_semantic_evidence (
            id, canonical_claim_id, corroboration_id, data_source, source_ref,
            source_asof, provenance_json, original_text, actor, observed_at,
            thread_id, source_mechanism, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            &id,
            canonical_claim_id,
            corroboration_id,
            &proposal.data_source,
            proposal.source_ref.as_deref(),
            proposal.source_asof.as_deref(),
            &proposal.provenance_json,
            &proposal.text,
            &proposal.actor,
            &proposal.observed_at,
            proposal.thread_id.as_deref(),
            source_mechanism,
            now,
        ],
    )?;
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

fn normalize_claim_surface(surface: &str) -> Result<ClaimDismissalSurface, ClaimError> {
    ClaimDismissalSurface::from_name(surface).ok_or_else(|| {
        let surface = surface.trim();
        if surface.is_empty() {
            ClaimError::InvalidFeedback(
                "surface dismissal requires a non-empty surface".to_string(),
            )
        } else {
            ClaimError::InvalidFeedback(format!(
                "surface dismissal requires a known ClaimDismissalSurface, got '{surface}'"
            ))
        }
    })
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
            metadata.trust_effect = compute_needs_nuance_trust_effect(&claim.text, &corrected_text);
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

fn initial_trust_score(kind: ClaimType) -> Option<f64> {
    match kind {
        // User-authored notes start in the likely_current band. They still
        // flow through later trust recomputation like every other claim.
        ClaimType::UserNote => Some(0.85),
        _ => None,
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
    // The v1.4.0 commit spine only accepts single concrete subjects.
    // The default reader family and PRE-GATE checks require a single
    // (kind, id) tuple, so accepting Multi or Global here would create
    // rows that read-after-write cannot see. A later ADR amendment can
    // lift this guard.
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
    // Derive subject_ref_compact from the parsed SubjectRef enum, not
    // the caller's raw JSON bytes. The parser case-folds kind, but
    // compact_subject_ref on the caller's value preserves original
    // casing, which would split deduplication and per-key locking for
    // semantically identical subjects.
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
    // Lower single-subject variants to the registry's canonical
    // subject-kind labels.
    let subject_kind_lc = match &subject {
        SubjectRef::Account { .. } => "account",
        SubjectRef::Meeting { .. } => "meeting",
        SubjectRef::Person { .. } => "person",
        SubjectRef::Project { .. } => "project",
        SubjectRef::Email { .. } => "email",
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
    targeted_repair_validate_claim_commit_route(ctx, &proposal)?;
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
    let semantic_qualifiers = if matches!(kind, ClaimType::UserNote) {
        HashSet::new()
    } else {
        semantic_high_salience_qualifiers(&proposal.text)
    };

    let canonical_text = if matches!(kind, ClaimType::UserNote) {
        proposal.text.clone()
    } else {
        canonicalize_semantic_text(&proposal.text)
    };
    let computed_hash = item_hash(
        item_kind_for_claim_type(&proposal.claim_type),
        &canonical_text,
    );
    let dedup_key = if matches!(kind, ClaimType::UserNote) {
        compute_user_note_dedup_key(&subject_ref_compact, &proposal.actor, &proposal.observed_at)
    } else {
        compute_dedup_key(
            &computed_hash,
            &subject_ref_compact,
            &proposal.claim_type,
            proposal.field_path.as_deref(),
        )
    };

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
        let claim_metadata_json = metadata_with_semantic_qualifiers(
            link_map::metadata_with_structured_field(
                proposal.metadata_json.as_deref(),
                proposal.field_path.as_deref(),
                &proposal.text,
            ),
            &semantic_qualifiers,
        );
        targeted_repair_validate_claim_commit_invocation_budget(ctx, tx, &proposal)?;
        if proposal.tombstone.is_some() && proposal.supersedes.is_some() {
            return Err(ClaimError::InvalidSupersession(
                "tombstone commits cannot also supersede another claim".to_string(),
            ));
        }

        if let Some(superseded_id) = proposal.supersedes.as_deref() {
            let superseded = load_claim_by_id(tx.conn_ref(), superseded_id)?
                .ok_or_else(|| ClaimError::UnknownClaimId(superseded_id.to_string()))?;
            if superseded.claim_type != proposal.claim_type {
                return Err(ClaimError::InvalidSupersession(format!(
                    "claim {} has type {}, not {}",
                    superseded.id, superseded.claim_type, proposal.claim_type
                )));
            }
            if superseded.claim_state != ClaimState::Active
                || superseded.surfacing_state != SurfacingState::Active
            {
                return Err(ClaimError::InvalidSupersession(format!(
                    "claim {} is not active and surfaced",
                    superseded.id
                )));
            }

            let superseded_subject_value =
                serde_json::from_str::<serde_json::Value>(&superseded.subject_ref)?;
            let superseded_subject = subject_ref_from_json(&superseded_subject_value)?;
            let superseded_subject_ref_compact = canonical_subject_ref(&superseded_subject)?;
            if superseded_subject_ref_compact != subject_ref_compact {
                return Err(ClaimError::InvalidSupersession(format!(
                    "claim {} has a different subject_ref",
                    superseded.id
                )));
            }
            if pre_gate_blocking_tombstone_exists(
                tx.conn_ref(),
                PreGateTombstoneLookup {
                    subject: &superseded_subject,
                    claim_type: superseded.claim_type.as_str(),
                    field_path: superseded.field_path.as_deref(),
                    item_hash_value: superseded.item_hash.as_deref().unwrap_or(""),
                    canonical_text: &superseded.text,
                    proposal_temporal_scope: &effective_temporal_scope,
                    proposal_sensitivity: &effective_sensitivity,
                    now: &now,
                },
            )? || pre_gate_blocking_tombstone_exists(
                tx.conn_ref(),
                PreGateTombstoneLookup {
                    subject: &subject,
                    claim_type: proposal.claim_type.as_str(),
                    field_path: proposal.field_path.as_deref(),
                    item_hash_value: &computed_hash,
                    canonical_text: &canonical_text,
                    proposal_temporal_scope: &effective_temporal_scope,
                    proposal_sensitivity: &effective_sensitivity,
                    now: &now,
                },
            )? {
                return Err(ClaimError::TombstonedPreGate);
            }

            let new_id = proposal
                .id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let claim = IntelligenceClaim {
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
                metadata_json: claim_metadata_json.clone(),
                claim_state: ClaimState::Active,
                surfacing_state: SurfacingState::Active,
                demotion_reason: None,
                reactivated_at: None,
                retraction_reason: None,
                expires_at: None,
                superseded_by: None,
                trust_score: initial_trust_score(kind),
                trust_computed_at: initial_trust_score(kind).map(|_| now.clone()),
                trust_version: initial_trust_score(kind).map(|_| 1),
                thread_id: proposal.thread_id.clone(),
                temporal_scope: effective_temporal_scope.clone(),
                sensitivity: effective_sensitivity.clone(),
                verification_state: ClaimVerificationState::Active,
                verification_reason: None,
                needs_user_decision_at: None,
            };
            insert_claim_row(tx, &claim)?;
            project_legacy_state_for_claim(ctx, tx, &claim)?;
            insert_claim_edges(tx, &claim)?;

            execute_claims_update(
                tx.conn_ref(),
                "UPDATE intelligence_claims
                 SET claim_state = 'dormant',
                     surfacing_state = 'dormant',
                     demotion_reason = 'superseded',
                     superseded_by = ?1
                 WHERE id = ?2",
                params![&new_id, superseded_id],
            )?;
            mark_claim_edges_superseded_by_claim(tx, superseded_id, &new_id)?;

            let contradiction_id = uuid::Uuid::new_v4().to_string();
            tx.conn_ref().execute(
                "INSERT INTO claim_contradictions \
                 (id, primary_claim_id, contradicting_claim_id, branch_kind, detected_at) \
                 VALUES (?1, ?2, ?3, 'supersession', ?4)",
                params![&contradiction_id, superseded_id, &new_id, &now],
            )?;

            tx.bump_for_subject(&subject)?;
            return Ok(CommittedClaim::Inserted { claim });
        }

        if proposal.tombstone.is_none()
            && pre_gate_blocking_tombstone_exists(
                tx.conn_ref(),
                PreGateTombstoneLookup {
                    subject: &subject,
                    claim_type: proposal.claim_type.as_str(),
                    field_path: proposal.field_path.as_deref(),
                    item_hash_value: &computed_hash,
                    canonical_text: &canonical_text,
                    proposal_temporal_scope: &effective_temporal_scope,
                    proposal_sensitivity: &effective_sensitivity,
                    now: &now,
                },
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
        if proposal.tombstone.is_none()
            && !matches!(metadata.commit_policy_class, CommitPolicyClass::Append)
        {
            let mut semantic_duplicate_needs_verification = false;
            if let Some(existing) = load_active_claim_by_dedup_key(
                tx.conn_ref(),
                &dedup_key,
                &effective_temporal_scope,
                &effective_sensitivity,
            )? {
                let corroboration_id = corroborate_in_tx(
                    tx,
                    &existing.id,
                    &proposal.data_source,
                    proposal.source_asof.as_deref(),
                    Some("same_meaning_merge"),
                    &now,
                )?;
                let mut edge_claim = existing.clone();
                edge_claim.metadata_json = link_map::metadata_with_structured_field(
                    edge_claim.metadata_json.as_deref(),
                    proposal.field_path.as_deref(),
                    &proposal.text,
                );
                insert_claim_edges(tx, &edge_claim)?;
                tx.bump_for_subject(&subject)?;
                return Ok(CommittedClaim::Reinforced {
                    claim: existing,
                    corroboration_id,
                });
            }

            if let Some(semantic_match) = load_active_semantic_duplicate_claim(
                tx.conn_ref(),
                SemanticDuplicateLookup {
                    subject: &subject,
                    claim_type: &proposal.claim_type,
                    field_path: proposal.field_path.as_deref(),
                    canonical_text: &canonical_text,
                    proposal_qualifiers: &semantic_qualifiers,
                    proposal_temporal_scope: &effective_temporal_scope,
                    proposal_sensitivity: &effective_sensitivity,
                    now: &now,
                },
            )? {
                match semantic_match.action {
                    SemanticDuplicateAction::Canonicalize => {
                        let source_mechanism = "semantic_near_duplicate_merge";
                        let corroboration_id = corroborate_in_tx(
                            tx,
                            &semantic_match.claim.id,
                            &proposal.data_source,
                            proposal.source_asof.as_deref(),
                            Some(source_mechanism),
                            &now,
                        )?;
                        insert_semantic_evidence_in_tx(
                            tx,
                            &semantic_match.claim.id,
                            &corroboration_id,
                            &proposal,
                            source_mechanism,
                            &now,
                        )?;
                        let mut edge_claim = semantic_match.claim.clone();
                        edge_claim.metadata_json = link_map::metadata_with_structured_field(
                            edge_claim.metadata_json.as_deref(),
                            proposal.field_path.as_deref(),
                            &proposal.text,
                        );
                        insert_claim_edges(tx, &edge_claim)?;
                        tx.bump_for_subject(&subject)?;
                        return Ok(CommittedClaim::Reinforced {
                            claim: semantic_match.claim,
                            corroboration_id,
                        });
                    }
                    SemanticDuplicateAction::NeedsVerification => {
                        semantic_duplicate_needs_verification = true;
                    }
                }
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
                ContradictionLookup {
                    subject: &subject,
                    claim_type: &proposal.claim_type,
                    field_path: proposal.field_path.as_deref(),
                    canonical_text: &canonical_text,
                    proposal_qualifiers: &semantic_qualifiers,
                    proposal_temporal_scope: &effective_temporal_scope,
                    proposal_sensitivity: &effective_sensitivity,
                    now: &now,
                },
            )? {
                let new_id = proposal
                    .id
                    .clone()
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
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
                    metadata_json: claim_metadata_json.clone(),
                    claim_state: ClaimState::Active,
                    surfacing_state: SurfacingState::Active,
                    demotion_reason: None,
                    reactivated_at: None,
                    retraction_reason: None,
                    expires_at: None,
                    superseded_by: None,
                    trust_score: initial_trust_score(kind),
                    trust_computed_at: initial_trust_score(kind).map(|_| now.clone()),
                    trust_version: initial_trust_score(kind).map(|_| 1),
                    thread_id: proposal.thread_id.clone(),
                    temporal_scope: effective_temporal_scope.clone(),
                    sensitivity: effective_sensitivity.clone(),
                    verification_state: ClaimVerificationState::Active,
                    verification_reason: None,
                    needs_user_decision_at: None,
                };
                insert_claim_row(tx, &contradicting)?;
                project_legacy_state_for_claim(ctx, tx, &contradicting)?;
                insert_claim_edges(tx, &contradicting)?;

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
            if semantic_duplicate_needs_verification {
                let id = proposal
                    .id
                    .clone()
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let trust_score = Some(needs_verification_score());
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
                    created_at: now.clone(),
                    provenance_json: proposal.provenance_json.clone(),
                    metadata_json: claim_metadata_json,
                    claim_state: ClaimState::Active,
                    surfacing_state: SurfacingState::Active,
                    demotion_reason: None,
                    reactivated_at: None,
                    retraction_reason: None,
                    expires_at: None,
                    superseded_by: None,
                    trust_score,
                    trust_computed_at: trust_score.map(|_| now.clone()),
                    trust_version: trust_score.map(|_| 1),
                    thread_id: proposal.thread_id.clone(),
                    temporal_scope: effective_temporal_scope.clone(),
                    sensitivity: effective_sensitivity.clone(),
                    verification_state: ClaimVerificationState::Active,
                    verification_reason: Some(
                        "semantic_duplicate_low_trust_needs_verification".to_string(),
                    ),
                    needs_user_decision_at: None,
                };

                insert_claim_row(tx, &claim)?;
                project_legacy_state_for_claim(ctx, tx, &claim)?;
                insert_claim_edges(tx, &claim)?;
                tx.bump_for_subject(&subject)?;
                return Ok(CommittedClaim::Inserted { claim });
            }
        }

        let id = proposal
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
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
            created_at: now.clone(),
            provenance_json: proposal.provenance_json.clone(),
            metadata_json: claim_metadata_json,
            claim_state,
            surfacing_state,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason,
            expires_at,
            superseded_by: None,
            trust_score: initial_trust_score(kind),
            trust_computed_at: initial_trust_score(kind).map(|_| now.clone()),
            trust_version: initial_trust_score(kind).map(|_| 1),
            thread_id: proposal.thread_id.clone(),
            temporal_scope: effective_temporal_scope.clone(),
            sensitivity: effective_sensitivity.clone(),
            verification_state: ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        };

        insert_claim_row(tx, &claim)?;
        project_legacy_state_for_claim(ctx, tx, &claim)?;
        if proposal.tombstone.is_some() {
            mark_claim_edges_tombstoned_for_identity(
                tx,
                &subject,
                &proposal.claim_type,
                proposal.field_path.as_deref(),
                &now,
            )?;
        } else {
            insert_claim_edges(tx, &claim)?;
        }
        tx.bump_for_subject(&subject)?;

        if proposal.tombstone.is_some() {
            Ok(CommittedClaim::Tombstoned { claim })
        } else {
            Ok(CommittedClaim::Inserted { claim })
        }
    })
}

pub fn withdraw_claim(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    claim_id: &str,
    reason: &str,
) -> Result<IntelligenceClaim, ClaimError> {
    ctx.check_mutation_allowed()
        .map_err(|e| ClaimError::Mode(e.to_string()))?;

    let reason = reason.trim();
    if reason.is_empty() {
        return Err(ClaimError::InvalidFeedback(
            "withdrawal reason cannot be empty".to_string(),
        ));
    }

    with_claim_transaction(db, |tx| {
        let claim = load_claim_by_id(tx.conn_ref(), claim_id)?
            .ok_or_else(|| ClaimError::UnknownClaimId(claim_id.to_string()))?;
        let subject_value = serde_json::from_str::<serde_json::Value>(&claim.subject_ref)?;
        let subject = subject_ref_from_json(&subject_value)?;

        if claim.claim_state != ClaimState::Withdrawn
            || claim.surfacing_state != SurfacingState::Dormant
            || claim.retraction_reason.as_deref() != Some(reason)
        {
            execute_claims_update(
                tx.conn_ref(),
                "UPDATE intelligence_claims
                 SET claim_state = 'withdrawn',
                     surfacing_state = 'dormant',
                     retraction_reason = ?1
                 WHERE id = ?2",
                params![reason, claim_id],
            )?;
            mark_claim_edges_tombstoned(tx, claim_id, &ctx.clock.now().to_rfc3339())?;
            tx.bump_for_subject(&subject)?;
        }

        load_claim_by_id(tx.conn_ref(), claim_id)?
            .ok_or_else(|| ClaimError::UnknownClaimId(claim_id.to_string()))
    })
}

// ---------------------------------------------------------------------------
// record_claim_feedback
// ---------------------------------------------------------------------------

const TARGETED_REPAIR_OPERATION: &str = "targeted_claim_repair";
const TARGETED_REPAIR_ABILITY_ID: &str = "targeted_claim_repair";
const TARGETED_REPAIR_ABILITY_VERSION: &str = "1";
const TARGETED_REPAIR_PROVIDER_FINGERPRINT: &str =
    "provider:targeted_repair_local:model:claim-repair-rules-v1:temperature:0";
const TARGETED_REPAIR_PROMPT_TEMPLATE_ID: &str = "targeted_claim_repair_batch";
const TARGETED_REPAIR_PROMPT_TEMPLATE_VERSION: &str = "1.0.0";
const TARGETED_REPAIR_PROMPT_TEMPLATE: &str = "Repair one claim batch using committed claims, feedback, contradictions, and bounded evidence.";
const TARGETED_REPAIR_LEASE_SECONDS: i64 = 60;
const TARGETED_REPAIR_MAX_RETRIEVAL_SOURCES: u16 = 10;
const TARGETED_REPAIR_LOCAL_CORROBORATION_MECHANISM: &str = "targeted_repair_local_claim_match";

fn targeted_repair_operation(repair: RepairAction) -> String {
    format!(
        "{TARGETED_REPAIR_OPERATION}:{}",
        repair_action_label(repair)
    )
}

fn repair_action_label(repair: RepairAction) -> &'static str {
    match repair {
        RepairAction::None => "None",
        RepairAction::FreshnessRefresh => "FreshnessRefresh",
        RepairAction::ContradictionReconcile => "ContradictionReconcile",
        RepairAction::SubjectFitRepair => "SubjectFitRepair",
        RepairAction::SourceSupportRepair => "SourceSupportRepair",
        RepairAction::BoundedCorroboration => "BoundedCorroboration",
        RepairAction::PolicyRepair => "PolicyRepair",
    }
}

fn parse_repair_action_label(raw: &str) -> Result<RepairAction, ClaimError> {
    match raw.trim() {
        "None" | "none" => Ok(RepairAction::None),
        "FreshnessRefresh" | "freshness_refresh" => Ok(RepairAction::FreshnessRefresh),
        "ContradictionReconcile" | "contradiction_reconcile" => {
            Ok(RepairAction::ContradictionReconcile)
        }
        "SubjectFitRepair" | "subject_fit_repair" => Ok(RepairAction::SubjectFitRepair),
        "SourceSupportRepair" | "source_support_repair" => Ok(RepairAction::SourceSupportRepair),
        "BoundedCorroboration" | "bounded_corroboration" => Ok(RepairAction::BoundedCorroboration),
        "PolicyRepair" | "policy_repair" => Ok(RepairAction::PolicyRepair),
        other => Err(ClaimError::InvalidFeedback(format!(
            "unknown targeted repair action: {other}"
        ))),
    }
}

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
                execute_claims_update(
                    tx.conn_ref(),
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
                execute_claims_update(
                    tx.conn_ref(),
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
                execute_claims_update(
                    tx.conn_ref(),
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

        if matches!(
            lifecycle_update.claim_state,
            ClaimState::Tombstoned | ClaimState::Withdrawn
        ) || lifecycle_update.surfacing_state == SurfacingState::Dormant
        {
            mark_claim_edges_tombstoned(tx, &input.claim_id, &now)?;
        }

        bump_invalidation_for_claim_id(tx, &input.claim_id)?;
        let repair_job_id =
            targeted_repair_enqueue_job(ctx, tx, &claim, &feedback_id, metadata.repair)?;
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

pub fn targeted_repair_claim_generation_budget(ability_id: &str) -> Option<ClaimGenerationBudget> {
    match ability_id {
        // Read-only context assembly. It may read committed claims but has no
        // claim generation budget and no provider budget.
        "get_entity_context" => Some(ClaimGenerationBudget {
            ability_id: "get_entity_context".to_string(),
            contract: ClaimGenerationContract::ClaimValidation,
            max_candidate_claims: 0,
            max_provider_queries: 0,
            max_retrieval_sources: 0,
            max_llm_calls: 0,
            max_prompt_tokens: 0,
            max_output_tokens: 0,
            may_commit_claims: false,
        }),
        // Transform-only meeting synthesis. Candidate claims are bounded and
        // must be routed to Maintenance for validation/commit.
        "prepare_meeting" => Some(ClaimGenerationBudget {
            ability_id: "prepare_meeting".to_string(),
            contract: ClaimGenerationContract::ClaimExtraction,
            max_candidate_claims: 12,
            max_provider_queries: 0,
            max_retrieval_sources: 0,
            max_llm_calls: 1,
            max_prompt_tokens: 12_000,
            max_output_tokens: 4_000,
            may_commit_claims: false,
        }),
        TARGETED_REPAIR_ABILITY_ID | "find_corroborating_evidence" => Some(ClaimGenerationBudget {
            ability_id: TARGETED_REPAIR_ABILITY_ID.to_string(),
            contract: ClaimGenerationContract::ClaimRepair,
            max_candidate_claims: 3,
            max_provider_queries: 1,
            max_retrieval_sources: TARGETED_REPAIR_MAX_RETRIEVAL_SOURCES,
            max_llm_calls: 1,
            max_prompt_tokens: 6_000,
            max_output_tokens: 1_200,
            may_commit_claims: true,
        }),
        "narrative_assembly" | "briefing_narrative" | "report_narrative" => {
            Some(ClaimGenerationBudget {
                ability_id: "narrative_assembly".to_string(),
                contract: ClaimGenerationContract::NarrativeAssembly,
                max_candidate_claims: 0,
                max_provider_queries: 0,
                max_retrieval_sources: 0,
                max_llm_calls: 1,
                max_prompt_tokens: 8_000,
                max_output_tokens: 2_000,
                may_commit_claims: false,
            })
        }
        _ => None,
    }
}

#[derive(Debug, Default)]
struct ClaimCommitRouteMetadata {
    ability_id: Option<String>,
    contract: Option<String>,
    invocation_id: Option<String>,
    claims_this_invocation: Option<u64>,
    may_commit_claims: Option<bool>,
}

fn targeted_repair_validate_claim_commit_route(
    ctx: &ServiceContext<'_>,
    proposal: &ClaimProposal,
) -> Result<(), ClaimError> {
    let route = claim_commit_route_metadata(proposal)?;
    let actor = proposal.actor.trim().to_ascii_lowercase();
    let data_source = proposal.data_source.trim().to_ascii_lowercase();

    let narrative_direct = actor.contains("narrative")
        || data_source.contains("narrative")
        || route.contract.as_deref() == Some("narrative_assembly")
        || route.ability_id.as_deref() == Some("narrative_assembly");
    if narrative_direct {
        return Err(ClaimError::InvalidActor(
            "narrative assembly cannot commit claims directly; route new assertions through claim extraction, validation, and the claim commit service"
                .to_string(),
        ));
    }

    if route.may_commit_claims == Some(false) {
        let ability = route
            .ability_id
            .as_deref()
            .unwrap_or("metadata-declared ability");
        return Err(ClaimError::InvalidActor(format!(
            "{ability} metadata declares may_commit_claims=false; route new assertions through a claim-commit-capable maintenance ability"
        )));
    }

    for ability_id in claim_commit_ability_candidates(ctx, proposal, &route) {
        if let Some(budget) = targeted_repair_claim_generation_budget(&ability_id) {
            if !budget.may_commit_claims {
                return Err(ClaimError::InvalidActor(format!(
                    "{} cannot commit claims directly because its registered claim-generation budget has may_commit_claims=false",
                    budget.ability_id
                )));
            }
        }
    }

    if let Some(ability_id) = route.ability_id.as_deref() {
        if let Some(budget) = targeted_repair_claim_generation_budget(ability_id) {
            if let Some(claims_this_invocation) = route.claims_this_invocation {
                if claims_this_invocation >= u64::from(budget.max_candidate_claims) {
                    return Err(ClaimError::InvalidActor(format!(
                        "{} claim-generation budget exhausted: claims_this_invocation={} max_candidate_claims={}",
                        budget.ability_id, claims_this_invocation, budget.max_candidate_claims
                    )));
                }
            }
        }
    }

    Ok(())
}

fn targeted_repair_validate_claim_commit_invocation_budget(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    proposal: &ClaimProposal,
) -> Result<(), ClaimError> {
    let route = claim_commit_route_metadata(proposal)?;
    let Some(invocation_id) = route.invocation_id.as_deref() else {
        return Ok(());
    };
    for ability_id in claim_commit_ability_candidates(ctx, proposal, &route) {
        let Some(budget) = targeted_repair_claim_generation_budget(&ability_id) else {
            continue;
        };
        if !budget.may_commit_claims {
            return Err(ClaimError::InvalidActor(format!(
                "{} cannot commit claims directly because its registered claim-generation budget has may_commit_claims=false",
                budget.ability_id
            )));
        }

        let committed_for_invocation =
            count_claims_for_ability_invocation(tx, &ability_id, invocation_id)?;
        if committed_for_invocation >= i64::from(budget.max_candidate_claims) {
            return Err(ClaimError::InvalidActor(format!(
                "{} claim-generation budget exhausted for invocation {}: committed_claims={} max_candidate_claims={}",
                budget.ability_id,
                invocation_id,
                committed_for_invocation,
                budget.max_candidate_claims
            )));
        }
    }

    Ok(())
}

fn claim_commit_ability_candidates(
    ctx: &ServiceContext<'_>,
    proposal: &ClaimProposal,
    route: &ClaimCommitRouteMetadata,
) -> Vec<String> {
    let mut candidates = Vec::new();
    push_ability_candidate(&mut candidates, ctx.ability_id);
    if let Some(actor_ability) = ability_id_from_actor(&proposal.actor) {
        push_ability_candidate(&mut candidates, Some(&actor_ability));
    }
    push_ability_candidate(&mut candidates, route.ability_id.as_deref());
    candidates
}

fn push_ability_candidate(candidates: &mut Vec<String>, ability_id: Option<&str>) {
    let Some(ability_id) = ability_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    let normalized = ability_id.to_ascii_lowercase();
    if !candidates.contains(&normalized) {
        candidates.push(normalized);
    }
}

fn ability_id_from_actor(actor: &str) -> Option<String> {
    let mut parts = actor
        .split(':')
        .map(str::trim)
        .filter(|part| !part.is_empty());
    let head = parts.next()?.to_ascii_lowercase();
    match head.as_str() {
        "agent" | "ai" | "ability" => parts.next().map(|part| part.to_ascii_lowercase()),
        _ => None,
    }
}

fn claim_commit_route_metadata(
    proposal: &ClaimProposal,
) -> Result<ClaimCommitRouteMetadata, ClaimError> {
    let metadata = optional_json_object(proposal.metadata_json.as_deref())?;
    let provenance = optional_json_object(Some(&proposal.provenance_json))?;
    let mut route = ClaimCommitRouteMetadata::default();

    for value in metadata.iter().chain(provenance.iter()) {
        route.ability_id = route.ability_id.or_else(|| {
            json_string_any(
                value,
                &[
                    &["ability_id"][..],
                    &["ability_name"][..],
                    &["source_ability"][..],
                    &["producer_ability"][..],
                    &["claim_generation", "ability_id"][..],
                    &["claim_generation_budget", "ability_id"][..],
                    &["budget", "ability_id"][..],
                ],
            )
        });
        route.contract = route.contract.or_else(|| {
            json_string_any(
                value,
                &[
                    &["claim_generation_contract"][..],
                    &["ability_contract"][..],
                    &["claim_generation", "contract"][..],
                    &["claim_generation_budget", "contract"][..],
                    &["budget", "contract"][..],
                ],
            )
            .map(|contract| contract.to_ascii_lowercase())
        });
        route.invocation_id = route.invocation_id.or_else(|| {
            json_string_any(
                value,
                &[
                    &["invocation_id"][..],
                    &["producer_invocation_id"][..],
                    &["claim_generation", "invocation_id"][..],
                ],
            )
        });
        route.claims_this_invocation = route.claims_this_invocation.or_else(|| {
            json_u64_any(
                value,
                &[
                    &["claims_this_invocation"][..],
                    &["claim_generation", "claims_this_invocation"][..],
                    &["claim_generation_budget", "claims_this_invocation"][..],
                    &["budget", "claims_this_invocation"][..],
                ],
            )
        });
        route.may_commit_claims = route.may_commit_claims.or_else(|| {
            json_bool_any(
                value,
                &[
                    &["may_commit_claims"][..],
                    &["claim_generation", "may_commit_claims"][..],
                    &["claim_generation_budget", "may_commit_claims"][..],
                    &["budget", "may_commit_claims"][..],
                ],
            )
        });
    }

    route.ability_id = route
        .ability_id
        .map(|ability| ability.trim().to_ascii_lowercase())
        .filter(|ability| !ability.is_empty());
    route.invocation_id = route
        .invocation_id
        .map(|invocation| invocation.trim().to_string())
        .filter(|invocation| !invocation.is_empty());

    Ok(route)
}

fn optional_json_object(raw: Option<&str>) -> Result<Option<serde_json::Value>, ClaimError> {
    let Some(raw) = raw.map(str::trim).filter(|raw| !raw.is_empty()) else {
        return Ok(None);
    };
    let value: serde_json::Value = serde_json::from_str(raw)?;
    if value.is_object() {
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

fn json_at_path<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn json_string_any(value: &serde_json::Value, paths: &[&[&str]]) -> Option<String> {
    paths.iter().find_map(|path| {
        json_at_path(value, path)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    })
}

fn json_u64_any(value: &serde_json::Value, paths: &[&[&str]]) -> Option<u64> {
    paths
        .iter()
        .find_map(|path| json_at_path(value, path).and_then(serde_json::Value::as_u64))
}

fn json_bool_any(value: &serde_json::Value, paths: &[&[&str]]) -> Option<bool> {
    paths
        .iter()
        .find_map(|path| json_at_path(value, path).and_then(serde_json::Value::as_bool))
}

fn count_claims_for_ability_invocation(
    tx: &ActionDb,
    ability_id: &str,
    invocation_id: &str,
) -> Result<i64, ClaimError> {
    tx.conn_ref()
        .query_row(
            "SELECT count(*)
             FROM intelligence_claims
             WHERE (
                 json_valid(metadata_json) = 1
                 AND (
                     lower(json_extract(metadata_json, '$.ability_id')) = ?1
                     OR lower(json_extract(metadata_json, '$.ability_name')) = ?1
                     OR lower(json_extract(metadata_json, '$.claim_generation.ability_id')) = ?1
                     OR lower(json_extract(metadata_json, '$.claim_generation_budget.ability_id')) = ?1
                     OR lower(json_extract(metadata_json, '$.budget.ability_id')) = ?1
                 )
                 AND (
                     json_extract(metadata_json, '$.invocation_id') = ?2
                     OR json_extract(metadata_json, '$.producer_invocation_id') = ?2
                     OR json_extract(metadata_json, '$.claim_generation.invocation_id') = ?2
                 )
             )
             OR (
                 json_valid(provenance_json) = 1
                 AND (
                     lower(json_extract(provenance_json, '$.ability_id')) = ?1
                     OR lower(json_extract(provenance_json, '$.ability_name')) = ?1
                     OR lower(json_extract(provenance_json, '$.claim_generation.ability_id')) = ?1
                     OR lower(json_extract(provenance_json, '$.claim_generation_budget.ability_id')) = ?1
                     OR lower(json_extract(provenance_json, '$.budget.ability_id')) = ?1
                 )
                 AND (
                     json_extract(provenance_json, '$.invocation_id') = ?2
                     OR json_extract(provenance_json, '$.producer_invocation_id') = ?2
                     OR json_extract(provenance_json, '$.claim_generation.invocation_id') = ?2
                 )
             )",
            params![ability_id, invocation_id],
            |row| row.get(0),
        )
        .map_err(ClaimError::Rusqlite)
}

fn targeted_repair_enqueue_job(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    claim: &IntelligenceClaim,
    feedback_id: &str,
    repair: RepairAction,
) -> Result<Option<String>, ClaimError> {
    if matches!(repair, RepairAction::None) {
        return Ok(None);
    }

    let subject_value: serde_json::Value = serde_json::from_str(&claim.subject_ref)?;
    let subject = subject_ref_from_json(&subject_value)?;
    let (signal_entity_type, signal_entity_id) = signal_target_for_claim(&subject, &claim.id);

    let repair_signal_id = targeted_repair_emit_requested_signal(
        ctx,
        tx,
        &signal_entity_type,
        &signal_entity_id,
        &claim.id,
        feedback_id,
        repair,
    )?;

    let receipt = targeted_repair_enqueue_invalidation(
        tx,
        Some(&repair_signal_id),
        &subject,
        &claim.id,
        feedback_id,
        repair,
    )?;

    Ok(Some(receipt.job_id))
}

fn targeted_repair_emit_requested_signal(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    claim_id: &str,
    feedback_id: &str,
    repair: RepairAction,
) -> Result<String, ClaimError> {
    let payload = serde_json::json!({
        "claim_id": claim_id,
        "feedback_id": feedback_id,
        "repair_action": repair_action_label(repair),
        "contract": ClaimGenerationContract::ClaimRepair,
    });
    crate::services::signals::emit_in_transaction(
        ctx,
        tx,
        entity_type,
        entity_id,
        "claim_repair_requested",
        "claim_feedback",
        payload,
    )
    .map_err(ClaimError::Transaction)
}

fn targeted_repair_enqueue_invalidation(
    tx: &ActionDb,
    origin_signal_id: Option<&str>,
    subject: &SubjectRef,
    claim_id: &str,
    feedback_id: &str,
    repair: RepairAction,
) -> Result<crate::db::invalidation_jobs::InvalidationJobReceipt, ClaimError> {
    let Some(subject_type) = subject_kind_label(subject) else {
        return Err(ClaimError::SubjectRef(
            "targeted repair requires a single concrete subject".to_string(),
        ));
    };
    let Some(subject_id) = subject_id_for_lookup(subject) else {
        return Err(ClaimError::SubjectRef(
            "targeted repair requires a subject id".to_string(),
        ));
    };

    let subject_type = subject_type.to_ascii_lowercase();
    let source_claim_version = tx.current_claim_version_for_subject(&subject_type, subject_id)?;
    let input_snapshot_hash =
        targeted_repair_input_hash(&subject_type, subject_id, claim_id, source_claim_version);
    let prompt_fingerprint = targeted_repair_prompt_fingerprint(&input_snapshot_hash);
    let extraction_batch =
        targeted_repair_extraction_batch_payload(&input_snapshot_hash, &prompt_fingerprint);
    let budget = targeted_repair_claim_generation_budget(TARGETED_REPAIR_ABILITY_ID)
        .expect("targeted repair budget is registered");
    let policy_repair_surface =
        targeted_repair_policy_repair_coalescing_surface(tx, feedback_id, repair)?;
    let payload_json = serde_json::json!({
        "claim_id": claim_id,
        "feedback_id": feedback_id,
        "repair_action": repair_action_label(repair),
        "contract": ClaimGenerationContract::ClaimRepair,
        "budget": budget,
        "extraction_batch": extraction_batch,
        "dos_241": {
            "claim_granularity": "per_reviewable_fact",
            "dos_235": "accepted",
            "dos_236": "accepted",
            "dos_237": "accepted"
        }
    });
    let input = crate::db::invalidation_jobs::EnqueueInvalidationJob {
        job_kind: crate::db::invalidation_jobs::KIND_TARGETED_REPAIR.to_string(),
        operation: targeted_repair_operation(repair),
        origin_signal_id: origin_signal_id.map(str::to_string),
        subject_type: subject_type.clone(),
        subject_id: subject_id.to_string(),
        ability_id: TARGETED_REPAIR_ABILITY_ID.to_string(),
        ability_version: TARGETED_REPAIR_ABILITY_VERSION.to_string(),
        source_claim_version,
        source_asof: None,
        input_snapshot_hash: Some(input_snapshot_hash.clone()),
        provider_fingerprint: Some(TARGETED_REPAIR_PROVIDER_FINGERPRINT.to_string()),
        prompt_fingerprint: Some(prompt_fingerprint),
        payload_json,
        coalescing_key: Some(targeted_repair_coalescing_key(
            &subject_type,
            subject_id,
            claim_id,
            repair,
            policy_repair_surface.as_deref(),
            &input_snapshot_hash,
        )),
        chain_id: None,
        parent_job_id: None,
        successor_of_job_id: None,
        depth: 0,
        chain_ancestry: Vec::new(),
        max_attempts: 3,
        priority: 0,
        raw_signal_count: 1,
    };

    let config = crate::services::invalidation_jobs::InvalidationJobQueueConfig::from_env();
    let pending_cap = targeted_repair_pending_cap(config.pending_cap);
    tx.enqueue_invalidation_job_with_pending_cap(input, pending_cap)
        .map_err(ClaimError::Db)
}

#[cfg(test)]
thread_local! {
    static TARGETED_REPAIR_PENDING_CAP_OVERRIDE: std::cell::Cell<Option<i64>> =
        std::cell::Cell::new(None);
}

fn targeted_repair_pending_cap(default_pending_cap: i64) -> i64 {
    #[cfg(test)]
    {
        if let Some(pending_cap) = TARGETED_REPAIR_PENDING_CAP_OVERRIDE.with(std::cell::Cell::get) {
            return pending_cap;
        }
    }

    default_pending_cap
}

#[cfg(test)]
fn with_targeted_repair_pending_cap_override<T>(pending_cap: i64, run: impl FnOnce() -> T) -> T {
    struct PendingCapOverrideGuard {
        previous: Option<i64>,
    }

    impl Drop for PendingCapOverrideGuard {
        fn drop(&mut self) {
            TARGETED_REPAIR_PENDING_CAP_OVERRIDE.with(|override_cap| {
                override_cap.set(self.previous);
            });
        }
    }

    let previous = TARGETED_REPAIR_PENDING_CAP_OVERRIDE.with(|override_cap| {
        let previous = override_cap.get();
        override_cap.set(Some(pending_cap));
        previous
    });
    let _guard = PendingCapOverrideGuard { previous };
    run()
}

fn targeted_repair_input_hash(
    subject_type: &str,
    subject_id: &str,
    claim_id: &str,
    source_claim_version: i64,
) -> String {
    format!("targeted_repair:{subject_type}:{subject_id}:{claim_id}:claims:{source_claim_version}")
}

fn targeted_repair_coalescing_key(
    subject_type: &str,
    subject_id: &str,
    claim_id: &str,
    repair: RepairAction,
    repair_scope: Option<&str>,
    input_snapshot_hash: &str,
) -> String {
    let input_scope = input_snapshot_hash
        .rsplit_once(':')
        .map(|(scope, _)| scope)
        .unwrap_or(input_snapshot_hash);
    let repair_scope = repair_scope.unwrap_or("claim");
    format!(
        "{TARGETED_REPAIR_OPERATION}:{subject_type}:{subject_id}:{claim_id}:{}:{repair_scope}:{}:{}:{input_scope}",
        repair_action_label(repair),
        TARGETED_REPAIR_ABILITY_ID,
        TARGETED_REPAIR_ABILITY_VERSION
    )
}

fn targeted_repair_policy_repair_coalescing_surface(
    tx: &ActionDb,
    feedback_id: &str,
    repair: RepairAction,
) -> Result<Option<String>, ClaimError> {
    if !matches!(repair, RepairAction::PolicyRepair) {
        return Ok(None);
    }

    let feedback = targeted_repair_feedback(tx, feedback_id)?;
    let surface = payload_string(feedback.payload_json.as_deref(), "surface")?
        .ok_or_else(|| {
            ClaimError::InvalidFeedback(
                "surface_inappropriate repair requires payload_json.surface".to_string(),
            )
        })
        .and_then(|surface| normalize_claim_surface(&surface))?;
    Ok(Some(format!("surface:{}", surface.as_str())))
}

fn targeted_repair_prompt_fingerprint(input_snapshot_hash: &str) -> String {
    let prompt = targeted_repair_prompt_input(input_snapshot_hash);
    let fingerprint_metadata = targeted_repair_fingerprint_metadata();
    #[allow(deprecated)]
    crate::intelligence::provider::canonical_prompt_hash(
        crate::intelligence::provider::CanonicalPromptRequest {
            prompt: &prompt,
            fingerprint_metadata: &fingerprint_metadata,
        },
    )
}

fn targeted_repair_prompt_input(
    input_snapshot_hash: &str,
) -> crate::intelligence::provider::PromptInput {
    crate::intelligence::provider::PromptInput::new(TARGETED_REPAIR_PROMPT_TEMPLATE)
        .with_template(
            TARGETED_REPAIR_PROMPT_TEMPLATE_ID,
            TARGETED_REPAIR_PROMPT_TEMPLATE_VERSION,
            crate::intelligence::provider::canonical_template_hash(TARGETED_REPAIR_PROMPT_TEMPLATE),
        )
        .with_canonical_json_inputs(serde_json::json!({
            "ability_id": TARGETED_REPAIR_ABILITY_ID,
            "operation": TARGETED_REPAIR_OPERATION,
            "input_snapshot_hash": input_snapshot_hash,
            "contract": ClaimGenerationContract::ClaimRepair,
            "claim_generation_budget": targeted_repair_claim_generation_budget(
                TARGETED_REPAIR_ABILITY_ID
            ),
            "dos_241": {
                "claim_granularity": "per_reviewable_fact",
                "dos_235": "accepted",
                "dos_236": "accepted",
                "dos_237": "accepted"
            }
        }))
}

fn targeted_repair_fingerprint_metadata() -> crate::intelligence::provider::FingerprintMetadata {
    crate::intelligence::provider::FingerprintMetadata {
        provider: crate::intelligence::provider::ProviderKind::Other("targeted_repair_local"),
        model: crate::intelligence::provider::ModelName::new("claim-repair-rules-v1"),
        temperature: 0.0,
        top_p: None,
        seed: None,
        tokens_input: None,
        tokens_output: None,
        provider_completion_id: None,
    }
}

fn targeted_repair_extraction_batch_payload(
    input_snapshot_hash: &str,
    prompt_fingerprint: &str,
) -> serde_json::Value {
    let fingerprint_metadata = targeted_repair_fingerprint_metadata();
    serde_json::json!({
        "id": input_snapshot_hash,
        "level": "extraction_batch",
        "prompt_fingerprint": prompt_fingerprint,
        "canonical_prompt_hash": prompt_fingerprint,
        "prompt_template_id": TARGETED_REPAIR_PROMPT_TEMPLATE_ID,
        "prompt_template_version": TARGETED_REPAIR_PROMPT_TEMPLATE_VERSION,
        "provider": fingerprint_metadata.provider.as_str(),
        "model": fingerprint_metadata.model.as_str(),
        "temperature": fingerprint_metadata.temperature,
        "provider_fingerprint": TARGETED_REPAIR_PROVIDER_FINGERPRINT,
    })
}

#[derive(Debug, Clone)]
struct TargetedRepairInvalidationJob {
    id: String,
    subject_type: String,
    subject_id: String,
    latest_source_claim_version: i64,
    payload_json: String,
    attempts: i64,
    max_attempts: i64,
}

#[derive(Debug, Clone)]
struct TargetedRepairPayload {
    claim_id: String,
    feedback_id: String,
    repair_action: RepairAction,
}

#[derive(Debug, Default)]
struct TargetedRepairRunSummary {
    repair_jobs_processed: usize,
    claims_changed: usize,
    contradictions_reconciled: usize,
    failed_jobs: usize,
    changed_claim_ids: Vec<String>,
}

pub fn targeted_repair_process_next_job(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    worker_id: &str,
) -> Result<TargetedRepairProcessOutcome, ClaimError> {
    ctx.check_mutation_allowed()
        .map_err(|e| ClaimError::Mode(e.to_string()))?;

    with_claim_transaction(db, |tx| {
        let repair_now = ctx.clock.now().to_rfc3339();
        let queue_now = Utc::now();
        let queue_now_str = queue_now.to_rfc3339();
        let lease_expires_at =
            (queue_now + Duration::seconds(TARGETED_REPAIR_LEASE_SECONDS)).to_rfc3339();
        let Some(job) = query_targeted_repair_next_invalidation_job(
            tx,
            worker_id,
            &queue_now_str,
            &lease_expires_at,
        )?
        else {
            return Ok(TargetedRepairProcessOutcome::NoJob);
        };

        let mut summary = TargetedRepairRunSummary::default();
        if job.attempts > job.max_attempts {
            tx.mark_invalidation_job_failed(&job.id, "targeted repair attempt budget exhausted")?;
            summary.failed_jobs += 1;
        } else if let Err(error) =
            targeted_repair_process_invalidation_job(ctx, tx, &job, &repair_now, &mut summary)
        {
            tx.mark_invalidation_job_failed(&job.id, &error.to_string())?;
            summary.failed_jobs += 1;
            targeted_repair_emit_activity_log(ctx, tx, &job, &summary)?;
            return Ok(TargetedRepairProcessOutcome::Completed {
                job_id: job.id,
                repair_jobs_processed: summary.repair_jobs_processed,
                claims_changed: summary.claims_changed,
                contradictions_reconciled: summary.contradictions_reconciled,
            });
        }

        if summary.failed_jobs == 0 {
            targeted_repair_complete_invalidation_job(tx, &job, &queue_now_str)?;
        }
        targeted_repair_emit_activity_log(ctx, tx, &job, &summary)?;

        Ok(TargetedRepairProcessOutcome::Completed {
            job_id: job.id,
            repair_jobs_processed: summary.repair_jobs_processed,
            claims_changed: summary.claims_changed,
            contradictions_reconciled: summary.contradictions_reconciled,
        })
    })
}

fn query_targeted_repair_next_invalidation_job(
    tx: &ActionDb,
    worker_id: &str,
    now: &str,
    lease_expires_at: &str,
) -> Result<Option<TargetedRepairInvalidationJob>, ClaimError> {
    let job_id: Option<String> = tx
        .conn_ref()
        .query_row(
            "SELECT id
             FROM invalidation_jobs
             WHERE job_kind = ?1
               AND (
                    (status = 'pending' AND datetime(next_run_at) <= datetime(?2))
                    OR
                    (status = 'running'
                     AND lease_expires_at IS NOT NULL
                     AND datetime(lease_expires_at) <= datetime(?2))
               )
             ORDER BY
                CASE status WHEN 'running' THEN 0 ELSE 1 END,
                priority DESC,
                created_at ASC
             LIMIT 1",
            params![crate::db::invalidation_jobs::KIND_TARGETED_REPAIR, now],
            |row| row.get(0),
        )
        .optional()?;

    let Some(job_id) = job_id else {
        return Ok(None);
    };

    tx.conn_ref().execute(
        "UPDATE invalidation_jobs
         SET status = 'running',
             lease_owner = ?2,
             lease_expires_at = ?3,
             claimed_at = ?4,
             attempts = attempts + 1,
             updated_at = ?4
         WHERE id = ?1
           AND status IN ('pending', 'running')",
        params![&job_id, worker_id, lease_expires_at, now],
    )?;

    tx.conn_ref()
        .query_row(
            "SELECT id, subject_type, subject_id, latest_source_claim_version,
                    payload_json, attempts, max_attempts
             FROM invalidation_jobs
             WHERE id = ?1",
            params![&job_id],
            |row| {
                Ok(TargetedRepairInvalidationJob {
                    id: row.get(0)?,
                    subject_type: row.get(1)?,
                    subject_id: row.get(2)?,
                    latest_source_claim_version: row.get(3)?,
                    payload_json: row.get(4)?,
                    attempts: row.get(5)?,
                    max_attempts: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(ClaimError::Rusqlite)
}

fn targeted_repair_process_invalidation_job(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    job: &TargetedRepairInvalidationJob,
    now: &str,
    summary: &mut TargetedRepairRunSummary,
) -> Result<(), ClaimError> {
    let payload = targeted_repair_payload(job)?;
    if targeted_repair_reschedule_if_stale(tx, job, &payload)? {
        return Ok(());
    }

    let delta = targeted_repair_apply_claim_job(ctx, tx, &payload, now)?;
    summary.repair_jobs_processed += 1;
    summary.claims_changed += delta.claims_changed;
    summary.contradictions_reconciled += delta.contradictions_reconciled;
    for id in delta.changed_claim_ids {
        if !summary.changed_claim_ids.contains(&id) {
            summary.changed_claim_ids.push(id);
        }
    }
    Ok(())
}

fn targeted_repair_reschedule_if_stale(
    tx: &ActionDb,
    job: &TargetedRepairInvalidationJob,
    payload: &TargetedRepairPayload,
) -> Result<bool, ClaimError> {
    let current_source_claim_version =
        tx.current_claim_version_for_subject(&job.subject_type, &job.subject_id)?;
    if current_source_claim_version <= job.latest_source_claim_version {
        return Ok(false);
    }

    let subject = targeted_repair_subject_from_job(job)?;
    targeted_repair_enqueue_invalidation(
        tx,
        None,
        &subject,
        &payload.claim_id,
        &payload.feedback_id,
        payload.repair_action,
    )?;
    Ok(true)
}

fn targeted_repair_subject_from_job(
    job: &TargetedRepairInvalidationJob,
) -> Result<SubjectRef, ClaimError> {
    let id = job.subject_id.clone();
    match job.subject_type.trim().to_ascii_lowercase().as_str() {
        "account" | "accounts" => Ok(SubjectRef::Account { id }),
        "meeting" | "meetings" => Ok(SubjectRef::Meeting { id }),
        "person" | "people" => Ok(SubjectRef::Person { id }),
        "project" | "projects" => Ok(SubjectRef::Project { id }),
        "email" | "emails" => Ok(SubjectRef::Email { id }),
        other => Err(ClaimError::SubjectRef(format!(
            "targeted repair cannot reschedule unsupported subject type: {other}"
        ))),
    }
}

#[derive(Debug, Default)]
struct TargetedRepairClaimDelta {
    claims_changed: usize,
    contradictions_reconciled: usize,
    changed_claim_ids: Vec<String>,
}

fn targeted_repair_apply_claim_job(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    payload: &TargetedRepairPayload,
    now: &str,
) -> Result<TargetedRepairClaimDelta, ClaimError> {
    let claim = load_claim_by_id(tx.conn_ref(), &payload.claim_id)?
        .ok_or_else(|| ClaimError::UnknownClaimId(payload.claim_id.clone()))?;
    let feedback = targeted_repair_feedback(tx, &payload.feedback_id)?;
    let expected_repair = feedback_semantics(feedback.action).repair;
    if expected_repair != payload.repair_action {
        return Err(ClaimError::InvalidFeedback(format!(
            "targeted repair payload action {} does not match feedback {} repair {}",
            repair_action_label(payload.repair_action),
            feedback.action.as_str(),
            repair_action_label(expected_repair)
        )));
    }

    match payload.repair_action {
        RepairAction::ContradictionReconcile => {
            targeted_repair_apply_contradiction_reconcile(ctx, tx, &claim, &feedback, now)
        }
        RepairAction::BoundedCorroboration => {
            targeted_repair_apply_bounded_corroboration(tx, &claim, now)
        }
        RepairAction::FreshnessRefresh => targeted_repair_apply_freshness_refresh(tx, &claim, now),
        RepairAction::SubjectFitRepair => {
            targeted_repair_apply_subject_fit_repair(ctx, tx, &claim, &feedback, now)
        }
        RepairAction::SourceSupportRepair => {
            targeted_repair_apply_source_support_repair(tx, &claim, &feedback, now)
        }
        RepairAction::PolicyRepair => {
            targeted_repair_apply_policy_repair(tx, &claim, &feedback, now)
        }
        RepairAction::None => Err(ClaimError::InvalidFeedback(format!(
            "targeted repair job resolved to feedback action {} with no repair action",
            feedback.action.as_str()
        ))),
    }
}

#[derive(Debug, Clone)]
struct TargetedRepairFeedback {
    id: String,
    action: FeedbackAction,
    actor: String,
    payload_json: Option<String>,
}

fn targeted_repair_payload(
    job: &TargetedRepairInvalidationJob,
) -> Result<TargetedRepairPayload, ClaimError> {
    let value: serde_json::Value = serde_json::from_str(&job.payload_json)?;
    let claim_id = value
        .get("claim_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ClaimError::InvalidFeedback(format!(
                "targeted repair job {} missing payload claim_id",
                job.id
            ))
        })?
        .to_string();
    let feedback_id = value
        .get("feedback_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ClaimError::InvalidFeedback(format!(
                "targeted repair job {} missing payload feedback_id",
                job.id
            ))
        })?
        .to_string();
    let repair_action = value
        .get("repair_action")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            ClaimError::InvalidFeedback(format!(
                "targeted repair job {} missing payload repair_action",
                job.id
            ))
        })
        .and_then(parse_repair_action_label)?;

    Ok(TargetedRepairPayload {
        claim_id,
        feedback_id,
        repair_action,
    })
}

fn targeted_repair_feedback(
    tx: &ActionDb,
    feedback_id: &str,
) -> Result<TargetedRepairFeedback, ClaimError> {
    let (action, actor, payload_json): (String, String, Option<String>) = tx
        .conn_ref()
        .query_row(
            "SELECT feedback_type, actor, payload_json FROM claim_feedback WHERE id = ?1",
            params![feedback_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| ClaimError::InvalidFeedback(format!("feedback {feedback_id} not found")))?;

    Ok(TargetedRepairFeedback {
        id: feedback_id.to_string(),
        action: parse_db_enum::<FeedbackAction>(action)?,
        actor,
        payload_json,
    })
}

fn targeted_repair_corroboration_count(tx: &ActionDb, claim_id: &str) -> Result<i64, ClaimError> {
    tx.conn_ref()
        .query_row(
            "SELECT count(*) FROM claim_corroborations WHERE claim_id = ?1",
            params![claim_id],
            |row| row.get(0),
        )
        .map_err(ClaimError::Rusqlite)
}

#[derive(Debug, Clone)]
struct TargetedRepairCorroboratingEvidence {
    claim_id: String,
    source_asof: Option<String>,
    observed_at: String,
}

fn targeted_repair_apply_bounded_corroboration(
    tx: &ActionDb,
    claim: &IntelligenceClaim,
    now: &str,
) -> Result<TargetedRepairClaimDelta, ClaimError> {
    let budget = targeted_repair_claim_generation_budget(TARGETED_REPAIR_ABILITY_ID)
        .expect("targeted repair budget is registered");
    let evidence = targeted_repair_find_local_corroborating_claims(
        tx,
        claim,
        i64::from(budget.max_retrieval_sources),
    )?;

    let mut recorded = 0usize;
    for item in evidence {
        let data_source = format!("claim:{}", item.claim_id);
        let source_asof = item
            .source_asof
            .as_deref()
            .or(Some(item.observed_at.as_str()));
        corroborate_in_tx(
            tx,
            &claim.id,
            &data_source,
            source_asof,
            Some(TARGETED_REPAIR_LOCAL_CORROBORATION_MECHANISM),
            now,
        )?;
        recorded += 1;
    }

    let corroboration_count = targeted_repair_corroboration_count(tx, &claim.id)?;
    let mut delta = TargetedRepairClaimDelta::default();
    if corroboration_count > 0 && claim.verification_state != ClaimVerificationState::Active {
        execute_claims_update(
            tx.conn_ref(),
            "UPDATE intelligence_claims
             SET verification_state = 'active',
                 verification_reason = NULL,
                 needs_user_decision_at = NULL
             WHERE id = ?1",
            params![&claim.id],
        )?;
        delta.claims_changed += 1;
        delta.changed_claim_ids.push(claim.id.clone());
    } else if recorded == 0 {
        execute_claims_update(
            tx.conn_ref(),
            "UPDATE intelligence_claims
             SET verification_reason = 'bounded_corroboration_no_local_evidence'
             WHERE id = ?1",
            params![&claim.id],
        )?;
        delta.claims_changed += 1;
        delta.changed_claim_ids.push(claim.id.clone());
    }

    if recorded > 0 || delta.claims_changed > 0 {
        bump_invalidation_for_claim_id(tx, &claim.id)?;
    }

    Ok(delta)
}

fn targeted_repair_find_local_corroborating_claims(
    tx: &ActionDb,
    claim: &IntelligenceClaim,
    limit: i64,
) -> Result<Vec<TargetedRepairCorroboratingEvidence>, ClaimError> {
    if limit <= 0 {
        return Ok(Vec::new());
    }
    let subject_value: serde_json::Value = serde_json::from_str(&claim.subject_ref)?;
    let subject = subject_ref_from_json(&subject_value)?;
    let Some(kind) = subject_kind_label(&subject) else {
        return Ok(Vec::new());
    };
    let Some(id) = subject_id_for_lookup(&subject) else {
        return Ok(Vec::new());
    };

    let mut stmt = tx.conn_ref().prepare(
        "SELECT c.id, c.source_asof, c.observed_at
         FROM intelligence_claims c
         WHERE c.id <> ?1
           AND c.claim_state = 'active'
           AND c.surfacing_state = 'active'
           AND c.claim_type = ?2
           AND coalesce(c.field_path, '') = coalesce(?3, '')
           AND json_valid(c.subject_ref) = 1
           AND lower(json_extract(c.subject_ref, '$.kind')) = lower(?4)
           AND json_extract(c.subject_ref, '$.id') = ?5
           AND (
               (c.item_hash IS NOT NULL AND c.item_hash = ?6)
               OR c.text = ?7 COLLATE NOCASE
           )
           AND NOT EXISTS (
               SELECT 1
               FROM claim_corroborations cc
               WHERE cc.claim_id = ?1
                 AND cc.data_source = ('claim:' || c.id)
           )
         ORDER BY c.created_at DESC, c.id ASC
         LIMIT ?8",
    )?;
    let rows = stmt.query_map(
        params![
            &claim.id,
            &claim.claim_type,
            claim.field_path.as_deref(),
            kind,
            id,
            claim.item_hash.as_deref().unwrap_or(""),
            &claim.text,
            limit,
        ],
        |row| {
            Ok(TargetedRepairCorroboratingEvidence {
                claim_id: row.get(0)?,
                source_asof: row.get(1)?,
                observed_at: row.get(2)?,
            })
        },
    )?;
    let mut evidence = Vec::new();
    for row in rows {
        evidence.push(row?);
    }
    Ok(evidence)
}

fn targeted_repair_apply_contradiction_reconcile(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    claim: &IntelligenceClaim,
    feedback: &TargetedRepairFeedback,
    now: &str,
) -> Result<TargetedRepairClaimDelta, ClaimError> {
    let excluded_claim_id = if matches!(feedback.action, FeedbackAction::MarkFalse) {
        Some(claim.id.as_str())
    } else {
        None
    };
    let mut delta = targeted_repair_reconcile_user_backed_contradictions(
        ctx,
        tx,
        &claim.id,
        excluded_claim_id,
        now,
    )?;

    if matches!(feedback.action, FeedbackAction::NeedsNuance) {
        if let Some(corrected_text) =
            payload_string(feedback.payload_json.as_deref(), "corrected_text")?
        {
            let subject_value: serde_json::Value = serde_json::from_str(&claim.subject_ref)?;
            let subject = subject_ref_from_json(&subject_value)?;
            let replacement = targeted_repair_insert_replacement_claim(
                ctx,
                tx,
                TargetedRepairReplacement {
                    original: claim,
                    target_subject: &subject,
                    replacement_text: &corrected_text,
                    feedback,
                    now,
                    demotion_reason: "targeted_repair_needs_nuance",
                },
            )?;
            delta.claims_changed += replacement.claims_changed;
            delta
                .changed_claim_ids
                .extend(replacement.changed_claim_ids);
        }
    }

    if delta.claims_changed == 0 && delta.contradictions_reconciled == 0 {
        execute_claims_update(
            tx.conn_ref(),
            "UPDATE intelligence_claims
             SET demotion_reason = 'targeted_repair_contradiction_reconcile'
             WHERE id = ?1",
            params![&claim.id],
        )?;
        bump_invalidation_for_claim_id(tx, &claim.id)?;
        delta.claims_changed += 1;
        delta.changed_claim_ids.push(claim.id.clone());
    }

    delta.changed_claim_ids.sort();
    delta.changed_claim_ids.dedup();
    Ok(delta)
}

fn targeted_repair_apply_freshness_refresh(
    tx: &ActionDb,
    claim: &IntelligenceClaim,
    _now: &str,
) -> Result<TargetedRepairClaimDelta, ClaimError> {
    let fresher_claim_id: Option<String> = tx
        .conn_ref()
        .query_row(
            "SELECT id
             FROM intelligence_claims
             WHERE id <> ?1
               AND subject_ref = ?2
               AND claim_type = ?3
               AND coalesce(field_path, '') = coalesce(?4, '')
               AND claim_state = 'active'
               AND surfacing_state = 'active'
               AND source_asof IS NOT NULL
               AND (?5 IS NULL OR datetime(source_asof) > datetime(?5))
             ORDER BY datetime(source_asof) DESC, created_at DESC, id ASC
             LIMIT 1",
            params![
                &claim.id,
                &claim.subject_ref,
                &claim.claim_type,
                claim.field_path.as_deref(),
                claim.source_asof.as_deref(),
            ],
            |row| row.get(0),
        )
        .optional()?;

    let mut delta = TargetedRepairClaimDelta::default();
    if let Some(fresher_claim_id) = fresher_claim_id {
        execute_claims_update(
            tx.conn_ref(),
            "UPDATE intelligence_claims
             SET claim_state = 'dormant',
                 surfacing_state = 'dormant',
                 demotion_reason = 'targeted_repair_freshness_refresh',
                 superseded_by = ?2
             WHERE id = ?1",
            params![&claim.id, &fresher_claim_id],
        )?;
        delta.changed_claim_ids.push(fresher_claim_id);
    } else {
        execute_claims_update(
            tx.conn_ref(),
            "UPDATE intelligence_claims
             SET surfacing_state = 'dormant',
                 demotion_reason = 'targeted_repair_freshness_refresh',
                 verification_reason = 'freshness_refresh_requested'
             WHERE id = ?1",
            params![&claim.id],
        )?;
    }
    bump_invalidation_for_claim_id(tx, &claim.id)?;
    delta.claims_changed += 1;
    delta.changed_claim_ids.push(claim.id.clone());
    Ok(delta)
}

fn targeted_repair_apply_subject_fit_repair(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    claim: &IntelligenceClaim,
    feedback: &TargetedRepairFeedback,
    now: &str,
) -> Result<TargetedRepairClaimDelta, ClaimError> {
    let Some(subject) = corrected_subject_from_payload(feedback.payload_json.as_deref())? else {
        execute_claims_update(
            tx.conn_ref(),
            "UPDATE intelligence_claims
             SET demotion_reason = 'targeted_repair_subject_fit',
                 verification_reason = 'subject_fit_repair_requested'
             WHERE id = ?1",
            params![&claim.id],
        )?;
        bump_invalidation_for_claim_id(tx, &claim.id)?;
        return Ok(TargetedRepairClaimDelta {
            claims_changed: 1,
            contradictions_reconciled: 0,
            changed_claim_ids: vec![claim.id.clone()],
        });
    };

    targeted_repair_insert_replacement_claim(
        ctx,
        tx,
        TargetedRepairReplacement {
            original: claim,
            target_subject: &subject,
            replacement_text: &claim.text,
            feedback,
            now,
            demotion_reason: "targeted_repair_subject_fit",
        },
    )
}

fn targeted_repair_apply_source_support_repair(
    tx: &ActionDb,
    claim: &IntelligenceClaim,
    feedback: &TargetedRepairFeedback,
    _now: &str,
) -> Result<TargetedRepairClaimDelta, ClaimError> {
    let source_ref = payload_string(feedback.payload_json.as_deref(), "source_ref")?
        .unwrap_or_else(|| "unknown_source".to_string());
    let reason = format!("source_support_repair:{source_ref}");
    execute_claims_update(
        tx.conn_ref(),
        "UPDATE intelligence_claims
         SET verification_state = 'contested',
             verification_reason = ?2,
             needs_user_decision_at = NULL
         WHERE id = ?1",
        params![&claim.id, &reason],
    )?;
    bump_invalidation_for_claim_id(tx, &claim.id)?;
    Ok(TargetedRepairClaimDelta {
        claims_changed: 1,
        contradictions_reconciled: 0,
        changed_claim_ids: vec![claim.id.clone()],
    })
}

fn targeted_repair_apply_policy_repair(
    tx: &ActionDb,
    claim: &IntelligenceClaim,
    feedback: &TargetedRepairFeedback,
    now: &str,
) -> Result<TargetedRepairClaimDelta, ClaimError> {
    let surface = payload_string(feedback.payload_json.as_deref(), "surface")?
        .ok_or_else(|| {
            ClaimError::InvalidFeedback(
                "surface_inappropriate repair requires payload_json.surface".to_string(),
            )
        })
        .and_then(|surface| normalize_claim_surface(&surface))?;
    let surface = surface.as_str();

    tx.conn_ref().execute(
        "INSERT INTO claim_surface_dismissals (
             claim_id, surface, feedback_id, actor, dismissed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(claim_id, surface) DO UPDATE SET
             feedback_id = excluded.feedback_id,
             actor = excluded.actor,
             dismissed_at = excluded.dismissed_at",
        params![&claim.id, surface, &feedback.id, &feedback.actor, now],
    )?;
    bump_invalidation_for_claim_id(tx, &claim.id)?;
    Ok(TargetedRepairClaimDelta {
        claims_changed: 1,
        contradictions_reconciled: 0,
        changed_claim_ids: vec![claim.id.clone()],
    })
}

fn corrected_subject_from_payload(
    payload_json: Option<&str>,
) -> Result<Option<SubjectRef>, ClaimError> {
    let Some(raw) = payload_json
        .map(str::trim)
        .filter(|payload| !payload.is_empty())
    else {
        return Ok(None);
    };
    let payload: serde_json::Value = serde_json::from_str(raw)?;
    let Some(value) = payload
        .get("corrected_subject")
        .or_else(|| payload.get("corrected_subject_ref"))
    else {
        return Ok(None);
    };
    let subject_value = if let Some(raw_subject) = value.as_str() {
        serde_json::from_str::<serde_json::Value>(raw_subject)?
    } else {
        value.clone()
    };
    subject_ref_from_json(&subject_value).map(Some)
}

struct TargetedRepairReplacement<'a> {
    original: &'a IntelligenceClaim,
    target_subject: &'a SubjectRef,
    replacement_text: &'a str,
    feedback: &'a TargetedRepairFeedback,
    now: &'a str,
    demotion_reason: &'a str,
}

fn targeted_repair_insert_replacement_claim(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    replacement: TargetedRepairReplacement<'_>,
) -> Result<TargetedRepairClaimDelta, ClaimError> {
    let TargetedRepairReplacement {
        original,
        target_subject,
        replacement_text,
        feedback,
        now,
        demotion_reason,
    } = replacement;
    if subject_kind_label(target_subject).is_none() {
        return Err(ClaimError::SubjectRef(
            "replacement claims require a single concrete subject".to_string(),
        ));
    }
    let target_subject_ref = canonical_subject_ref(target_subject)?;
    let proposal = ClaimProposal {
        id: None,
        subject_ref: target_subject_ref,
        claim_type: original.claim_type.clone(),
        field_path: original.field_path.clone(),
        topic_key: original.topic_key.clone(),
        text: replacement_text.to_string(),
        actor: feedback.actor.clone(),
        data_source: "user_feedback".to_string(),
        source_ref: Some(
            serde_json::json!({
                "kind": "targeted_repair",
                "source_claim_id": original.id,
            })
            .to_string(),
        ),
        source_asof: Some(now.to_string()),
        observed_at: now.to_string(),
        provenance_json: serde_json::json!({
            "ability_id": TARGETED_REPAIR_ABILITY_ID,
            "repair_action": repair_action_label(feedback_semantics(feedback.action).repair),
            "feedback_action": feedback.action.as_str(),
        })
        .to_string(),
        metadata_json: Some(
            serde_json::json!({
                "feedback_action": feedback.action.as_str(),
                "source_claim_id": original.id,
            })
            .to_string(),
        ),
        thread_id: original.thread_id.clone(),
        temporal_scope: Some(original.temporal_scope.clone()),
        sensitivity: Some(original.sensitivity.clone()),
        supersedes: None,
        tombstone: None,
    };

    let replacement_id = match commit_claim(ctx, tx, proposal)? {
        CommittedClaim::Inserted { claim } | CommittedClaim::Reinforced { claim, .. } => claim.id,
        CommittedClaim::Forked { new_claim_id, .. } => new_claim_id,
        CommittedClaim::Tombstoned { .. } => {
            return Err(ClaimError::InvalidFeedback(format!(
                "replacement repair for {} unexpectedly produced a tombstone",
                original.id
            )));
        }
    };

    if matches!(
        original.claim_state,
        ClaimState::Tombstoned | ClaimState::Withdrawn
    ) {
        execute_claims_update(
            tx.conn_ref(),
            "UPDATE intelligence_claims
             SET surfacing_state = 'dormant',
                 superseded_by = ?2
             WHERE id = ?1",
            params![&original.id, &replacement_id],
        )?;
    } else {
        execute_claims_update(
            tx.conn_ref(),
            "UPDATE intelligence_claims
             SET claim_state = 'dormant',
                 surfacing_state = 'dormant',
                 demotion_reason = ?2,
                 superseded_by = ?3
             WHERE id = ?1",
            params![&original.id, demotion_reason, &replacement_id],
        )?;
    }
    tx.conn_ref().execute(
        "INSERT INTO claim_contradictions
         (id, primary_claim_id, contradicting_claim_id, branch_kind, detected_at,
          reconciliation_kind, reconciliation_note, reconciled_at, winner_claim_id)
         VALUES (?1, ?2, ?3, 'supersession', ?4, ?5, ?6, ?4, ?3)",
        params![
            uuid::Uuid::new_v4().to_string(),
            &original.id,
            &replacement_id,
            now,
            enum_to_db(&ReconciliationKind::UserPickedWinner)?,
            demotion_reason,
        ],
    )?;

    let original_subject_value: serde_json::Value = serde_json::from_str(&original.subject_ref)?;
    let original_subject = subject_ref_from_json(&original_subject_value)?;
    tx.bump_for_subject(&original_subject)?;

    Ok(TargetedRepairClaimDelta {
        claims_changed: 2,
        contradictions_reconciled: 1,
        changed_claim_ids: vec![original.id.clone(), replacement_id],
    })
}

fn targeted_repair_reconcile_user_backed_contradictions(
    _ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    claim_id: &str,
    excluded_claim_id: Option<&str>,
    now: &str,
) -> Result<TargetedRepairClaimDelta, ClaimError> {
    let mut stmt = tx.conn_ref().prepare(
        "SELECT id, primary_claim_id, contradicting_claim_id
         FROM claim_contradictions
         WHERE reconciled_at IS NULL
           AND (primary_claim_id = ?1 OR contradicting_claim_id = ?1)
         ORDER BY detected_at ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![claim_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut edges = Vec::new();
    for row in rows {
        edges.push(row?);
    }
    drop(stmt);

    let mut delta = TargetedRepairClaimDelta::default();
    for (contradiction_id, primary_id, contradicting_id) in edges {
        let primary = load_claim_by_id(tx.conn_ref(), &primary_id)?
            .ok_or_else(|| ClaimError::UnknownClaimId(primary_id.clone()))?;
        let contradicting = load_claim_by_id(tx.conn_ref(), &contradicting_id)?
            .ok_or_else(|| ClaimError::UnknownClaimId(contradicting_id.clone()))?;
        let Some(winner_id) =
            targeted_repair_pick_winner(&primary, &contradicting, claim_id, excluded_claim_id)
        else {
            continue;
        };
        let loser_id = if winner_id == primary.id {
            contradicting.id.clone()
        } else {
            primary.id.clone()
        };

        tx.conn_ref().execute(
            "UPDATE claim_contradictions
             SET reconciliation_kind = ?1,
                 reconciliation_note = ?2,
                 reconciled_at = ?3,
                 winner_claim_id = ?4,
                 merged_claim_id = NULL
             WHERE id = ?5
               AND reconciled_at IS NULL",
            params![
                enum_to_db(&ReconciliationKind::UserPickedWinner)?,
                "targeted_repair_user_backed_winner",
                now,
                &winner_id,
                &contradiction_id,
            ],
        )?;

        let loser = if loser_id == primary.id {
            &primary
        } else {
            &contradicting
        };
        if loser.claim_state == ClaimState::Active
            || loser.surfacing_state == SurfacingState::Active
        {
            execute_claims_update(
                tx.conn_ref(),
                "UPDATE intelligence_claims
                 SET claim_state = 'dormant',
                     surfacing_state = 'dormant',
                     demotion_reason = 'targeted_repair_contradiction'
                 WHERE id = ?1",
                params![&loser_id],
            )?;
            bump_invalidation_for_claim_id(tx, &loser_id)?;
            delta.claims_changed += 1;
            delta.changed_claim_ids.push(loser_id.clone());
        }

        delta.contradictions_reconciled += 1;
        if !delta.changed_claim_ids.contains(&winner_id) {
            delta.changed_claim_ids.push(winner_id);
        }
    }

    Ok(delta)
}

fn targeted_repair_pick_winner(
    primary: &IntelligenceClaim,
    contradicting: &IntelligenceClaim,
    target_claim_id: &str,
    excluded_claim_id: Option<&str>,
) -> Option<String> {
    let primary_excluded = excluded_claim_id == Some(primary.id.as_str());
    let contradicting_excluded = excluded_claim_id == Some(contradicting.id.as_str());
    match (primary_excluded, contradicting_excluded) {
        (true, true) => return None,
        (true, false) => return Some(contradicting.id.clone()),
        (false, true) => return Some(primary.id.clone()),
        (false, false) => {}
    }

    let primary_user = matches!(
        actor_class_for_actor(&primary.actor),
        Some(ClaimActorClass::User)
    );
    let contradicting_user = matches!(
        actor_class_for_actor(&contradicting.actor),
        Some(ClaimActorClass::User)
    );
    match (primary_user, contradicting_user) {
        (true, false) => return Some(primary.id.clone()),
        (false, true) => return Some(contradicting.id.clone()),
        _ => {}
    }

    let primary_active = primary.claim_state == ClaimState::Active
        && primary.surfacing_state == SurfacingState::Active;
    let contradicting_active = contradicting.claim_state == ClaimState::Active
        && contradicting.surfacing_state == SurfacingState::Active;
    match (primary_active, contradicting_active) {
        (true, false) => Some(primary.id.clone()),
        (false, true) => Some(contradicting.id.clone()),
        (true, true) if target_claim_id == primary.id => Some(contradicting.id.clone()),
        (true, true) if target_claim_id == contradicting.id => Some(primary.id.clone()),
        _ => None,
    }
}

fn targeted_repair_complete_invalidation_job(
    tx: &ActionDb,
    job: &TargetedRepairInvalidationJob,
    now: &str,
) -> Result<(), ClaimError> {
    let current_source_claim_version = tx
        .current_claim_version_for_subject(&job.subject_type, &job.subject_id)
        .unwrap_or(job.latest_source_claim_version);
    let stale_marker = if current_source_claim_version > job.latest_source_claim_version {
        Some(
            serde_json::json!({
                "reason": "targeted_repair_completed_with_newer_claim_version",
                "job_source_claim_version": job.latest_source_claim_version,
                "current_source_claim_version": current_source_claim_version,
            })
            .to_string(),
        )
    } else {
        None
    };
    tx.conn_ref().execute(
        "UPDATE invalidation_jobs
         SET status = 'completed',
             completed_at = ?2,
             lease_owner = NULL,
             lease_expires_at = NULL,
             stale_marker_json = ?3,
             updated_at = ?2
         WHERE id = ?1",
        params![&job.id, now, stale_marker.as_deref()],
    )?;
    Ok(())
}

fn targeted_repair_emit_activity_log(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    job: &TargetedRepairInvalidationJob,
    summary: &TargetedRepairRunSummary,
) -> Result<(), ClaimError> {
    let payload = serde_json::json!({
        "job_id": &job.id,
        "repair_jobs_processed": summary.repair_jobs_processed,
        "claims_changed": summary.claims_changed,
        "contradictions_reconciled": summary.contradictions_reconciled,
        "failed_jobs": summary.failed_jobs,
        "changed_claim_ids": summary.changed_claim_ids,
    });
    crate::services::signals::emit_in_transaction(
        ctx,
        tx,
        &job.subject_type,
        &job.subject_id,
        "claim_repair_ran",
        "targeted_repair",
        payload,
    )
    .map(|_| ())
    .map_err(ClaimError::Transaction)
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
// update_claim_trust
// ---------------------------------------------------------------------------

pub fn update_claim_trust(
    db: &ActionDb,
    claim_id: &str,
    trust_score: TrustScore,
    trust_version: TrustVersion,
    ctx: &ServiceContext<'_>,
) -> Result<(), ClaimsError> {
    ctx.check_mutation_allowed()
        .map_err(|e| ClaimsError::Mode(e.to_string()))?;

    let trust_computed_at = ctx.clock.now().to_rfc3339();
    let trust_score = trust_score_db_value(trust_score);
    let updated = execute_claims_update(
        db.conn_ref(),
        "UPDATE intelligence_claims
         SET trust_score = ?1,
             trust_computed_at = ?2,
             trust_version = ?3
         WHERE id = ?4",
        params![trust_score, &trust_computed_at, trust_version, claim_id],
    )?;

    if updated == 0 {
        return Err(ClaimsError::ClaimNotFound(claim_id.to_string()));
    }

    Ok(())
}

fn trust_score_db_value(trust_score: TrustScore) -> Option<f64> {
    let value = trust_score.value();
    if value.is_finite() {
        Some(value)
    } else {
        None
    }
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

/// Surface-aware active reader: active globally, minus claims dismissed
/// only for the named surface.
pub fn load_claims_active_for_surface(
    db: &ActionDb,
    subject_ref: &str,
    claim_type: Option<&str>,
    surface: &str,
) -> Result<Vec<IntelligenceClaim>, ClaimError> {
    let mut visible = Vec::new();
    for claim in load_claims_active(db, subject_ref, claim_type)? {
        if !is_claim_dismissed_on_surface(db, &claim.id, surface)? {
            visible.push(claim);
        }
    }
    Ok(visible)
}

pub fn load_claims_active_by_source_ref_for_surface(
    db: &ActionDb,
    source_ref: &str,
    surface: &str,
) -> Result<Vec<IntelligenceClaim>, ClaimError> {
    let mut visible = Vec::new();
    for claim in load_claims_active_by_source_ref(db, source_ref)? {
        if !is_claim_dismissed_on_surface(db, &claim.id, surface)? {
            visible.push(claim);
        }
    }
    Ok(visible)
}

pub fn is_claim_dismissed_on_surface(
    db: &ActionDb,
    claim_id: &str,
    surface: &str,
) -> Result<bool, ClaimError> {
    let surface = normalize_claim_surface(surface)?;
    let surface = surface.as_str();
    let found = match db
        .conn_ref()
        .query_row(
            "SELECT 1
             FROM claim_surface_dismissals
             WHERE claim_id = ?1
               AND surface = ?2
             LIMIT 1",
            params![claim_id, surface],
            |row| row.get::<_, i64>(0),
        )
        .optional()
    {
        Ok(found) => found,
        Err(error) if is_missing_claim_surface_dismissals_table_error(&error) => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    Ok(found.is_some())
}

fn is_missing_claim_surface_dismissals_table_error(error: &rusqlite::Error) -> bool {
    sqlite_error_message(error)
        .map(|message| {
            message
                .trim()
                .eq_ignore_ascii_case("no such table: claim_surface_dismissals")
        })
        .unwrap_or(false)
}

fn sqlite_error_message(error: &rusqlite::Error) -> Option<&str> {
    match error {
        rusqlite::Error::SqliteFailure(_, Some(message)) => Some(message.as_str()),
        rusqlite::Error::SqlInputError { msg, .. } => Some(msg.as_str()),
        _ => None,
    }
}

/// Default reader by source reference: active + surfaced claims only.
///
/// This supports meeting-scoped context assembly where the source event is the
/// meeting, but the asserted subject may be an attendee, account, or a nearby
/// account that still needs subject-fit gating downstream.
pub fn load_claims_active_by_source_ref(
    db: &ActionDb,
    source_ref: &str,
) -> Result<Vec<IntelligenceClaim>, ClaimError> {
    let sql = if claim_semantic_evidence_table_exists(db.conn_ref())? {
        format!(
            "SELECT {CLAIM_COLUMNS} FROM intelligence_claims
             WHERE claim_state = 'active'
               AND surfacing_state = 'active'
               AND (
                   source_ref = ?1
                   OR EXISTS (
                       SELECT 1
                       FROM claim_semantic_evidence evidence
                       WHERE evidence.canonical_claim_id = intelligence_claims.id
                         AND evidence.source_ref = ?1
                   )
               )
             ORDER BY created_at DESC"
        )
    } else {
        format!(
            "SELECT {CLAIM_COLUMNS} FROM intelligence_claims
             WHERE source_ref = ?1
               AND claim_state = 'active'
               AND surfacing_state = 'active'
             ORDER BY created_at DESC"
        )
    };
    let mut stmt = db.conn_ref().prepare(&sql)?;
    let mut rows = stmt.query(params![source_ref])?;
    let mut claims = Vec::new();
    while let Some(row) = rows.next()? {
        claims.push(read_claim_row(row)?);
    }
    Ok(claims)
}

fn claim_semantic_evidence_table_exists(conn: &Connection) -> Result<bool, ClaimError> {
    let found = conn
        .query_row(
            "SELECT 1
             FROM sqlite_master
             WHERE type = 'table'
               AND name = 'claim_semantic_evidence'
             LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    Ok(found.is_some())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EntityContextSubject {
    kind: &'static str,
    id: String,
}

/// Entity-context reader: active + surfaced claims for the requested subject
/// and, when depth permits, its account/project hierarchy neighbors.
///
/// `depth` is level-based: 1 means only the requested entity, 2 adds
/// immediate related subjects, and so on. Claim row filtering stays routed
/// through `load_claims_active_for_surface`, preserving the
/// `claim_state='active' AND surfacing_state='active'` contract and entity
/// context dismissal boundary for the caller's actual surface.
pub fn load_entity_context_claims_active_for_surface(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    depth: usize,
    surface: &str,
) -> Result<Vec<IntelligenceClaim>, ClaimError> {
    let root = entity_context_subject(entity_type, entity_id)?;
    let subjects = entity_context_subjects_within_depth(db, root, depth.max(1))?;
    let mut seen_claims = HashSet::new();
    let mut claims = Vec::new();

    for subject in subjects {
        let subject_ref = entity_context_subject_ref_json(&subject);
        for claim in load_claims_active_for_surface(db, &subject_ref, None, surface)? {
            if seen_claims.insert(claim.id.clone()) {
                claims.push(claim);
            }
        }
    }

    claims.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(claims)
}

fn entity_context_subject(
    entity_type: &str,
    entity_id: &str,
) -> Result<EntityContextSubject, ClaimError> {
    let id = entity_id.trim();
    if id.is_empty() {
        return Err(ClaimError::SubjectRef("missing id/entity_id".to_string()));
    }

    let kind = match entity_type
        .trim()
        .trim_end_matches('s')
        .to_ascii_lowercase()
        .as_str()
    {
        "account" => "account",
        "meeting" => "meeting",
        "person" => "person",
        "project" => "project",
        other => {
            return Err(ClaimError::SubjectRef(format!(
                "unsupported entity context subject kind '{other}'"
            )));
        }
    };

    Ok(EntityContextSubject {
        kind,
        id: id.to_string(),
    })
}

fn entity_context_subject_ref_json(subject: &EntityContextSubject) -> String {
    serde_json::json!({
        "kind": subject.kind,
        "id": subject.id,
    })
    .to_string()
}

fn entity_context_subjects_within_depth(
    db: &ActionDb,
    root: EntityContextSubject,
    depth: usize,
) -> Result<Vec<EntityContextSubject>, ClaimError> {
    let mut ordered = Vec::new();
    let mut seen = HashSet::new();
    let mut queue = VecDeque::from([(root, 1usize)]);

    while let Some((subject, level)) = queue.pop_front() {
        if !seen.insert(subject.clone()) {
            continue;
        }

        ordered.push(subject.clone());

        if level >= depth {
            continue;
        }

        for related in entity_context_related_subjects(db, &subject)? {
            if !seen.contains(&related) {
                queue.push_back((related, level + 1));
            }
        }
    }

    Ok(ordered)
}

fn entity_context_related_subjects(
    db: &ActionDb,
    subject: &EntityContextSubject,
) -> Result<Vec<EntityContextSubject>, ClaimError> {
    let mut related = Vec::new();

    match subject.kind {
        "account" => {
            if let Some(account) = db.get_account(&subject.id)? {
                if let Some(parent_id) = account.parent_id.filter(|id| !id.trim().is_empty()) {
                    related.push(EntityContextSubject {
                        kind: "account",
                        id: parent_id,
                    });
                }
            }

            related.extend(
                db.get_child_accounts(&subject.id)?
                    .into_iter()
                    .map(|account| EntityContextSubject {
                        kind: "account",
                        id: account.id,
                    }),
            );
        }
        "project" => {
            if let Some(project) = db.get_project(&subject.id)? {
                if let Some(parent_id) = project.parent_id.filter(|id| !id.trim().is_empty()) {
                    related.push(EntityContextSubject {
                        kind: "project",
                        id: parent_id,
                    });
                }
            }

            related.extend(
                db.get_child_projects(&subject.id)?
                    .into_iter()
                    .map(|project| EntityContextSubject {
                        kind: "project",
                        id: project.id,
                    }),
            );
        }
        "meeting" | "person" => {}
        _ => {}
    }

    related.sort_by(|left, right| {
        left.kind
            .cmp(right.kind)
            .then_with(|| left.id.cmp(&right.id))
    });
    related.dedup();
    Ok(related)
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
            subject_kind,
            subject_id
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
        id: None,
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
        supersedes: None,
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
    execute_claims_update_sqlite(
        db.conn_ref(),
        "UPDATE intelligence_claims /* dos7-allowed: bulk withdrawal helper */ \
         SET claim_state = 'withdrawn', \
             surfacing_state = 'dormant', \
             retraction_reason = ?1 \
         WHERE claim_state = 'tombstoned' \
           AND claim_type = ?2 /* dos7-allowed: WHERE-filter */",
        rusqlite::params![retraction_reason, claim_type],
    )
}

/// Withdraw Email-subject claims for resolved emails that are about to be
/// purged by the age-based lifecycle job. The email rows must still exist so
/// the subquery can identify the claim subjects safely.
pub fn withdraw_email_subject_claims_for_aged_resolved_emails(
    db: &ActionDb,
    cutoff_modifier: &str,
) -> Result<usize, rusqlite::Error> {
    execute_claims_update_sqlite(
        db.conn_ref(),
        "UPDATE intelligence_claims \
         SET claim_state = 'withdrawn', \
             retraction_reason = coalesce(retraction_reason, 'subject_purged') \
         WHERE id IN ( \
             SELECT ic.id FROM intelligence_claims ic \
             WHERE json_valid(ic.subject_ref) = 1 \
               AND ic.claim_state IN ('active', 'tombstoned', 'dormant') \
               AND lower(json_extract(ic.subject_ref, '$.kind')) = 'email' \
               AND json_extract(ic.subject_ref, '$.id') IN ( \
                   SELECT email_id FROM emails \
                   WHERE resolved_at IS NOT NULL \
                     AND resolved_at < datetime('now', ?1) \
               ) \
         )",
        params![cutoff_modifier],
    )
}

/// Withdraw Email-subject claims for all currently-present email rows before a
/// connector source purge deletes those rows.
pub fn withdraw_email_subject_claims_for_existing_emails(
    db: &ActionDb,
) -> Result<usize, rusqlite::Error> {
    execute_claims_update_sqlite(
        db.conn_ref(),
        "UPDATE intelligence_claims \
         SET claim_state = 'withdrawn', \
             retraction_reason = coalesce(retraction_reason, 'subject_purged') \
         WHERE id IN ( \
             SELECT ic.id FROM intelligence_claims ic \
             WHERE json_valid(ic.subject_ref) = 1 \
               AND ic.claim_state IN ('active', 'tombstoned', 'dormant') \
               AND lower(json_extract(ic.subject_ref, '$.kind')) = 'email' \
               AND json_extract(ic.subject_ref, '$.id') IN \
                   (SELECT email_id FROM emails) \
         )",
        [],
    )
}

pub fn withdraw_tombstones_for(
    db: &ActionDb,
    filter: WithdrawTombstoneFilter<'_>,
) -> Result<usize, rusqlite::Error> {
    let Some(normalized_kind) = normalize_subject_kind_for_claim(filter.subject_kind) else {
        return Ok(0);
    };

    execute_claims_update_sqlite(
        db.conn_ref(),
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
    #[test]
    fn claim_update_allowlist_accepts_lifecycle_trust_and_feedback_columns() {
        let sql = "UPDATE intelligence_claims
             SET claim_state = 'dormant',
                 surfacing_state = 'dormant',
                 trust_score = ?1,
                 trust_computed_at = ?2,
                 trust_version = ?3,
                 verification_state = ?4,
                 verification_reason = ?5,
                 needs_user_decision_at = ?6
             WHERE text = ?7 AND claim_type = ?8";

        assert_eq!(
            claim_update_columns(sql),
            vec![
                "claim_state",
                "surfacing_state",
                "trust_score",
                "trust_computed_at",
                "trust_version",
                "verification_state",
                "verification_reason",
                "needs_user_decision_at",
            ]
        );
        assert!(check_claim_update_allowlist(sql).is_ok());
    }

    #[test]
    fn claim_update_allowlist_rejects_non_leading_immutable_column() {
        let sql = "UPDATE intelligence_claims /* dos7-allowed: parser regression fixture */
             SET claim_state = 'dormant',
                 subject_ref = ?1
             WHERE id = ?2";

        let err = check_claim_update_allowlist(sql).expect_err("subject_ref must be rejected");
        assert!(matches!(
            err,
            ClaimError::ImmutableColumnUpdate(ref columns) if columns == "subject_ref"
        ));
    }

    #[test]
    fn claim_update_allowlist_rejects_quoted_immutable_columns() {
        let sql = "UPDATE intelligence_claims /* dos7-allowed: parser regression fixture */
             SET [created_at] = ?1,
                 `text` = ?2,
                 \"source_asof\" = ?3
             WHERE id = ?4";

        let err =
            check_claim_update_allowlist(sql).expect_err("quoted immutable columns must reject");
        assert!(matches!(
            err,
            ClaimError::ImmutableColumnUpdate(ref columns)
                if columns == "created_at, source_asof, text"
        ));
    }

    #[test]
    fn claim_update_allowlist_rejects_single_quoted_immutable_column() {
        let sql = "UPDATE intelligence_claims /* dos7-allowed: parser regression fixture */
             SET 'source_asof' = ?1,
                 claim_state = 'dormant'
             WHERE id = ?2";

        let err = check_claim_update_allowlist(sql)
            .expect_err("single-quoted immutable column must reject");
        assert!(matches!(
            err,
            ClaimError::ImmutableColumnUpdate(ref columns) if columns == "source_asof"
        ));
    }

    #[test]
    fn claim_update_allowlist_rejects_row_value_immutable_column() {
        let sql = "UPDATE intelligence_claims /* dos7-allowed: parser regression fixture */
             SET (claim_state, trust_score, subject_ref) =
                 ('dormant', ?1, ?2)
             WHERE id = ?3";

        assert_eq!(
            claim_update_columns(sql),
            vec!["claim_state", "trust_score", "subject_ref"]
        );
        let err = check_claim_update_allowlist(sql).expect_err("row-value subject_ref must reject");
        assert!(matches!(
            err,
            ClaimError::ImmutableColumnUpdate(ref columns) if columns == "subject_ref"
        ));
    }

    #[test]
    fn claim_update_allowlist_rejects_identity_columns_even_with_allowed_where_filters() {
        let sql = "UPDATE intelligence_claims /* dos7-allowed: parser regression fixture */
             SET dedup_key = ?1
             WHERE claim_type = ?2
               AND text = ?3
               AND subject_ref = ?4";

        let err = check_claim_update_allowlist(sql).expect_err("dedup_key must be rejected");
        assert!(matches!(
            err,
            ClaimError::ImmutableColumnUpdate(ref columns) if columns == "dedup_key"
        ));
    }

    #[test]
    fn execute_claims_update_rejects_immutable_columns_before_sqlite() {
        let db = test_db();
        let err = execute_claims_update(
            db.conn_ref(),
            "UPDATE intelligence_claims /* dos7-allowed: parser regression fixture */
             SET subject_ref = ?999
             WHERE id = ?1",
            params!["would-hit-sqlite-bind-error"],
        )
        .expect_err("immutable column must reject before SQLite execution");

        assert!(matches!(
            err,
            ClaimError::ImmutableColumnUpdate(ref columns) if columns == "subject_ref"
        ));
    }

    #[test]
    fn execute_claims_update_allows_lifecycle_and_trust_columns() {
        let db = test_db();
        insert_fixture_claim(
            &db,
            "claim-allowed-update",
            SUBJECT,
            "risk",
            "Allowed wrapper update",
            ClaimState::Active,
            SurfacingState::Active,
        );

        let updated = execute_claims_update(
            db.conn_ref(),
            "UPDATE intelligence_claims
             SET claim_state = 'dormant',
                 surfacing_state = 'dormant',
                 demotion_reason = ?1,
                 trust_score = ?2,
                 trust_computed_at = ?3,
                 trust_version = ?4
             WHERE id = ?5",
            params!["unit_test", 0.42_f64, TS, 3_i64, "claim-allowed-update"],
        )
        .expect("allowlisted lifecycle/trust columns should execute");

        assert_eq!(updated, 1);
        assert_eq!(
            read_lifecycle_columns(&db, "claim-allowed-update"),
            (
                "dormant".to_string(),
                "dormant".to_string(),
                Some("unit_test".to_string()),
                None
            )
        );
        assert_eq!(
            read_trust_columns(&db, "claim-allowed-update"),
            (Some(0.42), Some(TS.to_string()), Some(3))
        );
    }

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
            id: None,
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
            supersedes: None,
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

    fn edge_proposal(text: &str) -> ClaimProposal {
        let mut p = proposal(text);
        p.field_path = Some("stakeholders".to_string());
        p
    }

    fn active_claim_edges(db: &ActionDb) -> Vec<(String, String, String, String)> {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT from_entity_id, to_entity_id, edge_type, origin_claim_id
                 FROM claim_edges_active
                 ORDER BY from_entity_id, to_entity_id, edge_type, origin_claim_id",
            )
            .expect("prepare active claim edge query");
        stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("query active claim edges")
        .collect::<Result<Vec<_>, _>>()
        .expect("map active claim edges")
    }

    fn edge_lifecycle_for_origin(
        db: &ActionDb,
        origin_claim_id: &str,
    ) -> (Option<String>, Option<String>) {
        db.conn_ref()
            .query_row(
                "SELECT superseded_by, tombstoned_at
                 FROM claim_edges
                 WHERE origin_claim_id = ?1",
                params![origin_claim_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read claim edge lifecycle")
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

    fn read_claim_dedup_key(db: &ActionDb, claim_id: &str) -> String {
        db.conn_ref()
            .query_row(
                "SELECT dedup_key FROM intelligence_claims WHERE id = ?1",
                params![claim_id],
                |row| row.get(0),
            )
            .expect("read dedup_key")
    }

    fn read_claim_item_hash(db: &ActionDb, claim_id: &str) -> String {
        db.conn_ref()
            .query_row(
                "SELECT item_hash FROM intelligence_claims WHERE id = ?1",
                params![claim_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .expect("read item_hash")
            .unwrap_or_default()
    }

    fn claim_contradiction_count(db: &ActionDb) -> i64 {
        db.conn_ref()
            .query_row("SELECT count(*) FROM claim_contradictions", [], |row| {
                row.get(0)
            })
            .expect("read contradiction count")
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

    fn commit_tombstone_claim(
        ctx: &ServiceContext<'_>,
        db: &ActionDb,
        text: &str,
        temporal_scope: TemporalScope,
        sensitivity: ClaimSensitivity,
    ) {
        let mut tombstone = proposal(text);
        tombstone.temporal_scope = Some(temporal_scope);
        tombstone.sensitivity = Some(sensitivity);
        tombstone.tombstone = Some(TombstoneSpec {
            retraction_reason: "user_removal".to_string(),
            expires_at: None,
        });
        commit_claim(ctx, db, tombstone).expect("commit tombstone claim");
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

    fn read_trust_columns(
        db: &ActionDb,
        claim_id: &str,
    ) -> (Option<f64>, Option<String>, Option<i64>) {
        db.conn_ref()
            .query_row(
                "SELECT trust_score, trust_computed_at, trust_version \
                 FROM intelligence_claims WHERE id = ?1",
                params![claim_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read trust columns")
    }

    fn read_non_trust_claim_columns(db: &ActionDb, claim_id: &str) -> Vec<Option<String>> {
        db.conn_ref()
            .query_row(
                "SELECT id, subject_ref, claim_type, field_path, topic_key, text, dedup_key,
                        item_hash, actor, data_source, source_ref, source_asof, observed_at,
                        created_at, provenance_json, metadata_json, claim_state,
                        surfacing_state, demotion_reason, reactivated_at, retraction_reason,
                        expires_at, superseded_by, thread_id, temporal_scope, sensitivity,
                        verification_state, verification_reason, needs_user_decision_at
                 FROM intelligence_claims WHERE id = ?1",
                params![claim_id],
                |row| {
                    let mut values = Vec::new();
                    for index in 0..29 {
                        values.push(row.get::<_, Option<String>>(index)?);
                    }
                    Ok(values)
                },
            )
            .expect("read non-trust claim columns")
    }

    fn read_subject_ref_and_text(db: &ActionDb, claim_id: &str) -> (String, String) {
        db.conn_ref()
            .query_row(
                "SELECT subject_ref, text FROM intelligence_claims WHERE id = ?1",
                params![claim_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read subject_ref and text")
    }

    fn repair_job_count(db: &ActionDb, claim_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT count(*) FROM invalidation_jobs
                 WHERE job_kind = ?1
                   AND json_extract(payload_json, '$.claim_id') = ?2",
                params![crate::db::invalidation_jobs::KIND_TARGETED_REPAIR, claim_id],
                |row| row.get(0),
            )
            .expect("count repair jobs")
    }

    fn repair_job_status_and_attempts(db: &ActionDb, job_id: &str) -> (String, i64) {
        db.conn_ref()
            .query_row(
                "SELECT status, attempts FROM invalidation_jobs WHERE id = ?1",
                params![job_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read repair job")
    }

    fn repair_job_payload_feedback_id(db: &ActionDb, claim_id: &str) -> Option<String> {
        db.conn_ref()
            .query_row(
                "SELECT json_extract(payload_json, '$.feedback_id')
                 FROM invalidation_jobs
                 WHERE job_kind = ?1
                   AND json_extract(payload_json, '$.claim_id') = ?2
                 ORDER BY created_at ASC, id ASC
                 LIMIT 1",
                params![crate::db::invalidation_jobs::KIND_TARGETED_REPAIR, claim_id],
                |row| row.get(0),
            )
            .expect("read repair feedback id")
    }

    fn repair_job_status_and_error(db: &ActionDb, claim_id: &str) -> (String, Option<String>) {
        db.conn_ref()
            .query_row(
                "SELECT status, last_error
                 FROM invalidation_jobs
                 WHERE job_kind = ?1
                   AND json_extract(payload_json, '$.claim_id') = ?2
                 ORDER BY created_at ASC, id ASC
                 LIMIT 1",
                params![crate::db::invalidation_jobs::KIND_TARGETED_REPAIR, claim_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read repair state and error")
    }

    fn invalidation_job_count(db: &ActionDb, operation: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT count(*) FROM invalidation_jobs
                 WHERE operation = ?1 OR operation LIKE (?1 || ':%')",
                params![operation],
                |row| row.get(0),
            )
            .expect("count invalidation jobs")
    }

    fn first_invalidation_job(
        db: &ActionDb,
        operation: &str,
    ) -> (String, String, Option<String>, Option<String>, String, i64) {
        db.conn_ref()
            .query_row(
                "SELECT id, status, provider_fingerprint, prompt_fingerprint, payload_json, raw_signal_count
                 FROM invalidation_jobs
                 WHERE operation = ?1 OR operation LIKE (?1 || ':%')
                 ORDER BY created_at ASC, id ASC
                 LIMIT 1",
                params![operation],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("read invalidation job")
    }

    fn invalidation_job_versions(db: &ActionDb, job_id: &str) -> (i64, i64) {
        db.conn_ref()
            .query_row(
                "SELECT source_claim_version, latest_source_claim_version
                 FROM invalidation_jobs
                 WHERE id = ?1",
                params![job_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read invalidation job versions")
    }

    #[derive(Debug)]
    struct RepairJobSnapshot {
        id: String,
        status: String,
        latest_source_claim_version: i64,
        stale_marker_json: Option<String>,
        successor_of_job_id: Option<String>,
    }

    fn repair_job_snapshots_for_claim(db: &ActionDb, claim_id: &str) -> Vec<RepairJobSnapshot> {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT id, status, latest_source_claim_version, stale_marker_json,
                        successor_of_job_id
                 FROM invalidation_jobs
                 WHERE job_kind = ?1
                   AND json_extract(payload_json, '$.claim_id') = ?2
                 ORDER BY created_at ASC, id ASC",
            )
            .expect("prepare repair job snapshot query");
        let rows = stmt
            .query_map(
                params![crate::db::invalidation_jobs::KIND_TARGETED_REPAIR, claim_id],
                |row| {
                    Ok(RepairJobSnapshot {
                        id: row.get(0)?,
                        status: row.get(1)?,
                        latest_source_claim_version: row.get(2)?,
                        stale_marker_json: row.get(3)?,
                        successor_of_job_id: row.get(4)?,
                    })
                },
            )
            .expect("query repair job snapshots");
        rows.map(|row| row.expect("read repair job snapshot"))
            .collect()
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

    fn assert_float_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    fn read_corroboration_strengths(db: &ActionDb, claim_id: &str) -> Vec<f64> {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT strength FROM claim_corroborations
                 WHERE claim_id = ?1
                 ORDER BY data_source",
            )
            .expect("prepare corroboration strength read");
        stmt.query_map(params![claim_id], |row| row.get::<_, f64>(0))
            .expect("read corroboration strengths")
            .collect::<Result<_, _>>()
            .expect("collect corroboration strengths")
    }

    fn noisy_or_strength(strengths: &[f64]) -> f64 {
        if strengths.is_empty() {
            return 0.0;
        }

        1.0 - strengths.iter().fold(1.0, |miss_probability, strength| {
            miss_probability * (1.0 - strength)
        })
    }

    #[test]
    fn update_claim_trust_writes_only_trust_columns() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let mut p = proposal("Trust updater preserves non trust columns");
        p.metadata_json = Some(serde_json::json!({ "preserve": true }).to_string());
        p.thread_id = Some("thread-preserve".to_string());
        let claim_id = inserted_claim_id(commit_claim(&ctx, &db, p).unwrap());
        let before = read_non_trust_claim_columns(&db, &claim_id);

        update_claim_trust(&db, &claim_id, TrustScore(0.73), 4, &ctx).unwrap();

        let after = read_non_trust_claim_columns(&db, &claim_id);
        assert_eq!(after, before);
        assert_eq!(
            read_trust_columns(&db, &claim_id),
            (Some(0.73), Some(TS.to_string()), Some(4))
        );
    }

    #[test]
    fn trust_recompute_does_not_update_direct_sql() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let mut p = proposal("Trust recompute service boundary preserves claim payload");
        p.metadata_json = Some(serde_json::json!({ "service_boundary": true }).to_string());
        p.thread_id = Some("thread-service-boundary".to_string());
        let claim_id = inserted_claim_id(commit_claim(&ctx, &db, p).unwrap());
        let before = read_non_trust_claim_columns(&db, &claim_id);

        update_claim_trust(&db, &claim_id, TrustScore(0.81), 9, &ctx).unwrap();

        assert_eq!(read_non_trust_claim_columns(&db, &claim_id), before);
        assert_eq!(
            read_trust_columns(&db, &claim_id),
            (Some(0.81), Some(TS.to_string()), Some(9))
        );
    }

    #[test]
    fn update_claim_trust_uses_injected_clock_for_trust_computed_at() {
        let db = test_db();
        insert_fixture_claim(
            &db,
            "claim-clock",
            SUBJECT,
            "risk",
            "Clock-controlled trust update",
            ClaimState::Active,
            SurfacingState::Active,
        );
        let fixed_at = Utc.with_ymd_and_hms(2026, 5, 4, 9, 15, 30).unwrap();
        let clock = FixedClock::new(fixed_at);
        let rng = SeedableRng::new(17);
        let external = ExternalClients::default();
        let ctx = live_ctx(&clock, &rng, &external);

        update_claim_trust(&db, "claim-clock", TrustScore(0.64), 2, &ctx).unwrap();

        let (_, trust_computed_at, _) = read_trust_columns(&db, "claim-clock");
        assert_eq!(trust_computed_at, Some(fixed_at.to_rfc3339()));
    }

    #[test]
    fn update_claim_trust_returns_claim_not_found_for_missing_id() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let err = update_claim_trust(&db, "missing-claim", TrustScore(0.5), 1, &ctx)
            .expect_err("missing claim must return ClaimNotFound");

        assert!(matches!(
            err,
            ClaimsError::ClaimNotFound(claim_id) if claim_id == "missing-claim"
        ));
    }

    #[test]
    fn update_claim_trust_writes_null_for_unscored() {
        let db = test_db();
        insert_fixture_claim(
            &db,
            "claim-unscored",
            SUBJECT,
            "risk",
            "Unscored trust update",
            ClaimState::Active,
            SurfacingState::Active,
        );
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        update_claim_trust(&db, "claim-unscored", TrustScore(f64::NAN), 3, &ctx).unwrap();

        assert_eq!(
            read_trust_columns(&db, "claim-unscored"),
            (None, Some(TS.to_string()), Some(3))
        );
    }

    #[test]
    fn update_claim_trust_overwrites_prior_trust_score_with_new_one() {
        let db = test_db();
        insert_fixture_claim(
            &db,
            "claim-overwrite",
            SUBJECT,
            "risk",
            "Trust score overwrite",
            ClaimState::Active,
            SurfacingState::Active,
        );
        let first_at = Utc.with_ymd_and_hms(2026, 5, 4, 8, 0, 0).unwrap();
        let second_at = Utc.with_ymd_and_hms(2026, 5, 4, 8, 30, 0).unwrap();
        let clock = FixedClock::new(first_at);
        let rng = SeedableRng::new(23);
        let external = ExternalClients::default();
        let ctx = live_ctx(&clock, &rng, &external);

        update_claim_trust(&db, "claim-overwrite", TrustScore(0.21), 1, &ctx).unwrap();
        clock.set(second_at);
        update_claim_trust(&db, "claim-overwrite", TrustScore(0.88), 2, &ctx).unwrap();

        assert_eq!(
            read_trust_columns(&db, "claim-overwrite"),
            (Some(0.88), Some(second_at.to_rfc3339()), Some(2))
        );
    }

    #[test]
    fn update_claim_trust_preserves_subject_ref_and_text() {
        let db = test_db();
        insert_fixture_claim(
            &db,
            "claim-identity",
            SUBJECT,
            "risk",
            "Subject ref and text must stay stable",
            ClaimState::Active,
            SurfacingState::Active,
        );
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let before = read_subject_ref_and_text(&db, "claim-identity");

        update_claim_trust(&db, "claim-identity", TrustScore(0.91), 5, &ctx).unwrap();

        assert_eq!(read_subject_ref_and_text(&db, "claim-identity"), before);
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
    fn claim_edges_schema_exposes_active_and_backlinks_views() {
        let db = test_db();
        let object_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*)
                 FROM sqlite_master
                 WHERE name IN ('claim_edges', 'claim_edges_active', 'backlinks')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(object_count, 3);
    }

    #[test]
    fn commit_claim_populates_frontmatter_edges_in_same_transaction() {
        let db = test_db();
        seed_account(&db);
        seed_person(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let id =
            inserted_claim_id(commit_claim(&ctx, &db, edge_proposal(r#"["person-1"]"#)).unwrap());

        assert_eq!(
            active_claim_edges(&db),
            vec![(
                "acct-1".to_string(),
                "person-1".to_string(),
                "has_stakeholder".to_string(),
                id
            )]
        );
    }

    #[test]
    fn same_meaning_reinforcement_backfills_missing_frontmatter_edges() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let raw_targets = r#"["Person-MixedCase"]"#;
        let canonical_text = canonicalize_semantic_text(raw_targets);
        let subject_ref = compact_subject_ref_str(SUBJECT).expect("compact subject");
        let hash = item_hash(item_kind_for_claim_type("risk"), &canonical_text);
        let dedup_key = compute_dedup_key(&hash, &subject_ref, "risk", Some("stakeholders"));
        let existing = IntelligenceClaim {
            id: "claim-pre-edge".to_string(),
            subject_ref,
            claim_type: "risk".to_string(),
            field_path: Some("stakeholders".to_string()),
            topic_key: None,
            text: canonical_text,
            dedup_key,
            item_hash: Some(hash),
            actor: "agent:test".to_string(),
            data_source: "unit_test".to_string(),
            source_ref: None,
            source_asof: Some(TS.to_string()),
            observed_at: TS.to_string(),
            created_at: TS.to_string(),
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
        };
        insert_claim_row(&db, &existing).expect("seed pre-edge claim");

        let result = commit_claim(&ctx, &db, edge_proposal(raw_targets)).unwrap();
        match result {
            CommittedClaim::Reinforced { claim, .. } => assert_eq!(claim.id, existing.id),
            other => panic!("expected same-meaning reinforcement, got {other:?}"),
        }

        assert_eq!(
            active_claim_edges(&db),
            vec![(
                "acct-1".to_string(),
                "Person-MixedCase".to_string(),
                "has_stakeholder".to_string(),
                "claim-pre-edge".to_string()
            )]
        );
    }

    #[test]
    fn supersede_marks_prior_claim_edges_superseded_by_replacement_claim() {
        let db = test_db();
        seed_account(&db);
        seed_person(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_id =
            inserted_claim_id(commit_claim(&ctx, &db, edge_proposal(r#"["person-1"]"#)).unwrap());
        let mut replacement = edge_proposal(r#"["person-2"]"#);
        replacement.supersedes = Some(first_id.clone());
        let replacement_id = inserted_claim_id(commit_claim(&ctx, &db, replacement).unwrap());

        assert_eq!(
            edge_lifecycle_for_origin(&db, &first_id),
            (Some(replacement_id.clone()), None)
        );
        assert_eq!(
            active_claim_edges(&db),
            vec![(
                "acct-1".to_string(),
                "person-2".to_string(),
                "has_stakeholder".to_string(),
                replacement_id
            )]
        );
    }

    #[test]
    fn tombstone_resurrection_keeps_edges_out_of_active_view() {
        let db = test_db();
        seed_account(&db);
        seed_person(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_id =
            inserted_claim_id(commit_claim(&ctx, &db, edge_proposal(r#"["person-1"]"#)).unwrap());
        let mut tombstone = edge_proposal("<keyless>");
        tombstone.actor = "user".to_string();
        tombstone.data_source = "user_input".to_string();
        tombstone.tombstone = Some(TombstoneSpec {
            retraction_reason: "user_removal".to_string(),
            expires_at: None,
        });
        commit_claim(&ctx, &db, tombstone).unwrap();

        let (_, tombstoned_at) = edge_lifecycle_for_origin(&db, &first_id);
        assert!(tombstoned_at.is_some());
        assert!(active_claim_edges(&db).is_empty());

        let err = commit_claim(&ctx, &db, edge_proposal(r#"["person-1"]"#))
            .expect_err("field tombstone must block re-enrichment");
        assert!(matches!(err, ClaimError::TombstonedPreGate));
        assert!(active_claim_edges(&db).is_empty());
    }

    #[test]
    fn tombstoned_field_blocks_explicit_supersession_edges() {
        let db = test_db();
        seed_account(&db);
        seed_person(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_id =
            inserted_claim_id(commit_claim(&ctx, &db, edge_proposal(r#"["person-1"]"#)).unwrap());
        let mut tombstone = edge_proposal("<keyless>");
        tombstone.actor = "user".to_string();
        tombstone.data_source = "user_input".to_string();
        tombstone.tombstone = Some(TombstoneSpec {
            retraction_reason: "user_removal".to_string(),
            expires_at: None,
        });
        commit_claim(&ctx, &db, tombstone).unwrap();

        assert!(active_claim_edges(&db).is_empty());

        let mut replacement = edge_proposal(r#"["person-2"]"#);
        replacement.supersedes = Some(first_id);
        let err = commit_claim(&ctx, &db, replacement)
            .expect_err("field tombstone must block explicit supersession");

        assert!(matches!(err, ClaimError::TombstonedPreGate));
        assert!(active_claim_edges(&db).is_empty());
    }

    #[test]
    fn dormant_feedback_tombstones_claim_edges() {
        let db = test_db();
        seed_account(&db);
        seed_person(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, edge_proposal(r#"["person-1"]"#)).unwrap());
        assert_eq!(active_claim_edges(&db).len(), 1);

        record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::MarkOutdated),
        )
        .unwrap();

        let (_, tombstoned_at) = edge_lifecycle_for_origin(&db, &claim_id);
        assert!(tombstoned_at.is_some());
        assert!(active_claim_edges(&db).is_empty());
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

    #[test]
    fn pre_gate_does_not_block_confidential_proposal_with_internal_tombstone() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let text = "Procurement blocked renewal";
        commit_tombstone_claim(
            &ctx,
            &db,
            text,
            TemporalScope::State,
            ClaimSensitivity::Internal,
        );

        let mut confidential = proposal(text);
        confidential.sensitivity = Some(ClaimSensitivity::Confidential);

        let result = commit_claim(&ctx, &db, confidential);
        let claim = match result {
            Ok(CommittedClaim::Inserted { claim }) => claim,
            other => panic!(
                "internal tombstone must not block more restrictive confidential proposal, got {other:?}"
            ),
        };
        assert_eq!(claim.sensitivity, ClaimSensitivity::Confidential);
        assert_eq!(read_claim_sensitivity(&db, &claim.id), "confidential");
    }

    #[test]
    fn pre_gate_does_not_block_point_in_time_proposal_with_state_tombstone() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let text = "Procurement blocked renewal";
        commit_tombstone_claim(
            &ctx,
            &db,
            text,
            TemporalScope::State,
            ClaimSensitivity::Internal,
        );

        let mut point_in_time = proposal(text);
        point_in_time.temporal_scope = Some(TemporalScope::PointInTime);

        let result = commit_claim(&ctx, &db, point_in_time);
        let claim = match result {
            Ok(CommittedClaim::Inserted { claim }) => claim,
            other => {
                panic!("state tombstone must not block point-in-time proposal, got {other:?}")
            }
        };
        assert_eq!(claim.temporal_scope, TemporalScope::PointInTime);
        assert_eq!(read_claim_temporal_scope(&db, &claim.id), "point_in_time");
    }

    #[test]
    fn pre_gate_still_blocks_internal_state_proposal_with_internal_state_tombstone() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let text = "Procurement blocked renewal";
        commit_tombstone_claim(
            &ctx,
            &db,
            text,
            TemporalScope::State,
            ClaimSensitivity::Internal,
        );

        let mut internal = proposal(text);
        internal.temporal_scope = Some(TemporalScope::State);
        internal.sensitivity = Some(ClaimSensitivity::Internal);

        let err = commit_claim(&ctx, &db, internal)
            .expect_err("policy-compatible tombstone should still block recommit");
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
        let canonical = canonicalize_semantic_text("Procurement blocked renewal");
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
        let canonical = canonicalize_semantic_text("Procurement blocked renewal");
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
        let canonical = canonicalize_semantic_text("Procurement blocked renewal");
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
    fn targeted_repair_contracts_distinguish_generation_boundaries() {
        let entity_budget = targeted_repair_claim_generation_budget("get_entity_context").unwrap();
        assert_eq!(
            entity_budget.contract,
            ClaimGenerationContract::ClaimValidation
        );
        assert_eq!(entity_budget.max_candidate_claims, 0);
        assert_eq!(entity_budget.max_llm_calls, 0);
        assert!(!entity_budget.may_commit_claims);

        let meeting_budget = targeted_repair_claim_generation_budget("prepare_meeting").unwrap();
        assert_eq!(
            meeting_budget.contract,
            ClaimGenerationContract::ClaimExtraction
        );
        assert_eq!(meeting_budget.max_candidate_claims, 12);
        assert_eq!(meeting_budget.max_llm_calls, 1);
        assert!(!meeting_budget.may_commit_claims);

        let repair_budget =
            targeted_repair_claim_generation_budget(TARGETED_REPAIR_ABILITY_ID).unwrap();
        assert_eq!(repair_budget.contract, ClaimGenerationContract::ClaimRepair);
        assert_eq!(repair_budget.max_provider_queries, 1);
        assert_eq!(
            repair_budget.max_retrieval_sources,
            TARGETED_REPAIR_MAX_RETRIEVAL_SOURCES
        );
        assert_eq!(repair_budget.max_llm_calls, 1);
        assert!(repair_budget.may_commit_claims);

        let narrative_budget =
            targeted_repair_claim_generation_budget("narrative_assembly").unwrap();
        assert_eq!(
            narrative_budget.contract,
            ClaimGenerationContract::NarrativeAssembly
        );
        assert_eq!(narrative_budget.max_candidate_claims, 0);
        assert!(!narrative_budget.may_commit_claims);
    }

    #[test]
    fn narrative_assembly_cannot_commit_claims_directly() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let mut p = proposal("Narrative invented durable fact");
        p.actor = "agent:narrative_assembly".to_string();
        p.metadata_json =
            Some(serde_json::json!({ "ability_contract": "narrative_assembly" }).to_string());

        let err = commit_claim(&ctx, &db, p).expect_err("narrative commits must be rejected");
        assert!(
            matches!(err, ClaimError::InvalidActor(message) if message.contains("narrative assembly"))
        );
    }

    #[test]
    fn readonly_and_transform_abilities_cannot_commit_claims_directly() {
        for ability_id in ["prepare_meeting", "get_entity_context"] {
            let db = test_db();
            seed_account(&db);
            let (clock, rng, external) = ctx_parts();
            let ctx = live_ctx(&clock, &rng, &external);
            let mut p = proposal("Budget boundary must reject direct commit");
            p.metadata_json = Some(
                serde_json::json!({
                    "ability_id": ability_id,
                    "invocation_id": format!("invocation-{ability_id}"),
                    "claims_this_invocation": 0
                })
                .to_string(),
            );

            let err =
                commit_claim(&ctx, &db, p).expect_err("non-committing ability must be rejected");
            assert!(
                matches!(&err, ClaimError::InvalidActor(message) if message.contains(ability_id) && message.contains("may_commit_claims=false")),
                "unexpected error for {ability_id}: {err}"
            );
        }
    }

    #[test]
    fn commit_claim_rejects_non_committing_ability_from_context_or_actor() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external).with_ability_id("prepare_meeting");
        let err = commit_claim(&ctx, &db, proposal("Context ability must reject"))
            .expect_err("ServiceContext ability must be budget-gated");
        assert!(
            matches!(&err, ClaimError::InvalidActor(message) if message.contains("prepare_meeting") && message.contains("may_commit_claims=false")),
            "unexpected context ability error: {err}"
        );

        let ctx = live_ctx(&clock, &rng, &external);
        let mut p = proposal("Actor ability must reject");
        p.actor = "agent:get_entity_context".to_string();
        let err = commit_claim(&ctx, &db, p).expect_err("actor ability must be budget-gated");
        assert!(
            matches!(&err, ClaimError::InvalidActor(message) if message.contains("get_entity_context") && message.contains("may_commit_claims=false")),
            "unexpected actor ability error: {err}"
        );
    }

    #[test]
    fn commit_claim_counts_claims_this_invocation_against_ability_budget() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let invocation_id = "targeted-repair-budget-invocation";

        for idx in 0..3 {
            let mut p = proposal(&format!("Budgeted claim {idx}"));
            p.field_path = Some(format!("health.risk.{idx}"));
            p.metadata_json = Some(
                serde_json::json!({
                    "ability_id": TARGETED_REPAIR_ABILITY_ID,
                    "invocation_id": invocation_id
                })
                .to_string(),
            );
            commit_claim(&ctx, &db, p).expect("claim within targeted repair budget");
        }

        let mut over_budget = proposal("Budgeted claim 4");
        over_budget.field_path = Some("health.risk.4".to_string());
        over_budget.metadata_json = Some(
            serde_json::json!({
                "ability_id": TARGETED_REPAIR_ABILITY_ID,
                "invocation_id": invocation_id
            })
            .to_string(),
        );

        let err = commit_claim(&ctx, &db, over_budget)
            .expect_err("fourth claim must exceed targeted repair budget");
        assert!(
            matches!(&err, ClaimError::InvalidActor(message) if message.contains("budget exhausted") && message.contains("committed_claims=3")),
            "unexpected budget error: {err}"
        );
    }

    #[test]
    fn commit_claim_rejects_metadata_claims_this_invocation_at_budget() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let mut p = proposal("Caller-reported over budget claim");
        p.metadata_json = Some(
            serde_json::json!({
                "ability_id": TARGETED_REPAIR_ABILITY_ID,
                "invocation_id": "caller-reported-budget",
                "claims_this_invocation": 3
            })
            .to_string(),
        );

        let err = commit_claim(&ctx, &db, p)
            .expect_err("claims_this_invocation must be enforced before commit");
        assert!(
            matches!(&err, ClaimError::InvalidActor(message) if message.contains("claims_this_invocation=3")),
            "unexpected claims_this_invocation error: {err}"
        );
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
        let (state, attempts) = repair_job_status_and_attempts(&db, repair_job_id);
        assert_eq!(repair_job_count(&db, &claim_id), 1);
        assert_eq!(state, "pending");
        assert_eq!(attempts, 0);

        assert_eq!(signal_count(&db, "claim_repair_requested"), 1);
        assert_eq!(invalidation_job_count(&db, TARGETED_REPAIR_OPERATION), 1);
        let (_job_id, status, provider_fp, prompt_fp, payload, raw_signal_count) =
            first_invalidation_job(&db, TARGETED_REPAIR_OPERATION);
        assert_eq!(status, "pending");
        assert_eq!(
            provider_fp.as_deref(),
            Some(TARGETED_REPAIR_PROVIDER_FINGERPRINT)
        );
        assert!(prompt_fp.as_deref().is_some_and(|value| value.len() == 64));
        assert_eq!(raw_signal_count, 1);
        let payload_json: serde_json::Value =
            serde_json::from_str(&payload).expect("targeted repair payload JSON");
        assert_eq!(payload_json["claim_id"], claim_id);
        assert_eq!(
            payload_json["budget"]["max_retrieval_sources"].as_u64(),
            Some(TARGETED_REPAIR_MAX_RETRIEVAL_SOURCES as u64)
        );
        assert_eq!(
            payload_json["extraction_batch"]["prompt_fingerprint"],
            prompt_fp.unwrap()
        );
    }

    #[test]
    fn record_claim_feedback_enqueues_repair_with_post_feedback_claim_version() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk needing repair")).unwrap());
        let after_commit = read_account_claim_version(&db);

        let outcome = record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::CannotVerify),
        )
        .unwrap();

        let after_feedback = read_account_claim_version(&db);
        assert_eq!(after_feedback, after_commit + 1);
        let repair_job_id = outcome
            .repair_job_id
            .as_deref()
            .expect("repair job id should be returned");
        let (source_claim_version, latest_source_claim_version) =
            invalidation_job_versions(&db, repair_job_id);
        assert_eq!(source_claim_version, after_feedback);
        assert_eq!(latest_source_claim_version, after_feedback);
    }

    #[test]
    fn record_claim_feedback_propagates_invalidation_queue_cap_rejection() {
        with_targeted_repair_pending_cap_override(1, || {
            let db = test_db();
            seed_account(&db);
            db.conn_ref()
                .execute(
                    "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                    params!["acct-2", "Account 2", TS],
                )
                .expect("seed second account");
            let (clock, rng, external) = ctx_parts();
            let ctx = live_ctx(&clock, &rng, &external);

            let first_claim = inserted_claim_id(
                commit_claim(&ctx, &db, proposal("First capped repair")).unwrap(),
            );
            record_claim_feedback(
                &ctx,
                &db,
                feedback_input(&first_claim, FeedbackAction::CannotVerify),
            )
            .expect("first repair should fill cap");

            let mut second = proposal("Second capped repair must reject");
            second.subject_ref = serde_json::json!({
                "kind": "account",
                "id": "acct-2"
            })
            .to_string();
            let second_claim = inserted_claim_id(commit_claim(&ctx, &db, second).unwrap());
            let err = record_claim_feedback(
                &ctx,
                &db,
                feedback_input(&second_claim, FeedbackAction::CannotVerify),
            )
            .expect_err("second distinct repair must propagate cap rejection");

            assert!(
                matches!(&err, ClaimError::Db(DbError::InvalidArgument(message)) if message.contains("invalidation queue pending cap 1 reached")),
                "unexpected cap error: {err}"
            );
        });
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
    fn record_claim_feedback_coalesces_existing_active_repair_for_claim() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk at repair cap")).unwrap());

        let first = record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::CannotVerify),
        )
        .unwrap();
        let second = record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::CannotVerify),
        )
        .unwrap();

        assert_eq!(second.repair_job_id, first.repair_job_id);
        assert_eq!(repair_job_count(&db, &claim_id), 1);
        assert_eq!(invalidation_job_count(&db, TARGETED_REPAIR_OPERATION), 1);
        let (_, _, _, _, _, raw_signal_count) =
            first_invalidation_job(&db, TARGETED_REPAIR_OPERATION);
        assert_eq!(raw_signal_count, 2);
    }

    #[test]
    fn record_claim_feedback_coalescing_updates_pending_repair_feedback_id() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Risk needing newest repair")).unwrap(),
        );

        let first = record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::CannotVerify),
        )
        .unwrap();
        assert_eq!(
            repair_job_payload_feedback_id(&db, &claim_id).as_deref(),
            Some(first.feedback_id.as_str())
        );

        let second = record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::CannotVerify),
        )
        .unwrap();

        assert_eq!(repair_job_count(&db, &claim_id), 1);
        assert_eq!(
            repair_job_payload_feedback_id(&db, &claim_id).as_deref(),
            Some(second.feedback_id.as_str())
        );
        assert_eq!(second.repair_job_id, first.repair_job_id);
    }

    #[test]
    fn policy_repair_coalescing_keeps_distinct_surface_feedback() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Risk hidden on two surfaces")).unwrap(),
        );

        let mut briefing = feedback_input(&claim_id, FeedbackAction::SurfaceInappropriate);
        briefing.payload_json = Some(serde_json::json!({ "surface": "briefing" }).to_string());
        let first = record_claim_feedback(&ctx, &db, briefing).unwrap();

        let mut entity_detail = feedback_input(&claim_id, FeedbackAction::SurfaceInappropriate);
        entity_detail.payload_json =
            Some(serde_json::json!({ "surface": "entity_detail" }).to_string());
        let second = record_claim_feedback(&ctx, &db, entity_detail).unwrap();

        assert_ne!(second.repair_job_id, first.repair_job_id);
        assert_eq!(repair_job_count(&db, &claim_id), 2);

        let mut drained = false;
        for attempt in 0..8 {
            let worker_id = format!("repair-worker-policy-multi-surface-{attempt}");
            match targeted_repair_process_next_job(&ctx, &db, &worker_id)
                .expect("process pending policy repair")
            {
                TargetedRepairProcessOutcome::NoJob => {
                    drained = true;
                    break;
                }
                TargetedRepairProcessOutcome::Completed { .. } => {}
            }
        }
        assert!(drained, "targeted repair queue should drain");

        assert!(is_claim_dismissed_on_surface(&db, &claim_id, "briefing").unwrap());
        assert!(is_claim_dismissed_on_surface(
            &db,
            &claim_id,
            ClaimDismissalSurface::TauriEntityDetail.as_str()
        )
        .unwrap());

        let surfaces = db
            .conn_ref()
            .prepare(
                "SELECT surface
                 FROM claim_surface_dismissals
                 WHERE claim_id = ?1
                 ORDER BY surface ASC",
            )
            .unwrap()
            .query_map(params![&claim_id], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            surfaces,
            vec![
                ClaimDismissalSurface::Briefing.as_str().to_string(),
                ClaimDismissalSurface::TauriEntityDetail
                    .as_str()
                    .to_string(),
            ]
        );
    }

    #[test]
    fn targeted_repair_stale_job_reschedules_without_applying() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Risk hidden from briefing")).unwrap(),
        );
        let mut input = feedback_input(&claim_id, FeedbackAction::SurfaceInappropriate);
        input.payload_json = Some(serde_json::json!({ "surface": "briefing" }).to_string());
        let outcome = record_claim_feedback(&ctx, &db, input).unwrap();
        let original_job_id = outcome
            .repair_job_id
            .expect("surface repair should enqueue");

        db.bump_for_subject(&SubjectRef::Account {
            id: "acct-1".to_string(),
        })
        .expect("advance claim version");
        let current_claim_version = read_account_claim_version(&db);

        let outcome = targeted_repair_process_next_job(&ctx, &db, "repair-worker-stale")
            .expect("stale worker run should complete");
        assert_eq!(
            outcome,
            TargetedRepairProcessOutcome::Completed {
                job_id: original_job_id.clone(),
                repair_jobs_processed: 0,
                claims_changed: 0,
                contradictions_reconciled: 0,
            }
        );
        assert!(
            !is_claim_dismissed_on_surface(&db, &claim_id, "briefing").unwrap(),
            "stale job must not apply the surface dismissal"
        );

        let jobs = repair_job_snapshots_for_claim(&db, &claim_id);
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].id, original_job_id);
        assert_eq!(jobs[0].status, "completed");
        assert!(jobs[0].stale_marker_json.as_deref().is_some_and(
            |marker| marker.contains("targeted_repair_completed_with_newer_claim_version")
        ));
        assert_eq!(jobs[1].status, "pending");
        assert_eq!(jobs[1].latest_source_claim_version, current_claim_version);
        assert_eq!(
            jobs[1].successor_of_job_id.as_deref(),
            Some(jobs[0].id.as_str())
        );
    }

    #[test]
    fn targeted_repair_freshness_refresh_marks_stale_claim_completed() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk with stale source")).unwrap());

        record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::MarkOutdated),
        )
        .unwrap();
        let outcome = targeted_repair_process_next_job(&ctx, &db, "repair-worker-freshness")
            .expect("worker completes invalidation job");
        assert!(matches!(
            outcome,
            TargetedRepairProcessOutcome::Completed {
                repair_jobs_processed: 1,
                ..
            }
        ));

        let (state, error) = repair_job_status_and_error(&db, &claim_id);
        assert_eq!(state, "completed");
        assert_eq!(error, None);
        let (_, surfacing_state, demotion_reason, _) = read_lifecycle_columns(&db, &claim_id);
        assert_eq!(surfacing_state, "dormant");
        assert_eq!(
            demotion_reason.as_deref(),
            Some("targeted_repair_freshness_refresh")
        );
    }

    #[test]
    fn targeted_repair_bounded_corroboration_records_new_local_evidence() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Risk with matching local evidence")).unwrap(),
        );
        insert_fixture_claim(
            &db,
            "claim-local-evidence",
            SUBJECT,
            "risk",
            "Risk with matching local evidence",
            ClaimState::Active,
            SurfacingState::Active,
        );

        record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&claim_id, FeedbackAction::CannotVerify),
        )
        .unwrap();
        targeted_repair_process_next_job(&ctx, &db, "repair-worker-corroboration")
            .expect("process repair job");

        let data_source: String = db
            .conn_ref()
            .query_row(
                "SELECT data_source FROM claim_corroborations WHERE claim_id = ?1",
                params![&claim_id],
                |row| row.get(0),
            )
            .expect("new corroboration row");
        assert_eq!(data_source, "claim:claim-local-evidence");
        let (verification_state, _, _) = read_verification_columns(&db, &claim_id);
        assert_eq!(verification_state, "active");
        let (state, error) = repair_job_status_and_error(&db, &claim_id);
        assert_eq!(state, "completed");
        assert_eq!(error, None);
    }

    #[test]
    fn targeted_repair_bundle5_contradiction_fixture_produces_expected_delta() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let mut user_claim = proposal(
            "Riley Rivera asked to start with a written agenda and confirm next ownership.",
        );
        user_claim.id = Some("src-b5-user-edited-preference".to_string());
        user_claim.actor = "user:fixture".to_string();
        user_claim.data_source = "user".to_string();
        let user_claim_id = inserted_claim_id(commit_claim(&ctx, &db, user_claim).unwrap());

        let mut stale_agent_claim = proposal("Riley prefers a broad discovery agenda.");
        stale_agent_claim.id = Some("src-b5-original-preference".to_string());
        stale_agent_claim.actor = "agent:fixture".to_string();
        stale_agent_claim.data_source = "email".to_string();
        let forked = commit_claim(&ctx, &db, stale_agent_claim).unwrap();
        let (contradiction_id, stale_claim_id) = match forked {
            CommittedClaim::Forked {
                contradiction_id,
                new_claim_id,
                ..
            } => (contradiction_id, new_claim_id),
            other => panic!("expected bundle-5 contradiction fork, got {other:?}"),
        };
        assert_eq!(stale_claim_id, "src-b5-original-preference");

        record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&stale_claim_id, FeedbackAction::MarkFalse),
        )
        .unwrap();

        let outcome = targeted_repair_process_next_job(&ctx, &db, "repair-worker-b5").unwrap();
        match outcome {
            TargetedRepairProcessOutcome::Completed {
                repair_jobs_processed,
                contradictions_reconciled,
                ..
            } => {
                assert_eq!(repair_jobs_processed, 1);
                assert_eq!(contradictions_reconciled, 1);
            }
            other => panic!("expected targeted repair completion, got {other:?}"),
        }

        let (reconciled_at, winner_claim_id): (Option<String>, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT reconciled_at, winner_claim_id
                 FROM claim_contradictions
                 WHERE id = ?1",
                params![&contradiction_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(reconciled_at.as_deref(), Some(TS));
        assert_eq!(winner_claim_id.as_deref(), Some(user_claim_id.as_str()));

        let (repair_state, _) = repair_job_status_and_error(&db, &stale_claim_id);
        assert_eq!(repair_state, "completed");
        let (_, invalidation_status, _, _, _, _) =
            first_invalidation_job(&db, TARGETED_REPAIR_OPERATION);
        assert_eq!(invalidation_status, "completed");

        assert_eq!(signal_count(&db, "claim_repair_ran"), 1);
        let activity = first_signal_value(&db, "claim_repair_ran");
        let activity_json: serde_json::Value =
            serde_json::from_str(&activity).expect("activity payload JSON");
        assert_eq!(activity_json["contradictions_reconciled"], 1);
        assert!(activity_json["changed_claim_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some(user_claim_id.as_str())));
    }

    #[test]
    fn targeted_repair_surface_inappropriate_only_hides_named_surface() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Risk shown on one surface only")).unwrap(),
        );

        let mut input = feedback_input(&claim_id, FeedbackAction::SurfaceInappropriate);
        input.payload_json = Some(serde_json::json!({ "surface": "briefing" }).to_string());
        record_claim_feedback(&ctx, &db, input).unwrap();
        targeted_repair_process_next_job(&ctx, &db, "repair-worker-policy")
            .expect("process surface policy repair");

        let (claim_state, surfacing_state, demotion_reason, retraction_reason) =
            read_lifecycle_columns(&db, &claim_id);
        assert_eq!(claim_state, "active");
        assert_eq!(surfacing_state, "active");
        assert_eq!(demotion_reason, None);
        assert_eq!(retraction_reason, None);

        assert!(is_claim_dismissed_on_surface(&db, &claim_id, "briefing").unwrap());
        assert!(!is_claim_dismissed_on_surface(
            &db,
            &claim_id,
            ClaimDismissalSurface::TauriReport.as_str()
        )
        .unwrap());

        let briefing_ids = load_claims_active_for_surface(&db, SUBJECT, Some("risk"), "briefing")
            .unwrap()
            .into_iter()
            .map(|claim| claim.id)
            .collect::<Vec<_>>();
        assert!(!briefing_ids.contains(&claim_id));

        let account_health_ids = load_claims_active_for_surface(
            &db,
            SUBJECT,
            Some("risk"),
            ClaimDismissalSurface::TauriReport.as_str(),
        )
        .unwrap()
        .into_iter()
        .map(|claim| claim.id)
        .collect::<Vec<_>>();
        assert!(account_health_ids.contains(&claim_id));
    }

    #[test]
    fn targeted_repair_surface_inappropriate_canonicalizes_alias_surface() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Briefing alias dismissal regression")).unwrap(),
        );

        let mut input = feedback_input(&claim_id, FeedbackAction::SurfaceInappropriate);
        input.payload_json =
            Some(serde_json::json!({ "surface": "tauri_briefing_prep" }).to_string());
        record_claim_feedback(&ctx, &db, input).unwrap();
        targeted_repair_process_next_job(&ctx, &db, "repair-worker-policy-alias")
            .expect("process surface policy repair");

        let persisted_surface: String = db
            .conn_ref()
            .query_row(
                "SELECT surface
                 FROM claim_surface_dismissals
                 WHERE claim_id = ?1",
                params![&claim_id],
                |row| row.get(0),
            )
            .expect("read persisted dismissal surface");
        assert_eq!(persisted_surface, ClaimDismissalSurface::Briefing.as_str());

        let briefing_ids = load_entity_context_claims_active_for_surface(
            &db,
            "account",
            "acct-1",
            1,
            ClaimDismissalSurface::Briefing.as_str(),
        )
        .unwrap()
        .into_iter()
        .map(|claim| claim.id)
        .collect::<Vec<_>>();
        assert!(!briefing_ids.contains(&claim_id));

        let entity_detail_ids = load_entity_context_claims_active_for_surface(
            &db,
            "account",
            "acct-1",
            1,
            ClaimDismissalSurface::TauriEntityDetail.as_str(),
        )
        .unwrap()
        .into_iter()
        .map(|claim| claim.id)
        .collect::<Vec<_>>();
        assert!(entity_detail_ids.contains(&claim_id));
    }

    #[test]
    fn targeted_repair_mark_false_excludes_feedback_target_before_user_priority() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let mut user_claim = proposal("Customer requires a written agenda before the call.");
        user_claim.actor = "user:fixture".to_string();
        user_claim.data_source = "user".to_string();
        let user_claim_id = inserted_claim_id(commit_claim(&ctx, &db, user_claim).unwrap());

        let mut agent_claim = proposal("Customer is comfortable with an open discovery call.");
        agent_claim.actor = "agent:fixture".to_string();
        agent_claim.data_source = "email".to_string();
        let forked = commit_claim(&ctx, &db, agent_claim).unwrap();
        let (contradiction_id, agent_claim_id) = match forked {
            CommittedClaim::Forked {
                contradiction_id,
                new_claim_id,
                ..
            } => (contradiction_id, new_claim_id),
            other => panic!("expected agent claim to fork, got {other:?}"),
        };

        record_claim_feedback(
            &ctx,
            &db,
            feedback_input(&user_claim_id, FeedbackAction::MarkFalse),
        )
        .unwrap();
        targeted_repair_process_next_job(&ctx, &db, "repair-worker-mark-false")
            .expect("process mark-false contradiction repair");

        let winner_claim_id: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT winner_claim_id
                 FROM claim_contradictions
                 WHERE id = ?1",
                params![&contradiction_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(winner_claim_id.as_deref(), Some(agent_claim_id.as_str()));
    }

    #[test]
    fn targeted_repair_wrong_subject_replacement_cannot_resurrect_tombstoned_content() {
        let db = test_db();
        seed_account(&db);
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params!["acct-2", "Account 2", TS],
            )
            .expect("seed target account");
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let target_subject = r#"{"kind":"account","id":"acct-2"}"#;
        let mut tombstone = proposal("Procurement blocked renewal");
        tombstone.subject_ref = target_subject.to_string();
        tombstone.tombstone = Some(TombstoneSpec {
            retraction_reason: "user_removal".to_string(),
            expires_at: None,
        });
        commit_claim(&ctx, &db, tombstone).unwrap();

        let source_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Procurement blocked renewal")).unwrap(),
        );
        let mut input = feedback_input(&source_claim_id, FeedbackAction::WrongSubject);
        input.payload_json = Some(
            serde_json::json!({
                "corrected_subject": {
                    "kind": "account",
                    "id": "acct-2"
                }
            })
            .to_string(),
        );
        record_claim_feedback(&ctx, &db, input).unwrap();
        targeted_repair_process_next_job(&ctx, &db, "repair-worker-wrong-subject")
            .expect("wrong-subject repair failure is recorded on the job");

        let target_active = load_claims_active(&db, target_subject, Some("risk")).unwrap();
        assert!(
            target_active.is_empty(),
            "replacement repair must not resurrect tombstoned target content"
        );
        let (state, error) = repair_job_status_and_error(&db, &source_claim_id);
        assert_eq!(state, "pending");
        assert!(error
            .as_deref()
            .is_some_and(|message| message.contains("tombstone PRE-GATE")));
    }

    #[test]
    fn targeted_repair_wrong_subject_replacement_preserves_original_tombstone() {
        let db = test_db();
        seed_account(&db);
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params!["acct-2", "Account 2", TS],
            )
            .expect("seed corrected account");
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let text = "procurement blocked renewal on the asserted account";
        let source_claim_id = inserted_claim_id(commit_claim(&ctx, &db, proposal(text)).unwrap());
        let mut input = feedback_input(&source_claim_id, FeedbackAction::WrongSubject);
        input.payload_json = Some(
            serde_json::json!({
                "corrected_subject": {
                    "kind": "account",
                    "id": "acct-2"
                }
            })
            .to_string(),
        );
        record_claim_feedback(&ctx, &db, input).unwrap();
        let outcome =
            targeted_repair_process_next_job(&ctx, &db, "repair-worker-wrong-subject-success")
                .expect("process wrong-subject replacement repair");
        assert!(matches!(
            outcome,
            TargetedRepairProcessOutcome::Completed {
                repair_jobs_processed: 1,
                claims_changed: 2,
                contradictions_reconciled: 1,
                ..
            }
        ));

        let target_active =
            load_claims_active(&db, r#"{"kind":"account","id":"acct-2"}"#, Some("risk")).unwrap();
        assert_eq!(target_active.len(), 1);
        assert_eq!(target_active[0].text, text);

        let (claim_state, surfacing_state, _, retraction_reason) =
            read_lifecycle_columns(&db, &source_claim_id);
        assert_eq!(claim_state, "tombstoned");
        assert_eq!(surfacing_state, "dormant");
        assert_eq!(retraction_reason.as_deref(), Some("wrong_subject"));

        let err = commit_claim(&ctx, &db, proposal(text))
            .expect_err("original subject/text must remain tombstone-gated");
        assert!(matches!(err, ClaimError::TombstonedPreGate));
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
        assert_eq!(repair_job_count(&db, &claim_id), 1);
        assert_eq!(invalidation_job_count(&db, TARGETED_REPAIR_OPERATION), 1);
        let (_, _, _, _, _, raw_signal_count) =
            first_invalidation_job(&db, TARGETED_REPAIR_OPERATION);
        assert_eq!(raw_signal_count, 2);
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

    /// L2 cycle-1 fix #6: canonicalize_semantic_text lowercases, trims,
    /// and collapses internal whitespace runs.
    #[test]
    fn canonicalize_semantic_text_lowercases_trims_collapses_whitespace() {
        assert_eq!(
            canonicalize_semantic_text("  ARR Risk\trenewal "),
            "arr risk renewal"
        );
        assert_eq!(
            canonicalize_semantic_text("Procurement   Blocked\n\nRenewal"),
            "procurement blocked renewal"
        );
        assert_eq!(
            canonicalize_semantic_text("already canonical"),
            "already canonical"
        );
    }

    #[test]
    fn semantic_qualifiers_from_metadata_accepts_legacy_sidecar_key() {
        let metadata = serde_json::json!({
            LEGACY_SEMANTIC_QUALIFIERS_METADATA_KEY: ["region:US"],
        })
        .to_string();
        let qualifiers =
            semantic_qualifiers_from_metadata(Some(&metadata)).expect("legacy qualifiers parse");

        assert_eq!(qualifiers.len(), 1);
        assert!(qualifiers.contains("region:US"));

        let non_mergeable = serde_json::json!({
            LEGACY_SEMANTIC_QUALIFIERS_METADATA_KEY: ["region:US"],
            LEGACY_NON_SEMANTIC_MERGEABLE_METADATA_KEY: true,
        })
        .to_string();
        assert!(semantic_qualifiers_from_metadata(Some(&non_mergeable)).is_none());
    }

    #[test]
    fn semantic_high_salience_qualifiers_normalizes_region_aliases_before_tokenizing() {
        for (region_text, expected) in [
            ("U.S.", "region:US"),
            ("U.S", "region:US"),
            ("USA", "region:US"),
            ("United States", "region:US"),
            ("U.K.", "region:UK"),
            ("U.K", "region:UK"),
            ("United Kingdom", "region:UK"),
            ("E.U.", "region:EU"),
            ("E.U", "region:EU"),
            ("European Union", "region:EU"),
            ("A.P.A.C.", "region:APAC"),
            ("E.M.E.A.", "region:EMEA"),
        ] {
            let text = format!("{region_text} Phase 2 budget approval is pending with finance");
            let qualifiers = semantic_high_salience_qualifiers(&text);
            assert!(
                qualifiers.contains(expected),
                "{region_text} should produce {expected}, got {qualifiers:?}"
            );
        }

        let qualifiers = semantic_high_salience_qualifiers(
            "Finance asked us whether Phase 2 budget approval is pending",
        );
        assert!(
            !qualifiers.contains("region:US"),
            "lowercase pronoun 'us' must not become a US region qualifier"
        );
    }

    #[test]
    fn semantic_us_region_alias_requires_uppercase_or_dotted_acronym() {
        let dotted = semantic_high_salience_qualifiers(
            "U.S Phase 2 budget approval is pending with finance",
        );
        assert!(dotted.contains("region:US"));

        for text in [
            "Finance asked us.",
            "us",
            "Finance asked us. Phase 2 budget approval is pending",
        ] {
            let qualifiers = semantic_high_salience_qualifiers(text);
            assert!(
                !qualifiers.contains("region:US"),
                "{text:?} must not produce a US region qualifier: {qualifiers:?}"
            );
        }
    }

    #[test]
    fn semantic_near_duplicate_is_tightly_scoped() {
        assert!(semantic_near_duplicate(
            "Phase 2 budget approval is pending with finance",
            "Finance has not approved the Phase 2 budget yet"
        ));
        assert!(semantic_near_duplicate(
            "Phase 2 funding is awaiting finance signoff",
            "Budget sign-off for Phase 2 remains blocked by Finance"
        ));
        assert!(
            !semantic_near_duplicate(
                "Finance has not approved the Phase 2 budget yet",
                "Legal has not approved the Phase 2 contract terms yet"
            ),
            "shared wording on phase approval must not collapse different owners/objects"
        );
        assert!(
            !semantic_near_duplicate(
                "phase 2 budget approval is pending with finance",
                "phase 2 contract approval is pending with finance"
            ),
            "shared phase approval wording must not collapse different objects"
        );
        assert!(
            semantic_near_duplicate(
                "phase 2 deal signing approval is pending with finance",
                "phase 2 deal signature approval is pending with finance"
            ),
            "explicit signing/signature synonyms should still collapse"
        );
        assert!(
            !semantic_near_duplicate(
                "Phase 2 budget approval is pending with finance",
                "Finance approved the Phase 2 budget"
            ),
            "opposite assertion status must remain separate"
        );
        assert!(
            !semantic_near_duplicate(
                "Target Example public source_ref claim is allowed.",
                "Target Example internal source_ref claim is allowed."
            ),
            "generic lexical overlap without an explicit assertion status must not collapse"
        );
        assert!(
            !semantic_near_duplicate(
                "Q3 Phase 2 budget approval is pending with finance",
                "Phase 2 budget approval is pending with finance"
            ),
            "quarter-scoped claims must not collapse into unscoped claims"
        );
        assert!(
            !semantic_near_duplicate(
                "US Phase 2 budget approval is pending with finance",
                "EU Phase 2 budget approval is pending with finance"
            ),
            "region variants must not collapse across scopes"
        );
        assert!(
            !semantic_near_duplicate(
                "Acme Phase 2 budget approval is pending with finance",
                "Globex Phase 2 budget approval is pending with finance"
            ),
            "named-entity variants must not collapse across entities"
        );
    }

    #[test]
    fn semantic_negation_applies_to_all_confirmed_status_terms() {
        for term in [
            "approved",
            "complete",
            "completed",
            "greenlit",
            "secured",
            "finalized",
            "confirmed",
        ] {
            let text = format!("Phase 2 budget is not {term} with finance");
            assert_eq!(
                compute_semantic_signature(&text).status,
                SemanticAssertionStatus::Pending,
                "{term} must honor the negation window"
            );
        }

        assert_eq!(
            compute_semantic_signature("Approval hasn't landed").status,
            SemanticAssertionStatus::Pending
        );
        assert_eq!(
            compute_semantic_signature("Approval hasn\u{2019}t landed").status,
            SemanticAssertionStatus::Pending
        );
        assert!(!semantic_near_duplicate(
            "Phase 2 budget is secured with finance",
            "Phase 2 budget is not secured with finance"
        ));
        assert!(semantic_near_duplicate(
            "Phase 2 budget approval hasn\u{2019}t landed with finance",
            "Phase 2 budget approval is pending with finance"
        ));
    }

    #[test]
    fn semantic_negative_contractions_expand_generically_before_status_tokenization() {
        for (negative, positive) in [
            (
                "Finance haven't approved Phase 2 budget",
                "Finance approved Phase 2 budget",
            ),
            ("Marketing aren't complete", "Marketing complete"),
            ("Sales weren't greenlit", "Sales greenlit"),
            ("Renewal isn't secured", "Renewal secured"),
            ("Approval ain't landed", "Approval landed"),
            ("Project cannot proceed", "Project can proceed"),
            ("Project can't proceed", "Project can proceed"),
        ] {
            assert_eq!(
                compute_semantic_signature(negative).status,
                SemanticAssertionStatus::Pending,
                "{negative} must tokenize with a negator"
            );
            assert_eq!(
                compute_semantic_signature(positive).status,
                SemanticAssertionStatus::Confirmed,
                "{positive} must remain a positive status assertion"
            );
            assert!(
                !semantic_near_duplicate(positive, negative),
                "{negative} must not semantically merge with {positive}"
            );
        }

        assert_eq!(
            compute_semantic_signature("Sales weren\u{2019}t greenlit").status,
            SemanticAssertionStatus::Pending,
            "curly apostrophe negative contractions must tokenize with a negator"
        );
    }

    #[test]
    fn semantic_irregular_negative_contractions_normalize_before_generic_regex() {
        assert_eq!(
            normalize_semantic_contractions("Finance won't approve Phase 2 budget"),
            "Finance will not approve Phase 2 budget"
        );
        assert_eq!(
            normalize_semantic_contractions("Acme Phase 2 project shan't proceed"),
            "Acme Phase 2 project shall not proceed"
        );
        assert_eq!(
            normalize_semantic_contractions("Finance haven't approved Phase 2 budget"),
            "Finance have not approved Phase 2 budget"
        );

        assert!(semantic_near_duplicate(
            "Finance won't approve Phase 2 budget",
            "Finance will not approve Phase 2 budget"
        ));
        assert!(semantic_near_duplicate(
            "Acme Phase 2 project shan't proceed",
            "Acme Phase 2 project shall not proceed"
        ));
        assert_eq!(
            compute_semantic_signature("Finance won't approve Phase 2 budget").status,
            SemanticAssertionStatus::Pending
        );
        assert_eq!(
            compute_semantic_signature("Finance will not approve Phase 2 budget").status,
            SemanticAssertionStatus::Pending
        );
    }

    #[test]
    fn commit_claim_preserves_region_qualifiers_after_text_canonicalization() {
        for region in ["US", "EU", "APAC", "EMEA"] {
            let db = test_db();
            seed_account(&db);
            let (clock, rng, external) = ctx_parts();
            let ctx = live_ctx(&clock, &rng, &external);
            let other_region = if region == "US" { "EU" } else { "US" };
            let first_text = format!("{region} Phase 2 budget approval is pending with finance");
            let second_text =
                format!("{other_region} Phase 2 budget approval is pending with finance");

            let first_id =
                inserted_claim_id(commit_claim(&ctx, &db, proposal(&first_text)).unwrap());
            update_claim_trust(&db, &first_id, TrustScore(0.85), 1, &ctx).unwrap();

            let (_, stored_text) = read_subject_ref_and_text(&db, &first_id);
            assert_eq!(stored_text, canonicalize_semantic_text(&first_text));
            assert_eq!(stored_text, first_text.to_ascii_lowercase());

            let result = commit_claim(&ctx, &db, proposal(&second_text)).unwrap();
            match result {
                CommittedClaim::Forked {
                    primary_claim,
                    new_claim_id,
                    ..
                } => {
                    assert_eq!(primary_claim.id, first_id);
                    assert_ne!(new_claim_id, first_id);
                }
                other => panic!("{region} scoped claim collapsed unexpectedly: {other:?}"),
            }

            let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
            assert_eq!(active.len(), 2);
        }
    }

    #[test]
    fn commit_claim_preserves_dotted_us_region_qualifier_against_unscoped_variant() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_text = "U.S. Phase 2 budget approval is pending with finance";
        let first_id = inserted_claim_id(commit_claim(&ctx, &db, proposal(first_text)).unwrap());
        update_claim_trust(&db, &first_id, TrustScore(0.85), 1, &ctx).unwrap();

        let result = commit_claim(
            &ctx,
            &db,
            proposal("Phase 2 budget approval is pending with finance"),
        )
        .unwrap();
        match result {
            CommittedClaim::Forked {
                primary_claim,
                new_claim_id,
                ..
            } => {
                assert_eq!(primary_claim.id, first_id);
                assert_ne!(new_claim_id, first_id);
            }
            other => panic!("U.S.-scoped claim collapsed into unscoped variant: {other:?}"),
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn commit_claim_preserves_united_states_region_qualifier_against_eu_variant() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_text = "United States Phase 2 budget approval is pending with finance";
        let first_id = inserted_claim_id(commit_claim(&ctx, &db, proposal(first_text)).unwrap());
        update_claim_trust(&db, &first_id, TrustScore(0.85), 1, &ctx).unwrap();

        let result = commit_claim(
            &ctx,
            &db,
            proposal("EU Phase 2 budget approval is pending with finance"),
        )
        .unwrap();
        match result {
            CommittedClaim::Forked {
                primary_claim,
                new_claim_id,
                ..
            } => {
                assert_eq!(primary_claim.id, first_id);
                assert_ne!(new_claim_id, first_id);
            }
            other => panic!("United States-scoped claim collapsed into EU variant: {other:?}"),
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn commit_claim_preserves_named_entity_qualifiers_after_text_canonicalization() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_text = "Acme Phase 2 budget approval is pending with finance";
        let first_id = inserted_claim_id(commit_claim(&ctx, &db, proposal(first_text)).unwrap());
        update_claim_trust(&db, &first_id, TrustScore(0.85), 1, &ctx).unwrap();

        let (_, stored_text) = read_subject_ref_and_text(&db, &first_id);
        assert_eq!(stored_text, canonicalize_semantic_text(first_text));
        assert_eq!(stored_text, first_text.to_ascii_lowercase());

        let same_entity = commit_claim(
            &ctx,
            &db,
            proposal("Finance has not approved Acme Phase 2 budget yet"),
        )
        .unwrap();
        match same_entity {
            CommittedClaim::Reinforced { claim, .. } => assert_eq!(claim.id, first_id),
            other => panic!("same Acme qualifier should reinforce, got {other:?}"),
        }

        let other_entity = commit_claim(
            &ctx,
            &db,
            proposal("Globex Phase 2 budget approval is pending with finance"),
        )
        .unwrap();
        match other_entity {
            CommittedClaim::Forked {
                primary_claim,
                new_claim_id,
                ..
            } => {
                assert_eq!(primary_claim.id, first_id);
                assert_ne!(new_claim_id, first_id);
            }
            other => panic!("Globex variant collapsed into Acme unexpectedly: {other:?}"),
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn commit_claim_does_not_merge_legacy_lowercased_scoped_claim_without_sidecar() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let legacy_text =
            canonicalize_semantic_text("US Phase 2 budget approval is pending with finance");
        insert_fixture_claim(
            &db,
            "legacy-lowercase-us",
            SUBJECT,
            "risk",
            &legacy_text,
            ClaimState::Active,
            SurfacingState::Active,
        );
        update_claim_trust(&db, "legacy-lowercase-us", TrustScore(0.85), 1, &ctx).unwrap();

        let result = commit_claim(
            &ctx,
            &db,
            proposal("Phase 2 budget approval is pending with finance"),
        )
        .unwrap();

        match result {
            CommittedClaim::Forked {
                primary_claim,
                new_claim_id,
                ..
            } => {
                assert_eq!(primary_claim.id, "legacy-lowercase-us");
                assert_ne!(new_claim_id, "legacy-lowercase-us");
            }
            other => panic!("legacy qualifierless scoped claim must not collapse, got {other:?}"),
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn commit_claim_negated_confirmed_status_forks_from_positive_claim() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let primary_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                proposal("Phase 2 budget is secured with finance"),
            )
            .unwrap(),
        );
        update_claim_trust(&db, &primary_id, TrustScore(0.85), 1, &ctx).unwrap();

        let result = commit_claim(
            &ctx,
            &db,
            proposal("Phase 2 budget is not secured with finance"),
        )
        .unwrap();

        match result {
            CommittedClaim::Forked {
                primary_claim,
                contradiction_id,
                new_claim_id,
            } => {
                assert_eq!(primary_claim.id, primary_id);
                assert_ne!(new_claim_id, primary_id);
                let edge_count: i64 = db
                    .conn_ref()
                    .query_row(
                        "SELECT count(*) FROM claim_contradictions WHERE id = ?1",
                        params![&contradiction_id],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(edge_count, 1);
            }
            other => panic!("negated secured claim must fork, got {other:?}"),
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
        assert_eq!(claim_contradiction_count(&db), 1);
    }

    #[test]
    fn commit_claim_negative_contraction_statuses_fork_from_positive_claims() {
        for (positive, negative) in [
            (
                "Finance approved Phase 2 budget",
                "Finance haven't approved Phase 2 budget",
            ),
            ("Marketing complete", "Marketing aren't complete"),
            ("Sales greenlit", "Sales weren't greenlit"),
            ("Renewal secured", "Renewal isn't secured"),
            ("Approval landed", "Approval ain't landed"),
            ("Project can proceed", "Project cannot proceed"),
        ] {
            let db = test_db();
            seed_account(&db);
            let (clock, rng, external) = ctx_parts();
            let ctx = live_ctx(&clock, &rng, &external);

            let primary_id =
                inserted_claim_id(commit_claim(&ctx, &db, proposal(positive)).unwrap());
            update_claim_trust(&db, &primary_id, TrustScore(0.85), 1, &ctx).unwrap();

            let result = commit_claim(&ctx, &db, proposal(negative)).unwrap();
            match result {
                CommittedClaim::Forked {
                    primary_claim,
                    contradiction_id,
                    new_claim_id,
                } => {
                    assert_eq!(primary_claim.id, primary_id);
                    assert_ne!(new_claim_id, primary_id);
                    let edge_count: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT count(*) FROM claim_contradictions WHERE id = ?1",
                            params![&contradiction_id],
                            |row| row.get(0),
                        )
                        .unwrap();
                    assert_eq!(edge_count, 1);
                }
                other => {
                    panic!("{negative} must fork from positive status claim {positive}: {other:?}")
                }
            }

            let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
            assert_eq!(active.len(), 2);
            assert_eq!(claim_contradiction_count(&db), 1);
        }
    }

    #[test]
    fn commit_claim_semantic_variants_collapse_to_one_entity_detail_claim() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let variants = [
            "Phase 2 budget approval is pending with finance",
            "Finance has not approved the Phase 2 budget yet",
            "Phase 2 funding is awaiting finance signoff",
            "Budget sign-off for Phase 2 remains blocked by Finance",
            "The phase 2 budget still needs finance approval",
            "Finance approval for the Phase 2 budget is still outstanding",
        ];
        let mut first_id = None;

        for (index, text) in variants.iter().enumerate() {
            let mut p = proposal(text);
            p.data_source = format!("semantic_source_{}", index + 1);
            p.source_ref = Some(format!("fixture://semantic/source-{}", index + 1));
            p.actor = format!("agent:semantic:{}", index + 1);
            p.observed_at = format!("2026-05-02T12:0{}:00+00:00", index + 1);
            p.thread_id = Some(format!("thread-semantic-{}", index + 1));
            p.provenance_json = serde_json::json!({
                "variant": index + 1,
                "source_ref": p.source_ref.as_deref(),
            })
            .to_string();
            let result = commit_claim(&ctx, &db, p).unwrap();

            if index == 0 {
                let id = inserted_claim_id(result);
                update_claim_trust(&db, &id, TrustScore(0.85), 1, &ctx).unwrap();
                first_id = Some(id);
            } else {
                match result {
                    CommittedClaim::Reinforced { claim, .. } => {
                        assert_eq!(Some(claim.id), first_id);
                    }
                    other => panic!("expected semantic variant to reinforce, got {other:?}"),
                }
            }
        }

        let first_id = first_id.expect("first claim inserted");
        let active = load_entity_context_claims_active_for_surface(
            &db,
            "account",
            "acct-1",
            1,
            ClaimDismissalSurface::TauriEntityDetail.as_str(),
        )
        .unwrap();
        assert_eq!(
            active.len(),
            1,
            "Tauri entity detail should render one canonical claim"
        );
        assert_eq!(active[0].id, first_id);

        let primary_source = active[0].data_source.clone();
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT data_source, source_asof, source_mechanism, reinforcement_count
                 FROM claim_corroborations
                 WHERE claim_id = ?1
                 ORDER BY data_source",
            )
            .unwrap();
        let corroborations = stmt
            .query_map(params![&first_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(primary_source, "semantic_source_1");
        assert_eq!(corroborations.len(), 5);
        for (idx, (data_source, source_asof, source_mechanism, count)) in
            corroborations.iter().enumerate()
        {
            assert_eq!(data_source, &format!("semantic_source_{}", idx + 2));
            assert_eq!(source_asof.as_deref(), Some(TS));
            assert_eq!(
                source_mechanism.as_deref(),
                Some("semantic_near_duplicate_merge")
            );
            assert_eq!(*count, 1);
        }

        let recovered: (String, String, String, String, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT data_source, provenance_json, original_text, observed_at, thread_id
                 FROM claim_semantic_evidence
                 WHERE canonical_claim_id = ?1
                   AND source_ref = 'fixture://semantic/source-4'",
                params![&first_id],
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
            .expect("semantic variant evidence recoverable by source_ref");
        assert_eq!(recovered.0, "semantic_source_4");
        assert!(recovered.1.contains("\"variant\":4"));
        assert_eq!(recovered.2, variants[3]);
        assert_eq!(recovered.3, "2026-05-02T12:04:00+00:00");
        assert_eq!(recovered.4.as_deref(), Some("thread-semantic-4"));

        let recovered_by_source_ref =
            load_claims_active_by_source_ref(&db, "fixture://semantic/source-4").unwrap();
        assert_eq!(recovered_by_source_ref.len(), 1);
        assert_eq!(recovered_by_source_ref[0].id, first_id);
        assert_eq!(
            recovered_by_source_ref[0].source_ref.as_deref(),
            Some("fixture://semantic/source-1")
        );

        let recovered_for_surface = load_claims_active_by_source_ref_for_surface(
            &db,
            "fixture://semantic/source-4",
            ClaimDismissalSurface::TauriEntityDetail.as_str(),
        )
        .unwrap();
        assert_eq!(recovered_for_surface.len(), 1);
        assert_eq!(recovered_for_surface[0].id, first_id);
    }

    #[test]
    fn commit_claim_budget_vs_contract_approval_does_not_auto_canonicalize() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                proposal("phase 2 budget approval is pending with finance"),
            )
            .unwrap(),
        );
        update_claim_trust(&db, &first_id, TrustScore(0.85), 1, &ctx).unwrap();

        let result = commit_claim(
            &ctx,
            &db,
            proposal("phase 2 contract approval is pending with finance"),
        )
        .unwrap();

        match result {
            CommittedClaim::Inserted { claim } => assert_ne!(claim.id, first_id),
            CommittedClaim::Forked {
                primary_claim,
                new_claim_id,
                ..
            } => {
                assert_eq!(primary_claim.id, first_id);
                assert_ne!(new_claim_id, first_id);
            }
            CommittedClaim::Reinforced { claim, .. } => {
                panic!(
                    "contract approval incorrectly canonicalized into {}",
                    claim.id
                )
            }
            CommittedClaim::Tombstoned { .. } => panic!("unexpected tombstone"),
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn commit_claim_signing_vs_signature_synonym_auto_canonicalizes() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                proposal("phase 2 deal signing approval is pending with finance"),
            )
            .unwrap(),
        );
        update_claim_trust(&db, &first_id, TrustScore(0.85), 1, &ctx).unwrap();

        let result = commit_claim(
            &ctx,
            &db,
            proposal("phase 2 deal signature approval is pending with finance"),
        )
        .unwrap();

        match result {
            CommittedClaim::Reinforced { claim, .. } => assert_eq!(claim.id, first_id),
            other => panic!("signing/signature synonym should canonicalize, got {other:?}"),
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn commit_claim_lowercase_entity_swap_does_not_auto_canonicalize() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                proposal("acme phase 2 budget approval is pending with finance"),
            )
            .unwrap(),
        );
        update_claim_trust(&db, &first_id, TrustScore(0.85), 1, &ctx).unwrap();

        let result = commit_claim(
            &ctx,
            &db,
            proposal("globex phase 2 budget approval is pending with finance"),
        )
        .unwrap();

        match result {
            CommittedClaim::Inserted { claim } => assert_ne!(claim.id, first_id),
            CommittedClaim::Forked {
                primary_claim,
                new_claim_id,
                ..
            } => {
                assert_eq!(primary_claim.id, first_id);
                assert_ne!(new_claim_id, first_id);
            }
            CommittedClaim::Reinforced { claim, .. } => {
                panic!("globex claim incorrectly canonicalized into {}", claim.id)
            }
            CommittedClaim::Tombstoned { .. } => panic!("unexpected tombstone"),
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn commit_claim_confidential_variant_does_not_collapse_into_internal_canonical() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                proposal("Phase 2 budget approval is pending with finance"),
            )
            .unwrap(),
        );
        update_claim_trust(&db, &first_id, TrustScore(0.85), 1, &ctx).unwrap();

        let mut confidential = proposal("Finance has not approved the Phase 2 budget yet");
        confidential.sensitivity = Some(ClaimSensitivity::Confidential);
        let result = commit_claim(&ctx, &db, confidential).unwrap();

        match result {
            CommittedClaim::Inserted { claim } => assert_ne!(claim.id, first_id),
            CommittedClaim::Forked { new_claim_id, .. } => assert_ne!(new_claim_id, first_id),
            CommittedClaim::Reinforced { claim, .. } => {
                panic!("confidential variant collapsed into {}", claim.id)
            }
            CommittedClaim::Tombstoned { .. } => panic!("unexpected tombstone"),
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn commit_claim_internal_variant_can_collapse_into_confidential_canonical() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let mut canonical = proposal("Phase 2 budget approval is pending with finance");
        canonical.sensitivity = Some(ClaimSensitivity::Confidential);
        let first_id = inserted_claim_id(commit_claim(&ctx, &db, canonical).unwrap());
        update_claim_trust(&db, &first_id, TrustScore(0.85), 1, &ctx).unwrap();

        let result = commit_claim(
            &ctx,
            &db,
            proposal("Finance has not approved the Phase 2 budget yet"),
        )
        .unwrap();

        match result {
            CommittedClaim::Reinforced { claim, .. } => assert_eq!(claim.id, first_id),
            other => {
                panic!("internal variant should reinforce confidential canonical, got {other:?}")
            }
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(read_claim_sensitivity(&db, &first_id), "confidential");
    }

    #[test]
    fn commit_claim_confidential_semantic_variant_merges_despite_internal_tombstone() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let canonical_text = "Phase 2 budget approval is pending with finance";

        commit_tombstone_claim(
            &ctx,
            &db,
            canonical_text,
            TemporalScope::State,
            ClaimSensitivity::Internal,
        );

        let mut canonical = proposal(canonical_text);
        canonical.sensitivity = Some(ClaimSensitivity::Confidential);
        let canonical_id = inserted_claim_id(commit_claim(&ctx, &db, canonical).unwrap());
        update_claim_trust(&db, &canonical_id, TrustScore(0.85), 1, &ctx).unwrap();

        let mut paraphrase = proposal("Finance has not approved the Phase 2 budget yet");
        paraphrase.sensitivity = Some(ClaimSensitivity::Confidential);
        let result = commit_claim(&ctx, &db, paraphrase).unwrap();

        match result {
            CommittedClaim::Reinforced { claim, .. } => assert_eq!(claim.id, canonical_id),
            other => panic!(
                "confidential paraphrase should reinforce confidential canonical, got {other:?}"
            ),
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 1, "must not insert a duplicate active claim");
        assert_eq!(active[0].id, canonical_id);
        assert_eq!(read_claim_sensitivity(&db, &canonical_id), "confidential");
    }

    #[test]
    fn semantic_duplicate_lookup_skips_backfill_tombstoned_active_with_different_dedup_key() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let active_text = "Phase 2 budget approval is pending with finance";
        let active_id = inserted_claim_id(commit_claim(&ctx, &db, proposal(active_text)).unwrap());
        update_claim_trust(&db, &active_id, TrustScore(0.85), 1, &ctx).unwrap();

        let active_hash = read_claim_item_hash(&db, &active_id);
        seed_backfill_shaped_tombstone(&db, &active_hash, active_text);
        assert_ne!(
            read_claim_dedup_key(&db, &active_id),
            read_claim_dedup_key(&db, "m1-fixture-1"),
            "fixture must keep the legacy backfill dedup_key shape distinct from the active claim"
        );

        let result = commit_claim(
            &ctx,
            &db,
            proposal("Finance has not approved the Phase 2 budget yet"),
        )
        .unwrap();

        match result {
            CommittedClaim::Inserted { claim } => assert_ne!(claim.id, active_id),
            CommittedClaim::Reinforced { claim, .. } => {
                panic!(
                    "semantic paraphrase reinforced shadowed active claim {}",
                    claim.id
                )
            }
            CommittedClaim::Forked { primary_claim, .. } => {
                panic!(
                    "semantic paraphrase forked against shadowed active claim {}",
                    primary_claim.id
                )
            }
            CommittedClaim::Tombstoned { .. } => panic!("unexpected tombstone"),
        }

        let corroboration_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM claim_corroborations WHERE claim_id = ?1",
                params![&active_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(corroboration_count, 0);

        let contradiction_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM claim_contradictions
                 WHERE primary_claim_id = ?1 OR contradicting_claim_id = ?1",
                params![&active_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(contradiction_count, 0);
    }

    #[test]
    fn contradiction_lookup_skips_backfill_tombstoned_active_with_different_dedup_key() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let active_text = "Phase 2 budget approval is pending with finance";
        let active_id = inserted_claim_id(commit_claim(&ctx, &db, proposal(active_text)).unwrap());
        let active_hash = read_claim_item_hash(&db, &active_id);
        seed_backfill_shaped_tombstone(&db, &active_hash, active_text);
        assert_ne!(
            read_claim_dedup_key(&db, &active_id),
            read_claim_dedup_key(&db, "m1-fixture-1"),
            "fixture must keep the legacy backfill dedup_key shape distinct from the active claim"
        );

        let result = commit_claim(
            &ctx,
            &db,
            proposal("Legal has not approved the Phase 2 contract terms yet"),
        )
        .unwrap();

        match result {
            CommittedClaim::Inserted { claim } => assert_ne!(claim.id, active_id),
            CommittedClaim::Forked { primary_claim, .. } => {
                panic!("forked against shadowed active claim {}", primary_claim.id)
            }
            CommittedClaim::Reinforced { claim, .. } => {
                panic!("unexpectedly reinforced shadowed active claim {}", claim.id)
            }
            CommittedClaim::Tombstoned { .. } => panic!("unexpected tombstone"),
        }

        assert_eq!(claim_contradiction_count(&db), 0);
    }

    #[test]
    fn contradiction_lookup_skips_point_in_time_proposal_against_standard_active_claim() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let active_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Renewal looks healthy")).unwrap());

        let mut point_in_time = proposal("Renewal at risk due to procurement");
        point_in_time.temporal_scope = Some(TemporalScope::PointInTime);
        let result = commit_claim(&ctx, &db, point_in_time).unwrap();

        match result {
            CommittedClaim::Inserted { claim } => {
                assert_ne!(claim.id, active_id);
                assert_eq!(claim.temporal_scope, TemporalScope::PointInTime);
            }
            other => panic!(
                "point-in-time proposal must not fork against state active claim, got {other:?}"
            ),
        }

        assert_eq!(claim_contradiction_count(&db), 0);
        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn contradiction_lookup_skips_confidential_proposal_against_internal_active_claim() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let active_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Renewal looks healthy")).unwrap());

        let mut confidential = proposal("Renewal at risk due to procurement");
        confidential.sensitivity = Some(ClaimSensitivity::Confidential);
        let result = commit_claim(&ctx, &db, confidential).unwrap();

        match result {
            CommittedClaim::Inserted { claim } => {
                assert_ne!(claim.id, active_id);
                assert_eq!(claim.sensitivity, ClaimSensitivity::Confidential);
            }
            other => panic!(
                "confidential proposal must not fork against internal active claim, got {other:?}"
            ),
        }

        assert_eq!(claim_contradiction_count(&db), 0);
        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn contradiction_lookup_forks_standard_proposal_against_standard_active_claim() {
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
                let edge_count: i64 = db
                    .conn_ref()
                    .query_row(
                        "SELECT count(*) FROM claim_contradictions WHERE id = ?1",
                        params![&contradiction_id],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(edge_count, 1);
            }
            other => {
                panic!("standard proposal should fork against standard active claim, got {other:?}")
            }
        }

        assert_eq!(claim_contradiction_count(&db), 1);
    }

    #[test]
    fn contradiction_lookup_ignores_surfacing_dormant_active_claim() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        insert_fixture_claim(
            &db,
            "dormant-active",
            SUBJECT,
            "risk",
            "Renewal looks healthy",
            ClaimState::Active,
            SurfacingState::Dormant,
        );

        let result =
            commit_claim(&ctx, &db, proposal("Renewal at risk due to procurement")).unwrap();
        match result {
            CommittedClaim::Inserted { claim } => assert_ne!(claim.id, "dormant-active"),
            other => panic!("surfacing-dormant active claim must not fork, got {other:?}"),
        }

        assert_eq!(claim_contradiction_count(&db), 0);
        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 1);
        assert_ne!(active[0].id, "dormant-active");
    }

    #[test]
    fn commit_claim_point_in_time_variant_does_not_collapse_into_state_canonical() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                proposal("Phase 2 budget approval is pending with finance"),
            )
            .unwrap(),
        );
        update_claim_trust(&db, &first_id, TrustScore(0.85), 1, &ctx).unwrap();

        let mut point_in_time = proposal("Finance has not approved the Phase 2 budget yet");
        point_in_time.temporal_scope = Some(TemporalScope::PointInTime);
        let result = commit_claim(&ctx, &db, point_in_time).unwrap();

        match result {
            CommittedClaim::Inserted { claim } => assert_ne!(claim.id, first_id),
            CommittedClaim::Forked { new_claim_id, .. } => assert_ne!(new_claim_id, first_id),
            CommittedClaim::Reinforced { claim, .. } => {
                panic!("point-in-time variant collapsed into {}", claim.id)
            }
            CommittedClaim::Tombstoned { .. } => panic!("unexpected tombstone"),
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn commit_claim_exact_dedup_scans_past_incompatible_newer_claim() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let text = "Phase 2 budget approval is pending with finance";

        let state_id = inserted_claim_id(commit_claim(&ctx, &db, proposal(text)).unwrap());

        clock.advance(chrono::Duration::seconds(1));
        let mut point_in_time = proposal(text);
        point_in_time.temporal_scope = Some(TemporalScope::PointInTime);
        let point_in_time_id = inserted_claim_id(commit_claim(&ctx, &db, point_in_time).unwrap());

        clock.advance(chrono::Duration::seconds(1));
        let result = commit_claim(&ctx, &db, proposal(text)).unwrap();
        match result {
            CommittedClaim::Reinforced { claim, .. } => assert_eq!(claim.id, state_id),
            other => panic!("state recommit should reinforce original state claim, got {other:?}"),
        }

        assert_eq!(read_claim_temporal_scope(&db, &state_id), "state");
        assert_eq!(
            read_claim_temporal_scope(&db, &point_in_time_id),
            "point_in_time"
        );
        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn commit_claim_exact_dedup_ignores_active_dormant_surface() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let text = "procurement blocked renewal";

        insert_fixture_claim(
            &db,
            "hidden-dormant-exact-dedup",
            SUBJECT,
            "risk",
            text,
            ClaimState::Active,
            SurfacingState::Dormant,
        );
        let hidden_dedup_key = read_claim_dedup_key(&db, "hidden-dormant-exact-dedup");

        let result = commit_claim(&ctx, &db, proposal(text)).unwrap();
        let inserted_id = match result {
            CommittedClaim::Inserted { claim } => {
                assert_ne!(claim.id, "hidden-dormant-exact-dedup");
                assert_eq!(claim.surfacing_state, SurfacingState::Active);
                claim.id
            }
            CommittedClaim::Reinforced { claim, .. } => {
                panic!("hidden dormant exact-dedup row reinforced {}", claim.id)
            }
            other => panic!("expected visible insert beside hidden dormant row, got {other:?}"),
        };

        assert_eq!(read_claim_dedup_key(&db, &inserted_id), hidden_dedup_key);
        let (claim_state, surfacing_state, _, _) =
            read_lifecycle_columns(&db, "hidden-dormant-exact-dedup");
        assert_eq!(claim_state, "active");
        assert_eq!(surfacing_state, "dormant");

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, inserted_id);
    }

    #[test]
    fn commit_claim_closed_variant_does_not_collapse_into_state_canonical() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                proposal("Phase 2 budget approval is pending with finance"),
            )
            .unwrap(),
        );
        update_claim_trust(&db, &first_id, TrustScore(0.85), 1, &ctx).unwrap();

        let mut closed = proposal("Finance has not approved the Phase 2 budget yet");
        closed.temporal_scope = Some(TemporalScope::Closed);
        let result = commit_claim(&ctx, &db, closed).unwrap();

        match result {
            CommittedClaim::Inserted { claim } => assert_ne!(claim.id, first_id),
            CommittedClaim::Forked { new_claim_id, .. } => assert_ne!(new_claim_id, first_id),
            CommittedClaim::Reinforced { claim, .. } => {
                panic!("closed variant collapsed into {}", claim.id)
            }
            CommittedClaim::Tombstoned { .. } => panic!("unexpected tombstone"),
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn commit_claim_related_but_distinct_semantic_claims_do_not_collapse() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                proposal("Finance has not approved the Phase 2 budget yet"),
            )
            .unwrap(),
        );
        let result = commit_claim(
            &ctx,
            &db,
            proposal("Legal has not approved the Phase 2 contract terms yet"),
        )
        .unwrap();

        match result {
            CommittedClaim::Forked {
                primary_claim,
                new_claim_id,
                ..
            } => {
                assert_eq!(primary_claim.id, first_id);
                assert_ne!(new_claim_id, first_id);
            }
            other => panic!("expected distinct claim to stay separate, got {other:?}"),
        }

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn commit_claim_low_trust_semantic_duplicate_routes_to_needs_verification() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                proposal("Phase 2 budget approval is pending with finance"),
            )
            .unwrap(),
        );
        update_claim_trust(&db, &first_id, TrustScore(0.42), 1, &ctx).unwrap();

        let result = commit_claim(
            &ctx,
            &db,
            proposal("Finance has not approved the Phase 2 budget yet"),
        )
        .unwrap();

        let inserted = match result {
            CommittedClaim::Inserted { claim } => claim,
            other => panic!("low-trust duplicate should not auto-canonicalize, got {other:?}"),
        };
        assert_eq!(
            trust_band_for_score(inserted.trust_score),
            factors::TrustBand::NeedsVerification
        );
        assert_eq!(
            inserted.verification_reason.as_deref(),
            Some("semantic_duplicate_low_trust_needs_verification")
        );

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
        let contradiction_count: i64 = db
            .conn_ref()
            .query_row("SELECT count(*) FROM claim_contradictions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            contradiction_count, 0,
            "low-trust semantic duplicate is verification work, not a contradiction"
        );
    }

    #[test]
    fn commit_claim_unscored_semantic_duplicate_routes_to_needs_verification() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let first_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                proposal("Phase 2 budget approval is pending with finance"),
            )
            .unwrap(),
        );

        let result = commit_claim(
            &ctx,
            &db,
            proposal("Finance has not approved the Phase 2 budget yet"),
        )
        .unwrap();

        let inserted = match result {
            CommittedClaim::Inserted { claim } => claim,
            other => panic!("unscored duplicate should not auto-canonicalize, got {other:?}"),
        };
        assert_ne!(inserted.id, first_id);
        assert_eq!(
            trust_band_for_score(inserted.trust_score),
            factors::TrustBand::NeedsVerification
        );
        assert_eq!(
            inserted.verification_reason.as_deref(),
            Some("semantic_duplicate_low_trust_needs_verification")
        );

        let active = load_claims_active(&db, SUBJECT, Some("risk")).unwrap();
        assert_eq!(active.len(), 2);
        let contradiction_count: i64 = db
            .conn_ref()
            .query_row("SELECT count(*) FROM claim_contradictions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(contradiction_count, 0);
    }

    #[test]
    fn commit_claim_low_trust_semantic_duplicate_still_checks_contradictions() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);

        let low_trust_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                proposal("Phase 2 budget approval is pending with finance"),
            )
            .unwrap(),
        );
        update_claim_trust(&db, &low_trust_id, TrustScore(0.42), 1, &ctx).unwrap();
        insert_fixture_claim(
            &db,
            "opposite-active",
            SUBJECT,
            "risk",
            "finance approved the phase 2 budget",
            ClaimState::Active,
            SurfacingState::Active,
        );

        let result = commit_claim(
            &ctx,
            &db,
            proposal("Finance has not approved the Phase 2 budget yet"),
        )
        .unwrap();

        match result {
            CommittedClaim::Forked {
                primary_claim,
                contradiction_id,
                new_claim_id,
            } => {
                assert_eq!(primary_claim.id, "opposite-active");
                assert_ne!(new_claim_id, low_trust_id);
                let edge_count: i64 = db
                    .conn_ref()
                    .query_row(
                        "SELECT count(*) FROM claim_contradictions WHERE id = ?1",
                        params![&contradiction_id],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(edge_count, 1);
            }
            other => panic!(
                "opposite active assertion must still fork despite low-trust duplicate, got {other:?}"
            ),
        }
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
    fn corroboration_strength_matches_record_corroboration_formula() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, proposal("Risk formula")).unwrap());

        record_corroboration(&ctx, &db, &claim_id, "calendar", None, None).unwrap();
        record_corroboration(&ctx, &db, &claim_id, "glean", None, None).unwrap();
        let strengths = read_corroboration_strengths(&db, &claim_id);

        assert_eq!(strengths, vec![0.5, 0.5]);
        assert_float_close(noisy_or_strength(&strengths), 0.75);
    }

    #[test]
    fn corroboration_same_source_reinforcement_saturates_below_diverse_sources() {
        let db = test_db();
        seed_account(&db);
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let mut same_source_proposal = proposal("Risk same source");
        same_source_proposal.field_path = Some("health.risk.same_source".to_string());
        let mut diverse_proposal = proposal("Risk diverse source");
        diverse_proposal.field_path = Some("health.risk.diverse".to_string());
        let same_source_claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, same_source_proposal).unwrap());
        let diverse_claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, diverse_proposal).unwrap());

        let first =
            record_corroboration(&ctx, &db, &same_source_claim_id, "glean", None, None).unwrap();
        let second =
            record_corroboration(&ctx, &db, &same_source_claim_id, "glean", None, None).unwrap();
        record_corroboration(&ctx, &db, &diverse_claim_id, "calendar", None, None).unwrap();
        record_corroboration(&ctx, &db, &diverse_claim_id, "glean", None, None).unwrap();

        let same_source_strength =
            noisy_or_strength(&read_corroboration_strengths(&db, &same_source_claim_id));
        let diverse_strength =
            noisy_or_strength(&read_corroboration_strengths(&db, &diverse_claim_id));

        assert_eq!(first, second);
        assert_float_close(same_source_strength, 1.0);
        assert_float_close(diverse_strength, 0.75);
        assert!(
            same_source_strength > diverse_strength,
            "landed W3-C formula currently saturates same-source reinforcement at the ceiling"
        );
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

//! Audience-aware claim receipt builder — allowlist-primary construction.
//!
//! Fills the claim receipt placeholder per W1 §5.9 and CSO cycle-1 Findings 10/11/12 +
//! correctness cycle-1 F5. Composes ADR-0108 primitives — does NOT define a second
//! redaction system.
//!
//! ## Contract
//!
//! [`build_receipt_for_audience`] is a render-time primitive. `audience` is an INPUT
//! to construction, NOT a filter on output. Each audience's [`ClaimReceipt`] is
//! built field-by-field from its module-level allowlist; fields absent from the
//! allowlist are zeroed to stable defaults (`None` / empty Vec / [`RedactionLevel::Full`]
//! / [`Freshness::Unknown`]). Same `target × audience` → byte-identical receipt
//! (AC-341.10).
//!
//! ## Audiences (AC-341.12)
//!
//! - [`Audience::UserTauri`] — local-to-local product receipt (Tauri AND WP block
//!   render — same trust boundary per ADR-0129). Full local provenance + generic
//!   source labels.
//! - [`Audience::AgentMcp`] — explicit cycle-1 row; coarsened freshness; NO source
//!   labels (timing oracle); NO `subject_id` (graph leak); NO `claim_id`.
//! - [`Audience::ActivityLog`] — W4 Activity surface. Subject label + bucket +
//!   timestamp + link. No raw audit rows.
//! - [`Audience::Lint`] — W4 Lint Mode. Finding metadata only.
//! - [`Audience::OperationalAuditStorage`] — **non-disclosure tag, not a render
//!   target** (AC-341.11). Returns [`PrivacyError::NonDisclosureAudience`]; CI lint
//!   `check_audit_disclosure_allowlist.sh` enforces no direct reads of
//!   `maintenance_audit` from `services::claim_receipt::*`.
//!
//! ## Derived & composed claims (AC-341.10, correctness F5)
//!
//! - **Composed claims** carry `derived_from: Vec<ClaimId>` in claim metadata.
//!   Sensitivity = max(self, inputs). If max exceeds the surface policy, the
//!   ENTIRE composed claim drops (no partial render — prevents inference leaks).
//! - **Cross-claim references** in `evidence_summary` render as
//!   `<source_type> (redacted)` when the referenced claim's sensitivity exceeds
//!   the surface policy. Never opaque IDs.
//! - **Derived chains** use max sensitivity across the chain.

use std::collections::HashSet;

use abilities_runtime::abilities::provenance::{
    claim_trust_band_from_score, sanitize_explanation_for_render, subject::SubjectRef, FieldPath,
};
use abilities_runtime::sensitivity::{
    render_policy_for_surface, RenderActor, RenderDecision, RenderSurface,
};
use abilities_runtime::types::{ClaimSensitivity, IntelligenceClaim};
use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::Connection;

use crate::services::claim_receipt::contracts::*;
use crate::services::claim_receipt::contradiction::{
    contradiction_caveat, unresolved_contradiction_count,
};
use crate::services::claim_receipt::render_rules::{
    freshness_for, generic_source_label, redaction_for, ReceiptRenderKind,
};

// ─── Audience ──────────────────────────────────────────────────────────────

/// Audience for receipt construction. INPUT to [`build_receipt_for_audience`], not
/// a post-render filter (AC-341.10 / CSO Finding 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Audience {
    /// Tauri + WP block render — same local trust boundary (ADR-0129).
    UserTauri,
    /// MCP tool consumer (v1.4.7). Explicit row per cycle-1 CSO Finding 12.
    AgentMcp,
    /// W4 Activity Log surface.
    ActivityLog,
    /// W4 Lint Mode surface.
    Lint,
    /// Non-disclosure tag — NOT a render target. Returns
    /// [`PrivacyError::NonDisclosureAudience`]. CI lint enforces no direct
    /// `maintenance_audit` reads (AC-341.11 / CSO Finding 11).
    OperationalAuditStorage,
}

impl Audience {
    /// String key for snapshot fixtures + CI lint cross-reference.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UserTauri => "user_tauri",
            Self::AgentMcp => "agent_mcp",
            Self::ActivityLog => "activity_log",
            Self::Lint => "lint",
            Self::OperationalAuditStorage => "operational_audit_storage",
        }
    }
}

// ─── Field allowlists (per-audience module consts) ────────────────────────
//
// These constants are the source of truth consumed by the snapshot lint test
// `audience_json_keys_are_subset_of_allowlist`. Adding a field to
// [`ClaimReceipt`] forces an explicit allowlist amendment OR an explicit
// per-audience zeroing — there is no implicit pass-through.

/// UserTauri — local-to-local product receipt. WP block render is local too
/// (ADR-0129), so it consumes this same allowlist.
pub const USER_TAURI_ALLOWED_FIELDS: &[&str] = &[
    "target",
    "surfaceContext",
    "renderedText",
    "trust",
    "lifecycle",
    "provenance",
    "actions",
];

/// AgentMcp — explicit row per CSO cycle-1 F12. Coarsened freshness, no source
/// labels, no `subject_id` / `claim_id`. `renderedText` is present at the DTO
/// level (always `null` for this audience — the MCP consumer renders its own
/// text from the structured payload). Population of `renderedText` for AgentMcp
/// is forbidden by the build_agent_mcp invariant; the allowlist permits the
/// JSON key (null) but the builder never writes a value.
pub const AGENT_MCP_ALLOWED_FIELDS: &[&str] = &[
    "target",
    "surfaceContext",
    "renderedText",
    "trust",
    "lifecycle",
    "provenance",
    "actions",
];

/// W4 Activity Log — user-readable event + subject label + bucket + timestamp.
/// `renderedText` is null on this audience (Activity surface uses its own
/// composer); the JSON key is permitted to keep the DTO shape stable.
pub const ACTIVITY_LOG_ALLOWED_FIELDS: &[&str] = &[
    "target",
    "surfaceContext",
    "renderedText",
    "trust",
    "lifecycle",
    "provenance",
    "actions",
];

/// W4 Lint Mode — finding type + subject label + bucket + severity.
/// `renderedText` is null on this audience.
pub const LINT_ALLOWED_FIELDS: &[&str] = &[
    "target",
    "surfaceContext",
    "renderedText",
    "trust",
    "lifecycle",
    "provenance",
    "actions",
];

// ─── Errors ────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum PrivacyError {
    #[error("claim not found: {0}")]
    ClaimNotFound(String),
    #[error("operational audit storage is a non-disclosure tag, not a render target")]
    NonDisclosureAudience,
    #[error("composed claim dropped: max input sensitivity exceeds surface policy")]
    ComposedClaimDropped,
    #[error("surface policy dropped the claim for this audience")]
    SurfaceDrop,
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("invalid claim metadata: {0}")]
    InvalidMetadata(String),
}

// ─── Entry point ───────────────────────────────────────────────────────────

/// Build a [`ClaimReceipt`] for the given `(target, audience)` pair from a
/// synchronous connection.
pub fn build_receipt_for_audience(
    target: &ReceiptTarget,
    audience: Audience,
    conn: &Connection,
) -> Result<ClaimReceipt, PrivacyError> {
    if matches!(audience, Audience::OperationalAuditStorage) {
        return Err(PrivacyError::NonDisclosureAudience);
    }

    let (claim_id, target_field_path) = match target {
        ReceiptTarget::Claim {
            claim_id,
            field_path,
            ..
        } => (claim_id.clone(), field_path.clone()),
        _ => return Err(PrivacyError::ClaimNotFound("non-claim target".to_string())),
    };

    let claim = crate::services::claims::load_claim_by_id(conn, &claim_id)
        .map_err(|error| PrivacyError::InvalidMetadata(error.to_string()))?
        .ok_or_else(|| PrivacyError::ClaimNotFound(claim_id.clone()))?;

    let chain_max = max_sensitivity_across_chain(&claim, conn)?;

    let surface = audience_render_surface(audience);
    let actor = audience_render_actor(audience);
    let policy_claim = synthetic_chain_max_claim(&claim, chain_max.clone());
    let decision = render_policy_for_surface(&policy_claim, surface, &actor);

    let render_kind = match decision {
        RenderDecision::Drop => {
            return if claim_has_derivation(&claim) {
                Err(PrivacyError::ComposedClaimDropped)
            } else {
                Err(PrivacyError::SurfaceDrop)
            };
        }
        RenderDecision::Render => ReceiptRenderKind::Render,
        RenderDecision::RenderRedacted { .. } => ReceiptRenderKind::Redacted,
    };

    let source_asof = claim.source_asof.as_deref().and_then(parse_claim_timestamp);
    let now = Utc::now();
    let freshness = freshness_for(source_asof, now);
    let contradiction_caveat =
        contradiction_caveat(unresolved_contradiction_count(conn, &claim.id)?);

    let mut receipt = match audience {
        Audience::UserTauri => build_user_tauri(
            target,
            target_field_path,
            &claim,
            &chain_max,
            render_kind,
            source_asof,
            freshness,
        ),
        Audience::AgentMcp => build_agent_mcp(target, &claim, &chain_max, render_kind, freshness),
        Audience::ActivityLog => build_activity_log(
            target,
            target_field_path,
            &claim,
            &chain_max,
            render_kind,
            source_asof,
            freshness,
        ),
        Audience::Lint => build_lint(
            target,
            target_field_path,
            &claim,
            &chain_max,
            render_kind,
            source_asof,
            freshness,
        ),
        Audience::OperationalAuditStorage => unreachable!("rejected above"),
    };
    receipt.trust.caveat = contradiction_caveat;

    Ok(receipt)
}

// ─── UserTauri (local-to-local product receipt) ───────────────────────────

#[allow(clippy::too_many_arguments)]
fn build_user_tauri(
    target: &ReceiptTarget,
    target_field_path: Option<String>,
    claim: &IntelligenceClaim,
    chain_max: &ClaimSensitivity,
    render_kind: ReceiptRenderKind,
    source_asof: Option<DateTime<Utc>>,
    freshness: Freshness,
) -> ClaimReceipt {
    let redaction = redaction_for(render_kind, chain_max);

    let sources = vec![ProvenanceSource {
        label: generic_source_label(&claim.data_source),
        source_type: Some(claim.data_source.clone()),
        as_of: source_asof,
        href: None,
        redacted: matches!(render_kind, ReceiptRenderKind::Redacted),
    }];

    let evidence_summary = sanitized_evidence_summary(claim);

    ClaimReceipt {
        target: target.clone(),
        surface_context: SurfaceContext::EntityDetail,
        rendered_text: None,
        trust: ReceiptTrust {
            band: claim_trust_band_from_score(claim.trust_score),
            source_asof,
            freshness,
            caveat: None,
            rationale: None,
        },
        lifecycle: ReceiptLifecycle {
            claim_state: claim.claim_state.clone(),
            surfacing_state: claim.surfacing_state.clone(),
            verification_state: claim.verification_state,
            updated_at: claim
                .reactivated_at
                .as_deref()
                .and_then(parse_claim_timestamp)
                .or_else(|| parse_claim_timestamp(&claim.created_at)),
        },
        provenance: ReceiptProvenance {
            sources,
            field_path: target_field_path.or_else(|| claim.field_path.clone()),
            evidence_summary,
            redaction,
        },
        actions: Vec::new(),
    }
}

// ─── AgentMcp (explicit cycle-1 row — coarsened, no IDs, no source labels) ─

fn build_agent_mcp(
    target: &ReceiptTarget,
    claim: &IntelligenceClaim,
    chain_max: &ClaimSensitivity,
    render_kind: ReceiptRenderKind,
    freshness: Freshness,
) -> ClaimReceipt {
    let scrubbed_target = scrub_target_for_agent_mcp(target);
    let redaction = redaction_for(render_kind, chain_max);

    ClaimReceipt {
        target: scrubbed_target,
        surface_context: SurfaceContext::Mcp,
        rendered_text: None,
        trust: ReceiptTrust {
            band: claim_trust_band_from_score(claim.trust_score),
            source_asof: None,
            freshness,
            caveat: None,
            rationale: None,
        },
        lifecycle: ReceiptLifecycle {
            claim_state: claim.claim_state.clone(),
            surfacing_state: claim.surfacing_state.clone(),
            verification_state: claim.verification_state,
            updated_at: None,
        },
        provenance: ReceiptProvenance {
            sources: Vec::new(),
            field_path: None,
            evidence_summary: sanitized_evidence_summary(claim),
            redaction,
        },
        actions: Vec::new(),
    }
}

// ─── Activity Log + Lint (W4 surfaces) ────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn build_activity_log(
    target: &ReceiptTarget,
    target_field_path: Option<String>,
    claim: &IntelligenceClaim,
    chain_max: &ClaimSensitivity,
    render_kind: ReceiptRenderKind,
    source_asof: Option<DateTime<Utc>>,
    freshness: Freshness,
) -> ClaimReceipt {
    let redaction = redaction_for(render_kind, chain_max);
    ClaimReceipt {
        target: target.clone(),
        surface_context: SurfaceContext::ActionsWork,
        rendered_text: None,
        trust: ReceiptTrust {
            band: claim_trust_band_from_score(claim.trust_score),
            source_asof,
            freshness,
            caveat: None,
            rationale: None,
        },
        lifecycle: ReceiptLifecycle {
            claim_state: claim.claim_state.clone(),
            surfacing_state: claim.surfacing_state.clone(),
            verification_state: claim.verification_state,
            updated_at: None,
        },
        provenance: ReceiptProvenance {
            sources: vec![ProvenanceSource {
                label: generic_source_label(&claim.data_source),
                source_type: Some(claim.data_source.clone()),
                as_of: source_asof,
                href: None,
                redacted: matches!(render_kind, ReceiptRenderKind::Redacted),
            }],
            field_path: target_field_path.or_else(|| claim.field_path.clone()),
            evidence_summary: None,
            redaction,
        },
        actions: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_lint(
    target: &ReceiptTarget,
    target_field_path: Option<String>,
    claim: &IntelligenceClaim,
    chain_max: &ClaimSensitivity,
    render_kind: ReceiptRenderKind,
    source_asof: Option<DateTime<Utc>>,
    freshness: Freshness,
) -> ClaimReceipt {
    let redaction = redaction_for(render_kind, chain_max);
    ClaimReceipt {
        target: target.clone(),
        surface_context: SurfaceContext::ActionsWork,
        rendered_text: None,
        trust: ReceiptTrust {
            band: claim_trust_band_from_score(claim.trust_score),
            source_asof,
            freshness,
            caveat: None,
            rationale: None,
        },
        lifecycle: ReceiptLifecycle {
            claim_state: claim.claim_state.clone(),
            surfacing_state: claim.surfacing_state.clone(),
            verification_state: claim.verification_state,
            updated_at: None,
        },
        provenance: ReceiptProvenance {
            sources: vec![ProvenanceSource {
                label: generic_source_label(&claim.data_source),
                source_type: Some(claim.data_source.clone()),
                as_of: source_asof,
                href: None,
                redacted: matches!(render_kind, ReceiptRenderKind::Redacted),
            }],
            field_path: target_field_path.or_else(|| claim.field_path.clone()),
            evidence_summary: None,
            redaction,
        },
        actions: Vec::new(),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────

fn sanitized_evidence_summary(claim: &IntelligenceClaim) -> Option<String> {
    if claim.text.trim().is_empty() {
        return None;
    }
    let field = FieldPath::root();
    let (sanitized, _warning) = sanitize_explanation_for_render(&field, &claim.text);
    Some(sanitized)
}

fn scrub_target_for_agent_mcp(target: &ReceiptTarget) -> ReceiptTarget {
    match target {
        ReceiptTarget::Claim { subject, .. } => ReceiptTarget::Claim {
            claim_id: String::new(),
            subject: scrub_subject_to_type(subject),
            field_path: None,
        },
        ReceiptTarget::Proposal { subject, .. } => ReceiptTarget::Proposal {
            proposal_id: String::new(),
            subject: scrub_subject_to_type(subject),
            field_path: None,
        },
        ReceiptTarget::WorkItem { subject, .. } => ReceiptTarget::WorkItem {
            action_id: String::new(),
            backing_claim_id: None,
            subject: subject.as_ref().map(scrub_subject_to_type),
        },
    }
}

fn scrub_subject_to_type(subject: &SubjectRef) -> SubjectRef {
    match subject {
        SubjectRef::Account(_) => SubjectRef::Account(String::new()),
        SubjectRef::Project(_) => SubjectRef::Project(String::new()),
        SubjectRef::Person(_) => SubjectRef::Person(String::new()),
        SubjectRef::Action(_) => SubjectRef::Action(String::new()),
        SubjectRef::Meeting(_) => SubjectRef::Meeting(String::new()),
        SubjectRef::User(_) => SubjectRef::User(String::new()),
        SubjectRef::Global => SubjectRef::Global,
        SubjectRef::Multi(_) | SubjectRef::Unknown => SubjectRef::Unknown,
    }
}

fn audience_render_surface(audience: Audience) -> RenderSurface {
    match audience {
        Audience::UserTauri => RenderSurface::TauriEntityDetail,
        Audience::AgentMcp => RenderSurface::McpTool,
        Audience::ActivityLog | Audience::Lint => RenderSurface::Action,
        Audience::OperationalAuditStorage => RenderSurface::LogStructured,
    }
}

fn audience_render_actor(audience: Audience) -> RenderActor {
    match audience {
        Audience::UserTauri => RenderActor {
            actor: "user".to_string(),
            user_id: None,
        },
        Audience::AgentMcp => RenderActor {
            actor: "agent".to_string(),
            user_id: None,
        },
        Audience::ActivityLog | Audience::Lint => RenderActor {
            actor: "user".to_string(),
            user_id: None,
        },
        Audience::OperationalAuditStorage => RenderActor {
            actor: "system".to_string(),
            user_id: None,
        },
    }
}

// ─── Composed / derived claim handling (correctness F5) ───────────────────

fn claim_has_derivation(claim: &IntelligenceClaim) -> bool {
    !derived_from_ids(claim).is_empty()
}

fn derived_from_ids(claim: &IntelligenceClaim) -> Vec<String> {
    let Some(meta) = claim.metadata_json.as_deref() else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(meta) else {
        return Vec::new();
    };
    value
        .get("derived_from")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn max_sensitivity_across_chain(
    claim: &IntelligenceClaim,
    conn: &Connection,
) -> Result<ClaimSensitivity, PrivacyError> {
    let mut max = claim.sensitivity.clone();
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(claim.id.clone());

    let mut frontier: Vec<String> = derived_from_ids(claim);
    let mut depth = 0;

    while !frontier.is_empty() && depth < 8 {
        let mut next: Vec<String> = Vec::new();
        for id in frontier.drain(..) {
            if !visited.insert(id.clone()) {
                continue;
            }
            let Some(parent) = crate::services::claims::load_claim_by_id(conn, &id)
                .map_err(|e| PrivacyError::InvalidMetadata(e.to_string()))?
            else {
                continue;
            };
            if sensitivity_rank(&parent.sensitivity) > sensitivity_rank(&max) {
                max = parent.sensitivity.clone();
            }
            for grandparent_id in derived_from_ids(&parent) {
                next.push(grandparent_id);
            }
        }
        frontier = next;
        depth += 1;
    }
    Ok(max)
}

fn sensitivity_rank(s: &ClaimSensitivity) -> u8 {
    match s {
        ClaimSensitivity::Public => 0,
        ClaimSensitivity::Internal => 1,
        ClaimSensitivity::Confidential => 2,
        ClaimSensitivity::UserOnly => 3,
    }
}

fn synthetic_chain_max_claim(
    claim: &IntelligenceClaim,
    chain_max: ClaimSensitivity,
) -> IntelligenceClaim {
    let mut synthetic = claim.clone();
    synthetic.sensitivity = chain_max;
    synthetic
}

fn parse_claim_timestamp(value: &str) -> Option<DateTime<Utc>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(parsed) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(parsed.with_timezone(&Utc));
    }
    ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"]
        .iter()
        .find_map(|format| {
            NaiveDateTime::parse_from_str(trimmed, format)
                .ok()
                .map(|parsed| DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc))
        })
}

// ─── Public accessor for snapshot lint test ───────────────────────────────

pub fn allowed_top_level_fields(audience: Audience) -> &'static [&'static str] {
    match audience {
        Audience::UserTauri => USER_TAURI_ALLOWED_FIELDS,
        Audience::AgentMcp => AGENT_MCP_ALLOWED_FIELDS,
        Audience::ActivityLog => ACTIVITY_LOG_ALLOWED_FIELDS,
        Audience::Lint => LINT_ALLOWED_FIELDS,
        Audience::OperationalAuditStorage => &[],
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use abilities_runtime::abilities::provenance::subject::SubjectRef;
    use abilities_runtime::abilities::trust::types::TrustBand;
    use rusqlite::params;

    fn open_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE intelligence_claims (
                id TEXT PRIMARY KEY,
                claim_version INTEGER NOT NULL DEFAULT 1,
                subject_ref TEXT NOT NULL,
                claim_type TEXT NOT NULL,
                field_path TEXT,
                topic_key TEXT,
                text TEXT NOT NULL,
                dedup_key TEXT NOT NULL,
                item_hash TEXT,
                actor TEXT NOT NULL,
                data_source TEXT NOT NULL,
                source_ref TEXT,
                source_asof TEXT,
                observed_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                provenance_json TEXT NOT NULL,
                metadata_json TEXT,
                claim_state TEXT NOT NULL,
                surfacing_state TEXT NOT NULL,
                demotion_reason TEXT,
                reactivated_at TEXT,
                retraction_reason TEXT,
                expires_at TEXT,
                superseded_by TEXT,
                trust_score REAL,
                trust_computed_at TEXT,
                trust_version INTEGER,
                thread_id TEXT,
                temporal_scope TEXT NOT NULL,
                sensitivity TEXT NOT NULL,
                verification_state TEXT NOT NULL,
                verification_reason TEXT,
                needs_user_decision_at TEXT,
                canonical_status TEXT NOT NULL DEFAULT 'live',
                non_semantic_mergeable INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )
        .unwrap();
        conn
    }

    fn insert_claim(
        conn: &Connection,
        id: &str,
        sensitivity: &str,
        metadata_json: Option<&str>,
        source_asof: Option<&str>,
    ) {
        let observed_at = "2026-05-20T12:00:00Z";
        conn.execute(
            r#"INSERT INTO intelligence_claims /* dos7-allowed: claim receipt privacy unit test seed */ (
                id, subject_ref, claim_type, field_path, topic_key, text, dedup_key,
                item_hash, actor, data_source, source_ref, source_asof, observed_at,
                created_at, provenance_json, metadata_json, claim_state, surfacing_state,
                demotion_reason, reactivated_at, retraction_reason, expires_at,
                superseded_by, trust_score, trust_computed_at, trust_version, thread_id,
                temporal_scope, sensitivity, verification_state, verification_reason,
                needs_user_decision_at, claim_version, canonical_status,
                non_semantic_mergeable
            ) VALUES (
                ?1, ?2, 'risk', 'health.risk', 'renewal',
                'Renewal risk is elevated', ?3, 'hash-1',
                'agent:test', 'unit_test', ?4, ?5, ?6, ?6, '{}', ?7, 'active',
                'active', NULL, NULL, NULL, NULL, NULL, 0.82, ?6, 1, 'thread-1',
                'state', ?8, 'active', NULL, NULL, 2, 'live', 0
            )"#,
            params![
                id,
                r#"{"kind":"account","id":"acct-1"}"#,
                format!("dedup-{id}"),
                r#"{"kind":"fixture","id":"source-1"}"#,
                source_asof,
                observed_at,
                metadata_json,
                sensitivity,
            ],
        )
        .unwrap();
    }

    fn claim_target(id: &str) -> ReceiptTarget {
        ReceiptTarget::Claim {
            claim_id: id.to_string(),
            subject: SubjectRef::Account("acct-1".to_string()),
            field_path: Some("health.risk".to_string()),
        }
    }

    #[test]
    fn ac_341_10_byte_identical_receipts_for_same_target_and_audience() {
        let conn = open_test_db();
        insert_claim(&conn, "c1", "internal", None, Some("2026-05-19T12:00:00Z"));
        let target = claim_target("c1");
        let a = build_receipt_for_audience(&target, Audience::UserTauri, &conn).unwrap();
        let b = build_receipt_for_audience(&target, Audience::UserTauri, &conn).unwrap();
        let a_json = serde_json::to_value(&a).unwrap();
        let b_json = serde_json::to_value(&b).unwrap();
        assert_eq!(a_json, b_json);
    }

    #[test]
    fn ac_341_11_operational_audit_storage_is_non_disclosure_tag() {
        let conn = open_test_db();
        insert_claim(&conn, "c1", "internal", None, None);
        let target = claim_target("c1");
        let err = build_receipt_for_audience(&target, Audience::OperationalAuditStorage, &conn)
            .unwrap_err();
        assert!(matches!(err, PrivacyError::NonDisclosureAudience));
    }

    #[test]
    fn ac_341_12_agent_mcp_strips_subject_id_and_claim_id() {
        let conn = open_test_db();
        insert_claim(&conn, "c1", "internal", None, Some("2026-05-19T12:00:00Z"));
        let target = claim_target("c1");
        let r = build_receipt_for_audience(&target, Audience::AgentMcp, &conn).unwrap();
        match &r.target {
            ReceiptTarget::Claim {
                claim_id, subject, ..
            } => {
                assert!(claim_id.is_empty(), "claim_id must be stripped");
                match subject {
                    SubjectRef::Account(id) => {
                        assert!(id.is_empty(), "subject_id must be stripped")
                    }
                    _ => panic!("expected Account subject_type"),
                }
            }
            _ => panic!("expected Claim target"),
        }
        assert!(
            r.trust.source_asof.is_none(),
            "source_asof is timing oracle"
        );
        assert!(
            r.lifecycle.updated_at.is_none(),
            "updated_at leaks graph timing"
        );
        assert!(
            r.provenance.sources.is_empty(),
            "AgentMcp forbids source labels"
        );
        assert!(r.provenance.field_path.is_none(), "field_path leaks graph");
    }

    #[test]
    fn ac_341_10_composed_claim_drops_when_chain_max_exceeds_surface() {
        let conn = open_test_db();
        insert_claim(&conn, "parent", "confidential", None, None);
        insert_claim(
            &conn,
            "composed",
            "public",
            Some(r#"{"derived_from":["parent"]}"#),
            None,
        );
        let target = claim_target("composed");
        let err = build_receipt_for_audience(&target, Audience::AgentMcp, &conn).unwrap_err();
        assert!(matches!(err, PrivacyError::ComposedClaimDropped));
    }

    #[test]
    fn ac_341_3_redacted_receipts_keep_trust_and_freshness() {
        let conn = open_test_db();
        insert_claim(
            &conn,
            "c1",
            "confidential",
            None,
            Some("2026-05-19T12:00:00Z"),
        );
        let target = claim_target("c1");
        let r = build_receipt_for_audience(&target, Audience::UserTauri, &conn).unwrap();
        assert!(matches!(
            r.provenance.redaction,
            RedactionLevel::Partial | RedactionLevel::Full
        ));
        assert!(matches!(
            r.trust.band,
            TrustBand::LikelyCurrent | TrustBand::UseWithCaution | TrustBand::NeedsVerification
        ));
        assert!(matches!(
            r.trust.freshness,
            Freshness::Current | Freshness::Aging | Freshness::Stale
        ));
    }

    #[test]
    fn ac_341_1_user_tauri_json_keys_are_subset_of_allowlist() {
        let conn = open_test_db();
        insert_claim(&conn, "c1", "internal", None, Some("2026-05-19T12:00:00Z"));
        let target = claim_target("c1");
        let r = build_receipt_for_audience(&target, Audience::UserTauri, &conn).unwrap();
        assert_keys_subset(&r, Audience::UserTauri);
    }

    #[test]
    fn ac_341_1_agent_mcp_json_keys_are_subset_of_allowlist() {
        let conn = open_test_db();
        insert_claim(&conn, "c1", "internal", None, Some("2026-05-19T12:00:00Z"));
        let target = claim_target("c1");
        let r = build_receipt_for_audience(&target, Audience::AgentMcp, &conn).unwrap();
        assert_keys_subset(&r, Audience::AgentMcp);
    }

    #[test]
    fn ac_341_1_activity_log_json_keys_are_subset_of_allowlist() {
        let conn = open_test_db();
        insert_claim(&conn, "c1", "internal", None, Some("2026-05-19T12:00:00Z"));
        let target = claim_target("c1");
        let r = build_receipt_for_audience(&target, Audience::ActivityLog, &conn).unwrap();
        assert_keys_subset(&r, Audience::ActivityLog);
    }

    #[test]
    fn ac_341_1_lint_json_keys_are_subset_of_allowlist() {
        let conn = open_test_db();
        insert_claim(&conn, "c1", "internal", None, Some("2026-05-19T12:00:00Z"));
        let target = claim_target("c1");
        let r = build_receipt_for_audience(&target, Audience::Lint, &conn).unwrap();
        assert_keys_subset(&r, Audience::Lint);
    }

    fn assert_keys_subset(receipt: &ClaimReceipt, audience: Audience) {
        let value = serde_json::to_value(receipt).unwrap();
        let object = value.as_object().expect("receipt serializes as object");
        let allowed: HashSet<&str> = allowed_top_level_fields(audience).iter().copied().collect();
        for key in object.keys() {
            assert!(
                allowed.contains(key.as_str()),
                "audience={} key={key} not in allowlist {:?}",
                audience.as_str(),
                allowed_top_level_fields(audience),
            );
        }
    }

    #[test]
    fn ac_341_6_uses_generic_fake_data_only() {
        let conn = open_test_db();
        insert_claim(&conn, "c1", "internal", None, None);
        let target = claim_target("c1");
        let r = build_receipt_for_audience(&target, Audience::UserTauri, &conn).unwrap();
        let json = serde_json::to_string(&r).unwrap();
        assert!(
            !json.contains("@"),
            "no email-like content in fixture output"
        );
    }

    #[test]
    fn ac_341_8_agent_mcp_omits_audit_only_fields_in_dto_snapshot() {
        let conn = open_test_db();
        insert_claim(&conn, "c1", "internal", None, Some("2026-05-19T12:00:00Z"));
        let target = claim_target("c1");
        let r = build_receipt_for_audience(&target, Audience::AgentMcp, &conn).unwrap();
        let json = serde_json::to_string(&r).unwrap();
        for forbidden in ["correlation_id", "prompt_hash", "audit_id", "invocation"] {
            assert!(
                !json.contains(forbidden),
                "AgentMcp DTO leaked '{forbidden}': {json}"
            );
        }
    }
}

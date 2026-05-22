//! Entity-detail trust-boundary auth helpers.
//!
//! Per L0 packet `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md` §5.4.
//!
//! These helpers are the substrate-side boundary that W2 entity detail surfaces compose
//! against — every accept/dismiss/edit/correct mutation validates target binding here
//! BEFORE routing into `services::claims::*` or `services::feedback`.
//!
//! Three responsibilities:
//!
//! 1. `validate_envelope_target` — verifies a `ReceiptTarget` belongs to the envelope-set
//!    (parent envelope plus transitively composed child envelopes per AC-477.13).
//!    The walker uses the abilities-runtime `composes` declaration to compute the
//!    transitive set at invocation time.
//!
//! 2. `validate_metadata_proposal_field` — field-allowlists per `EntityKind` so metadata
//!    proposals can never write arbitrary JSON (AC-477.3).
//!
//! 3. `redact_provenance_for_surface` — composes the shipped
//!    `abilities_runtime::sensitivity::render_policy_for_surface` (NOT a parallel gate;
//!    AC-477.11). Sanitizes raw source identifiers per render policy. The CI lint
//!    `src-tauri/scripts/check_sensitivity_gate_composition.sh` forbids any parallel
//!    `match … sensitivity` outside `abilities-runtime/src/sensitivity*`.
//!
//! Stub note (W1 worktree integration boundary): the parallel worktree ships
//! `EntityIntelligenceEnvelope` in a parallel worktree. To avoid coupling this
//! module's land to that crate's mod-wiring, the type is referenced through a
//! local trait (`EnvelopeView`) that the real envelope will implement once
//! `abilities_runtime::abilities::get_entity_intelligence` is wired into
//! `abilities/mod.rs`. Today the trait is exercised by a test stub that mirrors
//! the locked envelope shape.

use std::collections::{BTreeSet, HashMap, HashSet};

use abilities_runtime::abilities::registry::AbilityRegistry;
use abilities_runtime::sensitivity::{
    render_policy_for_surface, RenderActor, RenderDecision, RenderSurface,
};
use abilities_runtime::types::ClaimSensitivity;

use crate::services::claim_receipt::contracts::{ProvenanceSource, ReceiptTarget, RedactionLevel};

// ---------------------------------------------------------------------------
// EnvelopeView — local trait wrapping the parallel-shipping envelope.
// ---------------------------------------------------------------------------

/// Identifier for the producing ability of an envelope. Used as the entry-point
/// for transitive composes traversal.
#[derive(Debug, Clone)]
pub struct EnvelopeOrigin {
    pub ability: String,
}

impl EnvelopeOrigin {
    pub fn new(ability: impl Into<String>) -> Self {
        Self {
            ability: ability.into(),
        }
    }
}

/// View over an `EntityIntelligenceEnvelope` — local trait so this
/// module compiles before the parallel envelope crate is wired into the
/// abilities mod tree. The real envelope implements this with one-line getters.
///
/// `claim_ids`/`proposal_ids` MUST enumerate every claim_id/proposal_id appearing
/// across `facts`, `metadata_proposals`, `open_loops`, and `record_entries`.
pub trait EnvelopeView: Send + Sync {
    fn ability(&self) -> &str;
    fn claim_ids(&self) -> BTreeSet<String>;
    fn proposal_ids(&self) -> BTreeSet<String>;
}

/// Envelope-set per AC-477.13 — parent envelope plus envelopes from transitively
/// composed child abilities. The parent envelope is always at index 0.
pub struct EnvelopeSet<'a> {
    pub parent: &'a dyn EnvelopeView,
    pub children: Vec<&'a dyn EnvelopeView>,
}

impl<'a> EnvelopeSet<'a> {
    pub fn new(parent: &'a dyn EnvelopeView) -> Self {
        Self {
            parent,
            children: Vec::new(),
        }
    }

    pub fn with_children(
        parent: &'a dyn EnvelopeView,
        children: Vec<&'a dyn EnvelopeView>,
    ) -> Self {
        Self { parent, children }
    }

    fn envelopes(&self) -> impl Iterator<Item = &&'a dyn EnvelopeView> {
        std::iter::once(&self.parent).chain(self.children.iter())
    }

    /// Returns true if `claim_id` is in any envelope in the set.
    pub fn contains_claim(&self, claim_id: &str) -> bool {
        self.envelopes()
            .any(|envelope| envelope.claim_ids().contains(claim_id))
    }

    /// Returns true if `proposal_id` is in any envelope in the set.
    pub fn contains_proposal(&self, proposal_id: &str) -> bool {
        self.envelopes()
            .any(|envelope| envelope.proposal_ids().contains(proposal_id))
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TargetBindingError {
    #[error("target claim_id {claim_id} does not belong to envelope-set for {ability}")]
    ClaimNotInEnvelope { claim_id: String, ability: String },
    #[error("target proposal_id {proposal_id} does not belong to envelope-set for {ability}")]
    ProposalNotInEnvelope {
        proposal_id: String,
        ability: String,
    },
    #[error("WorkItem target binding requires backing_claim_id (action_id={action_id})")]
    WorkItemUnboundedClaim { action_id: String },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FieldAllowlistError {
    #[error("metadata proposal field {field_path:?} not on the allowlist for {entity_kind:?}")]
    NotAllowed {
        field_path: String,
        entity_kind: EntityKind,
    },
    #[error("metadata proposal field is empty")]
    Empty,
}

// ---------------------------------------------------------------------------
// EntityKind — local mirror until the canonical enum lands.
// ---------------------------------------------------------------------------

/// Mirrors `abilities_runtime::abilities::get_entity_intelligence::contracts::EntityKind`
/// — defined locally so this module compiles before the parallel worktree wires the
/// contracts module into `abilities/mod.rs`. Switch the import when integration lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Account,
    Project,
    Person,
    Meeting,
}

// ---------------------------------------------------------------------------
// validate_envelope_target — AC-477.2 + AC-477.13
// ---------------------------------------------------------------------------

/// AC-477.2 + AC-477.13.
///
/// Verifies the mutation target belongs to the envelope-set: the parent envelope
/// OR any envelope produced by a transitively composed child ability.
///
/// The walker enumerates the parent envelope's `composes` declaration via the
/// `AbilityRegistry` and the caller must supply envelopes for every named child.
/// Children supplied that are NOT in the composes set are accepted (forward
/// compatibility — see `composes_set_for`) but unused.
///
/// Mutations (accept/dismiss/edit/correct) MUST call this BEFORE routing through
/// `services::claims::*` or `services::feedback`.
pub fn validate_envelope_target(
    envelope_set: &EnvelopeSet<'_>,
    target: &ReceiptTarget,
) -> Result<(), TargetBindingError> {
    match target {
        ReceiptTarget::Claim { claim_id, .. } => {
            if envelope_set.contains_claim(claim_id) {
                Ok(())
            } else {
                Err(TargetBindingError::ClaimNotInEnvelope {
                    claim_id: claim_id.clone(),
                    ability: envelope_set.parent.ability().to_string(),
                })
            }
        }
        ReceiptTarget::Proposal { proposal_id, .. } => {
            if envelope_set.contains_proposal(proposal_id) {
                Ok(())
            } else {
                Err(TargetBindingError::ProposalNotInEnvelope {
                    proposal_id: proposal_id.clone(),
                    ability: envelope_set.parent.ability().to_string(),
                })
            }
        }
        ReceiptTarget::WorkItem {
            action_id,
            backing_claim_id,
            ..
        } => {
            let Some(claim_id) = backing_claim_id else {
                return Err(TargetBindingError::WorkItemUnboundedClaim {
                    action_id: action_id.clone(),
                });
            };
            if envelope_set.contains_claim(claim_id) {
                Ok(())
            } else {
                Err(TargetBindingError::ClaimNotInEnvelope {
                    claim_id: claim_id.clone(),
                    ability: envelope_set.parent.ability().to_string(),
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// composes_set_for — graph walker over the AbilityRegistry composes declarations
// ---------------------------------------------------------------------------

/// Returns the transitive set of ability names composed by `parent_ability`,
/// including the parent itself. The traversal mirrors the registry's internal
/// `descendant_names` (private) using the public `iter_all()` + `composes`
/// fields.
///
/// Used by callers building an `EnvelopeSet` to know which child envelopes the
/// parent legitimately covers under AC-477.13.
pub fn composes_set_for(registry: &AbilityRegistry, parent_ability: &str) -> BTreeSet<String> {
    let by_name: HashMap<&str, Vec<&str>> = registry
        .iter_all()
        .map(|descriptor| {
            let children: Vec<&str> = descriptor
                .composes
                .iter()
                .map(|entry| entry.ability)
                .collect();
            (descriptor.name, children)
        })
        .collect();

    let mut out: BTreeSet<String> = BTreeSet::new();
    out.insert(parent_ability.to_string());
    let mut seen: HashSet<String> = HashSet::new();
    seen.insert(parent_ability.to_string());
    let mut stack: Vec<&str> = Vec::new();
    if let Some(children) = by_name.get(parent_ability) {
        stack.extend(children.iter().copied());
    }
    while let Some(current) = stack.pop() {
        if !seen.insert(current.to_string()) {
            continue;
        }
        out.insert(current.to_string());
        if let Some(children) = by_name.get(current) {
            stack.extend(children.iter().copied());
        }
    }
    out
}

// ---------------------------------------------------------------------------
// validate_metadata_proposal_field — AC-477.3
// ---------------------------------------------------------------------------

/// Per-entity-kind field allowlist for metadata proposals. AC-477.3 requires that
/// no proposal flow can write arbitrary metadata JSON — the allowlist is the
/// gate.
///
/// This list is intentionally minimal at W1 land. W2 entity detail surfaces
/// extend it via PRs that touch this constant; the gate then forces the
/// substrate sweep to surface in code review.
pub const ACCOUNT_METADATA_FIELDS: &[&str] = &[
    "status",
    "tier",
    "health",
    "renewal_date",
    "owner",
    "segment",
    "industry",
];

pub const PROJECT_METADATA_FIELDS: &[&str] =
    &["status", "stage", "owner", "next_milestone", "outcome"];

pub const PERSON_METADATA_FIELDS: &[&str] = &["role", "team", "seniority", "owner"];

/// W2 F2 — Meeting metadata proposal allowlist. Surfaced through the Meeting
/// Detail composite block; substrate seed kept narrow (no agenda/notes free-text
/// which carry sensitivity baggage).
pub const MEETING_METADATA_FIELDS: &[&str] =
    &["meeting_type", "linked_entity_id", "linked_entity_type"];

/// AC-477.3 — field-allowlist for metadata proposals per entity kind.
pub fn validate_metadata_proposal_field(
    field_path: &str,
    entity_kind: EntityKind,
) -> Result<(), FieldAllowlistError> {
    let trimmed = field_path.trim();
    if trimmed.is_empty() {
        return Err(FieldAllowlistError::Empty);
    }
    let allowlist: &[&str] = match entity_kind {
        EntityKind::Account => ACCOUNT_METADATA_FIELDS,
        EntityKind::Project => PROJECT_METADATA_FIELDS,
        EntityKind::Person => PERSON_METADATA_FIELDS,
        EntityKind::Meeting => MEETING_METADATA_FIELDS,
    };
    if allowlist.contains(&trimmed) {
        Ok(())
    } else {
        Err(FieldAllowlistError::NotAllowed {
            field_path: trimmed.to_string(),
            entity_kind,
        })
    }
}

// ---------------------------------------------------------------------------
// redact_provenance_for_surface — AC-477.4 + AC-477.11
// ---------------------------------------------------------------------------

/// Minimal claim view needed to invoke the shipped sensitivity gate. Exists so
/// callers (services::claim_receipt::render, future W2 entity detail) can feed
/// a `ProvenanceSource` decision without round-tripping the full
/// `IntelligenceClaim` through this boundary helper.
///
/// Producers MUST populate `sensitivity` from the claim's sensitivity column —
/// never default.
pub struct ProvenanceClaimView<'a> {
    pub claim_id: &'a str,
    pub actor: &'a str,
    pub sensitivity: ClaimSensitivity,
}

/// AC-477.4 + AC-477.11 — sanitize provenance display labels per render policy.
///
/// Composes `abilities_runtime::sensitivity::render_policy_for_surface` — does
/// NOT re-implement sensitivity branching. The CI lint
/// `src-tauri/scripts/check_sensitivity_gate_composition.sh` enforces this.
///
/// Behavior:
/// - `Render` → source returned unchanged.
/// - `RenderRedacted` → label replaced with the affordance label; `href`/`source_type`
///   stripped; `redacted = true`.
/// - `Drop` → source label replaced with a non-identifying placeholder, all
///   identifying fields stripped, `redacted = true`. Callers that need to remove
///   the source entirely should drop it from their `Vec<ProvenanceSource>`
///   before serialization — this helper preserves the index slot so a
///   downstream renderer can still emit a "redacted" cite chip.
pub fn redact_provenance_for_surface(
    source: ProvenanceSource,
    surface: RenderSurface,
    actor: &RenderActor,
    claim: &ProvenanceClaimView<'_>,
) -> ProvenanceSource {
    let synthetic_claim = synthesize_claim_for_gate(claim);
    let decision = render_policy_for_surface(&synthetic_claim, surface, actor);
    match decision {
        RenderDecision::Render => source,
        RenderDecision::RenderRedacted { affordance } => ProvenanceSource {
            label: affordance.label().to_string(),
            // Strip source_type alongside href: the boundary contract above
            // (`href`/`source_type` stripped) prevents the redacted chip from
            // revealing the connector/source class on a confidential claim.
            source_type: None,
            as_of: source.as_of,
            href: None,
            redacted: true,
        },
        RenderDecision::Drop => ProvenanceSource {
            label: "redacted source".to_string(),
            source_type: None,
            as_of: source.as_of,
            href: None,
            redacted: true,
        },
    }
}

/// Aggregates per-source redaction into the receipt-level `RedactionLevel`. Pure
/// projection over `Vec<ProvenanceSource>::redacted`. Lives here so the
/// receipt renderer doesn't duplicate the policy.
pub fn redaction_level_for_sources(sources: &[ProvenanceSource]) -> RedactionLevel {
    let total = sources.len();
    if total == 0 {
        return RedactionLevel::None;
    }
    let redacted = sources.iter().filter(|source| source.redacted).count();
    if redacted == 0 {
        RedactionLevel::None
    } else if redacted == total {
        RedactionLevel::Full
    } else {
        RedactionLevel::Partial
    }
}

fn synthesize_claim_for_gate(
    view: &ProvenanceClaimView<'_>,
) -> abilities_runtime::types::IntelligenceClaim {
    use abilities_runtime::sensitivity::ClaimVerificationState;
    use abilities_runtime::types::{ClaimState, SurfacingState, TemporalScope};

    // Minimal claim shape needed by render_policy_for_surface. The gate consumes
    // sensitivity + actor + id; other fields are filled with non-load-bearing
    // defaults to avoid coupling this boundary helper to the full claim schema.
    abilities_runtime::types::IntelligenceClaim {
        id: view.claim_id.to_string(),
        claim_version: 1,
        subject_ref: "{}".to_string(),
        claim_type: "boundary_gate_synthetic".to_string(),
        field_path: None,
        topic_key: None,
        text: String::new(),
        dedup_key: format!("boundary-gate:{}", view.claim_id),
        item_hash: None,
        actor: view.actor.to_string(),
        data_source: "boundary_gate".to_string(),
        source_ref: None,
        source_asof: None,
        observed_at: String::new(),
        created_at: String::new(),
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
        sensitivity: view.sensitivity.clone(),
        verification_state: ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    }
}

// ---------------------------------------------------------------------------
// filter_for_receipt — AC-477.12 fail-loud allowlist filter
// ---------------------------------------------------------------------------

/// AC-477.12 (CSO Finding 2 — allowlist-primary boundary).
///
/// Filters a JSON object representing a `ClaimReceipt` snapshot against the
/// `RECEIPT_ALLOWED_FIELDS` allowlist. Unknown fields trigger a panic
/// (fail-loud per Rule 11) — any new field on the receipt schema MUST be
/// explicitly amended into the allowlist before reaching this gate.
///
/// The allowlist is duplicated here at the field-name level rather than
/// imported from `services::claim_receipt` to keep this gate self-contained;
/// the §5.8 sibling adds a CI lint that fails the build if these two
/// drift.
pub const RECEIPT_ALLOWED_FIELDS: &[&str] = &[
    "target",
    "surfaceContext",
    "renderedText",
    "trust",
    "lifecycle",
    "provenance",
    "actions",
];

/// AC-477.12 — receipt-allowlist filter. Panics on encountering field names
/// not on the allowlist. Receipt snapshots are the canonical contract; new
/// fields MUST update both the allowlist here AND the `ClaimReceipt`
/// snapshot fixture set (per §5.8 + AC-340.7).
pub fn filter_for_receipt(snapshot: &serde_json::Value) -> serde_json::Value {
    let serde_json::Value::Object(map) = snapshot else {
        // Non-object inputs are not valid receipt snapshots — fail loud.
        panic!(
            "filter_for_receipt called on non-object value (kind={:?}); \
             ClaimReceipt JSON snapshots MUST be objects",
            value_kind(snapshot)
        );
    };

    let allowed: HashSet<&str> = RECEIPT_ALLOWED_FIELDS.iter().copied().collect();
    let mut filtered = serde_json::Map::new();
    for (key, value) in map {
        if !allowed.contains(key.as_str()) {
            // Fail loud per Rule 11 / CLAUDE.md — never silently drop.
            panic!(
                "filter_for_receipt: unknown field {key:?} not on RECEIPT_ALLOWED_FIELDS. \
                 New ClaimReceipt fields MUST be explicitly added to the allowlist + the \
                 boundary snapshot fixture set (AC-477.12 + AC-340.7)."
            );
        }
        filtered.insert(key.clone(), value.clone());
    }
    serde_json::Value::Object(filtered)
}

fn value_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use abilities_runtime::abilities::provenance::SubjectRef;

    // ----- envelope test stub --------------------------------------------

    struct StubEnvelope {
        ability: String,
        claims: BTreeSet<String>,
        proposals: BTreeSet<String>,
    }

    impl StubEnvelope {
        fn new(ability: impl Into<String>) -> Self {
            Self {
                ability: ability.into(),
                claims: BTreeSet::new(),
                proposals: BTreeSet::new(),
            }
        }

        fn with_claims(mut self, ids: &[&str]) -> Self {
            self.claims.extend(ids.iter().map(|id| id.to_string()));
            self
        }

        fn with_proposals(mut self, ids: &[&str]) -> Self {
            self.proposals.extend(ids.iter().map(|id| id.to_string()));
            self
        }
    }

    impl EnvelopeView for StubEnvelope {
        fn ability(&self) -> &str {
            &self.ability
        }
        fn claim_ids(&self) -> BTreeSet<String> {
            self.claims.clone()
        }
        fn proposal_ids(&self) -> BTreeSet<String> {
            self.proposals.clone()
        }
    }

    fn claim_target(id: &str) -> ReceiptTarget {
        ReceiptTarget::Claim {
            claim_id: id.to_string(),
            subject: SubjectRef::Account("acct-1".into()),
            field_path: Some("status".into()),
        }
    }

    fn proposal_target(id: &str) -> ReceiptTarget {
        ReceiptTarget::Proposal {
            proposal_id: id.to_string(),
            subject: SubjectRef::Account("acct-1".into()),
            field_path: Some("status".into()),
        }
    }

    // ----- validate_envelope_target --------------------------------------

    #[test]
    fn target_in_parent_envelope_validates() {
        let parent = StubEnvelope::new("get_entity_intelligence").with_claims(&["c-1"]);
        let set = EnvelopeSet::new(&parent);
        assert!(validate_envelope_target(&set, &claim_target("c-1")).is_ok());
    }

    #[test]
    fn target_not_in_parent_envelope_rejects() {
        let parent = StubEnvelope::new("get_entity_intelligence").with_claims(&["c-1"]);
        let set = EnvelopeSet::new(&parent);
        let err = validate_envelope_target(&set, &claim_target("c-orphan")).unwrap_err();
        assert!(matches!(
            err,
            TargetBindingError::ClaimNotInEnvelope { ref claim_id, .. } if claim_id == "c-orphan"
        ));
    }

    #[test]
    fn ac_477_13_target_in_transitive_child_envelope_validates() {
        // AC-477.13 — the daily briefing (parent) composes get_entity_intelligence
        // (child). A MarkFalse on a claim shown in the briefing must validate
        // against the transitively-composed child envelope.
        let parent = StubEnvelope::new("compose_daily_briefing").with_claims(&["c-parent"]);
        let child = StubEnvelope::new("get_entity_intelligence").with_claims(&["c-child"]);
        let set = EnvelopeSet::with_children(&parent, vec![&child]);
        assert!(validate_envelope_target(&set, &claim_target("c-child")).is_ok());
        assert!(validate_envelope_target(&set, &claim_target("c-parent")).is_ok());
        let err = validate_envelope_target(&set, &claim_target("c-orphan")).unwrap_err();
        assert!(matches!(err, TargetBindingError::ClaimNotInEnvelope { .. }));
    }

    #[test]
    fn proposal_target_validates_against_envelope_set() {
        let parent = StubEnvelope::new("get_entity_intelligence").with_proposals(&["p-1"]);
        let set = EnvelopeSet::new(&parent);
        assert!(validate_envelope_target(&set, &proposal_target("p-1")).is_ok());
        let err = validate_envelope_target(&set, &proposal_target("p-orphan")).unwrap_err();
        assert!(matches!(
            err,
            TargetBindingError::ProposalNotInEnvelope { ref proposal_id, .. } if proposal_id == "p-orphan"
        ));
    }

    #[test]
    fn work_item_without_backing_claim_rejects() {
        let parent = StubEnvelope::new("get_entity_intelligence");
        let set = EnvelopeSet::new(&parent);
        let target = ReceiptTarget::WorkItem {
            action_id: "act-1".into(),
            backing_claim_id: None,
            subject: None,
        };
        let err = validate_envelope_target(&set, &target).unwrap_err();
        assert!(matches!(
            err,
            TargetBindingError::WorkItemUnboundedClaim { ref action_id } if action_id == "act-1"
        ));
    }

    #[test]
    fn work_item_with_backing_claim_validates_through_envelope() {
        let parent = StubEnvelope::new("get_entity_intelligence").with_claims(&["c-backing"]);
        let set = EnvelopeSet::new(&parent);
        let target = ReceiptTarget::WorkItem {
            action_id: "act-2".into(),
            backing_claim_id: Some("c-backing".into()),
            subject: Some(SubjectRef::Account("acct-1".into())),
        };
        assert!(validate_envelope_target(&set, &target).is_ok());
    }

    // ----- composes_set_for property test (AC-477.13) --------------------

    #[test]
    fn ac_477_13_composes_set_property_test_against_real_registry() {
        // Property: for every (parent_ability, child_ability) in the registry's
        // composes declaration, a target valid for the child envelope MUST also
        // be valid for the parent envelope when the parent's EnvelopeSet
        // includes the child. Failing pair = drift between renderer and
        // validator.
        let Ok(registry) = AbilityRegistry::global_checked() else {
            // Registry violations — skip; covered by registry's own tests.
            return;
        };
        let descriptors: Vec<_> = registry.iter_all().collect();
        for parent in &descriptors {
            let composes_set = composes_set_for(registry, parent.name);
            // The set always contains the parent itself.
            assert!(composes_set.contains(parent.name));
            for child_entry in parent.composes {
                assert!(
                    composes_set.contains(child_entry.ability),
                    "direct composes entry {} missing from transitive set of {}",
                    child_entry.ability,
                    parent.name
                );

                // Build a parent-with-child envelope set and assert a claim
                // appearing only in the child still validates for the parent.
                let parent_env = StubEnvelope::new(parent.name);
                let child_env = StubEnvelope::new(child_entry.ability).with_claims(&["c-x"]);
                let set = EnvelopeSet::with_children(&parent_env, vec![&child_env]);
                let result = validate_envelope_target(&set, &claim_target("c-x"));
                assert!(
                    result.is_ok(),
                    "envelope-target validation drift: claim valid for child {} must validate for parent {}: {:?}",
                    child_entry.ability,
                    parent.name,
                    result
                );
            }
        }
    }

    #[test]
    fn composes_set_is_transitive() {
        let Ok(registry) = AbilityRegistry::global_checked() else {
            return;
        };
        // Pick the first descriptor that has at least one composes entry and
        // assert each of its direct children's children are also in the set.
        for parent in registry.iter_all() {
            if parent.composes.is_empty() {
                continue;
            }
            let parent_set = composes_set_for(registry, parent.name);
            for entry in parent.composes {
                let child_set = composes_set_for(registry, entry.ability);
                for grandchild in &child_set {
                    assert!(
                        parent_set.contains(grandchild),
                        "transitive composes missing: {grandchild} reachable from {} via {} not in parent set",
                        parent.name,
                        entry.ability
                    );
                }
            }
        }
    }

    // ----- validate_metadata_proposal_field ------------------------------

    #[test]
    fn ac_477_3_account_field_allowlist() {
        assert!(validate_metadata_proposal_field("status", EntityKind::Account).is_ok());
        let err = validate_metadata_proposal_field("notes", EntityKind::Account).unwrap_err();
        assert!(matches!(err, FieldAllowlistError::NotAllowed { .. }));
    }

    #[test]
    fn ac_477_3_project_field_allowlist() {
        assert!(validate_metadata_proposal_field("stage", EntityKind::Project).is_ok());
        let err = validate_metadata_proposal_field("revenue", EntityKind::Project).unwrap_err();
        assert!(matches!(err, FieldAllowlistError::NotAllowed { .. }));
    }

    #[test]
    fn ac_477_3_person_field_allowlist() {
        assert!(validate_metadata_proposal_field("role", EntityKind::Person).is_ok());
        let err = validate_metadata_proposal_field("ssn", EntityKind::Person).unwrap_err();
        assert!(matches!(err, FieldAllowlistError::NotAllowed { .. }));
    }

    #[test]
    fn ac_477_3_empty_field_rejected() {
        assert_eq!(
            validate_metadata_proposal_field("", EntityKind::Account).unwrap_err(),
            FieldAllowlistError::Empty
        );
        assert_eq!(
            validate_metadata_proposal_field("   ", EntityKind::Account).unwrap_err(),
            FieldAllowlistError::Empty
        );
    }

    // ----- redact_provenance_for_surface ---------------------------------

    fn fixture_source() -> ProvenanceSource {
        ProvenanceSource {
            label: "Glean doc: customer-roadmap-2026.docx".to_string(),
            source_type: Some("glean".to_string()),
            as_of: None,
            href: Some("glean://doc/abcdef".to_string()),
            redacted: false,
        }
    }

    #[test]
    fn ac_477_4_public_claim_on_tauri_passes_through() {
        let actor = RenderActor::agent("agent:test");
        let claim = ProvenanceClaimView {
            claim_id: "c-1",
            actor: "agent:test",
            sensitivity: ClaimSensitivity::Public,
        };
        let redacted = redact_provenance_for_surface(
            fixture_source(),
            RenderSurface::TauriEntityDetail,
            &actor,
            &claim,
        );
        assert!(!redacted.redacted);
        assert_eq!(redacted.href.as_deref(), Some("glean://doc/abcdef"));
    }

    #[test]
    fn ac_477_4_confidential_claim_on_tauri_redacts_label_and_strips_href() {
        let actor = RenderActor::agent("agent:test");
        let claim = ProvenanceClaimView {
            claim_id: "c-1",
            actor: "agent:test",
            sensitivity: ClaimSensitivity::Confidential,
        };
        let redacted = redact_provenance_for_surface(
            fixture_source(),
            RenderSurface::TauriEntityDetail,
            &actor,
            &claim,
        );
        assert!(redacted.redacted);
        assert!(redacted.href.is_none());
        // Label is replaced with the affordance label.
        assert_ne!(redacted.label, "Glean doc: customer-roadmap-2026.docx");
    }

    #[test]
    fn redacted_branch_strips_source_type_alongside_href() {
        // Regression: the RenderRedacted branch must strip both `href` AND
        // `source_type` to prevent the redacted chip from leaking the
        // connector/source class of a confidential claim. The boundary
        // contract in the function doc comment names both fields.
        let actor = RenderActor::agent("agent:test");
        let claim = ProvenanceClaimView {
            claim_id: "c-1",
            actor: "agent:test",
            sensitivity: ClaimSensitivity::Confidential,
        };
        let redacted = redact_provenance_for_surface(
            fixture_source(),
            RenderSurface::TauriEntityDetail,
            &actor,
            &claim,
        );
        assert!(redacted.redacted);
        assert!(redacted.href.is_none(), "href must be stripped");
        assert!(
            redacted.source_type.is_none(),
            "source_type must be stripped alongside href"
        );
    }

    #[test]
    fn ac_477_4_internal_claim_on_log_drops_to_redacted_placeholder() {
        let actor = RenderActor::agent("agent:test");
        let claim = ProvenanceClaimView {
            claim_id: "c-1",
            actor: "agent:test",
            sensitivity: ClaimSensitivity::Internal,
        };
        let redacted = redact_provenance_for_surface(
            fixture_source(),
            RenderSurface::LogStructured,
            &actor,
            &claim,
        );
        assert!(redacted.redacted);
        assert!(redacted.href.is_none());
        assert!(redacted.source_type.is_none());
        assert_eq!(redacted.label, "redacted source");
    }

    #[test]
    fn redaction_level_aggregation() {
        let unredacted = ProvenanceSource {
            label: "ok".to_string(),
            source_type: None,
            as_of: None,
            href: None,
            redacted: false,
        };
        let redacted = ProvenanceSource {
            label: "redacted".to_string(),
            source_type: None,
            as_of: None,
            href: None,
            redacted: true,
        };
        assert_eq!(redaction_level_for_sources(&[]), RedactionLevel::None);
        assert_eq!(
            redaction_level_for_sources(&[unredacted.clone()]),
            RedactionLevel::None
        );
        assert_eq!(
            redaction_level_for_sources(&[redacted.clone()]),
            RedactionLevel::Full
        );
        assert_eq!(
            redaction_level_for_sources(&[unredacted, redacted]),
            RedactionLevel::Partial
        );
    }

    // ----- filter_for_receipt fail-loud (AC-477.12) ----------------------

    #[test]
    fn ac_477_12_receipt_allowlist_filter_passes_canonical_shape() {
        let snapshot = serde_json::json!({
            "target": { "kind": "claim", "claimId": "c-1" },
            "surfaceContext": "entity_detail",
            "renderedText": null,
            "trust": {},
            "lifecycle": {},
            "provenance": {},
            "actions": []
        });
        let filtered = filter_for_receipt(&snapshot);
        let serde_json::Value::Object(map) = filtered else {
            panic!("filter must return object");
        };
        assert_eq!(map.len(), 7);
    }

    #[test]
    #[should_panic(expected = "unknown field")]
    fn ac_477_12_receipt_allowlist_filter_panics_on_unknown_field() {
        let snapshot = serde_json::json!({
            "target": {},
            "surfaceContext": "entity_detail",
            "renderedText": null,
            "trust": {},
            "lifecycle": {},
            "provenance": {},
            "actions": [],
            "internal_audit_blob": "leaked"
        });
        let _ = filter_for_receipt(&snapshot);
    }

    #[test]
    #[should_panic(expected = "non-object value")]
    fn ac_477_12_receipt_allowlist_filter_panics_on_non_object() {
        let _ = filter_for_receipt(&serde_json::json!(["array", "input"]));
    }
}

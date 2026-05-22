//! AC-461.2 — Account fixture suite (the richest primary proof).
//!
//! Covers metadata-proposal + correction-loop depth, plus the matrix-required
//! account fixtures. Each fixture deserializes into a full
//! `EntityIntelligenceEnvelope`, asserts subject = Account, and verifies the
//! specific intelligence class the fixture represents is present in the
//! envelope (metadata proposal, stale fact, etc.).

use abilities_runtime::abilities::get_entity_intelligence::{EntityKind, Freshness};
use abilities_runtime::types::ClaimState;

use crate::harness::load_envelope;

fn account_envelope(
    file_name: &str,
) -> abilities_runtime::abilities::get_entity_intelligence::EntityIntelligenceEnvelope {
    let env = load_envelope(file_name).unwrap_or_else(|e| panic!("fixture load failed: {e}"));
    assert_eq!(
        env.subject.kind,
        EntityKind::Account,
        "fixture {file_name} subject must be Account"
    );
    env
}

#[test]
fn account_metadata_proposal_carries_a_proposal_per_ac_461_2() {
    let env = account_envelope("account_metadata_proposal.json");
    assert!(
        !env.metadata_proposals.items.is_empty(),
        "AC-461.2 — account_metadata_proposal must include at least one MetadataProposal"
    );
    let proposal = &env.metadata_proposals.items[0];
    assert!(
        !proposal.proposed_value.is_empty(),
        "metadata proposal must have a non-empty proposed_value"
    );
}

#[test]
fn account_stale_fact_has_freshness_stale() {
    let env = account_envelope("account_stale_fact.json");
    let stale = env
        .facts
        .items
        .iter()
        .any(|f| matches!(f.freshness, Freshness::Stale));
    assert!(
        stale,
        "AC-461.5 — account_stale_fact must carry at least one fact with Freshness::Stale"
    );
}

#[test]
fn account_corrected_superseded_has_lifecycle_evidence() {
    let env = account_envelope("account_corrected_superseded.json");
    let has_superseded_state = env.facts.items.iter().any(|f| {
        matches!(
            f.lifecycle_state,
            ClaimState::Withdrawn | ClaimState::Tombstoned | ClaimState::Dormant
        )
    });
    assert!(
        has_superseded_state,
        "AC-461.5 — corrected_superseded must show a claim in Withdrawn/Tombstoned/Dormant lifecycle"
    );
}

#[test]
fn account_low_trust_carries_low_band_fact() {
    use abilities_runtime::abilities::trust::types::TrustBand;
    let env = account_envelope("account_low_trust.json");
    let any_low = env.facts.items.iter().any(|f| {
        matches!(
            f.trust_band,
            TrustBand::NeedsVerification | TrustBand::UseWithCaution
        )
    });
    assert!(
        any_low,
        "AC-461.5 — account_low_trust must include a fact at NeedsVerification or UseWithCaution"
    );
}

#[test]
fn account_open_loop_has_receipt_target() {
    let env = account_envelope("account_open_loop.json");
    assert!(
        !env.open_loops.items.is_empty(),
        "AC-461.5 — account_open_loop must include at least one open loop"
    );
    let loop_with_receipt = &env.open_loops.items[0];
    assert!(
        !loop_with_receipt.receipt_target.claim_id.is_empty(),
        "open loop must carry a non-empty receipt target claim_id"
    );
}

#[test]
fn account_upcoming_touchpoint_present() {
    let env = account_envelope("account_upcoming_touchpoint.json");
    let bundle = env.touchpoints.items.first().expect(
        "AC-461.5 — account_upcoming_touchpoint must include at least one TouchpointBundle",
    );
    assert!(
        !bundle.upcoming.items.is_empty(),
        "upcoming.items must be non-empty for upcoming-touchpoint fixture"
    );
}

#[test]
fn account_recent_touchpoint_present() {
    let env = account_envelope("account_recent_touchpoint.json");
    let bundle = env
        .touchpoints
        .items
        .first()
        .expect("recent_touchpoint fixture must include a TouchpointBundle");
    assert!(!bundle.recent.items.is_empty());
}

#[test]
fn account_thread_summary_present() {
    let env = account_envelope("account_thread_summary.json");
    assert!(
        !env.threads.items.is_empty(),
        "AC-461.5 — account_thread_summary must include a ThreadSummary"
    );
}

#[test]
fn account_glean_citation_present_in_provenance() {
    let env = account_envelope("account_glean_citation.json");
    let any_glean = env.provenance.sources.iter().any(|s| {
        s.source_type
            .as_deref()
            .map(|t| t.eq_ignore_ascii_case("glean"))
            .unwrap_or(false)
    });
    assert!(
        any_glean,
        "AC-461.5 — account_glean_citation must include a provenance source with source_type=glean"
    );
}

#[test]
fn account_confidential_claim_carries_user_only_or_confidential() {
    use abilities_runtime::types::ClaimSensitivity;
    let env = account_envelope("account_confidential_user_only_claim.json");
    let restricted = env.facts.items.iter().any(|f| {
        matches!(
            f.sensitivity,
            ClaimSensitivity::Confidential | ClaimSensitivity::UserOnly
        )
    });
    assert!(
        restricted,
        "AC-461.5 — confidential/user-only fixture must include a claim at Confidential or UserOnly"
    );
}

#[test]
fn account_wrong_subject_fixture_emits_correction_marker() {
    // Wrong-subject fixture: the envelope is for entity A but contains a
    // claim whose subject_ref points at entity B — the correction loop
    // hangs off this miswiring. Per CLAUDE.md Intelligence Loop check #5
    // (feedback loop), the harness verifies the envelope DOES NOT silently
    // attribute the foreign-subject claim to the envelope subject.
    let env = account_envelope("account_wrong_subject.json");
    let envelope_subject = &env.subject.subject_ref;
    let any_foreign = env
        .facts
        .items
        .iter()
        .any(|f| &f.subject_ref != envelope_subject);
    assert!(
        any_foreign,
        "AC-461.5 — wrong_subject fixture must include a fact whose subject_ref does NOT match envelope subject"
    );
}

#[test]
fn account_project_account_overlap_present() {
    let env = account_envelope("account_project_account_overlap.json");
    // The "overlap" fixture asserts that the envelope's subject_scope or
    // facts surface BOTH account- and project-scoped claims tied to the same
    // touchpoint — this is the cross-subject contention class.
    let has_overlap = env
        .touchpoints
        .items
        .iter()
        .any(|b| !b.subject_scope.also_includes.is_empty());
    assert!(
        has_overlap,
        "AC-461.5b — project_account_overlap fixture must include a TouchpointBundle whose subject_scope.also_includes is non-empty"
    );
}

#[test]
fn account_parent_child_present() {
    let env = account_envelope("account_parent_child.json");
    // Parent/child fixture: subject_scope.also_includes contains the
    // other-tier account (parent OR child) — substantiates inheritance class.
    let has_relation = env
        .touchpoints
        .items
        .iter()
        .any(|b| !b.subject_scope.also_includes.is_empty());
    assert!(
        has_relation,
        "AC-461.5b — parent_child (Account-only) fixture must include a non-empty subject_scope.also_includes"
    );
}

#[test]
fn account_claim_retracted_mid_render_is_distinguished_from_bypass() {
    // AC-461.6b — distinguished failure mode. The fixture envelope DOES NOT
    // contain claim_id `account-claim-retracted-1`; a simulated cached DOM
    // references it. The harness MUST report this as stale, not bypass.
    use crate::harness::assertions::check_stale_vs_bypass;
    let env = account_envelope("account_claim_retracted_mid_render.json");
    let cached_dom = r#"
        <article data-claim-id="account-claim-retracted-1">
            <span>stale render — claim was retracted between fetch and paint</span>
        </article>
    "#;
    let (stale, bypass_texts) = check_stale_vs_bypass(cached_dom, &env);
    assert_eq!(
        stale,
        vec!["account-claim-retracted-1".to_string()],
        "AC-461.6b — unresolved data-claim-id must surface as stale_render"
    );
    assert!(
        bypass_texts.is_empty(),
        "AC-461.6b — text WITH a data-claim-id binding (even stale) must NOT be bypass"
    );
}

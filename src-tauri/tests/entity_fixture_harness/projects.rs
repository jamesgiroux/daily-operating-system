//! DOS-461 AC-461.3 — Project fixture suite.
//!
//! Per AC-461.3: covers trajectory / horizon / stakeholder / team context /
//! record entries / work / open loops / touchpoints / trust / provenance /
//! empty states. The matrix (AC-461.5b) also requires
//! `project_metadata_proposal`, the various touchpoint/thread/citation
//! fixtures, `wrong_subject`, and `project_account_overlap`.

use abilities_runtime::abilities::get_entity_intelligence::{EntityKind, Freshness};

use crate::harness::load_envelope;

fn project_envelope(file_name: &str) -> abilities_runtime::abilities::get_entity_intelligence::EntityIntelligenceEnvelope {
    let env = load_envelope(file_name)
        .unwrap_or_else(|e| panic!("fixture load failed: {e}"));
    assert_eq!(
        env.subject.kind,
        EntityKind::Project,
        "fixture {file_name} subject must be Project"
    );
    env
}

#[test]
fn project_metadata_proposal_present() {
    let env = project_envelope("project_metadata_proposal.json");
    assert!(
        !env.metadata_proposals.items.is_empty(),
        "AC-461.3 — project_metadata_proposal must include a proposal"
    );
}

#[test]
fn project_stale_fact_surfaces_freshness_stale() {
    let env = project_envelope("project_stale_fact.json");
    let stale = env.facts.items.iter().any(|f| matches!(f.freshness, Freshness::Stale));
    assert!(stale, "project_stale_fact must include Freshness::Stale");
}

#[test]
fn project_corrected_superseded_has_record_entries() {
    let env = project_envelope("project_corrected_superseded.json");
    // Corrected/superseded leaves a Record entry trail.
    assert!(
        !env.record_entries.items.is_empty(),
        "AC-461.3 — corrected_superseded must include record entries"
    );
}

#[test]
fn project_low_trust_carries_low_band_fact() {
    use abilities_runtime::abilities::trust::types::TrustBand;
    let env = project_envelope("project_low_trust.json");
    let any_low = env.facts.items.iter().any(|f| {
        matches!(
            f.trust_band,
            TrustBand::NeedsVerification | TrustBand::UseWithCaution
        )
    });
    assert!(any_low, "project_low_trust requires a low-band fact");
}

#[test]
fn project_open_loop_carries_owner_or_status() {
    let env = project_envelope("project_open_loop.json");
    let loop_with_receipt = env
        .open_loops
        .items
        .first()
        .expect("project_open_loop must include an open loop");
    let has_signal = loop_with_receipt.open_loop.owner.is_some()
        || loop_with_receipt.open_loop.status.is_some()
        || loop_with_receipt.open_loop.due_date.is_some();
    assert!(
        has_signal,
        "project open loops should carry owner OR status OR due_date (work-tracking shape per AC-461.3)"
    );
}

#[test]
fn project_upcoming_touchpoint_present() {
    let env = project_envelope("project_upcoming_touchpoint.json");
    let bundle = env
        .touchpoints
        .items
        .first()
        .expect("project_upcoming_touchpoint must include a TouchpointBundle");
    assert!(!bundle.upcoming.items.is_empty());
}

#[test]
fn project_recent_touchpoint_present() {
    let env = project_envelope("project_recent_touchpoint.json");
    let bundle = env
        .touchpoints
        .items
        .first()
        .expect("project_recent_touchpoint must include a TouchpointBundle");
    assert!(!bundle.recent.items.is_empty());
}

#[test]
fn project_thread_summary_present() {
    let env = project_envelope("project_thread_summary.json");
    assert!(!env.threads.items.is_empty());
}

#[test]
fn project_glean_citation_in_provenance() {
    let env = project_envelope("project_glean_citation.json");
    let any_glean = env.provenance.sources.iter().any(|s| {
        s.source_type
            .as_deref()
            .map(|t| t.eq_ignore_ascii_case("glean"))
            .unwrap_or(false)
    });
    assert!(any_glean, "project_glean_citation requires glean source");
}

#[test]
fn project_confidential_claim_present() {
    use abilities_runtime::types::ClaimSensitivity;
    let env = project_envelope("project_confidential_user_only_claim.json");
    let restricted = env.facts.items.iter().any(|f| {
        matches!(
            f.sensitivity,
            ClaimSensitivity::Confidential | ClaimSensitivity::UserOnly
        )
    });
    assert!(restricted, "project_confidential_user_only_claim requires restricted sensitivity");
}

#[test]
fn project_wrong_subject_fixture_emits_foreign_subject() {
    let env = project_envelope("project_wrong_subject.json");
    let envelope_subject = &env.subject.subject_ref;
    let any_foreign = env.facts.items.iter().any(|f| &f.subject_ref != envelope_subject);
    assert!(any_foreign, "project_wrong_subject must carry a foreign-subject fact");
}

#[test]
fn project_account_overlap_has_subject_scope_extension() {
    let env = project_envelope("project_project_account_overlap.json");
    let has_overlap = env.touchpoints.items.iter().any(|b| !b.subject_scope.also_includes.is_empty());
    assert!(has_overlap, "project_account_overlap requires subject_scope.also_includes");
}

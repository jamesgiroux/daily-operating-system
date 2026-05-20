//! DOS-461 AC-461.4 — Person fixture suite.
//!
//! Per AC-461.4: covers profile / dynamic / rhythm / network / relationships /
//! record entries / work / open loops / touchpoints / ambiguous-association /
//! trust / provenance / empty states. Matrix slot
//! `person_ambiguous_association.json` is Person-only.

use abilities_runtime::abilities::get_entity_intelligence::{EntityKind, Freshness};

use crate::harness::load_envelope;

fn person_envelope(file_name: &str) -> abilities_runtime::abilities::get_entity_intelligence::EntityIntelligenceEnvelope {
    let env = load_envelope(file_name)
        .unwrap_or_else(|e| panic!("fixture load failed: {e}"));
    assert_eq!(
        env.subject.kind,
        EntityKind::Person,
        "fixture {file_name} subject must be Person"
    );
    env
}

#[test]
fn person_stale_fact_surfaces_freshness_stale() {
    let env = person_envelope("person_stale_fact.json");
    let stale = env.facts.items.iter().any(|f| matches!(f.freshness, Freshness::Stale));
    assert!(stale, "person_stale_fact must include Freshness::Stale");
}

#[test]
fn person_corrected_superseded_has_record_entries() {
    let env = person_envelope("person_corrected_superseded.json");
    assert!(
        !env.record_entries.items.is_empty(),
        "AC-461.4 — corrected_superseded must include record entries"
    );
}

#[test]
fn person_low_trust_carries_low_band_fact() {
    use abilities_runtime::abilities::trust::types::TrustBand;
    let env = person_envelope("person_low_trust.json");
    let any_low = env.facts.items.iter().any(|f| {
        matches!(
            f.trust_band,
            TrustBand::NeedsVerification | TrustBand::UseWithCaution
        )
    });
    assert!(any_low);
}

#[test]
fn person_open_loop_present() {
    let env = person_envelope("person_open_loop.json");
    assert!(!env.open_loops.items.is_empty());
}

#[test]
fn person_upcoming_touchpoint_present() {
    let env = person_envelope("person_upcoming_touchpoint.json");
    let bundle = env.touchpoints.items.first().expect("touchpoint bundle required");
    assert!(!bundle.upcoming.items.is_empty());
}

#[test]
fn person_recent_touchpoint_present() {
    let env = person_envelope("person_recent_touchpoint.json");
    let bundle = env.touchpoints.items.first().expect("touchpoint bundle required");
    assert!(!bundle.recent.items.is_empty());
}

#[test]
fn person_thread_summary_present() {
    let env = person_envelope("person_thread_summary.json");
    assert!(!env.threads.items.is_empty());
}

#[test]
fn person_glean_citation_in_provenance() {
    let env = person_envelope("person_glean_citation.json");
    let any_glean = env.provenance.sources.iter().any(|s| {
        s.source_type
            .as_deref()
            .map(|t| t.eq_ignore_ascii_case("glean"))
            .unwrap_or(false)
    });
    assert!(any_glean);
}

#[test]
fn person_confidential_claim_present() {
    use abilities_runtime::types::ClaimSensitivity;
    let env = person_envelope("person_confidential_user_only_claim.json");
    let restricted = env.facts.items.iter().any(|f| {
        matches!(
            f.sensitivity,
            ClaimSensitivity::Confidential | ClaimSensitivity::UserOnly
        )
    });
    assert!(restricted);
}

#[test]
fn person_wrong_subject_emits_foreign_subject() {
    let env = person_envelope("person_wrong_subject.json");
    let envelope_subject = &env.subject.subject_ref;
    let any_foreign = env.facts.items.iter().any(|f| &f.subject_ref != envelope_subject);
    assert!(any_foreign);
}

#[test]
fn person_ambiguous_association_is_person_only_signal() {
    // AC-461.4 + AC-461.5b — person_ambiguous_association proves the
    // Person-only ambiguity case: same email/name across multiple subjects.
    // The envelope's subject_scope.also_includes is the substrate signal.
    let env = person_envelope("person_ambiguous_association.json");
    let has_ambiguous_scope = env.touchpoints.items.iter().any(|b| {
        !b.subject_scope.also_includes.is_empty()
    });
    assert!(
        has_ambiguous_scope,
        "AC-461.4 — person_ambiguous_association must include subject_scope.also_includes (multi-subject overlap)"
    );
}

use crate::abilities::provenance::SubjectRef;
use crate::abilities::trust::{EntityFootprint, TargetFootprint, TrustConfig};

use super::{cross_entity_coherence, CrossEntityCoherenceInput, CrossEntityHitKind};

fn target_footprint() -> TargetFootprint {
    TargetFootprint {
        subject: SubjectRef::Account("acct-target".to_string()),
        names: vec!["Target Account".to_string()],
        domains: vec!["target.example.com".to_string()],
        related_subjects: Vec::new(),
        allowed_aliases: vec!["TargetCo".to_string()],
    }
}

fn other_footprint() -> EntityFootprint {
    EntityFootprint {
        subject: SubjectRef::Account("acct-other".to_string()),
        names: vec!["Other Company".to_string()],
        domains: vec!["globex.example".to_string()],
        infrastructure_ids: Vec::new(),
    }
}

fn coherence_input(
    claim_text: &str,
    cross_entity_context_expected: bool,
) -> CrossEntityCoherenceInput {
    CrossEntityCoherenceInput {
        claim_text: claim_text.to_string(),
        target_footprint: target_footprint(),
        portfolio_footprints: vec![other_footprint()],
        cross_entity_context_expected,
    }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn cross_entity_coherence_clean_claim_scores_one() {
    let result = cross_entity_coherence(
        &coherence_input("Target Account renewal risk is stable.", false),
        &TrustConfig::default(),
    );

    assert_close(result.value, 1.0);
    assert!(result.hits.is_empty());
    assert!(!result.skipped_expected_context);
}

#[test]
fn cross_entity_coherence_foreign_domain_scores_low_without_rejecting() {
    let config = TrustConfig::default();
    let result = cross_entity_coherence(
        &coherence_input(
            "The renewal note accidentally references globex.example.",
            false,
        ),
        &config,
    );

    assert_close(result.value, 1.0 - config.cross_entity_hit_penalty);
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].kind, CrossEntityHitKind::Domain);
    assert_eq!(result.hits[0].token, "globex.example");
    assert!(!result.skipped_expected_context);
}

#[test]
fn cross_entity_coherence_foreign_vip_host_scores_low() {
    let config = TrustConfig::default();
    let result = cross_entity_coherence(
        &coherence_input(
            "The support note points at vip-foreign.com for the renewal.",
            false,
        ),
        &config,
    );

    assert_close(result.value, 1.0 - config.cross_entity_hit_penalty);
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].kind, CrossEntityHitKind::InfrastructureId);
    assert_eq!(result.hits[0].token, "vip-foreign.com");
}

#[test]
fn cross_entity_coherence_company_name_suppressed_when_target_name_present() {
    let result = cross_entity_coherence(
        &coherence_input(
            "Target Account discussed Other Company in a competitive note.",
            false,
        ),
        &TrustConfig::default(),
    );

    assert_close(result.value, 1.0);
    assert!(result.hits.is_empty());
}

#[test]
fn cross_entity_coherence_allows_target_subdomain() {
    let result = cross_entity_coherence(
        &coherence_input(
            "Target Account traffic moved through vip.app.target.example.com today",
            false,
        ),
        &TrustConfig::default(),
    );

    assert_close(result.value, 1.0);
    assert!(result.hits.is_empty());
}

#[test]
fn cross_entity_coherence_skips_when_context_expected() {
    let result = cross_entity_coherence(
        &coherence_input(
            "Peer benchmark compares Target Account with globex.example.",
            true,
        ),
        &TrustConfig::default(),
    );

    assert_close(result.value, 1.0);
    assert!(result.hits.is_empty());
    assert!(result.skipped_expected_context);
}

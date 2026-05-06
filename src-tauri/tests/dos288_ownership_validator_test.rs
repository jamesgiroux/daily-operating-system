use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::provenance::{
    validate_ability_output_value_ownership, CanonicalSubjectGroup, DataSource, EntityId,
    FieldAttribution, FieldPath, ProvenanceBuilder, ProvenanceBuilderConfig, SourceAttribution,
    SourceEntityLinkEvidence, SourceIdentifier, SourceIndex, SubjectAttribution, SubjectRef,
};
use dailyos_lib::abilities::provenance::{OwnershipError, OwnershipPolicy, OwnershipRenderPolicy};
use serde_json::{json, Value};

fn produced_at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap()
}

fn entity_source(entity_id: &str) -> SourceIdentifier {
    SourceIdentifier::Entity {
        entity_id: EntityId::new(entity_id),
        field: Some("claim".to_string()),
    }
}

fn signal_source() -> SourceIdentifier {
    SourceIdentifier::Signal {
        signal_id: dailyos_lib::abilities::provenance::SignalId::new("signal-1"),
    }
}

fn output_value(subject: SubjectRef, source_identifier: SourceIdentifier) -> Value {
    let subject = SubjectAttribution::direct_confident(subject);
    let mut builder = ProvenanceBuilder::new(ProvenanceBuilderConfig::new(
        "dos288_fixture",
        produced_at(),
    ));
    builder.set_subject(subject.clone());
    let source = SourceAttribution::new(
        DataSource::User,
        vec![source_identifier],
        produced_at(),
        Some(produced_at()),
        1.0,
        None,
    )
    .unwrap();
    let source_index = builder.add_source(source);
    builder
        .attribute(
            FieldPath::new("/claim").unwrap(),
            FieldAttribution::direct(subject, source_index),
        )
        .unwrap();
    let output = builder
        .finalize(json!({ "claim": "Target Example owns the renewal plan." }))
        .unwrap();
    serde_json::to_value(output).unwrap()
}

fn strict_policy() -> OwnershipPolicy {
    OwnershipPolicy::confident().requiring_entity_link_evidence()
}

#[test]
fn ownership_validator_rejects_blocked_field_subject() {
    let mut value = output_value(
        SubjectRef::Account("acct-a".into()),
        entity_source("acct-a"),
    );
    value["provenance"]["field_attributions"]["/claim"]["subject"]["fit"]["status"] =
        json!("blocked");

    let err = validate_ability_output_value_ownership(
        value,
        &[FieldPath::new("/claim").unwrap()],
        strict_policy(),
    )
    .unwrap_err();

    assert!(matches!(
        err,
        OwnershipError::AmbiguousOrBlockedSubjectFit { status, .. }
            if status == dailyos_lib::abilities::provenance::SubjectFitStatus::Blocked
    ));
}

#[test]
fn ownership_validator_rejects_ambiguous_competing_subjects() {
    let mut value = output_value(
        SubjectRef::Account("acct-a".into()),
        entity_source("acct-a"),
    );
    value["provenance"]["field_attributions"]["/claim"]["subject"]["fit"]["status"] =
        json!("ambiguous");
    value["provenance"]["field_attributions"]["/claim"]["subject"]["competing_subjects"] = json!([{
        "subject": serde_json::to_value(SubjectRef::Account("acct-b".into())).unwrap(),
        "confidence": 0.72,
        "reason": "same_domain_fixture"
    }]);

    let err = validate_ability_output_value_ownership(
        value,
        &[FieldPath::new("/claim").unwrap()],
        strict_policy(),
    )
    .unwrap_err();

    assert!(matches!(
        err,
        OwnershipError::AmbiguousOrBlockedSubjectFit { status, .. }
            if status == dailyos_lib::abilities::provenance::SubjectFitStatus::Ambiguous
    ));
}

#[test]
fn ownership_validator_requires_source_ref_entity_link_evidence() {
    let value = output_value(SubjectRef::Account("acct-a".into()), signal_source());

    let err = validate_ability_output_value_ownership(
        value,
        &[FieldPath::new("/claim").unwrap()],
        strict_policy(),
    )
    .unwrap_err();

    assert!(matches!(
        err,
        OwnershipError::SourceRefWithoutEntityLinkEvidence { .. }
    ));
}

#[test]
fn ownership_validator_allows_user_confirmed_subject_override() {
    let mut value = output_value(SubjectRef::Account("acct-a".into()), signal_source());
    value["provenance"]["field_attributions"]["/claim"]["subject"]["subject"] =
        serde_json::to_value(SubjectRef::Account("acct-b".into())).unwrap();
    value["provenance"]["field_attributions"]["/claim"]["subject"]["binding"] =
        json!("user_confirmed");

    let mut policy = strict_policy();
    policy.source_entity_links.push(SourceEntityLinkEvidence {
        source_index: SourceIndex(0),
        subject: SubjectRef::Account("acct-b".into()),
    });

    let report = validate_ability_output_value_ownership(
        value,
        &[FieldPath::new("/claim").unwrap()],
        policy,
    )
    .unwrap();

    assert_eq!(report.render_policy, OwnershipRenderPolicy::Confident);
}

#[test]
fn ownership_validator_rejects_cross_subject_canonical_merge() {
    let value = output_value(
        SubjectRef::Account("acct-a".into()),
        entity_source("acct-a"),
    );
    let mut policy = strict_policy();
    policy.canonical_subject_groups.push(CanonicalSubjectGroup {
        subjects: vec![
            SubjectRef::Account("acct-a".into()),
            SubjectRef::Account("acct-b".into()),
        ],
        explicit_user_confirmed_merge: false,
    });

    let err = validate_ability_output_value_ownership(
        value,
        &[FieldPath::new("/claim").unwrap()],
        policy,
    )
    .unwrap_err();

    assert!(matches!(
        err,
        OwnershipError::CrossSubjectCanonicalMerge { .. }
    ));
}

#[test]
fn ownership_validator_allows_explicit_multi_subject_when_declared() {
    let subject_a = SubjectAttribution::direct_confident(SubjectRef::Account("acct-a".into()));
    let subject_b = SubjectAttribution::direct_confident(SubjectRef::Account("acct-b".into()));
    let mut builder =
        ProvenanceBuilder::new(ProvenanceBuilderConfig::new("dos288_multi", produced_at()));
    builder.set_subject(SubjectAttribution::direct_confident(SubjectRef::Multi(
        vec![
            SubjectRef::Account("acct-a".into()),
            SubjectRef::Account("acct-b".into()),
        ],
    )));
    let source_a = builder.add_source(
        SourceAttribution::new(
            DataSource::User,
            vec![entity_source("acct-a")],
            produced_at(),
            Some(produced_at()),
            1.0,
            None,
        )
        .unwrap(),
    );
    let source_b = builder.add_source(
        SourceAttribution::new(
            DataSource::User,
            vec![entity_source("acct-b")],
            produced_at(),
            Some(produced_at()),
            1.0,
            None,
        )
        .unwrap(),
    );
    builder
        .attribute(
            FieldPath::new("/a").unwrap(),
            FieldAttribution::direct(subject_a, source_a),
        )
        .unwrap()
        .attribute(
            FieldPath::new("/b").unwrap(),
            FieldAttribution::direct(subject_b, source_b),
        )
        .unwrap();
    let value =
        serde_json::to_value(builder.finalize(json!({ "a": "A", "b": "B" })).unwrap()).unwrap();

    let report = validate_ability_output_value_ownership(value, &[], strict_policy()).unwrap();

    assert_eq!(report.render_policy, OwnershipRenderPolicy::Confident);
    assert_eq!(report.rendered_paths_checked.len(), 2);
}

use chrono::TimeZone;
use dailyos_lib::abilities::provenance::{
    DataSource, FieldPath, ProvenanceBuilder, ProvenanceBuilderConfig, ProvenanceError,
    SourceAttribution, SubjectAttribution, SubjectRef,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct FixtureOutput {
    name: String,
    risk: u8,
}

#[test]
fn ability_emitting_unattributed_field_fails_at_finalize() {
    let produced_at = chrono::Utc
        .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
        .unwrap();
    let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-1".into()));
    let source = SourceAttribution::new(
        DataSource::Google,
        Vec::new(),
        produced_at,
        Some(produced_at),
        1.0,
        None,
    )
    .unwrap();
    let mut builder =
        ProvenanceBuilder::new(ProvenanceBuilderConfig::new("fixture_ability", produced_at));
    builder.set_subject(subject.clone());
    let source_index = builder.add_source(source);
    builder
        .pass_through(FieldPath::new("/name").unwrap(), subject, source_index)
        .unwrap();

    let err = builder
        .finalize(FixtureOutput {
            name: "acct-1".into(),
            risk: 3,
        })
        .unwrap_err();

    assert!(matches!(
        err,
        ProvenanceError::MissingFieldAttribution { field_path } if field_path == FieldPath::new("/risk").unwrap()
    ));
}

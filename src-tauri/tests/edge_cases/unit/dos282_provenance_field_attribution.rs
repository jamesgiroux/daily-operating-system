use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::provenance::{
    DataSource, FieldAttribution, FieldPath, ProvenanceBuilder, ProvenanceBuilderConfig,
    SourceAttribution, SourceIdentifier, SubjectAttribution, SubjectRef,
};

#[test]
fn provenance_builder_requires_every_material_field_to_have_attribution() {
    let produced_at = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();
    let subject = SubjectAttribution::direct_confident(SubjectRef::Account("account-example".into()));
    let mut builder = ProvenanceBuilder::new(ProvenanceBuilderConfig::new(
        "dos282_field_attribution",
        produced_at,
    ));
    builder.set_subject(subject.clone());
    let source = builder.add_source(
        SourceAttribution::new(
            DataSource::Google,
            vec![SourceIdentifier::Entity {
                entity_id: dailyos_lib::abilities::provenance::EntityId::new("account-example"),
                field: Some("health".to_string()),
            }],
            produced_at,
            Some(produced_at),
            1.0,
            None,
        )
        .expect("source attribution"),
    );

    builder
        .attribute(
            FieldPath::from_json_pointer("/summary").expect("summary path"),
            FieldAttribution::direct(subject, source),
        )
        .expect("attribute summary");

    let missing = builder
        .finalize(serde_json::json!({
            "summary": "Ready",
            "health": "at_risk"
        }))
        .unwrap_err();
    assert!(
        missing.to_string().contains("/health"),
        "missing field attribution should name the uncovered field: {missing}"
    );
}

use chrono::TimeZone;
use dailyos_lib::abilities::provenance::{
    Confidence, DataSource, EffectiveTrust, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SourceAttribution, SourceRef, SubjectAttribution, SubjectRef,
};
use dailyos_lib::abilities::AbilityCategory;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct FixtureOutput {
    summary: String,
}

#[test]
fn transform_ability_with_llm_synthesis_field_over_trusted_source_is_untrusted() {
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
    let mut config = ProvenanceBuilderConfig::new("summarize_account", produced_at);
    config.category = AbilityCategory::Transform;
    let mut builder = ProvenanceBuilder::new(config);
    builder.set_subject(subject.clone());
    let source_index = builder.add_source(source);
    builder
        .attribute(
            FieldPath::new("/summary").unwrap(),
            FieldAttribution::llm_synthesis(
                subject,
                vec![SourceRef::Source { source_index }],
                Confidence::computed(0.9).unwrap(),
                None,
            )
            .unwrap(),
        )
        .unwrap();

    let output = builder
        .finalize(FixtureOutput {
            summary: "LLM-written summary over trusted Google data".to_string(),
        })
        .unwrap();

    assert!(output.provenance().prompt_fingerprint.is_none());
    assert_eq!(output.provenance().trust.effective, EffectiveTrust::Untrusted);
}

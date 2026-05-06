#[path = "harness/mod.rs"]
mod harness;

use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use dailyos_lib::abilities::provenance::trust::claim_trust_band_from_score;
use dailyos_lib::abilities::provenance::{
    Confidence, DataSource, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SourceAttribution, SourceRef, SubjectAttribution, SubjectRef,
};
use dailyos_lib::abilities::trust::TrustBand;
use dailyos_lib::abilities::AbilityRegistry;
use dailyos_lib::db::claims::IntelligenceClaim;
use harness::bundle_helpers::bundle_fixture_path;
use harness::{load_fixture, prepare_fixture_for_run, run_fixture, RunnerDeps};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
struct SynthesizedFixtureOutput {
    summary: String,
}

#[test]
fn bundle1_min_band_aggregation_uses_most_cautious_claim_source() {
    let fixture = load_fixture(&bundle_fixture_path(1)).expect("bundle-1 fixture loads");
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-1 fixture prepares");
    let likely = claim_by_id(
        &prepared.entity_context_claims,
        "claim-b1-renewal-owner-canonical",
    );
    let needs = claim_by_id(
        &prepared.entity_context_claims,
        "claim-b1-needs-verification-opaque",
    );

    let subject = SubjectAttribution::direct_confident(SubjectRef::Account(
        "dos287-target-example".to_string(),
    ));
    let mut builder = ProvenanceBuilder::new(ProvenanceBuilderConfig::new(
        "dos320_bundle1_min_band_fixture",
        Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap(),
    ));
    builder.set_subject(subject.clone());

    let likely_source = builder.add_source(source_for_claim(likely));
    builder.set_source_trust_band(
        likely_source,
        claim_trust_band_from_score(likely.trust_score),
    );
    let needs_source = builder.add_source(source_for_claim(needs));
    builder.set_source_trust_band(needs_source, claim_trust_band_from_score(needs.trust_score));

    builder
        .attribute(
            FieldPath::new("/summary").unwrap(),
            FieldAttribution::llm_synthesis(
                subject,
                vec![
                    SourceRef::Source {
                        source_index: likely_source,
                    },
                    SourceRef::Source {
                        source_index: needs_source,
                    },
                ],
                Confidence::provider_reported(0.84).unwrap(),
                None,
            )
            .unwrap(),
        )
        .unwrap();

    let output = builder
        .finalize(SynthesizedFixtureOutput {
            summary: "Target Example has mixed renewal evidence.".to_string(),
        })
        .unwrap();
    let attribution = output
        .provenance()
        .field_attributions
        .get(&FieldPath::new("/summary").unwrap())
        .expect("summary attribution");

    assert_eq!(attribution.trust_band, Some(TrustBand::NeedsVerification));
    assert_eq!(
        serde_json::to_value(output.provenance()).unwrap()["field_attributions"]["/summary"]
            ["trust_band"],
        "needs_verification"
    );
}

#[test]
fn bundle5_prepare_meeting_rendered_provenance_carries_claim_bands() {
    let result = run_real_bundle(5);
    let fields = field_attributions(&result.actual_provenance);

    assert_eq!(
        field_band(fields, "/attendee_context/0/context"),
        "likely_current"
    );
    assert_eq!(
        field_band(fields, "/attendee_context/1/context"),
        "likely_current"
    );
    assert_eq!(
        field_band(fields, "/attendee_context/2/context"),
        "likely_current"
    );
}

#[test]
fn bundle13_prepare_meeting_target_field_is_banded_after_subject_filtering() {
    let result = run_real_bundle(13);
    let fields = field_attributions(&result.actual_provenance);

    assert_eq!(field_band(fields, "/topics/0/detail"), "likely_current");
    assert!(serde_json::to_string(&result.actual_output)
        .expect("output serializes")
        .contains("Target Example wants to confirm rollout owner"));
    assert!(!serde_json::to_string(&result.actual_output)
        .expect("output serializes")
        .contains("Adjacent Example has an unrelated"));
}

fn run_real_bundle(bundle: u32) -> harness::RunResult {
    let fixture = load_fixture(&bundle_fixture_path(bundle)).expect("bundle fixture loads");
    let deps = RunnerDeps {
        registry: Arc::new(AbilityRegistry::from_inventory_checked().expect("registry builds")),
    };
    run_fixture(&deps, &fixture).unwrap_or_else(|error| panic!("bundle-{bundle} runs: {error}"))
}

fn field_attributions(rendered_provenance: &Value) -> &serde_json::Map<String, Value> {
    rendered_provenance["value"]["field_attributions"]
        .as_object()
        .expect("rendered provenance field_attributions object")
}

fn field_band<'a>(fields: &'a serde_json::Map<String, Value>, field_path: &str) -> &'a str {
    fields
        .get(field_path)
        .and_then(|field| field.get("trust_band"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("missing trust_band for `{field_path}`"))
}

fn claim_by_id<'a>(claims: &'a [IntelligenceClaim], claim_id: &str) -> &'a IntelligenceClaim {
    claims
        .iter()
        .find(|claim| claim.id == claim_id)
        .unwrap_or_else(|| panic!("missing claim `{claim_id}`"))
}

fn source_for_claim(claim: &IntelligenceClaim) -> SourceAttribution {
    let observed_at = parse_time(&claim.observed_at);
    let source_asof = claim.source_asof.as_deref().map(parse_time);
    SourceAttribution::new(
        DataSource::User,
        Vec::new(),
        observed_at,
        source_asof,
        1.0,
        None,
    )
    .unwrap()
}

fn parse_time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

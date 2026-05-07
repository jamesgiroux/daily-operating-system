#![cfg(feature = "release-gate")]

#[path = "harness/mod.rs"]
mod harness;

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::prepare_meeting::{prepare_meeting, PrepareMeetingInput};
use dailyos_lib::abilities::provenance::{
    validate_subject_ownership, DataSource, EntityId, FieldAttribution, FieldPath, OwnershipError,
    OwnershipPolicy, OwnershipRenderPolicy, ProvenanceBuilder, ProvenanceBuilderConfig,
    SourceAttribution, SourceIdentifier, SubjectAttribution, SubjectRef,
};
use dailyos_lib::abilities::trust::TargetFootprint;
use dailyos_lib::abilities::{AbilityContext, Actor, NOOP_ABILITY_TRACER};
use dailyos_lib::db::claims::{
    ClaimSensitivity, ClaimState, ClaimVerificationState, IntelligenceClaim, SurfacingState,
    TemporalScope,
};
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::trust_extraction::{extract_target_footprint, ExtractionOutcome};
use rusqlite::Connection;
use serde_json::{json, Value};

const TARGET_ACCOUNT_ID: &str = "dos287-target-example";
const FORCE_BLEED_REGRESSION_ENV: &str = "DAILYOS_DOS288_FORCE_BLEED_REGRESSION";

fn force_bleed_regression() -> bool {
    std::env::var_os(FORCE_BLEED_REGRESSION_ENV).is_some()
}

fn produced_at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap()
}

fn output_for_target_text(text: &str) -> dailyos_lib::abilities::provenance::AbilityOutput<Value> {
    let subject =
        SubjectAttribution::direct_confident(SubjectRef::Account(TARGET_ACCOUNT_ID.to_string()));
    let mut builder =
        ProvenanceBuilder::new(ProvenanceBuilderConfig::new("dos288_bundle", produced_at()));
    builder.set_subject(subject.clone());
    let source_index = builder.add_source(
        SourceAttribution::new(
            DataSource::User,
            vec![SourceIdentifier::Entity {
                entity_id: EntityId::new(TARGET_ACCOUNT_ID),
                field: Some("summary".to_string()),
            }],
            produced_at(),
            Some(produced_at()),
            1.0,
            None,
        )
        .unwrap(),
    );
    builder
        .attribute(
            FieldPath::new("/summary").unwrap(),
            FieldAttribution::direct(subject, source_index),
        )
        .unwrap();
    builder.finalize(json!({ "summary": text })).unwrap()
}

fn bundle1_policy() -> OwnershipPolicy {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(include_str!("fixtures/bundle-1/state.sql"))
        .expect("bundle-1 state applies");
    let db = ActionDb::from_conn(&conn);
    let extraction = extract_target_footprint(
        db,
        &SubjectRef::Account(TARGET_ACCOUNT_ID.to_string()),
        "account",
        TARGET_ACCOUNT_ID,
    )
    .expect("extract target footprint");
    let (target_footprint, portfolio_footprints) = match extraction {
        ExtractionOutcome::Ok {
            footprint,
            portfolio_footprints,
        } => (footprint, portfolio_footprints),
        ExtractionOutcome::SkipExtractorMismatch { reason } => {
            panic!("bundle-1 footprint extraction skipped: {reason:?}")
        }
    };
    assert!(
        portfolio_footprints.iter().any(|footprint| {
            footprint.subject == SubjectRef::Account("dos287-adjacent-example".to_string())
        }),
        "bundle-1 must include adjacent account footprint"
    );

    OwnershipPolicy::confident()
        .requiring_entity_link_evidence()
        .with_target_footprint(target_footprint, portfolio_footprints)
}

#[test]
fn bundle1_adjacent_account_content_needs_verification_before_confident_render() {
    let output = output_for_target_text(
        "Blake Branch owns cluster-1.example.com migration risk for Adjacent Example.",
    );

    let err = validate_subject_ownership(
        &output,
        &[FieldPath::new("/summary").unwrap()],
        bundle1_policy(),
    )
    .unwrap_err();

    assert!(matches!(
        err,
        OwnershipError::ConfidentRenderLowCrossEntityCoherence { hit_count, .. }
            if hit_count >= 1
    ));
}

#[test]
fn bundle1_target_account_content_remains_confident() {
    let text = if force_bleed_regression() {
        "Blake Branch owns cluster-1.example.com migration risk for Adjacent Example."
    } else {
        "Alice Adams owns the Target Example renewal plan."
    };
    let output = output_for_target_text(text);

    let report = validate_subject_ownership(
        &output,
        &[FieldPath::new("/summary").unwrap()],
        bundle1_policy(),
    )
    .unwrap();

    assert_eq!(report.render_policy, OwnershipRenderPolicy::Confident);
    assert!(report.cross_entity_coherence_hits.is_empty());
}

#[test]
fn bundle8_private_claim_policy_suppresses_public_confident_render() {
    let fixture = harness::load_fixture(&harness::bundle_helpers::bundle_fixture_path(8))
        .expect("bundle-8 fixture loads");
    assert_eq!(fixture.metadata.bundle, Some(8));

    let output = output_for_target_text("Reusable onboarding guide is ready for public summary.");
    let mut policy = OwnershipPolicy::confident().requiring_entity_link_evidence();
    policy.target_footprint = Some(TargetFootprint {
        subject: SubjectRef::Account(TARGET_ACCOUNT_ID.to_string()),
        names: vec!["Target Example".to_string()],
        domains: vec!["target.example.com".to_string()],
        related_subjects: Vec::new(),
        allowed_aliases: Vec::new(),
    });
    policy.prompt_input_claims = vec![claim_with_sensitivity(
        "claim-test-private-stakeholder-concern",
        ClaimSensitivity::Confidential,
    )];

    let report =
        validate_subject_ownership(&output, &[FieldPath::new("/summary").unwrap()], policy)
            .unwrap();

    assert_eq!(report.prompt_input_claims_checked, 1);
    assert_eq!(report.render_policy, OwnershipRenderPolicy::Suppressed);
}

#[test]
fn bundle13_prepare_meeting_output_passes_validator_without_adjacent_subject() {
    let fixture = harness::load_fixture(&harness::bundle_helpers::bundle_fixture_path(13))
        .expect("bundle-13 fixture loads");
    let prepared = harness::prepare_fixture_for_run(&fixture).expect("bundle-13 prepares");
    let services = prepared.service_context();
    let input: PrepareMeetingInput =
        serde_json::from_value(fixture.inputs_json["input_json"].clone())
            .expect("bundle-13 input parses");
    let ctx = AbilityContext::new(
        &services,
        &prepared.provider,
        &NOOP_ABILITY_TRACER,
        Actor::User,
        None,
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let output = runtime
        .block_on(prepare_meeting(&ctx, input))
        .expect("bundle-13 prepare_meeting succeeds");

    let report = validate_subject_ownership(&output, &[], OwnershipPolicy::confident()).unwrap();

    assert_eq!(report.render_policy, OwnershipRenderPolicy::Confident);
    assert!(output
        .data()
        .topics
        .iter()
        .all(|topic| topic.subject.id != "dos287-adjacent-example"));
    assert!(report
        .rendered_paths_checked
        .iter()
        .any(|path| { path.as_str().starts_with("/topics/0") }));
}

fn claim_with_sensitivity(id: &str, sensitivity: ClaimSensitivity) -> IntelligenceClaim {
    IntelligenceClaim {
        id: id.to_string(),
        subject_ref: json!({
            "kind": "account",
            "id": TARGET_ACCOUNT_ID,
        })
        .to_string(),
        claim_type: "entity_summary".to_string(),
        field_path: Some("summary".to_string()),
        topic_key: None,
        text: String::new(),
        dedup_key: format!("dedup-{id}"),
        item_hash: None,
        actor: "agent:fixture".to_string(),
        data_source: "user".to_string(),
        source_ref: None,
        source_asof: Some("2026-05-06T12:00:00Z".to_string()),
        observed_at: "2026-05-06T12:00:00Z".to_string(),
        created_at: "2026-05-06T12:00:00Z".to_string(),
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
        sensitivity,
        verification_state: ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    }
}

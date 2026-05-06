use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::*;
use crate::abilities::provenance::{
    json_leaf_paths, DerivationKind, FieldPath, ProvenanceWarning, SourceTimestampFallback,
    SubjectBindingKind,
};
use crate::abilities::{Actor, NOOP_ABILITY_TRACER};
use crate::intelligence::provider::ReplayProvider;
use crate::services::context::{
    EntityContextReadFuture, EntityContextReadHandle, FixedClock, SeedableRng, ServiceContext,
};

#[derive(Clone)]
struct FixtureEntityContextReader {
    rows: Vec<EntityContextEntry>,
    filter_requested_subject: bool,
}

impl EntityContextReadHandle for FixtureEntityContextReader {
    fn read_entity_context_entries<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
    ) -> EntityContextReadFuture<'a> {
        Box::pin(async move {
            let mut rows = self.rows.clone();
            if self.filter_requested_subject {
                rows.retain(|row| row.entity_type == entity_type && row.entity_id == entity_id);
            }
            Ok(rows)
        })
    }
}

fn fixture_entry(
    id: &str,
    entity_type: &str,
    entity_id: &str,
    created_at: &str,
    updated_at: &str,
) -> EntityContextEntry {
    EntityContextEntry {
        id: id.to_string(),
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        title: format!("title-{id}"),
        content: format!("content-{id}"),
        created_at: created_at.to_string(),
        updated_at: updated_at.to_string(),
    }
}

fn fixture_input(entity_type: &str, entity_id: &str) -> GetEntityContextInput {
    GetEntityContextInput {
        schema_version: 1,
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        depth: ContextDepth::Standard,
    }
}

async fn invoke_fixture(
    rows: Vec<EntityContextEntry>,
    input: GetEntityContextInput,
    filter_requested_subject: bool,
) -> AbilityResult<Vec<EntityContextEntry>> {
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(218);
    let provider = ReplayProvider::new(std::collections::HashMap::new());
    let reader = Arc::new(FixtureEntityContextReader {
        rows,
        filter_requested_subject,
    });
    let services = ServiceContext::new_evaluate_default(&clock, &rng)
        .with_actor("ability-test")
        .with_entity_context_reader(reader);
    let ctx = AbilityContext::new(
        &services,
        &provider,
        &NOOP_ABILITY_TRACER,
        Actor::User,
        None,
    );

    get_entity_context(&ctx, input).await
}

#[tokio::test]
async fn get_entity_context_input_rejects_unknown_entity_type() {
    let err = invoke_fixture(Vec::new(), fixture_input("workspace", "ws-1"), true)
        .await
        .expect_err("unknown entity type is rejected");

    assert_eq!(err.kind, AbilityErrorKind::Validation);
    assert!(err.message.contains("unsupported entity_type"));
}

#[test]
fn get_entity_context_subject_ref_account_project_person_meeting() {
    assert_eq!(
        subject_ref_for("account", "acct-1").unwrap(),
        SubjectRef::Account("acct-1".to_string())
    );
    assert_eq!(
        subject_ref_for("project", "proj-1").unwrap(),
        SubjectRef::Project("proj-1".to_string())
    );
    assert_eq!(
        subject_ref_for("person", "person-1").unwrap(),
        SubjectRef::Person("person-1".to_string())
    );
    assert_eq!(
        subject_ref_for("meeting", "meeting-1").unwrap(),
        SubjectRef::Meeting("meeting-1".to_string())
    );
}

#[tokio::test]
async fn get_entity_context_empty_returns_empty_vec_with_subject_provenance() {
    let output = invoke_fixture(Vec::new(), fixture_input("account", "acct-empty"), true)
        .await
        .expect("empty context succeeds");

    assert!(output.data().is_empty());
    assert_eq!(
        output.provenance().subject.subject,
        SubjectRef::Account("acct-empty".to_string())
    );
    assert!(output.provenance().sources.is_empty());
    assert!(output
        .provenance()
        .field_attributions
        .contains_key(&FieldPath::root()));
}

#[tokio::test]
async fn get_entity_context_orders_created_at_desc() {
    let older = fixture_entry(
        "older",
        "account",
        "acct-order",
        "2026-05-04T12:00:00Z",
        "2026-05-04T12:30:00Z",
    );
    let newer = fixture_entry(
        "newer",
        "account",
        "acct-order",
        "2026-05-05T12:00:00Z",
        "2026-05-05T12:30:00Z",
    );

    let output = invoke_fixture(
        vec![older, newer],
        fixture_input("account", "acct-order"),
        true,
    )
    .await
    .expect("context read succeeds");

    assert_eq!(output.data()[0].id, "newer");
    assert_eq!(output.data()[1].id, "older");
}

#[tokio::test]
async fn get_entity_context_field_attribution_covers_every_entry_leaf() {
    let output = invoke_fixture(
        vec![fixture_entry(
            "entry-1",
            "person",
            "person-1",
            "2026-05-04T12:00:00Z",
            "2026-05-04T12:30:00Z",
        )],
        fixture_input("person", "person-1"),
        true,
    )
    .await
    .expect("context read succeeds");

    let serialized = serde_json::to_value(output.data()).expect("data serializes");
    let leaf_paths = json_leaf_paths(&serialized).expect("leaf paths collect");
    for leaf_path in leaf_paths {
        let attribution = output
            .provenance()
            .field_attributions
            .get(&leaf_path)
            .unwrap_or_else(|| panic!("missing attribution for {}", leaf_path.as_str()));
        assert_eq!(attribution.derivation, DerivationKind::Direct);
        assert_eq!(
            attribution.subject.subject,
            SubjectRef::Person("person-1".to_string())
        );
        assert_eq!(attribution.subject.binding, SubjectBindingKind::DirectInput);
    }
}

#[tokio::test]
async fn get_entity_context_user_source_sets_source_asof_from_updated_at() {
    let output = invoke_fixture(
        vec![fixture_entry(
            "entry-updated",
            "project",
            "proj-1",
            "2026-05-04T12:00:00Z",
            "2026-05-05T15:45:00Z",
        )],
        fixture_input("project", "proj-1"),
        true,
    )
    .await
    .expect("context read succeeds");

    let source = &output.provenance().sources[0];
    assert_eq!(source.data_source, DataSource::User);
    assert_eq!(
        source.source_asof.expect("source_asof set").to_rfc3339(),
        "2026-05-05T15:45:00+00:00"
    );
    assert_eq!(source.observed_at.to_rfc3339(), "2026-05-05T15:45:00+00:00");
}

#[tokio::test]
async fn get_entity_context_unparseable_timestamp_warns_source_timestamp_unknown() {
    let output = invoke_fixture(
        vec![fixture_entry(
            "entry-bad-time",
            "meeting",
            "meeting-1",
            "not-a-created-time",
            "not-an-updated-time",
        )],
        fixture_input("meeting", "meeting-1"),
        true,
    )
    .await
    .expect("context read succeeds with warning");

    let source = &output.provenance().sources[0];
    assert!(source.source_asof.is_none());
    assert_eq!(source.observed_at.to_rfc3339(), "2026-05-06T12:00:00+00:00");
    assert!(output.provenance().warnings.iter().any(|warning| {
        matches!(
            warning,
            ProvenanceWarning::SourceTimestampUnknown {
                source_index,
                fallback: SourceTimestampFallback::ObservedAt
            } if source_index.as_usize() == 0
        )
    }));
}

#[tokio::test]
async fn get_entity_context_wrong_subject_fixture_blocks_or_marks() {
    let err = invoke_fixture(
        vec![fixture_entry(
            "wrong-subject",
            "account",
            "acct-other",
            "2026-05-04T12:00:00Z",
            "2026-05-04T12:30:00Z",
        )],
        fixture_input("account", "acct-target"),
        false,
    )
    .await
    .expect_err("wrong-subject row is blocked");

    assert_eq!(err.kind, AbilityErrorKind::Validation);
    assert!(err.message.contains("does not belong"));
}

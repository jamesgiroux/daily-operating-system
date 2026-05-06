use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::*;
use crate::abilities::feedback::{ClaimVerificationState, FeedbackAction};
use crate::abilities::provenance::{
    json_leaf_paths, DerivationKind, FieldPath, ProvenanceWarning, SourceTimestampFallback,
    SubjectBindingKind,
};
use crate::abilities::{AbilityRegistry, Actor, NOOP_ABILITY_TRACER};
use crate::db::claims::{ClaimSensitivity, ClaimState, SurfacingState, TemporalScope};
use crate::db::test_utils::test_db;
use crate::intelligence::provider::ReplayProvider;
use crate::services::claims::{
    commit_claim, load_entity_context_claims_active, record_claim_feedback, ClaimFeedbackInput,
    ClaimProposal, CommittedClaim,
};
use crate::services::context::{
    EntityContextClaimReadFuture, EntityContextClaimReadHandle, ExternalClients, FixedClock,
    SeedableRng, ServiceContext,
};

#[derive(Clone)]
struct FixtureEntityContextClaimReader {
    claims: Vec<IntelligenceClaim>,
    related: HashMap<(String, String), Vec<(String, String)>>,
}

impl EntityContextClaimReadHandle for FixtureEntityContextClaimReader {
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        depth: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        Box::pin(async move {
            let subjects = self.subjects_within_depth(entity_type, entity_id, depth.max(1));
            let mut claims = self
                .claims
                .iter()
                .filter(|claim| {
                    claim.claim_state == ClaimState::Active
                        && claim.surfacing_state == SurfacingState::Active
                        && claim_subject_identity(claim)
                            .map(|subject| subjects.contains(&subject))
                            .unwrap_or(false)
                })
                .cloned()
                .collect::<Vec<_>>();
            claims.sort_by(|left, right| right.created_at.cmp(&left.created_at));
            Ok(claims)
        })
    }
}

impl FixtureEntityContextClaimReader {
    fn subjects_within_depth(
        &self,
        entity_type: String,
        entity_id: String,
        depth: usize,
    ) -> HashSet<(String, String)> {
        let mut subjects = HashSet::new();
        let mut queue = VecDeque::from([((entity_type, entity_id), 1usize)]);
        while let Some((subject, level)) = queue.pop_front() {
            if !subjects.insert(subject.clone()) || level >= depth {
                continue;
            }

            if let Some(related) = self.related.get(&subject) {
                for next in related {
                    queue.push_back((next.clone(), level + 1));
                }
            }
        }
        subjects
    }
}

fn fixture_claim(
    id: &str,
    entity_type: &str,
    entity_id: &str,
    text: &str,
    created_at: &str,
    source_asof: Option<&str>,
) -> IntelligenceClaim {
    IntelligenceClaim {
        id: id.to_string(),
        subject_ref: serde_json::json!({
            "kind": entity_type,
            "id": entity_id,
        })
        .to_string(),
        claim_type: "entity_summary".to_string(),
        field_path: Some("summary".to_string()),
        topic_key: None,
        text: text.to_string(),
        dedup_key: format!("dedup-{id}"),
        item_hash: Some(format!("hash-{id}")),
        actor: "agent:test".to_string(),
        data_source: "user".to_string(),
        source_ref: Some(format!("source-{id}")),
        source_asof: source_asof.map(str::to_string),
        observed_at: created_at.to_string(),
        created_at: created_at.to_string(),
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
        sensitivity: ClaimSensitivity::Internal,
        verification_state: ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
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
    claims: Vec<IntelligenceClaim>,
    input: GetEntityContextInput,
    related: HashMap<(String, String), Vec<(String, String)>>,
) -> AbilityResult<Vec<EntityContextEntry>> {
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(218);
    let provider = ReplayProvider::new(std::collections::HashMap::new());
    let reader = Arc::new(FixtureEntityContextClaimReader { claims, related });
    let services = ServiceContext::new_evaluate_default(&clock, &rng)
        .with_actor("ability-test")
        .with_entity_context_claim_reader(reader);
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
    let err = invoke_fixture(
        Vec::new(),
        fixture_input("workspace", "ws-1"),
        HashMap::new(),
    )
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
    let output = invoke_fixture(
        Vec::new(),
        fixture_input("account", "acct-empty"),
        HashMap::new(),
    )
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
    let older = fixture_claim(
        "older",
        "account",
        "acct-order",
        "older content",
        "2026-05-04T12:00:00Z",
        Some("2026-05-04T12:30:00Z"),
    );
    let newer = fixture_claim(
        "newer",
        "account",
        "acct-order",
        "newer content",
        "2026-05-05T12:00:00Z",
        Some("2026-05-05T12:30:00Z"),
    );

    let output = invoke_fixture(
        vec![older, newer],
        fixture_input("account", "acct-order"),
        HashMap::new(),
    )
    .await
    .expect("context read succeeds");

    assert_eq!(output.data()[0].id, "newer");
    assert_eq!(output.data()[1].id, "older");
}

#[tokio::test]
async fn get_entity_context_field_attribution_covers_every_entry_leaf() {
    let output = invoke_fixture(
        vec![fixture_claim(
            "claim-1",
            "person",
            "person-1",
            "content-claim-1",
            "2026-05-04T12:00:00Z",
            Some("2026-05-04T12:30:00Z"),
        )],
        fixture_input("person", "person-1"),
        HashMap::new(),
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
async fn get_entity_context_user_source_preserves_claim_source_asof() {
    let output = invoke_fixture(
        vec![fixture_claim(
            "claim-asof",
            "project",
            "proj-1",
            "project content",
            "2026-05-04T12:00:00Z",
            Some("2026-05-05T15:45:00Z"),
        )],
        fixture_input("project", "proj-1"),
        HashMap::new(),
    )
    .await
    .expect("context read succeeds");

    let source = &output.provenance().sources[0];
    assert_eq!(source.data_source, DataSource::User);
    assert_eq!(
        source.source_asof.expect("source_asof set").to_rfc3339(),
        "2026-05-05T15:45:00+00:00"
    );
    assert_eq!(source.observed_at.to_rfc3339(), "2026-05-04T12:00:00+00:00");
}

#[tokio::test]
async fn get_entity_context_unparseable_timestamp_warns_source_timestamp_unknown() {
    let output = invoke_fixture(
        vec![fixture_claim(
            "claim-bad-time",
            "meeting",
            "meeting-1",
            "meeting content",
            "not-a-created-time",
            None,
        )],
        fixture_input("meeting", "meeting-1"),
        HashMap::new(),
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
async fn get_entity_context_depth_bounds_related_subject_fan_in() {
    let root = fixture_claim(
        "claim-root",
        "account",
        "acct-root",
        "root content",
        "2026-05-05T12:00:00Z",
        Some("2026-05-05T12:00:00Z"),
    );
    let child = fixture_claim(
        "claim-child",
        "account",
        "acct-child",
        "child content",
        "2026-05-05T11:00:00Z",
        Some("2026-05-05T11:00:00Z"),
    );
    let grandchild = fixture_claim(
        "claim-grandchild",
        "account",
        "acct-grandchild",
        "grandchild content",
        "2026-05-05T10:00:00Z",
        Some("2026-05-05T10:00:00Z"),
    );
    let related = HashMap::from([
        (
            ("account".to_string(), "acct-root".to_string()),
            vec![("account".to_string(), "acct-child".to_string())],
        ),
        (
            ("account".to_string(), "acct-child".to_string()),
            vec![("account".to_string(), "acct-grandchild".to_string())],
        ),
    ]);

    let mut shallow_input = fixture_input("account", "acct-root");
    shallow_input.depth = ContextDepth::Shallow;
    let shallow = invoke_fixture(
        vec![root.clone(), child.clone(), grandchild.clone()],
        shallow_input,
        related.clone(),
    )
    .await
    .expect("shallow read succeeds");
    assert_eq!(claim_ids(shallow.data()), vec!["claim-root"]);

    let standard = invoke_fixture(
        vec![root.clone(), child.clone(), grandchild.clone()],
        fixture_input("account", "acct-root"),
        related.clone(),
    )
    .await
    .expect("standard read succeeds");
    assert_eq!(
        claim_ids(standard.data()),
        vec!["claim-root", "claim-child"]
    );

    let mut deep_input = fixture_input("account", "acct-root");
    deep_input.depth = ContextDepth::Deep;
    let deep = invoke_fixture(vec![root, child, grandchild], deep_input, related)
        .await
        .expect("deep read succeeds");
    assert_eq!(
        claim_ids(deep.data()),
        vec!["claim-root", "claim-child", "claim-grandchild"]
    );
}

#[test]
fn get_entity_context_claim_loader_returns_only_active_surfacing_claims() {
    let db = test_db();
    seed_account(&db, "acct-filter", None);
    let (clock, rng, external) = ctx_parts();
    let ctx = ServiceContext::test_live(&clock, &rng, &external);
    let subject_ref = subject_ref_json("account", "acct-filter");

    let visible_id = seed_claim(&ctx, &db, &subject_ref, "visible", "risk.visible");
    let dormant_id = seed_claim(&ctx, &db, &subject_ref, "dormant", "risk.dormant");
    let withdrawn_id = seed_claim(&ctx, &db, &subject_ref, "withdrawn", "risk.withdrawn");

    record_claim_feedback(
        &ctx,
        &db,
        feedback_input(&dormant_id, FeedbackAction::MarkOutdated),
    )
    .expect("mark outdated");
    record_claim_feedback(
        &ctx,
        &db,
        feedback_input(&withdrawn_id, FeedbackAction::MarkFalse),
    )
    .expect("mark false");

    let ids = load_entity_context_claims_active(&db, "account", "acct-filter", 1)
        .expect("load active surfaced context claims")
        .into_iter()
        .map(|claim| claim.id)
        .collect::<Vec<_>>();

    assert_eq!(ids, vec![visible_id]);
}

#[test]
fn get_entity_context_claim_loader_enforces_depth_bound() {
    let db = test_db();
    seed_account(&db, "acct-root", None);
    seed_account(&db, "acct-child", Some("acct-root"));
    seed_account(&db, "acct-grandchild", Some("acct-child"));
    let (clock, rng, external) = ctx_parts();
    let ctx = ServiceContext::test_live(&clock, &rng, &external);

    let root_id = seed_claim(
        &ctx,
        &db,
        &subject_ref_json("account", "acct-root"),
        "root",
        "risk.root",
    );
    let child_id = seed_claim(
        &ctx,
        &db,
        &subject_ref_json("account", "acct-child"),
        "child",
        "risk.child",
    );
    let grandchild_id = seed_claim(
        &ctx,
        &db,
        &subject_ref_json("account", "acct-grandchild"),
        "grandchild",
        "risk.grandchild",
    );

    let shallow = loader_ids(&db, "acct-root", 1);
    assert_eq!(shallow, vec![root_id.clone()]);

    let standard = loader_ids(&db, "acct-root", 2);
    assert!(standard.contains(&root_id));
    assert!(standard.contains(&child_id));
    assert!(!standard.contains(&grandchild_id));

    let deep = loader_ids(&db, "acct-root", 3);
    assert!(deep.contains(&root_id));
    assert!(deep.contains(&child_id));
    assert!(deep.contains(&grandchild_id));
}

#[test]
fn get_entity_context_is_registered_for_agent_actor() {
    let registry = AbilityRegistry::from_inventory_checked().expect("registry builds");
    let names = registry
        .iter_for(Actor::Agent)
        .map(|descriptor| descriptor.name)
        .collect::<HashSet<_>>();

    assert!(names.contains("get_entity_context"));
}

fn claim_ids(entries: &[EntityContextEntry]) -> Vec<&str> {
    entries.iter().map(|entry| entry.id.as_str()).collect()
}

fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
    (
        FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap()),
        SeedableRng::new(218),
        ExternalClients::default(),
    )
}

fn subject_ref_json(entity_type: &str, entity_id: &str) -> String {
    serde_json::json!({
        "kind": entity_type,
        "id": entity_id,
    })
    .to_string()
}

fn proposal(subject_ref: &str, text: &str, field_path: &str) -> ClaimProposal {
    ClaimProposal {
        subject_ref: subject_ref.to_string(),
        claim_type: "risk".to_string(),
        field_path: Some(field_path.to_string()),
        topic_key: None,
        text: text.to_string(),
        actor: "agent:test".to_string(),
        data_source: "user".to_string(),
        source_ref: None,
        source_asof: Some("2026-05-05T12:00:00Z".to_string()),
        observed_at: "2026-05-05T12:00:00Z".to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: None,
        thread_id: None,
        temporal_scope: Some(TemporalScope::State),
        sensitivity: Some(ClaimSensitivity::Internal),
        tombstone: None,
    }
}

fn seed_claim(
    ctx: &ServiceContext<'_>,
    db: &crate::db::ActionDb,
    subject_ref: &str,
    text: &str,
    field_path: &str,
) -> String {
    match commit_claim(ctx, db, proposal(subject_ref, text, field_path)).expect("commit claim") {
        CommittedClaim::Inserted { claim }
        | CommittedClaim::Reinforced { claim, .. }
        | CommittedClaim::Tombstoned { claim } => claim.id,
        CommittedClaim::Forked { new_claim_id, .. } => new_claim_id,
    }
}

fn feedback_input(claim_id: &str, action: FeedbackAction) -> ClaimFeedbackInput {
    ClaimFeedbackInput {
        claim_id: claim_id.to_string(),
        action,
        actor: "user:test".to_string(),
        actor_id: Some("user-test".to_string()),
        payload_json: None,
    }
}

fn seed_account(db: &crate::db::ActionDb, id: &str, parent_id: Option<&str>) {
    db.conn_ref()
        .execute(
            "INSERT INTO accounts (id, name, parent_id, updated_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                id,
                format!("Account {id}"),
                parent_id,
                "2026-05-05T12:00:00Z"
            ],
        )
        .expect("seed account");
}

fn loader_ids(db: &crate::db::ActionDb, account_id: &str, depth: usize) -> Vec<String> {
    load_entity_context_claims_active(db, "account", account_id, depth)
        .expect("load context claims")
        .into_iter()
        .map(|claim| claim.id)
        .collect()
}

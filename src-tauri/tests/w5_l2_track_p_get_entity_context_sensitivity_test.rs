use std::collections::HashSet;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::feedback::ClaimVerificationState;
use dailyos_lib::abilities::{AbilityContext, AbilityRegistry, Actor, NOOP_ABILITY_TRACER};
use dailyos_lib::db::claims::{
    ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState, TemporalScope,
};
use dailyos_lib::intelligence::provider::ReplayProvider;
use dailyos_lib::services::context::{
    EntityContextClaimReadFuture, EntityContextClaimReadHandle, FixedClock, SeedableRng,
    ServiceContext,
};
use dailyos_lib::services::sensitivity::RenderPolicyKind;
use dailyos_lib::types::{EntityContextEntry, EntityContextText};
use serde_json::json;

const ENTITY_TYPE: &str = "account";
const ENTITY_ID: &str = "acct-track-p";
const INTERNAL_TEXT: &str = "internal context visible to agent";
const CONFIDENTIAL_TEXT: &str = "confidential context must not reach agent";
const USER_ONLY_TEXT: &str = "user only context must not reach agent";

struct FixtureClaimReader {
    claims: Vec<IntelligenceClaim>,
}

impl EntityContextClaimReadHandle for FixtureClaimReader {
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        _depth: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        let mut claims = self
            .claims
            .iter()
            .filter(|claim| claim_matches_subject(claim, &entity_type, &entity_id))
            .cloned()
            .collect::<Vec<_>>();
        claims.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Box::pin(std::future::ready(Ok(claims)))
    }
}

#[tokio::test]
async fn agent_erased_get_entity_context_filters_confidential_and_user_only_claims() {
    let claims = seeded_claims();

    let agent_entries = invoke_get_entity_context(Actor::Agent, claims.clone()).await;
    let agent_contents = entry_contents(&agent_entries);
    assert!(agent_contents.contains(INTERNAL_TEXT));
    assert!(!agent_contents.contains(CONFIDENTIAL_TEXT));
    assert!(!agent_contents.contains(USER_ONLY_TEXT));

    let user_entries = invoke_get_entity_context(Actor::User, claims.clone()).await;
    assert!(entry_contents(&user_entries).contains(INTERNAL_TEXT));
    assert!(!entry_contents(&user_entries).contains(CONFIDENTIAL_TEXT));
    assert_confidential_affordance(&user_entries);
    assert_user_only_hidden(&user_entries);

    let system_entries = invoke_get_entity_context(Actor::System, claims).await;
    assert!(entry_contents(&system_entries).contains(INTERNAL_TEXT));
    assert!(!entry_contents(&system_entries).contains(CONFIDENTIAL_TEXT));
    assert_confidential_affordance(&system_entries);
    assert_user_only_hidden(&system_entries);
}

async fn invoke_get_entity_context(
    actor: Actor,
    claims: Vec<IntelligenceClaim>,
) -> Vec<EntityContextEntry> {
    let registry = AbilityRegistry::from_inventory_checked().expect("registry builds");
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(218);
    let services = ServiceContext::new_evaluate_default(&clock, &rng)
        .with_actor("ability-test")
        .with_entity_context_claim_reader(Arc::new(FixtureClaimReader { claims }));
    let provider = ReplayProvider::new(std::collections::HashMap::new());
    let ctx = AbilityContext::new(&services, &provider, &NOOP_ABILITY_TRACER, actor, None);

    let value = registry
        .invoke_by_name_json(
            &ctx,
            "get_entity_context",
            json!({
                "schema_version": 2,
                "entity_type": ENTITY_TYPE,
                "entity_id": ENTITY_ID,
                "depth": "standard",
            }),
        )
        .await
        .expect("erased get_entity_context succeeds");

    serde_json::from_value(value["data"]["entries"].clone())
        .expect("entity context entries deserialize")
}

fn seeded_claims() -> Vec<IntelligenceClaim> {
    vec![
        fixture_claim(
            "claim-internal",
            INTERNAL_TEXT,
            ClaimSensitivity::Internal,
            "2026-05-06T12:00:00Z",
        ),
        fixture_claim(
            "claim-confidential",
            CONFIDENTIAL_TEXT,
            ClaimSensitivity::Confidential,
            "2026-05-06T11:00:00Z",
        ),
        fixture_claim(
            "claim-user-only",
            USER_ONLY_TEXT,
            ClaimSensitivity::UserOnly,
            "2026-05-06T10:00:00Z",
        ),
    ]
}

fn fixture_claim(
    id: &str,
    text: &str,
    sensitivity: ClaimSensitivity,
    created_at: &str,
) -> IntelligenceClaim {
    IntelligenceClaim {
        id: id.to_string(),
        subject_ref: json!({
            "kind": ENTITY_TYPE,
            "id": ENTITY_ID,
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
        source_asof: Some(created_at.to_string()),
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
        sensitivity,
        verification_state: ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    }
}

fn claim_matches_subject(claim: &IntelligenceClaim, entity_type: &str, entity_id: &str) -> bool {
    let subject_ref: serde_json::Value =
        serde_json::from_str(&claim.subject_ref).expect("fixture subject_ref parses");
    subject_ref["kind"] == entity_type && subject_ref["id"] == entity_id
}

fn assert_confidential_affordance(entries: &[EntityContextEntry]) {
    let entry = entries
        .iter()
        .find(|entry| entry.id == "claim-confidential")
        .expect("confidential entry present for user/system actor");
    let EntityContextText::Claim(content) = &entry.content else {
        panic!("confidential content should be a renderable claim carrier");
    };
    assert_eq!(content.text, "Confidential claim hidden");
    assert_eq!(content.policy.kind, RenderPolicyKind::Redacted);
    assert_eq!(content.policy.sensitivity, ClaimSensitivity::Confidential);
    assert!(content.policy.affordance.is_some());
}

fn assert_user_only_hidden(entries: &[EntityContextEntry]) {
    let entry = entries
        .iter()
        .find(|entry| entry.id == "claim-user-only")
        .expect("user-only entry present for user/system actor");
    assert_eq!(entry.content.as_str(), "User-only claim hidden");
    assert!(!entry.content.as_str().contains(USER_ONLY_TEXT));
}

fn entry_contents(entries: &[EntityContextEntry]) -> HashSet<&str> {
    entries.iter().map(|entry| entry.content.as_str()).collect()
}

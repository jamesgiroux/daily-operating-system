use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::feedback::ClaimVerificationState;
use dailyos_lib::abilities::{AbilityContext, AbilityError, AbilityErrorKind, AbilityRegistry};
use dailyos_lib::abilities::{Actor, NOOP_ABILITY_TRACER};
use dailyos_lib::db::claims::{
    ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState, TemporalScope,
};
use dailyos_lib::intelligence::provider::ReplayProvider;
use dailyos_lib::services::context::{
    ClaimDismissalSurface, FixedClock, ListOpenLoopsQuery, ListOpenLoopsReadError,
    ListOpenLoopsReadFuture, ListOpenLoopsReadHandle, ListOpenLoopsSnapshot, SeedableRng,
    ServiceContext,
};
use serde_json::{json, Value};

const ACCOUNT_ID: &str = "acct-w5b-subsidiary";
const ADJACENT_ACCOUNT_ID: &str = "acct-w5b-parent";
const PERSON_ID: &str = "person-w5b-casey";
const EMPTY_ACCOUNT_ID: &str = "acct-w5b-empty";
const CROSS_TENANT_ACCOUNT_ID: &str = "acct-w5b-other-workspace";
const REVOKED_ACCOUNT_ID: &str = "acct-w5b-revoked-source";

type SubjectKey = (String, String);

struct FixtureClaimReader {
    claims: Vec<IntelligenceClaim>,
    owned_subjects: HashSet<SubjectKey>,
}

impl ListOpenLoopsReadHandle for FixtureClaimReader {
    fn read_open_loops<'a>(&'a self, query: ListOpenLoopsQuery) -> ListOpenLoopsReadFuture<'a> {
        let result = match (query.entity_type.as_deref(), query.entity_id.as_deref()) {
            (Some(entity_type), Some(entity_id)) => {
                let requested = (entity_type.to_string(), entity_id.to_string());
                if !self.owned_subjects.contains(&requested) {
                    Err(ListOpenLoopsReadError::SubjectNotOwned {
                        entity_type: entity_type.to_string(),
                        entity_id: entity_id.to_string(),
                    })
                } else {
                    Ok(ListOpenLoopsSnapshot {
                        claims: self
                            .claims
                            .iter()
                            .filter(|claim| is_active_and_surfaced(claim))
                            .filter(|claim| claim_matches_subject(claim, entity_type, entity_id))
                            .cloned()
                            .collect(),
                    })
                }
            }
            (None, None) => Ok(ListOpenLoopsSnapshot {
                claims: self
                    .claims
                    .iter()
                    .filter(|claim| is_active_and_surfaced(claim))
                    .filter(|claim| self.owned_subjects.contains(&claim_subject_key(claim)))
                    .cloned()
                    .collect(),
            }),
            _ => Err(ListOpenLoopsReadError::ReadFailed(
                "entity_type and entity_id must be supplied together".to_string(),
            )),
        };

        Box::pin(std::future::ready(result))
    }
}

#[tokio::test]
async fn w5_b_list_open_loops_filters_account_subject_fit() {
    let value = invoke_list_open_loops(
        Some(("account", ACCOUNT_ID)),
        fixture_claims(),
        owned_subjects(),
    )
    .await
    .expect("account list_open_loops succeeds");

    assert_eq!(
        loop_ids(&value),
        BTreeSet::from([
            "loop-account-owner".to_string(),
            "loop-account-pricing".to_string(),
            "loop-account-security".to_string()
        ])
    );
    assert_eq!(value["data"]["schema_version"], 1);
    assert_eq!(
        loop_by_id(&value, "loop-account-owner")["subject"],
        json!({
            "entity_type": "account",
            "entity_id": ACCOUNT_ID,
        })
    );
    assert_eq!(
        loop_by_id(&value, "loop-account-owner")["owner"],
        "Casey Chen"
    );
    assert!(!loop_ids(&value).contains("loop-adjacent-parent"));
    assert!(!loop_ids(&value).contains("loop-person-contract"));
}

#[tokio::test]
async fn w5_b_list_open_loops_returns_person_loop() {
    let value = invoke_list_open_loops(
        Some(("person", PERSON_ID)),
        fixture_claims(),
        owned_subjects(),
    )
    .await
    .expect("person list_open_loops succeeds");

    assert_eq!(
        loop_ids(&value),
        BTreeSet::from(["loop-person-contract".to_string()])
    );
    let person_loop = loop_by_id(&value, "loop-person-contract");
    assert_eq!(
        person_loop["subject"],
        json!({
            "entity_type": "person",
            "entity_id": PERSON_ID,
        })
    );
    assert_eq!(person_loop["loop_kind"], "follow_up");
    assert_eq!(person_loop["claim_type"], "open_loop");
}

#[tokio::test]
async fn w5_b_list_open_loops_returns_empty_for_owned_entity_without_loops() {
    let value = invoke_list_open_loops(
        Some(("account", EMPTY_ACCOUNT_ID)),
        fixture_claims(),
        owned_subjects(),
    )
    .await
    .expect("empty entity list_open_loops succeeds");

    assert!(loops(&value).is_empty());
    assert!(
        value["provenance"]["field_attributions"]
            .get("/loops")
            .is_some(),
        "empty result should explicitly attribute /loops"
    );
}

#[tokio::test]
async fn w5_b_list_open_loops_rejects_cross_tenant_subject() {
    let err = invoke_list_open_loops(
        Some(("account", CROSS_TENANT_ACCOUNT_ID)),
        fixture_claims(),
        owned_subjects(),
    )
    .await
    .expect_err("cross-tenant subject must fail closed");

    assert_eq!(err.kind, AbilityErrorKind::SubjectNotOwned);
    assert!(err.message.contains(CROSS_TENANT_ACCOUNT_ID));
    assert!(
        fixture_claims()
            .iter()
            .any(|claim| claim.id == "loop-cross-tenant-active"),
        "fixture seeds an active other-workspace loop so this is not a no-row case"
    );
}

#[tokio::test]
async fn w5_b_list_open_loops_omits_revoked_glean_source_and_counts_warning() {
    let value = invoke_list_open_loops(
        Some(("account", REVOKED_ACCOUNT_ID)),
        fixture_claims(),
        owned_subjects(),
    )
    .await
    .expect("revoked-source entity list_open_loops succeeds");

    assert!(loops(&value).is_empty());
    assert_eq!(source_revoked_warning_count(&value), 1);
}

async fn invoke_list_open_loops(
    entity_filter: Option<(&str, &str)>,
    claims: Vec<IntelligenceClaim>,
    owned_subjects: HashSet<SubjectKey>,
) -> Result<Value, AbilityError> {
    let registry = AbilityRegistry::from_inventory_checked().expect("registry builds");
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(221);
    let services = ServiceContext::new_evaluate_default(&clock, &rng)
        .with_actor("ability-test")
        .with_list_open_loops_reader(Arc::new(FixtureClaimReader {
            claims,
            owned_subjects,
        }));
    let provider = ReplayProvider::new(std::collections::HashMap::new());
    let ctx = AbilityContext::new(
        &services,
        &provider,
        &NOOP_ABILITY_TRACER,
        Actor::User,
        None,
        ClaimDismissalSurface::Eval,
    );

    let mut input = json!({ "schema_version": 1 });
    if let Some((entity_type, entity_id)) = entity_filter {
        input["entity_type"] = json!(entity_type);
        input["entity_id"] = json!(entity_id);
    }

    registry
        .invoke_by_name_json(&ctx, "list_open_loops", input)
        .await
}

fn fixture_claims() -> Vec<IntelligenceClaim> {
    vec![
        with_metadata(
            claim(
                "loop-account-owner",
                "account",
                ACCOUNT_ID,
                "open_loop",
                "Subsidiary Example needs a named rollout owner before launch.",
                "2026-05-05T09:30:00Z",
            ),
            json!({
                "loop_kind": "follow_up",
                "owner": "Casey Chen",
                "due_date": "2026-05-10",
                "status": "active",
            }),
        ),
        with_metadata(
            claim(
                "loop-account-security",
                "account",
                ACCOUNT_ID,
                "commitment",
                "Security review notes for Subsidiary Example need final confirmation.",
                "2026-05-05T10:15:00Z",
            ),
            json!({
                "loop_kind": "commitment",
                "owner": "Devon Diaz",
                "status": "waiting",
            }),
        ),
        with_metadata(
            claim(
                "loop-account-pricing",
                "account",
                ACCOUNT_ID,
                "open_loop",
                "Pricing approval remains open for the Subsidiary Example expansion.",
                "2026-05-04T16:45:00Z",
            ),
            json!({
                "loop_kind": "decision",
                "owner": "Avery Adams",
                "status": "open",
            }),
        ),
        with_metadata(
            claim(
                "loop-adjacent-parent",
                "account",
                ADJACENT_ACCOUNT_ID,
                "open_loop",
                "Parent Example has a separate legal review open.",
                "2026-05-05T11:00:00Z",
            ),
            json!({
                "loop_kind": "follow_up",
                "owner": "Morgan Malik",
                "status": "active",
            }),
        ),
        with_metadata(
            claim(
                "loop-person-contract",
                "person",
                PERSON_ID,
                "open_loop",
                "Casey Chen needs the contract checklist before the next account review.",
                "2026-05-05T08:00:00Z",
            ),
            json!({
                "loop_kind": "follow_up",
                "owner": "Casey Chen",
                "status": "active",
            }),
        ),
        with_metadata(
            claim(
                "loop-cross-tenant-active",
                "account",
                CROSS_TENANT_ACCOUNT_ID,
                "open_loop",
                "Other Workspace Subsidiary Example has an active rollout follow-up.",
                "2026-05-05T12:15:00Z",
            ),
            json!({
                "loop_kind": "follow_up",
                "owner": "Riley Rivera",
                "status": "active",
            }),
        ),
        with_source(
            with_metadata(
                claim(
                    "loop-revoked-glean",
                    "account",
                    REVOKED_ACCOUNT_ID,
                    "open_loop",
                    "Revoked Source Example has a Glean-backed action that should be omitted.",
                    "2026-05-05T13:00:00Z",
                ),
                json!({
                    "loop_kind": "follow_up",
                    "owner": "Jamie Jordan",
                    "status": "active",
                    "primary_source": {
                        "id": "doc-w5b-revoked",
                        "source_lifecycle_state": "revoked"
                    }
                }),
            ),
            "glean",
            Some("doc-w5b-revoked"),
        ),
    ]
}

fn claim(
    id: &str,
    entity_type: &str,
    entity_id: &str,
    claim_type: &str,
    text: &str,
    source_asof: &str,
) -> IntelligenceClaim {
    IntelligenceClaim {
        id: id.to_string(),
        claim_version: 1,
        subject_ref: json!({
            "kind": entity_type,
            "id": entity_id,
        })
        .to_string(),
        claim_type: claim_type.to_string(),
        field_path: Some("summary".to_string()),
        topic_key: None,
        text: text.to_string(),
        dedup_key: format!("dedup-{id}"),
        item_hash: Some(format!("hash-{id}")),
        actor: "agent:fixture".to_string(),
        data_source: "user".to_string(),
        source_ref: Some(format!("source-{id}")),
        source_asof: Some(source_asof.to_string()),
        observed_at: source_asof.to_string(),
        created_at: source_asof.to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: None,
        claim_state: ClaimState::Active,
        surfacing_state: SurfacingState::Active,
        demotion_reason: None,
        reactivated_at: None,
        retraction_reason: None,
        expires_at: None,
        superseded_by: None,
        trust_score: Some(0.9),
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

fn with_metadata(mut claim: IntelligenceClaim, metadata: Value) -> IntelligenceClaim {
    claim.metadata_json = Some(metadata.to_string());
    claim
}

fn with_source(
    mut claim: IntelligenceClaim,
    data_source: &str,
    source_ref: Option<&str>,
) -> IntelligenceClaim {
    claim.data_source = data_source.to_string();
    claim.source_ref = source_ref.map(ToString::to_string);
    claim
}

fn owned_subjects() -> HashSet<SubjectKey> {
    HashSet::from([
        ("account".to_string(), ACCOUNT_ID.to_string()),
        ("account".to_string(), ADJACENT_ACCOUNT_ID.to_string()),
        ("person".to_string(), PERSON_ID.to_string()),
        ("account".to_string(), EMPTY_ACCOUNT_ID.to_string()),
        ("account".to_string(), REVOKED_ACCOUNT_ID.to_string()),
    ])
}

fn is_active_and_surfaced(claim: &IntelligenceClaim) -> bool {
    claim.claim_state == ClaimState::Active && claim.surfacing_state == SurfacingState::Active
}

fn claim_matches_subject(claim: &IntelligenceClaim, entity_type: &str, entity_id: &str) -> bool {
    claim_subject_key(claim) == (entity_type.to_string(), entity_id.to_string())
}

fn claim_subject_key(claim: &IntelligenceClaim) -> SubjectKey {
    let subject_ref: Value =
        serde_json::from_str(&claim.subject_ref).expect("fixture subject_ref parses");
    (
        subject_ref["kind"]
            .as_str()
            .expect("fixture subject kind is present")
            .to_string(),
        subject_ref["id"]
            .as_str()
            .expect("fixture subject id is present")
            .to_string(),
    )
}

fn loops(value: &Value) -> &[Value] {
    value["data"]["loops"]
        .as_array()
        .expect("list_open_loops output has loops array")
}

fn loop_ids(value: &Value) -> BTreeSet<String> {
    loops(value)
        .iter()
        .map(|entry| {
            entry["id"]
                .as_str()
                .expect("loop id is a string")
                .to_string()
        })
        .collect()
}

fn loop_by_id<'a>(value: &'a Value, id: &str) -> &'a Value {
    loops(value)
        .iter()
        .find(|entry| entry["id"] == id)
        .expect("loop id exists in output")
}

fn source_revoked_warning_count(value: &Value) -> usize {
    value["provenance"]["warnings"]
        .as_array()
        .expect("provenance warnings array exists")
        .iter()
        .filter(|warning| {
            warning.as_str() == Some("source_revoked") || warning["kind"] == "source_revoked"
        })
        .count()
}

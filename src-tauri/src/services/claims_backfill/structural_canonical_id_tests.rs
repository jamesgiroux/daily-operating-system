use super::*;

use abilities_runtime::structured_claim::StructuredClaim;
use chrono::TimeZone;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::db::ActionDb;
use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};

fn fixture_ctx<'a>(
    clock: &'a FixedClock,
    rng: &'a SeedableRng,
    ext: &'a ExternalClients,
) -> ServiceContext<'a> {
    ServiceContext::test_live(clock, rng, ext)
}

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    crate::migrations::run_migrations(&conn).unwrap();
    conn
}

fn expected_structural_id(values: [&str; 4]) -> String {
    let mut hasher = Sha256::new();
    for value in values {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn seed_pending_claim(
    conn: &Connection,
    id: &str,
    subject_ref: &str,
    claim_type: &str,
    field_path: Option<&str>,
    text: &str,
    metadata_json: Option<&str>,
) {
    let dedup_key = format!("dedup-{id}");
    let item_hash = format!("hash-{id}");
    conn.execute(
        "INSERT INTO intelligence_claims /* dos7-allowed: ADR-0131 structural backfill fixture seeds pending rows */ (
            id, subject_ref, claim_type, field_path, topic_key, text, dedup_key, item_hash,
            actor, data_source, observed_at, created_at, provenance_json, metadata_json,
            claim_state, surfacing_state, retraction_reason, temporal_scope, sensitivity,
            verification_state, canonical_status, non_semantic_mergeable
         ) VALUES (
            ?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7,
            'system_backfill', 'legacy_dismissal', '2026-05-02T00:00:00Z',
            '2026-05-02T00:00:00Z', '{}', ?8,
            'tombstoned', 'active', 'user_removal', 'state', 'internal', 'active',
            'pending_backfill', TRUE
         )",
        rusqlite::params![
            id,
            subject_ref,
            claim_type,
            field_path,
            text,
            dedup_key,
            item_hash,
            metadata_json,
        ],
    )
    .unwrap();
}

#[test]
fn structured_backfill_writes_deterministic_structural_canonical_id_for_live_commitment_only() {
    let conn = fresh_full_db();
    let db = ActionDb::from_conn(&conn);
    let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
    let rng = SeedableRng::new(42);
    let ext = ExternalClients::default();
    let ctx = fixture_ctx(&clock, &rng, &ext);

    seed_pending_claim(
        &conn,
        "commitment-live-structural-id",
        r#"{"kind":"Person","id":"person-owner-1"}"#,
        "commitment",
        Some("due_date"),
        "Owner will send onboarding checklist by Q3 2026",
        None,
    );
    seed_pending_claim(
        &conn,
        "commitment-legacy-no-structural-id",
        r#"{"kind":"Person","id":"person-owner-2"}"#,
        "commitment",
        Some("due_date"),
        "Legacy non-semantic commitment by Q3 2026",
        Some(r#"{"non_semantic_mergeable":true}"#),
    );

    let report = run_structured_claim_backfill(&ctx, &db).unwrap();
    assert_eq!(report.rows_examined, 2);
    assert_eq!(report.transitioned_live, 1);
    assert_eq!(report.transitioned_legacy_unmigrated, 1);
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    let (
        canonical_status,
        non_semantic_mergeable,
        structured_claim_json,
        predicate_ref,
        polarity,
        object_value,
        qualifiers,
        structural_canonical_id,
        content_hash,
    ): (
        String,
        bool,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT canonical_status, non_semantic_mergeable, structured_claim_json,
                    predicate_ref, polarity, object_value, qualifiers,
                    structural_canonical_id, structural_field_content_hash
             FROM intelligence_claims WHERE id = 'commitment-live-structural-id'",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            },
        )
        .unwrap();

    assert_eq!(canonical_status, "live");
    assert!(!non_semantic_mergeable);
    assert_eq!(predicate_ref, "commitment.due");
    assert_eq!(polarity, "affirm");
    assert!(content_hash.is_some());

    let structured: StructuredClaim = serde_json::from_str(&structured_claim_json).unwrap();
    assert_eq!(structured.subject_ref.kind, "Person");
    assert_eq!(structured.subject_ref.id, "person-owner-1");
    assert_eq!(
        structured
            .qualifiers
            .time
            .as_ref()
            .map(|time| time.normalized.as_str()),
        Some("q3")
    );
    assert_eq!(
        object_value,
        serde_json::to_string(&structured.object).unwrap()
    );
    assert_eq!(
        qualifiers,
        serde_json::to_string(&structured.qualifiers).unwrap()
    );

    let expected_id = expected_structural_id([
        predicate_ref.as_str(),
        polarity.as_str(),
        object_value.as_str(),
        qualifiers.as_str(),
    ]);
    assert_eq!(structural_canonical_id, expected_id);

    let (legacy_status, legacy_structural_canonical_id, legacy_predicate_ref): (
        String,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT canonical_status, structural_canonical_id, predicate_ref
             FROM intelligence_claims WHERE id = 'commitment-legacy-no-structural-id'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();

    assert_eq!(legacy_status, "legacy_unmigrated");
    assert!(legacy_structural_canonical_id.is_none());
    assert!(legacy_predicate_ref.is_none());
}

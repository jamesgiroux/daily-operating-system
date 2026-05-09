use chrono::{DateTime, TimeZone, Utc};
use dailyos_lib::abilities::provenance::{
    DataSource, MeetingId, SourceIdentifier, SourceIndex, SourceRef,
};
use dailyos_lib::abilities::temporal::{
    DetectRoleChangeInput, RefreshEngagementCurveInput, TrajectoryQueryDepth,
};
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use dailyos_lib::services::signals;
use dailyos_lib::services::temporal::{
    detect_role_change_in_db, mark_source_invalidated_in_db, read_trajectory_bundle_from_db,
    refresh_engagement_curve_in_db,
};
use dailyos_lib::signals::propagation::PropagationEngine;
use rusqlite::{params, Connection};
use std::sync::{Arc, Barrier};
use std::thread;

const ENTITY_ID: &str = "entity-dos215";
const ENTITY_TYPE: &str = "project";

#[test]
fn migration_148_creates_phase1_temporal_tables_and_indexes() {
    let conn = Connection::open_in_memory().expect("open DB");
    run_migrations(&conn).expect("apply migrations");

    assert_table_has_columns(
        &conn,
        "entity_engagement_curve",
        &[
            "entity_type",
            "entity_id",
            "week_start",
            "meetings_count",
            "emails_count",
            "bidirectional_ratio",
            "source_refs_json",
            "source_invalidated_at",
        ],
    );
    assert_table_has_columns(
        &conn,
        "person_role_progression",
        &[
            "entity_type",
            "entity_id",
            "started_at",
            "ended_at",
            "title",
            "org",
            "seniority",
            "source_refs_json",
            "source_invalidated_at",
        ],
    );
    assert_table_has_columns(
        &conn,
        "temporal_backfill_state",
        &[
            "entity_type",
            "entity_id",
            "ability_id",
            "last_completed_week_start",
            "retention_cutoff",
            "updated_at",
        ],
    );
    assert_index_exists(&conn, "idx_entity_engagement_curve_entity_week");
    assert_index_exists(&conn, "idx_person_role_progression_entity");
}

#[test]
fn engagement_refresh_is_idempotent_and_retention_bounded() {
    let conn = seeded_temporal_conn();
    let db = ActionDb::from_conn(&conn);
    let computed_at = Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap();

    seed_engagement_inputs(&conn);
    seed_stale_engagement_row(&conn);

    let input = RefreshEngagementCurveInput {
        schema_version: 2,
        entity_type: ENTITY_TYPE.to_string(),
        entity_id: ENTITY_ID.to_string(),
    };
    let first = refresh_engagement_curve_in_db(db, input.clone(), computed_at)
        .expect("first engagement refresh");
    let second =
        refresh_engagement_curve_in_db(db, input, computed_at).expect("second engagement refresh");

    assert_eq!(first.week_start, second.week_start);
    let row_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entity_engagement_curve
             WHERE entity_type = ?1 AND entity_id = ?2",
            params![ENTITY_TYPE, ENTITY_ID],
            |row| row.get(0),
        )
        .expect("count engagement rows");
    assert_eq!(
        row_count, 2,
        "refresh should upsert the latest row and backfill one non-empty historical week"
    );

    let (meetings, emails, ratio, refs_json): (i64, i64, f64, String) = conn
        .query_row(
            "SELECT meetings_count, emails_count, bidirectional_ratio, source_refs_json
             FROM entity_engagement_curve
             WHERE entity_type = ?1 AND entity_id = ?2 AND week_start = ?3",
            params![ENTITY_TYPE, ENTITY_ID, first.week_start.to_rfc3339()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read engagement row");
    assert_eq!(meetings, 1);
    assert_eq!(emails, 2);
    assert_eq!(ratio, 1.0);
    let refs: Vec<SourceRef> = serde_json::from_str(&refs_json).expect("source refs parse");
    assert_eq!(refs.len(), 3);
    assert!(matches!(
        &refs[0],
        SourceRef::Direct {
            identifier: SourceIdentifier::Meeting { meeting_id },
            ..
        } if meeting_id.0 == "meeting-dos215"
    ));
    assert!(matches!(
        &refs[1],
        SourceRef::Direct {
            identifier: SourceIdentifier::EmailMessage { email_id, message_id },
            ..
        } if email_id.0 == "email-dos215-a" && message_id.is_none()
    ));

    let historical_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM entity_engagement_curve
             WHERE entity_type = ?1
               AND entity_id = ?2
               AND week_start != ?3
               AND meetings_count = 1",
            params![ENTITY_TYPE, ENTITY_ID, first.week_start.to_rfc3339()],
            |row| row.get(0),
        )
        .expect("historical backfill row");
    assert_eq!(historical_count, 1);

    let deep_backfill_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM entity_engagement_curve
             WHERE entity_type = ?1
               AND entity_id = ?2
               AND week_start = '2024-08-05T00:00:00+00:00'",
            params![ENTITY_TYPE, ENTITY_ID],
            |row| row.get(0),
        )
        .expect("deep historical row count");
    assert_eq!(
        deep_backfill_count, 0,
        "first refresh should process a bounded backfill chunk, not the full retention window"
    );

    let state_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM temporal_backfill_state
             WHERE entity_type = ?1
               AND entity_id = ?2
               AND ability_id = 'refresh_engagement_curve'",
            params![ENTITY_TYPE, ENTITY_ID],
            |row| row.get(0),
        )
        .expect("backfill state count");
    assert_eq!(state_count, 1);
}

#[test]
fn engagement_refresh_partitions_same_id_by_entity_type() {
    let conn = seeded_temporal_conn();
    let db = ActionDb::from_conn(&conn);
    let computed_at = Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap();
    let shared_id = "shared-cross-type";

    conn.execute(
        "INSERT INTO accounts (id, name, updated_at)
         VALUES (?1, 'Shared Account', '2026-05-01T00:00:00Z')",
        [shared_id],
    )
    .expect("seed account");
    conn.execute(
        "INSERT INTO projects (id, name, updated_at)
         VALUES (?1, 'Shared Project', '2026-05-01T00:00:00Z')",
        [shared_id],
    )
    .expect("seed project");
    seed_engagement_input_for_type(&conn, shared_id, "account", "account");
    seed_engagement_input_for_type(&conn, shared_id, "project", "project-a");
    conn.execute(
        "INSERT INTO emails (email_id, subject, received_at, entity_id, entity_type, user_is_last_sender)
         VALUES ('project-b-email', 'Project B', '2026-05-01T11:00:00Z', ?1, 'project', 1)",
        [shared_id],
    )
    .expect("seed second project email");

    for entity_type in ["account", "project"] {
        refresh_engagement_curve_in_db(
            db,
            RefreshEngagementCurveInput {
                schema_version: 2,
                entity_type: entity_type.to_string(),
                entity_id: shared_id.to_string(),
            },
            computed_at,
        )
        .expect("refresh engagement for colliding entity id");
    }

    let rows: Vec<(String, i64, i64)> = {
        let mut stmt = conn
            .prepare(
                "SELECT entity_type, meetings_count, emails_count
                 FROM entity_engagement_curve
                 WHERE entity_id = ?1
                 ORDER BY entity_type ASC",
            )
            .expect("prepare engagement rows");
        stmt.query_map([shared_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .expect("query engagement rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("map engagement rows")
    };

    assert_eq!(
        rows,
        vec![("account".to_string(), 1, 1), ("project".to_string(), 1, 2),],
        "same textual id must produce separate account and project engagement rows"
    );
}

#[test]
fn role_change_detection_appends_and_closes_prior_entry() {
    let conn = seeded_temporal_conn();
    let db = ActionDb::from_conn(&conn);
    let computed_at = Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap();
    let first_at = Utc.with_ymd_and_hms(2026, 5, 1, 9, 0, 0).unwrap();
    let second_at = Utc.with_ymd_and_hms(2026, 5, 5, 9, 0, 0).unwrap();

    let first = detect_role_change_in_db(
        db,
        DetectRoleChangeInput {
            schema_version: 2,
            entity_type: "person".to_string(),
            entity_id: ENTITY_ID.to_string(),
            observed_at: Some(first_at),
            title: "Engineer".to_string(),
            org: Some("Product".to_string()),
            seniority: Some("mid".to_string()),
            source_refs: source_refs(1),
        },
        computed_at,
    )
    .expect("insert first role");
    assert!(first.appended);

    let duplicate = detect_role_change_in_db(
        db,
        DetectRoleChangeInput {
            schema_version: 2,
            entity_type: "person".to_string(),
            entity_id: ENTITY_ID.to_string(),
            observed_at: Some(first_at),
            title: "Engineer".to_string(),
            org: Some("Product".to_string()),
            seniority: Some("mid".to_string()),
            source_refs: source_refs(1),
        },
        computed_at,
    )
    .expect("ignore duplicate role");
    assert!(!duplicate.appended);

    let conflicting_duplicate = detect_role_change_in_db(
        db,
        DetectRoleChangeInput {
            schema_version: 2,
            entity_type: "person".to_string(),
            entity_id: ENTITY_ID.to_string(),
            observed_at: Some(first_at),
            title: "Lead".to_string(),
            org: Some("Product".to_string()),
            seniority: Some("senior".to_string()),
            source_refs: source_refs(1),
        },
        computed_at,
    )
    .expect_err("same observed_at with different role is rejected");
    assert!(conflicting_duplicate.contains("duplicate role progression observed_at"));

    let second = detect_role_change_in_db(
        db,
        DetectRoleChangeInput {
            schema_version: 2,
            entity_type: "person".to_string(),
            entity_id: ENTITY_ID.to_string(),
            observed_at: Some(second_at),
            title: "Lead".to_string(),
            org: Some("Product".to_string()),
            seniority: Some("senior".to_string()),
            source_refs: source_refs(1),
        },
        computed_at,
    )
    .expect("insert second role");
    assert!(second.appended);
    assert_eq!(second.prior_ended_at, Some(second_at));

    let prior_ended_at: String = conn
        .query_row(
            "SELECT ended_at
             FROM person_role_progression
             WHERE entity_type = 'person' AND entity_id = ?1 AND title = 'Engineer'",
            [ENTITY_ID],
            |row| row.get(0),
        )
        .expect("read prior role");
    assert_eq!(prior_ended_at, second_at.to_rfc3339());

    let bundle = read_trajectory_bundle_from_db(
        db,
        "person",
        ENTITY_ID,
        TrajectoryQueryDepth::Weeks(52),
        computed_at,
    )
    .expect("read trajectory bundle");
    assert!(bundle.role_progression.is_some());
}

#[test]
fn source_revocation_invalidates_and_filters_temporal_rows() {
    let conn = seeded_temporal_conn();
    let db = ActionDb::from_conn(&conn);
    let computed_at = Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap();
    let role_at = Utc.with_ymd_and_hms(2026, 5, 1, 9, 0, 0).unwrap();

    seed_engagement_inputs(&conn);
    refresh_engagement_curve_in_db(
        db,
        RefreshEngagementCurveInput {
            schema_version: 2,
            entity_type: ENTITY_TYPE.to_string(),
            entity_id: ENTITY_ID.to_string(),
        },
        computed_at,
    )
    .expect("refresh engagement");
    detect_role_change_in_db(
        db,
        DetectRoleChangeInput {
            schema_version: 2,
            entity_type: "person".to_string(),
            entity_id: ENTITY_ID.to_string(),
            observed_at: Some(role_at),
            title: "Engineer".to_string(),
            org: Some("Product".to_string()),
            seniority: Some("mid".to_string()),
            source_refs: vec![google_meeting_source_ref("meeting-dos215", role_at)],
        },
        computed_at,
    )
    .expect("insert role");

    let invalidated = mark_source_invalidated_in_db(
        db,
        "google",
        Utc.with_ymd_and_hms(2026, 5, 10, 12, 0, 0).unwrap(),
    )
    .expect("mark source invalidated");
    assert_eq!(invalidated, 3);

    let invalid_engagement_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM entity_engagement_curve
             WHERE entity_type = ?1
               AND entity_id = ?2
               AND source_invalidated_at IS NOT NULL",
            params![ENTITY_TYPE, ENTITY_ID],
            |row| row.get(0),
        )
        .expect("count invalidated engagement rows");
    assert_eq!(invalid_engagement_rows, 2);

    let invalid_role_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM person_role_progression
             WHERE entity_type = 'person'
               AND entity_id = ?1
               AND source_invalidated_at IS NOT NULL",
            [ENTITY_ID],
            |row| row.get(0),
        )
        .expect("count invalidated role rows");
    assert_eq!(invalid_role_rows, 1);

    refresh_engagement_curve_in_db(
        db,
        RefreshEngagementCurveInput {
            schema_version: 2,
            entity_type: ENTITY_TYPE.to_string(),
            entity_id: ENTITY_ID.to_string(),
        },
        computed_at,
    )
    .expect("refresh invalidated engagement");
    let active_engagement_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM entity_engagement_curve
             WHERE entity_type = ?1
               AND entity_id = ?2
               AND source_invalidated_at IS NULL",
            params![ENTITY_TYPE, ENTITY_ID],
            |row| row.get(0),
        )
        .expect("count active engagement rows after refresh");
    assert_eq!(
        active_engagement_rows, 0,
        "refresh must not resurrect rows invalidated by source revocation"
    );

    let bundle = read_trajectory_bundle_from_db(
        db,
        ENTITY_TYPE,
        ENTITY_ID,
        TrajectoryQueryDepth::Weeks(52),
        computed_at,
    )
    .expect("read trajectory bundle");
    assert!(bundle.engagement_curve.is_none());
    assert!(bundle.role_progression.is_none());
}

#[test]
fn out_of_order_role_change_splits_progression_chronologically() {
    let conn = seeded_temporal_conn();
    let db = ActionDb::from_conn(&conn);
    let computed_at = Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap();
    let older_at = Utc.with_ymd_and_hms(2026, 5, 1, 9, 0, 0).unwrap();
    let newer_at = Utc.with_ymd_and_hms(2026, 5, 5, 9, 0, 0).unwrap();

    detect_role_change_in_db(
        db,
        DetectRoleChangeInput {
            schema_version: 2,
            entity_type: "person".to_string(),
            entity_id: ENTITY_ID.to_string(),
            observed_at: Some(newer_at),
            title: "Lead".to_string(),
            org: Some("Product".to_string()),
            seniority: Some("senior".to_string()),
            source_refs: source_refs(1),
        },
        computed_at,
    )
    .expect("insert newer role first");

    let older = detect_role_change_in_db(
        db,
        DetectRoleChangeInput {
            schema_version: 2,
            entity_type: "person".to_string(),
            entity_id: ENTITY_ID.to_string(),
            observed_at: Some(older_at),
            title: "Engineer".to_string(),
            org: Some("Product".to_string()),
            seniority: Some("mid".to_string()),
            source_refs: source_refs(1),
        },
        computed_at,
    )
    .expect("insert older out-of-order role");
    assert!(older.appended);
    assert_eq!(older.prior_ended_at, None);

    let rows: Vec<(String, Option<String>, String)> = {
        let mut stmt = conn
            .prepare(
                "SELECT started_at, ended_at, title
                 FROM person_role_progression
                 WHERE entity_type = 'person' AND entity_id = ?1
                 ORDER BY datetime(started_at) ASC",
            )
            .expect("prepare role progression rows");
        stmt.query_map([ENTITY_ID], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .expect("query role progression rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("map role progression rows")
    };

    assert_eq!(
        rows,
        vec![
            (
                older_at.to_rfc3339(),
                Some(newer_at.to_rfc3339()),
                "Engineer".to_string(),
            ),
            (newer_at.to_rfc3339(), None, "Lead".to_string()),
        ]
    );
}

#[test]
fn concurrent_same_observed_at_role_changes_leave_single_active_row() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("temporal-concurrency.db");
    {
        let conn = Connection::open(&db_path).expect("open setup DB");
        run_migrations(&conn).expect("apply migrations");
    }

    let computed_at = Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap();
    let observed_at = Utc.with_ymd_and_hms(2026, 5, 1, 9, 0, 0).unwrap();
    let barrier = Arc::new(Barrier::new(2));

    let handles: Vec<_> = (0..2)
        .map(|_| {
            let db_path = db_path.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let conn = Connection::open(db_path).expect("open worker DB");
                conn.busy_timeout(std::time::Duration::from_secs(5))
                    .expect("set busy timeout");
                let db = ActionDb::from_conn(&conn);
                barrier.wait();

                detect_role_change_in_db(
                    db,
                    DetectRoleChangeInput {
                        schema_version: 2,
                        entity_type: "person".to_string(),
                        entity_id: ENTITY_ID.to_string(),
                        observed_at: Some(observed_at),
                        title: "Engineer".to_string(),
                        org: Some("Product".to_string()),
                        seniority: Some("mid".to_string()),
                        source_refs: source_refs(1),
                    },
                    computed_at,
                )
            })
        })
        .collect();

    for handle in handles {
        handle
            .join()
            .expect("role change worker should not panic")
            .expect("concurrent role change should serialize");
    }

    let conn = Connection::open(&db_path).expect("open verification DB");
    let active_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM person_role_progression
             WHERE entity_type = 'person'
               AND entity_id = ?1
               AND ended_at IS NULL
               AND source_invalidated_at IS NULL",
            [ENTITY_ID],
            |row| row.get(0),
        )
        .expect("count active role rows");
    assert!(
        active_count <= 1,
        "concurrent same-observed_at writes must leave at most one active role row"
    );

    let total_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM person_role_progression
             WHERE entity_type = 'person'
               AND entity_id = ?1
               AND source_invalidated_at IS NULL",
            [ENTITY_ID],
            |row| row.get(0),
        )
        .expect("count role rows");
    assert_eq!(total_count, 1);
}

#[test]
fn title_change_signal_updates_role_progression() {
    let conn = seeded_temporal_conn();
    let db = ActionDb::from_conn(&conn);
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(215);
    let external = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &external);
    let engine = PropagationEngine::default();
    let value = serde_json::json!({
        "new_value": "Director at Example Co",
        "seniority": "senior"
    })
    .to_string();

    signals::emit_and_propagate(
        &ctx,
        db,
        &engine,
        "person",
        ENTITY_ID,
        "title_change",
        "clay",
        Some(&value),
        0.85,
    )
    .expect("emit title change signal");

    let (title, org, seniority, refs_json): (String, Option<String>, Option<String>, String) = conn
        .query_row(
            "SELECT title, org, seniority, source_refs_json
             FROM person_role_progression
             WHERE entity_type = 'person' AND entity_id = ?1",
            [ENTITY_ID],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("role progression row from title signal");
    assert_eq!(title, "Director");
    assert_eq!(org.as_deref(), Some("Example Co"));
    assert_eq!(seniority.as_deref(), Some("senior"));
    let refs: Vec<SourceRef> = serde_json::from_str(&refs_json).expect("role source refs parse");
    assert_eq!(refs.len(), 1);
    assert!(matches!(
        &refs[0],
        SourceRef::Direct {
            identifier: SourceIdentifier::Signal { signal_id },
            ..
        } if signal_id.0.starts_with("sig-")
    ));
}

fn seeded_temporal_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open DB");
    run_migrations(&conn).expect("apply migrations");
    conn
}

fn seed_engagement_inputs(conn: &Connection) {
    conn.execute(
        "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
         VALUES ('meeting-dos215', 'Temporal Sync', 'external', '2026-04-29T14:00:00Z', '2026-04-29T14:00:00Z')",
        [],
    )
    .expect("seed meeting");
    conn.execute(
        "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
         VALUES ('meeting-dos215', ?1, 'project', 1.0, 1)",
        [ENTITY_ID],
    )
    .expect("seed meeting entity");
    conn.execute(
        "INSERT INTO emails (email_id, subject, received_at, entity_id, entity_type, user_is_last_sender)
         VALUES ('email-dos215-a', 'A', '2026-04-30T10:00:00Z', ?1, 'project', 0)",
        [ENTITY_ID],
    )
    .expect("seed inbound email");
    conn.execute(
        "INSERT INTO emails (email_id, subject, received_at, entity_id, entity_type, user_is_last_sender)
         VALUES ('email-dos215-b', 'B', '2026-05-01T10:00:00Z', ?1, 'project', 1)",
        [ENTITY_ID],
    )
    .expect("seed outbound email");
    conn.execute(
        "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
         VALUES ('meeting-dos215-history', 'Historical Sync', 'external', '2025-10-15T14:00:00Z', '2025-10-15T14:00:00Z')",
        [],
    )
    .expect("seed historical meeting");
    conn.execute(
        "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
         VALUES ('meeting-dos215-history', ?1, 'project', 1.0, 1)",
        [ENTITY_ID],
    )
    .expect("seed historical meeting entity");
    conn.execute(
        "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
         VALUES ('meeting-dos215-deep-history', 'Deep Historical Sync', 'external', '2024-08-07T14:00:00Z', '2024-08-07T14:00:00Z')",
        [],
    )
    .expect("seed deep historical meeting");
    conn.execute(
        "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
         VALUES ('meeting-dos215-deep-history', ?1, 'project', 1.0, 1)",
        [ENTITY_ID],
    )
    .expect("seed deep historical meeting entity");
}

fn seed_engagement_input_for_type(
    conn: &Connection,
    entity_id: &str,
    entity_type: &str,
    source_prefix: &str,
) {
    let meeting_id = format!("{source_prefix}-meeting");
    let email_id = format!("{source_prefix}-email");
    conn.execute(
        "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
         VALUES (?1, 'Same Id Sync', 'external', '2026-04-30T14:00:00Z', '2026-04-30T14:00:00Z')",
        [&meeting_id],
    )
    .expect("seed same-id meeting");
    conn.execute(
        "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type, confidence, is_primary)
         VALUES (?1, ?2, ?3, 1.0, 1)",
        params![meeting_id, entity_id, entity_type],
    )
    .expect("seed same-id meeting entity");
    conn.execute(
        "INSERT INTO emails (email_id, subject, received_at, entity_id, entity_type, user_is_last_sender)
         VALUES (?1, 'Same Id Email', '2026-05-01T10:00:00Z', ?2, ?3, 0)",
        params![email_id, entity_id, entity_type],
    )
    .expect("seed same-id email");
}

fn seed_stale_engagement_row(conn: &Connection) {
    conn.execute(
        "INSERT INTO entity_engagement_curve (
             entity_type, entity_id, week_start, meetings_count, emails_count,
             bidirectional_ratio, source_refs_json
         ) VALUES (?1, ?2, '2024-01-01T00:00:00Z', 1, 1, 1.0, '[]')",
        params![ENTITY_TYPE, ENTITY_ID],
    )
    .expect("seed stale engagement row");
}

fn source_refs(count: usize) -> Vec<SourceRef> {
    (0..count)
        .map(|source_index| SourceRef::Source {
            source_index: SourceIndex(source_index),
        })
        .collect()
}

fn google_meeting_source_ref(meeting_id: &str, observed_at: DateTime<Utc>) -> SourceRef {
    SourceRef::Direct {
        data_source: DataSource::Google,
        identifier: SourceIdentifier::Meeting {
            meeting_id: MeetingId::new(meeting_id.to_string()),
        },
        observed_at,
        source_asof: Some(observed_at),
    }
}

fn assert_table_has_columns(conn: &Connection, table: &str, expected: &[&str]) {
    for column in expected {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM pragma_table_info(?1)
                    WHERE name = ?2
                )",
                params![table, column],
                |row| row.get::<_, i64>(0).map(|count| count != 0),
            )
            .expect("inspect table column");
        assert!(exists, "{table}.{column} should exist");
    }
}

fn assert_index_exists(conn: &Connection, index_name: &str) {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'index' AND name = ?1
            )",
            [index_name],
            |row| row.get::<_, i64>(0).map(|count| count != 0),
        )
        .expect("inspect index");
    assert!(exists, "{index_name} should exist");
}

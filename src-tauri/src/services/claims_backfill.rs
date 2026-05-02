//! DOS-7 D3b-1: backfill mechanism 9 — DismissedItem entries from
//! workspace intelligence.json files into intelligence_claims tombstone rows.
//!
//! D3a (mechanisms 1-8) handled SQL-resident dismissal mechanisms via a
//! pure SQL migration. The 9th mechanism — `DismissedItem` entries
//! embedded in per-entity `intelligence.json` files — needs a Rust pass
//! that streams JSON rows. This module owns that pass.
//!
//! D3b-2 wires this into the cutover orchestration hook (drain workers
//! → bump epoch → run SQL backfills → run THIS pass → reconcile →
//! resume).

use std::path::Path;

use rusqlite::params;

use crate::db::ActionDb;
use crate::intelligence::io::{read_intelligence_json, IntelligenceJson};
use crate::services::context::ServiceContext;

/// Result of a single workspace backfill pass.
#[derive(Debug, Default, Clone)]
pub struct DismissedItemBackfillReport {
    /// Number of entity directories scanned (had an intelligence.json).
    pub entities_scanned: usize,
    /// Number of DismissedItem entries observed across all files.
    pub items_observed: usize,
    /// Number of new tombstone claims inserted (excludes idempotent skips).
    pub claims_inserted: usize,
    /// Per-entity-kind item counts for the walkthrough report.
    pub items_by_kind: std::collections::BTreeMap<String, usize>,
}

/// Backfill DismissedItem entries from `<workspace_root>/<EntityKind>/<name>/intelligence.json`
/// into intelligence_claims tombstone rows.
///
/// Idempotent: re-runs are no-ops because each generated `dedup_key`
/// is checked against the existing claims table before INSERT.
///
/// Returns a per-pass report; the caller (D3b-2 cutover hook) aggregates
/// into the migration walkthrough.
pub fn backfill_dismissed_items_from_workspace(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    workspace_root: &Path,
) -> Result<DismissedItemBackfillReport, String> {
    ctx.check_mutation_allowed()
        .map_err(|e| format!("DOS-7 D3b-1 mutation gate: {e}"))?;

    let mut report = DismissedItemBackfillReport::default();

    // Three entity kinds the workspace currently uses. The kind name maps
    // to both the directory name and the SubjectRef enum variant in the
    // claim row.
    const ENTITY_KINDS: &[(&str, &str)] = &[
        ("Accounts", "Account"),
        ("People", "Person"),
        ("Projects", "Project"),
    ];

    for (dir_name, subject_kind) in ENTITY_KINDS {
        let kind_root = workspace_root.join(dir_name);
        if !kind_root.is_dir() {
            continue; // entity-mode may exclude this kind
        }

        let entries = match std::fs::read_dir(&kind_root) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry_result in entries {
            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => continue,
            };
            let entity_dir = entry.path();
            if !entity_dir.is_dir() {
                continue;
            }
            let intel_path = entity_dir.join("intelligence.json");
            if !intel_path.is_file() {
                continue;
            }

            let intel: IntelligenceJson = match read_intelligence_json(&entity_dir) {
                Ok(i) => i,
                Err(e) => {
                    log::warn!(
                        "[dos7-d3b1] skip {}: failed to read intelligence.json: {}",
                        intel_path.display(),
                        e
                    );
                    continue;
                }
            };

            report.entities_scanned += 1;
            *report
                .items_by_kind
                .entry((*subject_kind).to_string())
                .or_insert(0) += intel.dismissed_items.len();

            // Subject id: prefer the entity_id field on intelligence.json
            // (set during enrichment) so workspace-rename doesn't drift
            // the subject reference. Fall back to the directory name.
            let subject_id = if !intel.entity_id.is_empty() {
                intel.entity_id.clone()
            } else {
                entity_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<unknown>")
                    .to_string()
            };

            for item in &intel.dismissed_items {
                report.items_observed += 1;

                let dedup_key = format!(
                    "{}:{}:{}:dismissed_item",
                    item.content, subject_id, item.field
                );

                // Idempotency check: skip if claim with this dedup_key exists.
                let existing: i64 = db
                    .conn_ref()
                    .query_row(
                        "SELECT count(*) FROM intelligence_claims WHERE dedup_key = ?1",
                        params![&dedup_key],
                        |row| row.get(0),
                    )
                    .map_err(|e| format!("dedup check failed: {e}"))?;
                if existing > 0 {
                    continue;
                }

                let subject_ref =
                    format!(r#"{{"kind":"{}","id":"{}"}}"#, subject_kind, subject_id);
                let provenance_json = format!(
                    r#"{{"backfill_mechanism":"dismissed_item_json","source_table":"intelligence.json","source_id":"{}:{}"}}"#,
                    subject_id, item.field
                );
                let metadata_json = format!(
                    r#"{{"field":"{}","content":"{}","dismissed_at":"{}"}}"#,
                    escape_json_str(&item.field),
                    escape_json_str(&item.content),
                    escape_json_str(&item.dismissed_at)
                );
                let claim_id = format!("m9-{}-{}", subject_id, sanitize_id_segment(&item.field));

                db.conn_ref()
                    .execute(
                        "INSERT INTO intelligence_claims ( \
                            id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
                            actor, data_source, observed_at, created_at, \
                            provenance_json, metadata_json, \
                            claim_state, surfacing_state, retraction_reason, expires_at, \
                            temporal_scope, sensitivity \
                         ) VALUES ( \
                            ?1, ?2, 'dismissed_item', ?3, ?4, ?5, '', \
                            'system_backfill', 'legacy_dismissal', ?6, ?6, \
                            ?7, ?8, \
                            'tombstoned', 'active', 'user_removal', NULL, \
                            'state', 'internal' \
                         )",
                        params![
                            &claim_id,
                            &subject_ref,
                            &item.field,
                            &item.content,
                            &dedup_key,
                            &item.dismissed_at,
                            &provenance_json,
                            &metadata_json,
                        ],
                    )
                    .map_err(|e| format!("insert m9 claim: {e}"))?;

                report.claims_inserted += 1;
            }
        }
    }

    Ok(report)
}

fn escape_json_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

fn sanitize_id_segment(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

// ---------------------------------------------------------------------------
// DOS-7 D3b-2: cutover orchestration hook
// ---------------------------------------------------------------------------

use std::time::Duration;

/// Aggregated report of the full DOS-7 cutover orchestration.
#[derive(Debug, Default, Clone)]
pub struct CutoverReport {
    pub schema_epoch_before: i64,
    pub schema_epoch_after: i64,
    pub drain_in_flight_remaining: usize,
    pub drain_timed_out: bool,
    pub sql_migrations_applied: usize,
    pub json_blob_report: DismissedItemBackfillReport,
    pub reconcile_findings: usize,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

/// D5 stub: ghost-resurrection reconcile pass per plan §8. Today returns
/// 0 findings; D5 implements the actual SQL parity check against
/// scripts/reconcile_ghost_resurrection.sql.
#[derive(Debug, Default, Clone)]
pub struct ReconcilePostMigrationReport {
    pub findings: usize,
    pub finding_summary: Vec<String>,
}

pub fn reconcile_dos7_post_migration(
    _db: &ActionDb,
) -> Result<ReconcilePostMigrationReport, String> {
    // TODO(D5): run scripts/reconcile_ghost_resurrection.sql against the
    // post-cutover DB and return findings. Cutover refuses to complete
    // when findings > 0.
    Ok(ReconcilePostMigrationReport::default())
}

fn read_schema_epoch(db: &ActionDb) -> Result<i64, String> {
    db.conn_ref()
        .query_row(
            "SELECT value FROM migration_state WHERE key = 'schema_epoch'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| format!("read schema_epoch: {e}"))
}

/// Run the full DOS-7 cutover sequence atomically. Returns Err if any
/// step fails so the caller can roll back from the pre-migration backup
/// (created by the migration runner before any version applies).
///
/// Sequence per plan §2 "Migration fence" + §8 "Failure modes":
///   1. Pre-flight log
///   2. Bump schema_epoch (causes in-flight workers to abort on recheck)
///   3. Drain in-flight FenceCycle handles (30s timeout)
///   4. Run pending schema/backfill migrations (131 + 132 + any newer)
///   5. Run JSON-blob (mechanism 9) backfill
///   6. Reconcile pass (D5 stub today)
///   7. Resume — no-op; workers re-capture epoch on next pickup
pub fn run_dos7_cutover(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    workspace_root: &Path,
) -> Result<CutoverReport, String> {
    ctx.check_mutation_allowed()
        .map_err(|e| format!("DOS-7 cutover mutation gate: {e}"))?;

    log::info!(
        "[DOS-7 cutover] starting; workspace={}",
        workspace_root.display()
    );

    let mut report = CutoverReport {
        completed_at: chrono::Utc::now(),
        ..Default::default()
    };

    // Step 2: bump schema_epoch.
    let epoch_before = read_schema_epoch(db)?;
    let epoch_after = crate::intelligence::write_fence::bump_schema_epoch(db)?;
    report.schema_epoch_before = epoch_before;
    report.schema_epoch_after = epoch_after;
    log::info!(
        "[DOS-7 cutover] schema_epoch: {} -> {}",
        epoch_before,
        epoch_after
    );

    // Step 3: drain in-flight FenceCycle handles.
    match crate::intelligence::write_fence::drain_with_timeout(Duration::from_secs(30)) {
        Ok(remaining) => {
            report.drain_in_flight_remaining = remaining;
            report.drain_timed_out = false;
        }
        Err(remaining) => {
            report.drain_in_flight_remaining = remaining;
            report.drain_timed_out = true;
            return Err(format!(
                "DOS-7 cutover: drain timed out with {remaining} in-flight FenceCycle handle(s); aborting before backfill per plan §8"
            ));
        }
    }
    log::info!(
        "[DOS-7 cutover] drained; in_flight={}",
        report.drain_in_flight_remaining
    );

    // Step 4: run pending migrations. Migrations 131+132 (DOS-7 SQL
    // backfills) plus any newer registered versions are applied here.
    let applied = crate::migrations::run_migrations(db.conn_ref())
        .map_err(|e| format!("DOS-7 cutover: migrations failed: {e}"))?;
    report.sql_migrations_applied = applied;
    log::info!("[DOS-7 cutover] migrations applied: {}", applied);

    // Step 5: JSON-blob backfill (mechanism 9).
    let json_report = backfill_dismissed_items_from_workspace(ctx, db, workspace_root)?;
    report.json_blob_report = json_report.clone();
    log::info!(
        "[DOS-7 cutover] JSON-blob backfill: {} entities scanned, {} items observed, {} claims inserted",
        json_report.entities_scanned,
        json_report.items_observed,
        json_report.claims_inserted,
    );

    // Step 6: reconcile pass (D5 stub today; full impl gates on
    // scripts/reconcile_ghost_resurrection.sql findings = 0).
    let reconcile = reconcile_dos7_post_migration(db)?;
    report.reconcile_findings = reconcile.findings;
    if reconcile.findings > 0 {
        return Err(format!(
            "DOS-7 cutover: reconcile pass found {} ghost-resurrection or shape-mismatch finding(s); per plan §8 the migration is not complete",
            reconcile.findings
        ));
    }

    // Step 7: resume is a no-op — the next FenceCycle::capture() reads the
    // new epoch and proceeds normally; any in-flight write that sees the
    // bumped epoch on recheck aborts and re-queues.
    log::info!("[DOS-7 cutover] complete");
    report.completed_at = chrono::Utc::now();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::TimeZone;
    use rusqlite::Connection;
    use std::fs;

    fn fixture_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        let claims_schema = include_str!("../migrations/129_dos_7_claims_schema.sql");
        conn.execute_batch(claims_schema).unwrap();
        conn
    }

    fn write_intel_json(workspace: &Path, kind_dir: &str, entity_name: &str, body: &str) {
        let dir = workspace.join(kind_dir).join(entity_name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("intelligence.json"), body).unwrap();
    }

    #[test]
    fn empty_workspace_produces_zero_claims() {
        let workspace = tempfile::tempdir().unwrap();
        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report =
            backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path()).unwrap();
        assert_eq!(report.entities_scanned, 0);
        assert_eq!(report.items_observed, 0);
        assert_eq!(report.claims_inserted, 0);
    }

    #[test]
    fn account_with_dismissed_items_backfills_to_tombstone_claims() {
        let workspace = tempfile::tempdir().unwrap();
        let body = serde_json::json!({
            "version": 4,
            "entityId": "acct-1",
            "entityType": "account",
            "enrichedAt": "2026-04-01T00:00:00Z",
            "sourceFileCount": 0,
            "dismissedItems": [
                {
                    "field": "risks",
                    "content": "Risk that user dismissed",
                    "dismissedAt": "2026-04-15T00:00:00Z"
                },
                {
                    "field": "recentWins",
                    "content": "Win that user dismissed",
                    "dismissedAt": "2026-04-16T00:00:00Z"
                }
            ]
        });
        write_intel_json(
            workspace.path(),
            "Accounts",
            "acct-1",
            &serde_json::to_string_pretty(&body).unwrap(),
        );

        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report =
            backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path()).unwrap();

        assert_eq!(report.entities_scanned, 1);
        assert_eq!(report.items_observed, 2);
        assert_eq!(report.claims_inserted, 2);
        assert_eq!(report.items_by_kind.get("Account"), Some(&2));

        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM intelligence_claims \
                 WHERE claim_state = 'tombstoned' AND data_source = 'legacy_dismissal' \
                   AND id LIKE 'm9-%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);

        // Verify subject_ref shape + claim_type.
        let (subject_ref, claim_type, field_path, text): (String, String, String, String) = db
            .conn_ref()
            .query_row(
                "SELECT subject_ref, claim_type, field_path, text \
                 FROM intelligence_claims WHERE id LIKE 'm9-%' AND field_path = 'risks'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert!(subject_ref.contains("\"kind\":\"Account\""));
        assert!(subject_ref.contains("\"id\":\"acct-1\""));
        assert_eq!(claim_type, "dismissed_item");
        assert_eq!(field_path, "risks");
        assert_eq!(text, "Risk that user dismissed");
    }

    #[test]
    fn rerun_is_idempotent() {
        let workspace = tempfile::tempdir().unwrap();
        let body = serde_json::json!({
            "version": 4,
            "entityId": "acct-2",
            "entityType": "account",
            "enrichedAt": "2026-04-01T00:00:00Z",
            "sourceFileCount": 0,
            "dismissedItems": [{
                "field": "risks",
                "content": "Same dismissed risk",
                "dismissedAt": "2026-04-15T00:00:00Z"
            }]
        });
        write_intel_json(
            workspace.path(),
            "Accounts",
            "acct-2",
            &serde_json::to_string_pretty(&body).unwrap(),
        );

        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let first = backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path()).unwrap();
        assert_eq!(first.claims_inserted, 1);

        let second =
            backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path()).unwrap();
        assert_eq!(
            second.claims_inserted, 0,
            "second pass must be idempotent — dedup_key already exists"
        );
        assert_eq!(second.items_observed, 1, "but the items are still observed");
    }

    #[test]
    fn missing_kind_directory_skips_silently() {
        // Workspace with no Accounts/People/Projects subdirs at all.
        let workspace = tempfile::tempdir().unwrap();
        // Drop in only a People/ dir so the function still iterates the
        // workspace root cleanly.
        fs::create_dir_all(workspace.path().join("People")).unwrap();

        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report =
            backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path()).unwrap();
        assert_eq!(report.entities_scanned, 0);
        assert_eq!(report.claims_inserted, 0);
    }

    #[test]
    fn person_kind_uses_person_subject_ref() {
        let workspace = tempfile::tempdir().unwrap();
        let body = serde_json::json!({
            "version": 4,
            "entityId": "person-jane",
            "entityType": "person",
            "enrichedAt": "2026-04-01T00:00:00Z",
            "sourceFileCount": 0,
            "dismissedItems": [{
                "field": "stakeholderInsights",
                "content": "Dismissed insight",
                "dismissedAt": "2026-04-15T00:00:00Z"
            }]
        });
        write_intel_json(
            workspace.path(),
            "People",
            "person-jane",
            &serde_json::to_string_pretty(&body).unwrap(),
        );

        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report =
            backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path()).unwrap();
        assert_eq!(report.claims_inserted, 1);

        let subject_ref: String = db
            .conn_ref()
            .query_row(
                "SELECT subject_ref FROM intelligence_claims WHERE id LIKE 'm9-%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(subject_ref.contains("\"kind\":\"Person\""));
        assert!(subject_ref.contains("\"id\":\"person-jane\""));
    }

    // ---------------------------------------------------------------------
    // D3b-2 cutover hook tests.
    // The cutover relies on the FULL migration runner (which expects the
    // schema_version + migration_state tables). For these tests we apply
    // ALL registered migrations, then reset state to exercise the relevant
    // step.
    // ---------------------------------------------------------------------

    fn fresh_full_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrations::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn cutover_reconcile_stub_returns_zero_findings() {
        let conn = fresh_full_db();
        let db = ActionDb::from_conn(&conn);
        let report = reconcile_dos7_post_migration(db).unwrap();
        assert_eq!(report.findings, 0);
        assert!(report.finding_summary.is_empty());
    }

    #[test]
    fn cutover_bumps_epoch_then_runs_backfills_then_returns_clean_report() {
        let workspace = tempfile::tempdir().unwrap();
        let body = serde_json::json!({
            "version": 4,
            "entityId": "acct-cutover",
            "entityType": "account",
            "enrichedAt": "2026-04-01T00:00:00Z",
            "sourceFileCount": 0,
            "dismissedItems": [{
                "field": "risks",
                "content": "Risk for cutover test",
                "dismissedAt": "2026-04-15T00:00:00Z"
            }]
        });
        write_intel_json(
            workspace.path(),
            "Accounts",
            "acct-cutover",
            &serde_json::to_string_pretty(&body).unwrap(),
        );

        let conn = fresh_full_db();
        let db = ActionDb::from_conn(&conn);

        let clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report = run_dos7_cutover(&ctx, db, workspace.path()).unwrap();

        // Epoch advanced by exactly 1.
        assert_eq!(report.schema_epoch_after, report.schema_epoch_before + 1);

        // Drain was clean (no in-flight cycles in tests).
        assert_eq!(report.drain_in_flight_remaining, 0);
        assert!(!report.drain_timed_out);

        // SQL migrations: 0 since fresh_full_db already applied everything.
        assert_eq!(report.sql_migrations_applied, 0);

        // JSON-blob backfill picked up the synthetic intelligence.json.
        assert_eq!(report.json_blob_report.entities_scanned, 1);
        assert_eq!(report.json_blob_report.items_observed, 1);
        assert_eq!(report.json_blob_report.claims_inserted, 1);

        // Reconcile stub clean.
        assert_eq!(report.reconcile_findings, 0);
    }

    #[test]
    fn cutover_aggregates_json_blob_report_correctly_across_entity_kinds() {
        let workspace = tempfile::tempdir().unwrap();

        let acct_body = serde_json::json!({
            "version": 4, "entityId": "acct-A", "entityType": "account",
            "enrichedAt": "2026-04-01T00:00:00Z", "sourceFileCount": 0,
            "dismissedItems": [
                {"field": "risks", "content": "r1", "dismissedAt": "2026-04-15T00:00:00Z"},
                {"field": "recentWins", "content": "w1", "dismissedAt": "2026-04-16T00:00:00Z"}
            ]
        });
        let person_body = serde_json::json!({
            "version": 4, "entityId": "person-B", "entityType": "person",
            "enrichedAt": "2026-04-01T00:00:00Z", "sourceFileCount": 0,
            "dismissedItems": [
                {"field": "stakeholderInsights", "content": "i1", "dismissedAt": "2026-04-15T00:00:00Z"}
            ]
        });
        write_intel_json(
            workspace.path(),
            "Accounts",
            "acct-A",
            &serde_json::to_string_pretty(&acct_body).unwrap(),
        );
        write_intel_json(
            workspace.path(),
            "People",
            "person-B",
            &serde_json::to_string_pretty(&person_body).unwrap(),
        );

        let conn = fresh_full_db();
        let db = ActionDb::from_conn(&conn);
        let clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report = run_dos7_cutover(&ctx, db, workspace.path()).unwrap();

        assert_eq!(report.json_blob_report.entities_scanned, 2);
        assert_eq!(report.json_blob_report.items_observed, 3);
        assert_eq!(report.json_blob_report.claims_inserted, 3);
        assert_eq!(report.json_blob_report.items_by_kind.get("Account"), Some(&2));
        assert_eq!(report.json_blob_report.items_by_kind.get("Person"), Some(&1));
    }

    #[test]
    #[ignore = "TODO(D5): drain-timeout test seam — needs FenceCycle injection without depending on full state"]
    fn cutover_aborts_on_drain_timeout_before_running_backfill() {
        // Hold a FenceCycle for longer than the cutover's drain timeout
        // (30s) and assert the cutover returns Err with drain_timed_out=true
        // and no claims inserted.
    }
}

//! DOS-7 D3b-1: backfill mechanism 9 — DismissedItem entries from
//! workspace intelligence.json files into intelligence_claims tombstone rows.
//!
//! D3a (mechanisms 1-8) handled SQL-resident dismissal mechanisms via a
//! pure SQL migration. The 9th mechanism — `DismissedItem` entries
//! embedded in per-entity `intelligence.json` files — needs a Rust pass
//! that streams JSON rows. This module owns that pass.
//!
//! D3b-2 wires this into the cutover orchestration hook (drain workers
//! → bump epoch → run SQL backfills → rekey m1-m8 → run THIS pass → reconcile →
//! resume).

use std::path::Path;

use rusqlite::params;

use crate::db::ActionDb;
use crate::intelligence::canonicalization::item_hash;
use crate::intelligence::io::{read_intelligence_json, IntelligenceJson};
use crate::services::claims::{
    canonicalize_for_dos280, compute_dedup_key, item_kind_for_claim_type,
};
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

/// Report for the DOS-7 L2 rekey pass over SQL-backfilled m1-m8 claims.
#[derive(Debug, Default, Clone)]
pub struct RekeyReport {
    pub rows_examined: usize,
    pub rows_rewritten: usize,
    pub errors: Vec<String>,
}

#[derive(Debug)]
struct RekeyRow {
    id: String,
    subject_ref: String,
    claim_type: String,
    field_path: Option<String>,
    text: String,
    dedup_key: String,
    item_hash: Option<String>,
}

/// Recompute SQL-backfilled claim identity with the same helpers used by
/// `commit_claim`.
///
/// Migration 130/131 rows keep their original assertion columns. This pass
/// only updates the lifecycle/identity columns that the D4 lint allowlist
/// permits here: `dedup_key` and `item_hash`.
pub fn rekey_backfilled_claims_via_runtime_helpers(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
) -> Result<RekeyReport, String> {
    ctx.check_mutation_allowed()
        .map_err(|e| format!("DOS-7 L2 rekey mutation gate: {e}"))?;

    let conn = db.conn_ref();
    let mut report = RekeyReport::default();
    let mut rows = Vec::new();

    {
        let mut stmt = conn
            .prepare(
                "SELECT id, subject_ref, claim_type, field_path, text, dedup_key, item_hash \
                 FROM intelligence_claims \
                 WHERE id GLOB 'm[1-8]-*' \
                 ORDER BY id",
            )
            .map_err(|e| format!("DOS-7 L2 rekey select prepare failed: {e}"))?;

        let mapped = stmt
            .query_map([], |row| {
                Ok(RekeyRow {
                    id: row.get(0)?,
                    subject_ref: row.get(1)?,
                    claim_type: row.get(2)?,
                    field_path: row.get(3)?,
                    text: row.get(4)?,
                    dedup_key: row.get(5)?,
                    item_hash: row.get(6)?,
                })
            })
            .map_err(|e| format!("DOS-7 L2 rekey query failed: {e}"))?;

        for row in mapped {
            match row {
                Ok(row) => rows.push(row),
                Err(e) => report.errors.push(format!("read m1-m8 claim row: {e}")),
            }
        }
    }

    for row in rows {
        report.rows_examined += 1;

        let result = runtime_identity_for_rekey(&row).and_then(|(next_dedup_key, next_hash)| {
            if row.dedup_key == next_dedup_key
                && row.item_hash.as_deref() == Some(next_hash.as_str())
            {
                return Ok(0);
            }

            conn.execute(
                "UPDATE intelligence_claims \
                 SET dedup_key = ?1, item_hash = ?2 \
                 WHERE id = ?3",
                params![&next_dedup_key, &next_hash, &row.id],
            )
            .map_err(|e| format!("update dedup_key/item_hash: {e}"))
        });

        match result {
            Ok(0) => {}
            Ok(_) => report.rows_rewritten += 1,
            Err(e) => report.errors.push(format!("{}: {}", row.id, e)),
        }
    }

    Ok(report)
}

fn runtime_identity_for_rekey(row: &RekeyRow) -> Result<(String, String), String> {
    let subject_value = serde_json::from_str::<serde_json::Value>(&row.subject_ref)
        .map_err(|e| format!("subject_ref is not JSON: {e}"))?;

    for key in ["kind", "id"] {
        let value = subject_value
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if value.is_none() {
            return Err(format!("subject_ref missing non-empty {key}"));
        }
    }

    let compact_subject_ref =
        serde_json::to_string(&subject_value).map_err(|e| format!("compact subject_ref: {e}"))?;
    let canonical_text = canonicalize_for_dos280(&row.text);
    let next_hash = item_hash(item_kind_for_claim_type(&row.claim_type), &canonical_text);
    let next_dedup_key = compute_dedup_key(
        &next_hash,
        &compact_subject_ref,
        &row.claim_type,
        row.field_path.as_deref(),
    );

    Ok((next_dedup_key, next_hash))
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
    pub rekey_report: RekeyReport,
    pub json_blob_report: DismissedItemBackfillReport,
    pub reconcile_findings: usize,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

/// DOS-7 D5-1: post-migration reconcile pass.
///
/// For each of the 8 SQL-resident dismissal mechanisms, count the legacy
/// source rows that the migration WHERE filter would have backfilled and
/// compare against the population of `m{N}-` prefixed claim rows.
/// A non-zero legacy count with a zero claim count is a hard finding
/// (the mechanism's INSERT failed silently or never ran). A claim count
/// strictly less than the expected count is a soft finding (some rows
/// failed dedup or filter divergence). Cutover refuses to complete when
/// `findings > 0`.
///
/// Mechanism 9 (DismissedItem JSON-blob) is handled by the in-process
/// backfill that emits its own report; no additional reconcile here.
#[derive(Debug, Default, Clone)]
pub struct ReconcilePostMigrationReport {
    pub findings: usize,
    pub finding_summary: Vec<String>,
    pub per_mechanism_counts: Vec<MechanismCount>,
}

#[derive(Debug, Clone)]
pub struct MechanismCount {
    pub mechanism: u8,
    pub label: &'static str,
    /// Source row count after applying the migration's WHERE filter.
    /// `None` when the legacy table is absent (fresh DB / test fixture).
    pub legacy_expected: Option<i64>,
    /// `m{N}-` prefixed rows present in `intelligence_claims`.
    pub claims_present: i64,
}

pub fn reconcile_dos7_post_migration(
    db: &ActionDb,
) -> Result<ReconcilePostMigrationReport, String> {
    let conn = db.conn_ref();
    let mut report = ReconcilePostMigrationReport::default();

    let checks: [(u8, &str, &str, &str); 8] = [
        (
            1,
            "suppression_tombstones",
            // Latest dismissed_at wins per (entity_id, field_key, item_key, item_hash);
            // older rows become claim_corroborations and are not counted as expected
            // claim rows.
            "SELECT count(*) FROM suppression_tombstones t1 \
             WHERE NOT EXISTS ( \
                 SELECT 1 FROM suppression_tombstones t2 \
                 WHERE t2.entity_id = t1.entity_id \
                   AND t2.field_key = t1.field_key \
                   AND coalesce(t2.item_key, '') = coalesce(t1.item_key, '') \
                   AND coalesce(t2.item_hash, '') = coalesce(t1.item_hash, '') \
                   AND t2.dismissed_at > t1.dismissed_at \
             )",
            "m1-%",
        ),
        (
            2,
            "account_stakeholder_roles",
            "SELECT count(*) FROM account_stakeholder_roles WHERE dismissed_at IS NOT NULL",
            "m2-%",
        ),
        (
            3,
            "email_dismissals",
            "SELECT count(*) FROM email_dismissals",
            "m3-%",
        ),
        (
            4,
            "meeting_entity_dismissals",
            "SELECT count(*) FROM meeting_entity_dismissals",
            "m4-%",
        ),
        (
            5,
            "linking_dismissals",
            // Mechanism 5 backfills rows whose owner is not a meeting OR whose
            // ld.created_at is newer than the matched meeting_entity_dismissals
            // dismissed_at. The remainder become m5-m4 corroborations only.
            "SELECT count(*) FROM linking_dismissals ld \
             LEFT JOIN meeting_entity_dismissals med \
               ON ld.owner_type = 'meeting' \
              AND med.meeting_id = ld.owner_id \
              AND med.entity_id = ld.entity_id \
              AND med.entity_type = ld.entity_type \
             WHERE med.meeting_id IS NULL OR ld.created_at > med.dismissed_at",
            "m5-%",
        ),
        (
            6,
            "briefing_callouts",
            "SELECT count(*) FROM briefing_callouts WHERE dismissed_at IS NOT NULL",
            "m6-%",
        ),
        (
            7,
            "nudge_dismissals",
            "SELECT count(*) FROM nudge_dismissals",
            "m7-%",
        ),
        (
            8,
            "triage_snoozes",
            // Migration filter uses `snoozed_until > datetime('now')`; the
            // reconcile clock has advanced since migration, so a snooze that
            // was active at migration time may now be expired. We use the
            // permissive expected = (resolved OR active-at-migration-or-now)
            // bound: a row is expected iff resolved_at IS NOT NULL OR
            // snoozed_until is non-null. This catches "mechanism never ran"
            // without false-positives from clock drift.
            "SELECT count(*) FROM triage_snoozes \
             WHERE resolved_at IS NOT NULL OR snoozed_until IS NOT NULL",
            "m8-%",
        ),
    ];

    for (mechanism, label, legacy_sql, claim_id_prefix) in checks {
        let legacy_expected: Option<i64> = match conn.query_row(legacy_sql, [], |r| r.get(0)) {
            Ok(n) => Some(n),
            Err(rusqlite::Error::SqliteFailure(_, Some(msg))) if msg.contains("no such table") => {
                None
            }
            Err(rusqlite::Error::SqliteFailure(_, None)) => None,
            Err(e) => {
                let s = format!("{e}");
                if s.contains("no such table") {
                    None
                } else {
                    return Err(format!(
                        "DOS-7 reconcile: legacy count for mechanism {mechanism} ({label}) failed: {e}"
                    ));
                }
            }
        };

        let claims_present: i64 = conn
            .query_row(
                "SELECT count(*) FROM intelligence_claims WHERE id LIKE ?1",
                [claim_id_prefix],
                |r| r.get(0),
            )
            .map_err(|e| {
                format!("DOS-7 reconcile: claim count for mechanism {mechanism} ({label}) failed: {e}")
            })?;

        if let Some(expected) = legacy_expected {
            if expected > 0 && claims_present == 0 {
                report.findings += 1;
                report.finding_summary.push(format!(
                    "mechanism {mechanism} ({label}): {expected} legacy rows but 0 m{mechanism}- claims — backfill did not run"
                ));
            } else if claims_present < expected {
                report.findings += 1;
                report.finding_summary.push(format!(
                    "mechanism {mechanism} ({label}): {expected} legacy rows, {claims_present} m{mechanism}- claims — gap of {} ({} missing)",
                    expected - claims_present,
                    expected - claims_present,
                ));
            }
        }

        report.per_mechanism_counts.push(MechanismCount {
            mechanism,
            label,
            legacy_expected,
            claims_present,
        });
    }

    Ok(report)
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
///      Rekey SQL-backfilled m1-m8 claims via runtime helpers
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

    // Step 4.5: rekey SQL-backfilled claims to the runtime DOS-280 shape.
    // Rekey row failures are reported but do not fail cutover; the reconcile
    // pass remains the hard gate for missing migrated claims.
    let rekey_report = match rekey_backfilled_claims_via_runtime_helpers(ctx, db) {
        Ok(report) => report,
        Err(e) => {
            let mut report = RekeyReport::default();
            report.errors.push(e);
            report
        }
    };
    log::info!(
        "[DOS-7 cutover] m1-m8 rekey: {} rows examined, {} rows rewritten, {} error(s)",
        rekey_report.rows_examined,
        rekey_report.rows_rewritten,
        rekey_report.errors.len(),
    );
    if !rekey_report.errors.is_empty() {
        log::warn!(
            "[DOS-7 cutover] m1-m8 rekey completed with errors: {:?}",
            rekey_report.errors
        );
    }
    report.rekey_report = rekey_report;

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

/// Migration-state key recording the unix timestamp at which
/// [`run_dos7_cutover`] last completed successfully against this DB.
/// Persists across runs so startup can be idempotent.
const DOS7_CUTOVER_COMPLETED_AT_KEY: &str = "dos7_cutover_completed_at";

/// L2 cycle-1 fix #3: idempotently run the DOS-7 cutover (rekey + JSON-blob
/// backfill + reconcile) on startup.
///
/// The cutover MUST run after migrations 130/131 apply, otherwise:
///   - JSON-blob (mechanism 9) backfill never runs: pre-DOS-7 dismissed
///     items stored in workspace `intelligence.json` files don't get
///     promoted into intelligence_claims tombstones.
///   - The rekey pass never normalizes m1-m8 dedup_keys to the runtime
///     shape that `compute_dedup_key` produces.
///   - Reconcile never fires, so a partial backfill silently ships.
///
/// Idempotency: once a cutover succeeds, the unix timestamp is recorded in
/// `migration_state` under [`DOS7_CUTOVER_COMPLETED_AT_KEY`]. Subsequent
/// startups read that timestamp and skip the work.
///
/// Returns `Ok(None)` when the cutover was already complete; `Ok(Some)`
/// with the report when it just ran. Errors propagate so the caller can
/// log + continue with degraded behavior or surface to the operator.
pub fn run_dos7_cutover_if_pending(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    workspace_root: &Path,
) -> Result<Option<CutoverReport>, String> {
    // Only run when the DOS-7 schema has been applied (migration 130 → SQL
    // version 130). On a fresh DB before migrations, this is a no-op.
    let claims_table_exists: bool = db
        .conn_ref()
        .query_row(
            "SELECT count(*) FROM sqlite_master \
             WHERE type = 'table' AND name = 'intelligence_claims'",
            [],
            |row| row.get::<_, i64>(0).map(|c| c > 0),
        )
        .map_err(|e| format!("DOS-7 cutover startup gate: {e}"))?;
    if !claims_table_exists {
        return Ok(None);
    }

    let already_completed: Option<i64> = db
        .conn_ref()
        .query_row(
            "SELECT value FROM migration_state WHERE key = ?1",
            [DOS7_CUTOVER_COMPLETED_AT_KEY],
            |row| row.get(0),
        )
        .ok();
    if let Some(ts) = already_completed {
        log::debug!(
            "[DOS-7 cutover] already completed at unix={}; skipping startup hook",
            ts
        );
        return Ok(None);
    }

    log::info!("[DOS-7 cutover] startup hook: cutover not yet recorded — running now");
    let report = run_dos7_cutover(ctx, db, workspace_root)?;

    // Record completion atomically. INSERT OR REPLACE so re-runs are safe.
    db.conn_ref()
        .execute(
            "INSERT OR REPLACE INTO migration_state (key, value) VALUES (?1, ?2)",
            rusqlite::params![DOS7_CUTOVER_COMPLETED_AT_KEY, chrono::Utc::now().timestamp()],
        )
        .map_err(|e| format!("DOS-7 cutover record completion: {e}"))?;

    Ok(Some(report))
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

    struct RekeySeed<'a> {
        id: &'a str,
        subject_ref: &'a str,
        claim_type: &'a str,
        field_path: Option<&'a str>,
        text: &'a str,
        dedup_key: &'a str,
        item_hash: Option<&'a str>,
    }

    fn seed_rekey_claim(db: &ActionDb, seed: RekeySeed<'_>) {
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims \
                 (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
                  actor, data_source, observed_at, created_at, provenance_json, \
                  claim_state, surfacing_state, retraction_reason, \
                  temporal_scope, sensitivity) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, \
                         'system_backfill', 'legacy_dismissal', ?8, ?8, '{}', \
                         'tombstoned', 'active', 'user_removal', \
                         'state', 'internal')",
                params![
                    seed.id,
                    seed.subject_ref,
                    seed.claim_type,
                    seed.field_path,
                    seed.text,
                    seed.dedup_key,
                    seed.item_hash,
                    "2026-04-15T00:00:00Z",
                ],
            )
            .unwrap();
    }

    fn expected_runtime_identity(
        subject_ref: &str,
        claim_type: &str,
        field_path: Option<&str>,
        text: &str,
    ) -> (String, String) {
        let subject_value = serde_json::from_str::<serde_json::Value>(subject_ref).unwrap();
        let compact_subject_ref = serde_json::to_string(&subject_value).unwrap();
        let canonical_text = crate::services::claims::canonicalize_for_dos280(text);
        let hash = crate::intelligence::canonicalization::item_hash(
            crate::services::claims::item_kind_for_claim_type(claim_type),
            &canonical_text,
        );
        let dedup_key = crate::services::claims::compute_dedup_key(
            &hash,
            &compact_subject_ref,
            claim_type,
            field_path,
        );

        (dedup_key, hash)
    }

    #[test]
    fn rekey_rewrites_m1_dedup_key_to_runtime_shape() {
        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let subject_ref = r#"{"kind":"Account","id":"acct-1"}"#;
        let text = "Procurement blocked renewal";
        let legacy_dedup = "legacy-hash:acct-1:risk:risks";
        let legacy_hash = "legacy-hash";
        let (expected_dedup, expected_hash) =
            expected_runtime_identity(subject_ref, "risk", Some("risks"), text);
        assert_ne!(legacy_dedup, expected_dedup);
        assert_ne!(legacy_hash, expected_hash);

        seed_rekey_claim(
            &db,
            RekeySeed {
                id: "m1-1",
                subject_ref,
                claim_type: "risk",
                field_path: Some("risks"),
                text,
                dedup_key: legacy_dedup,
                item_hash: Some(legacy_hash),
            },
        );

        let report = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
        assert_eq!(report.rows_examined, 1);
        assert_eq!(report.rows_rewritten, 1);
        assert!(report.errors.is_empty(), "{:?}", report.errors);

        let (dedup_key, item_hash): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT dedup_key, item_hash FROM intelligence_claims WHERE id = 'm1-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(dedup_key, expected_dedup);
        assert_eq!(item_hash, expected_hash);
    }

    #[test]
    fn rekey_is_idempotent() {
        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        seed_rekey_claim(
            &db,
            RekeySeed {
                id: "m2-acct-1:person-1:champion",
                subject_ref: r#"{"kind":"Person","id":"person-1"}"#,
                claim_type: "stakeholder_role",
                field_path: None,
                text: "champion",
                dedup_key: "champion:acct-1:person-1:stakeholder_role",
                item_hash: Some("champion"),
            },
        );

        let first = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
        assert_eq!(first.rows_examined, 1);
        assert_eq!(first.rows_rewritten, 1);
        assert!(first.errors.is_empty(), "{:?}", first.errors);

        let second = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
        assert_eq!(second.rows_examined, 1);
        assert_eq!(second.rows_rewritten, 0);
        assert!(second.errors.is_empty(), "{:?}", second.errors);
    }

    #[test]
    fn rekey_skips_non_backfilled_claims() {
        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        seed_rekey_claim(
            &db,
            RekeySeed {
                id: "runtime-claim-1",
                subject_ref: r#"{"kind":"Account","id":"acct-1"}"#,
                claim_type: "risk",
                field_path: Some("risks"),
                text: "Runtime claim text",
                dedup_key: "keep-this-dedup-key",
                item_hash: Some("keep-this-item-hash"),
            },
        );

        let report = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
        assert_eq!(report.rows_examined, 0);
        assert_eq!(report.rows_rewritten, 0);
        assert!(report.errors.is_empty(), "{:?}", report.errors);

        let (dedup_key, item_hash): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT dedup_key, item_hash FROM intelligence_claims \
                 WHERE id = 'runtime-claim-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(dedup_key, "keep-this-dedup-key");
        assert_eq!(item_hash, "keep-this-item-hash");
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
    fn reconcile_clean_db_returns_zero_findings() {
        let conn = fresh_full_db();
        let db = ActionDb::from_conn(&conn);
        let report = reconcile_dos7_post_migration(db).unwrap();
        assert_eq!(report.findings, 0, "{:?}", report.finding_summary);
        assert!(report.finding_summary.is_empty());
        // All 8 mechanisms enumerated even on a clean DB.
        assert_eq!(report.per_mechanism_counts.len(), 8);
    }

    #[test]
    fn reconcile_detects_unbackfilled_mechanism() {
        // Seed a suppression_tombstones row but DO NOT run the m1 backfill.
        // Reconcile must surface a finding.
        let conn = fresh_full_db();
        conn.execute(
            "INSERT INTO suppression_tombstones \
             (entity_id, field_key, item_key, item_hash, source_scope, dismissed_at) \
             VALUES ('acct-x', 'risks', 'r1', 'h1', 'manual', '2026-04-15T00:00:00Z')",
            [],
        )
        .unwrap();
        let db = ActionDb::from_conn(&conn);
        let report = reconcile_dos7_post_migration(db).unwrap();
        assert!(report.findings >= 1, "expected at least one finding, got {}", report.findings);
        assert!(
            report
                .finding_summary
                .iter()
                .any(|s| s.contains("mechanism 1") && s.contains("suppression_tombstones")),
            "summary missing mechanism 1 finding: {:?}",
            report.finding_summary
        );
    }

    #[test]
    fn reconcile_after_backfill_is_clean() {
        // Seed legacy rows then run the cutover (which runs all backfills),
        // then reconcile — expect zero findings.
        let conn = fresh_full_db();
        conn.execute(
            "INSERT INTO suppression_tombstones \
             (entity_id, field_key, item_key, item_hash, source_scope, dismissed_at) \
             VALUES ('acct-y', 'risks', 'r2', 'h2', 'manual', '2026-04-15T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO email_dismissals \
             (email_id, sender_domain, email_type, item_type, item_text, dismissed_at, entity_id) \
             VALUES ('em-1', 'example.com', 'reply', 'risk', 'r3', '2026-04-16T00:00:00Z', NULL)",
            [],
        )
        .unwrap();

        // Run the m1 + m3 backfills (D3a-1 idempotent re-execution against a DB
        // that already had the migrations applied is the supported pattern).
        let m1_a1 = include_str!("../migrations/130_dos_7_claims_backfill_a1.sql");
        conn.execute_batch(m1_a1).unwrap();

        let db = ActionDb::from_conn(&conn);
        let report = reconcile_dos7_post_migration(db).unwrap();
        assert_eq!(
            report.findings, 0,
            "post-backfill reconcile must be clean: {:?}",
            report.finding_summary
        );
    }

    /// L2 cycle-1 fix #3: the startup-hook wrapper must be idempotent.
    /// First call runs the cutover; second call short-circuits because
    /// `migration_state.dos7_cutover_completed_at` is set.
    #[test]
    fn run_dos7_cutover_if_pending_is_idempotent_via_migration_state() {
        let workspace = tempfile::tempdir().unwrap();
        let conn = fresh_full_db();
        let db = ActionDb::from_conn(&conn);

        let clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        // First call: runs the cutover.
        let first =
            run_dos7_cutover_if_pending(&ctx, db, workspace.path()).expect("first cutover");
        assert!(first.is_some(), "first call must run the cutover");

        // Second call: idempotent no-op. The migration_state guard short-circuits.
        let second =
            run_dos7_cutover_if_pending(&ctx, db, workspace.path()).expect("second cutover");
        assert!(
            second.is_none(),
            "second call must skip — already recorded in migration_state"
        );
    }

    /// L2 cycle-1 fix #3: when the claims schema has not been applied
    /// yet (pre-DOS-7 DB), the startup hook must be a no-op rather than
    /// erroring. Production startup runs migrations FIRST and the hook
    /// later, but the hook can be invoked on legacy DBs where migration
    /// 130 hasn't applied yet (e.g. fresh test fixtures).
    #[test]
    fn run_dos7_cutover_if_pending_no_op_when_claims_schema_absent() {
        let workspace = tempfile::tempdir().unwrap();
        // Bare DB without the claim tables.
        let conn = Connection::open_in_memory().unwrap();
        let db = ActionDb::from_conn(&conn);

        let clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let result =
            run_dos7_cutover_if_pending(&ctx, db, workspace.path()).expect("no-op should not fail");
        assert!(result.is_none());
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

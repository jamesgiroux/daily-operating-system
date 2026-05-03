//! Claims backfill D3b-1: backfill mechanism 9 — DismissedItem entries from
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

/// Report for the claims rekey pass over SQL-backfilled m1-m8 claims.
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
    // L2 cycle-10 fix #1: read provenance_json + metadata_json so the
    // rekey pass can validate them too. Pre-cycle-9 m9 writers built
    // these via raw `format!` interpolation; legacy values with
    // quotes/backslashes/controls produced malformed JSON. Cutover
    // must catch them or the substrate ships with structurally
    // broken claim rows.
    provenance_json: String,
    metadata_json: Option<String>,
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
                "SELECT id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
                        provenance_json, metadata_json \
                 FROM intelligence_claims \
                 WHERE id GLOB 'm[1-9]-*' \
                   AND claim_state <> 'withdrawn' \
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
                    provenance_json: row.get(7)?,
                    metadata_json: row.get(8)?,
                })
            })
            .map_err(|e| format!("DOS-7 L2 rekey query failed: {e}"))?;

        for row in mapped {
            match row {
                Ok(row) => rows.push(row),
                Err(e) => report.errors.push(format!("read m1-m9 claim row: {e}")),
            }
        }
    }

    for row in rows {
        report.rows_examined += 1;

        // L2 cycle-10 fix #1: validate JSON columns before rekeying
        // identity. A malformed metadata_json or provenance_json
        // from a pre-cycle-9 m9 writer would otherwise silently
        // pass the rekey since the previous version only parsed
        // subject_ref. Failing here surfaces the bad row so the
        // cutover refuses to mark complete (cycle-2 fix #2 makes
        // any rekey error fatal).
        if let Err(e) = validate_row_json_columns(&row) {
            report.errors.push(format!("{}: {}", row.id, e));
            continue;
        }

        let result = runtime_identity_for_rekey(&row).and_then(
            |(next_subject_ref, next_dedup_key, next_hash)| {
                if row.dedup_key == next_dedup_key
                    && row.item_hash.as_deref() == Some(next_hash.as_str())
                    && row.subject_ref == next_subject_ref
                {
                    return Ok(0);
                }

                // L2 cycle-18 fix: also canonicalize subject_ref so
                // PRE-GATE / contradiction / load_claims_where (which
                // all match by `json_extract(subject_ref, '$.kind')`
                // and `'$.id'`) can see this row. Alias-shaped rows
                // ({"type":...,"entity_id":...}) and PascalCase rows
                // would otherwise pass rekey but stay invisible to
                // those readers — defeating the cycle-16 invariant
                // that backfill and runtime semantic identity match.
                //
                // subject_ref is normally an immutable assertion
                // column, but rekey is an explicit one-time
                // canonicalization pass that preserves SEMANTIC
                // meaning while normalizing BYTE shape — the same
                // justification used for the dedup_key + item_hash
                // updates above. dos7-allowed: rekey canonical
                // subject_ref normalization preserves meaning.
                conn.execute(
                    "UPDATE intelligence_claims \
                     SET dedup_key = ?1, item_hash = ?2, subject_ref = ?3 /* dos7-allowed: rekey canonical subject_ref normalization preserves semantic meaning */ \
                     WHERE id = ?4",
                    params![&next_dedup_key, &next_hash, &next_subject_ref, &row.id],
                )
                .map_err(|e| format!("update dedup_key/item_hash/subject_ref: {e}"))
            },
        );

        match result {
            Ok(0) => {}
            Ok(_) => report.rows_rewritten += 1,
            Err(e) => report.errors.push(format!("{}: {}", row.id, e)),
        }
    }

    Ok(report)
}

/// L2 cycle-10 fix #1 + cycle-11 fix #1: parse subject_ref,
/// provenance_json, and metadata_json (if present) to surface any
/// rows whose JSON columns are structurally malformed. Returns the
/// FIRST parse error, or `Ok(())` when every column round-trips
/// cleanly through serde_json.
///
/// Cycle-11 fix: metadata_json is parsed strictly when present.
/// `Some("")` and `Some("   ")` are treated as malformed (empty /
/// whitespace is not valid JSON). Operators must either NULL the
/// column or write actual JSON. The previous "trim().is_empty()
/// → skip" behavior let blank strings slip past the gate.
fn validate_row_json_columns(row: &RekeyRow) -> Result<(), String> {
    serde_json::from_str::<serde_json::Value>(&row.subject_ref)
        .map_err(|e| format!("subject_ref is not valid JSON: {e}"))?;
    serde_json::from_str::<serde_json::Value>(&row.provenance_json)
        .map_err(|e| format!("provenance_json is not valid JSON: {e}"))?;
    if let Some(metadata) = row.metadata_json.as_deref() {
        serde_json::from_str::<serde_json::Value>(metadata)
            .map_err(|e| format!("metadata_json is not valid JSON: {e}"))?;
    }
    Ok(())
}

fn runtime_identity_for_rekey(row: &RekeyRow) -> Result<(String, String, String), String> {
    let subject_value = serde_json::from_str::<serde_json::Value>(&row.subject_ref)
        .map_err(|e| format!("subject_ref is not JSON: {e}"))?;

    // L2 cycle-12 fix #2 + cycle-16 fix + cycle-17 fix: parse the
    // row's subject_ref through the SAME SubjectRef parser
    // commit_claim uses, then serialize via canonical_subject_ref
    // so the rekey-produced dedup_key is byte-identical to what
    // runtime commit_claim would produce for the same semantic
    // subject. The parser is the SINGLE source of truth for
    // supported kinds AND supported alias keys (it accepts
    // kind/type/entity_type and id/entity_id) — earlier cycles had
    // a hand-rolled `$.kind`/`$.id` precheck that was stricter
    // than the parser, falsely rejecting subjects with the
    // alternate alias shape that runtime writes accept. Removing
    // that precheck keeps rekey and commit_claim in lockstep.
    let subject = crate::services::claims::subject_ref_from_json(&subject_value)
        .map_err(|e| format!("subject_ref kind not a supported SubjectRef variant: {e}"))?;
    let compact_subject_ref =
        crate::services::claims::canonical_subject_ref(&subject).map_err(|e| {
            format!(
                "subject cannot be canonicalized for dedup_key (likely Multi/Global per ADR-0125): {e}"
            )
        })?;
    let canonical_text = canonicalize_for_dos280(&row.text);
    let next_hash = item_hash(item_kind_for_claim_type(&row.claim_type), &canonical_text);
    let next_dedup_key = compute_dedup_key(
        &next_hash,
        &compact_subject_ref,
        &row.claim_type,
        row.field_path.as_deref(),
    );

    Ok((compact_subject_ref, next_dedup_key, next_hash))
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

                // L2 cycle-9 fix: build the JSON columns via serde_json
                // so legacy intelligence.json values containing quotes,
                // backslashes, newlines, or control characters cannot
                // produce malformed JSON. The previous raw `format!`
                // interpolation poisoned downstream rekey: a malformed
                // m9 row would commit, then the rekey pass would fail
                // to parse `subject_ref` and abort the cutover; on
                // retry, the same id-keyed idempotency check skipped
                // the bad row so the failure repeated indefinitely.
                let subject_ref = serde_json::json!({
                    "kind": subject_kind,
                    "id": subject_id,
                })
                .to_string();
                let provenance_json = serde_json::json!({
                    "backfill_mechanism": "dismissed_item_json",
                    "source_table": "intelligence.json",
                    "source_id": format!("{}:{}", subject_id, item.field),
                })
                .to_string();
                let metadata_json = serde_json::json!({
                    "field": item.field,
                    "content": item.content,
                    "dismissed_at": item.dismissed_at,
                })
                .to_string();
                // L2 cycle-6 fix #2 + cycle-8 fix #1: include a stable
                // hash of `(subject_kind, subject_id, field, content)`
                // in the claim_id so:
                //   - multiple dismissedItems in the same field don't
                //     collide on the PK (cycle-6),
                //   - and Account/Person/Project subjects sharing the
                //     same legacy id+field+content don't collide
                //     across kinds (cycle-8). Without subject_kind
                //     in the hash, an Account "x-1" tombstone for
                //     the same field+content as a Person "x-1"
                //     would silently lose the second one.
                let id_seed = format!(
                    "{}\u{1f}{}\u{1f}{}\u{1f}{}",
                    subject_kind, subject_id, item.field, item.content
                );
                let id_hash = crate::intelligence::canonicalization::item_hash(
                    crate::intelligence::canonicalization::ItemKind::_Reserved,
                    &id_seed,
                );
                let claim_id = format!(
                    "m9-{}-{}-{}",
                    subject_id,
                    sanitize_id_segment(&item.field),
                    &id_hash[..16.min(id_hash.len())],
                );

                // L2 cycle-7 fix #1: idempotency check by claim_id, not
                // dedup_key. The cutover orchestration runs
                // JSON-blob backfill BEFORE rekey (cycle-5 reorder),
                // so on a partial-cutover retry path m9 rows may
                // already exist with their REKEYED dedup_keys. The
                // claim_id is deterministic from
                // (subject_id, field, hash(content)) so a re-run
                // computes the same id; checking by id catches the
                // already-present row regardless of whether dedup_key
                // has been rewritten yet. Also use INSERT OR IGNORE
                // as a belt-and-suspenders for the same hazard.
                let existing: i64 = db
                    .conn_ref()
                    .query_row(
                        "SELECT count(*) FROM intelligence_claims WHERE id = ?1",
                        params![&claim_id],
                        |row| row.get(0),
                    )
                    .map_err(|e| format!("idempotency check failed: {e}"))?;
                if existing > 0 {
                    continue;
                }

                // L2 cycle-8 fix #2: capture the actual affected-row
                // count from INSERT OR IGNORE so claims_inserted
                // reflects reality. The pre-check above guards the
                // common-path race; OR IGNORE catches a concurrent
                // duplicate snuck in between the check and the
                // execute. Either way, the report tracks 0 vs 1
                // honestly so a partial-cutover retry doesn't make
                // the cutover report claim phantom inserts.
                // dos7-allowed: JSON-blob cutover backfill writes legacy
                // dismissal rows before runtime registry validation can
                // derive typed proposals from archived intelligence files.
                let inserted = db
                    .conn_ref()
                    .execute(
                        "INSERT OR IGNORE INTO intelligence_claims ( \
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

                if inserted == 1 {
                    report.claims_inserted += 1;
                }
            }
        }
    }

    Ok(report)
}

fn sanitize_id_segment(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Claims cutover orchestration hook
// ---------------------------------------------------------------------------

use std::time::Duration;

/// Aggregated report of the full claims cutover orchestration.
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
    pub source_asof_backfill_summary:
        Option<crate::services::source_asof_backfill::BackfillSummary>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

/// Post-migration reconcile pass.
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
            Err(e) if is_missing_legacy_table_error(&e) => None,
            Err(e) => {
                return Err(format!(
                    "DOS-7 reconcile: legacy count for mechanism {mechanism} ({label}) failed: {e}"
                ));
            }
        };

        let claims_present: i64 = conn
            .query_row(
                "SELECT count(*) FROM intelligence_claims WHERE id LIKE ?1",
                [claim_id_prefix],
                |r| r.get(0),
            )
            .map_err(|e| {
                format!(
                    "DOS-7 reconcile: claim count for mechanism {mechanism} ({label}) failed: {e}"
                )
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

fn is_missing_legacy_table_error(error: &rusqlite::Error) -> bool {
    let message = match error {
        rusqlite::Error::SqliteFailure(sqlite_error, Some(message))
            if sqlite_error.code == rusqlite::ErrorCode::Unknown =>
        {
            message
        }
        rusqlite::Error::SqlInputError {
            error,
            msg,
            sql: _,
            offset: _,
        } if error.code == rusqlite::ErrorCode::Unknown => msg,
        _ => return false,
    };
    // rusqlite 0.31 does not expose the newer missing-table extended code.
    // Keep the SQLite wording fallback constrained to this schema-probe helper.
    message
        .trim()
        .to_ascii_lowercase()
        .starts_with("no such table:")
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

/// Run the full claims cutover sequence atomically. Returns Err if any
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

    // Step 4: run pending migrations. Migrations 131+132 (claims SQL
    // backfills) plus any newer registered versions are applied here.
    let applied = crate::migrations::run_migrations(db.conn_ref())
        .map_err(|e| format!("DOS-7 cutover: migrations failed: {e}"))?;
    report.sql_migrations_applied = applied;
    log::info!("[DOS-7 cutover] migrations applied: {}", applied);

    // Step 5: JSON-blob backfill (mechanism 9). Run BEFORE rekey so the
    // m9- rows are present when we canonicalize hashes — L2 cycle-5
    // fix #1 caught that the original Step 4.5 ordering only rekeyed
    // m1-m8, leaving m9 with empty item_hash + raw text.
    let json_report = backfill_dismissed_items_from_workspace(ctx, db, workspace_root)?;
    report.json_blob_report = json_report.clone();
    log::info!(
        "[DOS-7 cutover] JSON-blob backfill: {} entities scanned, {} items observed, {} claims inserted",
        json_report.entities_scanned,
        json_report.items_observed,
        json_report.claims_inserted,
    );

    // Step 5.5: rekey ALL backfilled claims (m1-m9) to the runtime
    // canonical runtime shape.
    //
    // L2 cycle-2 fix #2: per-row failures are fatal to cutover. The
    // previous "warn-and-continue" behavior left rows under their
    // old hash while marking the cutover complete — making the
    // failure non-retriable on subsequent startups (the
    // dos7_cutover_completed_at marker short-circuits the hook).
    //
    // L2 cycle-5 fix #1: rekey now scans `m[1-9]-*` (was `m[1-8]-*`),
    // so m9 rows from the JSON-blob backfill above also get
    // runtime-canonical hashes. Without this, PRE-GATE text tier
    // would miss whitespace-anomalous m9 rows since hash tier is
    // skipped when item_hash is empty.
    let rekey_report = match rekey_backfilled_claims_via_runtime_helpers(ctx, db) {
        Ok(report) => report,
        Err(e) => {
            let mut report = RekeyReport::default();
            report.errors.push(e);
            report
        }
    };
    log::info!(
        "[DOS-7 cutover] m1-m9 rekey: {} rows examined, {} rows rewritten, {} error(s)",
        rekey_report.rows_examined,
        rekey_report.rows_rewritten,
        rekey_report.errors.len(),
    );
    if !rekey_report.errors.is_empty() {
        return Err(format!(
            "DOS-7 cutover: m1-m9 rekey produced {} error(s); refusing to mark cutover complete until they are resolved. First error: {}",
            rekey_report.errors.len(),
            rekey_report.errors.first().map(String::as_str).unwrap_or("(none)"),
        ));
    }
    report.rekey_report = rekey_report;

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

    let source_asof_summary =
        match crate::services::source_asof_backfill::backfill_source_asof_for_legacy_claims(
            ctx,
            db,
            workspace_root,
            ctx.clock.now(),
        ) {
            Ok(summary) => summary,
            Err(crate::services::source_asof_backfill::BackfillError::MigrationGate(message)) => {
                return Err(format!(
                    "DOS-7 cutover: source_asof backfill gate: {message}"
                ));
            }
            Err(crate::services::source_asof_backfill::BackfillError::Rusqlite(error)) => {
                return Err(format!(
                    "DOS-7 cutover: source_asof backfill database error: {error}"
                ));
            }
            Err(crate::services::source_asof_backfill::BackfillError::Mode(message)) => {
                return Err(format!(
                    "DOS-7 cutover: source_asof backfill mode error: {message}"
                ));
            }
        };
    log::info!(
        "[DOS-7 cutover] source_asof backfill summary: {:?}",
        source_asof_summary
    );
    report.source_asof_backfill_summary = Some(source_asof_summary);

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

/// Migration-state key holding the unix timestamp at which a cutover
/// run claimed exclusive responsibility. Used as a process-safe
/// compare-and-set marker (L2 cycle-2 fix #3): two processes starting
/// simultaneously cannot both claim it; the loser sees an existing
/// row and either waits or skips depending on whether the
/// completed-at marker is also set.
const DOS7_CUTOVER_STARTED_AT_KEY: &str = "dos7_cutover_started_at";

/// How long to consider a `started_at` marker as "in flight" before
/// treating it as stale (e.g. crashed mid-cutover) and reclaiming.
/// 30 minutes covers the worst-case JSON-blob workspace size.
const DOS7_CUTOVER_STALE_AFTER_SECS: i64 = 30 * 60;

/// Idempotently run the claims cutover (rekey + JSON-blob
/// backfill + reconcile) on startup.
///
/// The cutover MUST run after migrations 130/131 apply, otherwise:
///   - JSON-blob (mechanism 9) backfill never runs: legacy dismissed
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
    // Only run when the claims schema has been applied (migration 130 → SQL
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

    // L2 cycle-2 fix #3: process-safe claim/lock. Atomically check
    // that no other process has completed OR is mid-cutover, and
    // claim the work via INSERT OR IGNORE on a started marker. The
    // BEGIN IMMEDIATE upgrades the connection to a write lock so two
    // concurrent processes serialize at SQLite's lock layer.
    let now_ts = chrono::Utc::now().timestamp();
    let conn = db.conn_ref();
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| format!("DOS-7 cutover claim BEGIN: {e}"))?;

    let claim_decision = (|| -> Result<CutoverClaimDecision, String> {
        // Already completed by some prior process? Skip.
        let completed: Option<i64> = conn
            .query_row(
                "SELECT value FROM migration_state WHERE key = ?1",
                [DOS7_CUTOVER_COMPLETED_AT_KEY],
                |row| row.get(0),
            )
            .ok();
        if completed.is_some() {
            return Ok(CutoverClaimDecision::AlreadyComplete);
        }

        // In flight by another process? Wait or reclaim if stale.
        let in_flight: Option<i64> = conn
            .query_row(
                "SELECT value FROM migration_state WHERE key = ?1",
                [DOS7_CUTOVER_STARTED_AT_KEY],
                |row| row.get(0),
            )
            .ok();
        if let Some(started_ts) = in_flight {
            if now_ts - started_ts < DOS7_CUTOVER_STALE_AFTER_SECS {
                return Ok(CutoverClaimDecision::InFlightElsewhere { started_ts });
            }
            // Stale marker — reclaim by overwriting.
            conn.execute(
                "UPDATE migration_state SET value = ?2 WHERE key = ?1",
                rusqlite::params![DOS7_CUTOVER_STARTED_AT_KEY, now_ts],
            )
            .map_err(|e| format!("DOS-7 cutover reclaim stale marker: {e}"))?;
            return Ok(CutoverClaimDecision::Claimed);
        }

        // No completed, no in-flight → claim it.
        conn.execute(
            "INSERT INTO migration_state (key, value) VALUES (?1, ?2)",
            rusqlite::params![DOS7_CUTOVER_STARTED_AT_KEY, now_ts],
        )
        .map_err(|e| format!("DOS-7 cutover claim insert: {e}"))?;
        Ok(CutoverClaimDecision::Claimed)
    })();

    // Always end the claim transaction. We commit on success even
    // when the decision is AlreadyComplete / InFlight so the read
    // is durable; we rollback on error so partial state doesn't
    // persist.
    let claim_decision = match claim_decision {
        Ok(decision) => {
            conn.execute_batch("COMMIT")
                .map_err(|e| format!("DOS-7 cutover claim COMMIT: {e}"))?;
            decision
        }
        Err(e) => {
            // best-effort: preserve the original cutover error if rollback itself fails.
            let _ = conn.execute_batch("ROLLBACK");
            return Err(e);
        }
    };

    match claim_decision {
        CutoverClaimDecision::AlreadyComplete => {
            log::debug!(
                "[DOS-7 cutover] already completed by a prior process; skipping startup hook"
            );
            return Ok(None);
        }
        CutoverClaimDecision::InFlightElsewhere { started_ts } => {
            log::info!(
                "[DOS-7 cutover] in flight elsewhere since unix={}; this process defers (stale-after={}s)",
                started_ts, DOS7_CUTOVER_STALE_AFTER_SECS
            );
            return Ok(None);
        }
        CutoverClaimDecision::Claimed => {
            log::info!("[DOS-7 cutover] claimed by this process; running now");
        }
    }

    // Run the cutover. If it fails, leave the started marker so a
    // subsequent retry (after the stale-after window OR an operator
    // clearing the marker) picks up where this attempt left off.
    let report = run_dos7_cutover(ctx, db, workspace_root)?;

    // Record completion. INSERT OR REPLACE so a stale-claim reclaim
    // followed by a successful run still persists the marker.
    db.conn_ref()
        .execute(
            "INSERT OR REPLACE INTO migration_state (key, value) VALUES (?1, ?2)",
            rusqlite::params![
                DOS7_CUTOVER_COMPLETED_AT_KEY,
                chrono::Utc::now().timestamp()
            ],
        )
        .map_err(|e| format!("DOS-7 cutover record completion: {e}"))?;

    // Clear the started marker so the migration_state reflects only
    // the completed timestamp going forward.
    // best-effort: completed_at is authoritative; stale started_at is ignored after completion.
    let _ = db.conn_ref().execute(
        "DELETE FROM migration_state WHERE key = ?1",
        [DOS7_CUTOVER_STARTED_AT_KEY],
    );

    Ok(Some(report))
}

enum CutoverClaimDecision {
    AlreadyComplete,
    InFlightElsewhere { started_ts: i64 },
    Claimed,
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
        // L2 cycle-16: mirror runtime_identity_for_rekey's
        // canonical-subject path so test expectations track the
        // production canonical form (alphabetical keys, lowercase
        // kind), not whatever shape the test seed happened to use.
        let subject_value = serde_json::from_str::<serde_json::Value>(subject_ref).unwrap();
        let subject = crate::services::claims::subject_ref_from_json(&subject_value).unwrap();
        let compact_subject_ref = crate::services::claims::canonical_subject_ref(&subject).unwrap();
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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
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

    /// L2 cycle-4 fix #1: backfilled m3 email rows store the legacy
    /// `email_dismissals.item_text` verbatim (with whatever whitespace
    /// anomalies the user typed). Runtime canonicalize_for_dos280
    /// trims + collapses whitespace + lowercases. The rekey pass
    /// rewrites `item_hash` to the runtime-canonical hash so PRE-GATE
    /// hash tier matches a runtime commit_claim with the canonical
    /// version of the same item — even when the legacy text column
    /// retains its weird whitespace (text is in the immutability
    /// allowlist).
    ///
    /// This test seeds a backfilled m3 row with `text = "  Reply
    /// by\tFriday  "`, runs the rekey, then asserts the post-rekey
    /// item_hash matches what runtime would compute for the
    /// canonical `"reply by friday"`.
    #[test]
    fn rekey_canonicalizes_whitespace_anomaly_legacy_text_for_pre_gate_hash_tier() {
        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        // Legacy m3 backfill writes raw item_text into both `text` and
        // `item_hash` (item_hash = coalesce(item_text, '') per the
        // migration). The whitespace + case anomaly survives the
        // INSERT.
        let legacy_text = "  Reply by\tFriday  ";
        let legacy_dedup = format!("{legacy_text}:em-1:email_dismissed:commitment");
        seed_rekey_claim(
            &db,
            RekeySeed {
                id: "m3-1",
                subject_ref: r#"{"kind":"Email","id":"em-1"}"#,
                claim_type: "email_dismissed",
                field_path: Some("commitment"),
                text: legacy_text,
                dedup_key: &legacy_dedup,
                item_hash: Some(legacy_text),
            },
        );

        let report = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
        assert_eq!(report.rows_examined, 1);
        assert_eq!(report.rows_rewritten, 1, "rekey must rewrite the row");
        assert!(report.errors.is_empty(), "{:?}", report.errors);

        // Post-rekey item_hash must match what runtime would compute
        // for the canonical text — proving PRE-GATE hash tier will
        // catch a runtime commit_claim with `text = "Reply by Friday"`
        // (or any other capitalization / whitespace variant).
        let (_expected_dedup, expected_runtime_hash) = expected_runtime_identity(
            r#"{"kind":"Email","id":"em-1"}"#,
            "email_dismissed",
            Some("commitment"),
            "Reply by Friday",
        );
        let stored_hash: String = db
            .conn_ref()
            .query_row(
                "SELECT item_hash FROM intelligence_claims WHERE id = 'm3-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            stored_hash, expected_runtime_hash,
            "post-rekey item_hash must equal hash(canonicalize(legacy_text)) so PRE-GATE hash tier matches"
        );
    }

    /// L2 cycle-5 fix #1: rekey now scans `m[1-9]-*`, so m9
    /// JSON-blob backfill rows also get runtime-canonical hashes.
    /// Without this, m9 rows would retain empty item_hash + raw text
    /// and PRE-GATE text tier (NOCASE only) would miss
    /// whitespace-anomalous re-surfacings.
    #[test]
    fn rekey_includes_m9_json_blob_backfill_rows() {
        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let legacy_text = "  Reply by\tFriday  ";
        let legacy_dedup = format!("{legacy_text}:acct-1:risks:dismissed_item");
        seed_rekey_claim(
            &db,
            RekeySeed {
                id: "m9-acct1-risks",
                subject_ref: r#"{"kind":"Account","id":"acct-1"}"#,
                claim_type: "dismissed_item",
                field_path: Some("risks"),
                text: legacy_text,
                dedup_key: &legacy_dedup,
                item_hash: Some(""),
            },
        );

        let report = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
        assert_eq!(report.rows_examined, 1, "m9 row must be in the rekey scan");
        assert_eq!(report.rows_rewritten, 1, "m9 row must get rewritten");

        let (_expected_dedup, expected_runtime_hash) = expected_runtime_identity(
            r#"{"kind":"Account","id":"acct-1"}"#,
            "dismissed_item",
            Some("risks"),
            "Reply by Friday",
        );
        let stored_hash: String = db
            .conn_ref()
            .query_row(
                "SELECT item_hash FROM intelligence_claims WHERE id = 'm9-acct1-risks'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            stored_hash, expected_runtime_hash,
            "post-rekey m9 item_hash must equal hash(canonicalize(legacy_text))"
        );
    }

    #[test]
    fn rekey_skips_non_backfilled_claims() {
        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report = backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path()).unwrap();
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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report = backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path()).unwrap();

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

    /// L2 cycle-6 fix #2: multiple dismissedItems in the SAME field
    /// must not cause an m9 PK collision. The original `m9-{subject}-
    /// {field}` shape collided on the second item, aborting the
    /// JSON-blob backfill (and the entire cutover). Cycle-6 fix
    /// includes a content-hash suffix so each item gets a unique
    /// claim_id.
    #[test]
    fn multiple_dismissed_items_in_same_field_get_unique_m9_claim_ids() {
        let workspace = tempfile::tempdir().unwrap();
        // Two dismissedItems in the SAME field 'risks'.
        let body = serde_json::json!({
            "version": 4,
            "entityId": "acct-multi",
            "entityType": "account",
            "enrichedAt": "2026-04-01T00:00:00Z",
            "sourceFileCount": 0,
            "dismissedItems": [
                {
                    "field": "risks",
                    "content": "First dismissed risk",
                    "dismissedAt": "2026-04-15T00:00:00Z"
                },
                {
                    "field": "risks",
                    "content": "Second dismissed risk",
                    "dismissedAt": "2026-04-16T00:00:00Z"
                },
                {
                    "field": "risks",
                    "content": "Third dismissed risk",
                    "dismissedAt": "2026-04-17T00:00:00Z"
                }
            ]
        });
        write_intel_json(
            workspace.path(),
            "Accounts",
            "acct-multi",
            &serde_json::to_string_pretty(&body).unwrap(),
        );

        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report = backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path())
            .expect("3 items in same field must not PK-collide");
        assert_eq!(report.items_observed, 3);
        assert_eq!(
            report.claims_inserted, 3,
            "all 3 same-field dismissed items must persist"
        );

        // Each row got a unique claim_id.
        let distinct_ids: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(DISTINCT id) FROM intelligence_claims \
                 WHERE id LIKE 'm9-acct-multi-%' AND field_path = 'risks'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(distinct_ids, 3);
    }

    /// L2 cycle-10 fix #1: rekey must surface a malformed
    /// `metadata_json` (or `provenance_json`) on a pre-cycle-9 m9
    /// row that has a valid `subject_ref`. The previous rekey only
    /// validated subject_ref so structurally-broken legacy rows
    /// could pass cutover.
    /// L2 cycle-11 fix #1: a present-but-blank metadata_json
    /// (empty string OR whitespace) must fail rekey validation.
    /// The previous "trim().is_empty() → skip" logic let blank
    /// strings slip past, so a malformed-write-then-blank-out
    /// path could leave structurally-broken claims that pass
    /// cutover.
    #[test]
    fn rekey_fails_on_blank_string_metadata_json() {
        for blank in ["", "   ", "\t\n"] {
            let conn = fresh_conn();
            let db = ActionDb::from_conn(&conn);
            let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
            let rng = SeedableRng::new(42);
            let ext = ExternalClients::default();
            let ctx = fixture_ctx(&clock, &rng, &ext);

            // dos7-allowed: cycle-11 regression seed
            db.conn_ref()
                .execute(
                    "INSERT INTO intelligence_claims \
                     (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
                      actor, data_source, observed_at, created_at, provenance_json, metadata_json, \
                      claim_state, surfacing_state, retraction_reason, expires_at, \
                      temporal_scope, sensitivity) \
                     VALUES \
                     ('m1-blank-meta', \
                      '{\"kind\":\"Account\",\"id\":\"acct-1\"}', 'risk', 'risks', \
                      'whatever', 'k', 'h', 'system_backfill', 'legacy_dismissal', \
                      '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', \
                      '{}', ?1, \
                      'tombstoned', 'active', 'user_removal', NULL, \
                      'state', 'internal')",
                    rusqlite::params![blank],
                )
                .unwrap();

            let report = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
            assert!(
                report
                    .errors
                    .iter()
                    .any(|e| e.contains("metadata_json is not valid JSON")),
                "blank metadata_json {:?} must fail validation, got {:?}",
                blank,
                report.errors,
            );
        }
    }

    #[test]
    fn rekey_fails_on_malformed_metadata_json_with_valid_subject_ref() {
        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        // Seed a pre-cycle-9-style m9 row: valid subject_ref but
        // malformed metadata_json (raw format!-built with an
        // unescaped quote).
        // dos7-allowed: cycle-10 regression-test seed
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims \
                 (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
                  actor, data_source, observed_at, created_at, provenance_json, metadata_json, \
                  claim_state, surfacing_state, retraction_reason, expires_at, \
                  temporal_scope, sensitivity) \
                 VALUES \
                 ('m9-acct-pre9-risks-aaaaaaaaaaaaaaaa', \
                  '{\"kind\":\"Account\",\"id\":\"acct-pre9\"}', \
                  'dismissed_item', 'risks', 'whatever', 'k', '', \
                  'system_backfill', 'legacy_dismissal', \
                  '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', \
                  '{\"backfill_mechanism\":\"dismissed_item_json\"}', \
                  '{\"field\":\"risks\",\"content\":\"unescaped \"quote\" here\"}', \
                  'tombstoned', 'active', 'user_removal', NULL, \
                  'state', 'internal')",
                [],
            )
            .unwrap();

        let report = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
        assert_eq!(report.rows_examined, 1);
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("metadata_json is not valid JSON")),
            "rekey must surface malformed metadata_json error, got {:?}",
            report.errors,
        );
    }

    /// L2 cycle-9 fix: legacy intelligence.json values containing
    /// quotes, backslashes, newlines, and control characters must
    /// produce well-formed JSON in the m9 backfill output. Cycle-9
    /// switched from raw `format!` interpolation to serde_json,
    /// closing the JSON-injection hazard that would otherwise
    /// poison the rekey pass and dead-loop the cutover.
    #[test]
    fn m9_backfill_round_trips_malformed_legacy_characters() {
        let workspace = tempfile::tempdir().unwrap();
        // Mix of quotes, backslashes, newlines, and a control char.
        let evil_content = "She said \"hi\".\nWith\\backslash\tand\u{0007}bell.";
        let evil_field = "risks-with-\"quotes\"";
        let body = serde_json::json!({
            "version": 4,
            "entityId": "acct-evil",
            "entityType": "account",
            "enrichedAt": "2026-04-01T00:00:00Z",
            "sourceFileCount": 0,
            "dismissedItems": [{
                "field": evil_field,
                "content": evil_content,
                "dismissedAt": "2026-04-15T00:00:00Z"
            }]
        });
        write_intel_json(
            workspace.path(),
            "Accounts",
            "acct-evil",
            &serde_json::to_string_pretty(&body).unwrap(),
        );

        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report = backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path())
            .expect("malformed-character item must persist");
        assert_eq!(report.claims_inserted, 1);

        // Verify ALL JSON columns are valid JSON SQLite can parse.
        let (subject_ref, provenance_json, metadata_json): (String, String, String) = db
            .conn_ref()
            .query_row(
                "SELECT subject_ref, provenance_json, metadata_json \
                 FROM intelligence_claims \
                 WHERE id LIKE 'm9-acct-evil-%'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        for (label, raw) in [
            ("subject_ref", &subject_ref),
            ("provenance_json", &provenance_json),
            ("metadata_json", &metadata_json),
        ] {
            serde_json::from_str::<serde_json::Value>(raw)
                .unwrap_or_else(|e| panic!("{label} must be valid JSON, got {raw:?}: {e}"));
        }

        // Verify the rekey pass — which parses subject_ref and would
        // dead-loop on malformed JSON — succeeds.
        let rekey = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
        assert!(
            rekey.errors.is_empty(),
            "rekey must not error on serde_json-built m9 rows: {:?}",
            rekey.errors,
        );
        assert_eq!(rekey.rows_examined, 1);
        assert_eq!(rekey.rows_rewritten, 1);
    }

    /// L2 cycle-8 fix #1: m9 claim_id must namespace by subject_kind
    /// so an Account "x-1" and a Person "x-1" with the same dismissed
    /// field+content don't collide on PK. Without subject_kind in the
    /// hash, the second backfilled tombstone would silently lose.
    #[test]
    fn m9_claim_id_namespaced_by_subject_kind() {
        let workspace = tempfile::tempdir().unwrap();
        // Account "shared-id" + Person "shared-id" with identical
        // field + content. Both should get distinct m9 claim_ids.
        let acct_body = serde_json::json!({
            "version": 4,
            "entityId": "shared-id",
            "entityType": "account",
            "enrichedAt": "2026-04-01T00:00:00Z",
            "sourceFileCount": 0,
            "dismissedItems": [{
                "field": "risks",
                "content": "Same content",
                "dismissedAt": "2026-04-15T00:00:00Z"
            }]
        });
        let person_body = serde_json::json!({
            "version": 4,
            "entityId": "shared-id",
            "entityType": "person",
            "enrichedAt": "2026-04-01T00:00:00Z",
            "sourceFileCount": 0,
            "dismissedItems": [{
                "field": "risks",
                "content": "Same content",
                "dismissedAt": "2026-04-15T00:00:00Z"
            }]
        });
        write_intel_json(
            workspace.path(),
            "Accounts",
            "shared-id",
            &serde_json::to_string_pretty(&acct_body).unwrap(),
        );
        write_intel_json(
            workspace.path(),
            "People",
            "shared-id",
            &serde_json::to_string_pretty(&person_body).unwrap(),
        );

        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report = backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path())
            .expect("Account + Person sharing id+field+content must not PK-collide");
        assert_eq!(report.items_observed, 2);
        assert_eq!(
            report.claims_inserted, 2,
            "both Account and Person tombstones must persist"
        );

        // Verify exactly 2 distinct m9 rows for shared-id, one per kind.
        let kinds: Vec<String> = {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT json_extract(subject_ref, '$.kind') \
                     FROM intelligence_claims \
                     WHERE id LIKE 'm9-shared-id-%' \
                     ORDER BY id",
                )
                .unwrap();
            stmt.query_map([], |row| row.get::<_, String>(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };
        assert_eq!(kinds.len(), 2, "two distinct m9 rows expected");
        assert!(
            kinds.contains(&"Account".to_string()) && kinds.contains(&"Person".to_string()),
            "both Account and Person kinds must be present, got {kinds:?}"
        );
    }

    /// L2 cycle-7 fix #1: simulate a partial cutover where m9 rows
    /// were inserted AND rekeyed (so their dedup_key is the runtime
    /// shape, not the original `content:subject:field:dismissed_item`
    /// shape used by the JSON-blob backfill), then cutover failed
    /// before writing the completion marker. The next retry must NOT
    /// abort the JSON-blob backfill on a PK collision — the
    /// idempotency check now uses claim_id (deterministic via
    /// content-hash suffix), and the INSERT uses OR IGNORE.
    #[test]
    fn m9_backfill_rerun_after_rekey_does_not_pk_collide() {
        let workspace = tempfile::tempdir().unwrap();
        let body = serde_json::json!({
            "version": 4,
            "entityId": "acct-retry",
            "entityType": "account",
            "enrichedAt": "2026-04-01T00:00:00Z",
            "sourceFileCount": 0,
            "dismissedItems": [
                {
                    "field": "risks",
                    "content": "Retry-after-rekey item",
                    "dismissedAt": "2026-04-15T00:00:00Z"
                }
            ]
        });
        write_intel_json(
            workspace.path(),
            "Accounts",
            "acct-retry",
            &serde_json::to_string_pretty(&body).unwrap(),
        );

        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        // First pass: m9 row inserts with backfill-shape dedup_key.
        let first = backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path()).unwrap();
        assert_eq!(first.claims_inserted, 1);

        // Simulate rekey rewriting the dedup_key + item_hash to the
        // runtime canonical shape. The claim_id stays the same; only
        // the lifecycle/identity columns change (allowed by the
        // immutability lint).
        db.conn_ref()
            .execute(
                "UPDATE intelligence_claims \
                 SET dedup_key = 'rekeyed-shape:acct-retry:dismissed_item:risks', \
                     item_hash = 'runtime-canonical-hash' \
                 WHERE id LIKE 'm9-acct-retry-%'",
                [],
            )
            .unwrap();

        // Second pass (the partial-cutover retry path): must NOT
        // abort. The original idempotency-by-dedup_key gate would
        // have missed this row and PK-collided on INSERT.
        let second = backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path())
            .expect("retry after rekey must not PK-collide");
        assert_eq!(second.claims_inserted, 0, "row should already exist");
        assert_eq!(second.items_observed, 1);

        // Still exactly one m9 claim row.
        let row_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM intelligence_claims WHERE id LIKE 'm9-acct-retry-%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(row_count, 1);
    }

    /// L2 cycle-18 fix: rekey must canonicalize the stored
    /// subject_ref column (not just dedup_key + item_hash) so
    /// runtime readers that match via `json_extract($.kind)` and
    /// `'$.id'` — PRE-GATE, contradiction detection, and
    /// load_claims_where — can find alias-shaped or PascalCase
    /// historical rows. Otherwise rekey produces correct
    /// dedup_keys but the rows stay invisible to those readers.
    #[test]
    fn rekey_canonicalizes_subject_ref_so_readers_can_find_alias_rows() {
        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        // Alias keys (`type`, `entity_id`) instead of canonical
        // (`kind`, `id`). PascalCase value too.
        let alias_subject = r#"{"type":"Account","entity_id":"acct-1"}"#;
        seed_rekey_claim(
            &db,
            RekeySeed {
                id: "m1-alias-canon",
                subject_ref: alias_subject,
                claim_type: "risk",
                field_path: Some("risks"),
                text: "Alias row to be canonicalized",
                dedup_key: "legacy-shape",
                item_hash: Some("legacy-hash"),
            },
        );

        let report = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
        assert_eq!(report.rows_examined, 1);
        assert_eq!(report.rows_rewritten, 1);

        // Stored subject_ref must now be the canonical
        // alphabetical-key, lowercase-kind form. A reader that
        // queries via json_extract on $.kind/$.id will find it.
        let stored: String = db
            .conn_ref()
            .query_row(
                "SELECT subject_ref FROM intelligence_claims WHERE id = 'm1-alias-canon'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            stored, r#"{"id":"acct-1","kind":"account"}"#,
            "rekey must rewrite subject_ref to the canonical shape"
        );

        // And the canonical $.kind / $.id are now extractable.
        let kind: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT json_extract(subject_ref, '$.kind') FROM intelligence_claims \
                 WHERE id = 'm1-alias-canon'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kind, Some("account".to_string()));
    }

    /// L2 cycle-17 fix: rekey must accept the same alias-shaped
    /// subject_ref keys (`type`/`entity_type`, `entity_id`) that
    /// `subject_ref_from_json` parses for runtime commits. The
    /// previous hand-rolled `$.kind`/`$.id` precheck was stricter
    /// than the parser, so an alias-shaped historical row would
    /// fail rekey and abort cutover (rekey errors are fatal per
    /// cycle-2 fix #2).
    #[test]
    fn rekey_accepts_alias_shaped_subject_keys() {
        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        // Alias-shaped subject_ref: `type` instead of `kind`,
        // `entity_id` instead of `id`. Parser accepts both.
        let alias_subject = r#"{"type":"account","entity_id":"acct-1"}"#;
        seed_rekey_claim(
            &db,
            RekeySeed {
                id: "m1-alias",
                subject_ref: alias_subject,
                claim_type: "risk",
                field_path: Some("risks"),
                text: "Alias-shaped subject text",
                dedup_key: "legacy-shape",
                item_hash: Some("legacy-hash"),
            },
        );

        let report = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
        assert_eq!(report.rows_examined, 1);
        assert_eq!(report.rows_rewritten, 1);
        assert!(report.errors.is_empty(), "{:?}", report.errors);

        // The rekey-produced dedup_key must equal what runtime
        // commit_claim would compute for the canonical-shape input,
        // proving alias parity.
        let stored_dedup: String = db
            .conn_ref()
            .query_row(
                "SELECT dedup_key FROM intelligence_claims WHERE id = 'm1-alias'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let canonical_subject = r#"{"kind":"account","id":"acct-1"}"#;
        let (runtime_dedup, _) = expected_runtime_identity(
            canonical_subject,
            "risk",
            Some("risks"),
            "Alias-shaped subject text",
        );
        assert_eq!(
            stored_dedup, runtime_dedup,
            "rekey of alias-shape row must produce same dedup_key as canonical-shape commit"
        );
    }

    /// L2 cycle-16 fix: rekey + commit_claim must produce
    /// byte-identical dedup_keys for the same semantic subject,
    /// regardless of which path's input shape the row started
    /// with. The previous rekey serialized the row's raw
    /// subject_ref bytes, so a backfill row with PascalCase kind
    /// would carry a PascalCase-shaped dedup_key while runtime
    /// commits produce lowercase-shaped — fracturing same-meaning
    /// merge across the cutover boundary.
    #[test]
    fn rekey_dedup_key_matches_runtime_commit_for_same_subject() {
        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        // Seed a backfill row with PascalCase kind (the SQLite
        // json_object shape).
        let pascal_subject = r#"{"kind":"Account","id":"acct-1"}"#;
        seed_rekey_claim(
            &db,
            RekeySeed {
                id: "m1-pascal",
                subject_ref: pascal_subject,
                claim_type: "risk",
                field_path: Some("risks"),
                text: "Same risk text",
                dedup_key: "legacy-shape",
                item_hash: Some("legacy-hash"),
            },
        );

        let report = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
        assert_eq!(report.rows_examined, 1);
        assert_eq!(report.rows_rewritten, 1);
        assert!(report.errors.is_empty(), "{:?}", report.errors);

        // Read back the rekeyed row's dedup_key.
        let stored_dedup: String = db
            .conn_ref()
            .query_row(
                "SELECT dedup_key FROM intelligence_claims WHERE id = 'm1-pascal'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Compute what runtime commit_claim would produce for the
        // SAME semantic subject given lowercase input.
        let lowercase_subject = r#"{"kind":"account","id":"acct-1"}"#;
        let (runtime_dedup, _) =
            expected_runtime_identity(lowercase_subject, "risk", Some("risks"), "Same risk text");

        assert_eq!(
            stored_dedup, runtime_dedup,
            "rekey-produced dedup_key must equal runtime-canonical dedup_key regardless of input casing"
        );
    }

    /// L2 cycle-13 fix #1: rekey must skip rows already in
    /// claim_state='withdrawn'. Migration 133 transitions
    /// unsupported-kind m5 rows to withdrawn; without this skip,
    /// rekey would still scan them, hit the unsupported-kind
    /// guard, and fail cutover — defeating cycle-12's fix.
    #[test]
    fn rekey_skips_withdrawn_rows() {
        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        // Seed an m5 row with unsupported kind already withdrawn
        // (mimicking post-migration-133 state).
        // dos7-allowed: cycle-13 regression seed
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims \
                 (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
                  actor, data_source, observed_at, created_at, provenance_json, \
                  claim_state, surfacing_state, retraction_reason, expires_at, \
                  temporal_scope, sensitivity) \
                 VALUES \
                 ('m5-withdrawn', '{\"kind\":\"email_thread\",\"id\":\"thr-1\"}', \
                  'linking_dismissed', 'account', 'acct-1', 'k', 'h', \
                  'system_backfill', 'legacy_dismissal', \
                  '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', '{}', \
                  'withdrawn', 'dormant', 'unsupported_subject_kind', NULL, \
                  'state', 'internal')",
                [],
            )
            .unwrap();

        let report = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
        assert_eq!(
            report.rows_examined, 0,
            "withdrawn row must be skipped by rekey scan"
        );
        assert!(
            report.errors.is_empty(),
            "rekey must not error: {:?}",
            report.errors
        );
    }

    /// L2 cycle-12 fix #2: rekey must reject m5 rows whose
    /// subject_ref kind is not a supported SubjectRef variant
    /// (e.g. owner_type='email_thread' from linking_dismissals).
    /// Migration 133 withdraws existing bad rows; this guard
    /// catches any new rows that surface through rekey.
    #[test]
    fn rekey_rejects_unsupported_subject_kind() {
        let conn = fresh_conn();
        let db = ActionDb::from_conn(&conn);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        // Seed an m5 row with kind="email_thread" (unsupported).
        seed_rekey_claim(
            &db,
            RekeySeed {
                id: "m5-email_thread-thr1-acct1-account",
                subject_ref: r#"{"kind":"email_thread","id":"thr-1"}"#,
                claim_type: "linking_dismissed",
                field_path: Some("account"),
                text: "acct-1",
                dedup_key: "acct-1:email_thread:thr-1:linking_dismissed:account",
                item_hash: Some("acct-1"),
            },
        );

        let report = rekey_backfilled_claims_via_runtime_helpers(&ctx, &db).unwrap();
        assert_eq!(report.rows_examined, 1);
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("not a supported SubjectRef variant")),
            "rekey must surface unsupported-kind error, got {:?}",
            report.errors,
        );
    }

    /// L2 cycle-12 fix #2: migration 133 withdraws m5 rows with
    /// unsupported subject_kind so they don't pollute PRE-GATE /
    /// suppression. After migration, the row's claim_state is
    /// 'withdrawn' and retraction_reason is 'unsupported_subject_kind'.
    /// L2 cycle-15 fix #2: migration 133 must withdraw unsupported
    /// kinds across m6/m7/m8 too, not just m5. Those mechanisms
    /// (briefing_callouts, nudge_dismissals, triage_snoozes) all
    /// capitalize raw legacy entity_type without guarding, so a
    /// legacy row with entity_type='global' or 'multi' produces a
    /// Global/Multi-shaped subject_ref bypassing the v1.4.0 spine
    /// restriction.
    #[test]
    fn migration_133_withdraws_m6_m7_m8_unsupported_kind_rows() {
        let conn = fresh_full_db();

        // Seed bad rows for each mechanism prefix.
        // dos7-allowed: cycle-15 regression seed
        for (id, kind) in [
            ("m6-bad", "Global"),
            ("m7-bad", "Multi"),
            ("m8-bad", "email_thread"),
            // Plus a supported one per mechanism that must NOT be withdrawn.
            ("m6-ok", "Account"),
            ("m7-ok", "Person"),
            ("m8-ok", "Meeting"),
        ] {
            conn.execute(
                "INSERT INTO intelligence_claims \
                 (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
                  actor, data_source, observed_at, created_at, provenance_json, \
                  claim_state, surfacing_state, retraction_reason, expires_at, \
                  temporal_scope, sensitivity) \
                 VALUES \
                 (?1, ?2, 'briefing_callout_dismissed', 'risks', 'irrelevant', \
                  'k', 'h', 'system_backfill', 'legacy_dismissal', \
                  '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', '{}', \
                  'tombstoned', 'active', 'user_removal', NULL, \
                  'state', 'internal')",
                rusqlite::params![id, format!(r#"{{"kind":"{}","id":"e-1"}}"#, kind),],
            )
            .unwrap();
        }

        // Re-run migration 133.
        let migration_sql =
            include_str!("../migrations/133_dos_7_withdraw_unsupported_m5_kinds.sql");
        conn.execute_batch(migration_sql).unwrap();

        for (id, expected_state) in [
            ("m6-bad", "withdrawn"),
            ("m7-bad", "withdrawn"),
            ("m8-bad", "withdrawn"),
            ("m6-ok", "tombstoned"),
            ("m7-ok", "tombstoned"),
            ("m8-ok", "tombstoned"),
        ] {
            let state: String = conn
                .query_row(
                    "SELECT claim_state FROM intelligence_claims WHERE id = ?1",
                    rusqlite::params![id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(
                state, expected_state,
                "{id} must end in {expected_state} after migration 133"
            );
        }
    }

    #[test]
    fn migration_133_withdraws_m5_unsupported_kind_rows() {
        // Use the full-migration DB so migration 133 has applied.
        let conn = fresh_full_db();

        // Seed an m5 row with kind="email_thread" (mimicking what
        // migration 131 wrote pre-fix).
        // dos7-allowed: cycle-12 regression seed
        conn.execute(
            "INSERT INTO intelligence_claims \
             (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
              actor, data_source, observed_at, created_at, provenance_json, \
              claim_state, surfacing_state, retraction_reason, expires_at, \
              temporal_scope, sensitivity) \
             VALUES \
             ('m5-evil', '{\"kind\":\"email_thread\",\"id\":\"thr-1\"}', \
              'linking_dismissed', 'account', 'acct-1', 'k', 'h', \
              'system_backfill', 'legacy_dismissal', \
              '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', '{}', \
              'tombstoned', 'active', 'user_removal', NULL, \
              'state', 'internal')",
            [],
        )
        .unwrap();

        // Re-run migration 133 (simulating the migration pass).
        let migration_sql =
            include_str!("../migrations/133_dos_7_withdraw_unsupported_m5_kinds.sql");
        conn.execute_batch(migration_sql).unwrap();

        let (state, reason): (String, String) = conn
            .query_row(
                "SELECT claim_state, retraction_reason FROM intelligence_claims \
                 WHERE id = 'm5-evil'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(state, "withdrawn");
        assert_eq!(reason, "unsupported_subject_kind");

        // A supported-kind m5 row should NOT be touched.
        conn.execute(
            "INSERT INTO intelligence_claims \
             (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
              actor, data_source, observed_at, created_at, provenance_json, \
              claim_state, surfacing_state, retraction_reason, expires_at, \
              temporal_scope, sensitivity) \
             VALUES \
             ('m5-ok', '{\"kind\":\"Meeting\",\"id\":\"mtg-1\"}', \
              'linking_dismissed', 'account', 'acct-1', 'k2', 'h2', \
              'system_backfill', 'legacy_dismissal', \
              '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', '{}', \
              'tombstoned', 'active', 'user_removal', NULL, \
              'state', 'internal')",
            [],
        )
        .unwrap();
        conn.execute_batch(migration_sql).unwrap();
        let ok_state: String = conn
            .query_row(
                "SELECT claim_state FROM intelligence_claims WHERE id = 'm5-ok'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            ok_state, "tombstoned",
            "supported-kind row must be untouched"
        );
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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let first = backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path()).unwrap();
        assert_eq!(first.claims_inserted, 1);

        let second = backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path()).unwrap();
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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report = backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path()).unwrap();
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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report = backfill_dismissed_items_from_workspace(&ctx, &db, workspace.path()).unwrap();
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
        assert!(
            report.findings >= 1,
            "expected at least one finding, got {}",
            report.findings
        );
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

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        // First call: runs the cutover.
        let first = run_dos7_cutover_if_pending(&ctx, db, workspace.path()).expect("first cutover");
        assert!(first.is_some(), "first call must run the cutover");

        // Second call: idempotent no-op. The migration_state guard short-circuits.
        let second =
            run_dos7_cutover_if_pending(&ctx, db, workspace.path()).expect("second cutover");
        assert!(
            second.is_none(),
            "second call must skip — already recorded in migration_state"
        );
    }

    /// L2 cycle-2 fix #3: a stale `dos7_cutover_started_at` marker
    /// (older than DOS7_CUTOVER_STALE_AFTER_SECS) must be reclaimable
    /// — otherwise a crashed cutover would leave the substrate
    /// permanently stuck.
    #[test]
    fn run_dos7_cutover_if_pending_reclaims_stale_started_marker() {
        let workspace = tempfile::tempdir().unwrap();
        let conn = fresh_full_db();
        let db = ActionDb::from_conn(&conn);

        // Plant a stale started-at marker (1 hour ago).
        let stale_ts = chrono::Utc::now().timestamp() - 60 * 60;
        conn.execute(
            "INSERT INTO migration_state (key, value) VALUES (?1, ?2)",
            rusqlite::params!["dos7_cutover_started_at", stale_ts],
        )
        .expect("plant stale marker");

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let result = run_dos7_cutover_if_pending(&ctx, db, workspace.path()).expect("reclaim");
        assert!(
            result.is_some(),
            "stale marker must be reclaimable — got no-op instead"
        );

        // Completed marker now set.
        let completed: i64 = conn
            .query_row(
                "SELECT value FROM migration_state WHERE key = 'dos7_cutover_completed_at'",
                [],
                |row| row.get(0),
            )
            .expect("completed marker present");
        assert!(completed > 0);
    }

    /// L2 cycle-2 fix #3: a fresh-but-not-yet-stale started_at marker
    /// from another process must cause the second process to defer
    /// (return None) instead of running concurrently.
    #[test]
    fn run_dos7_cutover_if_pending_defers_when_in_flight_elsewhere() {
        let workspace = tempfile::tempdir().unwrap();
        let conn = fresh_full_db();
        let db = ActionDb::from_conn(&conn);

        // Plant a fresh started-at marker (1 minute ago — clearly in
        // flight, not stale).
        let fresh_ts = chrono::Utc::now().timestamp() - 60;
        conn.execute(
            "INSERT INTO migration_state (key, value) VALUES (?1, ?2)",
            rusqlite::params!["dos7_cutover_started_at", fresh_ts],
        )
        .expect("plant in-flight marker");

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let result = run_dos7_cutover_if_pending(&ctx, db, workspace.path()).expect("defer");
        assert!(
            result.is_none(),
            "in-flight marker must cause this process to defer"
        );

        // No completed marker — the in-flight process is expected to
        // write it.
        let completed_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM migration_state WHERE key = 'dos7_cutover_completed_at'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(completed_count, 0);
    }

    /// L2 cycle-2 fix #2: any rekey error must fail the cutover BEFORE
    /// the completion marker is written, so a subsequent retry can
    /// pick up the work.
    #[test]
    fn run_dos7_cutover_fails_when_rekey_errors_present() {
        let workspace = tempfile::tempdir().unwrap();
        let conn = fresh_full_db();
        let db = ActionDb::from_conn(&conn);

        // Seed a malformed m1- claim row that the rekey pass cannot
        // resolve — empty subject_ref, no kind/id.
        conn.execute(
            "INSERT INTO intelligence_claims \
             (id, subject_ref, claim_type, field_path, text, dedup_key, item_hash, \
              actor, data_source, observed_at, created_at, provenance_json, \
              claim_state, surfacing_state, retraction_reason, \
              temporal_scope, sensitivity) \
             VALUES ('m1-malformed-1', '{}', 'risk', 'risks', 'r1', 'malformed-key', '', \
                     'system_backfill', 'legacy_dismissal', '2026-04-15T00:00:00Z', \
                     '2026-04-15T00:00:00Z', '{}', \
                     'tombstoned', 'active', 'user_removal', 'state', 'internal')",
            [],
        )
        .expect("seed malformed claim");

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let result = run_dos7_cutover(&ctx, db, workspace.path());
        assert!(
            result.is_err(),
            "rekey error must propagate as cutover Err, not silent warn"
        );

        // No completion marker got written.
        let completed_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM migration_state WHERE key = 'dos7_cutover_completed_at'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            completed_count, 0,
            "completion marker must NOT be set when rekey produced errors"
        );
    }

    /// L2 cycle-1 fix #3: when the claims schema has not been applied
    /// yet (legacy DB), the startup hook must be a no-op rather than
    /// erroring. Production startup runs migrations FIRST and the hook
    /// later, but the hook can be invoked on legacy DBs where migration
    /// 130 hasn't applied yet (e.g. fresh test fixtures).
    #[test]
    fn run_dos7_cutover_if_pending_no_op_when_claims_schema_absent() {
        let workspace = tempfile::tempdir().unwrap();
        // Bare DB without the claim tables.
        let conn = Connection::open_in_memory().unwrap();
        let db = ActionDb::from_conn(&conn);

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
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

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 2, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = fixture_ctx(&clock, &rng, &ext);

        let report = run_dos7_cutover(&ctx, db, workspace.path()).unwrap();

        assert_eq!(report.json_blob_report.entities_scanned, 2);
        assert_eq!(report.json_blob_report.items_observed, 3);
        assert_eq!(report.json_blob_report.claims_inserted, 3);
        assert_eq!(
            report.json_blob_report.items_by_kind.get("Account"),
            Some(&2)
        );
        assert_eq!(
            report.json_blob_report.items_by_kind.get("Person"),
            Some(&1)
        );
    }

    #[test]
    #[ignore = "TODO(D5): drain-timeout test seam — needs FenceCycle injection without depending on full state"]
    fn cutover_aborts_on_drain_timeout_before_running_backfill() {
        // Hold a FenceCycle for longer than the cutover's drain timeout
        // (30s) and assert the cutover returns Err with drain_timed_out=true
        // and no claims inserted.
    }
}

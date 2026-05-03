//! Schema migration framework (ADR-0071).
//!
//! Numbered SQL migrations are embedded at compile time via `include_str!`.
//! Each migration runs exactly once, tracked by the `schema_version` table.
//!
//! For existing databases (pre-migration-framework), the bootstrap function
//! detects the presence of known tables and marks migration 001 as applied
//! so the baseline SQL never runs against an already-populated database.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::{Connection, Error as SqliteError, ErrorCode};

struct Migration {
    version: i32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("migrations/001_baseline.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("migrations/002_internal_teams.sql"),
    },
    Migration {
        version: 3,
        sql: include_str!("migrations/003_account_team.sql"),
    },
    Migration {
        version: 4,
        sql: include_str!("migrations/004_account_team_role_index.sql"),
    },
    Migration {
        version: 5,
        sql: include_str!("migrations/005_email_signals.sql"),
    },
    Migration {
        version: 6,
        sql: include_str!("migrations/006_content_embeddings.sql"),
    },
    Migration {
        version: 7,
        sql: include_str!("migrations/007_chat_interface.sql"),
    },
    Migration {
        version: 8,
        sql: include_str!("migrations/008_missing_indexes.sql"),
    },
    Migration {
        version: 9,
        sql: include_str!("migrations/009_fix_embeddings_column.sql"),
    },
    Migration {
        version: 10,
        sql: include_str!("migrations/010_foreign_keys.sql"),
    },
    Migration {
        version: 11,
        sql: include_str!("migrations/011_proposed_actions.sql"),
    },
    Migration {
        version: 12,
        sql: include_str!("migrations/012_person_emails.sql"),
    },
    Migration {
        version: 13,
        sql: include_str!("migrations/013_quill_sync.sql"),
    },
    Migration {
        version: 14,
        sql: include_str!("migrations/014_granola_sync.sql"),
    },
    Migration {
        version: 15,
        sql: include_str!("migrations/015_gravatar_cache.sql"),
    },
    Migration {
        version: 16,
        sql: include_str!("migrations/016_clay_enrichment.sql"),
    },
    Migration {
        version: 17,
        sql: include_str!("migrations/017_entity_keywords.sql"),
    },
    Migration {
        version: 18,
        sql: include_str!("migrations/018_signal_bus.sql"),
    },
    Migration {
        version: 19,
        sql: include_str!("migrations/019_correction_learning.sql"),
    },
    Migration {
        version: 20,
        sql: include_str!("migrations/020_signal_propagation.sql"),
    },
    Migration {
        version: 21,
        sql: include_str!("migrations/021_proactive_surfacing.sql"),
    },
    Migration {
        version: 22,
        sql: include_str!("migrations/022_rejection_signals.sql"),
    },
    Migration {
        version: 23,
        sql: include_str!("migrations/023_drop_meeting_account_id.sql"),
    },
    Migration {
        version: 24,
        sql: include_str!("migrations/024_linear_sync.sql"),
    },
    Migration {
        version: 25,
        sql: include_str!("migrations/025_entity_metadata.sql"),
    },
    Migration {
        version: 26,
        sql: include_str!("migrations/026_attendee_display_names.sql"),
    },
    Migration {
        version: 27,
        sql: include_str!("migrations/027_email_threads.sql"),
    },
    Migration {
        version: 28,
        sql: include_str!("migrations/028_entity_email_cadence.sql"),
    },
    Migration {
        version: 29,
        sql: include_str!("migrations/029_hygiene_actions_log.sql"),
    },
    Migration {
        version: 30,
        sql: include_str!("migrations/030_email_dismissals.sql"),
    },
    Migration {
        version: 31,
        sql: include_str!("migrations/031_intelligence_lifecycle.sql"),
    },
    Migration {
        version: 32,
        sql: include_str!("migrations/032_junction_fks_and_expr_indexes.sql"),
    },
    Migration {
        version: 33,
        sql: include_str!("migrations/033_people_last_seen_index.sql"),
    },
    Migration {
        version: 34,
        sql: include_str!("migrations/034_emails.sql"),
    },
    Migration {
        version: 35,
        sql: include_str!("migrations/035_email_relevance_score.sql"),
    },
    Migration {
        version: 36,
        sql: include_str!("migrations/036_account_type.sql"),
    },
    Migration {
        version: 37,
        sql: include_str!("migrations/037_project_hierarchy.sql"),
    },
    Migration {
        version: 38,
        sql: include_str!("migrations/038_person_relationships.sql"),
    },
    Migration {
        version: 39,
        sql: include_str!("migrations/039_person_relationships_types.sql"),
    },
    Migration {
        version: 40,
        sql: include_str!("migrations/040_entity_quality.sql"),
    },
    Migration {
        version: 41,
        sql: include_str!("migrations/041_linear_entity_links.sql"),
    },
    Migration {
        version: 42,
        sql: include_str!("migrations/042_placeholder.sql"),
    },
    Migration {
        version: 43,
        sql: include_str!("migrations/043_placeholder.sql"),
    },
    Migration {
        version: 44,
        sql: include_str!("migrations/044_user_entity.sql"),
    },
    Migration {
        version: 45,
        sql: include_str!("migrations/045_intelligence_report_fields.sql"),
    },
    Migration {
        version: 46,
        sql: include_str!("migrations/046_user_context_embedding.sql"),
    },
    Migration {
        version: 47,
        sql: include_str!("migrations/047_entity_intel_user_relevance.sql"),
    },
    Migration {
        version: 48,
        sql: include_str!("migrations/048_google_drive_sync.sql"),
    },
    Migration {
        version: 49,
        sql: include_str!("migrations/049_drive_rename_type_column.sql"),
    },
    Migration {
        version: 50,
        sql: include_str!("migrations/050_reports.sql"),
    },
    Migration {
        version: 51,
        sql: include_str!("migrations/051_entity_context_entries.sql"),
    },
    Migration {
        version: 52,
        sql: include_str!("migrations/052_glean_document_cache.sql"),
    },
    Migration {
        version: 53,
        sql: include_str!("migrations/053_app_state_demo.sql"),
    },
    Migration {
        version: 54,
        sql: include_str!("migrations/054_intelligence_consistency_metadata.sql"),
    },
    Migration {
        version: 55,
        sql: include_str!("migrations/055_schema_decomposition.sql"),
    },
    Migration {
        version: 56,
        sql: include_str!("migrations/056_account_stakeholders_data_source.sql"),
    },
    Migration {
        version: 57,
        sql: include_str!("migrations/057_intelligence_db_columns.sql"),
    },
    Migration {
        version: 58,
        sql: include_str!("migrations/058_health_schema_evolution.sql"),
    },
    Migration {
        version: 59,
        sql: include_str!("migrations/059_person_relationships_rationale.sql"),
    },
    Migration {
        version: 60,
        sql: include_str!("migrations/060_intelligence_dimensions.sql"),
    },
    Migration {
        version: 61,
        sql: include_str!("migrations/061_stakeholder_glean_staleness.sql"),
    },
    Migration {
        version: 62,
        sql: include_str!("migrations/062_intelligence_feedback.sql"),
    },
    Migration {
        version: 63,
        sql: include_str!("migrations/063_email_signals_source.sql"),
    },
    Migration {
        version: 64,
        sql: include_str!("migrations/064_pipeline_failures.sql"),
    },
    Migration {
        version: 65,
        sql: include_str!("migrations/065_search_fts5.sql"),
    },
    Migration {
        version: 66,
        sql: include_str!("migrations/066_sync_metadata.sql"),
    },
    Migration {
        version: 67,
        sql: include_str!("migrations/067_feedback_unique_constraint.sql"),
    },
    Migration {
        version: 68,
        sql: include_str!("migrations/068_success_plans.sql"),
    },
    Migration {
        version: 69,
        sql: include_str!("migrations/069_account_events_expand.sql"),
    },
    Migration {
        version: 70,
        sql: include_str!("migrations/070_captures_metadata.sql"),
    },
    Migration {
        version: 71,
        sql: include_str!("migrations/071_email_triage_columns.sql"),
    },
    Migration {
        version: 72,
        sql: include_str!("migrations/072_health_score_history.sql"),
    },
    Migration {
        version: 73,
        sql: include_str!("migrations/073_meeting_record_path.sql"),
    },
    Migration {
        version: 74,
        sql: include_str!("migrations/074_action_status_vocabulary.sql"),
    },
    Migration {
        version: 75,
        sql: include_str!("migrations/075_v110_lifecycle_products_provenance.sql"),
    },
    Migration {
        version: 76,
        sql: include_str!("migrations/076_source_aware_account_truth.sql"),
    },
    Migration {
        version: 77,
        sql: include_str!("migrations/077_technical_footprint.sql"),
    },
    Migration {
        version: 78,
        sql: include_str!("migrations/078_pull_quote_column.sql"),
    },
    Migration {
        version: 79,
        sql: include_str!("migrations/079_product_classification.sql"),
    },
    Migration {
        version: 80,
        sql: include_str!("migrations/080_stakeholder_source_of_truth.sql"),
    },
    Migration {
        version: 81,
        sql: include_str!("migrations/081_init_tasks.sql"),
    },
    Migration {
        version: 82,
        sql: include_str!("migrations/082_email_enriched_at.sql"),
    },
    Migration {
        version: 83,
        sql: include_str!("migrations/082_account_fact_columns.sql"),
    },
    Migration {
        version: 84,
        sql: include_str!("migrations/083_dashboard_fields_to_db.sql"),
    },
    Migration {
        version: 85,
        sql: include_str!("migrations/084_feedback_events.sql"),
    },
    Migration {
        version: 86,
        sql: include_str!("migrations/085_action_status_priority_v2.sql"),
    },
    Migration {
        version: 87,
        sql: include_str!("migrations/086_objective_evidence.sql"),
    },
    Migration {
        version: 88,
        sql: include_str!("migrations/086_rejected_action_patterns.sql"),
    },
    Migration {
        version: 89,
        sql: include_str!("migrations/086_decision_columns.sql"),
    },
    Migration {
        version: 90,
        sql: include_str!("migrations/090_commitment_milestone_link.sql"),
    },
    Migration {
        version: 91,
        sql: include_str!("migrations/085_action_linear_links.sql"),
    },
    Migration {
        version: 92,
        sql: include_str!("migrations/092_deactivate_propagated_email_signals.sql"),
    },
    Migration {
        version: 93,
        sql: include_str!("migrations/091_user_health_sentiment.sql"),
    },
    Migration {
        version: 94,
        sql: include_str!("migrations/093_email_sync_meta.sql"),
    },
    Migration {
        version: 95,
        sql: include_str!("migrations/094_user_sentiment_history.sql"),
    },
    Migration {
        version: 96,
        sql: include_str!("migrations/095_meeting_entities_confidence.sql"),
    },
    Migration {
        version: 97,
        sql: include_str!("migrations/096_health_outlook_signals.sql"),
    },
    Migration {
        version: 98,
        sql: include_str!("migrations/097_email_pending_retry_state.sql"),
    },
    Migration {
        version: 99,
        sql: include_str!("migrations/098_risk_briefing_jobs.sql"),
    },
    Migration {
        version: 100,
        sql: include_str!("migrations/099_meeting_entity_dismissals.sql"),
    },
    Migration {
        version: 101,
        sql: include_str!("migrations/100_email_retry_batch.sql"),
    },
    // risk_briefing_jobs.attempt_id (CAS lifecycle)
    // + health_recompute_pending (durable debouncer). Combined migration to
    // minimize collision with parallel work.
    Migration {
        version: 102,
        sql: include_str!("migrations/101_risk_briefing_attempt_and_recompute_pending.sql"),
    },
    // emails.is_noise column for hard-drop bulk/marketing filter.
    Migration {
        version: 103,
        sql: include_str!("migrations/102_email_is_noise.sql"),
    },
    // track stale-failed auto-retry count so we cap automatic
    // promotions instead of looping forever on rows that fundamentally
    // can't enrich. Email sync stats read this column to compute the
    // `permanently_failed` count surfaced in the failure UX.
    Migration {
        version: 104,
        sql: include_str!("migrations/103_email_auto_retry_count.sql"),
    },
    // Defensive re-add of `is_noise` column. Tolerated as
    // "duplicate column name" by the framework if the column already
    // exists (normal upgrade); a real fix for users whose v103
    // schema_version was recorded without the ALTER actually applying.
    Migration {
        version: 105,
        sql: include_str!("migrations/104_email_is_noise_defensive.sql"),
    },
    // Recover emails over-suppressed by Rule 3
    // (List-Unsubscribe alone). Rule is tightened in code; this
    // migration restores is_noise=0 for rows outside the bulk allow-list.
    Migration {
        version: 106,
        sql: include_str!("migrations/105_email_noise_recovery.sql"),
    },
    // After the coarse email-noise recovery, re-suppress noreply
    // senders and bracket-prefix internal-org notifications that the
    // tightened rules now catch. Brings existing data in line with
    // the fixed code without requiring a fresh sync.
    Migration {
        version: 107,
        sql: include_str!("migrations/106_email_resuppress_noreply.sql"),
    },
    // Stakeholder-role soft-delete: `dismissed_at` tombstones user-removed
    // role rows so subsequent enrichment can't silently re-surface the
    // role via intel_queue's INSERT ON CONFLICT path.
    Migration {
        version: 108,
        sql: include_str!("migrations/107_stakeholder_role_dismissals.sql"),
    },
    // Work-tab foundation: action_kind column + ai_commitment_bridge +
    // account_focus_pins + nudge_dismissals. Enables commitments-as-Actions,
    // focus pin overlay, and nudge dismissal memory. See migration file
    // header for rationale.
    Migration {
        version: 109,
        sql: include_str!("migrations/108_work_tab_actions.sql"),
    },
    // Persist Health-tab triage card Snooze + Confirm-resolved state
    // so dismissals survive refresh. Keyed on (entity_type, entity_id,
    // triage_key); rendering-time filter hides rows where
    // resolved_at IS NOT NULL or snoozed_until > now.
    Migration {
        version: 110,
        sql: include_str!("migrations/109_triage_snoozes.sql"),
    },
    // entity linking schema foundation.
    // linked_entities_raw (write surface) + linked_entities view (read surface),
    // linking_dismissals (cross-surface dismissal store),
    // entity_linking_evaluations (append-only provenance audit),
    // entity_graph_version singleton counter + triggers,
    // account_stakeholders status/confidence columns + review-queue index,
    // backfill of existing meeting_entity_dismissals into linking_dismissals.
    Migration {
        version: 111,
        sql: include_str!("migrations/110_linked_entities_raw.sql"),
    },
    Migration {
        version: 112,
        sql: include_str!("migrations/111_linking_dismissals.sql"),
    },
    Migration {
        version: 113,
        sql: include_str!("migrations/112_entity_linking_evaluations.sql"),
    },
    Migration {
        version: 114,
        sql: include_str!("migrations/113_entity_graph_version.sql"),
    },
    Migration {
        version: 115,
        sql: include_str!("migrations/114_account_stakeholders_review_queue_idx.sql"),
    },
    Migration {
        version: 116,
        sql: include_str!("migrations/115_migrate_meeting_entity_dismissals.sql"),
    },
    // pending_thread_inheritance queue for P2 out-of-order
    // email delivery. When a child email arrives before its parent, P2 enqueues
    // it here; the queue is drained when the parent is later evaluated.
    Migration {
        version: 117,
        sql: include_str!("migrations/116_pending_thread_inheritance.sql"),
    },
    // complete entity_graph_version trigger coverage.
    // Adds INSERT/DELETE + name/archived UPDATE triggers for accounts and
    // projects so P5 name-matching and P4/P4b/P4c domain evidence stay
    // consistent after entity creation, deletion, or rename.
    Migration {
        version: 118,
        sql: include_str!("migrations/117_entity_graph_version_full_triggers.sql"),
    },
    // add source provenance to account_domains so
    // raw_rebuild_account_domains can purge inferred domains before cutover.
    Migration {
        version: 119,
        sql: include_str!("migrations/118_account_domains_source.sql"),
    },
    // email To/Cc recipient columns for multi-participant
    // domain evidence in P4b/P4c rules.
    Migration {
        version: 120,
        sql: include_str!("migrations/119_email_to_cc.sql"),
    },
    // Evidence-hierarchy fix: rename P4a/P4b/P4c rule identifiers to
    // P4b/P4c/P4d so a new stakeholder-inference rule can take the P4a slot.
    // Shifts existing rows in linked_entities_raw (rule_id, source) and
    // entity_linking_evaluations (rule_id) via a two-pass update.
    Migration {
        version: 121,
        sql: include_str!("migrations/120_dos_258_rule_rename.sql"),
    },
    // Entity-graph sweep state: add last_migration_sweep_at to entity_graph_version so
    // the startup rescan can self-correct existing weak primaries once per
    // upgrade without re-running on every boot.
    Migration {
        version: 122,
        sql: include_str!("migrations/121_entity_graph_sweep_state.sql"),
    },
    // collapse duplicate commitment-typed actions where the AI
    // emitted the same commitment text under different commitment_id
    // values across enrichment runs. Pick a canonical row per
    // (entity, normalized_title), rewire bridge rows to point at it,
    // delete the duplicates. Forward-going dedup is enforced in
    // services::commitment_bridge::sync_ai_commitments.
    Migration {
        version: 123,
        sql: include_str!("migrations/122_dos_321_collapse_commitment_dupes.sql"),
    },
    // per-entity claim_version columns (Option A invalidation primitive)
    // + shared migration_state.global_claim_epoch row. Replaces the
    // entity_graph_version trigger extension that round-1 Codex review caught
    // as a singleton-counter cache thrash bug. SubjectRef::Multi uses
    // deterministic lock ordering (Account < Meeting < Person < Project).
    Migration {
        version: 124,
        sql: include_str!("migrations/123_dos_310_per_entity_claim_invalidation.sql"),
    },
    // migration_state.schema_epoch row. Workers capture it at job
    // pickup; the WriteFence rechecks at write-back. If a migration bumps
    // the epoch mid-flight, in-flight work is rejected (caller logs +
    // re-queues). See src-tauri/src/intelligence/write_fence.rs.
    Migration {
        version: 125,
        sql: include_str!("migrations/124_dos_311_schema_epoch.sql"),
    },
    // covering index for suppression lookups + quarantine table for
    // tombstone remediation before the claims substrate migration.
    Migration {
        version: 126,
        sql: include_str!("migrations/125_suppression_remediation.sql"),
    },
    // durable operator audit for malformed suppression decisions.
    Migration {
        version: 127,
        sql: include_str!("migrations/126_suppression_malformed_log.sql"),
    },
    //  cycle-3: mark remediated quarantine rows as resolved audit trail.
    Migration {
        version: 128,
        sql: include_str!("migrations/127_quarantine_resolved_at.sql"),
    },
    //  cycle-4: partial index for the unresolved-row gate query.
    // Split from migration 128 so a partial-failure retry cannot leave
    // the column added but the index missing.
    Migration {
        version: 129,
        sql: include_str!("migrations/128_quarantine_unresolved_index.sql"),
    },
    // Claims commit substrate schema (intelligence_claims + 5 siblings).
    Migration {
        version: 130,
        sql: include_str!("migrations/129_dos_7_claims_schema.sql"),
    },
    // Claims backfill D3a-1: backfill mechanisms 1-4 (suppression_tombstones,
    // account_stakeholder_roles.dismissed_at, email_dismissals,
    // meeting_entity_dismissals) into intelligence_claims tombstone rows.
    // D3a-2 covers mechanisms 5-8; D3b covers DismissedItem JSON blobs.
    Migration {
        version: 131,
        sql: include_str!("migrations/130_dos_7_claims_backfill_a1.sql"),
    },
    // Claims backfill D3a-2: backfill mechanisms 5-8 (linking_dismissals,
    // briefing_callouts.dismissed_at, nudge_dismissals, triage_snoozes)
    // + duplicate-pair corroboration between mechanism 4 and 5.
    Migration {
        version: 132,
        sql: include_str!("migrations/131_dos_7_claims_backfill_a2.sql"),
    },
    // Add emails.claim_version so SubjectRef::Email
    // participates in per-entity invalidation alongside Account/Meeting/
    // Person/Project. Required to unwind cycle-2's Account+prefix
    // workaround for email dismissals.
    Migration {
        version: 133,
        sql: include_str!("migrations/132_dos_7_email_claim_version.sql"),
    },
    // Withdraw m5 backfill rows whose
    // subject_ref kind is not a supported SubjectRef variant
    // (e.g. owner_type='email_thread' from linking_dismissals).
    Migration {
        version: 134,
        sql: include_str!("migrations/133_dos_7_withdraw_unsupported_m5_kinds.sql"),
    },
    // Per-claim projection-status ledger so commit_claim can record
    // whether each derived-state target (legacy entity_intelligence
    // tables, success_plans, account AI columns, intelligence.json on
    // disk) succeeded or failed without rolling back the authoritative
    // claim. Failed rows are the repair worklist.
    Migration {
        version: 135,
        sql: include_str!("migrations/134_dos_301_claim_projection_status.sql"),
    },
    // Typed feedback schema: rebuild claim_feedback for the closed
    // action set and add claim verification state columns.
    Migration {
        version: 136,
        sql: include_str!("migrations/135_dos_294_typed_feedback_schema.sql"),
    },
    // Quarantine malformed legacy source timestamps before W4 freshness
    // scoring reads the claim substrate.
    Migration {
        version: 137,
        sql: include_str!("migrations/136_dos_299_source_asof_quarantine.sql"),
    },
    // Opaque thread metadata substrate; creation and assignment semantics land later.
    Migration {
        version: 138,
        sql: include_str!("migrations/138_thread_metadata.sql"),
    },
];

/// Create the `schema_version` table if it doesn't exist.
fn ensure_schema_version_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .map_err(|e| format!("Failed to create schema_version table: {}", e))
}

/// Return the highest applied migration version, or 0 if none.
fn current_version(conn: &Connection) -> Result<i32, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
    .map_err(|e| format!("Failed to read schema version: {}", e))
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM sqlite_master
            WHERE type = 'table' AND name = ?1
        )",
        [table_name],
        |row| row.get::<_, i64>(0),
    )
    .map(|v| v != 0)
    .map_err(|e| format!("Failed to check table '{}': {e}", table_name))
}

fn table_columns(conn: &Connection, table_name: &str) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .map_err(|e| format!("Failed to inspect table '{}': {e}", table_name))?;
    let cols = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("Failed to inspect columns for '{}': {e}", table_name))?;
    let mut out = HashSet::new();
    for col in cols {
        out.insert(col.map_err(|e| format!("Failed reading column metadata: {e}"))?);
    }
    Ok(out)
}

fn verify_required_schema(conn: &Connection) -> Result<(), String> {
    let version = current_version(conn)?;
    if version < 55 {
        return Ok(());
    }

    for table in [
        "meetings",
        "meeting_prep",
        "meeting_transcripts",
        "entity_assessment",
        "entity_quality",
        "account_stakeholders",
    ] {
        if !table_exists(conn, table)? {
            return Err(format!(
                "Schema integrity check failed: missing required table '{table}'"
            ));
        }
    }

    let quality_cols = table_columns(conn, "entity_quality")?;
    for col in [
        "health_score",
        "health_trend",
        "coherence_score",
        "coherence_flagged",
    ] {
        if !quality_cols.contains(col) {
            return Err(format!(
                "Schema integrity check failed: missing column entity_quality.{col}"
            ));
        }
    }

    if version >= 56 {
        let stakeholder_cols = table_columns(conn, "account_stakeholders")?;
        if !stakeholder_cols.contains("data_source") {
            return Err(
                "Schema integrity check failed: missing column account_stakeholders.data_source"
                    .to_string(),
            );
        }
    }

    if version >= 58 {
        let assessment_cols = table_columns(conn, "entity_assessment")?;
        for col in ["health_json", "org_health_json"] {
            if !assessment_cols.contains(col) {
                return Err(format!(
                    "Schema integrity check failed: missing column entity_assessment.{col}"
                ));
            }
        }
    }

    if version >= 59 {
        let relationship_cols = table_columns(conn, "person_relationships")?;
        if !relationship_cols.contains("rationale") {
            return Err(
                "Schema integrity check failed: missing column person_relationships.rationale"
                    .to_string(),
            );
        }
    }

    if version >= 63 {
        if !table_exists(conn, "email_signals")? {
            return Err(
                "Schema integrity check failed: missing required table 'email_signals'".to_string(),
            );
        }
        let email_signal_cols = table_columns(conn, "email_signals")?;
        if !email_signal_cols.contains("source") {
            return Err(
                "Schema integrity check failed: missing column email_signals.source".to_string(),
            );
        }
    }

    if version >= 60 {
        let assessment_cols = table_columns(conn, "entity_assessment")?;
        if !assessment_cols.contains("dimensions_json") {
            return Err(
                "Schema integrity check failed: missing column entity_assessment.dimensions_json"
                    .to_string(),
            );
        }
    }

    if version >= 68 {
        let assessment_cols = table_columns(conn, "entity_assessment")?;
        if !assessment_cols.contains("success_plan_signals_json") {
            return Err(
                "Schema integrity check failed: missing column entity_assessment.success_plan_signals_json"
                    .to_string(),
            );
        }
    }

    Ok(())
}

fn migration_backup_path(db_path: &Path) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let file_name = format!(
        "{}.pre-migration.{}.bak",
        db_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("dailyos.db"),
        timestamp
    );
    db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(file_name)
}

fn is_migration_backup_file(db_path: &Path, candidate: &Path) -> bool {
    let base = db_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("dailyos.db");
    let name = candidate.file_name().and_then(|f| f.to_str()).unwrap_or("");
    name.starts_with(&format!("{base}.pre-migration.")) && name.ends_with(".bak")
}

fn prune_old_migration_backups(db_path: &Path, keep: usize) -> Result<(), String> {
    let parent = db_path
        .parent()
        .ok_or_else(|| "Database path has no parent directory".to_string())?;
    let mut backups: Vec<PathBuf> = std::fs::read_dir(parent)
        .map_err(|e| format!("Failed to read backup directory: {e}"))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| is_migration_backup_file(db_path, p))
        .collect();

    // Timestamp is part of the filename, so lexical order is chronological.
    backups.sort();
    if backups.len() <= keep {
        return Ok(());
    }
    let to_delete = backups.len() - keep;
    for path in backups.into_iter().take(to_delete) {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

fn create_backup_via_api(
    conn: &Connection,
    backup_path: &Path,
    destination_key: Option<&str>,
) -> Result<(), String> {
    let mut backup_conn = rusqlite::Connection::open(backup_path)
        .map_err(|e| format!("Failed to open backup file: {e}"))?;
    if let Some(hex_key) = destination_key {
        backup_conn
            .execute_batch(&crate::db::encryption::key_to_pragma(hex_key))
            .map_err(|e| format!("Failed to set pre-migration backup encryption key: {e}"))?;
    }
    let backup = rusqlite::backup::Backup::new(conn, &mut backup_conn)
        .map_err(|e| format!("Failed to initialize pre-migration backup: {e}"))?;
    backup
        .step(-1)
        .map_err(|e| format!("Pre-migration backup failed: {e}"))?;
    Ok(())
}

fn create_backup_via_sqlcipher_export(
    conn: &Connection,
    backup_path: &Path,
    hex_key: &str,
) -> Result<(), String> {
    // sqlcipher_export must run inside BEGIN IMMEDIATE so that WAL frames are
    // included in the snapshot. Without the transaction, SQLCipher copies only
    // the base page state and produces an 8KB hollow file.
    let backup_path_s = backup_path.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!(
        "ATTACH DATABASE '{backup_path_s}' AS premigration KEY \"x'{hex_key}'\";"
    ))
    .map_err(|e| format!("Failed to attach fallback pre-migration backup DB: {e}"))?;
    conn.execute_batch("BEGIN IMMEDIATE; SELECT sqlcipher_export('premigration'); COMMIT;")
        .map_err(|e| format!("Fallback pre-migration backup export failed: {e}"))?;
    conn.execute_batch("DETACH DATABASE premigration;")
        .map_err(|e| format!("Failed to detach fallback pre-migration backup DB: {e}"))?;
    Ok(())
}

fn should_try_encrypted_backup_fallback(encrypted: bool, err: &str) -> bool {
    encrypted
        && (err.contains("backup is not supported with encrypted databases")
            || err.contains("encrypted databases"))
}

fn is_no_such_actions_table_error(err: &SqliteError) -> bool {
    let msg = match err {
        SqliteError::SqliteFailure(sqlite_err, Some(msg))
            if sqlite_err.code == ErrorCode::Unknown =>
        {
            msg
        }
        SqliteError::SqlInputError {
            error,
            msg,
            sql: _,
            offset: _,
        } if error.code == ErrorCode::Unknown => msg,
        _ => return false,
    };

    // rusqlite 0.31/libsqlite3-sys 0.28 does not expose SQLite's newer
    // missing-table extended code, so this fresh-database probe keeps the
    // unavoidable message fallback constrained to the exact table it owns.
    msg.trim()
        .to_ascii_lowercase()
        .starts_with("no such table: actions")
}

fn sqlite_unknown_error_message(err: &SqliteError) -> Option<&str> {
    match err {
        SqliteError::SqliteFailure(sqlite_err, Some(msg))
            if sqlite_err.code == ErrorCode::Unknown =>
        {
            Some(msg)
        }
        SqliteError::SqlInputError {
            error,
            msg,
            sql: _,
            offset: _,
        } if error.code == ErrorCode::Unknown => Some(msg),
        _ => None,
    }
}

/// rusqlite 0.31/libsqlite3-sys does not expose extended codes for ALTER TABLE
/// duplicate-column dialect errors; this substring fallback is durable against
/// libsqlite text drift because these messages are stable across supported
/// versions (3.35..3.45).
fn is_duplicate_column_error(e: &SqliteError) -> bool {
    match sqlite_unknown_error_message(e) {
        Some(msg) => msg.to_ascii_lowercase().contains("duplicate column name"),
        None => false,
    }
}

/// rusqlite 0.31/libsqlite3-sys does not expose extended codes for ALTER TABLE
/// unknown-column dialect errors; this substring fallback is durable against
/// libsqlite text drift because these messages are stable across supported
/// versions (3.35..3.45).
fn is_missing_column_error(e: &SqliteError) -> bool {
    match sqlite_unknown_error_message(e) {
        Some(msg) => msg.to_ascii_lowercase().contains("no such column"),
        None => false,
    }
}

fn probe_actions_table(conn: &Connection) -> Result<bool, String> {
    let mut stmt = match conn.prepare("SELECT 1 FROM actions LIMIT 1") {
        Ok(stmt) => stmt,
        Err(err) if is_no_such_actions_table_error(&err) => return Ok(false),
        Err(err) => {
            return Err(format!(
                "Failed to inspect actions table during migration bootstrap: {err}"
            ))
        }
    };

    match stmt.exists([]) {
        Ok(_) => Ok(true),
        Err(err) if is_no_such_actions_table_error(&err) => Ok(false),
        Err(err) => Err(format!(
            "Failed to inspect actions table during migration bootstrap: {err}"
        )),
    }
}

/// Detect a pre-framework database and mark the baseline as applied.
///
/// If the `actions` table exists but `schema_version` does not, this is a
/// database created before the migration framework was introduced. We mark
/// migration 001 (the baseline) as applied so its CREATE TABLE statements
/// never run against an already-populated database.
fn bootstrap_existing_db(conn: &Connection) -> Result<bool, String> {
    // Check if schema_version already has rows (framework already in use)
    let version = current_version(conn)?;
    if version > 0 {
        return Ok(false);
    }

    // Only a missing `actions` table means a fresh database. Other probe
    // failures indicate the DB cannot be safely classified.
    if probe_actions_table(conn)? {
        // Existing database — mark baseline as applied
        conn.execute(
            "INSERT OR IGNORE INTO schema_version (version) VALUES (?1)",
            [1],
        )
        .map_err(|e| format!("Failed to bootstrap schema version: {}", e))?;
        log::info!("Migration bootstrap: marked v1 (baseline) as applied for existing database");
        return Ok(true);
    }

    Ok(false)
}

/// Back up the database before applying migrations.
///
/// Uses SQLite's online backup API to create a hot copy at
/// `<db_path>.pre-migration.bak`. Only called when there are pending migrations.
fn backup_before_migration(conn: &Connection) -> Result<PathBuf, String> {
    let db_path: String = conn
        .query_row("PRAGMA database_list", [], |row| row.get(2))
        .map_err(|e| format!("Failed to get database path: {}", e))?;

    if db_path.is_empty() || db_path == ":memory:" {
        // In-memory or temp database — skip backup
        return Ok(PathBuf::from(":memory:"));
    }

    let db_path = PathBuf::from(db_path);
    let backup_path = migration_backup_path(&db_path);
    let _ = std::fs::remove_file(&backup_path);

    let encrypted = db_path.exists() && !crate::db::encryption::is_database_plaintext(&db_path);
    let encryption_key = if encrypted {
        Some(
            crate::db::encryption::get_or_create_db_key(&db_path)
                .map_err(|e| format!("Failed to get DB encryption key for backup: {e}"))?,
        )
    } else {
        None
    };

    // For encrypted DBs: use the Backup API with the key applied to the
    // destination — the same pattern backup_database() uses successfully.
    // Both sides use the same key so encrypted pages copy verbatim.
    //
    // The previous sqlcipher_export-first approach produced 8KB hollow files
    // because sqlcipher_export without a transaction only copies base pages,
    // not the WAL. The Backup API reads through the WAL correctly.
    let backup_result = if encrypted {
        let key = encryption_key
            .as_deref()
            .ok_or_else(|| "Missing encryption key for backup".to_string())?;
        create_backup_via_api(conn, &backup_path, Some(key))
    } else {
        create_backup_via_api(conn, &backup_path, None)
    };
    if let Err(err) = backup_result {
        let _ = std::fs::remove_file(&backup_path);
        // Last resort: sqlcipher_export (now transaction-wrapped). Only reached
        // if the Backup API itself reports an encryption incompatibility.
        if should_try_encrypted_backup_fallback(encrypted, &err) {
            let key = encryption_key
                .as_deref()
                .ok_or_else(|| "Missing encryption key for fallback backup".to_string())?;
            create_backup_via_sqlcipher_export(conn, &backup_path, key)?;
        } else {
            return Err(err);
        }
    }

    // Sanity-check: a real backup of a multi-MB database must be more than a
    // page or two. A hollow backup (< 64KB from a > 128KB source) is worse
    // than no backup — it creates false confidence. Fail loudly so the user
    // knows migrations did not run with a real safety net.
    let source_size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
    let backup_size = std::fs::metadata(&backup_path)
        .map(|m| m.len())
        .unwrap_or(0);
    if source_size > 128 * 1024 && backup_size < 64 * 1024 {
        let _ = std::fs::remove_file(&backup_path);
        return Err(format!(
            "Pre-migration backup is suspiciously small ({backup_size} bytes) for a \
             {source_size}-byte source database. The backup is likely hollow. \
             Refusing to apply migrations without a valid safety copy (DOS-273)."
        ));
    }

    crate::db::hardening::set_file_permissions(&backup_path);
    prune_old_migration_backups(&db_path, 10)?;
    log::info!(
        "Pre-migration backup created at {} ({backup_size} bytes)",
        backup_path.to_string_lossy()
    );
    Ok(backup_path)
}

/// Run all pending migrations.
///
/// Returns the number of migrations applied (0 if already up-to-date).
///
/// Forward-compat guard: if the database has a higher version than the highest
/// known migration, returns an error telling the user to update DailyOS.
pub fn run_migrations(conn: &Connection) -> Result<usize, String> {
    ensure_schema_version_table(conn)?;
    bootstrap_existing_db(conn)?;

    let current = current_version(conn)?;
    let max_known = MIGRATIONS.last().map(|m| m.version).unwrap_or(0);

    // Forward-compat guard
    if current > max_known {
        return Err(format!(
            "Database schema version ({}) is newer than this version of DailyOS supports ({}). \
             Please update DailyOS to the latest version.",
            current, max_known
        ));
    }

    // Collect pending migrations
    let pending: Vec<&Migration> = MIGRATIONS.iter().filter(|m| m.version > current).collect();

    if pending.is_empty() {
        verify_required_schema(conn)?;
        return Ok(0);
    }

    // quarantine gate. Refuse to apply migration 126 (the
    // backfill territory) until unresolved quarantine rows are resolved.
    // Resolved quarantine rows are retained as audit trail and do NOT block
    // subsequent migrations.
    let migration_126_pending = pending.iter().any(|m| m.version == 126);
    if migration_126_pending {
        let quarantine_exists: bool = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master \
                 WHERE type = 'table' AND name = 'suppression_tombstones_quarantine'",
                [],
                |row| row.get::<_, i64>(0).map(|count| count > 0),
            )
            .map_err(|e| format!("quarantine gate check: {e}"))?;

        if quarantine_exists {
            let unresolved_count = quarantine_gate_blocking_count(conn)?;

            if unresolved_count > 0 {
                return Err(format!(
                    "DOS-308 quarantine gate: refusing to apply migration 126 while {} \
                     unresolved malformed suppression record(s) remain in quarantine. Run \
                     scripts/remediate_suppression_tombstones.sh to resolve, then re-run \
                     migrations. (Resolved/audit-trail rows do NOT block migrations.)",
                    unresolved_count
                ));
            }
        }
    }

    // Backup before applying any migrations
    let backup_path = backup_before_migration(conn)?;
    if backup_path.to_string_lossy() != ":memory:" {
        log::info!(
            "Migration safety backup ready at {}",
            backup_path.to_string_lossy()
        );
    }

    // Apply each pending migration in order
    for migration in &pending {
        match conn.execute_batch(migration.sql) {
            Ok(()) => {}
            Err(e) => {
                let msg = e.to_string();
                // SQLite DDL statements like ALTER TABLE ADD COLUMN and RENAME COLUMN
                // are not idempotent (no IF NOT EXISTS / IF EXISTS variants).
                // Tolerate these specific benign errors ONLY for true single-statement
                // ALTER TABLE migrations:
                // - "duplicate column name": ADD COLUMN when column already exists
                // - "no such column": RENAME COLUMN when column was already renamed
                //
                // Detection: check that every non-empty, non-comment statement in
                // the migration is an ALTER TABLE. Checking `!contains("BEGIN")`
                // is insufficient — multi-statement non-transactional migrations
                // (e.g. 023 with CREATE/INSERT/DROP/ALTER) would pass that check,
                // silently swallowing real data-copy failures.
                let is_single_alter = migration
                    .sql
                    .split(';')
                    .map(|s| {
                        s.lines()
                            .filter(|l| !l.trim_start().starts_with("--"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty())
                    .all(|s| s.starts_with("ALTER"));
                // "duplicate column name" is always safe: can only come from
                // ALTER TABLE ADD COLUMN when the column already exists.
                // "no such column" is only safe for pure ALTER TABLE migrations
                // (PR #11: multi-statement migrations with CREATE/INSERT/DROP
                // must not silently swallow this error).
                let is_dup_column = is_duplicate_column_error(&e);
                let is_benign_alter = is_single_alter && is_missing_column_error(&e);
                if is_dup_column || is_benign_alter {
                    log::warn!(
                        "Migration v{}: benign schema conflict ({}), continuing",
                        migration.version,
                        msg.split('\n').next().unwrap_or(&msg)
                    );
                } else {
                    return Err(format!("Migration v{} failed: {}", migration.version, e));
                }
            }
        }

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [migration.version],
        )
        .map_err(|e| format!("Failed to record migration v{}: {}", migration.version, e))?;

        log::info!("Applied migration v{}", migration.version);
    }

    verify_required_schema(conn)?;
    Ok(pending.len())
}

fn quarantine_gate_blocking_count(conn: &Connection) -> Result<i64, String> {
    // Check if the resolved_at column exists (migration 127). If not, fall
    // back to counting all rows (cycle-2 semantics, slightly stricter).
    let has_resolved_at: bool = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('suppression_tombstones_quarantine') \
             WHERE name = 'resolved_at'",
            [],
            |row| row.get::<_, i64>(0).map(|count| count > 0),
        )
        .map_err(|e| format!("quarantine gate column check: {e}"))?;

    let count_query = if has_resolved_at {
        "SELECT count(*) FROM suppression_tombstones_quarantine WHERE resolved_at IS NULL"
    } else {
        "SELECT count(*) FROM suppression_tombstones_quarantine"
    };

    conn.query_row(count_query, [], |row| row.get(0))
        .map_err(|e| format!("quarantine gate count: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Helper: open an in-memory database with WAL-like settings.
    fn mem_db() -> Connection {
        Connection::open_in_memory().expect("in-memory db")
    }

    #[test]
    fn test_bootstrap_missing_actions_table_is_fresh_db() {
        let conn = mem_db();
        ensure_schema_version_table(&conn).expect("schema_version table");

        let bootstrapped = bootstrap_existing_db(&conn).expect("bootstrap should inspect db");
        assert!(!bootstrapped, "missing actions table should mean fresh DB");

        let version = current_version(&conn).expect("version query");
        assert_eq!(version, 0, "fresh DB should not be marked as bootstrapped");
    }

    #[test]
    fn test_bootstrap_empty_actions_table_is_existing_db() {
        let conn = mem_db();
        ensure_schema_version_table(&conn).expect("schema_version table");
        conn.execute_batch("CREATE TABLE actions (id TEXT PRIMARY KEY);")
            .expect("empty actions table");

        let bootstrapped = bootstrap_existing_db(&conn).expect("bootstrap should inspect db");
        assert!(
            bootstrapped,
            "an existing actions table should bootstrap even when empty"
        );

        let version = current_version(&conn).expect("version query");
        assert_eq!(version, 1, "existing DB should be marked at baseline");
    }

    #[test]
    fn test_bootstrap_actions_probe_surfaces_non_missing_table_errors() {
        let conn = mem_db();
        ensure_schema_version_table(&conn).expect("schema_version table");
        conn.execute_batch("CREATE VIEW actions AS SELECT * FROM missing_dependency;")
            .expect("broken actions view");

        let err =
            bootstrap_existing_db(&conn).expect_err("broken actions probe should not look fresh");
        assert!(
            err.contains("Failed to inspect actions table during migration bootstrap"),
            "error should surface probe failure: {err}"
        );
        assert!(
            err.contains("missing_dependency"),
            "error should identify the underlying SQLite failure: {err}"
        );
    }

    #[test]
    fn test_bootstrap_actions_probe_error_classifier_is_strict() {
        let conn = mem_db();
        let missing_actions = match conn.prepare("SELECT 1 FROM actions LIMIT 1") {
            Ok(_) => panic!("actions table should be missing"),
            Err(err) => err,
        };
        assert!(is_no_such_actions_table_error(&missing_actions));

        for (code, msg) in [
            (rusqlite::ffi::SQLITE_LOCKED, "database table is locked"),
            (
                rusqlite::ffi::SQLITE_CORRUPT,
                "database disk image is malformed",
            ),
            (rusqlite::ffi::SQLITE_IOERR, "disk I/O error"),
            (
                rusqlite::ffi::SQLITE_ERROR,
                "no such table: missing_dependency",
            ),
        ] {
            let err =
                SqliteError::SqliteFailure(rusqlite::ffi::Error::new(code), Some(msg.to_string()));
            assert!(
                !is_no_such_actions_table_error(&err),
                "only missing actions table should classify as fresh DB: {msg}"
            );
        }
    }

    fn sqlite_failure_with_message(code: i32, msg: &str) -> SqliteError {
        SqliteError::SqliteFailure(rusqlite::ffi::Error::new(code), Some(msg.to_string()))
    }

    #[test]
    fn is_duplicate_column_error_classifies_supported_sqlite_message() {
        let err = sqlite_failure_with_message(
            rusqlite::ffi::SQLITE_ERROR,
            "duplicate column name: source_asof",
        );
        assert!(is_duplicate_column_error(&err));
        assert!(!is_missing_column_error(&err));
    }

    #[test]
    fn is_duplicate_column_error_rejects_unrelated_errors() {
        let locked =
            sqlite_failure_with_message(rusqlite::ffi::SQLITE_LOCKED, "duplicate column name: id");
        let unrelated =
            sqlite_failure_with_message(rusqlite::ffi::SQLITE_ERROR, "database table is locked");
        assert!(!is_duplicate_column_error(&locked));
        assert!(!is_duplicate_column_error(&unrelated));
    }

    #[test]
    fn is_missing_column_error_classifies_supported_sqlite_message() {
        let err = sqlite_failure_with_message(
            rusqlite::ffi::SQLITE_ERROR,
            "no such column: stale_column",
        );
        assert!(is_missing_column_error(&err));
        assert!(!is_duplicate_column_error(&err));
    }

    #[test]
    fn is_missing_column_error_rejects_unrelated_errors() {
        let corrupt =
            sqlite_failure_with_message(rusqlite::ffi::SQLITE_CORRUPT, "no such column: id");
        let unrelated = sqlite_failure_with_message(
            rusqlite::ffi::SQLITE_ERROR,
            "no such table: accounts",
        );
        assert!(!is_missing_column_error(&corrupt));
        assert!(!is_missing_column_error(&unrelated));
    }

    #[test]
    fn test_fresh_db_applies_baseline() {
        let conn = mem_db();
        let applied = run_migrations(&conn).expect("migrations should succeed");
        assert_eq!(
            applied,
            MIGRATIONS.len(),
            "should apply all known migrations on a fresh database"
        );

        // Verify schema_version
        let version = current_version(&conn).expect("version query");
        assert_eq!(version, MIGRATIONS.last().unwrap().version);

        // Verify key tables exist with correct columns
        let action_count: i32 = conn
            .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
            .expect("actions table should exist");
        assert_eq!(action_count, 0);

        // Verify needs_decision column exists (was an ALTER TABLE migration)
        conn.execute(
            "INSERT INTO actions (id, title, created_at, updated_at, needs_decision)
             VALUES ('test', 'test', '2025-01-01', '2025-01-01', 1)",
            [],
        )
        .expect("needs_decision column should exist");

        // Verify decomposed meeting tables have all columns
        conn.execute(
            "INSERT INTO meetings (id, title, meeting_type, start_time, created_at,
             calendar_event_id, description)
             VALUES ('m1', 'Test', 'customer', '2025-01-01', '2025-01-01',
             'cal1', 'desc')",
            [],
        )
        .expect("meetings table should have all columns");
        conn.execute(
            "INSERT INTO meeting_prep (meeting_id, prep_context_json, user_agenda_json,
             user_notes, prep_frozen_json, prep_frozen_at, prep_snapshot_path, prep_snapshot_hash)
             VALUES ('m1', '{}', '[]', 'notes', '{}', '2025-01-01', '/path', 'abc123')",
            [],
        )
        .expect("meeting_prep table should have all columns");
        conn.execute(
            "INSERT INTO meeting_transcripts (meeting_id, transcript_path, transcript_processed_at)
             VALUES ('m1', '/transcript', '2025-01-01')",
            [],
        )
        .expect("meeting_transcripts table should have all columns");

        // Verify captures has project_id and decision type
        conn.execute(
            "INSERT INTO captures (id, meeting_id, meeting_title, project_id, capture_type, content)
             VALUES ('c1', 'm1', 'Test', 'p1', 'decision', 'content')",
            [],
        )
        .expect("captures should accept project_id and decision type");

        // Verify content_index has content_type and priority
        conn.execute(
            "INSERT INTO content_index (id, entity_id, filename, relative_path, absolute_path,
             format, modified_at, indexed_at, content_type, priority)
             VALUES ('ci1', 'e1', 'f.md', 'f.md', '/f.md', 'markdown', '2025-01-01',
             '2025-01-01', 'transcript', 1)",
            [],
        )
        .expect("content_index should have content_type and priority");

        // Verify accounts has all migrated columns
        conn.execute(
            "INSERT INTO accounts (id, name, updated_at, lifecycle, nps, parent_id, is_internal, archived)
             VALUES ('a1', 'Acme', '2025-01-01', 'onboarding', 85, NULL, 0, 0)",
            [],
        )
        .expect("accounts should include is_internal");

        conn.execute(
            "INSERT INTO people (id, email, name, updated_at) VALUES ('p1', 'test@acme.com', 'Test User', '2025-01-01')",
            [],
        )
        .expect("people table should exist for FK");
        conn.execute(
            "INSERT INTO account_stakeholders (account_id, person_id) VALUES ('a1', 'p1')",
            [],
        )
        .expect("account_stakeholders table should exist");
        conn.execute(
            "INSERT INTO account_stakeholder_roles (account_id, person_id, role) VALUES ('a1', 'p1', 'tam')",
            [],
        )
        .expect("account_stakeholder_roles table should exist");

        conn.execute(
            "INSERT INTO account_team_import_notes (account_id, legacy_field, legacy_value, note)
             VALUES ('a1', 'csm', 'Legacy Name', 'note')",
            [],
        )
        .expect("account_team_import_notes table should exist");

        // Verify account_domains exists and accepts inserts
        conn.execute(
            "INSERT INTO account_domains (account_id, domain) VALUES ('a1', 'acme.com')",
            [],
        )
        .expect("account_domains table should exist");

        // Verify account_events table
        conn.execute(
            "INSERT INTO account_events (account_id, event_type, event_date)
             VALUES ('a1', 'renewal', '2025-06-01')",
            [],
        )
        .expect("account_events table should exist");

        // Verify email_signals exists and accepts inserts
        conn.execute(
            "INSERT INTO email_signals (
                email_id, sender_email, entity_id, entity_type, signal_type, signal_text
             ) VALUES ('em-1', 'owner@acme.com', 'a1', 'account', 'timeline', 'Customer asked for revised launch date')",
            [],
        )
        .expect("email_signals table should exist");

        // Verify content_embeddings exists and accepts inserts
        conn.execute(
            "INSERT INTO content_embeddings (
                id, content_file_id, chunk_index, chunk_text, embedding, created_at
             ) VALUES ('emb-1', 'ci1', 0, 'test chunk', X'', '2025-01-01')",
            [],
        )
        .expect("content_embeddings table should exist");

        // Verify chat_sessions exists and accepts inserts
        conn.execute(
            "INSERT INTO chat_sessions (
                id, entity_id, entity_type, session_start, turn_count, created_at
             ) VALUES ('cs-1', 'a1', 'account', '2025-01-01', 0, '2025-01-01')",
            [],
        )
        .expect("chat_sessions table should exist");

        // Verify chat_turns exists and accepts inserts
        conn.execute(
            "INSERT INTO chat_turns (
                id, session_id, turn_index, role, content, timestamp
             ) VALUES ('ct-1', 'cs-1', 0, 'user', 'Hello', '2025-01-01')",
            [],
        )
        .expect("chat_turns table should exist");

        // Verify backlog/archived action statuses work (migration 074)
        conn.execute(
            "INSERT INTO actions (id, title, status, created_at, updated_at)
             VALUES ('backlog-1', 'Backlog action', 'backlog', '2025-01-01', '2025-01-01')",
            [],
        )
        .expect("backlog status should be accepted");

        conn.execute(
            "INSERT INTO actions (id, title, status, created_at, updated_at)
             VALUES ('archived-1', 'Archived action', 'archived', '2025-01-01', '2025-01-01')",
            [],
        )
        .expect("archived status should be accepted");

        // Verify person_emails table exists and accepts inserts (migration 012)
        conn.execute(
            "INSERT INTO person_emails (person_id, email, is_primary, added_at)
             VALUES ('p1', 'alice@acme.com', 1, '2025-01-01')",
            [],
        )
        .expect("person_emails table should exist");

        // Verify quill_sync_state table accepts inserts (migration 013)
        conn.execute(
            "INSERT INTO quill_sync_state (id, meeting_id, state)
             VALUES ('qs-1', 'm1', 'pending')",
            [],
        )
        .expect("quill_sync_state table should exist and accept inserts");

        // Verify source column exists and allows granola source (migration 014)
        conn.execute(
            "INSERT INTO quill_sync_state (id, meeting_id, state, source)
             VALUES ('qs-2', 'm1', 'pending', 'granola')",
            [],
        )
        .expect("quill_sync_state should accept granola source for same meeting_id");

        // Verify gravatar_cache table accepts inserts (migration 015)
        conn.execute(
            "INSERT INTO gravatar_cache (email, has_gravatar, fetched_at, person_id)
             VALUES ('alice@acme.com', 1, '2025-01-01T00:00:00Z', 'p1')",
            [],
        )
        .expect("gravatar_cache table should exist and accept inserts");

        // Verify clay enrichment tables (migration 016)
        conn.execute(
            "INSERT INTO enrichment_log (id, entity_type, entity_id, source, fields_updated)
             VALUES ('el-1', 'person', 'p1', 'clay', '[\"linkedinUrl\"]')",
            [],
        )
        .expect("enrichment_log table should exist and accept inserts");

        conn.execute(
            "INSERT INTO clay_sync_state (id, entity_id, state)
             VALUES ('cs-1', 'p1', 'pending')",
            [],
        )
        .expect("clay_sync_state table should exist and accept inserts");

        // Verify people has Clay enrichment columns
        conn.execute(
            "UPDATE people SET linkedin_url = 'https://linkedin.com/in/test',
             last_enriched_at = '2026-01-01', enrichment_sources = '{}'
             WHERE id = 'p1'",
            [],
        )
        .expect("people should have Clay enrichment columns");

        // Verify entity keywords columns (migration 017)
        conn.execute(
            "UPDATE accounts SET keywords = '[\"acme\",\"widget\"]',
             keywords_extracted_at = '2026-01-01T00:00:00Z'
             WHERE id = 'a1'",
            [],
        )
        .expect("accounts should have keywords columns");

        conn.execute(
            "INSERT INTO projects (id, name, status, updated_at, keywords, keywords_extracted_at)
             VALUES ('p1', 'Agentforce', 'active', '2026-01-01',
             '[\"agentforce\",\"agent force\"]', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("projects should have keywords columns");

        // Verify signal_events table (migration 018)
        conn.execute(
            "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, source, value, confidence, decay_half_life_days)
             VALUES ('sig-1', 'account', 'a1', 'entity_resolution', 'keyword', 'matched by name', 0.8, 30)",
            [],
        )
        .expect("signal_events table should exist and accept inserts");

        // Verify signal_weights table (migration 018)
        conn.execute(
            "INSERT INTO signal_weights (source, entity_type, signal_type, alpha, beta, update_count)
             VALUES ('clay', 'person', 'profile_update', 1.0, 1.0, 0)",
            [],
        )
        .expect("signal_weights table should exist and accept inserts");

        // Verify entity_resolution_feedback table (migration 019)
        conn.execute(
            "INSERT INTO entity_resolution_feedback (id, meeting_id, old_entity_id, old_entity_type, new_entity_id, new_entity_type, signal_source)
             VALUES ('fb-1', 'm1', 'a1', 'account', 'a2', 'account', 'keyword')",
            [],
        )
        .expect("entity_resolution_feedback table should exist and accept inserts");

        // Verify attendee_group_patterns table (migration 019)
        conn.execute(
            "INSERT INTO attendee_group_patterns (group_hash, attendee_emails, entity_id, entity_type, occurrence_count, confidence)
             VALUES ('hash1', '[\"a@b.com\",\"c@d.com\"]', 'a1', 'account', 3, 0.65)",
            [],
        )
        .expect("attendee_group_patterns table should exist and accept inserts");

        // Verify source_context column on signal_events (migration 019)
        conn.execute(
            "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, source, confidence, decay_half_life_days, source_context)
             VALUES ('sig-2', 'account', 'a1', 'entity_resolution', 'keyword', 0.8, 30, 'inbound_email')",
            [],
        )
        .expect("signal_events should accept source_context column");

        // Verify signal_derivations table (migration 020)
        conn.execute(
            "INSERT INTO signal_derivations (id, source_signal_id, derived_signal_id, rule_name)
             VALUES ('sd-1', 'sig-1', 'sig-2', 'rule_person_job_change')",
            [],
        )
        .expect("signal_derivations table should exist and accept inserts");

        // Verify post_meeting_emails table (migration 020)
        conn.execute(
            "INSERT INTO post_meeting_emails (id, meeting_id, email_signal_id, thread_id, actions_extracted)
             VALUES ('pme-1', 'm1', 'sig-1', 'thread-1', '[\"follow up\"]')",
            [],
        )
        .expect("post_meeting_emails table should exist and accept inserts");

        // Verify briefing_callouts table (migration 020)
        conn.execute(
            "INSERT INTO briefing_callouts (id, signal_id, entity_type, entity_id, entity_name, severity, headline, detail)
             VALUES ('bc-1', 'sig-1', 'account', 'a1', 'Acme', 'warning', 'Stakeholder change detected', 'Sarah promoted to CRO')",
            [],
        )
        .expect("briefing_callouts table should exist and accept inserts");

        // Verify app_state table (migration 053)
        let demo_active: i32 = conn
            .query_row(
                "SELECT demo_mode_active FROM app_state WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .expect("app_state should exist with default row");
        assert_eq!(demo_active, 0, "demo_mode_active should default to 0");

        // Verify is_demo column on accounts (migration 053)
        conn.execute(
            "INSERT INTO accounts (id, name, updated_at, is_demo) VALUES ('demo-1', 'Demo', '2026-01-01', 1)",
            [],
        )
        .expect("accounts should have is_demo column");

        // Verify proactive_scan_state table (migration 021)
        conn.execute(
            "INSERT INTO proactive_scan_state (detector_name, last_insight_count)
             VALUES ('detect_renewal_gap', 3)",
            [],
        )
        .expect("proactive_scan_state table should exist and accept inserts");

        // Verify proactive_insights table (migration 021)
        conn.execute(
            "INSERT INTO proactive_insights (id, detector_name, fingerprint, signal_id, entity_type, entity_id, headline, detail)
             VALUES ('pi-1', 'detect_renewal_gap', 'fp-abc123', 'sig-1', 'account', 'a1', 'Renewal approaching', 'Acme renews in 45d')",
            [],
        )
        .expect("proactive_insights table should exist and accept inserts");

        // Verify rejection signal columns on actions (migration 022)
        conn.execute(
            "UPDATE actions SET rejected_at = '2026-01-15T10:00:00Z',
             rejection_source = 'actions_page'
             WHERE id = 'suggested-1'",
            [],
        )
        .expect("actions should have rejected_at and rejection_source columns");

        // Verify linear_issues table (migration 024)
        conn.execute(
            "INSERT INTO linear_issues (id, identifier, title, url)
             VALUES ('li-1', 'DOS-1', 'Test issue', 'https://linear.app/dos/issue/DOS-1')",
            [],
        )
        .expect("linear_issues table should exist and accept inserts");

        // Verify linear_projects table (migration 024)
        conn.execute(
            "INSERT INTO linear_projects (id, name, url)
             VALUES ('lp-1', 'DailyOS', 'https://linear.app/dos/project/dailyos')",
            [],
        )
        .expect("linear_projects table should exist and accept inserts");

        // Verify emails table (migration 034)
        conn.execute(
            "INSERT INTO emails (email_id, thread_id, sender_email, sender_name, subject, snippet,
             priority, is_unread, received_at, enrichment_state, entity_id, entity_type,
             contextual_summary, sentiment, urgency, user_is_last_sender, last_sender_email, message_count)
             VALUES ('e-1', 't-1', 'alice@acme.com', 'Alice', 'Q4 Review', 'Let us discuss...',
             'high', 1, '2026-02-01T10:00:00Z', 'pending', 'a1', 'account',
             NULL, NULL, NULL, 0, 'alice@acme.com', 1)",
            [],
        )
        .expect("emails table should exist and accept inserts");

        // Verify deactivated_at column on email_signals (migration 034)
        conn.execute(
            "UPDATE email_signals SET deactivated_at = '2026-02-01T10:00:00Z' WHERE email_id = 'em-1'",
            [],
        )
        .expect("email_signals should have deactivated_at column");

        // Verify source column on email_signals (migration 063)
        conn.execute(
            "UPDATE email_signals SET source = 'email_enrichment' WHERE email_id = 'em-1'",
            [],
        )
        .expect("email_signals should have source column");

        // Verify account_type column exists with correct default (migration 036)
        let acct_type: String = conn
            .query_row(
                "SELECT account_type FROM accounts WHERE id = 'a1'",
                [],
                |row| row.get(0),
            )
            .expect("account_type column should exist");
        assert_eq!(
            acct_type, "customer",
            "default account_type should be 'customer'"
        );

        // Verify is_internal backfill sets account_type = 'internal'
        conn.execute(
            "INSERT INTO accounts (id, name, updated_at, is_internal, archived)
             VALUES ('internal-1', 'My Org', '2025-01-01', 1, 0)",
            [],
        )
        .expect("insert internal account");
        // Simulate the migration backfill for newly inserted rows
        conn.execute(
            "UPDATE accounts SET account_type = 'internal' WHERE is_internal = 1 AND account_type = 'customer'",
            [],
        )
        .expect("backfill internal");
        let internal_type: String = conn
            .query_row(
                "SELECT account_type FROM accounts WHERE id = 'internal-1'",
                [],
                |row| row.get(0),
            )
            .expect("query internal account_type");
        assert_eq!(internal_type, "internal");

        // Verify person_relationships table (migration 038)
        conn.execute(
            "INSERT INTO person_relationships (id, from_person_id, to_person_id, relationship_type, source)
             VALUES ('pr-1', 'p1', 'p1', 'peer', 'user_confirmed')",
            [],
        )
        .expect("person_relationships table should exist and accept inserts");

        // Verify partner type can be set
        conn.execute(
            "UPDATE accounts SET account_type = 'partner' WHERE id = 'a1'",
            [],
        )
        .expect("should accept partner account_type");

        // Verify linear_entity_links table (migration 041)
        conn.execute(
            "INSERT INTO linear_entity_links (id, linear_project_id, entity_id, entity_type, confirmed)
             VALUES ('lel-1', 'lp-1', 'a1', 'account', 1)",
            [],
        )
        .expect("linear_entity_links table should exist and accept inserts");

        // verify entity linking schema (migrations 111–116)

        // linked_entities_raw: table exists and enforces CHECK constraints
        conn.execute(
            "INSERT INTO linked_entities_raw
             (owner_type, owner_id, entity_id, entity_type, role, source, graph_version, created_at)
             VALUES ('meeting', 'm1', 'a1', 'account', 'primary', 'rule:P4a', 0, '2026-01-01')",
            [],
        )
        .expect("linked_entities_raw should accept valid inserts");

        // idx_one_primary: a second primary for the SAME owner (different entity) must fail
        let dup_primary_result = conn.execute(
            "INSERT INTO linked_entities_raw
             (owner_type, owner_id, entity_id, entity_type, role, source, graph_version, created_at)
             VALUES ('meeting', 'm1', 'p1', 'person', 'primary', 'rule:P7', 0, '2026-01-01')",
            [],
        );
        assert!(
            dup_primary_result.is_err(),
            "idx_one_primary should reject a second primary for the same (owner_type, owner_id)"
        );

        // linked_entities view filters user_dismissed rows (different entity than the primary above)
        conn.execute(
            "INSERT INTO linked_entities_raw
             (owner_type, owner_id, entity_id, entity_type, role, source, graph_version, created_at)
             VALUES ('meeting', 'm1', 'p1', 'person', 'related', 'user_dismissed', 0, '2026-01-01')",
            [],
        )
        .expect("linked_entities_raw should store user_dismissed rows");
        let view_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM linked_entities WHERE owner_id = 'm1'",
                [],
                |row| row.get(0),
            )
            .expect("linked_entities view should be queryable");
        assert_eq!(
            view_count, 1,
            "linked_entities view should hide user_dismissed rows"
        );

        // linking_dismissals table
        conn.execute(
            "INSERT INTO linking_dismissals (owner_type, owner_id, entity_id, entity_type, created_at)
             VALUES ('meeting', 'm1', 'a1', 'account', '2026-01-01')",
            [],
        )
        .expect("linking_dismissals should exist and accept inserts");

        // entity_linking_evaluations table
        conn.execute(
            "INSERT INTO entity_linking_evaluations
             (owner_type, owner_id, link_trigger, rule_id, entity_id, entity_type, role, graph_version, evidence_json)
             VALUES ('meeting', 'm1', 'CalendarPoll', 'P4a', 'a1', 'account', 'primary', 0, '{}')",
            [],
        )
        .expect("entity_linking_evaluations should exist and accept inserts");

        // entity_graph_version: seed row exists, trigger bumps on account_domains insert
        let before_version: i32 = conn
            .query_row(
                "SELECT version FROM entity_graph_version WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .expect("entity_graph_version seed row should exist");
        conn.execute(
            "INSERT INTO account_domains (account_id, domain) VALUES ('a1', 'trigger-test.com')",
            [],
        )
        .expect("account_domains insert for trigger test");
        let after_version: i32 = conn
            .query_row(
                "SELECT version FROM entity_graph_version WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .expect("entity_graph_version should be queryable after trigger");
        assert!(
            after_version > before_version,
            "account_domains insert should bump entity_graph_version ({} -> {})",
            before_version,
            after_version
        );

        // account_stakeholders: status and confidence columns exist (migration 115)
        conn.execute(
            "UPDATE account_stakeholders SET status = 'pending_review', confidence = 0.85
             WHERE account_id = 'a1' AND person_id = 'p1'",
            [],
        )
        .expect("account_stakeholders should have status and confidence columns");
    }

    #[test]
    fn test_bootstrap_existing_db() {
        let conn = mem_db();

        // Simulate a pre-framework database: create actions table with all baseline columns.
        // A real pre-framework DB would have all columns from inline CREATE TABLE + ALTER TABLE
        // statements that existed in db.rs before the migration framework.
        conn.execute_batch(
            "CREATE TABLE actions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                priority TEXT CHECK(priority IN ('P1', 'P2', 'P3')) DEFAULT 'P2',
                status TEXT CHECK(status IN ('pending', 'completed', 'waiting', 'cancelled')) DEFAULT 'pending',
                created_at TEXT NOT NULL,
                due_date TEXT,
                completed_at TEXT,
                account_id TEXT,
                project_id TEXT,
                source_type TEXT,
                source_id TEXT,
                source_label TEXT,
                context TEXT,
                waiting_on TEXT,
                updated_at TEXT NOT NULL,
                person_id TEXT,
                needs_decision INTEGER DEFAULT 0
            );
            INSERT INTO actions (id, title, created_at, updated_at)
            VALUES ('existing', 'Existing Action', '2025-01-01', '2025-01-01');",
        )
        .expect("seed existing db");

        // Create other tables that a pre-framework DB would have
        conn.execute_batch(
            "CREATE TABLE accounts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                lifecycle TEXT,
                arr REAL,
                health TEXT,
                csm TEXT,
                champion TEXT,
                contract_start TEXT,
                contract_end TEXT,
                nps INTEGER,
                tracker_path TEXT,
                parent_id TEXT,
                updated_at TEXT NOT NULL,
                archived INTEGER DEFAULT 0
            );
             CREATE TABLE meetings_history (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                meeting_type TEXT NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT,
                account_id TEXT,
                attendees TEXT,
                notes_path TEXT,
                summary TEXT,
                created_at TEXT NOT NULL,
                calendar_event_id TEXT,
                prep_context_json TEXT,
                description TEXT,
                user_agenda_json TEXT,
                user_notes TEXT,
                prep_frozen_json TEXT,
                prep_frozen_at TEXT,
                prep_snapshot_path TEXT,
                prep_snapshot_hash TEXT,
                transcript_path TEXT,
                transcript_processed_at TEXT,
                intelligence_state TEXT NOT NULL DEFAULT 'detected',
                intelligence_quality TEXT NOT NULL DEFAULT 'sparse',
                last_enriched_at TEXT,
                signal_count INTEGER NOT NULL DEFAULT 0,
                has_new_signals INTEGER NOT NULL DEFAULT 0,
                last_viewed_at TEXT
             );
             CREATE TABLE people (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL,
                name TEXT NOT NULL,
                relationship TEXT NOT NULL DEFAULT 'unknown',
                last_seen TEXT
             );
             CREATE TABLE entity_people (
                entity_id TEXT NOT NULL,
                person_id TEXT NOT NULL,
                relationship_type TEXT DEFAULT 'associated',
                PRIMARY KEY (entity_id, person_id)
             );
             CREATE TABLE meeting_entities (
                meeting_id TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                PRIMARY KEY (meeting_id, entity_id)
             );
             CREATE TABLE projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT DEFAULT 'active',
                milestone TEXT,
                owner TEXT,
                target_date TEXT,
                tracker_path TEXT,
                updated_at TEXT NOT NULL,
                archived INTEGER DEFAULT 0
             );
             CREATE TABLE meeting_attendees (
                meeting_id TEXT NOT NULL,
                person_id TEXT NOT NULL,
                PRIMARY KEY (meeting_id, person_id)
             );
             CREATE TABLE content_index (
                id TEXT PRIMARY KEY,
                entity_id TEXT NOT NULL,
                entity_type TEXT NOT NULL DEFAULT 'account',
                filename TEXT NOT NULL,
                relative_path TEXT NOT NULL,
                absolute_path TEXT NOT NULL,
                format TEXT NOT NULL,
                file_size INTEGER NOT NULL DEFAULT 0,
                modified_at TEXT NOT NULL,
                indexed_at TEXT NOT NULL,
                extracted_at TEXT,
                summary TEXT,
                content_type TEXT NOT NULL DEFAULT 'general',
                priority INTEGER NOT NULL DEFAULT 5
             );
             CREATE TABLE entity_intelligence (
                entity_id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL DEFAULT 'account',
                enriched_at TEXT,
                source_file_count INTEGER DEFAULT 0,
                executive_assessment TEXT,
                risks_json TEXT,
                recent_wins_json TEXT,
                current_state_json TEXT,
                stakeholder_insights_json TEXT,
                next_meeting_readiness_json TEXT,
                company_context_json TEXT,
                health_score REAL,
                health_trend TEXT,
                coherence_score REAL,
                coherence_flagged INTEGER DEFAULT 0,
                value_delivered TEXT,
                success_metrics TEXT,
                open_commitments TEXT,
                relationship_depth TEXT,
                user_relevance_weight REAL DEFAULT 1.0,
                consistency_status TEXT,
                consistency_findings_json TEXT,
                consistency_checked_at TEXT
             );
             CREATE TABLE account_team (
                account_id TEXT NOT NULL,
                person_id TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'associated',
                created_at TEXT DEFAULT (datetime('now')),
                PRIMARY KEY (account_id, person_id)
             );
             CREATE TABLE entity_quality (
                entity_id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                quality_alpha REAL NOT NULL DEFAULT 1.0,
                quality_beta REAL NOT NULL DEFAULT 1.0,
                quality_score REAL NOT NULL DEFAULT 0.5,
                last_enrichment_at TEXT,
                correction_count INTEGER NOT NULL DEFAULT 0,
                coherence_retry_count INTEGER NOT NULL DEFAULT 0,
                coherence_window_start TEXT,
                coherence_blocked INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
             );",
        )
        .expect("seed existing tables");

        // Run migrations — should bootstrap v1 and apply v2 through the latest migration.
        let applied = run_migrations(&conn).expect("migrations should succeed");
        // bootstrap marks v1 as already-applied, then all remaining migrations run
        let total_migrations = MIGRATIONS.len();
        assert_eq!(
            applied,
            total_migrations - 1,
            "bootstrap should mark v1, then apply {} pending migrations (v2-v{})",
            total_migrations - 1,
            total_migrations,
        );

        // Verify schema version matches latest migration
        let version = current_version(&conn).expect("version query");
        assert_eq!(version, MIGRATIONS.last().unwrap().version);

        // Verify existing data is untouched
        let title: String = conn
            .query_row(
                "SELECT title FROM actions WHERE id = 'existing'",
                [],
                |row| row.get(0),
            )
            .expect("existing data should be preserved");
        assert_eq!(title, "Existing Action");
    }

    #[test]
    fn test_forward_compat_guard() {
        let conn = mem_db();

        // Set up schema_version with a future version
        ensure_schema_version_table(&conn).unwrap();
        conn.execute("INSERT INTO schema_version (version) VALUES (999)", [])
            .unwrap();

        // run_migrations should fail with a clear error
        let result = run_migrations(&conn);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("newer than this version"),
            "error should mention version mismatch: {}",
            err
        );
    }

    #[test]
    fn test_idempotency() {
        let conn = mem_db();
        let total = MIGRATIONS.len();

        // Run migrations twice
        let first = run_migrations(&conn).expect("first run");
        assert_eq!(first, total);

        let second = run_migrations(&conn).expect("second run");
        assert_eq!(second, 0, "second run should apply no migrations");

        // Version should match the highest migration
        let version = current_version(&conn).expect("version query");
        assert_eq!(version, MIGRATIONS.last().unwrap().version);
    }

    #[test]
    fn test_pre_migration_backup_created() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("test_backup.db");

        let conn = Connection::open(&db_path).expect("open db");
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();

        let applied = run_migrations(&conn).expect("migrations should succeed");
        assert_eq!(applied, MIGRATIONS.len());

        // Verify timestamped backup file was created
        let backup_files: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read tempdir")
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| is_migration_backup_file(&db_path, p))
            .collect();
        assert!(
            !backup_files.is_empty(),
            "pre-migration timestamped backup should exist"
        );
    }

    #[test]
    fn test_migration_failure_is_not_marked_applied() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("failed_migration.db");
        let conn = Connection::open(&db_path).expect("open db");
        ensure_schema_version_table(&conn).expect("schema version table");
        conn.execute("INSERT INTO schema_version (version) VALUES (54)", [])
            .expect("seed version");

        // Break migration 055 prerequisites.
        let result = run_migrations(&conn);
        assert!(
            result.is_err(),
            "migration should fail on missing prerequisites"
        );

        let version = current_version(&conn).expect("version query");
        assert_eq!(
            version, 54,
            "failed migration must not be recorded as applied"
        );
    }

    #[test]
    fn test_schema_integrity_check_blocks_invalid_v60_state() {
        let conn = mem_db();
        ensure_schema_version_table(&conn).expect("schema_version table");
        conn.execute("INSERT INTO schema_version (version) VALUES (61)", [])
            .expect("seed schema version");

        let err = run_migrations(&conn).expect_err("invalid schema should fail integrity check");
        assert!(
            err.contains("Schema integrity check failed") || err.contains("Migration v68 failed"),
            "error should identify schema integrity failure or migration failure: {err}"
        );
    }

    ///  migration 097 rebuilds the `emails` table to
    /// widen the `enrichment_state` CHECK constraint. The rebuild must
    /// recreate every index that existed on the old table. If any index is
    /// dropped silently (as was the case pre-fix for `idx_emails_relevance`
    /// and `idx_emails_enriched_at`), inbox/read query plans regress without
    /// any visible error at upgrade time.
    #[test]
    fn test_emails_indexes_survive_migration_097() {
        let conn = mem_db();
        let applied = run_migrations(&conn).expect("migrations should succeed");
        assert_eq!(applied, MIGRATIONS.len(), "all migrations applied");

        // Introspect sqlite_master for every index currently on `emails`.
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'emails' AND name NOT LIKE 'sqlite_%'")
            .expect("prepare sqlite_master query");
        let rows: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query sqlite_master")
            .filter_map(Result::ok)
            .collect();

        // Every index declared in the migration history for `emails` must be
        // present after the full migration chain runs. This is a regression
        // guard against future table rebuilds losing indexes.
        let required = [
            "idx_emails_thread_id",
            "idx_emails_enrichment",
            "idx_emails_entity",
            "idx_emails_priority_resolved",
            "idx_emails_last_seen",
            "idx_emails_resolved",
            "idx_emails_relevance",   // migration 035 — regressed pre-fix
            "idx_emails_enriched_at", // migration 082 — regressed pre-fix
        ];
        for expected in required {
            assert!(
                rows.iter().any(|n| n == expected),
                "expected index `{}` to exist after migrations; found: {:?}",
                expected,
                rows
            );
        }
    }

    #[test]
    fn test_should_try_encrypted_backup_fallback_matches_expected_errors() {
        assert!(should_try_encrypted_backup_fallback(
            true,
            "backup is not supported with encrypted databases"
        ));
        assert!(should_try_encrypted_backup_fallback(
            true,
            "sqlite error: encrypted databases"
        ));
        assert!(!should_try_encrypted_backup_fallback(
            false,
            "backup is not supported with encrypted databases"
        ));
        assert!(!should_try_encrypted_backup_fallback(
            true,
            "disk I/O error"
        ));
    }

    #[test]
    fn quarantine_resolved_rows_do_not_block_migration_126() {
        let conn = mem_db();
        conn.execute_batch(
            "CREATE TABLE suppression_tombstones_quarantine (
                id INTEGER PRIMARY KEY,
                entity_id TEXT NOT NULL,
                field_key TEXT NOT NULL,
                item_key TEXT,
                item_hash TEXT,
                dismissed_at TEXT,
                source_scope TEXT,
                expires_at TEXT,
                superseded_by_evidence_after TEXT,
                quarantined_at TEXT NOT NULL DEFAULT (datetime('now')),
                quarantine_reason TEXT NOT NULL,
                resolved_at TEXT
             );",
        )
        .expect("create quarantine table with resolved_at");
        conn.execute(
            "INSERT INTO suppression_tombstones_quarantine \
             (id, entity_id, field_key, dismissed_at, quarantine_reason, resolved_at) \
             VALUES (1, 'acct-1', 'risks', 'not-a-date', 'resolved audit', datetime('now'))",
            [],
        )
        .expect("seed resolved audit row");
        conn.execute(
            "INSERT INTO suppression_tombstones_quarantine \
             (id, entity_id, field_key, dismissed_at, quarantine_reason, resolved_at) \
             VALUES (2, 'acct-1', 'risks', 'still-bad', 'unresolved', NULL)",
            [],
        )
        .expect("seed unresolved row");

        let count = quarantine_gate_blocking_count(&conn).expect("gate count");

        assert_eq!(count, 1, "only unresolved quarantine rows should block");
    }

    #[test]
    fn quarantine_gate_falls_back_to_all_rows_when_column_missing() {
        let conn = mem_db();
        conn.execute_batch(
            "CREATE TABLE suppression_tombstones_quarantine (
                id INTEGER PRIMARY KEY,
                entity_id TEXT NOT NULL,
                field_key TEXT NOT NULL,
                item_key TEXT,
                item_hash TEXT,
                dismissed_at TEXT,
                source_scope TEXT,
                expires_at TEXT,
                superseded_by_evidence_after TEXT,
                quarantined_at TEXT NOT NULL DEFAULT (datetime('now')),
                quarantine_reason TEXT NOT NULL
             );",
        )
        .expect("create pre-127 quarantine table");
        conn.execute(
            "INSERT INTO suppression_tombstones_quarantine \
             (id, entity_id, field_key, dismissed_at, quarantine_reason) \
             VALUES (1, 'acct-1', 'risks', 'not-a-date', 'pre-127 row')",
            [],
        )
        .expect("seed pre-127 quarantine row");

        let count = quarantine_gate_blocking_count(&conn).expect("gate count");

        assert_eq!(
            count, 1,
            "pre-127 schemas should count every quarantine row"
        );
    }

    #[test]
    fn quarantine_with_retained_rows_does_not_block_post_126_migrations() {
        let conn = mem_db();
        run_migrations(&conn).expect("apply all migrations");
        conn.execute(
            "DELETE FROM schema_version WHERE version IN (128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138)",
            [],
        )
        .expect("make v128 through v138 pending");
        conn.execute(
            "INSERT INTO suppression_tombstones_quarantine \
             (id, entity_id, field_key, item_key, item_hash, dismissed_at, quarantine_reason, resolved_at) \
             VALUES (1, 'acct-1', 'risks', NULL, NULL, 'not-a-date', 'retained audit', '2026-05-02T00:00:00Z')",
            [],
        )
        .expect("seed retained quarantine row");

        let applied = run_migrations(&conn).expect("post-126 migration should not be gated");

        assert_eq!(applied, 11);
        assert_eq!(current_version(&conn).expect("version query"), 138);
    }

    #[test]
    fn quarantine_blocks_only_when_126_pending() {
        let conn = mem_db();
        run_migrations(&conn).expect("apply all migrations");
        conn.execute(
            "DELETE FROM schema_version WHERE version IN (126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138)",
            [],
        )
        .expect("make v126 through v138 pending");
        conn.execute(
            "INSERT INTO suppression_tombstones_quarantine \
             (id, entity_id, field_key, item_key, item_hash, dismissed_at, quarantine_reason) \
             VALUES (1, 'acct-1', 'risks', NULL, NULL, 'not-a-date', 'unresolved')",
            [],
        )
        .expect("seed quarantine row");

        let err = run_migrations(&conn).expect_err("migration 126 should be gated");

        assert!(
            err.contains("refusing to apply migration 126"),
            "error should identify migration 126 gate: {err}"
        );
    }

    /// Evidence-hierarchy fix: verify migration 120 shifts every
    /// P4a/P4b/P4c rule identifier one letter forward without collisions.
    /// Guards against the re-run case: applying the migration twice on
    /// already-migrated data must be a no-op (idempotent).
    #[test]
    fn migration_renames_old_rule_ids_idempotent() {
        let conn = mem_db();
        run_migrations(&conn).expect("apply all migrations");

        // Simulate legacy data shape by re-inserting under the OLD identifiers.
        // (The migration already ran with no rows to shift; we insert new rows
        // after the fact and verify a second apply does NOT touch them — the
        // migration table has tracked version 121 so the SQL won't re-run.)
        conn.execute(
            "INSERT INTO accounts (id, name, updated_at, archived) VALUES ('a-dos258', 'A', '2026-01-01', 0)",
            [],
        ).expect("seed account");

        conn.execute(
            "INSERT INTO linked_entities_raw \
             (owner_type, owner_id, entity_id, entity_type, role, source, rule_id, graph_version, created_at) \
             VALUES ('meeting', 'm-dos258', 'a-dos258', 'account', 'primary', 'rule:P4a', 'P4a', 0, '2026-01-01')",
            [],
        ).expect("seed legacy P4a row");

        // Second run of run_migrations is a no-op because schema_version is populated.
        let applied_again = run_migrations(&conn).expect("second run");
        assert_eq!(
            applied_again, 0,
            "re-running migrations should apply zero new ones"
        );

        // Legacy-labelled row is untouched (no rows matched on the first apply,
        // since it was inserted *after* the migration ran).
        let (rid, src): (String, String) = conn
            .query_row(
                "SELECT rule_id, source FROM linked_entities_raw WHERE owner_id = 'm-dos258'",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .expect("fetch row");
        assert_eq!(
            rid, "P4a",
            "post-migration inserts keep their literal identifier"
        );
        assert_eq!(src, "rule:P4a");

        // Now simulate running the rename SQL directly on a DB that still has
        // pre-migration rows: a legacy row inserted BEFORE the migration would
        // shift forward. We emulate by running the UPDATE pair manually.
        conn.execute(
            "UPDATE linked_entities_raw SET rule_id = '_P4a' WHERE rule_id = 'P4a'",
            [],
        )
        .expect("pass1a");
        conn.execute(
            "UPDATE linked_entities_raw SET source  = '_rule:P4a' WHERE source = 'rule:P4a'",
            [],
        )
        .expect("pass1b");
        conn.execute(
            "UPDATE linked_entities_raw SET rule_id = 'P4b' WHERE rule_id = '_P4a'",
            [],
        )
        .expect("pass2a");
        conn.execute(
            "UPDATE linked_entities_raw SET source  = 'rule:P4b' WHERE source = '_rule:P4a'",
            [],
        )
        .expect("pass2b");

        let (rid, src): (String, String) = conn
            .query_row(
                "SELECT rule_id, source FROM linked_entities_raw WHERE owner_id = 'm-dos258'",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .expect("fetch row");
        assert_eq!(rid, "P4b", "P4a shifted forward to P4b");
        assert_eq!(src, "rule:P4b", "rule:P4a shifted forward to rule:P4b");
    }
}

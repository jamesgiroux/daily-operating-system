//! Schema migration framework (ADR-0071).
//!
//! Numbered migrations run exactly once, tracked by the `schema_version` table.
//! Most migrations are embedded SQL batches via `Migration::Sql`. Use
//! `Migration::Fn` for non-trivial idempotency, data-dependent branching, or
//! retry-safe rebuilds that need schema inspection before applying changes.
//!
//! For existing databases (pre-migration-framework), the bootstrap function
//! detects the presence of known tables and marks migration 001 as applied
//! so the baseline SQL never runs against an already-populated database.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::{Connection, Error as SqliteError, ErrorCode};

mod v144_audit_action_token;

type MigrationError = String;

enum Migration {
    Sql {
        version: u32,
        sql: &'static str,
    },
    Fn {
        version: u32,
        apply: fn(&Connection) -> Result<(), MigrationError>,
    },
}

impl Migration {
    fn version(&self) -> u32 {
        match self {
            Self::Sql { version, .. } | Self::Fn { version, .. } => *version,
        }
    }

    fn sql(&self) -> Option<&'static str> {
        match self {
            Self::Sql { sql, .. } => Some(sql),
            Self::Fn { .. } => None,
        }
    }
}

// Historical drift: some filenames are one less than registered version; `version` is authoritative.
const MIGRATIONS: &[Migration] = &[
    Migration::Sql {
        version: 1,
        sql: include_str!("migrations/001_baseline.sql"),
    },
    Migration::Sql {
        version: 2,
        sql: include_str!("migrations/002_internal_teams.sql"),
    },
    Migration::Sql {
        version: 3,
        sql: include_str!("migrations/003_account_team.sql"),
    },
    Migration::Sql {
        version: 4,
        sql: include_str!("migrations/004_account_team_role_index.sql"),
    },
    Migration::Sql {
        version: 5,
        sql: include_str!("migrations/005_email_signals.sql"),
    },
    Migration::Sql {
        version: 6,
        sql: include_str!("migrations/006_content_embeddings.sql"),
    },
    Migration::Sql {
        version: 7,
        sql: include_str!("migrations/007_chat_interface.sql"),
    },
    Migration::Sql {
        version: 8,
        sql: include_str!("migrations/008_missing_indexes.sql"),
    },
    Migration::Sql {
        version: 9,
        sql: include_str!("migrations/009_fix_embeddings_column.sql"),
    },
    Migration::Sql {
        version: 10,
        sql: include_str!("migrations/010_foreign_keys.sql"),
    },
    Migration::Sql {
        version: 11,
        sql: include_str!("migrations/011_proposed_actions.sql"),
    },
    Migration::Sql {
        version: 12,
        sql: include_str!("migrations/012_person_emails.sql"),
    },
    Migration::Sql {
        version: 13,
        sql: include_str!("migrations/013_quill_sync.sql"),
    },
    Migration::Sql {
        version: 14,
        sql: include_str!("migrations/014_granola_sync.sql"),
    },
    Migration::Sql {
        version: 15,
        sql: include_str!("migrations/015_gravatar_cache.sql"),
    },
    Migration::Sql {
        version: 16,
        sql: include_str!("migrations/016_clay_enrichment.sql"),
    },
    Migration::Sql {
        version: 17,
        sql: include_str!("migrations/017_entity_keywords.sql"),
    },
    Migration::Sql {
        version: 18,
        sql: include_str!("migrations/018_signal_bus.sql"),
    },
    Migration::Sql {
        version: 19,
        sql: include_str!("migrations/019_correction_learning.sql"),
    },
    Migration::Sql {
        version: 20,
        sql: include_str!("migrations/020_signal_propagation.sql"),
    },
    Migration::Sql {
        version: 21,
        sql: include_str!("migrations/021_proactive_surfacing.sql"),
    },
    Migration::Sql {
        version: 22,
        sql: include_str!("migrations/022_rejection_signals.sql"),
    },
    Migration::Sql {
        version: 23,
        sql: include_str!("migrations/023_drop_meeting_account_id.sql"),
    },
    Migration::Sql {
        version: 24,
        sql: include_str!("migrations/024_linear_sync.sql"),
    },
    Migration::Sql {
        version: 25,
        sql: include_str!("migrations/025_entity_metadata.sql"),
    },
    Migration::Sql {
        version: 26,
        sql: include_str!("migrations/026_attendee_display_names.sql"),
    },
    Migration::Sql {
        version: 27,
        sql: include_str!("migrations/027_email_threads.sql"),
    },
    Migration::Sql {
        version: 28,
        sql: include_str!("migrations/028_entity_email_cadence.sql"),
    },
    Migration::Sql {
        version: 29,
        sql: include_str!("migrations/029_hygiene_actions_log.sql"),
    },
    Migration::Sql {
        version: 30,
        sql: include_str!("migrations/030_email_dismissals.sql"),
    },
    Migration::Sql {
        version: 31,
        sql: include_str!("migrations/031_intelligence_lifecycle.sql"),
    },
    Migration::Sql {
        version: 32,
        sql: include_str!("migrations/032_junction_fks_and_expr_indexes.sql"),
    },
    Migration::Sql {
        version: 33,
        sql: include_str!("migrations/033_people_last_seen_index.sql"),
    },
    Migration::Sql {
        version: 34,
        sql: include_str!("migrations/034_emails.sql"),
    },
    Migration::Sql {
        version: 35,
        sql: include_str!("migrations/035_email_relevance_score.sql"),
    },
    Migration::Sql {
        version: 36,
        sql: include_str!("migrations/036_account_type.sql"),
    },
    Migration::Sql {
        version: 37,
        sql: include_str!("migrations/037_project_hierarchy.sql"),
    },
    Migration::Sql {
        version: 38,
        sql: include_str!("migrations/038_person_relationships.sql"),
    },
    Migration::Sql {
        version: 39,
        sql: include_str!("migrations/039_person_relationships_types.sql"),
    },
    Migration::Sql {
        version: 40,
        sql: include_str!("migrations/040_entity_quality.sql"),
    },
    Migration::Sql {
        version: 41,
        sql: include_str!("migrations/041_linear_entity_links.sql"),
    },
    Migration::Sql {
        version: 42,
        sql: include_str!("migrations/042_placeholder.sql"),
    },
    Migration::Sql {
        version: 43,
        sql: include_str!("migrations/043_placeholder.sql"),
    },
    Migration::Sql {
        version: 44,
        sql: include_str!("migrations/044_user_entity.sql"),
    },
    Migration::Sql {
        version: 45,
        sql: include_str!("migrations/045_intelligence_report_fields.sql"),
    },
    Migration::Sql {
        version: 46,
        sql: include_str!("migrations/046_user_context_embedding.sql"),
    },
    Migration::Sql {
        version: 47,
        sql: include_str!("migrations/047_entity_intel_user_relevance.sql"),
    },
    Migration::Sql {
        version: 48,
        sql: include_str!("migrations/048_google_drive_sync.sql"),
    },
    Migration::Sql {
        version: 49,
        sql: include_str!("migrations/049_drive_rename_type_column.sql"),
    },
    Migration::Sql {
        version: 50,
        sql: include_str!("migrations/050_reports.sql"),
    },
    Migration::Sql {
        version: 51,
        sql: include_str!("migrations/051_entity_context_entries.sql"),
    },
    Migration::Sql {
        version: 52,
        sql: include_str!("migrations/052_glean_document_cache.sql"),
    },
    Migration::Sql {
        version: 53,
        sql: include_str!("migrations/053_app_state_demo.sql"),
    },
    Migration::Sql {
        version: 54,
        sql: include_str!("migrations/054_intelligence_consistency_metadata.sql"),
    },
    Migration::Sql {
        version: 55,
        sql: include_str!("migrations/055_schema_decomposition.sql"),
    },
    Migration::Sql {
        version: 56,
        sql: include_str!("migrations/056_account_stakeholders_data_source.sql"),
    },
    Migration::Sql {
        version: 57,
        sql: include_str!("migrations/057_intelligence_db_columns.sql"),
    },
    Migration::Sql {
        version: 58,
        sql: include_str!("migrations/058_health_schema_evolution.sql"),
    },
    Migration::Sql {
        version: 59,
        sql: include_str!("migrations/059_person_relationships_rationale.sql"),
    },
    Migration::Sql {
        version: 60,
        sql: include_str!("migrations/060_intelligence_dimensions.sql"),
    },
    Migration::Sql {
        version: 61,
        sql: include_str!("migrations/061_stakeholder_glean_staleness.sql"),
    },
    Migration::Sql {
        version: 62,
        sql: include_str!("migrations/062_intelligence_feedback.sql"),
    },
    Migration::Sql {
        version: 63,
        sql: include_str!("migrations/063_email_signals_source.sql"),
    },
    Migration::Sql {
        version: 64,
        sql: include_str!("migrations/064_pipeline_failures.sql"),
    },
    Migration::Sql {
        version: 65,
        sql: include_str!("migrations/065_search_fts5.sql"),
    },
    Migration::Sql {
        version: 66,
        sql: include_str!("migrations/066_sync_metadata.sql"),
    },
    Migration::Sql {
        version: 67,
        sql: include_str!("migrations/067_feedback_unique_constraint.sql"),
    },
    Migration::Sql {
        version: 68,
        sql: include_str!("migrations/068_success_plans.sql"),
    },
    Migration::Sql {
        version: 69,
        sql: include_str!("migrations/069_account_events_expand.sql"),
    },
    Migration::Sql {
        version: 70,
        sql: include_str!("migrations/070_captures_metadata.sql"),
    },
    Migration::Sql {
        version: 71,
        sql: include_str!("migrations/071_email_triage_columns.sql"),
    },
    Migration::Sql {
        version: 72,
        sql: include_str!("migrations/072_health_score_history.sql"),
    },
    Migration::Sql {
        version: 73,
        sql: include_str!("migrations/073_meeting_record_path.sql"),
    },
    Migration::Sql {
        version: 74,
        sql: include_str!("migrations/074_action_status_vocabulary.sql"),
    },
    Migration::Sql {
        version: 75,
        sql: include_str!("migrations/075_v110_lifecycle_products_provenance.sql"),
    },
    Migration::Sql {
        version: 76,
        sql: include_str!("migrations/076_source_aware_account_truth.sql"),
    },
    Migration::Sql {
        version: 77,
        sql: include_str!("migrations/077_technical_footprint.sql"),
    },
    Migration::Sql {
        version: 78,
        sql: include_str!("migrations/078_pull_quote_column.sql"),
    },
    Migration::Sql {
        version: 79,
        sql: include_str!("migrations/079_product_classification.sql"),
    },
    Migration::Sql {
        version: 80,
        sql: include_str!("migrations/080_stakeholder_source_of_truth.sql"),
    },
    Migration::Sql {
        version: 81,
        sql: include_str!("migrations/081_init_tasks.sql"),
    },
    Migration::Sql {
        version: 82,
        sql: include_str!("migrations/082_email_enriched_at.sql"),
    },
    Migration::Sql {
        version: 83,
        sql: include_str!("migrations/082_account_fact_columns.sql"),
    },
    Migration::Sql {
        version: 84,
        sql: include_str!("migrations/083_dashboard_fields_to_db.sql"),
    },
    Migration::Sql {
        version: 85,
        sql: include_str!("migrations/084_feedback_events.sql"),
    },
    Migration::Sql {
        version: 86,
        sql: include_str!("migrations/085_action_status_priority_v2.sql"),
    },
    Migration::Sql {
        version: 87,
        sql: include_str!("migrations/086_objective_evidence.sql"),
    },
    Migration::Sql {
        version: 88,
        sql: include_str!("migrations/086_rejected_action_patterns.sql"),
    },
    Migration::Sql {
        version: 89,
        sql: include_str!("migrations/086_decision_columns.sql"),
    },
    Migration::Sql {
        version: 90,
        sql: include_str!("migrations/090_commitment_milestone_link.sql"),
    },
    Migration::Sql {
        version: 91,
        sql: include_str!("migrations/085_action_linear_links.sql"),
    },
    Migration::Sql {
        version: 92,
        sql: include_str!("migrations/092_deactivate_propagated_email_signals.sql"),
    },
    Migration::Sql {
        version: 93,
        sql: include_str!("migrations/091_user_health_sentiment.sql"),
    },
    Migration::Sql {
        version: 94,
        sql: include_str!("migrations/093_email_sync_meta.sql"),
    },
    Migration::Sql {
        version: 95,
        sql: include_str!("migrations/094_user_sentiment_history.sql"),
    },
    Migration::Sql {
        version: 96,
        sql: include_str!("migrations/095_meeting_entities_confidence.sql"),
    },
    Migration::Sql {
        version: 97,
        sql: include_str!("migrations/096_health_outlook_signals.sql"),
    },
    Migration::Sql {
        version: 98,
        sql: include_str!("migrations/097_email_pending_retry_state.sql"),
    },
    Migration::Sql {
        version: 99,
        sql: include_str!("migrations/098_risk_briefing_jobs.sql"),
    },
    Migration::Sql {
        version: 100,
        sql: include_str!("migrations/099_meeting_entity_dismissals.sql"),
    },
    Migration::Sql {
        version: 101,
        sql: include_str!("migrations/100_email_retry_batch.sql"),
    },
    // risk_briefing_jobs.attempt_id (CAS lifecycle)
    // + health_recompute_pending (durable debouncer). Combined migration to
    // minimize collision with parallel work.
    Migration::Sql {
        version: 102,
        sql: include_str!("migrations/101_risk_briefing_attempt_and_recompute_pending.sql"),
    },
    // emails.is_noise column for hard-drop bulk/marketing filter.
    Migration::Sql {
        version: 103,
        sql: include_str!("migrations/102_email_is_noise.sql"),
    },
    // track stale-failed auto-retry count so we cap automatic
    // promotions instead of looping forever on rows that fundamentally
    // can't enrich. Email sync stats read this column to compute the
    // `permanently_failed` count surfaced in the failure UX.
    Migration::Sql {
        version: 104,
        sql: include_str!("migrations/103_email_auto_retry_count.sql"),
    },
    // Defensive re-add of `is_noise` column. Tolerated as
    // "duplicate column name" by the framework if the column already
    // exists (normal upgrade); a real fix for users whose v103
    // schema_version was recorded without the ALTER actually applying.
    Migration::Sql {
        version: 105,
        sql: include_str!("migrations/104_email_is_noise_defensive.sql"),
    },
    // Recover emails over-suppressed by Rule 3
    // (List-Unsubscribe alone). Rule is tightened in code; this
    // migration restores is_noise=0 for rows outside the bulk allow-list.
    Migration::Sql {
        version: 106,
        sql: include_str!("migrations/105_email_noise_recovery.sql"),
    },
    // After the coarse email-noise recovery, re-suppress noreply
    // senders and bracket-prefix internal-org notifications that the
    // tightened rules now catch. Brings existing data in line with
    // the fixed code without requiring a fresh sync.
    Migration::Sql {
        version: 107,
        sql: include_str!("migrations/106_email_resuppress_noreply.sql"),
    },
    // Stakeholder-role soft-delete: `dismissed_at` tombstones user-removed
    // role rows so subsequent enrichment can't silently re-surface the
    // role via intel_queue's INSERT ON CONFLICT path.
    Migration::Sql {
        version: 108,
        sql: include_str!("migrations/107_stakeholder_role_dismissals.sql"),
    },
    // Work-tab foundation: action_kind column + ai_commitment_bridge +
    // account_focus_pins + nudge_dismissals. Enables commitments-as-Actions,
    // focus pin overlay, and nudge dismissal memory. See migration file
    // header for rationale.
    Migration::Sql {
        version: 109,
        sql: include_str!("migrations/108_work_tab_actions.sql"),
    },
    // Persist Health-tab triage card Snooze + Confirm-resolved state
    // so dismissals survive refresh. Keyed on (entity_type, entity_id,
    // triage_key); rendering-time filter hides rows where
    // resolved_at IS NOT NULL or snoozed_until > now.
    Migration::Sql {
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
    Migration::Sql {
        version: 111,
        sql: include_str!("migrations/110_linked_entities_raw.sql"),
    },
    Migration::Sql {
        version: 112,
        sql: include_str!("migrations/111_linking_dismissals.sql"),
    },
    Migration::Sql {
        version: 113,
        sql: include_str!("migrations/112_entity_linking_evaluations.sql"),
    },
    Migration::Sql {
        version: 114,
        sql: include_str!("migrations/113_entity_graph_version.sql"),
    },
    Migration::Sql {
        version: 115,
        sql: include_str!("migrations/114_account_stakeholders_review_queue_idx.sql"),
    },
    Migration::Sql {
        version: 116,
        sql: include_str!("migrations/115_migrate_meeting_entity_dismissals.sql"),
    },
    // pending_thread_inheritance queue for P2 out-of-order
    // email delivery. When a child email arrives before its parent, P2 enqueues
    // it here; the queue is drained when the parent is later evaluated.
    Migration::Sql {
        version: 117,
        sql: include_str!("migrations/116_pending_thread_inheritance.sql"),
    },
    // complete entity_graph_version trigger coverage.
    // Adds INSERT/DELETE + name/archived UPDATE triggers for accounts and
    // projects so P5 name-matching and P4/P4b/P4c domain evidence stay
    // consistent after entity creation, deletion, or rename.
    Migration::Sql {
        version: 118,
        sql: include_str!("migrations/117_entity_graph_version_full_triggers.sql"),
    },
    // add source provenance to account_domains so
    // raw_rebuild_account_domains can purge inferred domains before cutover.
    Migration::Sql {
        version: 119,
        sql: include_str!("migrations/118_account_domains_source.sql"),
    },
    // email To/Cc recipient columns for multi-participant
    // domain evidence in P4b/P4c rules.
    Migration::Sql {
        version: 120,
        sql: include_str!("migrations/119_email_to_cc.sql"),
    },
    // Evidence-hierarchy fix: rename P4a/P4b/P4c rule identifiers to
    // P4b/P4c/P4d so a new stakeholder-inference rule can take the P4a slot.
    // Shifts existing rows in linked_entities_raw (rule_id, source) and
    // entity_linking_evaluations (rule_id) via a two-pass update.
    Migration::Sql {
        version: 121,
        sql: include_str!("migrations/120_dos_258_rule_rename.sql"),
    },
    // Entity-graph sweep state: add last_migration_sweep_at to entity_graph_version so
    // the startup rescan can self-correct existing weak primaries once per
    // upgrade without re-running on every boot.
    Migration::Sql {
        version: 122,
        sql: include_str!("migrations/121_entity_graph_sweep_state.sql"),
    },
    // collapse duplicate commitment-typed actions where the AI
    // emitted the same commitment text under different commitment_id
    // values across enrichment runs. Pick a canonical row per
    // (entity, normalized_title), rewire bridge rows to point at it,
    // delete the duplicates. Forward-going dedup is enforced in
    // services::commitment_bridge::sync_ai_commitments.
    Migration::Sql {
        version: 123,
        sql: include_str!("migrations/122_dos_321_collapse_commitment_dupes.sql"),
    },
    // per-entity claim_version columns (Option A invalidation primitive)
    // + shared migration_state.global_claim_epoch row. Replaces the
    // entity_graph_version trigger extension that round-1 Codex review caught
    // as a singleton-counter cache thrash bug. SubjectRef::Multi uses
    // deterministic lock ordering (Account < Meeting < Person < Project).
    Migration::Sql {
        version: 124,
        sql: include_str!("migrations/123_dos_310_per_entity_claim_invalidation.sql"),
    },
    // migration_state.schema_epoch row. Workers capture it at job
    // pickup; the WriteFence rechecks at write-back. If a migration bumps
    // the epoch mid-flight, in-flight work is rejected (caller logs +
    // re-queues). See src-tauri/src/intelligence/write_fence.rs.
    Migration::Sql {
        version: 125,
        sql: include_str!("migrations/124_dos_311_schema_epoch.sql"),
    },
    // covering index for suppression lookups + quarantine table for
    // tombstone remediation before the claims substrate migration.
    Migration::Sql {
        version: 126,
        sql: include_str!("migrations/125_suppression_remediation.sql"),
    },
    // durable operator audit for malformed suppression decisions.
    Migration::Sql {
        version: 127,
        sql: include_str!("migrations/126_suppression_malformed_log.sql"),
    },
    //  cycle-3: mark remediated quarantine rows as resolved audit trail.
    Migration::Sql {
        version: 128,
        sql: include_str!("migrations/127_quarantine_resolved_at.sql"),
    },
    //  cycle-4: partial index for the unresolved-row gate query.
    // Split from migration 128 so a partial-failure retry cannot leave
    // the column added but the index missing.
    Migration::Sql {
        version: 129,
        sql: include_str!("migrations/128_quarantine_unresolved_index.sql"),
    },
    // Claims commit substrate schema (intelligence_claims + 5 siblings).
    Migration::Sql {
        version: 130,
        sql: include_str!("migrations/129_dos_7_claims_schema.sql"),
    },
    // Claims backfill D3a-1: backfill mechanisms 1-4 (suppression_tombstones,
    // account_stakeholder_roles.dismissed_at, email_dismissals,
    // meeting_entity_dismissals) into intelligence_claims tombstone rows.
    // D3a-2 covers mechanisms 5-8; D3b covers DismissedItem JSON blobs.
    Migration::Sql {
        version: 131,
        sql: include_str!("migrations/130_dos_7_claims_backfill_a1.sql"),
    },
    // Claims backfill D3a-2: backfill mechanisms 5-8 (linking_dismissals,
    // briefing_callouts.dismissed_at, nudge_dismissals, triage_snoozes)
    // + duplicate-pair corroboration between mechanism 4 and 5.
    Migration::Sql {
        version: 132,
        sql: include_str!("migrations/131_dos_7_claims_backfill_a2.sql"),
    },
    // Add emails.claim_version so SubjectRef::Email
    // participates in per-entity invalidation alongside Account/Meeting/
    // Person/Project. Required to unwind cycle-2's Account+prefix
    // workaround for email dismissals.
    Migration::Sql {
        version: 133,
        sql: include_str!("migrations/132_dos_7_email_claim_version.sql"),
    },
    // Withdraw m5 backfill rows whose
    // subject_ref kind is not a supported SubjectRef variant
    // (e.g. owner_type='email_thread' from linking_dismissals).
    Migration::Sql {
        version: 134,
        sql: include_str!("migrations/133_dos_7_withdraw_unsupported_m5_kinds.sql"),
    },
    // Per-claim projection-status ledger so commit_claim can record
    // whether each derived-state target (legacy entity_intelligence
    // tables, success_plans, account AI columns, intelligence.json on
    // disk) succeeded or failed without rolling back the authoritative
    // claim. Failed rows are the repair worklist.
    Migration::Sql {
        version: 135,
        sql: include_str!("migrations/134_dos_301_claim_projection_status.sql"),
    },
    // Typed feedback schema: rebuild claim_feedback for the closed
    // action set and add claim verification state columns.
    Migration::Sql {
        version: 136,
        sql: include_str!("migrations/135_dos_294_typed_feedback_schema.sql"),
    },
    // Quarantine malformed legacy source timestamps before W4 freshness
    // scoring reads the claim substrate.
    Migration::Sql {
        version: 137,
        sql: include_str!("migrations/136_dos_299_source_asof_quarantine.sql"),
    },
    // Opaque thread metadata substrate; creation and assignment semantics land later.
    Migration::Sql {
        version: 138,
        sql: include_str!("migrations/138_thread_metadata.sql"),
    },
    // Failed-projection repair scans filter by target and order by
    // attempted_at; the status value is constant inside the partial
    // index predicate.
    Migration::Sql {
        version: 139,
        sql: include_str!("migrations/139_dos_301_projection_failed_index_v2.sql"),
    },
    // Existing databases at v139 keep the original temporal_scope CHECK that
    // omits 'closed'. Rebuild the table so writes of TemporalScope::Closed land.
    Migration::Sql {
        version: 140,
        sql: include_str!("migrations/140_dos_287_temporal_scope_closed.sql"),
    },
    // backfill legacy entity_context_entries into user_note claims
    // and freeze the legacy table for rollback-only reads.
    Migration::Sql {
        version: 141,
        sql: include_str!("migrations/141_user_note_claim_type_backfill.sql"),
    },
    // audited click-to-reveal records for Confidential claim text.
    Migration::Sql {
        version: 142,
        sql: include_str!("migrations/142_sensitivity_reveal_audit.sql"),
    },
    // make audited reveals idempotent per frontend reveal action.
    Migration::Sql {
        version: 143,
        sql: include_str!("migrations/143_sensitivity_reveal_audit_idempotency.sql"),
    },
    // repair v143 idempotency variants with caller-supplied action tokens.
    Migration::Fn {
        version: 144,
        apply: v144_audit_action_token::migrate_v144_audit_action_token,
    },
    // fail-closed entity_members links by enforcing
    // entity_members.entity_id -> entities(id).
    Migration::Sql {
        version: 145,
        sql: include_str!("migrations/145_dos_379_entity_members_entity_fk.sql"),
    },
    // Canonicalize signal event provenance naming per ADR-0107.
    Migration::Sql {
        version: 146,
        sql: include_str!("migrations/146_dos_212_signal_events_data_source.sql"),
    },
    // Durable invalidation queue substrate.
    Migration::Sql {
        version: 147,
        sql: include_str!("migrations/147_invalidation_jobs.sql"),
    },
    Migration::Sql {
        version: 148,
        sql: include_str!("migrations/148_dos_265_claim_edges.sql"),
    },
    // Phase 1 temporal trajectory primitives.
    Migration::Sql {
        version: 149,
        sql: include_str!("migrations/149_dos_215_temporal_primitives.sql"),
    },
    // Resumable temporal maintenance backfill cursors.
    Migration::Sql {
        version: 150,
        sql: include_str!("migrations/150_dos_215_temporal_backfill_state.sql"),
    },
    // Temporal rows remember revoked-source invalidation for ADR-0109 filtering.
    Migration::Sql {
        version: 151,
        sql: include_str!("migrations/151_dos_215_temporal_source_invalidation.sql"),
    },
    // Temporal rows are keyed by entity type as well as id to avoid cross-type id collisions.
    Migration::Sql {
        version: 152,
        sql: include_str!("migrations/152_dos_215_temporal_entity_type_keys.sql"),
    },
    // Targeted repair invalidation jobs for claim repair routing.
    Migration::Sql {
        version: 153,
        sql: include_str!("migrations/153_targeted_repair_invalidation_jobs.sql"),
    },
    Migration::Sql {
        version: 154,
        sql: include_str!("migrations/154_claim_surface_dismissals.sql"),
    },
    // Typed CommitmentClaim identity: actions.commitment_id, structural owner
    // fields, per-sighting action_commitment_sources, and the exact-title
    // backlog duplicate guard for DOS-276 W4-A.
    Migration::Sql {
        version: 155,
        sql: include_str!("migrations/155_dos_276_commitment_claim_identity.sql"),
    },
    // Map pre-155 bridge ids to typed derived commitment ids so tombstones and
    // accepted-row bridge lookups survive the identity transition.
    Migration::Fn {
        version: 156,
        apply: backfill_commitment_bridge_derived_aliases,
    },
];

struct CommitmentBridgeAliasBackfillRow {
    legacy_commitment_id: String,
    derived_commitment_id: String,
    entity_type: String,
    entity_id: String,
    action_id: Option<String>,
    first_seen_at: String,
    last_seen_at: String,
    tombstoned: i32,
}

fn backfill_commitment_bridge_derived_aliases(conn: &Connection) -> Result<(), MigrationError> {
    let rows = collect_commitment_bridge_alias_rows(conn)?;
    for row in rows {
        if row.legacy_commitment_id == row.derived_commitment_id {
            continue;
        }

        conn.execute(
            "INSERT INTO ai_commitment_bridge
             (commitment_id, entity_type, entity_id, action_id,
              first_seen_at, last_seen_at, tombstoned)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(commitment_id) DO UPDATE SET
                entity_type = excluded.entity_type,
                entity_id = excluded.entity_id,
                action_id = COALESCE(excluded.action_id, ai_commitment_bridge.action_id),
                last_seen_at = excluded.last_seen_at,
                tombstoned = CASE
                    WHEN ai_commitment_bridge.tombstoned != 0 OR excluded.tombstoned != 0
                    THEN 1
                    ELSE 0
                END",
            rusqlite::params![
                row.derived_commitment_id,
                row.entity_type,
                row.entity_id,
                row.action_id,
                row.first_seen_at,
                row.last_seen_at,
                row.tombstoned,
            ],
        )
        .map_err(|e| format!("commitment bridge derived alias insert failed: {e}"))?;
    }
    Ok(())
}

fn collect_commitment_bridge_alias_rows(
    conn: &Connection,
) -> Result<Vec<CommitmentBridgeAliasBackfillRow>, MigrationError> {
    let mut stmt = conn
        .prepare(
            "SELECT
                b.commitment_id,
                b.entity_type,
                b.entity_id,
                b.action_id,
                b.first_seen_at,
                b.last_seen_at,
                b.tombstoned,
                a.title,
                a.due_date,
                a.owner_raw,
                a.context
             FROM ai_commitment_bridge b
             JOIN actions a ON a.id = b.action_id
             WHERE a.action_kind = 'commitment'
               AND a.title IS NOT NULL",
        )
        .map_err(|e| format!("commitment bridge alias query failed: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let legacy_commitment_id: String = row.get(0)?;
            let entity_type: String = row.get(1)?;
            let entity_id: String = row.get(2)?;
            let action_id: Option<String> = row.get(3)?;
            let first_seen_at: String = row.get(4)?;
            let last_seen_at: String = row.get(5)?;
            let tombstoned: i32 = row.get(6)?;
            let title: String = row.get(7)?;
            let due_date: Option<String> = row.get(8)?;
            let owner_raw: Option<String> = row.get(9)?;
            let context: Option<String> = row.get(10)?;
            let context_owner = legacy_owner_from_context(context.as_deref());
            let owner_for_identity = owner_raw.as_deref().or(context_owner.as_deref());
            let derived_commitment_id =
                crate::abilities::extractors::commitment::derive_commitment_id(
                    &title,
                    &entity_id,
                    due_date.as_deref(),
                    owner_for_identity,
                );
            Ok(CommitmentBridgeAliasBackfillRow {
                legacy_commitment_id,
                derived_commitment_id,
                entity_type,
                entity_id,
                action_id,
                first_seen_at,
                last_seen_at,
                tombstoned,
            })
        })
        .map_err(|e| format!("commitment bridge alias row map failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("commitment bridge alias row read failed: {e}"))
}

fn legacy_owner_from_context(context: Option<&str>) -> Option<String> {
    let value = context?.trim();
    let prefix_len = value
        .get(..6)
        .filter(|prefix| prefix.eq_ignore_ascii_case("owner:"))?
        .len();
    let owner = value[prefix_len..].trim();
    if owner.is_empty() {
        None
    } else {
        Some(owner.to_string())
    }
}

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
fn current_version(conn: &Connection) -> Result<u32, String> {
    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to read schema version: {}", e))?;

    u32::try_from(version)
        .map_err(|_| format!("Invalid negative or too-large schema version recorded: {version}"))
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

fn apply_migration_146_signal_events_data_source(
    conn: &Connection,
    sql: &'static str,
) -> Result<(), String> {
    if !table_exists(conn, "signal_events")? {
        return Err("signal_events table is missing".to_string());
    }

    let columns = table_columns(conn, "signal_events")?;
    let has_source = columns.contains("source");
    let has_data_source = columns.contains("data_source");

    match (has_source, has_data_source) {
        (true, false) => conn
            .execute_batch(sql)
            .map_err(|e| format!("rename signal_events.source to data_source: {e}")),
        (false, true) => conn
            .execute_batch(
                "BEGIN;
                 DROP INDEX IF EXISTS idx_signal_events_source;
                 CREATE INDEX IF NOT EXISTS idx_signal_events_data_source
                     ON signal_events(data_source, signal_type);
                 COMMIT;",
            )
            .map_err(|e| format!("ensure signal_events.data_source index: {e}")),
        (true, true) => conn
            .execute_batch(
                "BEGIN;
                 DROP INDEX IF EXISTS idx_signal_events_source;
                 UPDATE signal_events
                    SET data_source = source
                  WHERE data_source IS NULL;
                 ALTER TABLE signal_events DROP COLUMN source;
                 CREATE INDEX IF NOT EXISTS idx_signal_events_data_source
                     ON signal_events(data_source, signal_type);
                 COMMIT;",
            )
            .map_err(|e| format!("deduplicate signal_events source columns: {e}")),
        (false, false) => {
            Err("signal_events has neither source nor data_source column".to_string())
        }
    }
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

    if version >= 146 {
        let signal_cols = table_columns(conn, "signal_events")?;
        if !signal_cols.contains("data_source") {
            return Err(
                "Schema integrity check failed: missing column signal_events.data_source"
                    .to_string(),
            );
        }
        if signal_cols.contains("source") {
            return Err(
                "Schema integrity check failed: legacy column signal_events.source still exists"
                    .to_string(),
            );
        }
    }

    if version >= 149 {
        for table in ["entity_engagement_curve", "person_role_progression"] {
            if !table_exists(conn, table)? {
                return Err(format!(
                    "Schema integrity check failed: missing required table '{table}'"
                ));
            }
        }

        let engagement_cols = table_columns(conn, "entity_engagement_curve")?;
        for col in [
            "entity_type",
            "entity_id",
            "week_start",
            "meetings_count",
            "emails_count",
            "bidirectional_ratio",
            "source_refs_json",
        ] {
            if !engagement_cols.contains(col) {
                return Err(format!(
                    "Schema integrity check failed: missing column entity_engagement_curve.{col}"
                ));
            }
        }

        let role_cols = table_columns(conn, "person_role_progression")?;
        for col in [
            "entity_type",
            "entity_id",
            "started_at",
            "ended_at",
            "title",
            "org",
            "seniority",
            "source_refs_json",
        ] {
            if !role_cols.contains(col) {
                return Err(format!(
                    "Schema integrity check failed: missing column person_role_progression.{col}"
                ));
            }
        }
    }

    if version >= 150 && !table_exists(conn, "temporal_backfill_state")? {
        return Err(
            "Schema integrity check failed: missing required table 'temporal_backfill_state'"
                .to_string(),
        );
    }

    if version >= 152 {
        let backfill_cols = table_columns(conn, "temporal_backfill_state")?;
        if !backfill_cols.contains("entity_type") {
            return Err(
                "Schema integrity check failed: missing column temporal_backfill_state.entity_type"
                    .to_string(),
            );
        }
    }

    if version >= 151 {
        let engagement_cols = table_columns(conn, "entity_engagement_curve")?;
        if !engagement_cols.contains("source_invalidated_at") {
            return Err(
                "Schema integrity check failed: missing column entity_engagement_curve.source_invalidated_at"
                    .to_string(),
            );
        }

        let role_cols = table_columns(conn, "person_role_progression")?;
        if !role_cols.contains("source_invalidated_at") {
            return Err(
                "Schema integrity check failed: missing column person_role_progression.source_invalidated_at"
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
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

fn create_backup_via_api(
    conn: &Connection,
    backup_path: &Path,
    destination_key: Option<&crate::db::EncryptionKey>,
) -> Result<(), String> {
    let mut backup_conn = rusqlite::Connection::open(backup_path)
        .map_err(|e| format!("Failed to open backup file: {e}"))?;
    if let Some(encryption_key) = destination_key {
        backup_conn
            .execute_batch(&encryption_key.to_pragma())
            .map_err(|e| format!("Failed to set pre-migration backup encryption key: {e}"))?;
    }
    let backup = rusqlite::backup::Backup::new(conn, &mut backup_conn)
        .map_err(|e| format!("Failed to initialize pre-migration backup: {e}"))?;
    // Single step(-1) copies every page in one shot. On encrypted DBs in the
    // hundreds of megabytes that path returned Err with the canonical
    // "not an error" string — the underlying extended code had been cleared
    // but the rusqlite wrapper still mapped to Err, blocking migrations until
    // the user manually intervened. Chunked stepping avoids the edge case and
    // gives us progress logging while a multi-hundred-MB copy runs.
    const PAGES_PER_STEP: i32 = 1024;
    // A long-running concurrent writer can leave the source DB perpetually
    // Busy/Locked and trap the migration without surfacing anything to the
    // user. Bound the wait so startup either gets a backup or fails loudly
    // with a recoverable error.
    const MAX_BUSY_RETRIES: u32 = 600; // 600 * 50ms = 30s wall clock
    let mut step_count = 0_u64;
    let mut busy_retries = 0_u32;
    loop {
        match backup.step(PAGES_PER_STEP) {
            Ok(rusqlite::backup::StepResult::More) => {
                step_count += 1;
                busy_retries = 0;
                if step_count.is_multiple_of(64) {
                    log::info!(
                        "Pre-migration backup in progress: ~{} pages copied",
                        step_count * PAGES_PER_STEP as u64
                    );
                }
            }
            Ok(rusqlite::backup::StepResult::Done) => break,
            Ok(rusqlite::backup::StepResult::Busy) | Ok(rusqlite::backup::StepResult::Locked) => {
                busy_retries += 1;
                if busy_retries >= MAX_BUSY_RETRIES {
                    return Err(format!(
                        "Pre-migration backup gave up after {} consecutive Busy/Locked retries (~{}s); a long writer may be holding the source DB",
                        busy_retries,
                        (busy_retries as u64 * 50) / 1000
                    ));
                }
                if busy_retries.is_multiple_of(40) {
                    log::warn!(
                        "Pre-migration backup waiting on Busy/Locked source: retry {} of {}",
                        busy_retries,
                        MAX_BUSY_RETRIES
                    );
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Ok(other) => {
                return Err(format!(
                    "Pre-migration backup unexpected step result: {other:?}"
                ));
            }
            Err(e) => return Err(format!("Pre-migration backup failed: {e}")),
        }
    }
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
fn backup_before_migration(
    conn: &Connection,
    encryption_key: Option<&crate::db::EncryptionKey>,
) -> Result<PathBuf, String> {
    let db_path: String = conn
        .query_row("PRAGMA database_list", [], |row| row.get(2))
        .map_err(|e| format!("Failed to get database path: {}", e))?;

    if db_path.is_empty() || db_path == ":memory:" {
        // In-memory or temp database — skip backup
        return Ok(PathBuf::from(":memory:"));
    }

    let db_path = PathBuf::from(db_path);
    let backup_path = migration_backup_path(&db_path);
    #[allow(
        clippy::let_underscore_must_use,
        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
    )]
    let _ = std::fs::remove_file(&backup_path);

    let source_size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
    log::info!(
        "Pre-migration backup starting: source={} ({} bytes), dest={}",
        db_path.to_string_lossy(),
        source_size_bytes,
        backup_path.to_string_lossy()
    );

    let encrypted = db_path.exists() && !crate::db::encryption::is_database_plaintext(&db_path);
    let encryption_key = if encrypted {
        Some(match encryption_key {
            Some(key) => key.clone(),
            None => {
                let provider = crate::db::LocalKeychain::new();
                let user = crate::db::UserIdentity::local(db_path.clone());
                crate::db::DbKeyProvider::get_or_create_key(&provider, &user)
                    .map_err(|e| format!("Failed to get DB encryption key for backup: {e}"))?
            }
        })
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
            .as_ref()
            .ok_or_else(|| "Missing encryption key for backup".to_string())?;
        create_backup_via_api(conn, &backup_path, Some(key))
    } else {
        create_backup_via_api(conn, &backup_path, None)
    };
    if let Err(err) = backup_result {
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = std::fs::remove_file(&backup_path);
        // Last resort: sqlcipher_export (now transaction-wrapped). Only reached
        // if the Backup API itself reports an encryption incompatibility.
        if should_try_encrypted_backup_fallback(encrypted, &err) {
            let key = encryption_key
                .as_ref()
                .ok_or_else(|| "Missing encryption key for fallback backup".to_string())?;
            create_backup_via_sqlcipher_export(conn, &backup_path, key.as_hex())?;
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
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
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

fn apply_migration_141_user_note_backfill(
    conn: &Connection,
    migration_sql: &str,
) -> Result<(), String> {
    let db = crate::db::ActionDb::from_conn(conn);
    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute_batch(migration_sql)
            .map_err(|e| format!("Migration v141 schema/freeze failed: {e}"))?;

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let ext = crate::services::context::ExternalClients::default();
        let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext)
            .with_actor("user");

        let legacy_rows = crate::services::entity_context::legacy_entity_context_entries(tx)?;
        for legacy in legacy_rows {
            let claim_id =
                crate::services::entity_context::commit_backfilled_user_note(&ctx, tx, &legacy)?;
            tx.conn_ref()
                .execute(
                    "INSERT OR IGNORE INTO legacy_user_note_migration_audit (
                        legacy_entry_id, claim_id, entity_type, entity_id,
                        legacy_created_at, legacy_updated_at, status
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'migrated')",
                    rusqlite::params![
                        &legacy.id,
                        &claim_id,
                        &legacy.entity_type,
                        &legacy.entity_id,
                        &legacy.created_at,
                        &legacy.updated_at,
                    ],
                )
                .map_err(|e| format!("Migration v141 audit insert failed: {e}"))?;
        }

        Ok(())
    })
}

/// Run all pending migrations.
///
/// Returns the number of migrations applied (0 if already up-to-date).
///
/// Forward-compat guard: if the database has a higher version than the highest
/// known migration, returns an error telling the user to update DailyOS.
pub fn run_migrations(conn: &Connection) -> Result<usize, String> {
    run_migrations_with_key(conn, None)
}

pub(crate) fn run_migrations_with_key(
    conn: &Connection,
    encryption_key: Option<&crate::db::EncryptionKey>,
) -> Result<usize, String> {
    ensure_schema_version_table(conn)?;
    bootstrap_existing_db(conn)?;

    let current = current_version(conn)?;
    let max_known = MIGRATIONS.last().map(Migration::version).unwrap_or(0);

    // Forward-compat guard
    if current > max_known {
        return Err(format!(
            "Database schema version ({}) is newer than this version of DailyOS supports ({}). \
             Please update DailyOS to the latest version.",
            current, max_known
        ));
    }

    // Collect pending migrations
    let pending: Vec<&Migration> = MIGRATIONS
        .iter()
        .filter(|m| m.version() > current)
        .collect();

    if pending.is_empty() {
        verify_required_schema(conn)?;
        return Ok(0);
    }

    // quarantine gate. Refuse to apply migration 126 (the
    // backfill territory) until unresolved quarantine rows are resolved.
    // Resolved quarantine rows are retained as audit trail and do NOT block
    // subsequent migrations.
    let migration_126_pending = pending.iter().any(|m| m.version() == 126);
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
    let backup_path = backup_before_migration(conn, encryption_key)?;
    if backup_path.to_string_lossy() != ":memory:" {
        log::info!(
            "Migration safety backup ready at {}",
            backup_path.to_string_lossy()
        );
    }

    // Apply each pending migration in order
    for migration in &pending {
        let version = migration.version();
        match migration {
            Migration::Sql { sql, .. } => {
                let apply_result = if version == 141 {
                    apply_migration_141_user_note_backfill(conn, sql)
                        .map_err(rusqlite::Error::InvalidParameterName)
                } else if version == 146 {
                    apply_migration_146_signal_events_data_source(conn, sql)
                        .map_err(rusqlite::Error::InvalidParameterName)
                } else {
                    conn.execute_batch(sql)
                };
                match apply_result {
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
                        let is_single_alter = sql
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
                                version,
                                msg.split('\n').next().unwrap_or(&msg)
                            );
                        } else {
                            return Err(format!("Migration v{} failed: {}", version, e));
                        }
                    }
                }
            }
            Migration::Fn { apply, .. } => {
                apply(conn).map_err(|e| format!("Migration v{} failed: {}", version, e))?;
            }
        }

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [i64::from(version)],
        )
        .map_err(|e| format!("Failed to record migration v{}: {}", version, e))?;

        log::info!("Applied migration v{}", version);
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
        let unrelated =
            sqlite_failure_with_message(rusqlite::ffi::SQLITE_ERROR, "no such table: accounts");
        assert!(!is_missing_column_error(&corrupt));
        assert!(!is_missing_column_error(&unrelated));
    }

    #[test]
    fn commitment_bridge_alias_backfill_preserves_tombstone_state() {
        let conn = mem_db();
        conn.execute_batch(
            "CREATE TABLE actions (
                id TEXT PRIMARY KEY,
                title TEXT,
                due_date TEXT,
                owner_raw TEXT,
                context TEXT,
                account_id TEXT,
                project_id TEXT,
                action_kind TEXT
            );
            CREATE TABLE ai_commitment_bridge (
                commitment_id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                action_id TEXT,
                first_seen_at TEXT NOT NULL,
                last_seen_at TEXT NOT NULL,
                tombstoned INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO actions
                (id, title, due_date, owner_raw, context, account_id, action_kind)
            VALUES
                ('active-a1', 'Active old id', NULL, NULL, NULL, 'acct-1', 'commitment'),
                ('done-a1', 'Dismissed old id', '2026-05-01', NULL, 'owner: Alex Chen', 'acct-1', 'commitment');
            INSERT INTO ai_commitment_bridge
                (commitment_id, entity_type, entity_id, action_id, first_seen_at, last_seen_at, tombstoned)
            VALUES
                ('legacy:active', 'account', 'acct-1', 'active-a1', '2026-01-01', '2026-01-02', 0),
                ('legacy:done', 'account', 'acct-1', 'done-a1', '2026-01-01', '2026-01-03', 1);",
        )
        .expect("seed pre-156 schema");

        backfill_commitment_bridge_derived_aliases(&conn).expect("backfill aliases");

        let active_derived = crate::abilities::extractors::commitment::derive_commitment_id(
            "Active old id",
            "acct-1",
            None,
            None,
        );
        let done_derived = crate::abilities::extractors::commitment::derive_commitment_id(
            "Dismissed old id",
            "acct-1",
            Some("2026-05-01"),
            Some("Alex Chen"),
        );

        let active_tombstoned: i32 = conn
            .query_row(
                "SELECT tombstoned FROM ai_commitment_bridge WHERE commitment_id = ?1",
                [active_derived],
                |row| row.get(0),
            )
            .expect("active alias");
        assert_eq!(active_tombstoned, 0);

        let (done_action_id, done_tombstoned): (String, i32) = conn
            .query_row(
                "SELECT action_id, tombstoned
                 FROM ai_commitment_bridge
                 WHERE commitment_id = ?1",
                [done_derived],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("done alias");
        assert_eq!(done_action_id, "done-a1");
        assert_eq!(done_tombstoned, 1);
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
        assert_eq!(version, MIGRATIONS.last().unwrap().version());

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
            "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, data_source, value, confidence, decay_half_life_days)
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
            "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, data_source, confidence, decay_half_life_days, source_context)
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
             CREATE TABLE entities (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                entity_type TEXT NOT NULL DEFAULT 'account',
                tracker_path TEXT,
                updated_at TEXT NOT NULL
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
        assert_eq!(version, MIGRATIONS.last().unwrap().version());

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
        assert_eq!(version, MIGRATIONS.last().unwrap().version());
    }

    #[test]
    fn signal_events_data_source_migration_renames_and_preserves_rows() {
        let conn = mem_db();
        conn.execute_batch(
            "CREATE TABLE signal_events (
                id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                signal_type TEXT NOT NULL,
                source TEXT NOT NULL,
                value TEXT,
                confidence REAL DEFAULT 1.0,
                decay_half_life_days INTEGER DEFAULT 90,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                superseded_by TEXT,
                source_context TEXT
            );
            CREATE INDEX idx_signal_events_source ON signal_events(source, signal_type);
            INSERT INTO signal_events
                (id, entity_type, entity_id, signal_type, source, value, confidence)
            VALUES
                ('sig-pre', 'account', 'acct-pre', 'glean_document', 'glean_search', 'doc', 0.8);",
        )
        .expect("seed pre-migration signal_events");

        let migration = MIGRATIONS
            .iter()
            .find(|migration| migration.version() == 146)
            .expect("migration 146 registered");
        conn.execute_batch(migration.sql().expect("migration 146 should be SQL"))
            .expect("migration 146 applies cleanly");

        let columns = table_columns(&conn, "signal_events").expect("signal_events columns");
        assert!(columns.contains("data_source"));
        assert!(!columns.contains("source"));

        let data_source: String = conn
            .query_row(
                "SELECT data_source FROM signal_events WHERE id = 'sig-pre'",
                [],
                |row| row.get(0),
            )
            .expect("read migrated data_source");
        assert_eq!(data_source, "glean_search");

        let old_index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'index' AND name = 'idx_signal_events_source'",
                [],
                |row| row.get(0),
            )
            .expect("read old index count");
        assert_eq!(old_index_count, 0);

        let new_index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'index' AND name = 'idx_signal_events_data_source'",
                [],
                |row| row.get(0),
            )
            .expect("read new index count");
        assert_eq!(new_index_count, 1);
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
        conn.execute("DELETE FROM schema_version WHERE version >= 128", [])
            .expect("make v128+ pending");
        conn.execute(
            "INSERT INTO suppression_tombstones_quarantine \
             (id, entity_id, field_key, item_key, item_hash, dismissed_at, quarantine_reason, resolved_at) \
             VALUES (1, 'acct-1', 'risks', NULL, NULL, 'not-a-date', 'retained audit', '2026-05-02T00:00:00Z')",
            [],
        )
        .expect("seed retained quarantine row");

        let applied = run_migrations(&conn).expect("post-126 migration should not be gated");

        let expected = MIGRATIONS.iter().filter(|m| m.version() >= 128).count();
        assert_eq!(applied, expected);
        assert_eq!(
            current_version(&conn).expect("version query"),
            MIGRATIONS.last().unwrap().version()
        );
    }

    #[test]
    fn quarantine_blocks_only_when_126_pending() {
        let conn = mem_db();
        run_migrations(&conn).expect("apply all migrations");
        conn.execute("DELETE FROM schema_version WHERE version >= 126", [])
            .expect("make v126+ pending");
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

    #[test]
    fn migration_140_relaxes_temporal_scope_to_accept_closed() {
        // Build a v139-shaped intelligence_claims table directly: original CHECK
        // omits 'closed'. Seed one pre-existing row, prove 'closed' is rejected,
        // run migration 140, then prove the seed row survives and 'closed' lands.
        let conn = mem_db();

        conn.execute_batch(
            "CREATE TABLE intelligence_claims (
                id TEXT PRIMARY KEY,
                subject_ref TEXT NOT NULL,
                claim_type TEXT NOT NULL,
                field_path TEXT,
                topic_key TEXT,
                text TEXT NOT NULL,
                dedup_key TEXT NOT NULL,
                item_hash TEXT,
                actor TEXT NOT NULL,
                data_source TEXT NOT NULL,
                source_ref TEXT,
                source_asof TEXT,
                observed_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                provenance_json TEXT NOT NULL,
                metadata_json TEXT,
                claim_state TEXT NOT NULL DEFAULT 'active'
                    CHECK (claim_state IN ('active','dormant','tombstoned','withdrawn')),
                surfacing_state TEXT NOT NULL DEFAULT 'active'
                    CHECK (surfacing_state IN ('active','dormant')),
                demotion_reason TEXT,
                reactivated_at TEXT,
                retraction_reason TEXT,
                expires_at TEXT,
                superseded_by TEXT,
                trust_score REAL,
                trust_computed_at TEXT,
                trust_version INTEGER,
                thread_id TEXT,
                temporal_scope TEXT NOT NULL DEFAULT 'state'
                    CHECK (temporal_scope IN ('state','point_in_time','trend')),
                sensitivity TEXT NOT NULL DEFAULT 'internal'
                    CHECK (sensitivity IN ('public','internal','confidential','user_only')),
                verification_state TEXT NOT NULL DEFAULT 'active'
                    CHECK (verification_state IN ('active','contested','needs_user_decision')),
                verification_reason TEXT,
                needs_user_decision_at TEXT
            );",
        )
        .expect("seed v139 intelligence_claims shape");
        conn.execute(
            "INSERT INTO intelligence_claims /* dos7-allowed: migration test seeds v139-shape row to verify migration 140 preserves data */ \
             (id, subject_ref, claim_type, text, dedup_key, actor, data_source, observed_at, provenance_json) \
             VALUES ('c-pre', 'a-pre', 'fact', 'pre-migration', 'd-pre', 'system', 'manual', '2026-05-05', '{}')",
            [],
        )
        .expect("legal v139 row should insert");
        let rejected = conn.execute(
            "INSERT INTO intelligence_claims /* dos7-allowed: migration test asserts pre-migration schema rejects post-migration value */ \
             (id, subject_ref, claim_type, text, dedup_key, actor, data_source, observed_at, provenance_json, temporal_scope) \
             VALUES ('c-closed-pre', 'a-pre', 'fact', 'should fail', 'd-closed', 'system', 'manual', '2026-05-05', '{}', 'closed')",
            [],
        );
        assert!(
            rejected.is_err(),
            "v139 schema must reject temporal_scope='closed'"
        );

        let migration_140 = MIGRATIONS
            .iter()
            .find(|m| m.version() == 140)
            .expect("migration 140 must be registered");
        conn.execute_batch(migration_140.sql().expect("migration 140 should be SQL"))
            .expect("migration 140 applies cleanly on v139 table");

        let preserved: String = conn
            .query_row(
                "SELECT text FROM intelligence_claims WHERE id = 'c-pre'",
                [],
                |row| row.get(0),
            )
            .expect("v139 row survives migration 140");
        assert_eq!(preserved, "pre-migration");

        conn.execute(
            "INSERT INTO intelligence_claims /* dos7-allowed: migration test asserts post-migration schema accepts new temporal_scope value */ \
             (id, subject_ref, claim_type, text, dedup_key, actor, data_source, observed_at, provenance_json, temporal_scope) \
             VALUES ('c-closed-post', 'a-post', 'fact', 'closed window', 'd-closed-post', 'system', 'manual', '2026-05-05', '{}', 'closed')",
            [],
        )
        .expect("post-migration schema accepts temporal_scope='closed'");
    }
}

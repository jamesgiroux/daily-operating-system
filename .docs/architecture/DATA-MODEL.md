# Data Model Reference

**Auto-generated:** 2026-05-11 by `.docs/generators/gen-data-model.sh`

**Database:** SQLite (SQLCipher-encrypted, WAL mode)
**Migrations:** 153 files (`001_baseline.sql` through `154_claim_surface_dismissals.sql`)
**DB modules:** `src-tauri/src/db/`

---

## Table Inventory

| Table | Created In | Columns Added Later |
|-------|-----------|-------------------|
| `account_domains` | `002_internal_teams` | 118_account_domains_source |
| `account_domains_new` | `010_foreign_keys` | — |
| `account_events` | `001_baseline` | — |
| `account_events_new` | `069_account_events_expand` | — |
| `account_focus_pins` | `108_work_tab_actions` | — |
| `account_milestones` | `068_success_plans` | 075_v110_lifecycle_products_provenance |
| `account_milestones_new` | `069_account_events_expand` | — |
| `account_objectives` | `068_success_plans` | 086_objective_evidence |
| `account_objectives_new` | `069_account_events_expand` | — |
| `account_products` | `075_v110_lifecycle_products_provenance` | 079_product_classification |
| `account_source_refs` | `076_source_aware_account_truth` | — |
| `account_stakeholder_roles` | `080_stakeholder_source_of_truth` | 107_stakeholder_role_dismissals |
| `account_stakeholders` | `055_schema_decomposition` | 061_stakeholder_glean_staleness, 114_account_stakeholders_review_queue_idx |
| `account_stakeholders_new` | `056_account_stakeholders_data_source` | — |
| `account_team` | `003_account_team` | — |
| `account_team_import_notes` | `003_account_team` | — |
| `account_team_new` | `010_foreign_keys` | — |
| `account_technical_footprint` | `077_technical_footprint` | — |
| `accounts` | `001_baseline` | 002_internal_teams, 017_entity_keywords, 025_entity_metadata, 036_account_type, 053_app_state_demo, 075_v110_lifecycle_products_provenance, 076_source_aware_account_truth, 082_account_fact_columns, 083_dashboard_fields_to_db, 091_user_health_sentiment, 123_dos_310_per_entity_claim_invalidation |
| `accounts_new` | `003_account_team` | — |
| `action_linear_links` | `085_action_linear_links` | — |
| `action_objective_links` | `068_success_plans` | — |
| `actions` | `001_baseline` | 022_rejection_signals, 053_app_state_demo, 086_decision_columns, 108_work_tab_actions |
| `actions_backup` | `011_proposed_actions` | — |
| `actions_new` | `010_foreign_keys` | — |
| `agent_trust_ledger` | `129_dos_7_claims_schema` | — |
| `ai_commitment_bridge` | `108_work_tab_actions` | — |
| `app_state` | `053_app_state_demo` | — |
| `app_state_kv` | `057_intelligence_db_columns` | — |
| `attendee_display_names` | `026_attendee_display_names` | — |
| `attendee_group_patterns` | `019_correction_learning` | — |
| `briefing_callouts` | `020_signal_propagation` | — |
| `captured_commitments` | `068_success_plans` | 090_commitment_milestone_link |
| `captures` | `001_baseline` | — |
| `captures_new` | `070_captures_metadata` | — |
| `chat_sessions` | `007_chat_interface` | — |
| `chat_turns` | `007_chat_interface` | — |
| `claim_contradictions` | `129_dos_7_claims_schema` | — |
| `claim_corroborations` | `129_dos_7_claims_schema` | — |
| `claim_edges` | `148_dos_265_claim_edges` | — |
| `claim_feedback` | `129_dos_7_claims_schema` | — |
| `claim_feedback_new` | `135_dos_294_typed_feedback_schema` | — |
| `claim_projection_status` | `134_dos_301_claim_projection_status` | — |
| `claim_repair_job` | `129_dos_7_claims_schema` | — |
| `claim_surface_dismissals` | `154_claim_surface_dismissals` | — |
| `clay_sync_state` | `016_clay_enrichment` | — |
| `content_embeddings` | `006_content_embeddings` | — |
| `content_index` | `001_baseline` | 006_content_embeddings, 009_fix_embeddings_column |
| `context_mode_config` | `052_glean_document_cache` | — |
| `drive_watched_sources` | `048_google_drive_sync` | — |
| `email_dismissals` | `030_email_dismissals` | — |
| `email_signals` | `005_email_signals` | 034_emails |
| `email_signals_new` | `063_email_signals_source` | — |
| `email_sync_meta` | `093_email_sync_meta` | — |
| `email_threads` | `027_email_threads` | — |
| `emails` | `034_emails` | 035_email_relevance_score, 071_email_triage_columns, 082_email_enriched_at, 100_email_retry_batch, 102_email_is_noise, 103_email_auto_retry_count, 104_email_is_noise_defensive, 119_email_to_cc, 132_dos_7_email_claim_version |
| `emails_new` | `097_email_pending_retry_state` | — |
| `enrichment_log` | `016_clay_enrichment` | — |
| `entities` | `001_baseline` | 095_meeting_entities_confidence |
| `entity_assessment` | `055_schema_decomposition` | 057_intelligence_db_columns, 060_intelligence_dimensions, 078_pull_quote_column, 096_health_outlook_signals |
| `entity_assessment_new` | `058_health_schema_evolution` | — |
| `entity_context_entries` | `051_entity_context_entries` | — |
| `entity_context_entries_frozen_dos` | `141_user_note_claim_type_backfill` | — |
| `entity_email_cadence` | `028_entity_email_cadence` | — |
| `entity_engagement_curve` | `149_dos_215_temporal_primitives` | — |
| `entity_feedback_events` | `084_feedback_events` | — |
| `entity_graph_version` | `113_entity_graph_version` | 121_entity_graph_sweep_state |
| `entity_intelligence` | `001_baseline` | 040_entity_quality, 045_intelligence_report_fields, 047_entity_intel_user_relevance, 054_intelligence_consistency_metadata |
| `entity_linking_evaluations` | `112_entity_linking_evaluations` | — |
| `entity_members` | `055_schema_decomposition` | — |
| `entity_members_migration_` | `145_dos_379_entity_members_entity_fk` | — |
| `entity_members_new` | `145_dos_379_entity_members_entity_fk` | — |
| `entity_people` | `001_baseline` | — |
| `entity_quality` | `040_entity_quality` | — |
| `entity_quality_new` | `055_schema_decomposition` | — |
| `entity_resolution_feedback` | `019_correction_learning` | — |
| `glean_document_cache` | `052_glean_document_cache` | — |
| `gravatar_cache` | `015_gravatar_cache` | — |
| `health_recompute_pending` | `101_risk_briefing_attempt_and_recompute_pending` | — |
| `health_score_history` | `072_health_score_history` | — |
| `hygiene_actions_log` | `029_hygiene_actions_log` | — |
| `init_tasks` | `081_init_tasks` | — |
| `intelligence_claims` | `129_dos_7_claims_schema` | 135_dos_294_typed_feedback_schema |
| `intelligence_claims_new` | `140_dos_287_temporal_scope_closed` | — |
| `intelligence_feedback` | `062_intelligence_feedback` | — |
| `intelligence_feedback_new` | `067_feedback_unique_constraint` | — |
| `invalidation_jobs` | `147_invalidation_jobs` | — |
| `invalidation_jobs_v` | `153_targeted_repair_invalidation_jobs` | — |
| `legacy_user_note_migration_audit` | `141_user_note_claim_type_backfill` | — |
| `lifecycle_changes` | `075_v110_lifecycle_products_provenance` | — |
| `linear_entity_links` | `041_linear_entity_links` | — |
| `linear_issues` | `024_linear_sync` | — |
| `linear_projects` | `024_linear_sync` | — |
| `linked_entities_raw` | `110_linked_entities_raw` | — |
| `linking_dismissals` | `111_linking_dismissals` | — |
| `meeting_attendees` | `001_baseline` | — |
| `meeting_attendees_new` | `032_junction_fks_and_expr_indexes` | — |
| `meeting_champion_health` | `070_captures_metadata` | — |
| `meeting_entities` | `001_baseline` | 095_meeting_entities_confidence |
| `meeting_entities_new` | `032_junction_fks_and_expr_indexes` | — |
| `meeting_entity_dismissals` | `099_meeting_entity_dismissals` | — |
| `meeting_interaction_dynamics` | `070_captures_metadata` | — |
| `meeting_prep` | `055_schema_decomposition` | — |
| `meeting_prep_state` | `001_baseline` | — |
| `meeting_role_changes` | `070_captures_metadata` | — |
| `meeting_transcripts` | `055_schema_decomposition` | 073_meeting_record_path |
| `meetings` | `055_schema_decomposition` | 031_intelligence_lifecycle, 123_dos_310_per_entity_claim_invalidation |
| `meetings_history` | `001_baseline` | 031_intelligence_lifecycle |
| `meetings_history_new` | `023_drop_meeting_account_id` | — |
| `migration_state` | `123_dos_310_per_entity_claim_invalidation` | — |
| `nudge_dismissals` | `108_work_tab_actions` | — |
| `pending_thread_inheritance` | `116_pending_thread_inheritance` | — |
| `people` | `001_baseline` | 016_clay_enrichment, 053_app_state_demo, 123_dos_310_per_entity_claim_invalidation |
| `person_emails` | `012_person_emails` | — |
| `person_relationships` | `038_person_relationships` | 059_person_relationships_rationale |
| `person_relationships_new` | `039_person_relationships_types` | — |
| `person_role_progression` | `149_dos_215_temporal_primitives` | — |
| `pipeline_failures` | `064_pipeline_failures` | — |
| `post_meeting_emails` | `020_signal_propagation` | — |
| `proactive_insights` | `021_proactive_surfacing` | — |
| `proactive_scan_state` | `021_proactive_surfacing` | — |
| `processing_log` | `001_baseline` | — |
| `projects` | `001_baseline` | 017_entity_keywords, 025_entity_metadata, 037_project_hierarchy, 083_dashboard_fields_to_db, 123_dos_310_per_entity_claim_invalidation |
| `quill_sync_state` | `013_quill_sync` | — |
| `quill_sync_state_new` | `055_schema_decomposition` | — |
| `rejected_action_patterns` | `086_rejected_action_patterns` | — |
| `reports` | `050_reports` | — |
| `risk_briefing_jobs` | `098_risk_briefing_jobs` | 101_risk_briefing_attempt_and_recompute_pending |
| `sensitivity_reveal_audit` | `142_sensitivity_reveal_audit` | — |
| `signal_derivations` | `020_signal_propagation` | — |
| `signal_events` | `018_signal_bus` | 019_correction_learning |
| `signal_weights` | `018_signal_bus` | — |
| `source_asof_backfill_quarantine` | `136_dos_299_source_asof_quarantine` | — |
| `stakeholder_suggestions` | `080_stakeholder_source_of_truth` | — |
| `suppression_malformed_log` | `126_suppression_malformed_log` | — |
| `suppression_tombstones` | `084_feedback_events` | 127_quarantine_resolved_at |
| `suppression_tombstones_quarantine` | `125_suppression_remediation` | 127_quarantine_resolved_at |
| `sync_metadata` | `066_sync_metadata` | — |
| `temporal_backfill_state` | `150_dos_215_temporal_backfill_state` | — |
| `thread_metadata` | `138_thread_metadata` | — |
| `triage_snoozes` | `109_triage_snoozes` | — |
| `user_context_entries` | `044_user_entity` | 046_user_context_embedding |
| `user_entity` | `044_user_entity` | — |
| `user_sentiment_history` | `094_user_sentiment_history` | — |

---

## Table Details

### `account_domains`

**Created in:** `002_internal_teams`

| Column | Definition |
|--------|-----------|
| `account_id` | TEXT NOT NULL |
| `domain` | TEXT NOT NULL |
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `domain` | TEXT NOT NULL |
- `source` *(added in 118_account_domains_source)*

**Indexes:** idx_account_domains_domain

---

### `account_domains_new`

**Created in:** `010_foreign_keys`

| Column | Definition |
|--------|-----------|
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `domain` | TEXT NOT NULL |

---

### `account_events`

**Created in:** `001_baseline`

| Column | Definition |
|--------|-----------|
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `account_id` | TEXT NOT NULL |
| `event_type` | TEXT NOT NULL CHECK(event_type IN ('renewal', 'expansion', 'churn', 'downgrade')) |
| `event_date` | TEXT NOT NULL |
| `arr_impact` | REAL |
| `notes` | TEXT |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `account_id` | TEXT NOT NULL |
| `event_type` | TEXT NOT NULL CHECK(event_type IN ('renewal', 'expansion', 'churn', 'downgrade')) |
| `event_date` | TEXT NOT NULL |
| `arr_impact` | REAL |
| `notes` | TEXT |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `account_id` | TEXT NOT NULL |
| `event_type` | TEXT NOT NULL CHECK(event_type IN ( |
| `event_date` | TEXT NOT NULL |
| `arr_impact` | REAL |
| `notes` | TEXT |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

**Indexes:** idx_account_events_account, idx_account_events_date

---

### `account_events_new`

**Created in:** `069_account_events_expand`

| Column | Definition |
|--------|-----------|
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `account_id` | TEXT NOT NULL |
| `event_type` | TEXT NOT NULL CHECK(event_type IN ( |
| `event_date` | TEXT NOT NULL |
| `arr_impact` | REAL |
| `notes` | TEXT |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `account_focus_pins`

**Created in:** `108_work_tab_actions`

| Column | Definition |
|--------|-----------|
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `action_id` |  TEXT NOT NULL REFERENCES actions(id) ON DELETE CASCADE |
| `rank` |       INTEGER NOT NULL |
| `pinned_at` |  TEXT NOT NULL |

---

### `account_milestones`

**Created in:** `068_success_plans`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `objective_id` | TEXT NOT NULL REFERENCES account_objectives(id) ON DELETE CASCADE |
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `title` | TEXT NOT NULL |
| `status` | TEXT NOT NULL DEFAULT 'pending' |
| `target_date` | TEXT |
| `completed_at` | TEXT |
| `auto_detect_signal` | TEXT |
| `sort_order` | INTEGER NOT NULL DEFAULT 0 |
| `created_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |
| `updated_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |
| `id` | TEXT PRIMARY KEY |
| `objective_id` | TEXT NOT NULL REFERENCES account_objectives(id) ON DELETE CASCADE |
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `title` | TEXT NOT NULL |
| `status` | TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'completed', 'skipped')) |
| `target_date` | TEXT |
| `completed_at` | TEXT |
| `auto_detect_signal` | TEXT |
| `sort_order` | INTEGER NOT NULL DEFAULT 0 |
| `created_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |
| `updated_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |
- `completed_by` *(added in 075_v110_lifecycle_products_provenance)*
- `completion_trigger` *(added in 075_v110_lifecycle_products_provenance)*

---

### `account_milestones_new`

**Created in:** `069_account_events_expand`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `objective_id` | TEXT NOT NULL REFERENCES account_objectives(id) ON DELETE CASCADE |
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `title` | TEXT NOT NULL |
| `status` | TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'completed', 'skipped')) |
| `target_date` | TEXT |
| `completed_at` | TEXT |
| `auto_detect_signal` | TEXT |
| `sort_order` | INTEGER NOT NULL DEFAULT 0 |
| `created_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |
| `updated_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |

---

### `account_objectives`

**Created in:** `068_success_plans`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `title` | TEXT NOT NULL |
| `description` | TEXT |
| `status` | TEXT NOT NULL DEFAULT 'active' |
| `target_date` | TEXT |
| `completed_at` | TEXT |
| `source` | TEXT NOT NULL DEFAULT 'user' |
| `sort_order` | INTEGER NOT NULL DEFAULT 0 |
| `created_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |
| `updated_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |
| `id` | TEXT PRIMARY KEY |
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `title` | TEXT NOT NULL |
| `description` | TEXT |
| `status` | TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('draft', 'active', 'completed', 'abandoned')) |
| `target_date` | TEXT |
| `completed_at` | TEXT |
| `source` | TEXT NOT NULL DEFAULT 'user' |
| `sort_order` | INTEGER NOT NULL DEFAULT 0 |
| `created_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |
| `updated_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |
- `evidence_json` *(added in 086_objective_evidence)*
- `ai_origin_id` *(added in 086_objective_evidence)*

---

### `account_objectives_new`

**Created in:** `069_account_events_expand`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `title` | TEXT NOT NULL |
| `description` | TEXT |
| `status` | TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('draft', 'active', 'completed', 'abandoned')) |
| `target_date` | TEXT |
| `completed_at` | TEXT |
| `source` | TEXT NOT NULL DEFAULT 'user' |
| `sort_order` | INTEGER NOT NULL DEFAULT 0 |
| `created_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |
| `updated_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |

---

### `account_products`

**Created in:** `075_v110_lifecycle_products_provenance`

| Column | Definition |
|--------|-----------|
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) |
| `name` | TEXT NOT NULL |
| `category` | TEXT |
| `status` | TEXT NOT NULL DEFAULT 'active' |
| `arr_portion` | REAL |
| `source` | TEXT NOT NULL |
| `confidence` | REAL NOT NULL DEFAULT 0.7 |
| `notes` | TEXT |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `updated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
- `product_type` *(added in 079_product_classification)*
- `tier` *(added in 079_product_classification)*
- `billing_terms` *(added in 079_product_classification)*
- `arr` *(added in 079_product_classification)*
- `last_verified_at` *(added in 079_product_classification)*
- `data_source` *(added in 079_product_classification)*

---

### `account_source_refs`

**Created in:** `076_source_aware_account_truth`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `account_id` | TEXT NOT NULL |
| `field` | TEXT NOT NULL |
| `source_system` | TEXT NOT NULL |
| `source_kind` | TEXT NOT NULL DEFAULT 'inference' |
| `source_value` | TEXT |
| `observed_at` | TEXT NOT NULL |
| `source_record_ref` | TEXT |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

**Indexes:** idx_source_refs_account_field

---

### `account_stakeholder_roles`

**Created in:** `080_stakeholder_source_of_truth`

| Column | Definition |
|--------|-----------|
| `account_id` |  TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `person_id` |   TEXT NOT NULL REFERENCES people(id)   ON DELETE CASCADE |
| `role` |        TEXT NOT NULL |
| `data_source` | TEXT NOT NULL DEFAULT 'ai' |
| `created_at` |  TEXT NOT NULL DEFAULT (datetime('now')) |
- `dismissed_at` *(added in 107_stakeholder_role_dismissals)*

---

### `account_stakeholders`

**Created in:** `055_schema_decomposition`

| Column | Definition |
|--------|-----------|
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `person_id` | TEXT NOT NULL |
| `role` | TEXT NOT NULL DEFAULT 'associated' |
| `relationship_type` | TEXT DEFAULT 'associated' |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `person_id` | TEXT NOT NULL |
| `role` | TEXT NOT NULL DEFAULT 'associated' |
| `relationship_type` | TEXT DEFAULT 'associated' |
| `data_source` | TEXT NOT NULL DEFAULT 'user' |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `account_id` |             TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `person_id` |              TEXT NOT NULL REFERENCES people(id)   ON DELETE CASCADE |
| `engagement` |             TEXT,          -- strong_advocate | engaged | neutral | disengaged | unknown |
| `data_source_engagement` | TEXT NOT NULL DEFAULT 'ai' |
| `assessment` |             TEXT,          -- free-text assessment of the person's stance |
| `data_source_assessment` | TEXT NOT NULL DEFAULT 'ai' |
| `data_source` |            TEXT NOT NULL DEFAULT 'user',       -- row-level provenance (preserved) |
| `last_seen_in_glean` |     TEXT,                                -- staleness tracking (preserved) |
| `created_at` |             TEXT NOT NULL DEFAULT (datetime('now')) |
- `last_seen_in_glean` *(added in 061_stakeholder_glean_staleness)*
- `status` *(added in 114_account_stakeholders_review_queue_idx)*
- `confidence` *(added in 114_account_stakeholders_review_queue_idx)*

**Indexes:** idx_account_stakeholders_person

---

### `account_stakeholders_new`

**Created in:** `056_account_stakeholders_data_source`

| Column | Definition |
|--------|-----------|
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `person_id` | TEXT NOT NULL |
| `role` | TEXT NOT NULL DEFAULT 'associated' |
| `relationship_type` | TEXT DEFAULT 'associated' |
| `data_source` | TEXT NOT NULL DEFAULT 'user' |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `account_team`

**Created in:** `003_account_team`

| Column | Definition |
|--------|-----------|
| `account_id` | TEXT NOT NULL |
| `person_id` | TEXT NOT NULL |
| `role` | TEXT NOT NULL |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `account_id` | TEXT NOT NULL |
| `legacy_field` | TEXT NOT NULL |
| `legacy_value` | TEXT NOT NULL |
| `note` | TEXT NOT NULL |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `person_id` | TEXT NOT NULL |
| `role` | TEXT NOT NULL |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

**Indexes:** idx_account_team_account, idx_account_team_account_role, idx_account_team_notes_account, idx_account_team_person

---

### `account_team_import_notes`

**Created in:** `003_account_team`

| Column | Definition |
|--------|-----------|
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `account_id` | TEXT NOT NULL |
| `legacy_field` | TEXT NOT NULL |
| `legacy_value` | TEXT NOT NULL |
| `note` | TEXT NOT NULL |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

**Indexes:** idx_account_team_notes_account

---

### `account_team_new`

**Created in:** `010_foreign_keys`

| Column | Definition |
|--------|-----------|
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `person_id` | TEXT NOT NULL |
| `role` | TEXT NOT NULL |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `account_technical_footprint`

**Created in:** `077_technical_footprint`

| Column | Definition |
|--------|-----------|
| `account_id` | TEXT PRIMARY KEY |
| `integrations_json` | TEXT,        -- JSON array of integration names/types |
| `usage_tier` | TEXT,               -- 'enterprise', 'professional', 'starter', etc. |
| `adoption_score` | REAL,           -- 0.0-1.0 |
| `active_users` | INTEGER |
| `support_tier` | TEXT,             -- 'premium', 'standard', 'basic' |
| `csat_score` | REAL |
| `open_tickets` | INTEGER DEFAULT 0 |
| `services_stage` | TEXT,           -- 'onboarding', 'implementation', 'optimization', 'steady-state' |
| `source` | TEXT NOT NULL DEFAULT 'glean' |
| `sourced_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `updated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `accounts`

**Created in:** `001_baseline`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `name` | TEXT NOT NULL |
| `lifecycle` | TEXT |
| `arr` | REAL |
| `health` | TEXT CHECK(health IN ('green', 'yellow', 'red')) |
| `contract_start` | TEXT |
| `contract_end` | TEXT |
| `csm` | TEXT |
| `champion` | TEXT |
| `nps` | INTEGER |
| `tracker_path` | TEXT |
| `parent_id` | TEXT |
| `updated_at` | TEXT NOT NULL |
| `archived` | INTEGER DEFAULT 0 |
| `id` | TEXT PRIMARY KEY |
| `name` | TEXT NOT NULL |
| `lifecycle` | TEXT |
| `arr` | REAL |
| `health` | TEXT CHECK(health IN ('green', 'yellow', 'red')) |
| `contract_start` | TEXT |
| `contract_end` | TEXT |
| `nps` | INTEGER |
| `tracker_path` | TEXT |
| `parent_id` | TEXT |
| `is_internal` | INTEGER NOT NULL DEFAULT 0 |
| `updated_at` | TEXT NOT NULL |
| `archived` | INTEGER DEFAULT 0 |
- `is_internal` *(added in 002_internal_teams)*
- `keywords` *(added in 017_entity_keywords)*
- `keywords_extracted_at` *(added in 017_entity_keywords)*
- `metadata` *(added in 025_entity_metadata)*
- `account_type` *(added in 036_account_type)*
- `is_demo` *(added in 053_app_state_demo)*
- `renewal_stage` *(added in 075_v110_lifecycle_products_provenance)*
- `arr_source` *(added in 075_v110_lifecycle_products_provenance)*
- `arr_updated_at` *(added in 075_v110_lifecycle_products_provenance)*
- `lifecycle_source` *(added in 075_v110_lifecycle_products_provenance)*
- `lifecycle_updated_at` *(added in 075_v110_lifecycle_products_provenance)*
- `contract_end_source` *(added in 075_v110_lifecycle_products_provenance)*
- `contract_end_updated_at` *(added in 075_v110_lifecycle_products_provenance)*
- `nps_source` *(added in 075_v110_lifecycle_products_provenance)*
- `nps_updated_at` *(added in 075_v110_lifecycle_products_provenance)*
- `commercial_stage` *(added in 076_source_aware_account_truth)*
- `arr_range_low` *(added in 082_account_fact_columns)*
- `arr_range_high` *(added in 082_account_fact_columns)*
- `renewal_likelihood` *(added in 082_account_fact_columns)*
- `renewal_likelihood_source` *(added in 082_account_fact_columns)*
- `renewal_likelihood_updated_at` *(added in 082_account_fact_columns)*
- `renewal_model` *(added in 082_account_fact_columns)*
- `renewal_pricing_method` *(added in 082_account_fact_columns)*
- `support_tier` *(added in 082_account_fact_columns)*
- `support_tier_source` *(added in 082_account_fact_columns)*
- `support_tier_updated_at` *(added in 082_account_fact_columns)*
- `active_subscription_count` *(added in 082_account_fact_columns)*
- `growth_potential_score` *(added in 082_account_fact_columns)*
- `growth_potential_score_source` *(added in 082_account_fact_columns)*
- `icp_fit_score` *(added in 082_account_fact_columns)*
- `icp_fit_score_source` *(added in 082_account_fact_columns)*
- `primary_product` *(added in 082_account_fact_columns)*
- `customer_status` *(added in 082_account_fact_columns)*
- `customer_status_source` *(added in 082_account_fact_columns)*
- `customer_status_updated_at` *(added in 082_account_fact_columns)*
- `company_overview` *(added in 083_dashboard_fields_to_db)*
- `strategic_programs` *(added in 083_dashboard_fields_to_db)*
- `notes` *(added in 083_dashboard_fields_to_db)*
- `user_health_sentiment` *(added in 091_user_health_sentiment)*
- `sentiment_set_at` *(added in 091_user_health_sentiment)*
- `claim_version` *(added in 123_dos_310_per_entity_claim_invalidation)*

**Indexes:** idx_accounts_account_type, idx_accounts_archived, idx_accounts_internal, idx_accounts_name_lower, idx_accounts_parent

---

### `accounts_new`

**Created in:** `003_account_team`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `name` | TEXT NOT NULL |
| `lifecycle` | TEXT |
| `arr` | REAL |
| `health` | TEXT CHECK(health IN ('green', 'yellow', 'red')) |
| `contract_start` | TEXT |
| `contract_end` | TEXT |
| `nps` | INTEGER |
| `tracker_path` | TEXT |
| `parent_id` | TEXT |
| `is_internal` | INTEGER NOT NULL DEFAULT 0 |
| `updated_at` | TEXT NOT NULL |
| `archived` | INTEGER DEFAULT 0 |

---

### `action_linear_links`

**Created in:** `085_action_linear_links`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `action_id` | TEXT NOT NULL REFERENCES actions(id) ON DELETE CASCADE |
| `linear_issue_id` | TEXT NOT NULL |
| `linear_identifier` | TEXT NOT NULL |
| `linear_url` | TEXT NOT NULL |
| `pushed_at` | TEXT NOT NULL |

**Indexes:** idx_action_linear_links_action

---

### `action_objective_links`

**Created in:** `068_success_plans`

| Column | Definition |
|--------|-----------|
| `action_id` | TEXT NOT NULL REFERENCES actions(id) ON DELETE CASCADE |
| `objective_id` | TEXT NOT NULL REFERENCES account_objectives(id) ON DELETE CASCADE |
| `created_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |

---

### `actions`

**Created in:** `001_baseline`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `title` | TEXT NOT NULL |
| `priority` | TEXT CHECK(priority IN ('P1', 'P2', 'P3')) DEFAULT 'P2' |
| `status` | TEXT CHECK(status IN ('pending', 'completed', 'waiting', 'cancelled')) DEFAULT 'pending' |
| `created_at` | TEXT NOT NULL |
| `due_date` | TEXT |
| `completed_at` | TEXT |
| `account_id` | TEXT |
| `project_id` | TEXT |
| `source_type` | TEXT |
| `source_id` | TEXT |
| `source_label` | TEXT |
| `context` | TEXT |
| `waiting_on` | TEXT |
| `updated_at` | TEXT NOT NULL |
| `person_id` | TEXT |
| `needs_decision` | INTEGER DEFAULT 0 |
| `id` | TEXT PRIMARY KEY |
| `title` | TEXT NOT NULL |
| `priority` | TEXT CHECK(priority IN ('P1', 'P2', 'P3')) DEFAULT 'P2' |
| `status` | TEXT CHECK(status IN ('pending', 'completed', 'waiting', 'cancelled')) DEFAULT 'pending' |
| `created_at` | TEXT NOT NULL |
| `due_date` | TEXT |
| `completed_at` | TEXT |
| `account_id` | TEXT REFERENCES accounts(id) ON DELETE SET NULL |
| `project_id` | TEXT REFERENCES projects(id) ON DELETE SET NULL |
| `source_type` | TEXT |
| `source_id` | TEXT |
| `source_label` | TEXT |
| `context` | TEXT |
| `waiting_on` | TEXT |
| `updated_at` | TEXT NOT NULL |
| `person_id` | TEXT REFERENCES people(id) ON DELETE SET NULL |
| `needs_decision` | INTEGER DEFAULT 0 |
| `id` | TEXT PRIMARY KEY |
| `title` | TEXT NOT NULL |
| `priority` | TEXT CHECK(priority IN ('P1', 'P2', 'P3')) DEFAULT 'P2' |
| `status` | TEXT CHECK(status IN ('pending', 'completed', 'waiting', 'cancelled', 'proposed', 'archived')) DEFAULT 'pending' |
| `created_at` | TEXT NOT NULL |
| `due_date` | TEXT |
- `rejected_at` *(added in 022_rejection_signals)*
- `rejection_source` *(added in 022_rejection_signals)*
- `is_demo` *(added in 053_app_state_demo)*
- `decision_owner` *(added in 086_decision_columns)*
- `decision_stakes` *(added in 086_decision_columns)*
- `action_kind` *(added in 108_work_tab_actions)*

**Indexes:** idx_actions_account, idx_actions_due_date, idx_actions_kind, idx_actions_rejected, idx_actions_status, idx_actions_status_due_date, idx_actions_title_lower

---

### `actions_backup`

**Created in:** `011_proposed_actions`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `title` | TEXT NOT NULL |
| `priority` | TEXT CHECK(priority IN ('P1', 'P2', 'P3')) DEFAULT 'P2' |
| `status` | TEXT CHECK(status IN ('pending', 'completed', 'waiting', 'cancelled', 'proposed', 'archived')) DEFAULT 'pending' |
| `created_at` | TEXT NOT NULL |
| `due_date` | TEXT |
| `completed_at` | TEXT |
| `account_id` | TEXT REFERENCES accounts(id) ON DELETE SET NULL |
| `project_id` | TEXT REFERENCES projects(id) ON DELETE SET NULL |
| `source_type` | TEXT |
| `source_id` | TEXT |
| `source_label` | TEXT |
| `context` | TEXT |
| `waiting_on` | TEXT |
| `updated_at` | TEXT NOT NULL |
| `person_id` | TEXT REFERENCES people(id) ON DELETE SET NULL |
| `needs_decision` | INTEGER DEFAULT 0 |
| `id` | TEXT PRIMARY KEY |
| `title` | TEXT NOT NULL |
| `priority` | TEXT CHECK(priority IN ('P1', 'P2', 'P3')) DEFAULT 'P2' |
| `status` | TEXT CHECK(status IN ('suggested', 'pending', 'completed', 'archived')) DEFAULT 'pending' |
| `created_at` | TEXT NOT NULL |
| `due_date` | TEXT |
| `completed_at` | TEXT |
| `account_id` | TEXT REFERENCES accounts(id) ON DELETE SET NULL |
| `project_id` | TEXT REFERENCES projects(id) ON DELETE SET NULL |
| `source_type` | TEXT |
| `source_id` | TEXT |
| `source_label` | TEXT |
| `context` | TEXT |
| `waiting_on` | TEXT |
| `updated_at` | TEXT NOT NULL |
| `person_id` | TEXT REFERENCES people(id) ON DELETE SET NULL |
| `needs_decision` | INTEGER DEFAULT 0 |
| `rejected_at` | TEXT |
| `rejection_source` | TEXT |
| `is_demo` | INTEGER NOT NULL DEFAULT 0 |
| `id` | TEXT PRIMARY KEY |
| `title` | TEXT NOT NULL |
| `priority` | INTEGER CHECK(priority BETWEEN 0 AND 4) DEFAULT 3 |

---

### `actions_new`

**Created in:** `010_foreign_keys`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `title` | TEXT NOT NULL |
| `priority` | TEXT CHECK(priority IN ('P1', 'P2', 'P3')) DEFAULT 'P2' |
| `status` | TEXT CHECK(status IN ('pending', 'completed', 'waiting', 'cancelled')) DEFAULT 'pending' |
| `created_at` | TEXT NOT NULL |
| `due_date` | TEXT |
| `completed_at` | TEXT |
| `account_id` | TEXT REFERENCES accounts(id) ON DELETE SET NULL |
| `project_id` | TEXT REFERENCES projects(id) ON DELETE SET NULL |
| `source_type` | TEXT |
| `source_id` | TEXT |
| `source_label` | TEXT |
| `context` | TEXT |
| `waiting_on` | TEXT |
| `updated_at` | TEXT NOT NULL |
| `person_id` | TEXT REFERENCES people(id) ON DELETE SET NULL |
| `needs_decision` | INTEGER DEFAULT 0 |

---

### `agent_trust_ledger`

**Created in:** `129_dos_7_claims_schema`

| Column | Definition |
|--------|-----------|
| `id` |                 INTEGER PRIMARY KEY AUTOINCREMENT |
| `agent_kind` |         TEXT NOT NULL,                  -- 'pty' | 'glean' | 'human' etc. |
| `agent_id` |           TEXT NOT NULL |
| `claim_type` |         TEXT,                           -- per-claim-type accumulation |
| `correct_count` |      INTEGER NOT NULL DEFAULT 0 |
| `incorrect_count` |    INTEGER NOT NULL DEFAULT 0 |
| `total_count` |        INTEGER NOT NULL DEFAULT 0 |
| `last_updated_at` |    TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `ai_commitment_bridge`

**Created in:** `108_work_tab_actions`

| Column | Definition |
|--------|-----------|
| `commitment_id` | TEXT PRIMARY KEY |
| `entity_type` |   TEXT NOT NULL |
| `entity_id` |     TEXT NOT NULL |
| `action_id` |     TEXT REFERENCES actions(id) ON DELETE SET NULL |
| `first_seen_at` | TEXT NOT NULL |
| `last_seen_at` |  TEXT NOT NULL |
| `tombstoned` |    INTEGER NOT NULL DEFAULT 0 |

---

### `app_state`

**Created in:** `053_app_state_demo`

| Column | Definition |
|--------|-----------|
| `id` | INTEGER PRIMARY KEY CHECK (id = 1) |
| `demo_mode_active` | INTEGER NOT NULL DEFAULT 0 |
| `has_completed_tour` | INTEGER NOT NULL DEFAULT 0 |
| `wizard_completed_at` | TEXT |
| `wizard_last_step` | TEXT |
| `key` | TEXT PRIMARY KEY |
| `value_json` | TEXT NOT NULL |
| `updated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `app_state_kv`

**Created in:** `057_intelligence_db_columns`

| Column | Definition |
|--------|-----------|
| `key` | TEXT PRIMARY KEY |
| `value_json` | TEXT NOT NULL |
| `updated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `attendee_display_names`

**Created in:** `026_attendee_display_names`

| Column | Definition |
|--------|-----------|
| `email` |        TEXT PRIMARY KEY |
| `display_name` | TEXT NOT NULL |
| `last_seen` |    TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `attendee_group_patterns`

**Created in:** `019_correction_learning`

| Column | Definition |
|--------|-----------|
| `group_hash` | TEXT PRIMARY KEY |
| `attendee_emails` | TEXT NOT NULL |
| `entity_id` | TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL |
| `occurrence_count` | INTEGER DEFAULT 1 |
| `last_seen_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `confidence` | REAL DEFAULT 0.0 |

**Indexes:** idx_group_patterns_entity

---

### `briefing_callouts`

**Created in:** `020_signal_propagation`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `signal_id` | TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL |
| `entity_id` | TEXT NOT NULL |
| `entity_name` | TEXT |
| `severity` | TEXT NOT NULL DEFAULT 'info' |
| `headline` | TEXT NOT NULL |
| `detail` | TEXT |
| `context_json` | TEXT |
| `surfaced_at` | TEXT |
| `dismissed_at` | TEXT |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

**Indexes:** idx_briefing_callouts_unsurfaced

---

### `captured_commitments`

**Created in:** `068_success_plans`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `meeting_id` | TEXT REFERENCES meetings(id) ON DELETE SET NULL |
| `title` | TEXT NOT NULL |
| `owner` | TEXT |
| `target_date` | TEXT |
| `confidence` | TEXT NOT NULL DEFAULT 'medium' |
| `source` | TEXT |
| `consumed` | INTEGER NOT NULL DEFAULT 0 |
| `created_at` | TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP |
- `milestone_id` *(added in 090_commitment_milestone_link)*
- `suggested_objective_id` *(added in 090_commitment_milestone_link)*

---

### `captures`

**Created in:** `001_baseline`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `meeting_id` | TEXT NOT NULL |
| `meeting_title` | TEXT NOT NULL |
| `account_id` | TEXT |
| `project_id` | TEXT |
| `capture_type` | TEXT CHECK(capture_type IN ('win', 'risk', 'action', 'decision')) NOT NULL |
| `content` | TEXT NOT NULL |
| `owner` | TEXT |
| `due_date` | TEXT |
| `captured_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `id` | TEXT PRIMARY KEY |
| `meeting_id` | TEXT NOT NULL |
| `meeting_title` | TEXT NOT NULL |
| `account_id` | TEXT |
| `project_id` | TEXT |
| `capture_type` | TEXT CHECK(capture_type IN ('win', 'risk', 'action', 'decision')) NOT NULL |
| `content` | TEXT NOT NULL |
| `owner` | TEXT |
| `due_date` | TEXT |
| `captured_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `id` | TEXT PRIMARY KEY |
| `meeting_id` | TEXT NOT NULL |
| `meeting_title` | TEXT NOT NULL |
| `account_id` | TEXT |
| `project_id` | TEXT |
| `capture_type` | TEXT CHECK(capture_type IN ('win', 'risk', 'action', 'decision', 'commitment')) |
| `content` | TEXT NOT NULL |
| `owner` | TEXT |
| `due_date` | TEXT |
| `captured_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `sub_type` | TEXT |
| `urgency` | TEXT |
| `impact` | TEXT |
| `evidence_quote` | TEXT |
| `speaker` | TEXT |

**Indexes:** idx_captures_account, idx_captures_meeting, idx_captures_type

---

### `captures_new`

**Created in:** `070_captures_metadata`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `meeting_id` | TEXT NOT NULL |
| `meeting_title` | TEXT NOT NULL |
| `account_id` | TEXT |
| `project_id` | TEXT |
| `capture_type` | TEXT CHECK(capture_type IN ('win', 'risk', 'action', 'decision', 'commitment')) |
| `content` | TEXT NOT NULL |
| `owner` | TEXT |
| `due_date` | TEXT |
| `captured_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `sub_type` | TEXT |
| `urgency` | TEXT |
| `impact` | TEXT |
| `evidence_quote` | TEXT |
| `speaker` | TEXT |

---

### `chat_sessions`

**Created in:** `007_chat_interface`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `entity_id` | TEXT,              -- nullable (general chat not tied to entity) |
| `entity_type` | TEXT,            -- 'account' | 'project' | NULL |
| `session_start` | TEXT NOT NULL |
| `session_end` | TEXT,            -- NULL if active |
| `turn_count` | INTEGER DEFAULT 0 |
| `last_message` | TEXT |
| `created_at` | TEXT NOT NULL |

**Indexes:** idx_sessions_entity

---

### `chat_turns`

**Created in:** `007_chat_interface`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `session_id` | TEXT NOT NULL |
| `turn_index` | INTEGER NOT NULL |
| `role` | TEXT NOT NULL,           -- 'user' | 'assistant' |
| `content` | TEXT NOT NULL |
| `timestamp` | TEXT NOT NULL |

**Indexes:** idx_turns_session

---

### `claim_contradictions`

**Created in:** `129_dos_7_claims_schema`

| Column | Definition |
|--------|-----------|
| `id` |                     TEXT PRIMARY KEY,             -- UUID v4 |
| `primary_claim_id` |       TEXT NOT NULL REFERENCES intelligence_claims(id) |
| `contradicting_claim_id` | TEXT NOT NULL REFERENCES intelligence_claims(id) |
| `branch_kind` |            TEXT NOT NULL |
| `detected_at` |            TEXT NOT NULL DEFAULT (datetime('now')) |
| `reconciliation_kind` |    TEXT |
| `reconciliation_note` |    TEXT |
| `reconciled_at` |          TEXT |
| `winner_claim_id` |        TEXT REFERENCES intelligence_claims(id) |
| `merged_claim_id` |        TEXT REFERENCES intelligence_claims(id) |

---

### `claim_corroborations`

**Created in:** `129_dos_7_claims_schema`

| Column | Definition |
|--------|-----------|
| `id` |                    TEXT PRIMARY KEY,            -- UUID v4 |
| `claim_id` |              TEXT NOT NULL REFERENCES intelligence_claims(id) |
| `data_source` |           TEXT NOT NULL,               -- DataSource variant per DOS-212 |
| `source_asof` |           TEXT,                        -- ADR-0105 |
| `source_mechanism` |      TEXT,                        -- which legacy mechanism (backfill audit) |
| `strength` |              REAL NOT NULL DEFAULT 0.5 |
| `reinforcement_count` |   INTEGER NOT NULL DEFAULT 1 |
| `last_reinforced_at` |    TEXT NOT NULL DEFAULT (datetime('now')) |
| `created_at` |            TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `claim_edges`

**Created in:** `148_dos_265_claim_edges`

| Column | Definition |
|--------|-----------|
| `id` |              TEXT PRIMARY KEY |
| `from_entity_id` |  TEXT NOT NULL |
| `to_entity_id` |    TEXT NOT NULL |
| `edge_type` |       TEXT NOT NULL |
| `origin_claim_id` | TEXT NOT NULL REFERENCES intelligence_claims(id) |
| `link_source` |     TEXT NOT NULL CHECK (link_source IN ('frontmatter_map', 'manual', 'extracted')) |
| `weight` |          REAL NOT NULL DEFAULT 1.0 |
| `confidence` |      REAL NOT NULL DEFAULT 1.0 |
| `superseded_by` |   TEXT |
| `tombstoned_at` |   TEXT |
| `created_at` |      TEXT NOT NULL |

---

### `claim_feedback`

**Created in:** `129_dos_7_claims_schema`

| Column | Definition |
|--------|-----------|
| `id` |              TEXT PRIMARY KEY,                  -- UUID v4 |
| `claim_id` |        TEXT NOT NULL REFERENCES intelligence_claims(id) |
| `feedback_type` |   TEXT NOT NULL |
| `actor` |           TEXT NOT NULL |
| `actor_id` |        TEXT |
| `payload_json` |    TEXT,                              -- typed feedback content (correction, etc.) |
| `submitted_at` |    TEXT NOT NULL DEFAULT (datetime('now')) |
| `id` |              TEXT PRIMARY KEY |
| `claim_id` |        TEXT NOT NULL REFERENCES intelligence_claims(id) |
| `feedback_type` |   TEXT NOT NULL |
| `actor` |           TEXT NOT NULL |
| `actor_id` |        TEXT |
| `payload_json` |    TEXT |
| `submitted_at` |    TEXT NOT NULL DEFAULT (datetime('now')) |
| `applied_at` |      TEXT NULL |

---

### `claim_feedback_new`

**Created in:** `135_dos_294_typed_feedback_schema`

| Column | Definition |
|--------|-----------|
| `id` |              TEXT PRIMARY KEY |
| `claim_id` |        TEXT NOT NULL REFERENCES intelligence_claims(id) |
| `feedback_type` |   TEXT NOT NULL |
| `actor` |           TEXT NOT NULL |
| `actor_id` |        TEXT |
| `payload_json` |    TEXT |
| `submitted_at` |    TEXT NOT NULL DEFAULT (datetime('now')) |
| `applied_at` |      TEXT NULL |

---

### `claim_projection_status`

**Created in:** `134_dos_301_claim_projection_status`

| Column | Definition |
|--------|-----------|
| `claim_id` |            TEXT NOT NULL REFERENCES intelligence_claims(id) |
| `projection_target` |   TEXT NOT NULL |
| `status` |              TEXT NOT NULL |
| `error_message` |       TEXT |
| `attempted_at` |        TEXT NOT NULL |
| `succeeded_at` |        TEXT |

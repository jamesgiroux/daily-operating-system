# Data Model Reference

**Auto-generated:** 2026-04-20 by `.docs/generators/gen-data-model.sh`

**Database:** SQLite (SQLCipher-encrypted, WAL mode)
**Migrations:** 116 files (`001_baseline.sql` through `115_migrate_meeting_entity_dismissals.sql`)
**DB modules:** `src-tauri/src/db/`

---

## Table Inventory

| Table | Created In | Columns Added Later |
|-------|-----------|-------------------|
| `account_domains` | `002_internal_teams` | — |
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
| `accounts` | `001_baseline` | 002_internal_teams, 017_entity_keywords, 025_entity_metadata, 036_account_type, 053_app_state_demo, 075_v110_lifecycle_products_provenance, 076_source_aware_account_truth, 082_account_fact_columns, 083_dashboard_fields_to_db, 091_user_health_sentiment |
| `accounts_new` | `003_account_team` | — |
| `action_linear_links` | `085_action_linear_links` | — |
| `action_objective_links` | `068_success_plans` | — |
| `actions` | `001_baseline` | 022_rejection_signals, 053_app_state_demo, 086_decision_columns, 108_work_tab_actions |
| `actions_backup` | `011_proposed_actions` | — |
| `actions_new` | `010_foreign_keys` | — |
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
| `emails` | `034_emails` | 035_email_relevance_score, 071_email_triage_columns, 082_email_enriched_at, 100_email_retry_batch, 102_email_is_noise, 103_email_auto_retry_count, 104_email_is_noise_defensive |
| `emails_new` | `097_email_pending_retry_state` | — |
| `enrichment_log` | `016_clay_enrichment` | — |
| `entities` | `001_baseline` | 095_meeting_entities_confidence |
| `entity_assessment` | `055_schema_decomposition` | 057_intelligence_db_columns, 060_intelligence_dimensions, 078_pull_quote_column, 096_health_outlook_signals |
| `entity_assessment_new` | `058_health_schema_evolution` | — |
| `entity_context_entries` | `051_entity_context_entries` | — |
| `entity_email_cadence` | `028_entity_email_cadence` | — |
| `entity_feedback_events` | `084_feedback_events` | — |
| `entity_graph_version` | `113_entity_graph_version` | — |
| `entity_intelligence` | `001_baseline` | 040_entity_quality, 045_intelligence_report_fields, 047_entity_intel_user_relevance, 054_intelligence_consistency_metadata |
| `entity_linking_evaluations` | `112_entity_linking_evaluations` | — |
| `entity_members` | `055_schema_decomposition` | — |
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
| `intelligence_feedback` | `062_intelligence_feedback` | — |
| `intelligence_feedback_new` | `067_feedback_unique_constraint` | — |
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
| `meetings` | `055_schema_decomposition` | 031_intelligence_lifecycle |
| `meetings_history` | `001_baseline` | 031_intelligence_lifecycle |
| `meetings_history_new` | `023_drop_meeting_account_id` | — |
| `nudge_dismissals` | `108_work_tab_actions` | — |
| `people` | `001_baseline` | 016_clay_enrichment, 053_app_state_demo |
| `person_emails` | `012_person_emails` | — |
| `person_relationships` | `038_person_relationships` | 059_person_relationships_rationale |
| `person_relationships_new` | `039_person_relationships_types` | — |
| `pipeline_failures` | `064_pipeline_failures` | — |
| `post_meeting_emails` | `020_signal_propagation` | — |
| `proactive_insights` | `021_proactive_surfacing` | — |
| `proactive_scan_state` | `021_proactive_surfacing` | — |
| `processing_log` | `001_baseline` | — |
| `projects` | `001_baseline` | 017_entity_keywords, 025_entity_metadata, 037_project_hierarchy, 083_dashboard_fields_to_db |
| `quill_sync_state` | `013_quill_sync` | — |
| `quill_sync_state_new` | `055_schema_decomposition` | — |
| `rejected_action_patterns` | `086_rejected_action_patterns` | — |
| `reports` | `050_reports` | — |
| `risk_briefing_jobs` | `098_risk_briefing_jobs` | 101_risk_briefing_attempt_and_recompute_pending |
| `signal_derivations` | `020_signal_propagation` | — |
| `signal_events` | `018_signal_bus` | 019_correction_learning |
| `signal_weights` | `018_signal_bus` | — |
| `stakeholder_suggestions` | `080_stakeholder_source_of_truth` | — |
| `suppression_tombstones` | `084_feedback_events` | — |
| `sync_metadata` | `066_sync_metadata` | — |
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

### `clay_sync_state`

**Created in:** `016_clay_enrichment`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `entity_type` | TEXT NOT NULL DEFAULT 'person' |
| `entity_id` | TEXT NOT NULL |
| `state` | TEXT NOT NULL DEFAULT 'pending' |
| `attempts` | INTEGER NOT NULL DEFAULT 0 |
| `max_attempts` | INTEGER NOT NULL DEFAULT 3 |
| `clay_contact_id` | TEXT |
| `last_attempt_at` | TEXT |
| `completed_at` | TEXT |
| `error_message` | TEXT |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `updated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

**Indexes:** idx_clay_sync_state

---

### `content_embeddings`

**Created in:** `006_content_embeddings`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `content_file_id` | TEXT NOT NULL |
| `chunk_index` | INTEGER NOT NULL |
| `chunk_text` | TEXT NOT NULL |
| `embedding` | BLOB NOT NULL,        -- f32 vector, 768 dimensions |
| `created_at` | TEXT NOT NULL |

**Indexes:** idx_embeddings_file

---

### `content_index`

**Created in:** `001_baseline`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `entity_id` | TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL DEFAULT 'account' |
| `filename` | TEXT NOT NULL |
| `relative_path` | TEXT NOT NULL |
| `absolute_path` | TEXT NOT NULL |
| `format` | TEXT NOT NULL |
| `file_size` | INTEGER NOT NULL DEFAULT 0 |
| `modified_at` | TEXT NOT NULL |
| `indexed_at` | TEXT NOT NULL |
| `extracted_at` | TEXT |
| `summary` | TEXT |
| `content_type` | TEXT NOT NULL DEFAULT 'general' |
| `priority` | INTEGER NOT NULL DEFAULT 5 |
- `embeddings_generated_at` *(added in 006_content_embeddings)*
- `embeddings_generated_at` *(added in 009_fix_embeddings_column)*

**Indexes:** idx_content_entity, idx_content_modified

---

### `context_mode_config`

**Created in:** `052_glean_document_cache`

| Column | Definition |
|--------|-----------|
| `id` |          INTEGER PRIMARY KEY CHECK (id = 1) |
| `mode_json` |   TEXT |
| `updated_at` |  TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `drive_watched_sources`

**Created in:** `048_google_drive_sync`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `google_id` | TEXT NOT NULL |
| `name` | TEXT NOT NULL |
| `file_type` | TEXT NOT NULL DEFAULT 'document' |
| `google_doc_url` | TEXT |
| `entity_id` | TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL |
| `last_synced_at` | TEXT |
| `changes_token` | TEXT |
| `created_at` | TEXT NOT NULL |

---

### `email_dismissals`

**Created in:** `030_email_dismissals`

| Column | Definition |
|--------|-----------|
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `item_type` | TEXT NOT NULL |
| `email_id` | TEXT NOT NULL |
| `sender_domain` | TEXT |
| `email_type` | TEXT |
| `entity_id` | TEXT |
| `item_text` | TEXT NOT NULL |
| `dismissed_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

**Indexes:** idx_email_dismissals_domain, idx_email_dismissals_email, idx_email_dismissals_type

---

### `email_signals`

**Created in:** `005_email_signals`

| Column | Definition |
|--------|-----------|
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `email_id` | TEXT NOT NULL |
| `sender_email` | TEXT |
| `person_id` | TEXT |
| `entity_id` | TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL |
| `signal_type` | TEXT NOT NULL |
| `signal_text` | TEXT NOT NULL |
| `confidence` | REAL |
| `sentiment` | TEXT |
| `urgency` | TEXT |
| `detected_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `email_id` | TEXT NOT NULL |
| `sender_email` | TEXT |
| `person_id` | TEXT |
| `entity_id` | TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL |
| `signal_type` | TEXT NOT NULL |
| `signal_text` | TEXT NOT NULL |
| `confidence` | REAL |
| `sentiment` | TEXT |
| `urgency` | TEXT |
| `detected_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `deactivated_at` | TEXT |
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `email_id` | TEXT NOT NULL |
| `sender_email` | TEXT |
| `person_id` | TEXT |
| `entity_id` | TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL |
| `signal_type` | TEXT NOT NULL |
| `signal_text` | TEXT NOT NULL |
| `confidence` | REAL |
| `sentiment` | TEXT |
| `urgency` | TEXT |
| `detected_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `deactivated_at` | TEXT |
| `source` | TEXT NOT NULL DEFAULT 'email_enrichment' |

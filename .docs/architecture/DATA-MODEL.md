# Data Model Reference

**Auto-generated:** 2026-04-19 by `.docs/generators/gen-data-model.sh`

**Database:** SQLite (SQLCipher-encrypted, WAL mode)
**Migrations:** 109 files (`001_baseline.sql` through `108_work_tab_actions.sql`)
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
| `account_stakeholders` | `055_schema_decomposition` | 061_stakeholder_glean_staleness |
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
| `entity_intelligence` | `001_baseline` | 040_entity_quality, 045_intelligence_report_fields, 047_entity_intel_user_relevance, 054_intelligence_consistency_metadata |
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
- `deactivated_at` *(added in 034_emails)*

---

### `email_signals_new`

**Created in:** `063_email_signals_source`

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
| `deactivated_at` | TEXT |
| `source` | TEXT NOT NULL DEFAULT 'email_enrichment' |

---

### `email_sync_meta`

**Created in:** `093_email_sync_meta`

| Column | Definition |
|--------|-----------|
| `id` | INTEGER PRIMARY KEY CHECK (id = 1) |
| `last_successful_fetch_at` | TEXT |
| `updated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `email_threads`

**Created in:** `027_email_threads`

| Column | Definition |
|--------|-----------|
| `thread_id` | TEXT PRIMARY KEY |
| `subject` | TEXT NOT NULL DEFAULT '' |
| `last_sender_email` | TEXT NOT NULL DEFAULT '' |
| `last_message_date` | TEXT NOT NULL DEFAULT '' |
| `message_count` | INTEGER NOT NULL DEFAULT 1 |
| `user_is_last_sender` | INTEGER NOT NULL DEFAULT 0 |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `updated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `emails`

**Created in:** `034_emails`

| Column | Definition |
|--------|-----------|
| `person_id` | TEXT NOT NULL |
| `email` | TEXT NOT NULL COLLATE NOCASE |
| `is_primary` | INTEGER NOT NULL DEFAULT 0 |
| `added_at` | TEXT NOT NULL |
| `id` | TEXT PRIMARY KEY |
| `meeting_id` | TEXT NOT NULL |
| `email_signal_id` | TEXT NOT NULL |
| `thread_id` | TEXT |
| `correlated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `actions_extracted` | TEXT |
| `email_id` | TEXT PRIMARY KEY |
| `thread_id` | TEXT |
| `sender_email` | TEXT |
| `sender_name` | TEXT |
| `subject` | TEXT |
| `snippet` | TEXT |
| `priority` | TEXT |
| `is_unread` | INTEGER DEFAULT 1 |
| `received_at` | TEXT |
| `enrichment_state` | TEXT DEFAULT 'pending' CHECK(enrichment_state IN ('pending', 'enriching', 'enriched', 'failed')) |
| `enrichment_attempts` | INTEGER DEFAULT 0 |
| `last_enrichment_at` | TEXT |
| `last_seen_at` | TEXT |
| `resolved_at` | TEXT |
| `entity_id` | TEXT |
| `entity_type` | TEXT |
| `contextual_summary` | TEXT |
| `sentiment` | TEXT |
| `urgency` | TEXT |
| `user_is_last_sender` | INTEGER DEFAULT 0 |
| `last_sender_email` | TEXT |
| `message_count` | INTEGER DEFAULT 1 |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `updated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `email_id` | TEXT PRIMARY KEY |
| `thread_id` | TEXT |
| `sender_email` | TEXT |
| `sender_name` | TEXT |
| `subject` | TEXT |
| `snippet` | TEXT |
- `relevance_score` *(added in 035_email_relevance_score)*
- `score_reason` *(added in 035_email_relevance_score)*
- `pinned_at` *(added in 071_email_triage_columns)*
- `commitments` *(added in 071_email_triage_columns)*
- `questions` *(added in 071_email_triage_columns)*
- `enriched_at` *(added in 082_email_enriched_at)*
- `retry_batch_id` *(added in 100_email_retry_batch)*
- `retry_started_at` *(added in 100_email_retry_batch)*
- `is_noise` *(added in 102_email_is_noise)*
- `auto_retry_count` *(added in 103_email_auto_retry_count)*
- `is_noise` *(added in 104_email_is_noise_defensive)*

**Indexes:** idx_emails_enriched_at, idx_emails_enrichment, idx_emails_entity, idx_emails_is_noise, idx_emails_last_seen, idx_emails_priority_resolved, idx_emails_relevance, idx_emails_resolved, idx_emails_thread_id, idx_person_emails_email, idx_post_meeting_emails_meeting

---

### `emails_new`

**Created in:** `097_email_pending_retry_state`

| Column | Definition |
|--------|-----------|
| `email_id` | TEXT PRIMARY KEY |
| `thread_id` | TEXT |
| `sender_email` | TEXT |
| `sender_name` | TEXT |
| `subject` | TEXT |
| `snippet` | TEXT |
| `priority` | TEXT |
| `is_unread` | INTEGER DEFAULT 1 |
| `received_at` | TEXT |
| `enrichment_state` | TEXT DEFAULT 'pending' CHECK(enrichment_state IN ('pending', 'pending_retry', 'enriching', 'enriched', 'failed')) |
| `enrichment_attempts` | INTEGER DEFAULT 0 |
| `last_enrichment_at` | TEXT |
| `enriched_at` | DATETIME |
| `last_seen_at` | TEXT |
| `resolved_at` | TEXT |
| `entity_id` | TEXT |
| `entity_type` | TEXT |
| `contextual_summary` | TEXT |
| `sentiment` | TEXT |
| `urgency` | TEXT |
| `user_is_last_sender` | INTEGER DEFAULT 0 |
| `last_sender_email` | TEXT |
| `message_count` | INTEGER DEFAULT 1 |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `updated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `relevance_score` | REAL |
| `score_reason` | TEXT |
| `pinned_at` | TEXT |
| `commitments` | TEXT |
| `questions` | TEXT |

---

### `enrichment_log`

**Created in:** `016_clay_enrichment`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `entity_type` | TEXT NOT NULL,        -- 'person' | 'account' |
| `entity_id` | TEXT NOT NULL |
| `source` | TEXT NOT NULL,             -- 'clay' | 'gravatar' | 'ai' | 'user' |
| `event_type` | TEXT NOT NULL DEFAULT 'enrichment',  -- 'enrichment' | 'signal' |
| `signal_type` | TEXT,                 -- 'title_change' | 'company_change' | null |
| `fields_updated` | TEXT,             -- JSON array of field names |
| `raw_payload` | TEXT,                -- full response for debugging |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

**Indexes:** idx_enrichment_log_entity, idx_enrichment_log_recent

---

### `entities`

**Created in:** `001_baseline`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `name` | TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL DEFAULT 'account' |
| `tracker_path` | TEXT |
| `updated_at` | TEXT NOT NULL |
| `meeting_id` | TEXT NOT NULL |
| `entity_id` | TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL DEFAULT 'account' |
| `meeting_id` |  TEXT NOT NULL REFERENCES meetings_history(id) ON DELETE CASCADE |
| `entity_id` |   TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL DEFAULT 'account' |
| `meeting_id` |  TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE |
| `entity_id` |   TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL DEFAULT 'account' |
- `confidence` *(added in 095_meeting_entities_confidence)*
- `is_primary` *(added in 095_meeting_entities_confidence)*

**Indexes:** idx_entities_type, idx_meeting_entities_entity

---

### `entity_assessment`

**Created in:** `055_schema_decomposition`

| Column | Definition |
|--------|-----------|
| `entity_id` | TEXT PRIMARY KEY |
| `entity_type` | TEXT NOT NULL DEFAULT 'account' |
| `enriched_at` | TEXT |
| `source_file_count` | INTEGER DEFAULT 0 |
| `executive_assessment` | TEXT |
| `risks_json` | TEXT |
| `recent_wins_json` | TEXT |
| `current_state_json` | TEXT |
| `stakeholder_insights_json` | TEXT |
| `next_meeting_readiness_json` | TEXT |
| `company_context_json` | TEXT |
| `value_delivered` | TEXT |
| `success_metrics` | TEXT |
| `open_commitments` | TEXT |
| `relationship_depth` | TEXT |
| `user_relevance_weight` | REAL DEFAULT 1.0 |
| `consistency_status` | TEXT |
| `consistency_findings_json` | TEXT |
| `consistency_checked_at` | TEXT |
| `entity_id` | TEXT PRIMARY KEY |
| `entity_type` | TEXT NOT NULL DEFAULT 'account' |
| `enriched_at` | TEXT |
| `source_file_count` | INTEGER DEFAULT 0 |
| `executive_assessment` | TEXT |
| `risks_json` | TEXT |
| `recent_wins_json` | TEXT |
| `current_state_json` | TEXT |
| `stakeholder_insights_json` | TEXT |
| `next_meeting_readiness_json` | TEXT |
| `company_context_json` | TEXT |
| `value_delivered` | TEXT |
| `success_metrics` | TEXT |
| `open_commitments` | TEXT |
| `relationship_depth` | TEXT |
| `health_json` | TEXT |
| `org_health_json` | TEXT |
| `user_relevance_weight` | REAL DEFAULT 1.0 |
| `consistency_status` | TEXT |
| `consistency_findings_json` | TEXT |
| `consistency_checked_at` | TEXT |
- `portfolio_json` *(added in 057_intelligence_db_columns)*
- `network_json` *(added in 057_intelligence_db_columns)*
- `user_edits_json` *(added in 057_intelligence_db_columns)*
- `source_manifest_json` *(added in 057_intelligence_db_columns)*
- `dimensions_json` *(added in 060_intelligence_dimensions)*
- `pull_quote` *(added in 078_pull_quote_column)*
- `health_outlook_signals_json` *(added in 096_health_outlook_signals)*

**Indexes:** idx_entity_assessment_type

---

### `entity_assessment_new`

**Created in:** `058_health_schema_evolution`

| Column | Definition |
|--------|-----------|
| `entity_id` | TEXT PRIMARY KEY |
| `entity_type` | TEXT NOT NULL DEFAULT 'account' |
| `enriched_at` | TEXT |
| `source_file_count` | INTEGER DEFAULT 0 |
| `executive_assessment` | TEXT |
| `risks_json` | TEXT |
| `recent_wins_json` | TEXT |
| `current_state_json` | TEXT |
| `stakeholder_insights_json` | TEXT |
| `next_meeting_readiness_json` | TEXT |
| `company_context_json` | TEXT |
| `value_delivered` | TEXT |
| `success_metrics` | TEXT |
| `open_commitments` | TEXT |
| `relationship_depth` | TEXT |
| `health_json` | TEXT |
| `org_health_json` | TEXT |
| `user_relevance_weight` | REAL DEFAULT 1.0 |
| `consistency_status` | TEXT |
| `consistency_findings_json` | TEXT |
| `consistency_checked_at` | TEXT |
| `portfolio_json` | TEXT |
| `network_json` | TEXT |
| `user_edits_json` | TEXT |
| `source_manifest_json` | TEXT |
| `entity_id` | TEXT PRIMARY KEY |
| `entity_type` | TEXT NOT NULL DEFAULT 'account' |
| `enriched_at` | TEXT |
| `source_file_count` | INTEGER DEFAULT 0 |
| `executive_assessment` | TEXT |
| `risks_json` | TEXT |
| `recent_wins_json` | TEXT |
| `current_state_json` | TEXT |
| `stakeholder_insights_json` | TEXT |
| `next_meeting_readiness_json` | TEXT |
| `company_context_json` | TEXT |
| `value_delivered` | TEXT |
| `success_metrics` | TEXT |
| `open_commitments` | TEXT |
| `relationship_depth` | TEXT |

---

### `entity_context_entries`

**Created in:** `051_entity_context_entries`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `entity_type` | TEXT NOT NULL |
| `entity_id` | TEXT NOT NULL |
| `title` | TEXT NOT NULL |
| `content` | TEXT NOT NULL |
| `embedding` | BLOB |
| `created_at` | DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP |
| `updated_at` | DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP |

---

### `entity_email_cadence`

**Created in:** `028_entity_email_cadence`

| Column | Definition |
|--------|-----------|
| `entity_id` | TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL |
| `period` | TEXT NOT NULL |
| `message_count` | INTEGER NOT NULL DEFAULT 0 |
| `rolling_avg` | REAL NOT NULL DEFAULT 0.0 |
| `updated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `entity_feedback_events`

**Created in:** `084_feedback_events`

| Column | Definition |
|--------|-----------|
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `entity_id` | TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL DEFAULT 'account' |
| `field_key` | TEXT NOT NULL |
| `item_key` | TEXT |
| `feedback_type` | TEXT NOT NULL |
| `source_system` | TEXT |
| `source_kind` | TEXT |
| `previous_value` | TEXT |
| `corrected_value` | TEXT |
| `reason` | TEXT |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

**Indexes:** idx_feedback_entity

---

### `entity_intelligence`

**Created in:** `001_baseline`

| Column | Definition |
|--------|-----------|
| `entity_id` | TEXT PRIMARY KEY |
| `entity_type` | TEXT NOT NULL DEFAULT 'account' |
| `enriched_at` | TEXT |
| `source_file_count` | INTEGER DEFAULT 0 |
| `executive_assessment` | TEXT |
| `risks_json` | TEXT |
| `recent_wins_json` | TEXT |
| `current_state_json` | TEXT |
| `stakeholder_insights_json` | TEXT |
| `next_meeting_readiness_json` | TEXT |
| `company_context_json` | TEXT |
- `coherence_score` *(added in 040_entity_quality)*
- `coherence_flagged` *(added in 040_entity_quality)*
- `health_score` *(added in 045_intelligence_report_fields)*
- `health_trend` *(added in 045_intelligence_report_fields)*
- `value_delivered` *(added in 045_intelligence_report_fields)*
- `success_metrics` *(added in 045_intelligence_report_fields)*
- `open_commitments` *(added in 045_intelligence_report_fields)*
- `relationship_depth` *(added in 045_intelligence_report_fields)*
- `user_relevance_weight` *(added in 047_entity_intel_user_relevance)*
- `consistency_status` *(added in 054_intelligence_consistency_metadata)*
- `consistency_findings_json` *(added in 054_intelligence_consistency_metadata)*
- `consistency_checked_at` *(added in 054_intelligence_consistency_metadata)*

---

### `entity_members`

**Created in:** `055_schema_decomposition`

| Column | Definition |
|--------|-----------|
| `entity_id` | TEXT NOT NULL |
| `person_id` | TEXT NOT NULL |
| `relationship_type` | TEXT DEFAULT 'associated' |

**Indexes:** idx_entity_members_person

---

### `entity_people`

**Created in:** `001_baseline`

| Column | Definition |
|--------|-----------|
| `entity_id` | TEXT NOT NULL |
| `person_id` | TEXT NOT NULL |
| `relationship_type` | TEXT DEFAULT 'associated' |

**Indexes:** idx_entity_people_person

---

### `entity_quality`

**Created in:** `040_entity_quality`

| Column | Definition |
|--------|-----------|
| `entity_id` | TEXT PRIMARY KEY |
| `entity_type` | TEXT NOT NULL |
| `quality_alpha` | REAL NOT NULL DEFAULT 1.0 |
| `quality_beta` | REAL NOT NULL DEFAULT 1.0 |
| `quality_score` | REAL NOT NULL DEFAULT 0.5 |
| `last_enrichment_at` | TEXT |
| `correction_count` | INTEGER NOT NULL DEFAULT 0 |
| `coherence_retry_count` | INTEGER NOT NULL DEFAULT 0 |
| `coherence_window_start` | TEXT |
| `coherence_blocked` | INTEGER NOT NULL DEFAULT 0 |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `updated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `entity_id` | TEXT PRIMARY KEY |
| `entity_type` | TEXT NOT NULL |
| `quality_alpha` | REAL NOT NULL DEFAULT 1.0 |
| `quality_beta` | REAL NOT NULL DEFAULT 1.0 |
| `quality_score` | REAL NOT NULL DEFAULT 0.5 |
| `last_enrichment_at` | TEXT |
| `correction_count` | INTEGER NOT NULL DEFAULT 0 |
| `coherence_retry_count` | INTEGER NOT NULL DEFAULT 0 |
| `coherence_window_start` | TEXT |
| `coherence_blocked` | INTEGER NOT NULL DEFAULT 0 |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `updated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `health_score` | REAL |
| `health_trend` | TEXT |
| `coherence_score` | REAL |
| `coherence_flagged` | INTEGER DEFAULT 0 |

**Indexes:** idx_entity_quality_blocked, idx_entity_quality_score

---

### `entity_quality_new`

**Created in:** `055_schema_decomposition`

| Column | Definition |
|--------|-----------|
| `entity_id` | TEXT PRIMARY KEY |
| `entity_type` | TEXT NOT NULL |
| `quality_alpha` | REAL NOT NULL DEFAULT 1.0 |
| `quality_beta` | REAL NOT NULL DEFAULT 1.0 |
| `quality_score` | REAL NOT NULL DEFAULT 0.5 |
| `last_enrichment_at` | TEXT |
| `correction_count` | INTEGER NOT NULL DEFAULT 0 |
| `coherence_retry_count` | INTEGER NOT NULL DEFAULT 0 |
| `coherence_window_start` | TEXT |
| `coherence_blocked` | INTEGER NOT NULL DEFAULT 0 |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `updated_at` | TEXT NOT NULL DEFAULT (datetime('now')) |
| `health_score` | REAL |
| `health_trend` | TEXT |
| `coherence_score` | REAL |
| `coherence_flagged` | INTEGER DEFAULT 0 |

---

### `entity_resolution_feedback`

**Created in:** `019_correction_learning`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `meeting_id` | TEXT NOT NULL |
| `old_entity_id` | TEXT |
| `old_entity_type` | TEXT |
| `new_entity_id` | TEXT |
| `new_entity_type` | TEXT |
| `signal_source` | TEXT |
| `corrected_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

**Indexes:** idx_feedback_meeting, idx_feedback_source

---

### `glean_document_cache`

**Created in:** `052_glean_document_cache`

| Column | Definition |
|--------|-----------|
| `cache_key` |   TEXT PRIMARY KEY |
| `kind` |        TEXT NOT NULL DEFAULT 'document' |
| `content` |     TEXT NOT NULL |
| `cached_at` |   TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `gravatar_cache`

**Created in:** `015_gravatar_cache`

| Column | Definition |
|--------|-----------|
| `email` | TEXT PRIMARY KEY |
| `avatar_url` | TEXT |
| `display_name` | TEXT |
| `bio` | TEXT |
| `location` | TEXT |
| `company` | TEXT |
| `job_title` | TEXT |
| `interests_json` | TEXT |
| `has_gravatar` | INTEGER NOT NULL DEFAULT 0 |
| `fetched_at` | TEXT NOT NULL |
| `person_id` | TEXT REFERENCES people(id) ON DELETE SET NULL |

**Indexes:** idx_gravatar_cache_person_id

---

### `health_recompute_pending`

**Created in:** `101_risk_briefing_attempt_and_recompute_pending`

| Column | Definition |
|--------|-----------|
| `account_id` |    TEXT PRIMARY KEY |
| `requested_at` |  TEXT NOT NULL |

---

### `health_score_history`

**Created in:** `072_health_score_history`

| Column | Definition |
|--------|-----------|
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `account_id` | TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE |
| `score` | REAL NOT NULL |
| `band` | TEXT NOT NULL |
| `confidence` | REAL NOT NULL |
| `computed_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `hygiene_actions_log`

**Created in:** `029_hygiene_actions_log`

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `source_signal_id` | TEXT |
| `action_type` | TEXT NOT NULL |
| `entity_id` | TEXT NOT NULL |
| `entity_type` | TEXT NOT NULL DEFAULT '' |
| `confidence` | REAL NOT NULL DEFAULT 0.0 |
| `result` | TEXT NOT NULL DEFAULT '' |
| `created_at` | TEXT NOT NULL DEFAULT (datetime('now')) |

---

### `init_tasks`

**Created in:** `081_init_tasks`

| Column | Definition |
|--------|-----------|
| `task_name` | TEXT PRIMARY KEY |
| `completed_at` | TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP |

---

## v1.4.0 Substrate Tables

The following tables land as part of v1.4.0 Abilities Runtime substrate. They do not yet exist in code; migrations for them ship across the v1.4.0–v1.4.1 cycle per the DOS-7, DOS-215, DOS-216, DOS-234, DOS-235, DOS-236, DOS-238, DOS-259 issue cluster. Schema specifications from the owning ADRs.

### `intelligence_claims`

First-class claim table replacing inline JSON-blob assertions on `entity_assessment`. Append-only for assertion changes; trust annotation mutates in place.

Source of truth: [ADR-0113](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md) + [DOS-7](https://linear.app/a8c/issue/DOS-7).

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `entity_id` | TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE |
| `claim_type` | TEXT NOT NULL |
| `field_path` | TEXT NOT NULL |
| `claim_text` | TEXT (NULL for tombstones) |
| `actor` | TEXT NOT NULL (serialized ClaimActor: user / user_removal / human:role:id / agent:name:version / system:component / external:source) |
| `claim_state` | TEXT NOT NULL (proposed / committed / tombstoned / superseded / withdrawn) |
| `retraction_reason` | TEXT (required when tombstoned or withdrawn) |
| `dedup_key` | TEXT NOT NULL (hash of entity_id, claim_type, field_path, normalized claim_text) |
| `claim_sequence` | INTEGER NOT NULL (monotonic per entity_id + claim_type + field_path) |
| `previous_claim_id` | TEXT REFERENCES intelligence_claims(id) |
| `superseded_at` | TIMESTAMP |
| `superseded_by` | TEXT REFERENCES intelligence_claims(id) |
| `corroboration_count` | INTEGER NOT NULL DEFAULT 0 (cached from claim_corroborations) |
| `caused_by_invocation_id` | TEXT (observability correlation per ADR-0120) |
| `trust_score` | REAL |
| `trust_computed_at` | TIMESTAMP |
| `trust_version` | INTEGER NOT NULL DEFAULT 1 (monotonic on recomputation) |
| `pruned_at` | TIMESTAMP (soft-prune null-masks claim_text) |
| `created_at` | TIMESTAMP NOT NULL |
| `updated_at` | TIMESTAMP NOT NULL |

**Indexes:** `(entity_id, claim_type, field_path) WHERE superseded_at IS NULL` (default read), `(entity_id, claim_type, field_path, claim_sequence DESC)` (history), `(actor, created_at DESC)` (per-actor audits). `UNIQUE(entity_id, claim_type, field_path, claim_sequence)`.

**Read semantics:** default filter `claim_state IN ('committed', 'tombstoned') AND superseded_at IS NULL`. Tombstones are authoritative negative assertions.

### `claim_corroborations`

Per-asserter child table; preserves corroboration history without mutating claim rows.

Source: [ADR-0113 R1.6](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md#r16-dedup-semantics--preserve-per-asserter-history).

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `claim_id` | TEXT NOT NULL REFERENCES intelligence_claims(id) ON DELETE CASCADE |
| `corroborating_actor` | TEXT NOT NULL |
| `corroborating_source_ref` | TEXT |
| `asserted_at` | TIMESTAMP NOT NULL |

**Unique:** `(claim_id, corroborating_actor)`.

### `claim_contradictions`

Never auto-resolved. Both claims remain `committed`; surfaced explicitly.

Source: [ADR-0113 §7](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md).

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `field_path_ref` | TEXT NOT NULL |
| `claim_a_id` | TEXT NOT NULL REFERENCES intelligence_claims(id) |
| `claim_b_id` | TEXT NOT NULL REFERENCES intelligence_claims(id) |
| `detected_at` | TIMESTAMP NOT NULL |
| `resolved_at` | TIMESTAMP |
| `resolution` | TEXT (accept_a / accept_b / both_wrong / both_right_merge) |
| `resolved_by` | TEXT |

### `agent_trust_ledger`

Per-agent-per-version-per-claim-type Bayesian reliability.

Source: [ADR-0113 §6](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md).

| Column | Definition |
|--------|-----------|
| `agent_name` | TEXT NOT NULL |
| `agent_version` | TEXT NOT NULL |
| `claim_type` | TEXT NOT NULL |
| `alpha` | REAL NOT NULL DEFAULT 1.0 |
| `beta` | REAL NOT NULL DEFAULT 1.0 |
| `posterior_score` | REAL NOT NULL DEFAULT 0.5 |
| `last_updated` | TIMESTAMP NOT NULL |

**Primary key:** `(agent_name, agent_version, claim_type)`. Version-bump warming halves prior α/β per [ADR-0113 R1.4](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md#r14-trust-ratchet--shadow-sampling-prevents-permanent-quarantine).

### `evaluation_traces`

Runtime evaluator trace per Transform invocation.

Source: [ADR-0119 §6](../decisions/0119-runtime-evaluator-pass-for-transform-abilities.md).

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `ability_name` | TEXT NOT NULL |
| `ability_version` | TEXT NOT NULL |
| `primary_invocation_id` | TEXT NOT NULL |
| `primary_output_hash` | TEXT NOT NULL |
| `judge_model` | TEXT NOT NULL |
| `judge_prompt_version` | TEXT NOT NULL |
| `scores_json` | TEXT NOT NULL |
| `composite` | REAL NOT NULL |
| `critique` | TEXT |
| `threshold` | REAL NOT NULL |
| `passed` | INTEGER NOT NULL |
| `retry_invocation_id` | TEXT |
| `retry_composite` | REAL |
| `caused_by_invocation_id` | TEXT |
| `evaluated_at` | TIMESTAMP NOT NULL |
| `duration_ms` | INTEGER NOT NULL |

**Retention:** 90 days.

### `invalidation_jobs`

Durable queue replacing depth-limited-drop invalidation.

Source: [ADR-0115 §5](../decisions/0115-signal-granularity-audit.md).

| Column | Definition |
|--------|-----------|
| `id` | TEXT PRIMARY KEY |
| `chain_id` | TEXT NOT NULL |
| `origin_signal_id` | TEXT NOT NULL |
| `depth` | INTEGER NOT NULL |
| `affected_output_ids` | TEXT NOT NULL (JSON array) |
| `chain_ancestry` | TEXT NOT NULL (JSON array for cycle detection) |
| `enqueued_at` | TIMESTAMP NOT NULL |
| `status` | TEXT NOT NULL (Pending / Running / Completed / Failed / DeadLettered / CycleDetected) |
| `attempt_count` | INTEGER NOT NULL DEFAULT 0 |
| `last_error` | TEXT |

### `publish_drafts`, `publish_outbox`, `confirmation_tokens`

Pencil/Pen protocol. Types in v1.4.0, tables in v1.4.1.

Sources: [ADR-0117 R1.11](../decisions/0117-publish-boundary-pencil-and-pen.md#r111-scope-for-v140--revised-minimal), R1.4, R1.9. Schema shapes per each table's section in ADR-0117.

### `db_key_metadata` (v2.x)

Metadata about the DB encryption key (never the key material itself).

Source: [ADR-0116 R1.5](../decisions/0116-tenant-control-plane-boundary.md#r15-metadata-storage--acknowledge-the-schema-cost).

| Column | Definition |
|--------|-----------|
| `key_id` | TEXT PRIMARY KEY |
| `provider_type` | TEXT NOT NULL |
| `wrapping_key_ref` | TEXT |
| `version` | INTEGER NOT NULL |
| `rotated_at` | TIMESTAMP NOT NULL |
| `rekey_in_progress` | BOOLEAN NOT NULL DEFAULT 0 |

### Amendment to `signal_events`

[ADR-0115 Phase 0](../decisions/0115-signal-granularity-audit.md#r11-signaltype-enum-is-a-prerequisite-not-a-feature-of-this-adr) adds a Rust `SignalType` enum. Column stays `TEXT`; serialization is what changes. Plus a nullable `caused_by_invocation_id` column for observability correlation ([ADR-0120](../decisions/0120-observability-contract.md)).

## Append-only claim pattern — worked example

Scenario: AI proposes "champion: Alice" at t0; AI asserts "champion: Bob" at t1; user tombstones at t2.

**t0 — AI proposes "champion: Alice"**

- `claim-001`: `actor=agent:detect_champion:2.1`, `state=proposed`, `sequence=1`, `text="Alice Chen"`, `trust_score=0.62` (below 0.80 threshold).

Gate outcome: stays `proposed`; visible in Analysis Inbox only.

**t1 — same agent asserts "champion: Bob" (same actor version → supersede, not contradiction)**

- `claim-002`: `actor=agent:detect_champion:2.1`, `state=proposed`, `sequence=2`, `text="Bob Smith"`, `previous_claim_id=claim-001`.
- `claim-001` update: `superseded_at=t1`, `superseded_by=claim-002`, `state=superseded`.

**t2 — user tombstones the field**

- `claim-003`: `actor=user`, `state=tombstoned`, `sequence=3`, `text=NULL`, `retraction_reason="user_removal"`, `previous_claim_id=claim-002`.
- `claim-002` update: `superseded_at=t2`, `superseded_by=claim-003`, `state=superseded`.

**Default read at t2:** returns claim-003 (authoritative assertion of absence). Surface renders "no champion" with user-removal provenance.

**Future agent assertion attempt:** blocked by tombstone window check per [ADR-0113 R1.1](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md). Only 3+ independent corroborations within 7 days override.

**History read:** returns all three with supersede chain intact.

## Relationships

| From | To | Cardinality |
|---|---|---|
| `intelligence_claims.entity_id` | `entities.id` | N:1 |
| `claim_corroborations.claim_id` | `intelligence_claims.id` | N:1 |
| `claim_contradictions.claim_a_id` / `.claim_b_id` | `intelligence_claims.id` | N:1 each |
| `intelligence_claims.previous_claim_id` / `.superseded_by` | `intelligence_claims.id` | N:1 each |
| `invalidation_jobs.origin_signal_id` | `signal_events.id` | N:1 |
| `publish_outbox.draft_id` | `publish_drafts.id` | N:1 |
| `publish_outbox.confirmation_token_id` | `confirmation_tokens.id` | N:1 |
| `confirmation_tokens.draft_id` | `publish_drafts.id` | N:1 |

## Migration landing order

- **Phase 0** (substrate prerequisites): `ExecutionMode` (code-only), `IntelligenceProvider` trait (code-only), `signal_events` + `caused_by_invocation_id` column.
- **Phase 1 (v1.4.0):** `intelligence_claims`, `claim_corroborations`, `claim_contradictions`, `agent_trust_ledger`, `invalidation_jobs`.
- **Phase 2 (v1.4.1):** `evaluation_traces`, `publish_drafts`, `publish_outbox`, `confirmation_tokens`.
- **Phase 3 (v2.x, gated on enterprise demand):** `db_key_metadata`.

Brownfield-as-greenfield per founder D1 (2026-04-20): backward-compat paths for the 27 JSON-blob columns on `entity_assessment` are not required. Consolidation into `intelligence_claims` can be destructive. Old tombstone mechanisms (`suppression_tombstones`, `DismissedItem`, `account_stakeholder_roles.dismissed_at`) are replaced by claim-level tombstones; retirement ships in v1.4.1 alongside the enrichment refactor.

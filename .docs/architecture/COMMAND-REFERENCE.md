# Command Reference

Complete inventory of all Tauri IPC commands (`#[tauri::command]` functions).

**Auto-generated:** 2026-04-18 by `.docs/generators/gen-command-reference.sh`
**Registered in lib.rs:** ~369 commands
**Source files:** 11

---

## `commands/accounts_content_chat`

| Command | Async | Parameters |
|---------|-------|------------|
| `accept_account_field_conflict` | yes | account_id, field, suggested_value, source, signal_id |
| `accept_stakeholder_suggestion` | yes | suggestion_id |
| `add_account_team_member` | yes | account_id, person_id, role |
| `add_stakeholder_role` | yes | account_id, person_id, role |
| `backfill_internal_meeting_associations` | yes | — |
| `chat_get_briefing` | yes | — |
| `chat_list_entities` | yes | entity_type |
| `chat_query_entity` | yes | entity_id, question |
| `chat_search_content` | yes | entity_id, query, top_k |
| `confirm_lifecycle_change` | yes | change_id |
| `correct_account_product` | yes | account_id, product_id, name, status, source_to_penalize |
| `correct_lifecycle_change` | yes | change_id, corrected_lifecycle, corrected_stage, notes |
| `create_account` | yes | name, parent_id, account_type |
| `create_child_account` | yes | parent_id, name, description, owner_person_id |
| `create_internal_organization` | yes | company_name, domains, team_name, colleagues, existing_person_ids |
| `create_team` | yes | name, description, owner_person_id |
| `dismiss_account_field_conflict` | yes | account_id, field, signal_id, source, suggested_value |
| `dismiss_stakeholder_suggestion` | yes | suggestion_id |
| `enrich_account` | yes | account_id |
| `export_briefing_html` | — | meeting_id, markdown |
| `get_account_ancestors` | yes | account_id |
| `get_account_detail` | yes | account_id |
| `get_account_team` | yes | account_id |
| `get_accounts_for_picker` | yes | — |
| `get_accounts_list` | yes | — |
| `get_child_accounts_list` | yes | parent_id |
| `get_descendant_accounts` | yes | ancestor_id |
| `get_entity_files` | yes | entity_id |
| `get_internal_team_setup_status` | yes | — |
| `get_person_stakeholder_roles` | yes | person_id |
| `get_stakeholder_suggestions` | yes | account_id |
| `index_entity_files` | yes | entity_type, entity_id |
| `remove_account_team_member` | yes | account_id, person_id, role |
| `remove_stakeholder_role` | yes | account_id, person_id, role |
| `reveal_in_finder` | — | path |
| `set_team_member_role` | yes | account_id, person_id, new_role |
| `set_user_health_sentiment` | yes | account_id, sentiment, note |
| `update_account_field` | yes | account_id, field, value |
| `update_account_notes` | yes | account_id, notes |
| `update_account_programs` | yes | account_id, programs_json |
| `update_stakeholder_assessment` | yes | account_id, person_id, assessment |
| `update_stakeholder_engagement` | yes | account_id, person_id, engagement |

## `commands/actions_calendar`

| Command | Async | Parameters |
|---------|-------|------------|
| `accept_suggested_action` | yes | id |
| `archive_email` | yes | email_id |
| `attach_meeting_transcript` | yes | file_path, meeting |
| `capture_meeting_outcome` | yes | outcome |
| `complete_action` | yes | id |
| `create_action` | yes | request |
| `disconnect_google` | — | — |
| `dismiss_email_item` | yes | item_type, email_id, item_text, email_type, entity_id |
| `dismiss_meeting_prompt` | — | meeting_id |
| `get_action_detail` | yes | action_id |
| `get_actions_from_db` | yes | days_ahead |
| `get_calendar_events` | — | — |
| `get_capture_settings` | — | — |
| `get_current_meeting` | — | — |
| `get_google_auth_status` | — | — |
| `get_meeting_continuity_thread` | yes | meeting_id |
| `get_meeting_history` | yes | account_id, lookback_days, limit |
| `get_meeting_history_detail` | yes | meeting_id |
| `get_meeting_outcomes` | yes | meeting_id |
| `get_meeting_post_intelligence` | yes | meeting_id |
| `get_next_meeting` | — | — |
| `get_prediction_scorecard` | yes | meeting_id |
| `get_suggested_actions` | yes | — |
| `list_dismissed_email_items` | yes | — |
| `list_meeting_preps` | — | — |
| `pin_email` | yes | email_id |
| `reject_suggested_action` | yes | id, source |
| `reopen_action` | yes | id |
| `reprocess_meeting_transcript` | yes | meeting_id |
| `reset_email_preferences` | yes | crate |
| `resolve_decision` | yes | id |
| `search_meetings` | yes | query |
| `set_capture_delay` | — | delay_minutes |
| `set_capture_enabled` | — | enabled |
| `start_google_auth` | yes | — |
| `unarchive_email` | yes | email_id |
| `update_action` | yes | request |
| `update_action_priority` | yes | id, priority |
| `update_capture` | yes | id, content, crate |

## `commands/app_support`

| Command | Async | Parameters |
|---------|-------|------------|
| `bulk_recompute_health` | yes | — |
| `check_claude_status` | yes | — |
| `clear_claude_status_cache` | — | — |
| `clear_demo_data` | yes | — |
| `clear_intelligence` | yes | — |
| `delete_all_data` | yes | — |
| `dev_apply_scenario` | yes | scenario |
| `dev_clean_artifacts` | — | include_workspace |
| `dev_get_state` | — | — |
| `dev_onboarding_scenario` | yes | scenario |
| `dev_purge_mock_data` | — | — |
| `dev_restore_live` | yes | — |
| `dev_run_today_full` | — | — |
| `dev_run_today_mechanical` | — | — |
| `dev_set_auth_override` | — | claude_mode, google_mode |
| `export_all_data` | yes | dest_path |
| `get_ai_usage_diagnostics` | yes | — |
| `get_app_state` | yes | — |
| `get_data_summary` | yes | — |
| `get_db_growth_report` | yes | — |
| `get_executive_intelligence` | yes | — |
| `get_feature_flags` | yes | — |
| `get_feedback_diagnostics` | yes | — |
| `get_frequent_correspondents` | yes | user_email |
| `get_latency_rollups` | — | — |
| `get_onboarding_priming_context` | yes | — |
| `get_processing_history` | yes | limit |
| `get_sync_freshness` | yes | — |
| `install_claude_cli` | yes | — |
| `install_demo_data` | yes | — |
| `install_inbox_sample` | — | — |
| `launch_claude_login` | — | — |
| `populate_workspace` | yes | accounts, projects |
| `rebuild_search_index` | yes | — |
| `search_global` | yes | query |
| `set_tour_completed` | yes | — |
| `set_wizard_completed` | yes | — |
| `set_wizard_step` | yes | step |

## `commands/core`

| Command | Async | Parameters |
|---------|-------|------------|
| `backfill_prep_semantics` | yes | dry_run |
| `enrich_meeting_background` | yes | meeting_id |
| `generate_meeting_intelligence` | yes | meeting_id, force |
| `get_config` | — | — |
| `get_dashboard_data` | yes | — |
| `get_execution_history` | — | limit |
| `get_live_proactive_suggestions` | yes | force_refresh |
| `get_meeting_intelligence` | yes | meeting_id |
| `get_meeting_prep` | yes | meeting_id |
| `get_next_run_time` | — | workflow |
| `get_week_data` | — | — |
| `get_workflow_status` | — | workflow |
| `refresh_meeting_briefing` | yes | meeting_id |
| `refresh_meeting_preps` | yes | — |
| `reload_configuration` | — | — |
| `run_workflow` | — | workflow |

## `commands/integrations`

| Command | Async | Parameters |
|---------|-------|------------|
| `add_google_drive_watch` | yes | google_id, name, file_type, google_doc_url, entity_id, entity_type |
| `bulk_fetch_gravatars` | yes | — |
| `configure_claude_desktop` | — | — |
| `correct_email_disposition` | yes | email_id, corrected_priority |
| `create_linear_entity_link` | yes | crate, linear_project_id, entity_id, entity_type |
| `create_person_from_stakeholder` | yes | entity_id, entity_type, name, role |
| `delete_linear_entity_link` | yes | link_id |
| `delete_person_relationship` | yes | id |
| `detect_smithery_settings` | yes | — |
| `dev_explore_glean_tools` | yes | account_name |
| `disconnect_glean` | — | — |
| `disconnect_smithery` | — | — |
| `discover_accounts_from_glean` | yes | — |
| `dismiss_intelligence_item` | yes | entity_id, entity_type, field, item_text |
| `dismiss_recommendation` | yes | entity_id, entity_type, index |
| `enrich_account_from_clay` | yes | account_id |
| `enrich_person_from_clay` | yes | person_id |
| `export_audit_log` | — | dest_path |
| `export_cowork_plugin` | — | plugin_name |
| `fetch_gravatar` | yes | person_id |
| `get_active_preset` | yes | — |
| `get_audit_log_records` | — | limit, category_filter |
| `get_available_presets` | yes | — |
| `get_claude_desktop_status` | — | — |
| `get_clay_status` | yes | — |
| `get_context_mode` | — | — |
| `get_cowork_plugins_status` | — | — |
| `get_enrichment_log` | yes | entity_id |
| `get_entity_metadata` | yes | entity_type, entity_id |
| `get_glean_auth_status` | — | — |
| `get_glean_token_health` | — | — |
| `get_google_access_token` | yes | — |
| `get_google_client_id` | — | — |
| `get_google_drive_status` | yes | — |
| `get_google_drive_watches` | yes | — |
| `get_granola_status` | yes | — |
| `get_gravatar_status` | — | — |
| `get_linear_entity_links` | yes | — |
| `get_linear_projects` | yes | — |
| `get_linear_recent_issues` | yes | — |
| `get_linear_status` | — | — |
| `get_linear_teams` | yes | — |
| `get_meeting_timeline` | yes | days_before, days_after |
| `get_person_avatar` | yes | person_id |
| `get_person_relationships` | yes | person_id |
| `get_quill_status` | yes | — |
| `get_quill_sync_states` | yes | meeting_id |
| `get_smithery_status` | — | — |
| `import_account_from_glean` | yes | request |
| `import_google_drive_file` | yes | google_id, name, entity_id, entity_type |
| `onboarding_enrichment_status` | yes | account_names |
| `onboarding_import_accounts` | yes | #[allow)] account_names, accounts |
| `onboarding_prefill_profile` | yes | — |
| `push_action_to_linear` | yes | action_id, team_id, project_id, title |
| `query_ephemeral_account` | yes | name |
| `remove_google_drive_watch` | yes | watch_id |
| `run_linear_auto_link` | yes | — |
| `save_smithery_api_key` | yes | key |
| `set_clay_api_key` | — | key |
| `set_clay_auto_enrich` | — | enabled |
| `set_clay_enabled` | — | enabled |
| `set_context_mode` | — | mode |
| `set_google_drive_enabled` | — | enabled |
| `set_granola_enabled` | — | enabled |
| `set_granola_poll_interval` | — | minutes |
| `set_gravatar_api_key` | — | key |
| `set_gravatar_enabled` | — | enabled |
| `set_linear_api_key` | — | key |
| `set_linear_enabled` | — | enabled |
| `set_quill_enabled` | — | enabled |
| `set_quill_poll_interval` | — | minutes |
| `set_role` | yes | role |
| `set_smithery_connection` | — | namespace, connection_id |
| `start_clay_bulk_enrich` | yes | — |
| `start_glean_auth` | yes | endpoint |
| `start_granola_backfill` | — | days_back |
| `start_linear_sync` | — | — |
| `start_quill_backfill` | yes | days_back |
| `test_clay_connection` | yes | — |
| `test_granola_cache` | — | — |
| `test_linear_connection` | yes | — |
| `test_quill_connection` | yes | — |
| `track_recommendation` | yes | entity_id, entity_type, index |
| `trigger_drive_sync_now` | — | — |
| `trigger_granola_sync_for_meeting` | yes | meeting_id, force |
| `trigger_quill_sync_for_meeting` | yes | meeting_id, force |
| `update_entity_metadata` | yes | entity_type, entity_id, metadata |
| `update_intelligence_field` | yes | entity_id, entity_type, field_path, value |
| `update_stakeholders` | yes | entity_id, entity_type, stakeholders_json |
| `upsert_person_relationship` | yes | payload |
| `verify_audit_log_integrity` | — | — |

## `commands/people_entities`

| Command | Async | Parameters |
|---------|-------|------------|
| `add_meeting_entity` | yes | meeting_id, entity_id, entity_type, meeting_title, start_time, meeting_type_str |
| `create_person` | yes | email, name, organization, role, relationship |
| `delete_person` | yes | person_id |
| `enrich_person` | yes | person_id |
| `get_entity_feedback` | yes | entity_id, entity_type |
| `get_meeting_attendees` | yes | meeting_id |
| `get_meeting_entities` | yes | meeting_id |
| `get_people` | yes | relationship |
| `get_people_for_entity` | yes | entity_id |
| `get_person_detail` | yes | person_id |
| `link_meeting_entity` | yes | meeting_id, entity_id, entity_type |
| `link_person_entity` | yes | person_id, entity_id, relationship_type |
| `merge_people` | yes | keep_id, remove_id |
| `remove_account_keyword` | yes | account_id, keyword |
| `remove_meeting_entity` | yes | meeting_id, entity_id, entity_type |
| `remove_project_keyword` | yes | project_id, keyword |
| `search_people` | yes | query |
| `submit_intelligence_correction` | yes | entity_id, entity_type, field, action, corrected_value, annotation |
| `submit_intelligence_feedback` | yes | entity_id, entity_type, field, feedback_type, context |
| `unlink_meeting_entity` | yes | meeting_id, entity_id |
| `unlink_person_entity` | yes | person_id, entity_id |
| `update_meeting_entity` | yes | meeting_id, entity_id, entity_type, meeting_title, start_time, meeting_type_str |
| `update_person` | yes | person_id, field, value |

## `commands/planning_reports`

| Command | Async | Parameters |
|---------|-------|------------|
| `apply_meeting_prep_prefill` | yes | meeting_id, agenda_items, notes_append |
| `backfill_account_domains` | yes | — |
| `backfill_historical_meetings` | yes | — |
| `generate_meeting_agenda_message_draft` | yes | meeting_id, context_hint |
| `generate_report` | yes | entity_id, entity_type, report_type, spotlight_account_ids |
| `generate_risk_briefing` | yes | account_id |
| `get_report` | yes | entity_id, entity_type, report_type |
| `get_reports_for_entity` | yes | entity_id, entity_type |
| `get_risk_briefing` | yes | account_id |
| `recover_archived_transcripts` | yes | — |
| `save_report` | yes | entity_id, entity_type, report_type, content_json |
| `update_meeting_prep_field` | yes | meeting_id, field_path, value, target_person_id |
| `update_meeting_user_agenda` | yes | meeting_id, agenda, dismissed_topics, hidden_attendees |
| `update_meeting_user_notes` | yes | meeting_id, notes |

## `commands/projects_data`

| Command | Async | Parameters |
|---------|-------|------------|
| `archive_account` | yes | id, archived |
| `archive_person` | yes | id, archived |
| `archive_project` | yes | id, archived |
| `backup_database` | yes | — |
| `bulk_create_accounts` | yes | names |
| `bulk_create_projects` | yes | names |
| `create_project` | yes | name, parent_id |
| `enrich_project` | yes | project_id |
| `export_database_copy` | yes | destination |
| `get_account_events` | yes | account_id |
| `get_archived_accounts` | yes | — |
| `get_archived_people` | yes | — |
| `get_archived_projects` | yes | — |
| `get_child_projects_list` | yes | parent_id |
| `get_database_info` | — | — |
| `get_database_recovery_status` | — | — |
| `get_duplicate_people` | yes | — |
| `get_duplicate_people_for_person` | yes | person_id |
| `get_hygiene_narrative` | — | — |
| `get_hygiene_report` | — | — |
| `get_intelligence_hygiene_status` | — | — |
| `get_project_ancestors` | yes | project_id |
| `get_project_detail` | yes | project_id |
| `get_projects_list` | yes | — |
| `list_database_backups` | — | — |
| `merge_accounts` | yes | from_id, into_id |
| `rebuild_database` | yes | — |
| `record_account_event` | yes | account_id, event_type, event_date, arr_impact, notes |
| `restore_account` | yes | account_id, restore_children |
| `restore_database_from_backup` | yes | backup_path |
| `run_hygiene_scan_now` | — | — |
| `set_user_domains` | yes | domains |
| `start_fresh_database` | yes | — |
| `update_project_field` | yes | project_id, field, value |
| `update_project_notes` | yes | project_id, notes |

## `commands/success_plans`

| Command | Async | Parameters |
|---------|-------|------------|
| `abandon_objective` | yes | id |
| `apply_success_plan_template` | yes | account_id, template_id |
| `complete_milestone` | yes | id |
| `complete_objective` | yes | id |
| `create_milestone` | yes | objective_id, title, target_date, auto_detect_signal |
| `create_objective` | yes | account_id, title, description, target_date, source |
| `create_objective_from_suggestion` | yes | account_id, suggestion_json |
| `delete_milestone` | yes | id |
| `delete_objective` | yes | id |
| `get_objective_suggestions` | yes | account_id |
| `link_action_to_objective` | yes | action_id, objective_id |
| `list_success_plan_templates` | — | — |
| `reorder_milestones` | yes | objective_id, ordered_ids |
| `reorder_objectives` | yes | account_id, ordered_ids |
| `skip_milestone` | yes | id |
| `unlink_action_from_objective` | yes | action_id, objective_id |
| `update_milestone` | yes | id, fields |
| `update_objective` | yes | id, fields |

## `commands/workspace`

| Command | Async | Parameters |
|---------|-------|------------|
| `archive_low_priority_emails` | yes | — |
| `check_icloud_warning` | — | — |
| `copy_to_inbox` | — | paths |
| `create_entity_context_entry` | yes | entity_type, entity_id, title, content |
| `create_user_context_entry` | yes | title, content |
| `delete_entity_context_entry` | yes | id |
| `delete_user_context_entry` | yes | id |
| `dismiss_email_signal` | yes | signal_id |
| `dismiss_gone_quiet` | yes | entity_id |
| `dismiss_icloud_warning` | — | — |
| `enrich_inbox_file` | yes | filename, entity_id |
| `get_all_actions` | yes | — |
| `get_all_emails` | — | — |
| `get_email_sync_status` | yes | — |
| `get_emails_enriched` | yes | — |
| `get_encryption_key_status` | — | — |
| `get_entity_context_entries` | yes | entity_type, entity_id |
| `get_entity_emails` | yes | entity_id, entity_type |
| `get_inbox_file_content` | — | filename |
| `get_inbox_files` | yes | — |
| `get_lock_status` | — | — |
| `get_user_context_entries` | yes | — |
| `get_user_entity` | yes | — |
| `lock_app` | yes | — |
| `mark_reply_sent` | yes | email_id |
| `process_all_inbox` | yes | — |
| `process_inbox_file` | yes | filename, entity_id |
| `process_user_attachment` | yes | path |
| `refresh_emails` | yes | — |
| `reset_ai_models_to_recommended` | — | — |
| `retry_failed_emails` | yes | — |
| `set_ai_model` | — | tier, model |
| `set_developer_mode` | yes | enabled |
| `set_entity_mode` | — | mode |
| `set_google_poll_settings` | — | calendar_poll_interval_minutes, email_poll_interval_minutes |
| `set_hygiene_config` | — | scan_interval_hours, ai_budget, pre_meeting_hours |
| `set_lock_timeout` | — | minutes |
| `set_notification_config` | — | config |
| `set_personality` | — | personality |
| `set_profile` | — | profile |
| `set_schedule` | — | workflow, hour, minute, timezone |
| `set_text_scale` | — | percent |
| `set_user_profile` | yes | name, company, title, focus, domain, domains |
| `set_workspace_path` | yes | path |
| `signal_user_activity` | — | — |
| `signal_window_focus` | — | focused |
| `sync_email_inbox_presence` | yes | — |
| `unlock_app` | yes | — |
| `update_email_entity` | yes | email_id, entity_id, entity_type |
| `update_entity_context_entry` | yes | id, title, content |
| `update_user_context_entry` | yes | id, title, content |
| `update_user_entity_field` | yes | field, value |


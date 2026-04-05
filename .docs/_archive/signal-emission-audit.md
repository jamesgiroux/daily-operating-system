# I512 Signal Emission Audit

## Legend

- Class:
  - `DomainRequiredSignal`
  - `OperationalNoSignal`
- Status:
  - `pending`
  - `migrated`

## Runtime Mutation Inventory (Hotspot Scope)

| File | Function / Path | Mutation | Owner Service | Class | Required Signal | Propagation | Status |
|---|---|---|---|---|---|---|---|
| `commands.rs` | `backfill_db_prep_contexts` | `update_meeting_prep_context` | `services::meetings` | OperationalNoSignal | n/a | n/a | migrated |
| `commands.rs` | `reset_email_preferences` | `reset_email_dismissals` | `services::emails` | OperationalNoSignal | n/a | n/a | migrated |
| `commands.rs` | `update_capture` | `update_capture` | `services::meetings` | DomainRequiredSignal | `capture_updated` | no | migrated |
| `commands.rs` | `populate_workspace` | `upsert_account`, `upsert_project` | `services::accounts`, `services::projects` | DomainRequiredSignal | `entity_updated` | yes | migrated |
| `commands.rs` | `remove_project_keyword` | `remove_project_keyword` | `services::projects` | DomainRequiredSignal | `project_keywords_updated` | yes | migrated |
| `commands.rs` | `remove_account_keyword` | `remove_account_keyword` | `services::accounts` | DomainRequiredSignal | `account_keywords_updated` | yes | migrated |
| `commands.rs` | `chat_* helpers` | `create_chat_session`, `append_chat_turn`, `bump_chat_session_stats` | `services::entities` | OperationalNoSignal | n/a | n/a | migrated |
| `commands.rs` | `apply_meeting_prep_prefill_inner` | `update_meeting_user_layer` | `services::meetings` | DomainRequiredSignal | `prep_edited` | yes | migrated |
| `commands.rs` | `start_clay_bulk_enrich` | insert `clay_sync_state` | `services::integrations` | OperationalNoSignal | n/a | n/a | migrated |
| `commands.rs` | `run_linear_auto_link` | insert `linear_entity_links` | `services::integrations` | OperationalNoSignal | n/a | n/a | migrated |
| `commands.rs` | `delete_linear_entity_link` | delete `linear_entity_links` | `services::integrations` | OperationalNoSignal | n/a | n/a | migrated |
| `commands.rs` | `create_linear_entity_link` | insert `linear_entity_links` | `services::integrations` | OperationalNoSignal | n/a | n/a | migrated |
| `commands.rs` | `update_entity_metadata` | `update_entity_metadata` | `services::accounts` / `services::projects` | DomainRequiredSignal | `entity_metadata_updated` | yes | migrated |
| `commands.rs` | `correct_email_disposition` | `upsert_email_signal` | `services::emails` | DomainRequiredSignal | `email_disposition_corrected` | no | migrated |
| `commands.rs` | `get_meeting_timeline` | `upsert_meeting`, `link_meeting_entity` | `services::meetings` | DomainRequiredSignal | `meeting_upserted` | yes | migrated |
| `commands.rs` | `upsert_person_relationship` | `upsert_person_relationship` | `services::people` | DomainRequiredSignal | `relationship_graph_changed` | yes | migrated |
| `commands.rs` | `delete_person_relationship` | `delete_person_relationship` | `services::people` | DomainRequiredSignal | `relationship_graph_changed` | yes | migrated |
| `commands.rs` | `set_context_mode` | `save_context_mode` | `services::settings` | OperationalNoSignal | n/a | n/a | migrated |
| `intel_queue.rs` | `run_enrichment` | `update_account_keywords`, `update_project_keywords` | `services::intelligence` | DomainRequiredSignal | `keywords_updated` | yes | migrated |
| `intel_queue.rs` | `write_enrichment_results` | `upsert_entity_intelligence` | `services::intelligence` | DomainRequiredSignal | `entity_intelligence_updated` | yes | migrated |
| `intel_queue.rs` | `invalidate_meeting_prep_for_entity` | update `meeting_prep` via SQL | `services::meetings` | DomainRequiredSignal | `prep_invalidated` | yes | migrated |
| `processor/transcript.rs` | transcript parse writeback | `insert_capture` | `services::meetings` | DomainRequiredSignal | `transcript_outcomes` | yes | migrated |
| `processor/transcript.rs` | processing log | `insert_processing_log` | `services::integrations` | OperationalNoSignal | n/a | n/a | migrated |
| `processor/transcript.rs` | action extraction | `upsert_action_if_not_completed` | `services::actions` | DomainRequiredSignal | `action_created` | yes | migrated |
| `workflow/reconcile.rs` | meeting persistence | `upsert_meeting` | `services::meetings` | DomainRequiredSignal | `meeting_upserted` | yes | migrated |
| `hygiene.rs` | relationship reclass | `update_person_relationship` | `services::hygiene` | DomainRequiredSignal | `relationship_reclassified` | yes | migrated |
| `hygiene.rs` | content summary update | `content_index` SQL updates | `services::hygiene` | OperationalNoSignal | n/a | n/a | migrated |
| `hygiene.rs` | renewal rollover | `record_account_event` + account update SQL | `services::hygiene` | DomainRequiredSignal | `renewal_rolled_over` | yes | migrated |
| `hygiene.rs` | retry abandoned syncs | `reset_quill_sync_for_retry` | `services::hygiene` | OperationalNoSignal | n/a | n/a | migrated |
| `hygiene.rs` | name resolve | `update_person_name` | `services::hygiene` | DomainRequiredSignal | `person_name_updated` | yes | migrated |
| `hygiene.rs` | alias dedupe | `merge_people` | `services::hygiene` | DomainRequiredSignal | `people_merged` | yes | migrated |
| `hygiene.rs` | auto-merge | `merge_people` | `services::hygiene` | DomainRequiredSignal | `auto_merged` | yes | migrated |
| `hygiene.rs` | co-attendance link | `link_person_to_entity` | `services::hygiene` | DomainRequiredSignal | `account_linked` | yes | migrated |
| `hygiene.rs` | calendar resolve | `update_person_name` | `services::hygiene` | DomainRequiredSignal | `person_name_updated` | yes | migrated |

## Service Signal Gap Sweep

Rows for service-internal mutations and `let _ = emit...` conversions are tracked during Wave 3 and must be marked `migrated` before closing I512.

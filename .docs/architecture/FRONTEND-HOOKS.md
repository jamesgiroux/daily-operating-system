# Frontend Hooks Reference

> Registry of all React hooks in `src/hooks/`.
> **Auto-generated:** 2026-04-12 by `.docs/generators/gen-frontend-hooks.sh`

**40** hook files | **4616** total lines

---

## Hook Registry

| Hook | File | Lines | Tauri Commands | Events Listened |
|------|------|-------|---------------|-----------------|
| `useIsMobile` | `use-mobile.ts` | 19 | — | — |
| `useAccountDetail` | `useAccountDetail.ts` | 454 | archive_account, create_action, create_child_account, enrich_account, get_account_detail, get_account_events, get_entity_files, index_entity_files, record_account_event, update_account_programs | — |
| `useAccountFields` | `useAccountFields.ts` | 109 | update_account_field | — |
| `useAccountFieldSave` | `useAccountFieldSave.ts` | 127 | accept_account_field_conflict, dismiss_account_field_conflict, update_account_field, update_entity_metadata | — |
| `useActions` | `useActions.ts` | 219 | complete_action, create_action, get_actions_from_db, reopen_action | — |
| `useActivePreset` | `useActivePreset.ts` | 23 | get_active_preset | — |
| `useActivePreset` | `useActivePreset.tsx` | 53 | get_active_preset | preset-changed |
| `useActivitySignal` | `useActivitySignal.ts` | 48 | signal_user_activity, signal_window_focus | — |
| `useAppLock` | `useAppLock.ts` | 23 | get_lock_status | app-locked, app-unlocked |
| `useAppState` | `useAppState.ts` | 132 | clear_demo_data, get_app_state, install_demo_data, set_tour_completed, set_wizard_completed | — |
| `useBackgroundStatus` | `useBackgroundStatus.ts` | 99 | — | — |
| `useCalendar` | `useCalendar.ts` | 54 | get_calendar_events | calendar-updated |
| `useChapterObserver` | `useChapterObserver.ts` | 42 | — | — |
| `useClaudeStatus` | `useClaudeStatus.ts` | 104 | check_claude_status, clear_claude_status_cache | — |
| `useConnectivity` | `useConnectivity.ts` | 53 | get_sync_freshness | — |
| `useCopyToClipboard` | `useCopyToClipboard.ts` | 21 | — | — |
| `useDashboardData` | `useDashboardData.ts` | 144 | get_dashboard_data | calendar-updated, emails-updated, entity-updated, prep-ready, workflow-completed |
| `useDatabaseRecoveryStatus` | `useDatabaseRecoveryStatus.ts` | 32 | get_database_recovery_status | — |
| `useEnrichmentProgress` | `useEnrichmentProgress.ts` | 71 | — | — |
| `useEntityContextEntries` | `useEntityContextEntries.ts` | 72 | create_entity_context_entry, delete_entity_context_entry, get_entity_context_entries, update_entity_context_entry | — |
| `useExecutiveIntelligence` | `useExecutiveIntelligence.ts` | 80 | — | calendar-updated, workflow-completed |
| `useGleanAuth` | `useGleanAuth.ts` | 122 | disconnect_glean, get_glean_auth_status, start_glean_auth | — |
| `useGoogleAuth` | `useGoogleAuth.ts` | 114 | disconnect_google, get_google_auth_status, start_google_auth | — |
| `useInbox` | `useInbox.ts` | 101 | get_inbox_files | inbox-updated |
| `useIntelligenceFeedback` | `useIntelligenceFeedback.ts` | 103 | get_entity_feedback, submit_intelligence_feedback | — |
| `useIntelligenceFieldUpdate` | `useIntelligenceFieldUpdate.ts` | 61 | update_intelligence_field | — |
| `useMagazineShell.test.tsx` | `useMagazineShell.test.tsx` | 96 | — | — |
| `useMagazineShellProvider` | `useMagazineShell.ts` | 197 | — | — |
| `useMe` | `useMe.ts` | 132 | create_user_context_entry, delete_user_context_entry, get_user_context_entries, get_user_entity, update_user_context_entry, update_user_entity_field | user-entity-updated |
| `useMeetingOutcomes` | `useMeetingOutcomes.ts` | 51 | — | — |
| `useNotifications` | `useNotifications.ts` | 109 | refresh_emails | — |
| `usePersonality` | `usePersonality.tsx` | 66 | get_config | — |
| `usePersonDetail` | `usePersonDetail.ts` | 420 | archive_person, create_action, delete_person, enrich_person, get_entity_files, get_person_detail, index_entity_files, link_person_entity, merge_people, search_people, unlink_person_entity, update_person | intelligence-updated |
| `usePostMeetingCapture` | `usePostMeetingCapture.ts` | 111 | capture_meeting_outcome, dismiss_meeting_prompt | — |
| `useProjectDetail` | `useProjectDetail.ts` | 287 | archive_project, create_action, create_project, enrich_project, get_entity_files, get_project_detail, index_entity_files, update_project_field | intelligence-updated |
| `useRevealObserver` | `useRevealObserver.ts` | 42 | — | — |
| `useSuggestedActions` | `useSuggestedActions.ts` | 69 | accept_suggested_action, get_suggested_actions, reject_suggested_action | intelligence-updated, transcript-processed |
| `useTauriEvent` | `useTauriEvent.ts` | 26 | — | — |
| `useTeamManagement` | `useTeamManagement.ts` | 289 | accept_stakeholder_suggestion, add_account_team_member, add_stakeholder_role, create_person, dismiss_stakeholder_suggestion, get_stakeholder_suggestions, remove_account_team_member, remove_stakeholder_role, search_people, set_team_member_role, update_stakeholder_assessment, update_stakeholder_engagement | — |
| `useWorkflow` | `useWorkflow.ts` | 241 | get_execution_history, get_next_run_time, get_workflow_status, run_workflow | — |

---

## Command Usage Summary

All Tauri commands invoked from hooks:

- `get_entity_files` (5 hooks)
- `create_action` (4 hooks)
- `update_account_field` (3 hooks)
- `index_entity_files` (3 hooks)
- `get_active_preset` (3 hooks)
- `add_account_team_member` (3 hooks)
- `update_project_field` (2 hooks)
- `signal_window_focus` (2 hooks)
- `search_people` (2 hooks)
- `get_stakeholder_suggestions` (2 hooks)
- `get_person_detail` (2 hooks)
- `get_inbox_files` (2 hooks)
- `get_calendar_events` (2 hooks)
- `complete_action` (2 hooks)
- `archive_project` (2 hooks)
- `archive_person` (2 hooks)
- `archive_account` (2 hooks)
- `update_user_entity_field` (1 hooks)
- `update_user_context_entry` (1 hooks)
- `update_stakeholder_engagement` (1 hooks)
- `update_stakeholder_assessment` (1 hooks)
- `update_person` (1 hooks)
- `update_intelligence_field` (1 hooks)
- `update_entity_metadata` (1 hooks)
- `update_entity_context_entry` (1 hooks)
- `update_account_programs` (1 hooks)
- `unlink_person_entity` (1 hooks)
- `submit_intelligence_feedback` (1 hooks)
- `start_google_auth` (1 hooks)
- `start_glean_auth` (1 hooks)
- `signal_user_activity` (1 hooks)
- `set_wizard_completed` (1 hooks)
- `set_tour_completed` (1 hooks)
- `set_team_member_role` (1 hooks)
- `run_workflow` (1 hooks)
- `reopen_action` (1 hooks)
- `remove_stakeholder_role` (1 hooks)
- `remove_account_team_member` (1 hooks)
- `reject_suggested_action` (1 hooks)
- `refresh_emails` (1 hooks)
- `record_account_event` (1 hooks)
- `merge_people` (1 hooks)
- `link_person_entity` (1 hooks)
- `install_demo_data` (1 hooks)
- `get_workflow_status` (1 hooks)
- `get_user_entity` (1 hooks)
- `get_user_context_entries` (1 hooks)
- `get_sync_freshness` (1 hooks)
- `get_suggested_actions` (1 hooks)
- `get_project_detail` (1 hooks)
- `get_next_run_time` (1 hooks)
- `get_lock_status` (1 hooks)
- `get_google_auth_status` (1 hooks)
- `get_glean_auth_status` (1 hooks)
- `get_execution_history` (1 hooks)
- `get_entity_feedback` (1 hooks)
- `get_entity_context_entries` (1 hooks)
- `get_database_recovery_status` (1 hooks)
- `get_dashboard_data` (1 hooks)
- `get_config` (1 hooks)
- `get_app_state` (1 hooks)
- `get_actions_from_db` (1 hooks)
- `get_account_events` (1 hooks)
- `get_account_detail` (1 hooks)
- `enrich_project` (1 hooks)
- `enrich_person` (1 hooks)
- `enrich_account` (1 hooks)
- `dismiss_stakeholder_suggestion` (1 hooks)
- `dismiss_meeting_prompt` (1 hooks)
- `dismiss_account_field_conflict` (1 hooks)
- `disconnect_google` (1 hooks)
- `disconnect_glean` (1 hooks)
- `delete_user_context_entry` (1 hooks)
- `delete_person` (1 hooks)
- `delete_entity_context_entry` (1 hooks)
- `create_user_context_entry` (1 hooks)
- `create_project` (1 hooks)
- `create_person` (1 hooks)
- `create_entity_context_entry` (1 hooks)
- `create_child_account` (1 hooks)
- `clear_demo_data` (1 hooks)
- `clear_claude_status_cache` (1 hooks)
- `check_claude_status` (1 hooks)
- `capture_meeting_outcome` (1 hooks)
- `add_stakeholder_role` (1 hooks)
- `accept_suggested_action` (1 hooks)
- `accept_stakeholder_suggestion` (1 hooks)
- `accept_account_field_conflict` (1 hooks)

## Event Listener Summary

All Tauri events listened to from hooks:

- `intelligence-updated` (3 hooks)
- `calendar-updated` (3 hooks)
- `workflow-completed` (2 hooks)
- `inbox-updated` (2 hooks)
- `user-entity-updated` (1 hooks)
- `transcript-processed` (1 hooks)
- `preset-changed` (1 hooks)
- `prep-ready` (1 hooks)
- `entity-updated` (1 hooks)
- `emails-updated` (1 hooks)
- `app-unlocked` (1 hooks)
- `app-locked` (1 hooks)


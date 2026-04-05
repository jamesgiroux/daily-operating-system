# I377: Signal System Completeness Map

Generated: 2026-02-21

## 1. Signal Emitters

### 1A. User Edit Handlers (commands.rs + services/)

| Handler | File:Line | Entity Type | Signal Type | Source | Status |
|---------|-----------|-------------|-------------|--------|--------|
| `update_account_field` | commands.rs:4245 | account | `field_updated` | user_edit | OK |
| `update_project_field` | commands.rs:5554 | project | `field_updated` | user_edit | OK |
| `update_person` | commands.rs:3572 | person | (none) | - | **MISSING** |
| `update_account_notes` | commands.rs:4275 | account | (none) | - | **MISSING** |
| `update_account_programs` | commands.rs:4315 | account | (none) | - | **MISSING** |
| `update_project_notes` | commands.rs:5591 | project | (none) | - | **MISSING** |
| `add_account_team_member` | commands.rs:4199 | account | (none) | - | **MISSING** |
| `remove_account_team_member` | commands.rs:4217 | account | (none) | - | **MISSING** |
| `update_stakeholders` | commands.rs:7369 | varies | (none) | - | **MISSING** |
| `record_account_event` | commands.rs:6131 | account | (none) | - | **MISSING** |
| `merge_accounts` | commands.rs:5856 | account | (none) | - | **MISSING** |
| `link_person_entity` | commands.rs:3613 | varies | `person_linked` | user_action | OK |
| `unlink_person_entity` | commands.rs:3643 | entity | `person_unlinked` | user_action | OK |
| `archive_account` | commands.rs:5849 | account | `entity_archived`/`entity_unarchived` | user_action | OK |
| `archive_project` | commands.rs:5881 | project | `entity_archived`/`entity_unarchived` | user_action | OK |
| `archive_person` | commands.rs:5900 | person | `entity_archived`/`entity_unarchived` | user_action | OK |
| `update_intelligence_field` | commands.rs:7351 | varies | `user_correction` | user_edit | OK |
| `update_meeting_user_agenda` | commands.rs:6547 | varies | `prep_edited` | user_edit | OK |
| `dismiss_email_item` | commands.rs:2110 | varies | `email_item_dismissed` | item_type | OK |

### 1B. Action Handlers (services/actions.rs)

| Handler | Signal Type | Source | Status |
|---------|-------------|--------|--------|
| `complete_action` | `action_completed` | source_type | OK |
| `reopen_action` | `action_reopened` | user_correction | OK |
| `accept_proposed_action` | `action_accepted` | source_type | OK |
| `reject_proposed_action` | `action_rejected` | source_type | OK |
| `update_action_priority` | `priority_corrected` | source_type | OK |

### 1C. System/Pipeline Emitters

| Source | File | Signal Type | Entity Type |
|--------|------|-------------|-------------|
| Entity resolver | prepare/entity_resolver.rs:116,194 | `entity_resolution` | account/project |
| Meeting context | prepare/meeting_context.rs:169 | `entity_resolved` | varies |
| Orchestrate | prepare/orchestrate.rs:2102 | `thread_position` | thread |
| Transcript | processor/transcript.rs:231 | `transcript_outcomes` | varies |
| Google calendar | google.rs:712 | `person_created` | person |
| Google calendar | google.rs:816 | `meeting_cancelled` | meeting |
| Email processing | executor.rs:317 | `email_received` | person |
| Email processing | executor.rs:329 | `negative_sentiment` | person |
| Clay enricher | clay/enricher.rs:417 | `title_change`/`company_change` | person |
| Clay enricher | clay/enricher.rs:649 | varies | person |
| Gravatar | gravatar/client.rs:314 | `profile_discovered` | person |
| Proactive engine | proactive/engine.rs:93 | `proactive_*` (8 types) | varies |
| Hygiene | hygiene.rs:956 | `auto_merged` | person |
| Hygiene | hygiene.rs:1038 | `account_linked` | person |
| Hygiene | hygiene.rs:2243 | `low_confidence_match` | varies |
| Email bridge | email_bridge.rs:117 | `pre_meeting_context` | meeting |
| Email bridge | email_bridge.rs:139 | `pre_meeting_context` | account |
| Email bridge | email_bridge.rs:264+ | `email_sentiment`/`email_urgency_high`/`email_commitment` | varies |
| Cadence | cadence.rs:179 | `cadence_anomaly` | varies |
| Post-meeting | post_meeting.rs:114 | `post_meeting_followup` | account/meeting |
| Services/people | services/people.rs:70 | `entity_deleted` | person |

## 2. Propagation Rules

| Rule | File | Listens For | Produces | Has Emitter? |
|------|------|-------------|----------|-------------|
| `rule_person_job_change` | rules.rs:18 | person:`title_change`/`company_change` | account:`stakeholder_change` | YES (clay) |
| `rule_meeting_frequency_drop` | rules.rs:61 | account:`meeting_frequency` | account:`engagement_warning` | **NO EMITTER — DEAD** |
| `rule_overdue_actions` | rules.rs:105 | `proactive_action_cluster` | `project_health_warning` | YES (proactive engine) |
| `rule_champion_sentiment` | rules.rs:141 | person:`negative_sentiment` | account:`champion_risk` | YES (executor email) |
| `rule_departure_renewal` | rules.rs:193 | person:`person_departed`/`company_change` | account:`renewal_risk_escalation` | **PARTIAL — `person_departed` has NO emitter** |
| `rule_renewal_engagement_compound` | rules.rs:269 | account:`renewal_proximity` | account:`renewal_at_risk` | YES (proactive detector) |

## 3. Downstream Consumers

| Consumer | File | Reads From |
|----------|------|-----------|
| Callout generation | callouts.rs:37 | 16 signal types via `get_recent_callout_signals` |
| Cadence tracking | cadence.rs | email_signals → cadence_anomaly signals |
| Prep invalidation | invalidation.rs:31 | 8 signal types trigger meeting prep regeneration |
| Email bridge | email_bridge.rs | email_signals → pre_meeting_context signals |
| Post-meeting | post_meeting.rs | meetings_history + email_signals → post_meeting_followup |
| Scoring | scoring.rs | signal_events count per entity |
| Email scoring | email_scoring.rs | Uses scoring.rs against DbEmail |
| Relevance ranking | relevance.rs | Embedding similarity of signal values |
| Feedback/weights | feedback.rs | entity_resolution signals for correction learning |
| Hygiene rules | rules.rs:361 | `person_created`, `email_received`, `entity_resolved` |

## 4. Issues Found

### 4A. Dead Rules (no emitter)
1. **`rule_meeting_frequency_drop`** — listens for `"meeting_frequency"` signal but nothing emits it
2. **`rule_departure_renewal`** — partially dead: `"person_departed"` has no emitter (only `"company_change"` works)

### 4B. Missing Signal Emissions (user edit handlers)
1. `update_person` — writes person field, no signal
2. `update_account_notes` — writes notes, no signal
3. `update_account_programs` — writes programs, no signal
4. `update_project_notes` — writes notes, no signal
5. `add_account_team_member` — modifies team, no signal
6. `remove_account_team_member` — modifies team, no signal
7. `update_stakeholders` — modifies intelligence, no signal
8. `record_account_event` — records renewal/events, no signal
9. `merge_accounts` — merges entities, no signal

### 4C. Email Bridge Scope
The email bridge (`emit_enriched_email_signals`) correctly emits signals for both person AND account entity types (via person→account propagation at line 346). This was flagged as potentially limited to meetings but it actually handles all enriched emails with resolved entities.

### 4D. Post-meeting reliability
Post-meeting correlation (`correlate_post_meeting_emails`) correctly fires for meetings ended 1-48h ago. The `get_recently_ended_meetings` query uses `start_time <= datetime('now', '-1 hour')` as a proxy for meeting-ended, which is reasonable for 1-hour default meetings.

## 5. Remediation Applied

### 5A. Signal emissions added to user edit handlers (9 handlers)
1. `update_person` → emits `field_updated` on person (commands.rs)
2. `update_account_notes` → emits `field_updated` on account (commands.rs)
3. `update_account_programs` → emits `field_updated` on account (commands.rs)
4. `update_project_notes` → emits `field_updated` on project (commands.rs)
5. `add_account_team_member` → emits `team_member_added` on account (commands.rs)
6. `remove_account_team_member` → emits `team_member_removed` on account (commands.rs)
7. `update_stakeholders` → emits `stakeholders_updated` on entity (commands.rs)
8. `record_account_event` → emits `account_event_recorded` on account (commands.rs)
9. `merge_accounts` → emits `entity_merged` on target account (commands.rs)

### 5B. Dead rule removed
- `rule_meeting_frequency_drop` unregistered from PropagationEngine (propagation.rs)
- Rule function kept with documentation note explaining why it's dead (rules.rs)
- No code anywhere emits `"meeting_frequency"` signal type

### 5C. Prep invalidation expanded
- Added `stakeholders_updated`, `team_member_added`, `team_member_removed` to invalidation signal types (invalidation.rs)
- These user edits now trigger meeting prep regeneration for affected meetings

### 5D. Not changed (documented as known gaps)
- `person_departed` signal type has no emitter (Clay detects `company_change` instead)
- `rule_departure_renewal` still handles `company_change` path correctly; `person_departed` branch is future extension

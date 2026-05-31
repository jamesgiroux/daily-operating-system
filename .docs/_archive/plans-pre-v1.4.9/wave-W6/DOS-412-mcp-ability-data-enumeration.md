# DOS-412 MCP Ability Data Enumeration

Date: 2026-05-06

Scope: registry abilities whose `allowed_actors` include `Agent`, found with:

```bash
rg "allowed_actors" src-tauri/src/abilities -A 2
```

Result: two production Agent-allowed abilities exist today.

## Boundary Contract

MCP ability `data` is now deny-by-default at `src-tauri/src/services/sensitivity.rs`.

Every string leaf must resolve to exactly one safe class:

- Claim text: tagged carrier or bridge-provenance attested field, rendered to minimal `{ text, policy }` only after authoritative claim lookup. The rendered bytes come from the active surfaced claim row, or an audited stored projection of that row.
- Non-content metadata: explicit JSON-pointer path allowlist only, with a validator per path class.
- Drop: every other string leaf is omitted.

Tauri, worker, and eval surfaces keep the raw ability DTO and diagnostics. Serialized MCP responses omit the `diagnostics` key entirely.

## Audit Table

| Ability | Output type | Agent allowed | String fields | Classification | MCP handling |
| --- | --- | --- | --- | --- | --- |
| `get_entity_context` | `Vec<EntityContextEntry>` | Yes | `id` | Claim row identifier metadata | Allowlisted identifier |
| `get_entity_context` | `Vec<EntityContextEntry>` | Yes | `entityType`, `entityId` | Entity routing metadata | Allowlisted enum/id metadata |
| `get_entity_context` | `Vec<EntityContextEntry>` | Yes | `title` | Stored projection from claim row `claim_type`/`field_path` | Claim-attested by entry `id`/provenance, renders only if DTO bytes match the stored projection and policy permits |
| `get_entity_context` | `Vec<EntityContextEntry>` | Yes | `content` | Claim text from `IntelligenceClaim.text` | Claim-attested by entry `id`/provenance, renders as `{ text, policy }` when Public/Internal; drops when private |
| `get_entity_context` | `Vec<EntityContextEntry>` | Yes | `createdAt`, `updatedAt` | Timestamp metadata | Allowlisted timestamp metadata |
| `prepare_meeting` | `MeetingBrief.meeting: MeetingSummary` | Yes | `meeting.id` | Meeting identifier metadata | Allowlisted identifier |
| `prepare_meeting` | `MeetingBrief.meeting: MeetingSummary` | Yes | `meeting.title` | Calendar meeting title metadata | Allowlisted only at `/meeting/title` |
| `prepare_meeting` | `MeetingBrief.meeting: MeetingSummary` | Yes | `starts_at`, `ends_at` | Timestamp metadata | Allowlisted timestamp metadata |
| `prepare_meeting` | `MeetingAttendee` | Yes | `name` | Calendar attendee name metadata | Allowlisted only at `/meeting/attendees/*/name` |
| `prepare_meeting` | `MeetingAttendee` | Yes | `person_id`, `account_id` | Entity identifier metadata | Allowlisted identifier metadata |
| `prepare_meeting` | `MeetingAttendee` | Yes | `email`, `domain` | Contact metadata, not in DOS-412 MCP allowlist | Dropped on MCP |
| `prepare_meeting` | `Topic` | Yes | `title`, `detail` | LLM synthesis from evidence claims | Bridge-provenance attested; renders only if leaf bytes equal stored claim text and policy permits; otherwise drops |
| `prepare_meeting` | `Topic.subject: BriefSubjectRef` | Yes | `kind`, `id` | Subject routing metadata | Allowlisted enum/id metadata |
| `prepare_meeting` | `Topic.temporal_scope` | Yes | `state`, `occurred_at`, `window_start`, `window_end` serialized values | Enum/timestamp metadata | Allowlisted enum/timestamp metadata |
| `prepare_meeting` | `AttendeeContext` | Yes | `attendee`, `context` | LLM synthesis from evidence claims; `attendee` is not trusted as pure metadata because it comes from provider output | Bridge-provenance attested; renders only if leaf bytes equal stored claim text and policy permits; otherwise drops |
| `prepare_meeting` | `OpenLoop` | Yes | `description`, `owner` | LLM/direct synthesis from evidence claims; `owner` is not trusted as pure metadata because it comes from provider output | Bridge-provenance attested; renders only if leaf bytes equal stored claim text and policy permits; otherwise drops |
| `prepare_meeting` | `ChangeMarker` | Yes | `description` | Direct synthesis from evidence claims | Bridge-provenance attested; renders only if leaf bytes equal stored claim text and policy permits; otherwise drops |
| `prepare_meeting` | `SuggestedOutcome` | Yes | `outcome`, `rationale` | LLM synthesis from evidence claims | Bridge-provenance attested; renders only if leaf bytes equal stored claim text and policy permits; otherwise drops |

## Path-Scoped Metadata Allowlist

Implementation paths are relative to `AbilityResponseJson.data`; this table shows the full MCP response pointer with `/data` included. Every entry has a value validator. No leaf-key-only metadata rule is valid.

| JSON-pointer pattern | Class | Validator |
| --- | --- | --- |
| `/data/*/id` | Identifier | UUID or namespaced ASCII identifier |
| `/data/*/entityType` | Enum | `account`, `email`, `meeting`, `person`, `project` |
| `/data/*/entityId` | Identifier | UUID or namespaced ASCII identifier |
| `/data/*/createdAt` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/*/updatedAt` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/meeting/id` | Identifier | UUID or namespaced ASCII identifier |
| `/data/meeting/title` | Meeting title metadata | Non-empty single-line label |
| `/data/meeting/starts_at` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/meeting/ends_at` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/meeting/attendees/*/name` | Entity name metadata | Non-empty single-line label; entity-name wrapper remains the preferred explicit carrier where available |
| `/data/meeting/attendees/*/person_id` | Identifier | UUID or namespaced ASCII identifier |
| `/data/meeting/attendees/*/account_id` | Identifier | UUID or namespaced ASCII identifier |
| `/data/topics/*/subject/kind` | Enum | `account`, `email`, `meeting`, `person`, `project` |
| `/data/topics/*/subject/id` | Identifier | UUID or namespaced ASCII identifier |
| `/data/topics/*/temporal_scope` | Enum | `state` |
| `/data/topics/*/temporal_scope/point_in_time/occurred_at` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/topics/*/temporal_scope/trend/window_start` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/topics/*/temporal_scope/trend/window_end` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/attendee_context/*/subject/kind` | Enum | `account`, `email`, `meeting`, `person`, `project` |
| `/data/attendee_context/*/subject/id` | Identifier | UUID or namespaced ASCII identifier |
| `/data/attendee_context/*/temporal_scope` | Enum | `state` |
| `/data/attendee_context/*/temporal_scope/point_in_time/occurred_at` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/attendee_context/*/temporal_scope/trend/window_start` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/attendee_context/*/temporal_scope/trend/window_end` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/open_loops/*/subject/kind` | Enum | `account`, `email`, `meeting`, `person`, `project` |
| `/data/open_loops/*/subject/id` | Identifier | UUID or namespaced ASCII identifier |
| `/data/open_loops/*/temporal_scope` | Enum | `state` |
| `/data/open_loops/*/temporal_scope/point_in_time/occurred_at` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/open_loops/*/temporal_scope/trend/window_start` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/open_loops/*/temporal_scope/trend/window_end` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/what_changed_since_last/*/subject/kind` | Enum | `account`, `email`, `meeting`, `person`, `project` |
| `/data/what_changed_since_last/*/subject/id` | Identifier | UUID or namespaced ASCII identifier |
| `/data/what_changed_since_last/*/temporal_scope` | Enum | `state` |
| `/data/what_changed_since_last/*/temporal_scope/point_in_time/occurred_at` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/what_changed_since_last/*/temporal_scope/trend/window_start` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/what_changed_since_last/*/temporal_scope/trend/window_end` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/suggested_outcomes/*/subject/kind` | Enum | `account`, `email`, `meeting`, `person`, `project` |
| `/data/suggested_outcomes/*/subject/id` | Identifier | UUID or namespaced ASCII identifier |
| `/data/suggested_outcomes/*/temporal_scope` | Enum | `state` |
| `/data/suggested_outcomes/*/temporal_scope/point_in_time/occurred_at` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/suggested_outcomes/*/temporal_scope/trend/window_start` | Timestamp | RFC3339/ISO-8601 timestamp |
| `/data/suggested_outcomes/*/temporal_scope/trend/window_end` | Timestamp | RFC3339/ISO-8601 timestamp |

Claim/provenance-attested content paths are intentionally not metadata allowlist entries. Current content paths are `/data/*/title`, `/data/*/content`, `/data/topics/*/{title,detail}`, `/data/attendee_context/*/{attendee,context}`, `/data/open_loops/*/{description,owner}`, `/data/what_changed_since_last/*/description`, and `/data/suggested_outcomes/*/{outcome,rationale}`. They render only when the leaf text exactly matches the stored claim text or the audited `get_entity_context.title` stored projection, and the claim policy permits MCP rendering.

## Regression Coverage

`src-tauri/tests/dos412_mcp_ability_data_redaction_test.rs` now covers:

- Tagged Public/Internal/Confidential/UserOnly claim carriers.
- Tagged carrier stored-text mismatch, withdrawn-claim drop, and stored-text match rendering.
- Untagged top-level, nested-object, nested-array, and deeply nested strings.
- Explicit path-scoped metadata allowlist fields and validators.
- Cycle-4 named fields: `open_loops[].owner` and `attendee_context[].attendee`.
- Serialized MCP diagnostics key omission while Tauri diagnostics remain unchanged.
- Provenance-attested raw claim text rendering to `{ text, policy }`, plus mismatch drop.
- DTO sensitivity downgrade against stored Confidential sensitivity.
- Tagged carrier sibling stripping for `source_text`, `sourceSummary`, `evidenceText`, `rawText`, and `quote`.

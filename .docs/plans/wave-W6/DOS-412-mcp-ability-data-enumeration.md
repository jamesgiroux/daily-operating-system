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

- Claim text: tagged carrier or bridge-provenance attested field, rendered to minimal `{ text, policy }` after authoritative claim lookup.
- Non-content metadata: explicit key/path allowlist only.
- Drop: every other string leaf is omitted.

Tauri, worker, and eval surfaces keep the raw ability DTO and diagnostics. MCP drops diagnostics warnings entirely.

## Audit Table

| Ability | Output type | Agent allowed | String fields | Classification | MCP handling |
| --- | --- | --- | --- | --- | --- |
| `get_entity_context` | `Vec<EntityContextEntry>` | Yes | `id` | Claim row identifier metadata | Allowlisted identifier |
| `get_entity_context` | `Vec<EntityContextEntry>` | Yes | `entity_type`, `entity_id` | Entity routing metadata | Allowlisted enum/id metadata |
| `get_entity_context` | `Vec<EntityContextEntry>` | Yes | `title` | Claim-derived label from `claim_type`/`field_path` | Claim-attested by entry `id`/provenance, renders as `{ text, policy }` when Public/Internal; drops when private |
| `get_entity_context` | `Vec<EntityContextEntry>` | Yes | `content` | Claim text from `IntelligenceClaim.text` | Claim-attested by entry `id`/provenance, renders as `{ text, policy }` when Public/Internal; drops when private |
| `get_entity_context` | `Vec<EntityContextEntry>` | Yes | `created_at`, `updated_at` | Timestamp metadata | Allowlisted timestamp metadata |
| `prepare_meeting` | `MeetingBrief.meeting: MeetingSummary` | Yes | `meeting.id` | Meeting identifier metadata | Allowlisted identifier |
| `prepare_meeting` | `MeetingBrief.meeting: MeetingSummary` | Yes | `meeting.title` | Calendar meeting title metadata | Allowlisted only at `/meeting/title` |
| `prepare_meeting` | `MeetingBrief.meeting: MeetingSummary` | Yes | `starts_at`, `ends_at` | Timestamp metadata | Allowlisted timestamp metadata |
| `prepare_meeting` | `MeetingAttendee` | Yes | `name` | Calendar attendee name metadata | Allowlisted only at `/meeting/attendees/*/name` |
| `prepare_meeting` | `MeetingAttendee` | Yes | `person_id`, `account_id` | Entity identifier metadata | Allowlisted identifier metadata |
| `prepare_meeting` | `MeetingAttendee` | Yes | `email`, `domain` | Contact metadata, not in DOS-412 MCP allowlist | Dropped on MCP |
| `prepare_meeting` | `Topic` | Yes | `title`, `detail` | LLM synthesis from evidence claims | Bridge-provenance attested, renders as `{ text, policy }` when Public/Internal; drops without attestation or when private |
| `prepare_meeting` | `Topic.subject: BriefSubjectRef` | Yes | `kind`, `id` | Subject routing metadata | Allowlisted enum/id metadata |
| `prepare_meeting` | `Topic.temporal_scope` | Yes | `state`, `occurred_at`, `window_start`, `window_end` serialized values | Enum/timestamp metadata | Allowlisted enum/timestamp metadata |
| `prepare_meeting` | `AttendeeContext` | Yes | `attendee`, `context` | LLM synthesis from evidence claims; `attendee` is not trusted as pure metadata because it comes from provider output | Bridge-provenance attested, renders as `{ text, policy }` when Public/Internal; drops without attestation or when private |
| `prepare_meeting` | `OpenLoop` | Yes | `description`, `owner` | LLM/direct synthesis from evidence claims; `owner` is not trusted as pure metadata because it comes from provider output | Bridge-provenance attested, renders as `{ text, policy }` when Public/Internal; drops without attestation or when private |
| `prepare_meeting` | `ChangeMarker` | Yes | `description` | Direct synthesis from evidence claims | Bridge-provenance attested, renders as `{ text, policy }` when Public/Internal; drops without attestation or when private |
| `prepare_meeting` | `SuggestedOutcome` | Yes | `outcome`, `rationale` | LLM synthesis from evidence claims | Bridge-provenance attested, renders as `{ text, policy }` when Public/Internal; drops without attestation or when private |

## Regression Coverage

`src-tauri/tests/dos412_mcp_ability_data_redaction_test.rs` now covers:

- Tagged Public/Internal/Confidential/UserOnly claim carriers.
- Untagged top-level, nested-object, nested-array, and deeply nested strings.
- Explicit metadata allowlist fields.
- Cycle-4 named fields: `open_loops[].owner` and `attendee_context[].attendee`.
- Bridge-level MCP diagnostics drop while Tauri diagnostics remain unchanged.
- Provenance-attested raw claim text rendering to `{ text, policy }`.
- DTO sensitivity downgrade against stored Confidential sensitivity.
- Tagged carrier sibling stripping for `source_text`, `sourceSummary`, `evidenceText`, `rawText`, and `quote`.

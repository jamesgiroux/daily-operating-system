# DOS-412 Render Surface Enumeration

Source of truth: DOS-412 plan v1 and ADR-0108.

ADR-0108 surface classes:

- Tauri app: first-party user surface. Public and Internal text may render. Confidential text requires an explicit redacted affordance with click-to-reveal. UserOnly text may render only for the originating user.
- MCP server: agent-facing surface. Public and Internal text may render. Confidential and UserOnly text must be dropped because an agent cannot perform an audited reveal.
- P2 publication: external/shared publication surface. Public text may render. Internal, Confidential, and UserOnly text must be dropped unless a future publish-time confirmation flow adds a narrower rendered detail surface.
- Logs and structured telemetry: no claim text. Operational identifiers only.
- OS-level push/tray notifications: no private claim text. Public text may render; Internal and above must not be emitted outside the app window.

Policy legend:

- Render: emit text directly.
- Reveal: emit a redacted policy annotation; the UI may reveal only after an audited click.
- Drop: omit the text entirely.

| Surface | Code path | Upstream data source | Claim-derived fields | Public | Internal | Confidential | UserOnly | Current status before DOS-412 | DOS-412 handling |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Dashboard daily briefing | `src-tauri/src/commands/core.rs::get_dashboard_data`, `src-tauri/src/services/dashboard.rs`, `src/components/dashboard/DailyBriefing.tsx` | `_today/data/briefing.json`, DB dashboard assembly | Narrative briefing, agenda/callout text, meeting prep snippets | Render | Render | Drop for auto callouts | Drop unless originating user and inside app | Partial inherited filtering only | Route claim-derived values through Tauri/briefing policy; auto callouts do not reveal private text |
| Briefing prep cards | `src/components/dashboard/BriefingMeetingCard.tsx`, `src-tauri/src/commands/core.rs::get_meeting_prep`, `get_meeting_intelligence` | `meeting_prep.prep_context_json`, `FullMeetingPrep` | `intelligenceSummary`, `entityRisks`, talking points, recent wins, questions, source labels | Render | Render | Drop for auto prep callouts; Reveal only in detail pane | Drop unless originating user and inside app | None | Backend annotates claim-derived prep fields; frontend uses `ClaimTextRenderer` where wrapped |
| Meeting detail | `src/pages/MeetingDetailPage.tsx`, `src-tauri/src/commands/core.rs::get_meeting_intelligence` | `MeetingIntelligence`, `FullMeetingPrep`, transcript metadata, captures | Prep context, transcript summary, post-meeting captures/outcomes | Render | Render | Reveal | Render only for originating user | Partial prompt-side filters only | Annotate claim-derived meeting/prep text on Tauri surface; no MCP reveal |
| Meeting history detail | `src-tauri/src/commands/actions_calendar.rs::get_meeting_history_detail`, `src/pages/MeetingHistoryDetailPage.tsx` | `meetings`, `meeting_transcripts`, captures, prep context | `summary`, `prep_context`, captures/actions from transcript | Render | Render | Reveal | Render only for originating user | None | Wrap or redact transcript/prep text before UI emission |
| Meeting search | `src-tauri/src/commands/actions_calendar.rs::search_meetings`, `src-tauri/src/mcp/main.rs::search_meetings` | `meeting_transcripts.summary`, `meeting_prep.prep_context_json` | Match snippets from summary/prep | Render | Render | UI: Reveal or redacted snippet; MCP: Drop | UI: originating user only; MCP: Drop | None | Search result snippets use surface policy; MCP drops private snippets |
| Account detail | `src-tauri/src/commands/accounts_content_chat.rs::get_account_detail`, `src-tauri/src/services/accounts.rs::build_account_detail_result`, `src/pages/AccountDetailPage.tsx`, account components | `entity_assessment` projection from `intelligence_claims`, account DB columns | `intelligence.executiveAssessment`, risks, wins, current state, stakeholder insights, value delivered, company context, pull quote | Render | Render | Reveal | Render only for originating user | Projection loses sensitivity at read time | Rebuild claim-derived intelligence fields from active claims at Tauri output using render helper |
| Project detail | `src-tauri/src/commands/projects_data.rs::get_project_detail`, `src-tauri/src/services/projects.rs`, `src/pages/ProjectDetailEditorial.tsx` | `entity_assessment` projection from `intelligence_claims`, project DB columns | `intelligence.*`, description/notes when claim-backed | Render | Render | Reveal | Render only for originating user | Projection loses sensitivity at read time | Same entity-detail wrapper as account |
| Person detail | `src-tauri/src/commands/people_entities.rs::get_person_detail`, `src-tauri/src/services/people.rs`, `src/pages/PersonDetailEditorial.tsx` | `entity_assessment` projection from `intelligence_claims`, person DB columns | relationship intelligence, stakeholder engagement, risks/wins, network narrative | Render | Render | Reveal | Render only for originating user | Projection loses sensitivity at read time | Same entity-detail wrapper as account |
| Entity context notes | `src-tauri/src/commands/workspace.rs::{get_user_context_entries,get_entity_context_entries}`, `src/components/entity/ContextEntryList.tsx` | `entity_context_entries` legacy rollback table, `user_note` claims after DOS-411 | User note title/content | Render | Render | Reveal if claim-backed | Render only for originating user | DOS-411 just landed claim path; legacy read remains rollback-only | Do not rewrite legacy component in DOS-412; claim-backed read paths must use render helper |
| Email list/detail | `src-tauri/src/commands/workspace.rs::{get_all_emails,get_emails_enriched,get_entity_emails}`, `src/pages/EmailsPage.tsx`, `src/pages/InboxPage.tsx` | `emails.contextual_summary`, email signals, enriched email rows | contextual summary, signal summaries, action suggestions | Render | Render | Reveal in app | Render only for originating user | None | Annotate claim-derived summaries when backed by claims; MCP/static exports drop private text |
| Transcript and post-meeting summaries | `src-tauri/src/commands/actions_calendar.rs::{get_meeting_outcomes,get_meeting_post_intelligence}`, `src/components/meeting/PostMeetingIntelligence.tsx` | transcript processor, captures, outcomes | meeting summary, risks/wins/decisions, action rationale | Render | Render | Reveal | Render only for originating user | None | Wrap transcript-derived claim text before UI emission |
| Provenance rendering | `src-tauri/src/abilities/provenance/*`, bridge rendered provenance, Tauri/MCP ability responses | `AbilityOutput.provenance`, field attributions, source identifiers | Field explanations and any source text/excerpt carried by provenance | Render | Render | UI: Reveal if text is claim-derived; MCP: Drop | UI: originating user only; MCP: Drop | ADR-0108 actor filtering exists for MCP provenance internals; no claim-text policy | Do not touch DOS-288/DOS-320 ownership/trust files; add render policy beside trust band where fields carry claim text |
| MCP static `get_briefing` | `src-tauri/src/mcp/main.rs::get_briefing` | `_today/data/*.json` | briefing JSON, email summaries, prep snippets | Render | Render | Drop | Drop | None | Filter static JSON payloads before serialization where claim-derived annotations exist |
| MCP static `query_entity` | `src-tauri/src/mcp/main.rs::query_entity` | DB entities and `entity_assessment.executive_assessment` | `intelligence_summary` | Render | Render | Drop | Drop | None | Use MCP surface helper; no redacted private text in response |
| MCP static `list_entities` | `src-tauri/src/mcp/main.rs::list_entities` | accounts/projects/people tables | entity names/status only; not claim text unless future projections back names | Render | Render | Drop if claim-backed | Drop | Not claim-derived today | Keep enumerated; lint blocks future unwrapped claim text |
| MCP static `search_meetings` | `src-tauri/src/mcp/main.rs::search_meetings` | meeting transcript summaries and prep JSON | `summary` match snippet | Render | Render | Drop | Drop | None | Apply MCP helper; no click-to-reveal in MCP |
| MCP static `search_content` | `src-tauri/src/mcp/main.rs::search_content` | semantic index chunks | document/transcript excerpts, possible claim language | Render | Render | Drop | Drop | None | Treat snippets as MCP text excerpts; drop private claim-derived excerpts where annotated |
| MCP registry ability responses except `get_entity_context` | `src-tauri/src/mcp/main.rs::invoke_mcp_ability_tool`, `src-tauri/src/bridges/mcp.rs` | registered abilities | ability data plus rendered provenance | Render | Render | Drop | Drop | `get_entity_context` already filters via cycle-6/7 | Preserve `get_entity_context`; apply output policy to other ability payloads that carry claim-derived wrappers |
| Reports and generated publications | `src-tauri/src/commands/planning_reports.rs`, `src-tauri/src/reports/*`, report pages | report generators reading entity intelligence and captures | generated narratives, risks/wins, source excerpts | Render in app | Render in app | App: Reveal; P2/export: Drop unless confirmed | App: originating user only; P2/export: Drop | None | Reports use Tauri policy for in-app preview; exported/public copies must not include private claim text |
| Chat commands | `src-tauri/src/commands/accounts_content_chat.rs::{chat_query_entity,chat_search_content,chat_get_briefing,chat_list_entities}` | DB entity intelligence, search index, briefing JSON | entity intelligence JSON, snippets, briefing text | Render | Render | Drop for agent-like chat/tool payloads unless first-party UI wraps | Drop unless originating user in Tauri UI | None | Treat chat payloads as non-revealable command responses unless consumed by `ClaimTextRenderer` |
| Push/tray notifications | `src/hooks/useNotifications.ts`, Tauri notification plugin, background enrichment events | event payloads from sync/enrichment/meeting processors | notification title/body, meeting/enrichment summaries | Render public only | Drop text; use generic status | Drop | Drop | Mostly status-only today | Keep private claim-derived text out of notification payloads |
| Audit/log/export controls | `src-tauri/src/audit.rs`, `src-tauri/src/audit_log.rs`, privacy export commands | raw AI output audit files, structured audit JSONL, data export | raw outputs may contain claim-like language | Drop from structured logs | Drop | Drop | Drop | Existing raw AI audit stores full output in workspace `_audit` | New sensitivity reveal audit stores IDs only; no claim text in structured logs |

## MCP Ability Data Rendering

Track GG replaces the prior named-field redaction model with a deny-by-default
MCP ability-data boundary. `src-tauri/src/bridges/types.rs` renders
`AbilityResponseJson.data` for `BridgeSurface::McpTool`/`McpToolDetail` before
the response is returned. Tauri, worker, and eval surfaces continue to receive
the raw ability payload.

The MCP walker in `src-tauri/src/services/sensitivity.rs` treats every string
leaf as unsafe until one of three outcomes applies:

- Claim text: a tagged carrier or bridge-provenance-attested field resolves to
  a persisted claim, reloads authoritative sensitivity, and serializes as the
  minimal `{ text, policy }` object.
- Non-content metadata: explicit allowlist only for identifiers, enum-like
  state, timestamps, meeting title, and meeting attendee names.
- Drop: all other strings are omitted.

Authoritative claim lookup is required. The bridge opens a read-only
`ActionDb` for MCP ability data rendering and passes raw provenance into
`render_mcp_ability_data_for_surface_with_provenance`. Missing claim lookup,
malformed tags, DTO/stored sensitivity mismatch, and un-attributed raw strings
all fail closed. Tagged carrier surviving fields are allowlisted to `text` and
`policy` only; siblings such as `source_text`, `sourceSummary`, `evidenceText`,
`rawText`, `quote`, `claim_id`, `sensitivity`, and `originating_actor` are
stripped.

Diagnostics are not content-safe. MCP responses replace diagnostics with
`{ "warnings": [] }`; Tauri/worker/eval keep diagnostics unchanged.

The Agent-allowed ability audit is committed at
`.docs/plans/wave-W6/DOS-412-mcp-ability-data-enumeration.md`. Current
Agent-allowed abilities are `get_entity_context` and `prepare_meeting`.
`get_entity_context` keeps the cycle-6/7 Agent prompt-input sensitivity gate
unchanged. `prepare_meeting` keeps the raw Tauri DTO and relies on field
provenance at the MCP bridge to render Public/Internal claim text or drop it.

Regression coverage lives in
`src-tauri/tests/dos412_mcp_ability_data_redaction_test.rs`: tagged carrier
policy, top-level/nested/deep untagged string drops, explicit metadata
allowlist, provenance-attested raw claim text, cycle-4 `open_loops[].owner`
and `attendee_context[].attendee`, diagnostics drop on MCP only, DTO
sensitivity downgrade, and tagged sibling stripping.

Implementation notes:

- `services/claims.rs::claim_allowed_for_prompt_input` remains the immutable
  prompt-input boundary.
- DOS-288 ownership validation and DOS-320 `FieldAttribution.trust_band` stay
  separate. DOS-412 render policy is an output boundary.
- Unknown sensitivity values, unknown surfaces, and unrecognized strings fail
  closed as Drop.
- Future MCP metadata fields must be deliberately added to the allowlist rather
  than inherited from ability DTOs.

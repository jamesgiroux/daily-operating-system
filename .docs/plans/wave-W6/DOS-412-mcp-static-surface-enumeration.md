# DOS-412 MCP Static Surface Enumeration

Date: 2026-05-06

Scope: static MCP tools registered by `DailyOsMcp::tool_box()` in `src-tauri/src/mcp/main.rs`. Dynamic registry abilities, `get_provenance`, `request_confirmation`, `get_entity_context`, prompt-input gates, and DOS-288 ownership validation are out of scope for this Track AA sweep.

ADR-0108 MCP policy: Public and Internal claim text may render. Confidential and UserOnly claim text must be dropped because MCP has no audited click-to-reveal affordance.

## Static Tool Registry

| Tool | Returned text-bearing fields | Upstream source | Claim mapping used by Track AA | Handling |
| --- | --- | --- | --- | --- |
| `get_briefing` | `BriefingResponse.schedule`, `actions`, `emails`, `briefing` JSON string leaves | `_today/data/schedule.json`, `actions.json`, `emails.json`, `briefing.json` | Workspace-wide active claims, matched by exact/substring claim text or `legacy_projection_value` | Recursively route every JSON string leaf through `render_mcp_static_json_for_surface`; drop private leaves |
| `query_entity` | `EntityResult.intelligence_summary`, `open_actions[].title`, `upcoming_meetings[].title` | `entity_assessment.executive_assessment`, `intelligence_claims`, `actions.title`, `meetings.title` | Entity claims: `entity_summary`; action claims: `open_loop`, `suggested_outcome`, `meeting_readiness`, `entity_current_state`, `user_note`; meeting claims when action source is a meeting | Route summary and action/meeting titles through `render_mcp_static_text_for_surface`; filter dropped action/meeting rows |
| `list_entities` | `name`, `health`, `status` metadata only | `accounts`, `projects`, `people` tables | None today. Account/project/person descriptions, notes, bios, and intelligence are not emitted by this DTO. | No private claim-derived field currently emitted. Lint covers future `description`, `summary`, `content`, `text`, `snippet`, and related DTO fields. |
| `search_meetings` | `MeetingSearchItem.title`, `summary` | `meetings.title`, `meeting_transcripts.summary`, `meeting_prep.prep_context_json.intelligenceSummary` | Meeting claims: `meeting_readiness`, `open_loop`, `meeting_topic`, `meeting_event_note`, `attendee_context`, `meeting_change_marker`, `suggested_outcome`; linked account entity claims for prep summaries | Route title and match snippet through `render_mcp_static_text_for_surface`; omit private snippet/title rows |
| `search_content` | Markdown result body containing semantic `chunk_text` | `content_embeddings.chunk_text`; fallback search chunks from `content_index.summary`; transcript/document excerpts | Workspace-wide active claims, matched by exact/substring claim text or `legacy_projection_value` | Route each rendered chunk through `render_mcp_static_text_for_surface`; omit private chunks from results |

## Explicit Non-Emissions

- `query_entity` does not currently emit account/project/person `description`, `notes`, `bio`, project milestones, account company overview, or full intelligence JSON. If those fields are added to the static DTO later, they must use `render_mcp_static_text_for_surface` or `render_mcp_static_json_for_surface`.
- `list_entities` is metadata-only today and does not emit descriptions, notes, transcript excerpts, action text, prep JSON, briefing JSON, or intelligence summaries.
- `get_provenance` already uses the MCP rendered-provenance response shape and is not a static tool-box tool. This sweep coexists with that path and does not replace it.

## Enforcement

- `src-tauri/scripts/check_render_policy_coverage.sh` now checks MCP static DTO field assignments in `src-tauri/src/mcp/main.rs`, not only direct `claim.text` and `source_text` patterns.
- The lint fails when new static MCP code assigns text-bearing fields such as `summary`, `briefing`, `content`, `description`, `snippet`, `text`, `title`, `actions`, `emails`, `schedule`, or `open_actions` without the MCP static render helper or an explicit `dos412-render-policy-covered` justification.
- Per-tool regression coverage lives in `src-tauri/tests/dos412_mcp_static_surface_test.rs` and seeds Public, Internal, Confidential, and UserOnly claims for the enumerated static surfaces.

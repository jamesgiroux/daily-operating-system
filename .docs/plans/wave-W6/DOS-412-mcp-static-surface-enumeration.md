# DOS-412 MCP Static Surface Enumeration

Date: 2026-05-06

Scope: static MCP tools registered by `DailyOsMcp::tool_box()` in `src-tauri/src/mcp/main.rs`. Dynamic registry abilities, `get_provenance`, `request_confirmation`, `get_entity_context`, prompt-input gates, DOS-411 `user_note`, Track EE ability bridge data paths, and DOS-288 ownership validation are out of scope for this Track DD sweep.

ADR-0108 MCP policy: Public and Internal claim text may render when the DTO carries durable claim metadata. Confidential and UserOnly claim text must be dropped because MCP has no audited click-to-reveal affordance. Unannotated static text is never upgraded to synthetic Internal text.

## Track DD Rendering Contract

- Claim-derived DTO text uses `RenderableMcpClaimText { text, claim_id, sensitivity }`. The renderer reloads the claim by `claim_id` and applies the stored sensitivity. Stale or missing claim IDs drop.
- Non-claim DTO text uses `RenderableMcpStaticText { text, surface_class }`. Only explicit metadata classes render: account/project/person names, meeting titles/types, entity health/status/lifecycle/type, datetimes, content filenames/relative paths/types, and action priority.
- Static classes that can contain generated or private-derived prose are intentionally not allowlisted: action title, briefing narrative, email subject/snippet, meeting summary, meeting prep summary, and content chunk. Those drop unless a future projection carries claim metadata.

## Static Tool Registry

| Tool | Returned text-bearing fields | Upstream source | Metadata available at projection time | Track DD handling |
| --- | --- | --- | --- | --- |
| `get_briefing` | `BriefingResponse.schedule`, `actions`, `emails`, `briefing` JSON string leaves | `_today/data/schedule.json`, `actions.json`, `emails.json`, `briefing.json` | No claim IDs or sensitivity in the JSON files. Key path supplies only a static class. | Schedule metadata such as meeting title/type/time and names renders by allowlist. Action priority/status/dates render by allowlist. Action titles, email subjects/snippets, and briefing narrative are tagged non-allowlisted and drop. Unknown string leaves drop. |
| `query_entity` | `EntityResult.intelligence_summary`, `open_actions[].title`, `upcoming_meetings[].title`, entity metadata | `intelligence_claims`, `entity_assessment.executive_assessment`, `actions.title`, `meetings.title`, account/project/person tables | `intelligence_claims` rows have claim ID and sensitivity. Legacy `entity_assessment.executive_assessment` and `actions.title` do not. Meeting/entity metadata has a static class. | Claim-backed entity summaries render through `RenderableMcpClaimText`. Legacy summaries without a claim row drop. Action titles are tagged `ActionTitle` and drop. Meeting titles, meeting types/times, names, health/status/lifecycle render by allowlist. |
| `list_entities` | `name`, `health`, `status` metadata only | `accounts`, `projects`, `people` tables | No claim backing; fields are table metadata. Account/project/person descriptions, notes, bios, and intelligence are not emitted. | Names, health, status, and lifecycle render by explicit metadata allowlist. Future prose fields such as `description`, `summary`, `content`, `text`, or `snippet` must carry claim metadata or drop. |
| `search_meetings` | `MeetingSearchItem.title`, `summary`, account name, type/time metadata | `meetings.title`, `meeting_transcripts.summary`, `meeting_prep.prep_context_json.intelligenceSummary` | Meeting title/type/time and account name are metadata. Transcript summaries and prep intelligence summaries do not carry claim IDs. | Meeting title/type/time and account name render by allowlist. Transcript and prep snippets are tagged `MeetingSummary` / `MeetingPrepSummary` and drop. Unknown string leaves drop. |
| `search_content` | Markdown result body containing semantic `chunk_text`; file metadata | `content_embeddings.chunk_text`; fallback search chunks from `content_index.summary`; transcript/document excerpts | Search chunks and fallback summaries do not carry claim IDs. Filename, relative path, and content type are metadata. | `chunk_text` is tagged `ContentChunk` and drops without claim metadata, including paraphrased/truncated private text. File metadata is allowlisted but only emitted when a renderable chunk exists. |

## Explicit Non-Emissions

- `query_entity` does not currently emit account/project/person `description`, `notes`, `bio`, project milestones, account company overview, or full intelligence JSON. If those fields are added to the static DTO later, they must carry `RenderableMcpClaimText` metadata or use an explicitly justified non-claim class.
- `list_entities` is metadata-only today and does not emit descriptions, notes, transcript excerpts, action text, prep JSON, briefing JSON, or intelligence summaries.
- `get_provenance` already uses the MCP rendered-provenance response shape and is not a static tool-box tool. This sweep coexists with that path and does not replace it.

## Enforcement

- `src-tauri/scripts/check_render_policy_coverage.sh` now checks MCP static DTO field assignments in `src-tauri/src/mcp/main.rs`, not only direct `claim.text` and `source_text` patterns.
- The lint fails when new static MCP code assigns text-bearing fields such as `summary`, `briefing`, `content`, `description`, `snippet`, `text`, `title`, `actions`, `emails`, `schedule`, or `open_actions` without the MCP static render helper or an explicit `dos412-render-policy-covered` justification.
- Per-tool regression coverage lives in `src-tauri/tests/dos412_mcp_static_surface_test.rs` and seeds Public, Internal, Confidential, and UserOnly claims for the enumerated static surfaces. The paraphrase regression verifies that unannotated Confidential-derived content chunks drop instead of rendering as Internal.

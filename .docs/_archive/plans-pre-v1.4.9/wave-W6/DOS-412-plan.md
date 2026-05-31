# Implementation Plan: DOS-412

## Revision history
- v1 (2026-05-06) — initial L0 draft. Pulled into W6 from v1.4.1 follow-up backlog after no-deferrals call on the wave; original ticket filed as a W5 cycle-8 follow-up at commit `b5bb3bbd`.

## 1. Contract restated

DOS-412 is the output-side mirror of the W5 cycle-7 prompt-channel sensitivity sweep. W5 sealed the prompt-INPUT boundary against private claim text via a centralized Public/Internal-only gate in `services/claims.rs` (cycle 7, commit `055516ea`). The cycle-8 review noted: *"provenance paths I inspected do not carry private claim text"* — meaning the wave is not shipping a known leak, but the OUTPUT side was not exhaustively audited.

ADR-0108 governs sensitivity rendering across UI surfaces and MCP responses. The prompt-input gate is binary (text reaches the LLM or it doesn't); ADR-0108 is policy-richer per surface. Examples:

- Confidential claims may render with a "click to reveal" affordance rather than dropping
- UserOnly claims may render only for the originating workspace user / actor
- Callouts may suppress source attribution but show the conclusion
- MCP tool responses other than `get_entity_context` may have surface-specific rules

DOS-412 ships when:

- Every UI rendering surface and MCP response that consumes claim-derived text is enumerated.
- ADR-0108's per-surface policy is implemented uniformly through one centralized helper, the same way cycle-7 centralized prompt-input policy in `services/claims.rs`.
- Per-surface regression tests seed Public + Internal + Confidential + UserOnly claims and assert the rendered output matches ADR-0108 expectations.
- No surface renders Confidential or UserOnly text without the ADR-0108-prescribed affordance.

This is the same shape as the cycle-7 sweep but applied to the output boundary. Cycles 5/6/7 demonstrated that channel-by-channel patching is expensive — the lesson is to enumerate every channel before patching even one. The matching memory is `feedback_enumerate_channels_before_patching.md`.

## 2. Approach

### 2.1 Phase 1: enumeration

Before any code changes, produce an authoritative list of every code path that emits claim-derived text to a public surface or MCP response. The enumeration is the work, per the recurring-issue-class memory.

Surfaces to audit (start from this list; expand during the audit):

1. **Briefing prep callouts** — `src/components/dashboard/BriefingMeetingCard.tsx` and the prep grid render claim-text-derived content. Backend payload comes from prep-related Tauri commands.
2. **Account / Person / Project detail surfaces** — desktop UI under `src/pages/AccountDetailPage.tsx`, `PersonDetailPage.tsx`, `ProjectDetailPage.tsx`, plus `src/components/entity/*`. Backend payload from a wide set of Tauri commands.
3. **Frontend payloads from Tauri commands** — every command in `src-tauri/src/commands/` that returns claim-derived text. List must be exhaustive: search `src-tauri/src/commands/` for return types containing `claim`, `text`, `content`, `note`, `summary`, `topic`, `loop`, `outcome`, etc.
4. **MCP tool responses other than `get_entity_context`** — `src-tauri/src/mcp/main.rs` registers a tool set; every tool that returns claim-derived text needs to apply the ADR-0108 helper. `get_entity_context` already filters via cycle-7 work and is the reference example.
5. **Email / transcript summaries that render claim text inline** — wherever email or transcript artifact rendering pulls claim text (likely in transcript ingestion + email card components).
6. **Provenance fields exposed to UI or MCP** — `RenderedProvenance` payloads carry source attributions, field attributions, and trust assessments. Source `text` excerpts can carry claim-derived language. `ProvenanceBuilder` already strips MCP-redacted fields for the MCP surface; this needs a sensitivity-aware extension.
7. **Push notifications, system tray text, or any OS-level integration** — anywhere claim-derived text might surface outside the app window.

The enumeration produces a markdown table at the top of the implementation PR description with each surface, the relevant file(s), the upstream data source, and the current sensitivity-handling status (none / inherited / partial / full).

### 2.2 Phase 2: ADR-0108 policy mapping

For each enumerated surface, decide the ADR-0108-prescribed behavior. Read the ADR carefully — it likely has surface-class semantics, not a single uniform rule. Expected behaviors by sensitivity (to be confirmed against ADR-0108 text, not assumed):

- **Public**: render unconditionally on every surface.
- **Internal**: render on user-facing UI surfaces (Tauri); render on Agent MCP responses (already enforced by cycle-7); may render on read-only public dashboards if those exist.
- **Confidential**: render with an explicit "Confidential" indicator and possibly a "click to reveal" affordance on user-facing UI; do NOT render on Agent MCP responses; do NOT render in callouts that auto-summarize without explicit context.
- **UserOnly**: render only for the originating workspace user; do NOT render on Agent MCP responses; do NOT render in shared / multi-actor contexts (e.g., team-shared views if those exist).

Each surface gets a row in the policy mapping with its prescribed Public / Internal / Confidential / UserOnly behavior, derived from the ADR.

### 2.3 Phase 3: centralized helper

Mirror cycle-7's pattern. Add `services::sensitivity::render_policy_for_surface(claim, surface, actor) -> RenderDecision` (or similar shape) returning one of:

- `Render` — pass through unchanged
- `RenderRedacted { affordance: RedactionAffordance }` — render a redacted indicator with click-to-reveal or similar
- `Drop` — do not render

`RedactionAffordance` enumerates the visual treatments ADR-0108 prescribes. The helper composes: claim sensitivity, surface class, requesting actor. Unknown sensitivities or unknown surfaces fail closed (Drop).

The helper lives in Rust (`services/sensitivity.rs` or co-located with `services/claims.rs`'s prompt-input gate) and is called from every Tauri command return path and MCP tool response that emits claim-derived text. Frontend rendering consumes the helper's decision via the bridge envelope: each claim-derived field carries a `render_policy` annotation that the React layer translates into the appropriate component (text, click-to-reveal, hidden).

### 2.4 Phase 4: per-surface integration

For each surface in the enumeration, wire it through the helper:

- Tauri commands: command result type gains a `RenderableClaimText` wrapper that carries `text` + `policy`. Existing return types are migrated.
- MCP tool responses: similar wrapper; tools currently returning raw claim text upgrade to the wrapper.
- Frontend: `<ClaimTextRenderer>` component reads the policy and renders the prescribed affordance. Replaces ad-hoc claim text rendering across the surface inventory.

### 2.5 Phase 5: sweep regression

For EACH enumerated surface, add a regression that:

- Seeds one Public + one Internal + one Confidential + one UserOnly claim
- Invokes the surface (Tauri command, MCP tool, or React component test)
- Asserts the rendered output matches the per-surface ADR-0108 policy: which sensitivities render, which redact-with-affordance, which drop entirely

Per-surface tests, not a single mega-test. The rationale is the cycle-7 lesson: the sweep regression is what proves "no other channel" with a single signal. Surface-specific regressions are what catch a future regression where someone adds a NEW surface but forgets the helper call.

Add an additional lint test `dos412_render_policy_lint_test.rs` that:

- Greps `src-tauri/src/commands/` and `src-tauri/src/mcp/` for return types carrying claim-text-shaped fields without going through the helper. Fails CI on any unwrapped emission.

## 3. Key decisions

**The helper is the contract.** Every claim-text-emitting code path goes through `render_policy_for_surface`. Direct rendering of claim text in any production code path fails the new lint. This is exactly the cycle-7 ethic: one helper, called everywhere, with a sweep that proves no surface bypasses it.

**Frontend trusts the policy annotation, doesn't re-derive.** Frontend code does not look up sensitivity and decide rendering — it reads the `render_policy` attached to each text field by the backend helper. The backend is the source of truth. Frontend's job is only translating the policy decision into visual treatment.

**Click-to-reveal is a real interaction.** "Click to reveal Confidential note" is an explicit user action that should be auditable. The interaction lands a record (claim_id, user_id, revealed_at) in a `sensitivity_reveal_audit` table so we can audit "who has seen what Confidential content." This is a defensible-design feature, not a privacy violation.

**MCP responses are stricter than UI.** Agent surfaces never see Confidential or UserOnly content even with a click-to-reveal affordance — Agent has no way to "click." Cycle-6's get_entity_context Agent filter is the precedent; DOS-412 generalizes that semantic across all MCP tools.

**Enumeration is committed, not just described.** The full surface enumeration goes into a `.docs/plans/wave-W6/DOS-412-render-surface-enumeration.md` document committed alongside the helper. This document is referenced by the proof bundle and by future waves; per the channel-enumeration memory, this is what makes "next wave starts from the audit, not from rediscovering channels."

## 4. File scope

New / modified files:
- `src-tauri/src/services/sensitivity.rs` (new) — centralized helper
- `src-tauri/src/services/claims.rs` — possibly extends with sensitivity surface helpers if they're co-located
- `src-tauri/src/types.rs` (or per-command return types) — `RenderableClaimText` wrapper, `RenderPolicy` enum
- `src-tauri/src/commands/*.rs` — every command emitting claim-derived text upgraded to the wrapper
- `src-tauri/src/mcp/main.rs` — every tool emitting claim-derived text upgraded to the wrapper
- `src-tauri/src/abilities/provenance/envelope.rs` — provenance source-text fields gain the policy annotation
- `src-tauri/scripts/check_render_policy_coverage.sh` (new) — lint blocking unwrapped claim text
- `src/components/ui/ClaimTextRenderer.tsx` (new) — frontend component consuming policy annotations
- Per-surface React components (entity detail pages, briefing card, etc.) migrated to use `<ClaimTextRenderer>`
- `src-tauri/migrations/<NEXT>_sensitivity_reveal_audit.sql` — audit table for click-to-reveal events
- Per-surface regression tests under `src-tauri/tests/dos412_*` (Tauri / MCP) and `src/__tests__/dos412_*` (frontend)
- `src-tauri/tests/dos412_render_policy_lint_test.rs` — sweep + coverage lint
- `.docs/plans/wave-W6/DOS-412-render-surface-enumeration.md` — committed enumeration

Files NOT in scope:
- `services/claims.rs::620` cycle-7 prompt-input gate is unchanged; DOS-412 is about output, not prompt input.
- `prepare_meeting` synthesis sensitivity filters are unchanged; they enforce prompt boundary, which is upstream of DOS-412's output boundary.
- `get_entity_context` Agent sensitivity filter is unchanged; cycle-6 already enforces it.

## 5. Acceptance

- Surface enumeration document committed at `.docs/plans/wave-W6/DOS-412-render-surface-enumeration.md` with per-surface ADR-0108 policy mapping
- `cargo clippy --no-default-features -- -D warnings` clean
- `cargo test --no-default-features --tests` passes including:
  - Per-surface regression tests for every enumerated surface
  - `tests/dos412_render_policy_lint_test.rs` — coverage lint passes (no unwrapped emissions)
- `pnpm test` passes including frontend `<ClaimTextRenderer>` tests
- Manual smoke on a dev workspace seeded with one claim of each sensitivity per entity: every surface renders the prescribed affordance for each sensitivity
- ADR-0108 explicitly cited in the helper's doc comments with the exact section that motivated each policy decision
- Click-to-reveal audit records correctly populated when a user reveals a Confidential note

## 6. Open questions

1. ADR-0108 specifics: confirm per-surface, per-sensitivity policy table against the actual ADR text before implementing the helper. The Section 2.2 mapping above is a reasonable default but the ADR is authoritative.
2. Does the workspace currently distinguish between "originating user" vs "any workspace user" for UserOnly checks? If not, the UserOnly semantics may need a workspace-actor field added before this ticket can fully enforce.
3. Click-to-reveal: do we want a session-scoped reveal (revealed once, stays revealed for this session) or a per-render reveal (every render requires a new click)? The audit table records each event regardless; the UX policy is the question.
4. Are there read-only "public dashboard" surfaces in DailyOS today that would ever expose claims to non-workspace-user audiences? If yes, they need their own surface class in the policy table.
5. Is there an existing Tauri command return-type wrapper pattern we should reuse for `RenderableClaimText`, or does this introduce a new pattern across commands?
6. How does click-to-reveal interact with Tauri-MCP bridge: does an Agent ever indirectly see revealed content via a Tauri command result that the Agent then queries? (Probably not, but worth confirming the MCP return path doesn't leak post-reveal.)

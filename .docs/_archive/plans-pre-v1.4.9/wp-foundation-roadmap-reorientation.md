# WordPress Foundation Roadmap Reorientation

**Date:** 2026-05-15 (locked 2026-05-17 with v1.4.x renumber + reorder)
**Status:** v1.4.2 spike landed end-to-end 2026-05-17 (PR #298). v1.4.x roadmap renumbered + reordered; this doc is the canonical map.
**Authority:** [ADR-0129 — Composable surfaces: WordPress Studio as primary surface](../decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md), [ADR-0130 — Surface-independent Composition contract](../decisions/0130-surface-independent-composition-contract.md), [v1.4.2 project](https://linear.app/a8c/project/v142-personal-intelligence-engine-wordpress-foundation-5b2473379b7f), [v1.4.3 — WordPress Foundation](https://linear.app/a8c/project/v143-wordpress-foundation-1cfeb70f5e3e), [v1.4.4 — WordPress Surface Migration](https://linear.app/a8c/project/v144-wordpress-surface-migration-877aaa780177)

## Why this doc exists

Pre-2026-05-15, v1.4.3–v1.4.9 wave plans were authored with Tauri React as the primary surface target. ADR-0129 committed the program to **WordPress Studio as the primary user-facing surface**; Tauri reorients to runtime-host + dev-admin role. This doc reorients the active sequence accordingly.

On 2026-05-17, v1.4.2 W4-F landed (PR #298) — the spike that proved DailyOS substrate content can render end-to-end as a WordPress Gutenberg block. The roadmap was then renumbered + reordered to fit consumer-driven substrate development: every version ships substrate + the surfaces that consume it, in vertical-slice waves.

## v1.4.x active sequence (post-renumber, 2026-05-17)

| Version | Subject | Status |
|---|---|---|
| v1.4.0 | Abilities Runtime Spine | Completed |
| v1.4.1 | Abilities Runtime Completion | In Progress |
| v1.4.1 Path-α | Substrate Hardening (sub) | Backlog |
| **v1.4.2** | Personal Intelligence Engine: WordPress Foundation (SPIKE — complete) | DOS-655 Done; W5/W6 moved to v1.4.3 |
| **v1.4.3** | **WordPress Foundation** (primitives + theme + starter kit + stabilization + feedback infra) | Backlog (this is where work resumes) |
| **v1.4.4** | **WordPress Surface Migration** (substrate gaps + ~18 surfaces → blocks; Tauri↔WP parity) | Backlog (W0 surface audit gate before W1) |
| v1.4.5 | Workspace Memory Refactor (substrate + sources/intake surfaces) | Backlog (was v1.4.6) |
| v1.4.6 | Salience & Recommendations (substrate + suggestion surfaces) | Backlog (was v1.4.5) |
| v1.4.7 | MCP Server v2 (headless, abilities-first) | Backlog |
| v1.4.8 | Reports as Shareable Intelligence (substrate + report blocks, absorbs the report-shaped surfaces v1.4.4 deferred) | Backlog |
| v1.4.9 | Self-Healing v2 (substrate + review queue/audit surfaces) | Backlog |
| ~~v1.4.10~~ | ~~Entity Intelligence on WP Foundation (parked)~~ | **DISSOLVED 2026-05-17** — substrate folded into v1.4.4 W1; surfaces into v1.4.4 W2; UX patterns into v1.4.3 primitives; DOS-297 threading into v1.4.5 |

Open slot at v1.4.10 (renumber leaves room for a future capability decided when we get there).

## Anchored decisions (James, 2026-05-15)

1. **Many blocks, not few.** Each Tauri component or pattern gets a Gutenberg block equivalent. Primitives exist regardless. Per-page composition and user customization come at zero additional lift once primitives are in place.
2. **Inline edit affordances.** Same UX shape as current Tauri app. Pull/push of feedback or user edits captured as user-feedback claims through the substrate (not direct DB writes from WP).
3. **User-swappable themes.** N themes; surface is data-customizable; data shape decoupled from theme treatment.
4. **Tauri UI freeze.** No new UI work in Tauri from 2026-05-15 forward. Existing surfaces in stasis until WP parity (end of v1.4.4); single substrate-first transition release flips primary after that.
5. **Eventual substrate-first install path.** DailyOS ships as a WordPress plugin long-term.
6. **Surface-agnostic substrate.** Parity across agent / Claude Desktop / Cursor / WordPress / future surfaces.

## Execution constraints (locked 2026-05-17)

These constraints govern v1.4.3 and v1.4.4. v1.4.5+ inherits the same vertical-slice rule (substrate + surface paired, never substrate without consumers).

### C1. Block authoring starter kit is working code, not documentation

The playbook ships as real code: `block.json` scaffold + `render.php` template + producer template + projection rule template + shared producer→projection→renderer integration test fixture every new block plugs into. The integration test exercises the contract gap class DOS-670 caught. No new block ships without passing the kit's harness.

Translation utilities (existing Tauri TSX + CSS module → `block.json` + `render.php`) and a token-to-`theme.json` generator from the three canonical token sources reduce per-primitive authoring cost.

### C2. v1.4.5+ versions are strict-gated on v1.4.4 closing

v1.4.5+ is paired substrate+surface vertical slices. v1.4.5 cannot start until v1.4.4 closes with Tauri ↔ WP parity proven. Substrate-only work below the surface tier (in v1.4.5+ projects) is independent and can run in parallel as planned.

### C3. Runtime stays side-process; Studio compatibility is a v1.4.3 design call

Tauri (or a headless Rust binary) hosts the runtime; WP plugin signs HTTP requests to it. Studio sandbox compatibility (the marker URL drift problem the W4-F L4 spike surfaced) gets a real fix in v1.4.3 — port stability, service registration, or equivalent. PHP-extension and FFI-bridge runtime models are out of scope.

### C4. Primitives ship before composites; consumers drive substrate

v1.4.3 ships Wave 1 primitives (per `.docs/design/primitives/README.md`). v1.4.4 builds composite surfaces against real substrate — substrate gaps the surfaces need land in v1.4.4 W1 alongside, in the same wave. **No surface ships against the old data model; substrate without consumers doesn't get designed right.**

Translation cost is low because every Wave 1 primitive has a Tauri React implementation + reference HTML at `.docs/design/reference/_shared/primitives.css`. The Gutenberg translation wraps existing TSX into block.json + render.php.

## v1.4.3 — WordPress Foundation

**Mission:** ship the foundation that v1.4.4 assembles on top of.

**No new composite surfaces in v1.4.3.** `account-overview` from v1.4.2 stays as the working reference.

**Scope:**

- **Stabilization** (from v1.4.2 W4-F L4 spike findings): DOS-671 (30s disappearance), DOS-672 (reload-on-window-switch), DOS-673 (keychain miss vs error), DOS-674 (orphan keychain secrets), DOS-675 (shutdown cleanup), plus Studio sandbox compatibility per C3.
- **C1 starter kit:** real code per the spec above.
- **Wave 1 primitive blocks** (10): `Pill`, `HealthBadge`, `StatusDot`, `Avatar`, `TrustBandBadge`, `IntelligenceQualityBadge`, `FreshnessIndicator`, `ProvenanceTag`, `EntityChip`, `TypeBadge` — per `.docs/design/primitives/README.md`. Nine `integrated`; `TrustBandBadge` `proposed` (needs source promotion). UX-pattern primitives folded in from dissolved v1.4.10: DOS-9 cite-chip tooltip, DOS-11 trust-band UI, DOS-325 score bands/evidence drill-down.
- **DailyOS magazine theme** (DOS-574 from v1.4.2 W5).
- **Feedback write infrastructure**: DOS-571 W4-E user-presence nonce + DOS-573 W5-A click-bound feedback router (from v1.4.2).
- **Audit + clean-machine validation**: DOS-575 negative fixture catalog, DOS-576 SurfaceClient audit attribution, DOS-577 clean-machine Studio validation (from v1.4.2 W5/W6).

## v1.4.4 — WordPress Surface Migration

**Mission:** end of v1.4.4 = WordPress has surface parity with the Tauri app. Tauri shell can be deprecated after.

**Wave structure:**

- **W0 — Surface audit (gate)** — DOS-677. Active / carry-forward / inactive / open-questions lists from `src/router.tsx` + `.docs/design/INVENTORY.md` + nav reachability + James's call. **Gates W1+ start.**
- **W1 — Substrate gaps** — substrate the surfaces will consume, landing IN v1.4.4 so surfaces aren't shells:
  - `get_entity_intelligence` envelope (DOS-459 from dissolved v1.4.10)
  - Canonical entity touchpoints + open-loops contract (DOS-460)
  - Entity fixture harness + no-bypass checks (DOS-461)
  - Entity-detail trust-boundary hardening (DOS-477)
  - `get_daily_briefing` Read/User-only ability (from old v1.4.3 W1)
  - Meeting prep/readiness DTO (DOS-335)
  - Shared Receipt DTO + semantic feedback action surface (DOS-339, DOS-8 from dissolved v1.4.4 Claim Experience)
  - Receipt privacy/redaction rules (DOS-341)
  - Receipt vs operational audit boundary (DOS-340)
  - W0 audit may surface additional gaps; they land here.
- **W2 — Entity surfaces:** Account Detail (DOS-462), Project Detail (DOS-483), Person Detail (DOS-484), Meeting Detail, entity list shells. Metadata proposals interaction (DOS-328).
- **W3 — Briefing surfaces:** Daily Briefing block, Meeting Briefing block, FolioBar + FloatingNavIsland as block primitives.
- **W4 — Action surfaces:** Actions/Work block (DOS-514), Activity Log block (DOS-444), Action Detail, Lint Mode (DOS-445). Review queue (DOS-443), contradiction UX (DOS-318), per-claim_type render rules (DOS-447).
- **W5 — System/history surfaces:** History block, Email surfaces (subject to W0 audit), Settings surfaces (subject to W0 scope). Adversarial fixtures (DOS-446).
- **W6 — Parity proof + Tauri shell deprecation.** DOS-458 release gate concept absorbed here as parity gate. Tauri shell marked deprecated; primary flips to WordPress.

**Deferred to v1.4.8** (Reports): BookOfBusiness, EbrQbr, SWOT, AccountHealth-as-report. Not part of v1.4.4 surface migration.

**Translation reality:** most surfaces already exist as HTML + Tauri components. Migration is straightforward translation work, not new design. Token-to-`theme.json` generator and the v1.4.3 starter kit reduce per-block authoring cost. Interaction reconfiguration is normal — some Tauri patterns (keyboard-shortcut-heavy flows, custom toolbar overlays, command palette) need reconfiguring for Gutenberg's interaction model. Acceptance is "user can do the thing the Tauri surface enabled," not "WP looks identical to Tauri."

## v1.4.5 — Workspace Memory Refactor (was v1.4.6)

**Mission:** local files, AI work product, inbox, MCP ingestion feed the claim/provenance graph automatically instead of living as manual side channels.

Substrate (source registry, ingestion service, lifecycle policy, MCP/AI placement contract) + WP surfaces (sources view, source detail, entity-intake block — DOS-468/472/473 already scoped). Includes DOS-297 longitudinal topic threading from dissolved v1.4.10.

## v1.4.6 — Salience & Recommendations (was v1.4.5)

**Mission:** personal intelligence layer that decides what matters, why it matters now, and what the user may want to do next.

Substrate (`RecommendationClaim` variant, salience scoring engine, surfacing/trigger policy, feedback semantics, source/relevance learning, eval fixtures) + WP surfaces (Suggested Next Steps, What's Unusual, Why This Now blocks). Includes DOS-336 review queue extension from dissolved v1.4.4 Claim Experience.

## v1.4.7 — MCP Server v2 (Abilities-First)

**Mission:** expand the DailyOS MCP tool surface as a headless personal intelligence layer.

Now framed as the SECOND MCP path — v1.4.2 W3-C ships WP-mediated MCP; v1.4.7 is direct headless MCP from runtime to Claude Desktop / Cursor / other agents WITHOUT WordPress in the loop. The two paths coexist.

## v1.4.8 — Reports as Shareable Intelligence

**Mission:** reports as the curated, shareable output boundary — audience-aware, source-safe, claim-grounded, editable, evaluable, exportable.

Absorbs the report-shaped surfaces v1.4.4 deferred (BookOfBusiness, EbrQbr, SWOT, AccountHealth-as-report). Substrate (typed report ability outputs, source contract, generation lifecycle, regenerate/refresh/diff with edit preservation) + WP report blocks per audience.

## v1.4.9 — Self-Healing v2

**Mission:** detect → classify → plan → preview → apply or queue → verify → explain. Late-wave skillify track (DOS-540).

Substrate + WP surfaces (review queue, audit, skillify proposals).

## v1.4.10 — Open slot

Future capability decided when we get there. Could be proactive surfacing/interruption policy, longitudinal threading enhancements (if DOS-297 in v1.4.5 surfaces more substrate needs), or another capability layer.

## Sequencing rule

Strictly sequential:

1. v1.4.2 W4-F shipped (the spike) ✓
2. v1.4.3 ships foundation (no new composites)
3. v1.4.4 ships surface migration (W0 audit gates → W1 substrate gaps → W2-5 surfaces → W6 parity)
4. v1.4.5+ vertical slices, in order: Workspace Memory → Salience → MCP → Reports → Self-Healing → open slot

Substrate-only work below the surface tier in any version is not gated on this chain.

## L0 retention

No new full L0 panel triggered by this reorientation. Substrate L0s remain valid. Targeted re-review at per-version restart:

- `/cso` re-review when first new-version block design lands (trust-band-in-block rendering + edit-as-feedback-claim wire + DOS-510 inheritance for block context).
- `/plan-eng-review` for block granularity decisions per version.
- v1.4.7 W2-A requires CSO-approved L0 amendment per DOS-624 + v1.4.3 actor-exposure carve-out + coordination with v1.4.2 W3-C MCP path.

## Already-filed maintenance issues this doc supersedes

- DOS-624 — v1.4.7 W2-A CSO L0 amendment for MCP exposure (still required at v1.4.7 implementation kickoff).
- DOS-625 — v1.4.5/v1.4.6 (now v1.4.6) W3 plan substrate citations (close on this doc landing).
- DOS-626 — v1.4.5/v1.4.6 W4-B briefing surface dependency (moot post-reorientation).
- DOS-627 — v1.4.5/v1.4.6 plan DOS-510 inheritance (subsumed; DOS-510 inheritance applies to all block rendering on the v1.4.2 surface bridge, codified in C1 starter kit).
- DOS-676 — pre-push hook structurally blocks pushes via GitHub SSH idle timeout (DONE in PR #299).

## Open architectural questions

Captured for the starter kit (C1) authoring step:

1. **Block granularity strategy.** One composed `dailyos/daily-briefing` block vs many small blocks the user composes? Per anchored decision #1: many blocks. Implies a "default page composition" template per surface (briefing, meeting, account, project) that ships out-of-box but is editable.
2. **Edit affordance per block.** Inline (per anchored decision #2). Pull/push as user-feedback claim. Wire shape needs codification: "edit captured as `FeedbackAction::ClaimCorrection` (or analogous), routed through SurfaceClient → substrate, returned projection re-renders."
3. **Theme contract.** What does a theme own (color, typography, density, editorial chrome) vs what does the substrate render (data, trust band, provenance, claim text)? Token boundary needs to be sharp before second theme exists.
4. **Headless agent parity.** Per anchored decision #6, surface-agnostic substrate. Block producers run on read; the same producer should be invocable from MCP for headless agents. Producer/renderer split per ADR-0130 already supports this; needs to be exercised cross-surface in W4 evidence.
5. **Tauri end-state.** Per anchored decision #4, freeze + stasis. Per anchored decision #5, eventual WP plugin install. The transition release that flips primary needs its own scope: bundle path (Tauri continues hosting runtime + MCP), UI deprecation (gradual or all-at-once), data migration (likely none — substrate is shared). Defer to that release's planning when v1.4.4 closes.

## Reading order for downstream work

1. ADR-0129 (composable surfaces decision)
2. ADR-0130 (composition contract)
3. v1.4.2 project description — particularly Wave 4 outcomes
4. This doc
5. v1.4.2 W4-F proof artifacts (PR #298 — `account-overview` block end-to-end)
6. `.docs/design/INVENTORY.md`, `.docs/design/primitives/README.md`, `.docs/design/patterns/`, `.docs/design/surfaces/`, `.docs/design/tokens/`
7. The block authoring starter kit (ships during v1.4.3, post-v1.4.2 W4-F evidence)
8. The version-specific wave plan

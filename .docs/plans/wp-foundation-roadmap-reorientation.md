# WordPress Foundation Roadmap Reorientation

**Date:** 2026-05-15
**Status:** Active reorientation; per-version surface plans deferred until v1.4.2 W4 lands
**Authority:** [ADR-0129 — Composable surfaces: WordPress Studio as primary surface](../decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md), [ADR-0130 — Surface-independent Composition contract](../decisions/0130-surface-independent-composition-contract.md), [v1.4.2 project (Personal Intelligence Engine: WordPress Foundation)](https://linear.app/a8c/project/v142-personal-intelligence-engine-wordpress-foundation-5b2473379b7f)

## Why this doc exists

The wave plans for v1.4.3, v1.4.4, v1.4.5, v1.4.6, and v1.4.7 were authored at different times with different assumptions about the user-facing surface. v1.4.3 still talks D-spine cutover, FolioBar, AtmosphereLayer, MagazinePageLayout — Tauri React shell terminology. v1.4.4/4.5 lane bodies describe React components. v1.4.7 W2-A wraps `get_daily_briefing` for MCP without acknowledging the WordPress-mediated MCP path that v1.4.2 W3-C already establishes.

ADR-0129 (and the v1.4.2 project that implements it) commits the program to a different surface strategy: **WordPress Studio is the primary user-facing surface; Tauri reorients to runtime-host + dev-admin role.** This doc reorients v1.4.3+ accordingly.

This is not a re-plan. The L0-approved substrate work in each version is unchanged. The reorientation is a target translation: Tauri React surfaces → DailyOS Gutenberg blocks. User-facing intent and substrate consumption are identical.

## Anchored decisions (from user 2026-05-15)

1. **Many blocks, not few.** Each Tauri component or pattern gets a Gutenberg block equivalent. Block primitives exist regardless. Per-page composition and user customization come at zero additional lift on the DailyOS side once primitives are in place.
2. **Inline edit affordances.** Same UX shape as current Tauri app. Pull/push of feedback or user edits is mostly decided as a user-feedback claim path (substrate writes through the existing claim/feedback path, not direct DB writes from WP).
3. **User-swappable themes.** DailyOS may ship 3 — or 10 — themes. The surface is data-customizable; data shape decoupled from theme treatment.
4. **Tauri UI freeze.** No new UI work in Tauri from 2026-05-15 forward. Existing Tauri surfaces remain in stasis. As WP equivalents ship, no active decommissioning of Tauri surfaces is planned — they sit alongside until WP parity, then a single substrate-first transition release flips primary.
5. **Eventual substrate-first install path.** DailyOS will eventually ship as a WordPress plugin; substrate install path = WP plugin install. No independent install path required long-term.
6. **Surface-agnostic substrate.** Parity across agent / Claude Desktop / Cursor / WordPress / future surfaces. The substrate doesn't know which surface is rendering it.

## What this means version-by-version

| Version | Substrate (unchanged, L0-blessed) | Surface (reorients to blocks) | Tauri freeze impact |
|---|---|---|---|
| **v1.4.2** | (this is the foundation work) | Magazine theme, `dailyos/account-overview` block (W4) — reference implementation for downstream block authoring | n/a (v1.4.2 IS the WP foundation) |
| **v1.4.3** | `get_daily_briefing` ability (Read/User-only); meeting prep/readiness DTO (DOS-335); `services::briefing::*`; signals + lifecycle; DOS-510 hostile-input/redaction gates | Daily-briefing rendering reorients from D-spine Tauri cutover → composed set of DailyOS Gutenberg blocks. **Surface lanes parked until v1.4.2 W4 ships and a block authoring playbook is in hand.** | D-spine cutover (Wave 3) **frozen**. Substrate W0/W1/W2 substrate work proceeds. |
| **v1.4.4** | Shared Receipt DTO; semantic feedback action surface; Actions/Work service substrate; `services::claim_receipt::*` | Actions/Work page → `dailyos/actions-work` block. Receipt rendering UI → block primitive. **Surface lanes parked until v1.4.2 W4 lands → playbook/pattern evidence in hand.** | Actions/Work React page **frozen** as authored target. |
| **v1.4.5** | RecommendationClaim variant + `CLAIM_TYPE_REGISTRY` extension; salience scoring engine; surfacing policy; trigger policy; deviation/engagement substrate; eval harness | W3-A "Suggested Next Steps" surface contract → `dailyos/suggested-next-steps` block; W3-B cross-surface rendering → block embedding rules; W4-B "What's unusual" → `dailyos/whats-unusual` block. **Surface lanes parked until v1.4.2 W4 lands → playbook/pattern evidence in hand.** | W3 surface lanes **frozen** as Tauri targets. Substrate W1/W2/W4-A/W4-C/W5 proceed. |
| **v1.4.6** | Workspace memory refactor + ADRs + retrieval primitives + indexing pipeline | **Already correctly Gutenberg-targeted:** v1.4.6 W4 includes DOS-468 (`wp/dailyos/blocks/entity-intake/`), DOS-472, DOS-473 — each with explicit `v1.4.2 W4 block.json scaffolding pattern + trust-band-render helper` dependency lines. This is the actual second-block exercise post-v1.4.2 W4. **Scope discovery:** v1.4.2 explicit non-goal routes "bidirectional markdown↔substrate ingestion" to v1.4.6; current v1.4.6 plan does not document the markdown-back-to-claim-store side. Filed as [DOS-628](https://linear.app/a8c/issue/DOS-628). | No new Tauri surface lanes; existing v1.4.6 W4 lanes already target WP blocks correctly. |
| **v1.4.7** | MCP runtime, ability proxy, host-selection contract, write-path tools | **Reframed:** v1.4.7 is the **second MCP path** — direct headless MCP from runtime to Claude Desktop / Cursor without WordPress in the loop. v1.4.2 W3-C already ships the WordPress-mediated MCP server. v1.4.7 W2-A briefing tools must coordinate with v1.4.2's path (DOS-624 already filed). | No surface impact (v1.4.7 surfaces are headless agents). |

## What still needs to happen before v1.4.3+ surface work restarts

The dependency chain is strictly sequential: v1.4.2 W4 ships → playbook is authored from W4 evidence → v1.4.3+ surface lanes restart with both in hand. Substrate work in any version is not gated on this chain.

1. **First block validates the model.** v1.4.2 W4 ships `dailyos/account-overview` end-to-end: producer ability, renderer, trust band rendering, provenance refs, fallback projection, edit/save round-trip. Until this is proven, downstream block design is speculative.
2. **Second block exercises the pattern under different conditions.** v1.4.6 W4 (DOS-468 `wp/dailyos/blocks/entity-intake/` + DOS-472 + DOS-473) is *already shipping* a Gutenberg block — it explicitly depends on v1.4.2 W4's `block.json` scaffolding pattern + trust-band-render helper (see v1.4.6-waves.md `Surface Bridge dependency line` at multiple sites). v1.4.6 W4 is the actual second-block exercise post-W4, not v1.4.4 as the initial reorientation framing implied. This is good: the pattern gets proven on two different intelligence shapes (account overview + workspace memory intake) before v1.4.3/4.4/4.5 surfaces restart.
3. **DailyOS Gutenberg block authoring playbook.** Authored from v1.4.2 W4 + v1.4.6 W4 evidence — capturing producer/renderer split, trust-band-inside-block rendering, provenance reference handling, fallback projection rules, edit affordance pattern, edit→feedback-claim wire format, theme-token consumption rules, performance budgets (re-invocation on read vs cached projection). May not need to be a separate written doc if v1.4.6 W4's `block.json` consumption pattern proves repeatable directly; either way, the convention exists and is documented somewhere before v1.4.3+ surface lanes restart. Strict ordering: W4 → playbook (or pattern evidence) → restart.
4. **Block granularity per version.** Per anchored decision #1, many blocks. The per-version block list is a planning step that happens after the playbook (or equivalent pattern evidence) exists. Likely shape:
   - v1.4.3: `dailyos/daily-briefing-overview`, `dailyos/meeting-briefing`, `dailyos/prep-status`, `dailyos/finis-marker` (if not theme), and small primitives (entity link chip, claim summary, trust band).
   - v1.4.4: `dailyos/actions-work`, `dailyos/claim-receipt`, `dailyos/activity-log`, `dailyos/lint-mode`.
   - v1.4.5: `dailyos/suggested-next-steps`, `dailyos/whats-unusual`, `dailyos/why-this-now-popover`.
5. **Theme inventory.** Per anchored decision #3, ship N themes. Initial v1.4.2 magazine theme is the reference; subsequent themes are downstream (no version assigned yet).
6. **Per-version Linear re-issue.** Each version's surface-specific issues need re-shaping or splitting (block-X delivery vs page-X composition vs surface-region-X salvage). Defer until pattern evidence exists.

## L0 retention

**No new full L0 panel triggered by this reorientation.** Substrate L0s in each version remain valid (surface-agnostic contracts). Per `feedback_l2_path_alpha_to_maintenance_project`, new findings introduced by the reorientation file as Linear maintenance issues against each version, not as cycle-N+1 L0 amendments.

**Targeted re-review at per-version restart:**
- `/cso` re-review when first new-version block design lands (covers trust-band-in-block rendering + edit-as-feedback-claim wire + DOS-510 inheritance for block context).
- `/plan-eng-review` for block granularity decisions per version (architectural, not safety; light touch).
- v1.4.7 W2-A specifically requires CSO-approved L0 amendment per DOS-624 + the v1.4.3 actor-exposure carve-out + coordination with v1.4.2 W3-C MCP path.

## Already-filed maintenance issues this doc supersedes

- [DOS-624](https://linear.app/a8c/issue/DOS-624) — v1.4.7 W2-A CSO L0 amendment for MCP exposure (subsumes one part of the reorientation; resolution still required at v1.4.7 implementation kickoff).
- [DOS-625](https://linear.app/a8c/issue/DOS-625) — v1.4.5 W3 plan v1.4.3 substrate citations (becomes moot once both versions reorient; close on this doc landing).
- [DOS-626](https://linear.app/a8c/issue/DOS-626) — v1.4.5 W4-B briefing surface dependency (same — moot post-reorientation).
- [DOS-627](https://linear.app/a8c/issue/DOS-627) — v1.4.5 plan DOS-510 inheritance (subsumed by reorientation: DOS-510 inheritance applies to ALL block rendering on the v1.4.2 surface bridge, codified in the block authoring playbook when authored).

## Open architectural questions (post-W4)

Captured for the playbook authoring step; not blocking substrate work:

1. **Block granularity strategy.** One composed `dailyos/daily-briefing` block vs many small blocks the user composes? Per anchored decision #1: many blocks. Implies a "default page composition" template per surface (briefing, meeting, account, project) that ships out-of-box but is editable.
2. **Edit affordance per block.** Inline (per anchored decision #2). Pull/push as user-feedback claim. Wire shape needs codification: "edit captured as `FeedbackAction::ClaimCorrection` (or analogous), routed through SurfaceClient → substrate, returned projection re-renders."
3. **Theme contract.** What does a theme own (color, typography, density, editorial chrome) vs what does the substrate render (data, trust band, provenance, claim text)? Token boundary needs to be sharp before second theme exists.
4. **Headless agent parity.** Per anchored decision #6, surface-agnostic substrate. Block producers run on read; the same producer should be invocable from MCP for headless agents. Producer/renderer split per ADR-0130 already supports this; needs to be exercised cross-surface in W4 evidence.
5. **Tauri end-state.** Per anchored decision #4, freeze + stasis. Per anchored decision #5, eventual WP plugin install. The transition release that flips primary needs its own scope: bundle path (Tauri continues hosting runtime + MCP), UI deprecation (gradual or all-at-once), data migration (likely none — substrate is shared). Defer to that release's planning when the time comes.

## Reading order for downstream version work

Anyone touching v1.4.3+ surface work should read in this order before drafting:
1. ADR-0129 (composable surfaces decision)
2. ADR-0130 (composition contract)
3. v1.4.2 project description (Linear) — particularly Wave 4 outcomes
4. This doc
5. v1.4.2 W4 proof artifacts when they land (account-overview block end-to-end evidence)
6. The DailyOS Gutenberg block authoring playbook (TBA, post-W4)
7. The version-specific wave plan

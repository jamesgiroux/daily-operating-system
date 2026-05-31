# WP-skill-grounded reviewer — v1.4.4 wave-level L0 packet (cycle 1)

**Reviewer scope:** WordPress / Gutenberg / theme.json correctness for the wave-level L0 packet at `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md`. Grounded in `/Users/jamesgiroux/.claude/skills/wp-block-development` + `/Users/jamesgiroux/.claude/skills/wp-block-themes` + memory `feedback_wp_skill_grounded_reviewer_for_wp_l0`. Comparator: `wp/dailyos/blocks/account-overview/block.json` (v1.4.2 W4-F reference) and `wp/dailyos/theme/theme.json` (v1.4.3 generated magazine theme).

---

## VERDICT: CONDITIONAL APPROVE

The wave-level shape (W0 audit → W1 substrate → W2–W5 surfaces → W6 parity proof + Tauri flag-flip) is sound and the substrate-in-same-wave + many-blocks-not-few invariants are correctly stated. The acknowledged anchored decisions on block granularity (outer + N inner) and cursor pagination are idiomatic Gutenberg. **Conditional** because the packet leaves four WP-specific decisions for sub-L0 packets that need wave-level guard rails BEFORE W2 enters sub-L0 (otherwise each sub-L0 reinvents them inconsistently across Account / Project / Person / Meeting / Briefing). The conditions are recorded as findings 1–4 below; once the packet adopts the recommended wave-level invariants, this reviewer approves.

Wave packet correctly does NOT design block.json or render.php per surface — that work belongs in sub-L0. The findings below are wave-level guard-rails the sub-L0 packets need to inherit, not premature implementation detail.

---

## Findings

### Finding 1 — High — §13 anchored decision #1 (outer + N inner) needs an inner-block-model invariant

**Section cited:** §13 "Resolutions locked 2026-05-20" #1 (block granularity for entity-detail composites) and §5.2 W2 open questions bullet 1.

**WP concern.** The packet locks "one outer `dailyos/account-detail` block, N inner pieces, user reorders inside it" but does not pick between the two idiomatic Gutenberg patterns:

- **Pattern A — `InnerBlocks` + `template` + `templateLock`** (`useInnerBlocksProps()` in edit.js; `<InnerBlocks.Content />` in save.js or `do_blocks( $content )` in render.php). User can add/remove/reorder inner blocks; the outer block stores its own attributes; inner blocks store theirs separately in post content. Idiomatic for composable layouts. Reference: Block Editor Handbook, "Nested Blocks: Inner Blocks" — https://developer.wordpress.org/block-editor/reference-guides/block-api/block-api/block-api-edit-save-inner-blocks/ and `references/inner-blocks.md` in `/Users/jamesgiroux/.claude/skills/wp-block-development`.
- **Pattern B — `parent` field on inner blocks** (in `block.json`: `"parent": ["dailyos/account-detail"]`). Restricts inserter so the inner-only blocks only appear inside the named parent. Combines with Pattern A; does not replace it.

These are NOT alternatives — they compose. Pattern A defines the container; Pattern B scopes the inner-block inventory. The packet's anchored decision is silent on:

1. Whether the outer block declares an `InnerBlocks` template with `templateLock: false` (full reorder) vs `templateLock: "insert"` (no reorder, only edit) vs `templateLock: "all"` (frozen) vs `false` (user can also remove). Most magazine-detail layouts want `false` for power-user composition + a synced-pattern default for casual users.
2. Whether the v1.4.3 primitive blocks (`Pill`, `EntityChip`, `TrustBandBadge`, etc.) declare `parent` ⇒ exclusive to entity-detail outers, or stay inserter-global so a user can drop a `TrustBandBadge` into a vanilla post. The packet's "many blocks, not few" rule plus "user reorders inside it" strongly implies global, but doesn't say.
3. Whether server-side rendering of the outer composite calls `do_blocks( $content )` in render.php (preserves inner-block reorders) or hard-codes a render order from the ability output (reorder is illusory).

The existing W4-F `wp/dailyos/blocks/account-overview/block.json` (the comparator) uses **no inner blocks at all** — single-block monolith with `"supports": { "html": false, "reusable": false, "inserter": true }` and a render.php that reads `account_id`/`composition_id` and emits the whole composition. W2 is a different model from the v1.4.2 reference; the wave packet should say so explicitly.

**Recommended fix.** Add an architecture invariant to §10 (new row):

> **Outer/inner block contract.** Entity-detail composites (Account / Project / Person / Meeting) use Gutenberg `InnerBlocks` with `templateLock: false` and a default `template` array enumerating the chapters in canonical order. Inner blocks register WITHOUT a `parent` field — primitive blocks stay inserter-global so they compose into vanilla posts too (per ADR-0129 §2 "WordPress is the composition layer"). render.php for outer blocks calls `do_blocks( $content )` to render reordered inner blocks; the outer block's render.php is responsible only for chrome (subject binding, section dividers, ability invocation that feeds inner blocks via block context). The v1.4.2 `dailyos/account-overview` monolith is preserved as-is for backward compatibility; v1.4.4 W2 introduces `dailyos/account-detail` as the new outer block, NOT a rewrite of `account-overview`.

This also resolves the implicit question "what about block context?" — outer block uses `providesContext` (e.g. `{ "dailyos/accountId": "account_id" }`) and inner blocks declare `usesContext: ["dailyos/accountId"]`. The packet should name this as the substrate-to-inner-block plumbing because each inner chapter needs the entity id without re-passing it through attributes.

---

### Finding 2 — High — §13 anchored decision #2 (cursor pagination) needs WP-side hook contract

**Section cited:** §13 #2 (entity list pagination) and §5.2 W2 open questions bullet 2.

**WP concern.** "Server-side pagination via cursor. List abilities return one page + `next_cursor`; block invokes ability again on scroll/filter change." This is the right shape but the WP-side mechanism is ambiguous. The three idiomatic options:

1. **REST endpoint + `useEntityRecords` from `@wordpress/core-data`** — built-in pagination/caching but assumes WordPress post-type semantics (`?page=2`). Doesn't natively support opaque cursors; requires custom REST route. Reference: https://developer.wordpress.org/block-editor/reference-guides/data/data-core/ — `core-data` is post-shaped.
2. **WP Abilities API client-side `executeAbility()` (WP 7.0)** — calls the registered ability over REST; ability returns the cursor in its output. Matches ADR-0129 §4's "WP path: abilities via WP Abilities API + MCP Adapter" exactly. The 2026-05-20 client-side merge in Gutenberg 7.0 is the canonical path. Reference: ADR-0129 §4 amended 2026-05-10; https://developer.wordpress.org/news/2025/11/introducing-the-wordpress-abilities-api/.
3. **Custom hook (`useAbilityCursor( ability, args )`)** wrapping `executeAbility()` + `useState` for cursor + `IntersectionObserver` for auto-load. Block-local.

Pattern 2 is the architecturally correct one per ADR-0129 §4 (the substrate-to-WP transport is abilities, not post-types). Pattern 1 will tempt sub-L0 authors because `useEntityRecords` is more familiar — and it's wrong here because it bypasses the ability gate (rate budget, render-path authorization per §10 invariant "render-path authorization mandatory before cache lookup"). Pattern 3 builds on Pattern 2 with the cursor state machine the packet wants.

**Recommended fix.** Add to §10 (new row) and/or extend §13 #2:

> **Entity list pagination contract.** List shapes (account list, person list, project list, action list, history list) consume substrate via `executeAbility()` (WP 7.0 client-side Abilities API per ADR-0129 §4), NOT via `@wordpress/core-data` `useEntityRecords`. List abilities return `{ items, next_cursor: Option<String>, total_hint: Option<u64> }`; opaque cursor is server-encoded (no client should parse it). W2 sub-L0 ships a shared `useAbilityCursor()` hook under `wp/dailyos/blocks/_shared/`; list blocks consume it. Cursor invalidation on signal-driven refresh follows the v1.4.2 W4-F cache discipline (carried forward per §10 "producer commit on cache miss") — cursor resets when watermark changes.

Without this guard rail, three W2 sub-L0 packets will independently pick three patterns and pagination drifts.

---

### Finding 3 — Medium — theme.json + §13 #8 `dailyos_project` tint resolution under-specified for the W2 sub-L0

**Section cited:** §13 deferred item #8 (`dailyos_project` tint resolution) and §5.2 W2 open questions bullet 3.

**WP concern.** The packet defers tint resolution to W2 sub-L0 with `ce-design-lens-reviewer` as the panelist but doesn't name the WP-specific surface area. There are two coupled decisions:

1. **CPT registration.** `dailyos_project` as a registered custom post type (so each project is a WP post with a tint stored as post-meta) vs. a substrate-only entity (project lives only in the DailyOS substrate; WP just renders blocks parameterized by `project_id` attribute). The comparator `wp/dailyos/blocks/account-overview/block.json` takes the **substrate-only** approach (account is not a CPT; the block has an `account_id` attribute). Project Detail should match unless there's a CMS reason to make `dailyos_project` a CPT (URL routing, comments, taxonomies, REST exposure).
2. **theme.json palette emission for per-project tints.** The current `wp/dailyos/theme/theme.json` palette has hard-coded color slugs (e.g. `project`, `project-10`, `project-6`, `project-8`) — one color per entity type, not per project instance. Per-project tints can't be a palette slug because palettes are static. Two viable WP-native paths:
   - **Inline CSS variable** set on the outer-block wrapper from the substrate (e.g. `style="--dailyos-project-tint: #...;"` injected by render.php from claim data). theme.json palette stays generic; the per-project tint flows through a CSS custom property the primitives consume.
   - **Block style variations** generated dynamically — not idiomatic Gutenberg (block style variations are theme-time, not runtime).

The packet doesn't tell the W2 sub-L0 which path to evaluate.

**Recommended fix.** Add explicit guidance to §13 deferred #8:

> Per-project tint flows as a CSS custom property on the outer-block wrapper (`style="--dailyos-project-tint: <hex>;"` from render.php, sourced from the substrate's project claim), NOT as a theme.json palette slug or block style variation. Sub-L0 must justify any deviation. CPT decision (`dailyos_project` registration) is orthogonal to tint mechanism and stays open for W2 sub-L0 — default-no per the comparator (`account-overview` doesn't CPT-ify accounts); justify yes if a non-substrate reason emerges (likely none).

Memory `feedback_no_inline_css` does NOT apply: CSS custom-property values on a wrapper element computed from substrate data is the narrow exception called out in that memory ("runtime-computed values via CSS custom property"). State this explicitly so the W2 sub-L0 reviewer doesn't flag it.

---

### Finding 4 — Medium — §5.6 W6 Tauri shell flag-flip needs externalBin acknowledgement

**Section cited:** §5.6 W6 + §13 #5 (Tauri shell deprecation = flag-flip at W6).

**WP concern.** The flag-flip story is "hide Tauri React UI behind a build flag." Per CLAUDE.md "Gotchas":

> `Tauri externalBin`: `build-mcp.sh` creates empty stub BEFORE `cargo build`, overwrites after

Confirmed at `src-tauri/tauri.conf.json` line 33: `"externalBin": ["binaries/dailyos-mcp"]`. The Tauri shell is not purely a UI container — it ships the dailyos-mcp binary as an external bin, and the runtime stays alive even when UI is hidden. The packet correctly notes (§13 #5 + §10 invariants + memory `feedback_tauri_ui_freeze`) that "Tauri continues hosting runtime + MCP + keychain + dev/admin." Good. But the flag-flip PR design needs to:

1. Confirm the build flag (e.g. `VITE_HIDE_LEGACY_UI=true` or a Cargo feature flag) hides ONLY React UI routes; does NOT alter `externalBin` packaging or the `build-mcp.sh` stub-create-before-cargo-build dance.
2. Decide whether the Tauri tray / dock / window remains for runtime-host duties (preferred — admin/status surface stays) or whether Tauri runs fully background-headless (more invasive; affects keychain prompts).
3. The W6 parity-proof artifact (§5.6 + §13 #4) needs to include a "runtime-still-running" smoke test post-flip, not just "UI is hidden."

**Recommended fix.** Tighten §5.6 W6 acceptance shape:

> Tauri shell deprecation flag-flip MUST preserve `src-tauri/tauri.conf.json` `externalBin` packaging and the `build-mcp.sh` stub-create-before-cargo-build sequence (per CLAUDE.md Gotchas). Flag-flip hides React UI routes only; runtime, MCP server, keychain, and admin/status surfaces remain reachable through a minimal Tauri window or system tray. Parity-proof artifact for W6 includes runtime-still-running smoke test (MCP server responds; keychain reachable; signal propagation fires) AFTER flag-flip applied, not only before.

---

### Finding 5 — Low — `dailyos` block category registration is acknowledged substrate but worth naming

**Section cited:** §6 "Substrate consumed → From v1.4.3."

**WP concern.** `wp/dailyos/includes/class-dailyos-plugin.php` lines 74 + 128–137 already register the `dailyos` block category via `block_categories_all` filter. The wave packet's §6 substrate-consumed inventory lists v1.4.3 primitives and starter kit but doesn't name the block category registration as inherited substrate. Any new block.json declaring `"category": "dailyos"` requires this filter to be live; sub-L0 packets adding new blocks need to know it's already in place and NOT re-register.

**Recommended fix.** Add to §6 "From v1.4.3 (WordPress foundation)":

> - `dailyos` block category registration (`block_categories_all` filter in `class-dailyos-plugin.php`) — consumed unchanged by all new W2–W5 blocks; do NOT re-register.

Minor housekeeping; prevents two sub-L0 packets independently adding the filter.

---

### Finding 6 — Low — apiVersion 3 is correct in the comparator, worth naming as a wave invariant

**Section cited:** §10 (architecture invariants).

**WP concern.** WordPress 6.9 (Dec 2025) enforces `apiVersion: 3`; 7.0 (May 2026) runs the post editor iframed regardless of block apiVersion. The v1.4.2 comparator `wp/dailyos/blocks/account-overview/block.json` is correctly at `"apiVersion": 3`. The wave packet should make this explicit for every new W2–W5 block.

**Recommended fix.** Add to §10:

> **Block apiVersion 3 mandatory.** All new W2–W5 blocks declare `"apiVersion": 3` (WP 6.9+ enforcement; 7.0 iframed editor compatibility). Sub-L0 packets must surface any apiVersion downgrade as scope-revision.

Trivial guard; codifies what the v1.4.3 starter kit already does.

---

### Finding 7 — Low — Template parts vs synced patterns story for "default page composition template per composite surface" (AC #W4)

**Section cited:** §7 AC #W4 ("Default page composition template ships per composite surface but is editable").

**WP concern.** Two idiomatic WP mechanisms for "default but editable":

1. **Template parts** (`wp/dailyos/theme/parts/*.html`) — referenced from page templates; users edit in Site Editor; edits stored in DB; theme file remains the default. theme.json already declares `header`, `footer`, `sidebar-account-summary` template parts. Suits chrome-shaped composition (one per surface area).
2. **Synced patterns** (`patterns/*.php` or `wp_block` post type) — inserter content; user inserts then customizes; "synced" version propagates changes from theme. Suits per-page composition templates ("default Account Detail layout").

AC #W4 says "default ... but editable" — synced patterns are the closer match because they're per-block-insertion defaults, not per-template-region. The packet doesn't name which.

**Recommended fix.** Add to §7 AC #W4:

> "Default page composition template" ships as a synced pattern (`patterns/*.php`) per composite surface (Account Detail default, Project Detail default, Daily Briefing default, Meeting Briefing default). Theme template parts (`parts/*.html`) are reserved for chrome-shaped regions (header/footer/sidebars) consistent with current `theme.json` `templateParts` declaration. W2/W3 sub-L0 packets ship synced patterns alongside outer-block registrations.

This is the difference between "user customizes one site-wide region" (template part) and "user customizes a default page-shape per Account Detail instance" (synced pattern).

---

## Notes (non-blocking)

- **K-in record (§3) is correct** for WP scope: the chrome lane L0 reference plus the parallel-wave `.synced-from` solutions doc cover the WP-touching prior art. No documented WP substrate is reinvented.
- **§5.1 W1 substrate items** are read shapes (envelope, DTOs, abilities). WP-side consumption is via `executeAbility()` per ADR-0129 §4 (confirmed). No WP-specific gap.
- **§10 invariant "Render-path authorization mandatory before cache lookup"** + "Producer commit on cache miss" + "Chrome runtime-injection scope" are all correctly carried forward; no WP-skill objection.
- **theme.json v3** is correctly declared; `custom.dailyos.tokens` namespace is correctly under `settings.custom` (theme.json v3 schema). Generator path is documented (`wp/dailyos/scripts/generate-theme-json.mjs`). No theme.json structural issue at wave scope.

---

## Convergence path

Findings 1–4 should fold into §10 (invariants) and §13 (open questions / deferred) as wave-level guard rails in cycle 2. Findings 5–7 fold as one-line additions to §6 / §7 / §10. None of the findings expand scope; all narrow WP-specific decisions so the W1–W6 sub-L0 packets inherit consistent answers rather than reinventing them. After cycle 2 edits, expectation is APPROVE.

Path-α offload (per memory `feedback_l2_path_alpha_to_maintenance_project`) does NOT apply at L0 — these are wave-level architectural decisions that govern substrate consumption, not theoretical hardening.

Cycle 1 ends. Awaiting packet revision V1.1.

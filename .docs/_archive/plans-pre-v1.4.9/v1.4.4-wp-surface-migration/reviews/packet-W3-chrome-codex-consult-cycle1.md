# L0 Codex Consult - Packet W3 Chrome Lane (Pulled Forward) - Cycle 1

**Reviewer:** `/codex consult`
**Date:** 2026-05-19
**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` V1.0

## Verdict: CONDITIONAL APPROVE

The parallel lane holds. No direct file conflict with v1.4.3 W4 after the W4 K-in correction. Conditions below.

## Conditions

- **C1 - Amend W4 coupling matrix.** Packet must cite `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:14-20` and stop treating W4 Packet F V1.0 as authoritative. Actual W4 gap is "extend existing nonce substrate + wire verify to `record_claim_feedback`", not new `surface_feedback` table/service. Assert disjoint write sets:
  - W4 expected: `src-tauri/src/services/surface_nonce.rs`, `src-tauri/src/bridges/surface_client.rs`, `src-tauri/src/commands/surface_runtime.rs`, `wp/dailyos/includes/class-dailyos-plugin.php`, `wp/dailyos/includes/transport/class-dailyos-runtime-client.php`, `wp/dailyos/blocks/account-overview/*`, tests.
  - Chrome lane: `wp/dailyos/theme/**` only, especially `functions.php`, `parts/header.html`, `parts/footer.html`, `assets/chrome/**`, `tools/sync-chrome.sh`, `tools/patch-chrome-js.py`.
  - No W4 PR edits under `wp/dailyos/theme/**`; no chrome PR edits under W4 runtime/plugin/blocks paths except tests proving non-regression.

- **C2 - Add chrome-vs-block decision rule.** Runtime injection is allowed only for page-invariant shell chrome: `FolioBar`, `FloatingNavIsland`, `AtmosphereLayer`, `MagazinePageLayout` shell container. Content-shaped surfaces, footer links, breadcrumbs, entity bodies, claim/trust/provenance UI, and per-page authorable regions stay Gutenberg blocks. Future runtime injection outside that set requires L0 amendment.

- **C3 - Add stock-theme negative gate.** Under TwentyTwentyFive or any non-DailyOS theme, assert no `<script>` or stylesheet loads from `wp/dailyos/theme/assets/chrome/`. This pairs with `.docs/plans/v1.4.3-waves.md:76` stock-theme trust/provenance fallback.

- **C4 - Lock Option C alias discipline.** `token-aliases.css` is the right bridge, but make it checkable: derive required raw names from `var(--*)` usage in the 5 chrome modules, assert each alias source exists in `wp/dailyos/theme/theme.json`, and assert aliases load after WordPress global styles / theme presets and before chrome modules. Do not translate the lifted modules to `--wp--preset--*`; that forks canonical Tauri chrome.

- **C5 - Add dual-primitive allowlist.** `Pill.module.css` may coexist with `wp/dailyos/blocks/pill/style.css` for this lift only. Add a static gate or review checklist: any future `assets/chrome/styles/*.module.css` whose basename maps to `wp/dailyos/blocks/<slug>/` requires explicit L0 approval or a reconciliation ticket. File the `Pill` reconciliation in Codebase Maintenance.

- **C6 - Bound `functions.php`.** Theme `functions.php` is acceptable for asset enqueue + theme-only chrome config. It must not grow substrate behavior: no DB writes, no runtime calls, no claim/trust/provenance branching, no direct transport. Prefix functions, hook all work, no closing `?>`. If it grows past asset/chrome config, split to `inc/` or move behavior to plugin.

## Question Answers

### 1. Pulling W3 Chrome Forward vs v1.4.3 W4

**Holds with C1.**

`v1.4.3-waves.md` W4 is a feedback write path: nonce lifecycle, REST endpoint, JS affordance, `record_claim_feedback`, tests (`.docs/plans/v1.4.3-waves.md:203-218`). Its architecture invariant is "feedback writes flow through substrate claim path" (`.docs/plans/v1.4.3-waves.md:77`).

Chrome lane is theme shell assets (`L0-packet-W3...md:11-20`, `179-220`, `233-239`). No schema, no claim write, no plugin registration. The only coupling is visual QA sequencing: W4 screenshots run inside the active theme, so W4 may need to re-run L4 after chrome lands. That is not a file conflict.

Important correction: W4 V1.0 was already called out as reinvention. `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:16-20` says existing `surface_nonce.rs` plus WP transport already cover the nonce substrate; W4 should extend, not create parallel. This strengthens disjointness because revised W4 shrinks away from new migration/service files.

### 2. Option C Token Aliasing

**Right pattern.**

WP literature stance:
- `theme.json` first for styling because it participates in core/plugin/user customization hierarchy: WordPress docs say styling blocks should use `theme.json` when possible, with stylesheets for cases outside that reach: https://developer.wordpress.org/themes/features/block-stylesheets/
- `settings.custom` exists to generate CSS custom properties for theme CSS: https://developer.wordpress.org/themes/global-settings-and-styles/settings/custom/
- User configuration overrides theme defaults in the hierarchy: https://developer.wordpress.org/themes/core-concepts/global-settings-and-styles/

Therefore:
- **A coexist** keeps raw canonical tokens alive but bypasses WP preset/custom override semantics. Wrong long-term.
- **B translate modules** would make WP-native CSS, but forks the canonical Tauri chrome source. Wrong for parity.
- **C alias raw from WP preset/custom** lets canonical modules stay byte-equivalent while WP remains the customization authority. Correct bridge.

Keep `design-tokens.css` sync-only. `token-aliases.css` is the WP adaptation layer.

### 3. `functions.php` in a Block Theme

**Not a smell. Pragmatic boundary.**

Official docs name `functions.php` as optional but standard for block themes, loaded by the active theme, and acceptable for hooks, setup, scripts, and styles: https://developer.wordpress.org/themes/core-concepts/custom-functionality/

Official asset docs route frontend styles/scripts through `functions.php` hooks and enqueue APIs: https://developer.wordpress.org/themes/core-concepts/including-assets/

What would be a smell:
- substrate reads/writes in `functions.php`
- claim/trust/provenance logic in theme
- direct runtime transport from theme
- plugin-like behavior that should survive theme swap

This packet stays on the right side if `functions.php` only enqueues chrome assets, fonts, aliases, and emits shell config derived from WP route context.

`register_block_style` and `wp_enqueue_block_style` are not better for shell chrome. They are for block style variants and per-block CSS. `theme.json` cannot enqueue `chrome.js` or produce per-route runtime config.

### 4. Many-Small-Blocks Trade

**Justified for shell chrome, not a general exception.**

The roadmap says "many blocks, not few" for Tauri component/pattern equivalents and user-customizable page composition (`.docs/plans/wp-foundation-roadmap-reorientation.md:34-37`, `173-175`). ADR-0129 says Gutenberg blocks are the right shape for AI-generated, user-editable intelligence (`.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:97-107`). ADR-0130 says WordPress is a renderer for substrate-authored compositions, not the document model (`.docs/decisions/0130-surface-independent-composition-contract.md:148-188`, `204-214`).

Chrome is different: FolioBar/nav/atmosphere are fixed shell treatment from ADR-0077 (`.docs/decisions/0077-magazine-layout-editorial-redesign.md:27-31`, `45-49`). Users author body content, not the publication frame, in v1.4.x.

So the trade is not optimizing for the wrong thing. It is optimizing for parity and dogfood speed in the shell layer. It becomes a shortcut if reused for footer, breadcrumbs, entity bodies, claim cards, trust/provenance affordances, or any page-specific content.

### 5. `Pill.module.css` Dual Existence

**Works only with C5.**

Selector collision risk is low: chrome `.Pill_*` and plugin `.dailyos-pill*` are disjoint per packet AC #6 (`L0-packet-W3...md:137-140`). Architectural drift risk is real. Without an allowlist, the next sync can quietly add `Button.module.css`, `StatusDot.module.css`, etc. and create two primitive universes.

Deferral to v1.4.5+ is acceptable for this one primitive because it is chrome-internal paint, not a claim/trust primitive. It must be made explicit and mechanically guarded.

## K-in Hits

### `docs/solutions/`

- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:14-20` - W4 V1.0 already blocked for missing existing nonce substrate. Relevant to coupling question.
- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:49-51` - grep by substrate-type primitives, not proposed names. Apply to chrome lane K-in.
- `docs/solutions/tooling-decisions/phpcs-warning-severity-zero-prevents-warning-only-ci-fails-2026-05-19.md:13-15` - WP PHP lint posture; relevant to new `functions.php`.

No direct prior solution for `chrome`, `FolioBar`, `FloatingNavIsland`, `AtmosphereLayer`, `token-aliases.css`, or theme-runtime injection found.

### `.docs/decisions/`

- `.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:54-63` - WP primary surface includes custom block library, magazine theme, CPTs, plugin integration.
- `.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:97-107` - Gutenberg block-shaped composition applies to AI-generated, user-editable intelligence.
- `.docs/decisions/0130-surface-independent-composition-contract.md:148-188` - surfaces are renderers; WP maps substrate blocks to Gutenberg blocks.
- `.docs/decisions/0130-surface-independent-composition-contract.md:200-202` - editing semantics and composition lifecycle stay in existing feedback/signal paths.
- `.docs/decisions/0077-magazine-layout-editorial-redesign.md:27-31` and `45-49` - shared fixed chrome: FolioBar, FloatingNavIsland, atmosphere, watermark.
- `.docs/decisions/0073-editorial-design-language.md:25-40`, `74-83` - typography as architecture, cards only for featured content.
- `.docs/decisions/0076-brand-identity.md:35-83`, `114-120` - material color families and entity/state color boundaries.

## Verdict

**CONDITIONAL APPROVE.** Cycle-2 should fold C1-C6 into the packet. No rewrite required.

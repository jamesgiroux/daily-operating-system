# L0 Packet — v1.4.4 W3 Chrome Lane (Pulled Forward, Parallel to v1.4.3 W4+)

**Current revision: V1.3.1 (post-W4-merge confirmation, 2026-05-19). See §2 Changelog.**

## 1. Header

Date: 2026-05-19
Project: v1.4.4 — WordPress Surface Migration ([Linear](https://linear.app/a8c/project/v144-wordpress-surface-migration-877aaa780177))
Wave: W3 chrome (canonically W3 per `.docs/plans/v1.4.4-waves.md` placeholder + `wp-foundation-roadmap-reorientation.md`) — **pulled forward as a parallel lane** before W0 surface audit / W1 substrate gaps / W2 entity surfaces land.
Issues: TBD on packet approval — one new Linear ticket per lane sub-deliverable per §5 sections (1 packet, ~4 sub-tickets: canonical pre-lift upgrade / tokens+aliases / chrome modules+fonts / functions.php+chrome.js sync).
Surface: WordPress block theme assets (chrome layer) — `wp/dailyos/theme/`
Primary code (new):
- `wp/dailyos/theme/functions.php` (new file — block theme currently has none)
- `wp/dailyos/theme/assets/chrome/styles/token-aliases.css` (new — Option C bridge)
- `wp/dailyos/theme/assets/chrome/styles/{FolioBar,FloatingNavIsland,AtmosphereLayer,MagazinePageLayout,Pill}.module.css` (5 modules, lifted verbatim from canonical Tauri reference, including upgraded `.FolioBar_folioRefreshButton` rule per §5.4.1)
- `wp/dailyos/theme/assets/chrome/styles/design-tokens.css` (synced from canonical; emits the raw `--color-*` / `--folio-*` / `--frosted-glass-*` names the chrome modules consume)
- `wp/dailyos/theme/assets/chrome/fonts/` (13 woff2 + fonts.css)
- `wp/dailyos/theme/assets/chrome/chrome.js` (post-patched output; do not edit directly)
- `wp/dailyos/theme/tools/sync-chrome.sh` (one-way sync from `.docs/design/reference/_shared/`)
- `wp/dailyos/theme/tools/patch-chrome-js.py` (8 WP-vs-Tauri patches — V1.2 removes V1.1's Patch 9b CSS-override after upstreaming to canonical per §5.4.1; only 9a DOM-idempotency remains as a JS-only patch)

Canonical pre-lift edits (V1.2, §5.4.1 — one-time Tauri-side upgrade, lands BEFORE chrome lane sync):
- `.docs/design/reference/_shared/styles/FolioBar.module.css` (add `.FolioBar_folioRefreshButton` class rule with FULL parity CSS matching `src/components/ui/folio-refresh-button.tsx`)
- `.docs/design/reference/_shared/chrome.js` (line 192-203: replace inline `style=` with `class: F('folioRefreshButton')`)

Primary anchors:
- ADR-0129 — composable surfaces; WordPress Studio as primary surface
- ADR-0130 — surface-independent composition contract
- ADR-0077 — magazine layout editorial redesign
- ADR-0073 — editorial design language
- ADR-0076 — brand identity (paper/desk/spice/garden palette + tints)
- `.docs/plans/v1.4.3-waves.md` §"Architecture invariants" line 76 — theme owns no trust/provenance; plugin owns essential; stock TwentyTwentyFive fallback supported
- `.docs/design/patterns/FolioBar.md` + `FloatingNavIsland.md` — canonical pattern specs
Reference implementation: `~/Studio/dailyos/wp-content/themes/dailyos/` (the Ollie-child marketing-site theme from 2026-05-09..10, sha `4ebec482`) — the chrome architecture lift source. **De-Ollified during the lift.**
Diagnostic anchor: v1.4.3 W3 magazine theme (PR #315, DOS-698) — the substrate-rendering bones we extend.

This packet pulls the v1.4.4 W3 "FolioBar + FloatingNavIsland primitives" deliverable forward as a parallel lane that runs alongside v1.4.3 W4 (Feedback Write Infrastructure) — disjoint file regions per §10's W4-coupling matrix, no merge conflict surface.

**Strategic framing — runtime-injected chrome, scoped to page-invariant shell only.** The Studio prior work uses a **runtime DOM-injection model**: template part `parts/header.html` carries a no-op `wp:group` (valid block markup; not literal raw `<div>` — see §5.6), and `chrome.js` (lifted from canonical `_shared/`) builds FolioBar + NavIsland + AtmosphereLayer client-side from `body.dataset.*` populated by PHP. This is **a bounded divergence from the project-wide "many small Gutenberg blocks" strategy** (memory `project_wp_block_custom_vs_core_strategy`) — chrome is treated as **runtime shell**, not a **block-tree concern**, but the divergence applies ONLY to:

- `FolioBar` (top frosted bar)
- `FloatingNavIsland` (right-side dual-pill nav)
- `AtmosphereLayer` (background gradient + watermark)
- `MagazinePageLayout` (shell content container — the *outer* page wrapper, not body content blocks)

**Pill scope clarification (V1.2, cycle-2 challenge fold):** `Pill.module.css` is lifted but Pill is NOT standalone runtime-injected chrome. Pill is consumed as a **child primitive INSIDE** the 4 shell modules (e.g., NavIsland active-state pill chips, FolioBar status indicators, AtmosphereLayer watermark labels). Pill itself remains a Gutenberg block (`wp/dailyos/blocks/pill/`) for body-content authoring. The dual-namespace coexistence (`.Pill_*` chrome / `.dailyos-pill*` block) is governed by AC #28 allowlist gate.

**Everything else stays Gutenberg.** Footer (Tier 5.7), breadcrumb content, entity bodies, claim cards, trust/provenance UI, action affordances, any per-page authorable region: ALL stay as Gutenberg blocks per the surface-independent composition contract (ADR-0130). The chrome-vs-block decision rule is codified in §10 invariant "Chrome runtime-injection scope" — **future runtime injection outside the 4-module shell set requires a wave-plan amendment + new L0 packet documenting the parity-vs-authorability trade.**

Justification for this bounded trade:
1. Tauri-WP chrome parity preserved (identical chrome.js + identical module CSS on both surfaces — canonical Tauri-side source is the design authority per `.docs/design/reference/_shared/`)
2. Lift is days, not weeks
3. End-user authoring of chrome via Site Editor is the v1.4.x non-goal — users author **body content** with W2 primitives; chrome stays surface-shell
4. Block-ification can land as a separate lane later (~700-1100 LOC / 2-4 days per codex challenge §5.4 cost analysis if class contracts preserved) once we have a working substrate-on-chrome reference to translate from

**Intelligence Loop integration check — exempt.** No claim/table/surface added; no provenance/trust impact; no signal change; no runtime context surface consumes new state; no feedback loop change. The lift is theme assets only. CLAUDE.md §"Critical Rules — Intelligence Loop integration check" does not apply.

## 2. Changelog

- **V1.3.1 (2026-05-19 post-L0-close, post-W4-merge confirmation):** v1.4.3 W4 merged at `d781f2b4` ("v1.4.3 W4-F: feedback wire-through (DOS-683) (#326)") shortly after L0 closure. Empirical `git show --stat d781f2b4 -- 'wp/dailyos/theme/'` returns ZERO files — §10 W4-coupling matrix disjoint-write-sets assertion held in practice. §6 expanded with post-W4 substrate inventory confirming chrome lane PRs land cleanly against post-W4 `public/dev`. New worktree at `worktrees/v1.4.4-w3-chrome-lane` (branched from post-W4 `public/dev`) carries this revision forward. No L0 re-review triggered — empirical confirmation, not architectural change.

- **V1.3 (2026-05-19, cycle-3 fold + cycle-4 challenge-only dispatch):** Cycle-3 panel returned 4× APPROVE (architecture, design-lens, codex consult, wp-block-themes) + 1× CONDITIONAL APPROVE (codex challenge, 4 clerical must-fix). All cycle-2 NEW BLOCKERS confirmed RESOLVED across all 5 reviewers. 4 clerical items + 2 advisory notes folded:

  - **AC #30 `--color-alert-red` orphan resolved (challenge cycle-3 #1).** Regex now catches `var(--color-alert-red, #dc2626)` references (Patch 9a area + canonical FolioBar.module.css), but no `--wp--preset--color--alert-red` source exists in theme.json. V1.3 fix: add explicit alias `--color-alert-red: var(--wp--preset--color--spice-chili)` to `token-aliases.css` (chili at #9b3a2a is the existing critical/red semantic token; canonical chrome.js fallback `#dc2626` becomes a non-load-bearing safety net once alias resolves). AC #30 §5.1 updated with alert-red mapping classification.
  - **§5.4.1 reference substrate acknowledgment (challenge cycle-3 #2).** Added explicit framing: `.docs/design/reference/_shared/` is canonical Tauri-side mockup substrate that ordinarily mirrors shipped source; §5.4.1's pre-W3 edit is a **deliberate design-system improvement** (introduces class-based refresh-button styling that didn't previously exist anywhere), not a one-off exception to the mirror rule. Treated as a normal canonical PR with codex review + design-reviewer + design-system author awareness, not a sync-rule violation.
  - **AC #31 split into #31a + #31b (challenge cycle-3 #3).** V1.2 AC #31 was hybrid (static check + L4 hover/focus-visible proof). V1.3 splits: **AC #31a** static-grep verifiable — class rule present in canonical FolioBar.module.css, no `style=` attribute on refresh-button in canonical chrome.js, class reference present in `class: F('folioRefreshButton')` call. **AC #31b** L4-hands-on verifiable — hover state actually swaps color/border on reference HTML pages, focus-visible outline renders on keyboard tab.
  - **§10 canonical source-of-truth escape hatch (challenge cycle-3 #4).** V1.2 invariant was absolute. V1.3 adds escape valve: WP-only CSS deviations are permitted IF they (a) require L0 amendment + (b) land as separate non-synced overlay file at `wp/dailyos/theme/assets/chrome/styles/wp-overlay-*.css` enqueued after the synced modules + (c) document the WP-specific reason. Direct edits to synced module CSS files remain forbidden. This preserves canonical mirror discipline while leaving a defined path for legitimate WP-only needs.
  - **V1.2 misstatement corrected (architecture cycle-3 F1 + challenge cycle-3 note 2).** V1.2 §1 + §5.4.1 incorrectly claimed React `src/components/ui/folio-refresh-button.tsx` "already uses class-shaped React state" — verified file actually uses inline `style={{}}` + `onMouseEnter`/`onMouseLeave` callbacks, same pattern as canonical chrome.js pre-§5.4.1-upgrade. V1.3 rephrases to: React production component currently uses inline-style + JS-driven hover callbacks (same shape as pre-upgrade canonical chrome.js); §5.4.1 upgrades canonical mockup substrate to class-based, surfacing the misalignment between mockup and production. React-component class-based refactor is a separate Tauri-side improvement deferred to Tauri-freeze-lift release (§8 new deferral row).
  - **POSIX-portable regex spelling (challenge cycle-3 note 1).** AC #30 regex updated from `var\(--[A-Za-z0-9_-]+(?:,[^)]+)?\)` to `var\(--[A-Za-z0-9_-]+(,[^)]+)?\)` — drops the `?:` non-capturing-group syntax for BSD-grep portability. Functionally identical; supports macOS grep without -E enhancement requirements (still uses -E for `+`).

  No new BLOCKERs in cycle-3. Convergence trajectory clean across 3 cycles: 4 CONDITIONAL (cycle-1) → 3 CONDITIONAL + 2 APPROVE (cycle-2) → 1 CONDITIONAL + 4 APPROVE (cycle-3). Cycle-4 dispatches codex challenge only (other 4 reviewers' concerns unchanged in V1.3); if challenge APPROVE, L0 closes unanimous.

- **V1.2 (2026-05-19, cycle-2 fold + cycle-3 dispatch):** Cycle-2 panel returned 2× APPROVE (architecture, design-lens), 3× CONDITIONAL APPROVE (codex consult, codex challenge, wp-block-themes-grounded). 3 NEW BLOCKERS surfaced by codex challenge + 14 clerical items folded:

  **NEW BLOCKERS resolved:**
  - **B1 + B2 (challenge) — Patch 9b CSS parity + source-of-truth (V1.2 §5.4.1).** Cycle-1 fold's Patch 9b proposal as a WP-overlay step had two flaws: (a) the proposed CSS rule was incomplete (dropped font/size/weight/letter-spacing/text-transform/color/background/border/radius/padding from canonical chrome.js:202 + src/components/ui/folio-refresh-button.tsx:38-52), and (b) module CSS overlays violated the sync model's "never edit synced files directly" + `patch-chrome-js.py` only patches chrome.js, not module CSS. **V1.2 decision: upstream to canonical instead of WP-overlay.** The canonical-side edits (FolioBar.module.css gains `.FolioBar_folioRefreshButton` rule with FULL parity CSS; chrome.js line 202 replaces inline `style=` with class reference) land FIRST in `.docs/design/reference/_shared/` as a single pre-lift Linear ticket; chrome lane then consumes the upgraded canonical via standard `sync-chrome.sh`. Benefits: single source-of-truth preserved, Tauri-side mockup substrate also gains class-based hover/focus-visible (currently inline-style-only; React production component at `src/components/ui/folio-refresh-button.tsx` already uses class-shaped React state, so canonical alignment improves Tauri parity), Patch 9 reduces to Patch 9a (DOM idempotency only).

  - **B3 (challenge + consult) — AC #30 regex insufficient.** V1.1's `var\(--[a-z-]+\)` misses digit-suffixed tokens (`--color-spice-turmeric-10`, `--color-desk-charcoal-4`, etc.) and tokens with fallback values (`var(--color-alert-red, #dc2626)`). V1.2 AC #30 fixes regex to `var\(--[A-Za-z0-9_-]+(?:,[^)]+)?\)` + classifies local computed props (`--pill-*`, `--local-pill-top`, `--page-margin-top`, etc.) separately from cross-module aliases that need theme.json source verification.

  **Clerical folds (14 items):**
  - **AC #28 promoted to real body (consult C5):** static CI gate + failure rule + Pill reconciliation Linear ticket reference (filed concurrently with packet approval; ID inserted at L1 kickoff).
  - **§8 deferral ticket IDs (consult):** Linear tickets filed for blockification cost, scroll-spy, Pill reconciliation; IDs inserted at L1 kickoff. §8 marked "TBD-at-L1-kickoff" until filed.
  - **§13 WP-CLI reset path corrected (challenge I1 + wp-skill A2):** `wp template-part delete` is NOT a registered WP-CLI command (verified locally — `wp help template-part` returns "Error: 'template-part' is not a registered wp command"). V1.2 reset path uses validated `wp post list --post_type=wp_template_part --format=ids | xargs -I{} wp post delete {} --force`. SQL fallback example REMOVED (V1.1 referenced `post_author` which is wrong — template parts scoped via `wp_theme` taxonomy on `wp_term_relationships`; correct SQL is complex enough that WP-CLI path is the only documented option).
  - **§10 Pill scope clarified (challenge stress-test).** §1 + §10 explicit: Pill is child primitive INSIDE the 4 shell modules, not standalone runtime-injected chrome.
  - **§3 K-in inventory rescan (challenge K-in note):** `docs/solutions/` has 14 .md files at scan time (V1.1 said 4 — stale snapshot). V1.2 §3 lists the full set; no new chrome-relevant hits beyond the 2 cross-references already cited.
  - **style.css `Requires at least: 6.3` (wp-skill M1):** Block theme declares minimum WP 6.3 (when `enqueue_block_assets` entered the editor iframe per WP block-editor handbook). Pre-existing PR #315 style.css already says `Requires at least: 6.5`; V1.2 confirms this is acceptable (>= 6.3) and explicitly notes the AC #26 editor-iframe alias proof depends on the 6.5 floor.
  - **§5.5 prohibition list expanded (wp-skill M2):** Theme functions.php must NOT call `register_sidebar()`, `register_nav_menus()`, or `add_theme_support('custom-header'/'custom-background')` — classic-theme paradigms that don't belong in a block theme.
  - **§5.5 `wp_body_open` advisory note (wp-skill A5):** `wp_body_open` action fires server-side once per render and CAN'T substitute for chrome.js client-side idempotency guard (which must catch client-side re-invocation from preview reloads / partial refresh). Documented in §5.4 as non-applicable.
  - **§5.1 cascade semantics note (wp-skill A4):** `:root` last-declaration-wins is the cascade rule for `token-aliases.css`. Aliases override `--wp--preset--*` defaults only if loaded AFTER the WP-emitted preset stylesheet. Load order enforced by `wp_enqueue_style` dependency graph (AC #15 + AC #30).
  - **AC #24 rationale sharpened (wp-skill A3):** `! is_customize_preview()` is NOT a generic safety pattern per WP docs warning. Specific rationale: Customizer renders its own preview chrome that conflicts visually with chrome.js DOM injection; the guard prevents stacked-chrome rendering, not security. AC #24 commentary updated.
  - **§5.1 digit-segment audit (wp-skill A1):** Numeric segments in theme.json `settings.custom.*` keys hyphenate digit-by-digit (`abc123` → `abc-1-2-3`). Chrome consumes `--space-2xl`, `--space-3xl`, etc. via theme.json `settings.spacing.spacingSizes`; theme.json palette tokens like `spice-turmeric-10` are alphanumeric slugs (not multi-digit numeric segments) and emit cleanly. Verified by reading `wp/dailyos/theme/theme.json` palette entries.
  - **§8 `dailyos_project` tint deferral (design-lens advisory 1):** Explicit one-line entry — W2 L0 must resolve project tint via ADR-0077 amendment OR new ADR before W2 CPT registration. V1.2 packet stubs to turmeric.
  - **AC #17 PHPUnit fixture detail (design-lens advisory 2):** PHPUnit branch test for each of 4 stub CPTs requires `register_post_type()` fixture in test setUp() — without it `is_singular()` and `is_post_type_archive()` return false for the not-yet-registered CPTs.
  - **`docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` solution still controls W4 (consult re-affirmation).** W4 cycle-2 has not yet folded the K-in correction; W4 is independently blocked. W3 chrome lane is unaffected as long as PRs stay in `wp/dailyos/theme/**` per the §10 W4-coupling matrix.

  Cycle-2 architecture + design-lens reviewers returned unconditional APPROVE; their concerns were resolved cleanly in V1.1 and V1.2 doesn't regress them. Cycle-3 dispatch verifies the 3 NEW BLOCKER + 14 clerical folds against V1.2.

- **V1.1 (2026-05-19, cycle-1 fold + cycle-2 dispatch):** [retained from V1.1 — see git history; 20 cycle-1 fold items consolidated into the structural shape V1.2 inherits]

- **V1.0 (2026-05-19, initial L0 draft):** First L0 cycle. 4× CONDITIONAL APPROVE returned; folded into V1.1.

## 3. K-in record (substrate-grep audit, V1.2 rescan)

Per CLAUDE.md "Knowledge store discovery" + engineering-ladder.md L0 K-in obligation. Both directories greped 2026-05-19 (V1.0 initial + V1.1 reviewer-confirmed + V1.2 full rescan).

### `docs/solutions/` — 14 .md files at scan time; 2 cross-references relevant, no blockers.

Full inventory (V1.2 rescan):

```
docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md
docs/solutions/architecture-patterns/emit-or-log-wrapper-silent-error-swallow-class-2026-05-18.md
docs/solutions/conventions/migration-filename-version-offset-2026-05-18.md
docs/solutions/README.md
docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md
docs/solutions/test-failures/parallel-test-singleton-state-flake-2026-05-18.md
docs/solutions/tooling-decisions/codex-worktree-isolation-incompatible-with-rescue-forwarder-2026-05-18.md
docs/solutions/tooling-decisions/gh-pr-merge-delete-branch-multi-worktree-incompatibility-2026-05-19.md
docs/solutions/tooling-decisions/phpcs-warning-severity-zero-prevents-warning-only-ci-fails-2026-05-19.md
docs/solutions/tooling-decisions/pre-push-hook-duration-vs-ssh-idle-timeout-2026-05-19.md
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md
docs/solutions/workflow-issues/node-modules-tracked-symlink-enotdir-pnpm-install-2026-05-19.md
docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md
docs/solutions/workflow-issues/worktree-setup-needs-pnpm-install-2026-05-19.md
```

**Greps run:** `chrome`, `foliobar`, `navisland`, `atmosphere`, `theme.json`, `token.alias`, `design.tokens`, `baseline.shim`, `ollie`, `wp.theme`, `wp.enqueue`, `magazine.theme`, `tokens`, `wordpress`, `enqueue`, `asset`, `runtime.injection`, `dom.mutation`.

**Relevant cross-references (cited in body of packet):**
- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` — lines 14-20 record the W4 V1.0 reinvention finding (existing `surface_nonce.rs` substrate covers nonce; W4 should extend not parallel). **Strengthens** the chrome lane's disjoint-from-W4 claim. See §10 W4-coupling matrix.
- `docs/solutions/tooling-decisions/phpcs-warning-severity-zero-prevents-warning-only-ci-fails-2026-05-19.md` — lines 13-15, 49-52: WP PHPCS CI posture; relevant to `wp/dailyos/theme/functions.php` lint compliance. Pre-applied per §5.5.
- `docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md` — sets precedent for wave-plan amendment requirement when scope substantially changes. **This packet's "pull v1.4.4 W3 forward" amendment fits this pattern** — recording precedent here, not a blocker.

**Verdict: net new substrate territory for WP chrome. No documented prior solution reinvented.**

### `.docs/decisions/` — 5 ADRs consumed verbatim, none overridden.

[Unchanged from V1.1 — see git history for the consumption matrix.]

ADR-0073, ADR-0076, ADR-0077, ADR-0129, ADR-0130 consumed; reviewers cited specific line numbers in cycle-1 + cycle-2 verdicts.

**Verdict: K-in complete across cycles. 5 ADRs consumed; no conflicts; no reinvention. Triangulated across 5 cycle-2 reviewers + 2 cycle-1 reviewers.**

## 4. Scope summary

7 lift tiers + 1 canonical pre-lift upgrade (V1.2 §5.4.1). Tiers 1–5 + §5.4.1 are this packet's L1 implementation scope. Tier 6 (menus) defers to v1.4.4 W2 entity-surfaces. Tier 7 (marketing patterns) explicitly out of scope.

| Tier | Subject | Scope | Files | LOC budget |
|---|---|---|---|---|
| **§5.4.1** | **Canonical pre-lift upgrade** (V1.2 NEW) | Add `.FolioBar_folioRefreshButton` class rule with full parity CSS to canonical `_shared/styles/FolioBar.module.css`; replace inline `style=` at canonical `_shared/chrome.js:202` with class reference. Single PR lands BEFORE chrome lane sync. | 2 canonical files (Tauri-side mockup substrate) | ~30 LOC (CSS rule) + ~5 LOC (chrome.js patch) |
| **1** | Tokens + alias layer | `design-tokens.css` synced from canonical + new `token-aliases.css` declaring raw names as aliases of `--wp--preset--*` / `--wp--custom--*` vars, with discipline gate (§5.1 AC #30) | 2 files | ~360 |
| **2** | Chrome CSS modules | 5 modules lifted verbatim from canonical `_shared/styles/` (FolioBar.module.css carries V1.2 upgraded `.FolioBar_folioRefreshButton` rule from §5.4.1) | 5 files | ~960 |
| **3** | Fonts | 13 woff2 + fonts.css + LICENSES.md | 14 files | ~14KB binary + ~100 LOC |
| **4** | chrome.js + sync tooling | Sync script + WP-overrides patch script (Patch 9a only after V1.2 §5.4.1 absorbs 9b into canonical) + first synced+patched chrome.js | 4 files (incl. `.synced-from`) | ~880 (orchestration), ~535 (synced output) |
| **5** | Theme functions.php (orchestration glue) | New file: enqueue chain + per-surface chrome_config() driven by `dailyos_account` CPT with stubs for `dailyos_briefing`/`person`/`project`/`meeting`; Customizer guard; bounded-behavior list (§5.5 C6 + V1.2 wp-skill M2 expansion) | 1 file | ~250 |
| **6** | Menus | Site nav driven from WP Nav Menu or post-type archive links | — | **DEFERRED to v1.4.4 W2** |
| **7** | Marketing patterns | `home.php`, `tour.php`, `philosophy.php`, `setup.php` from Studio | — | **OUT OF SCOPE** |

## 5. Detailed sections

### §5.1 — Tier 1: Tokens + alias layer

**Files owned (new):**
- `wp/dailyos/theme/assets/chrome/styles/design-tokens.css` (synced; ~299 LOC)
- `wp/dailyos/theme/assets/chrome/styles/token-aliases.css` (new; ~80 LOC)

**Files shared (no edits):** `wp/dailyos/theme/theme.json` (already auto-generated from `src/styles/design-tokens.css` via `wp/dailyos/scripts/generate-theme-json.mjs`; remains canonical for WP preset palette + custom block tokens), `wp/dailyos/assets/dailyos-baseline-tokens.css` (plugin-owned 91-line shim; unchanged — its job is `--wp--preset--color--*` fallback for stock-theme contexts and is not chrome's concern).

**`design-tokens.css` lift:** verbatim sync from `.docs/design/reference/_shared/styles/design-tokens.css` (sha `4ebec482` per Studio's `.synced-from`). Defines `--color-*`, `--font-*`, `--space-*`, `--folio-*`, `--page-*`, `--nav-island-right`, `--radius-editorial-*`, `--shadow-*`, `--transition-*`, `--backdrop-blur` + `--frosted-glass-*`, `--z-*`, `atmosphere-breathe` keyframe.

**`token-aliases.css` purpose:** Option C bridge. Chrome modules consume raw `--color-spice-turmeric` etc.; theme.json emits `--wp--preset--color--spice-turmeric` etc.; aliases bridge the two.

```css
:root {
  --color-spice-turmeric: var(--wp--preset--color--spice-turmeric);
  --color-spice-turmeric-10: var(--wp--preset--color--spice-turmeric-10);
  --folio-height: var(--wp--custom--dailyos--tokens--folio-height);
  --frosted-glass-background: var(--wp--custom--dailyos--tokens--frosted-glass-background);
  /* ...etc for every chrome-module-consumed token... */
}
```

**Cascade semantics (V1.2, wp-skill A4 fold):** `:root` last-declaration-wins. `token-aliases.css` must enqueue AFTER the WP-emitted preset/custom stylesheet for aliases to win the cascade. Load order enforced by `wp_enqueue_style` dependency graph (AC #15 + AC #30).

**Digit-segment audit (V1.2, wp-skill A1 fold):** Theme.json `settings.custom.*` keys with multi-digit numeric segments hyphenate digit-by-digit (`abc123` → `abc-1-2-3`). Audit confirmed `wp/dailyos/theme/theme.json` palette tokens like `spice-turmeric-10` use alphanumeric slugs (not multi-digit numeric segments), so `--wp--preset--color--spice-turmeric-10` emits cleanly. Spacing presets `2xl`/`3xl`/`4xl`/`5xl` also clean (single character + letters). No digit-segment hyphenation issues in current scope; AC #30 grep would surface them if introduced later.

**Why theme-owned, not plugin-owned:** [unchanged from V1.1]

**Enqueue order from §5.5 functions.php:** baseline tokens (plugin, priority 9) → design-tokens.css (theme raw names) → token-aliases.css (bridge) → chrome modules → theme glue.

**Acceptance criteria:**
- **AC #1** — `design-tokens.css` byte-equivalent to canonical source at sha pin recorded in `.synced-from`
- **AC #2** — `token-aliases.css` declares aliases for every CSS custom property consumed by FolioBar/FloatingNavIsland/AtmosphereLayer/MagazinePageLayout/Pill modules.
- **AC #3** — No alias points to a `--wp--preset--*` / `--wp--custom--*` source that doesn't exist in `wp/dailyos/theme/theme.json`'s palette or `settings.custom.dailyos.tokens` block.
- **AC #30 (V1.3 alert-red mapping fix + POSIX regex)** — Alias discipline gate. CI test:
  1. **Cross-module alias extraction:** `grep -hoE 'var\(--[A-Za-z0-9_-]+(,[^)]+)?\)' wp/dailyos/theme/assets/chrome/styles/{FolioBar,FloatingNavIsland,AtmosphereLayer,MagazinePageLayout,Pill}.module.css | sort -u` (POSIX-portable; works on macOS BSD grep without `-E` enhancement requirements beyond `+`).
  2. **Classify:** Strip fallback values; group by prefix.
     - **Module-local** (declared and used WITHIN a single module): `--pill-*`, `--local-pill-top`, etc. → classified as module-local, no alias required.
     - **Cross-module / "alias-required"** (consumed by chrome modules but defined in canonical `design-tokens.css` + aliased to WP-preset/custom): `--color-*`, `--folio-*`, `--font-*`, `--space-*`, `--frosted-glass-*`, `--radius-editorial-*`, `--shadow-*`, `--transition-*`, `--z-*`, `--nav-island-right`, `--page-*`, `--backdrop-blur`. Every name in this group MUST appear as LHS of a declaration in `token-aliases.css`.
     - **Special: `--color-alert-red`** (V1.3 explicit handling per challenge cycle-3 #1). Appears in canonical chrome.js Patch 9a area + FolioBar.module.css with fallback `#dc2626`. **NOT defined in `design-tokens.css`.** Mapping: `token-aliases.css` declares `--color-alert-red: var(--wp--preset--color--spice-chili)` — aliases to existing chili token (`design-tokens.css:39` defines `--color-spice-chili: #9b3a2a`; theme.json palette has corresponding slug). Canonical fallback `#dc2626` remains as a non-load-bearing safety net for stock-theme contexts where chrome doesn't render anyway. Future option: define `--color-alert-red` as net-new token in `src/styles/design-tokens.css` if alert-red semantic distinction from chili emerges; for now, alias is sufficient.
  3. **Assert:** Every "alias-required" name appears as LHS of a declaration in `token-aliases.css`. RHS source of each alias (`--wp--preset--*` / `--wp--custom--*`) exists in `wp/dailyos/theme/theme.json` (palette slug or custom-tokens key).
  4. **Enqueue order:** `wp/dailyos/theme/functions.php` declares `dailyos-aliases` as depending on `dailyos-tokens`, and each chrome-module stylesheet (`dailyos-magazine`, `dailyos-atmosphere`, `dailyos-folio`, `dailyos-nav`, `dailyos-pill`) declares dependency on `dailyos-aliases`.

### §5.2 — Tier 2: Chrome CSS modules

**Files owned (new, verbatim lift from canonical):**
- `wp/dailyos/theme/assets/chrome/styles/AtmosphereLayer.module.css` (~152 LOC, 1 keyframe)
- `wp/dailyos/theme/assets/chrome/styles/MagazinePageLayout.module.css` (~59 LOC)
- `wp/dailyos/theme/assets/chrome/styles/FolioBar.module.css` (~290 LOC after §5.4.1 canonical upgrade)
- `wp/dailyos/theme/assets/chrome/styles/FloatingNavIsland.module.css` (~356 LOC)
- `wp/dailyos/theme/assets/chrome/styles/Pill.module.css` (~104 LOC)

**FolioBar.module.css note (V1.2):** Carries the new `.FolioBar_folioRefreshButton` class rule introduced by §5.4.1 canonical pre-lift upgrade. Verbatim lift after canonical upgrade lands.

**Acceptance criteria:**
- **AC #4** — All 5 modules byte-equivalent to canonical at sha pin (where canonical sha is the POST-§5.4.1-upgrade sha)
- **AC #5** — Each module's `var(--*)` references resolve via Tier 1 aliases
- **AC #6** — Pill chrome version (`.Pill_*`) does not collide with plugin block (`.dailyos-pill*`) — dual-namespace coexistence governed by AC #28 allowlist.

### §5.3 — Tier 3: Fonts

[Unchanged from V1.1]

### §5.4 — Tier 4: chrome.js + sync tooling (V1.2: Patch 9 reduced to 9a only)

**Files owned (new):**
- `wp/dailyos/theme/tools/sync-chrome.sh` (~83 LOC; one-way mirror from `.docs/design/reference/_shared/` into `wp/dailyos/theme/assets/chrome/`)
- `wp/dailyos/theme/tools/patch-chrome-js.py` (~240 LOC; 8 patches from Studio + Patch 9a DOM idempotency from V1.1 fold)
- `wp/dailyos/theme/assets/chrome/chrome.js` (~540 LOC post-patch; lifted from canonical 533 LOC + 9 JS patches applied)
- `wp/dailyos/theme/assets/chrome/.synced-from` (auto-generated; records source path + canonical-sha + UTC timestamp)

**The 9 patches in patch-chrome-js.py (V1.2 — Patch 9b moved upstream to canonical per §5.4.1):**

| # | Patch | Status |
|---|---|---|
| 1 | `brandMark()` override via `window.dailyosBrandMark` | Inherited from Studio |
| 2 | `inject()` merges `window.dailyosChrome` into `body.dataset` | Inherited from Studio |
| 3 | Strip dev-only `buildReferenceControls()` + `wireOnboardingNav()` | Inherited from Studio |
| 4 | `buildNav()` honors `data-nav-items-json` for custom website nav | Inherited from Studio |
| 5 | `buildNav()` per-item href; flat render when items lack `group` | Inherited from Studio |
| 6 | `buildNav()` home button href/label/id from body data attrs | Inherited from Studio |
| 7 | Group render falls back to flat when items lack `group` | Inherited from Studio |
| 8 | `buildFolio()` home link href from `body.dataset.folioHomeHref` | Inherited from Studio |
| **9a** | **DOM idempotency guard** | V1.1 fold (retained) |
| ~~9b~~ | ~~Refresh-button class replacement~~ | **V1.2: MOVED UPSTREAM to canonical per §5.4.1** |

**Patch 9a (V1.1, CH2 fold — retained):** In canonical `chrome.js` `inject()` (line ~465-488), insert at top:
```js
if (body.querySelector('.FolioBar_folio, .FloatingNavIsland_navIslandContainer, .AtmosphereLayer_atmosphere')) return;
```
Prevents duplicate-chrome injection on preview reloads, partial refresh, or future iframe asset routing.

**`wp_body_open` non-substitute note (V1.2, wp-skill A5):** `wp_body_open` action fires server-side once per render and CAN'T substitute for chrome.js client-side idempotency guard (the guard must catch client-side re-invocation from preview reloads / partial refresh / `inject()` called twice from any source). `wp_body_open` could OPTIONALLY emit a server-side mount marker (`<span id="dailyos-chrome-mount"></span>`) that the JS guard checks, but this adds complexity without resolving the core client-side guard requirement. Out of scope.

**Sync model:**
- Source of truth: `.docs/design/reference/_shared/chrome.js` + `_shared/styles/*.module.css` + `_shared/fonts/`
- Sync direction: one-way (canonical → theme). Never edit synced files directly.
- Re-sync trigger: when canonical changes (manual; sha drift between `.synced-from` and `git rev-parse` indicates resync needed)
- Patch application: `patch-chrome-js.py` runs as `cat chrome.js | python3 patch-chrome-js.py > chrome.js.patched`. Failures (anchor not found) exit code 2 with named-patch diagnostic.

**Scroll-spy deferral (V1.1, D-C fold; retained V1.2; ticket ID DOS-724 inserted V1.3):** Canonical `chrome.js` does NOT implement `IntersectionObserver` scroll-spy for FloatingNavIsland chapter local-pill. Deferred per **DOS-724** (Codebase Maintenance, Medium) — lands either in v1.4.5+ lane or as opportunistic canonical patch when first consumer surface (v1.4.4 W2 entity detail) needs it.

**Acceptance criteria:**
- **AC #11** — `sync-chrome.sh` lifted verbatim from Studio prior work; only path mutations.
- **AC #12** — `patch-chrome-js.py` lifted from Studio prior work; 8 patches preserved + Patch 9a (V1.1 retained, V1.2 confirms only 9a remains; 9b moved upstream).
- **AC #13** — `chrome.js` post-patch matches expected output. Re-run `sync-chrome.sh` after lift → zero diff against committed `chrome.js`.
- **AC #14** — `.synced-from` records source path, canonical-sha, UTC timestamp.
- **AC #27** — `inject()` is idempotent. Invoke twice via JS console; only one chrome set present.

### §5.4.1 — Canonical pre-lift upgrade (V1.2 NEW, B1+B2 fold)

**Purpose:** Upstream the refresh-button class-based styling to canonical Tauri-side mockup substrate so the chrome lane lift remains a clean verbatim sync. This is a one-time pre-W3 canonical edit, landed as a separate Linear ticket BEFORE the chrome lane PRs.

**Files modified (Tauri-side canonical):**
1. `.docs/design/reference/_shared/styles/FolioBar.module.css` — add `.FolioBar_folioRefreshButton` class rule (~30 LOC):

```css
.FolioBar_folioRefreshButton {
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--color-text-tertiary);
  background: none;
  border: 1px solid var(--color-rule-heavy);
  border-radius: var(--radius-editorial-sm);
  padding: 2px 10px;
  cursor: pointer;
  transition: color var(--transition-normal), border-color var(--transition-normal), opacity var(--transition-normal);
  -webkit-app-region: no-drag;
}

.FolioBar_folioRefreshButton:hover {
  color: var(--color-text-secondary);
  border-color: var(--color-text-tertiary);
}

.FolioBar_folioRefreshButton:focus-visible {
  outline: 2px solid var(--color-spice-turmeric);
  outline-offset: 2px;
}

.FolioBar_folioRefreshButton:disabled,
.FolioBar_folioRefreshButton[aria-busy="true"] {
  cursor: default;
  opacity: 0.6;
}
```

Parity cite: matches `src/components/ui/folio-refresh-button.tsx:38-52` inline styles + `onMouseEnter`/`onMouseLeave` color swaps (lines 53-62) + canonical chrome.js:202 inline `style=` (now obsolete). The `:focus-visible` rule is a NET IMPROVEMENT — current React component has no focus-visible style (accessibility gap silently inherited from inline-style approach); class-based version surfaces it.

2. `.docs/design/reference/_shared/chrome.js` line 192-203 — replace the inline `style="..."` attribute with `class: F('folioRefreshButton')`:

```js
// Before (canonical inline style — V1.0-V1.1 era):
if (a === 'refresh') {
  actWrap.append(el('button', {
    type: 'button',
    title: refreshTitle,
    style: "font-family:var(--font-mono); ...",  // ~12 properties inline
  }, 'Refresh'));
}

// After (V1.2 canonical upgrade):
if (a === 'refresh') {
  actWrap.append(el('button', {
    type: 'button',
    title: refreshTitle,
    class: F('folioRefreshButton'),
  }, 'Refresh'));
}
```

**Why upstream-to-canonical instead of WP-overlay (B2 decision):**
- (a) Single source-of-truth preserved (`.docs/design/reference/_shared/` remains canonical for chrome assets; chrome lane consumes via verbatim sync)
- (b) Tauri-side mockup substrate also gains class-based hover/focus-visible (currently inline-style-only)
- (c) Sync model unchanged (one-way canonical → theme; no module-CSS exceptions; no patch-modules.py tooling needed)
- (d) AC #13 verification stays simple (re-sync idempotent)

**Reference substrate acknowledgment (V1.3, challenge cycle-3 #2 fold):** `.docs/design/reference/_shared/` is canonical Tauri-side mockup substrate that ordinarily mirrors shipped source. §5.4.1's pre-W3 edit is a **deliberate design-system improvement** (introduces class-based refresh-button styling that didn't previously exist anywhere on either surface), NOT a one-off exception to the mirror rule. Treated as a normal canonical PR with codex review + design-reviewer + design-system author awareness — not a sync-rule violation.

**Tauri-side regression risk:** Low. Canonical chrome.js is the mockup substrate (powers `.docs/design/reference/surfaces/*.html` reference pages). The React production component (`src/components/ui/folio-refresh-button.tsx`) uses inline-style + onMouseEnter/onMouseLeave callbacks — SAME shape as canonical chrome.js pre-§5.4.1-upgrade. Moving canonical chrome.js to class-based has no impact on the production React component (different file, independent rendering path). Mockup-substrate HTML pages will get hover/focus-visible behavior they didn't have before — net improvement. **The React component itself remains inline-style for now**; class-based refactor of `folio-refresh-button.tsx` is a separate Tauri-side improvement deferred to Tauri-freeze-lift release per §8.

**Pre-L1 dependency:** Chrome lane L1 implementation cannot start until §5.4.1 canonical upgrade lands (chrome lane sync would lift pre-upgrade canonical → missing class rule). Ticket sequencing:
- **Pre-W3 ticket** (filed at L1 kickoff): Apply §5.4.1 canonical upgrade. Diff: 2 files, ~35 LOC. L2 review: codex review + design-reviewer.
- **W3-1 (tokens + aliases):** chrome lane Tier 1; starts after pre-W3 ticket merges.
- **W3-2 through W3-4:** chrome lane Tiers 2-5.

**Acceptance criteria:**
- **AC #31a (V1.3 split, static-grep verifiable)** — Canonical pre-lift upgrade lands as separate PR before chrome lane sync, with static-checkable outputs:
  - `.docs/design/reference/_shared/styles/FolioBar.module.css` contains `.FolioBar_folioRefreshButton` rule with all 12 parity properties (font-family, font-size, font-weight, letter-spacing, text-transform, color, background, border, border-radius, padding, cursor, transition) + hover state + focus-visible state + disabled state.
  - `.docs/design/reference/_shared/chrome.js` line ~202 uses `class: F('folioRefreshButton')` attribute — no `style=` attribute on refresh-button button element. Verification: `grep -n 'class.*folioRefreshButton\|style.*font-family.*var(--font-mono)' .docs/design/reference/_shared/chrome.js` confirms class present, style absent.
- **AC #31b (V1.3 split, L4-hands-on verifiable)** — Mockup-substrate HTML pages at `.docs/design/reference/surfaces/*.html` render refresh button with class-driven behavior:
  - Hover over refresh button → color swaps from `--color-text-tertiary` to `--color-text-secondary`; border-color swaps from `--color-rule-heavy` to `--color-text-tertiary`.
  - Keyboard-tab to refresh button → focus-visible outline renders (2px turmeric solid, 2px offset).
  - Disabled state (if testable via reference page) → cursor:default, opacity 0.6.

### §5.5 — Tier 5: Theme functions.php (orchestration glue)

[Mostly unchanged from V1.1; V1.2 deltas inline:]

**`functions.php` bounded behavior (V1.2 expansion, wp-skill M2 fold):** Theme functions.php MUST NOT:
- Write to the database (no `$wpdb->insert/update/delete`, no rusqlite, no direct SQL)
- Invoke abilities-runtime, MCP, or any DailyOS Rust runtime path
- Carry claim / trust band / provenance / signal / sensitivity branching logic
- Make HTTP calls to the DailyOS runtime (`class-dailyos-runtime-client.php` is plugin's domain, not theme's)
- Register block types, REST routes, CPTs, or admin pages (those belong to the plugin)
- **V1.2 expansion:** Call `register_sidebar()`, `register_nav_menus()`, or `add_theme_support('custom-header')` / `add_theme_support('custom-background')` — these are classic-theme paradigms that don't apply to block themes. The block-theme equivalents are template parts + `theme.json` `templateParts` registration (already in place per `wp/dailyos/theme/theme.json:730-745`).
- Use top-level execution; all work hooked to actions/filters
- Include a closing `?>` tag

**AC #24 rationale sharpened (V1.2, wp-skill A3 fold):** `! is_customize_preview()` is NOT a generic safety pattern per WP docs warning at https://developer.wordpress.org/reference/functions/is_customize_preview/. The specific rationale for this guard is **visual conflict**: Customizer renders its own preview chrome (toolbar, breadcrumbs, controls) inside the preview iframe. Chrome.js DOM-injecting FolioBar + NavIsland on top of Customizer's chrome produces stacked-chrome rendering that breaks dogfood preview. The guard prevents visual stacking, not security.

**AC #17 PHPUnit fixture detail (V1.2, design-lens advisory 2):** Branch test for each of 4 stub CPTs requires `register_post_type()` fixture in test setUp() — without it `is_singular()` and `is_post_type_archive()` return false for unregistered CPTs. Fixture: `register_post_type('dailyos_briefing', ['public' => true, 'rewrite' => ['slug' => 'briefings']])`, etc.

[All other §5.5 content unchanged from V1.1 — hook scoping table, enqueue chain, chrome_config + tint stub table, local-pill hidden note]

### §5.6 — `parts/header.html` template-part stays as no-op valid Group block

[Unchanged from V1.1]

### §5.7 — `parts/footer.html` rewrites to substrate-mode

[Unchanged from V1.1]

## 6. Substrate consumed (no rewrites except §5.4.1 canonical upgrade)

[V1.1 list unchanged, plus V1.2 addition, plus V1.3.1 W4-merge confirmation:]

- `.docs/design/reference/_shared/styles/FolioBar.module.css` — **EDITED by §5.4.1** (canonical pre-lift upgrade; adds `.FolioBar_folioRefreshButton` rule). Single-shot one-time upstream edit.
- `.docs/design/reference/_shared/chrome.js` — **EDITED by §5.4.1** (line ~202 inline style → class reference). Single-shot one-time upstream edit.

**V1.3.1 W4-merge confirmation (2026-05-19, post-L0-close):** v1.4.3 W4 merged at commit `d781f2b4` ("v1.4.3 W4-F: feedback wire-through (DOS-683) (#326)"). Empirical check of `git show --stat d781f2b4 -- 'wp/dailyos/theme/'` returns **zero files** — chrome lane's §10 W4-coupling matrix disjoint-write-sets assertion held in practice. W4 touched:

- `wp/dailyos/blocks/account-overview/` — substantial additions (block.json, edit.js, editor.css, style.css, render-functions.php +335 LOC; new view.js +561 LOC, view.tsx +58 LOC, view.asset.php +14 LOC). Chrome lane consumes account-overview UNCHANGED — chrome shell renders AROUND the block, not inside the block tree. No impact on chrome.
- `wp/dailyos/includes/class-dailyos-plugin.php` — +290 LOC (new `/dailyos/v1/nonce/verify` REST route + supporting infra). Chrome lane does NOT consume this route (chrome reads CPT info via PHP page-context functions, not via REST). No impact on chrome.
- `wp/dailyos/src/components/FeedbackAffordance/` — NEW TS component dir (`FeedbackAffordance.tsx` +509 LOC, `FeedbackAffordance.module.css` +187 LOC). Chrome lane uses plain JS (`chrome.js`), no TS build pipeline dependency. No impact on chrome.
- `wp/dailyos/tsconfig.json` — NEW TS build config. Chrome lane unaffected (chrome.js is plain JS).
- `wp/dailyos/tests/{FeedbackPayloadRedactionTest,FeedbackRuntimeContractTest,SurfaceNonceFeedbackEndpointTest,SurfaceNonceFeedbackInputTest}.php` — new PHPUnit suites. Chrome lane adds its own tests under `wp/dailyos/tests/theme/`. No overlap.
- `src-tauri/src/services/surface_nonce.rs` (+751 LOC), `surface_runtime/mod.rs` (+176 LOC), `version_dispatcher.rs` (+132 LOC), other substrate. Chrome lane is theme-side only; no Rust dependency.

**Net W4-impact verdict for chrome lane:** Zero. Chrome lane PRs land cleanly against post-W4 `public/dev` without rebase conflicts. AC #29 (Site Editor DB override) is unaffected by W4. AC #28 (Pill dual-existence allowlist) is unaffected. AC #18 (chrome_config JSON shape) is unaffected.

## 7. Acceptance criteria — consolidated (V1.2: 31 ACs)

[V1.1 list unchanged + V1.2 additions:]

L4-hands-on verifiable: AC #20, #22, #23, #24, #26, #27, #29.
PHPUnit-verifiable: AC #15, #17, #18, #19, #23, #24, #25.
Static-grep verifiable: AC #1-#14 (where applicable), #16, #21, #28, #30, #31.

**AC #28 (V1.2 PROMOTED to real body per consult C5 fold)** — Pill dual-primitive allowlist gate:
- CI step `wp/dailyos/scripts/check-chrome-block-collision.sh` enumerates basenames in `wp/dailyos/theme/assets/chrome/styles/*.module.css` (strip `.module.css` suffix), kebab-cases each, and asserts no matching directory under `wp/dailyos/blocks/<slug>/` exists EXCEPT for the allowlisted pair `Pill` ↔ `pill` (explicit grandfather per V1.2 §10).
- Failure mode: CI exits non-zero with message `"Chrome module <Name>.module.css collides with plugin block <slug>; reconcile or add to allowlist with L0 amendment"`.
- Pill reconciliation Linear ticket: **DOS-722** (Codebase Maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`, Medium priority, filed 2026-05-19 with packet approval).

**AC #31 (V1.2 NEW per B1+B2 fold)** — Canonical pre-lift upgrade lands as separate PR before chrome lane sync. [See §5.4.1 for full criteria.]

## 8. Out of scope (explicit deferrals, V1.2 with ticket IDs)

| Out of scope | Linear ticket | Where it goes |
|---|---|---|
| Chrome-as-Gutenberg-blocks (~700-1100 LOC / 2-4 days per cycle-1 challenge cost analysis) | **DOS-723** (Codebase Maintenance, Medium) | Separate L0 packet when scoped |
| Editor-iframe chrome rendering | n/a | Architectural non-goal |
| Scroll-spy in FloatingNavIsland chapter local-pill | **DOS-724** (Codebase Maintenance, Medium) | v1.4.5+ OR canonical Tauri-side patch + re-sync |
| Customizer postMessage live updates | n/a | Out of scope per AC #24 |
| Marketing-site patterns | n/a | Stays in `~/Studio/dailyos/` |
| Per-section advanced chrome variants | n/a | v1.4.4 W3 — original deliverable scope |
| Menus (WP Nav Menu / archive auto-generation) | n/a | v1.4.4 W2 |
| Pill chrome-version-vs-block-version reconciliation | **DOS-722** (Codebase Maintenance, Medium) | v1.4.5+ reconciliation lane |
| Trust/provenance theme-level styling | n/a | Plugin owns per architecture invariant |
| New Style Variation (Editorial Dark) | DOS-699 | Existing backlog ticket |
| **V1.2 NEW: `dailyos_project` tint resolution** (design-lens advisory 1) | **DOS-725** (v1.4.4 project, Medium) | v1.4.4 W2 L0 must resolve via ADR-0077 amendment OR new ADR before CPT registration |
| **V1.2 NEW: Canonical refresh-button upgrade outside chrome lane** | **DOS-721** (v1.4.4 project, High) | Lands BEFORE chrome lane Tier 1 |
| **V1.3 NEW: React `folio-refresh-button.tsx` class-based refactor** | **DOS-726** (Codebase Maintenance, Low; Tauri-freeze-blocked) | Deferred until Tauri-UI freeze lifts. Mockup substrate (canonical chrome.js) goes class-based via §5.4.1; production React component stays inline-style for now. L5 drift surveillance tracks the divergence so re-convergence happens at freeze-lift. |

**Ticket-ID note (V1.3 update):** All 6 tickets filed 2026-05-19 at L0 closure. IDs inserted above. DOS-721 (pre-W3 canonical) is **required to merge BEFORE chrome lane Tier 1 starts**.

**Linear ticket map (2026-05-19, L0 closure):**

| ID | Project | Priority | Status | Scope |
|---|---|---|---|---|
| [DOS-721](https://linear.app/a8c/issue/DOS-721) | v1.4.4 | High | Backlog | Pre-W3 canonical refresh-button class upgrade — gates chrome lane Tier 1 |
| [DOS-722](https://linear.app/a8c/issue/DOS-722) | Codebase Maintenance | Medium | Backlog | Pill primitive dual-existence reconciliation |
| [DOS-723](https://linear.app/a8c/issue/DOS-723) | Codebase Maintenance | Medium | Backlog | Chrome block-ification (4 modules → Gutenberg blocks) |
| [DOS-724](https://linear.app/a8c/issue/DOS-724) | Codebase Maintenance | Medium | Backlog | IntersectionObserver scroll-spy for FloatingNavIsland |
| [DOS-725](https://linear.app/a8c/issue/DOS-725) | v1.4.4 | Medium | Backlog | `dailyos_project` chrome tint resolution at W2 L0 |
| [DOS-726](https://linear.app/a8c/issue/DOS-726) | Codebase Maintenance | Low (Tauri-freeze-blocked) | Backlog | React `folio-refresh-button.tsx` class refactor |

## 9. Migration slots

**None.** Lift is theme assets only; no schema changes.

## 10. Architecture invariants

[V1.1 list unchanged + V1.2 sharpening of Pill scope:]

- **Chrome runtime-injection scope (V1.2 sharpened from V1.1).** Runtime DOM-injection via `chrome.js` is permitted ONLY for: `FolioBar`, `FloatingNavIsland`, `AtmosphereLayer`, `MagazinePageLayout`. **Pill is NOT in this set** — Pill is a child primitive consumed INSIDE the 4 shell modules (e.g., NavIsland active-state pill chips, FolioBar status indicators). Pill remains a standalone Gutenberg block (`wp/dailyos/blocks/pill/`) for body-content authoring. The dual-namespace existence is governed by AC #28 allowlist gate. Footer, breadcrumbs, entity bodies, claim cards, trust/provenance UI, action affordances, any per-page authorable region: STAYS as Gutenberg blocks. Future runtime-injection patterns outside this 4-module set REQUIRE a wave-plan amendment + new L0 packet documenting the parity-vs-authorability trade.

- **Downstream targets class contract, not DOM insertion order (V1.1, retained).**

- **W4-coupling matrix asserts disjoint write sets (V1.1, retained).**

- **Canonical source-of-truth for chrome assets (V1.2 NEW, V1.3 escape hatch added).** `.docs/design/reference/_shared/{chrome.js, styles/*.module.css, fonts/, fonts.css}` is the single canonical source for chrome assets. Chrome lane consumes via verbatim one-way sync. Direct edits to synced module CSS files are FORBIDDEN. WP-side patches LIMITED to `chrome.js` only (via `patch-chrome-js.py`).

  **V1.3 escape hatch (per challenge cycle-3 #4):** Any chrome improvement that benefits both Tauri-side mockup substrate and WP-side rendering MUST land in canonical first (per §5.4.1 precedent), then re-sync. **WP-only CSS deviations** are permitted IF AND ONLY IF: (a) the deviation requires an L0 amendment documenting why a WP-only rule is correct (e.g., editor-iframe-specific styling that doesn't apply to Tauri), AND (b) it lands as a separate non-synced overlay file at `wp/dailyos/theme/assets/chrome/styles/wp-overlay-*.css` enqueued AFTER the synced modules, AND (c) the overlay file's purpose + scope is documented in its header comment. The synced module CSS files (`{FolioBar,FloatingNavIsland,AtmosphereLayer,MagazinePageLayout,Pill}.module.css`) themselves are not edited under any circumstance — overlays add WP-only rules without modifying canonical-mirrored content. This preserves the one-way sync discipline while leaving a defined path for legitimate WP-only needs.

## 11. Reviewer matrix (L0 panel)

### Cycle-1 (V1.0) — COMPLETE — 4× CONDITIONAL APPROVE → V1.1 fold
### Cycle-2 (V1.1) — COMPLETE — 2× APPROVE + 3× CONDITIONAL APPROVE (3 BLOCKERS + 14 clerical) → V1.2 fold
### Cycle-3 (V1.2) — COMPLETE — 4× APPROVE + 1× CONDITIONAL APPROVE (4 clerical must-fix) → V1.3 fold
### Cycle-4 (V1.3) — IN FLIGHT (codex challenge only)

Other 4 cycle-3 reviewers (architecture, design-lens, codex consult, wp-block-themes) returned unconditional APPROVE in cycle-3 against V1.2. V1.3 deltas (AC #30 alert-red alias, §5.4.1 reference acknowledgment, AC #31a/#31b split, §10 escape hatch, V1.2 misstatement fix, POSIX regex) do not touch any of their cycle-3 concerns. Their APPROVE verdicts carry forward to V1.3.

Cycle-4 dispatches `/codex challenge` only to verify the 4 clerical folds resolve. If APPROVE, L0 closes unanimous (4 carry-forward + 1 fresh). Verdict artifact: `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-challenge-cycle4.md`.

Same 5-reviewer panel. Re-dispatched against V1.2 with explicit "verify V1.2 fold items resolve cycle-2 BLOCKERS + clerical items" framing.

| Reviewer | Cycle-3 verdict artifact |
|---|---|
| `/codex challenge` | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-challenge-cycle3.md` |
| `ce-architecture-strategist` | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle3.md` |
| `/codex consult` | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle3.md` |
| `ce-design-lens-reviewer` | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md` |
| WP-block-themes-grounded (general-purpose + skill refs) | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle3.md` |

**Pass rule:** Unanimous APPROVE. **CLAUDE.md pacing rule says "2 revision cycles without convergence ⇒ L6"** — V1.2 is the 2nd revision (cycle-3 dispatch). If V1.2 produces unanimous APPROVE, L0 closes. If any reviewer returns BLOCKED or substantive CONDITIONAL, L6 escalation per CLAUDE.md (single revision cycle remaining ≤ 0). Memory `feedback_review_loop_l6_policy` allows continued looping for high/critical/architectural issues vs L6 at strict cycle count; this packet treats cycle-3 as the convergence target with L6 as the escalation if blockers persist.

## 12. References

[V1.1 list unchanged + V1.2 additions:]

- `docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md` (V1.2 K-in rescan cite) — precedent for wave-plan amendment when scope substantively changes.
- WP developer docs (URLs cited inline in V1.1 codex challenge + V1.2 wp-skill verdicts):
  - https://developer.wordpress.org/themes/global-settings-and-styles/settings/custom/
  - https://developer.wordpress.org/block-editor/how-to-guides/themes/theme-json/
  - https://developer.wordpress.org/themes/core-concepts/including-assets/
  - https://developer.wordpress.org/block-editor/how-to-guides/enqueueing-assets-in-the-editor/
  - https://developer.wordpress.org/themes/templates/template-parts/
  - https://developer.wordpress.org/block-editor/explanations/architecture/full-site-editing-templates/
  - https://developer.wordpress.org/reference/functions/is_customize_preview/
  - https://developer.wordpress.org/reference/functions/wp_enqueue_style/

## 13. Appendix — Site Editor DB override reset path (V1.2 corrected per challenge I1 + wp-skill A2)

If a developer accidentally saves `parts/header.html` (or `parts/footer.html`) edits in Site Editor, the `wp_template_part` DB row overrides the theme file.

**Reset via WP-CLI (V1.2 verified):**

```sh
# List all template-part DB overrides
wp post list --post_type=wp_template_part --format=table --fields=ID,post_name,post_title,post_modified

# Delete a specific override by ID (force = skip trash)
wp post delete <ID> --force

# Or delete all template-part overrides for this theme in one command:
wp post list --post_type=wp_template_part --format=ids | xargs -I{} wp post delete {} --force
```

**Why not `wp template-part delete`:** That command does not exist in WP-CLI core. `wp help template-part` returns `Error: 'template-part' is not a registered wp command.` V1.1 packet erroneously documented this command; V1.2 corrects to the validated `wp post` path above.

**SQL fallback:** Template parts are scoped via `wp_theme` taxonomy on `wp_term_relationships` (per wp-skill A2 fold). Raw SQL deletion would require joining `wp_posts` → `wp_term_relationships` → `wp_terms` → `wp_term_taxonomy` to filter by theme. Complex enough that WP-CLI path is the documented option. **SQL example dropped from V1.1 §13 in V1.2.**

**Prevention:** L4 hands-on test sequence per AC #29 includes "edit Site Editor template part, save without edits, reload" loop to surface any inadvertent DB override before it lands in user state.

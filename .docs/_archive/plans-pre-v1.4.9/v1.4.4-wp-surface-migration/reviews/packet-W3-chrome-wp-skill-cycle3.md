# L0 Cycle-3 Review — Packet W3 Chrome (WordPress Block-Themes Lens)

**Reviewer:** wp-block-themes skill lens (`/Users/jamesgiroux/.claude/skills/wp-block-themes/`) + upstream WP docs.
**Packet under review:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` (V1.2).
**Date:** 2026-05-19.
**Verdict:** **APPROVE.**

---

## K-in result (V1.2 rescan, independent of packet's claim)

Re-greped `docs/solutions/` + `.docs/decisions/`:

- `docs/solutions/` — 14 .md files at scan time (matches V1.2 §3 rescan). Re-ran greps for `wp\|wordpress\|theme.json\|chrome\|enqueue\|block.theme\|template.part\|gutenberg`. Two relevant hits, both already cited by V1.2 §3:
  - `workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` — W4 substrate-discovery note; informs §10 W4-coupling matrix.
  - `tooling-decisions/phpcs-warning-severity-zero-prevents-warning-only-ci-fails-2026-05-19.md` — WP PHPCS posture; relevant to `functions.php` lint.
- `.docs/decisions/` — ADRs 0073, 0076, 0077, 0129, 0130 (the 5 packet-cited ADRs) are the only WP-surface-touching ADRs. No conflicts; no reinvention; nothing overridden.

**K-in clean. No BLOCKED-with-cited-path.**

---

## Cycle-2 must-fix + advisory resolution (7 items)

| ID | Cycle-2 finding | V1.2 location | Resolved? |
|---|---|---|---|
| **M1** | `Requires at least` floor declaration | §2 changelog + §5.5 (PR #315 style.css `Requires at least: 6.5`, ≥ 6.3 satisfied) | **YES** — 6.5 floor strictly exceeds the 6.3 needed for `enqueue_block_assets` iframe behavior; AC #26 contract unambiguous. |
| **M2** | Expanded prohibition list in §5.5 | §5.5 expansion block + §2 changelog | **YES** — `register_sidebar`, `register_nav_menus`, `add_theme_support('custom-header')`, `add_theme_support('custom-background')` all explicitly forbidden. Block-theme paradigm preserved. |
| **A1** | Digit-segment hyphenation audit | §5.1 audit block + §2 changelog | **YES** — audit confirms `spice-turmeric-10` etc. are alphanumeric slugs (single-digit segments after letters), not multi-digit numeric segments; emission is clean. AC #30 grep would surface future regressions. |
| **A2** | §13 SQL example fix | §13 rewrite + §2 changelog | **YES** — SQL example dropped entirely; validated `wp post list/delete` path used. Verified locally that `wp template-part delete` is not a registered command (per V1.2 §13 note). |
| **A3** | AC #24 rationale sharpened | §5.5 AC #24 rationale block | **YES** — visual-conflict rationale (Customizer renders its own preview chrome; stacked-chrome DOM injection breaks preview) is the defensible justification per `is_customize_preview()` docs warning. |
| **A4** | `:root` cascade semantics note | §5.1 cascade-semantics block | **YES** — "`:root` last-declaration-wins; aliases override `--wp--preset--*` only if loaded AFTER" is correct cascade model. Load order enforced by `wp_enqueue_style` `$deps`. |
| **A5** | `wp_body_open` advisory | §5.4 non-substitute block | **YES** — server-side once-per-render correctly characterized as structurally incapable of preventing client-side re-invocation. Optional mount-marker hardening deferred (out of scope for L1). |

**All 2 must-fix and all 5 advisory items resolved in V1.2.** No regressions introduced.

---

## Cycle-3 stress-area probes (4 items)

### Stress 1 — §5.4.1 canonical pre-lift upgrade. **APPROVE.**

Question: WP block-themes best practice on theme assets depending on canonical source-of-truth files OUTSIDE the theme dir.

The wp-block-themes skill references (`creating-new-block-theme.md`, `theme-json.md`, `templates-and-parts.md`, `style-variations.md`, `patterns.md`, `debugging.md`) are silent on external-canonical asset sourcing. Upstream WP docs at https://developer.wordpress.org/themes/block-themes/theme-structure/ specify only the *committed shape* of the theme directory — not the provenance of how files land there. Standard practice (e.g., `@wordpress/scripts` builds, the "Create Block Theme" plugin's export workflow, vendored Tailwind/PostCSS outputs across many production WP themes) demonstrates the pattern of "built/synced outputs commit into the theme dir; sources live elsewhere" as routine and uncontroversial.

V1.2's sync model is sound under this convention:
1. Canonical lives at `.docs/design/reference/_shared/` (outside theme dir).
2. `sync-chrome.sh` materializes the assets into `wp/dailyos/theme/assets/chrome/` (inside theme dir, committed).
3. WP only ever loads from the theme dir; the canonical path is invisible to runtime.
4. `.synced-from` records source-sha provenance for L5 drift detection.
5. The §10 invariant "Canonical source-of-truth for chrome assets" forbids editing synced files directly, closing the drift loop.

The Tauri-side mockup substrate also being a downstream consumer (alongside the WP theme) is a *strengthening* of the canonical-source-of-truth pattern, not a weakening — it makes the canonical genuinely surface-independent.

**Verdict: aligned with WP block-themes practice. No skill-ref violation.**

### Stress 2 — §10 invariant "WP-only module-CSS overlays forbidden". **APPROVE.**

Question: future scenarios where WP-only CSS would be the right call.

Considered four scenarios where WP-only CSS *might* be argued for:

1. **WP-emitted markup not present in Tauri shell** (e.g., admin bar at `#wpadminbar`, Customizer chrome). V1.2 §5.5 already removes the admin bar via `show_admin_bar(false)` and `add_filter('show_admin_bar', '__return_false')`; Customizer is gated out via AC #24. These are handled in `functions.php`, not module CSS. **Not a counterexample.**
2. **Stock-theme co-existence styles.** Stock-theme contexts deliberately don't load chrome assets per AC #23 (negative gate). **Not a counterexample.**
3. **WP-version-conditional fixes** (e.g., a 6.5-only regression). Should land as a canonical chrome.js patch (since canonical chrome.js can detect at runtime), or as a `patch-chrome-js.py` patch — not as theme-only CSS. **Not a counterexample.**
4. **WP-block-editor iframe-specific styles** (e.g., a rule that only applies inside the editor's iframe rendering context). The correct surface is `enqueue_block_assets` priority+context filtering (already in §5.5 hook-scoping table), or `editorStyle` in block.json (plugin domain, not theme chrome). **Not a counterexample.**

Every plausible "WP-only" scenario either resolves to:
- a `functions.php` change (hook, filter, version branch) — already permitted by V1.2
- a canonical chrome.js patch + re-sync — preserves single source-of-truth
- a `patch-chrome-js.py` WP-specific patch — already permitted

The "WP-only module-CSS overlay" path is genuinely closed. The §10 invariant is sound.

**Verdict: invariant correctly forecloses drift. APPROVE.**

### Stress 3 — AC #31 canonical pre-lift upgrade as separate PR. **APPROVE.**

Question: alignment with WP plugin/theme distribution and CI patterns.

V1.2 §5.4.1 sequences:
1. Pre-W3 ticket: canonical upgrade (2 files in `.docs/design/reference/_shared/`, ~35 LOC).
2. Pre-W3 PR merges to `dev`.
3. W3-1 (tokens + aliases) starts; `sync-chrome.sh` lifts post-upgrade canonical.

This matches the standard WP-ecosystem build-pipeline pattern (e.g., upstream library bump lands first; downstream theme/plugin re-syncs in a second PR). It also matches the project's own monorepo discipline — substrate edits land on `dev` first, surface PRs follow on the same trunk.

The pre-W3 ticket touches Tauri-side substrate ONLY (`.docs/design/reference/_shared/`) — disjoint from `wp/dailyos/theme/**`. No risk of step-on with v1.4.3 W4 (which is independently blocked per V1.2 K-in note); no risk of step-on with active wave PRs touching plugin or theme.

L2 review on the pre-W3 PR is correctly scoped (codex review + design-reviewer per §5.4.1), since the change is presentation-layer only (no abilities, no claims, no provenance — Intelligence Loop integration check exempt at the same posture as the chrome lane).

**Verdict: sequencing sound. APPROVE.**

### Stress 4 — AC #30 regex sufficiency. **APPROVE with one observational note.**

Question: regex `var\(--[A-Za-z0-9_-]+(?:,[^)]+)?\)` sufficient per WP CSS-custom-property emission conventions, particularly underscore-variant tokens.

WP emission rules (per https://developer.wordpress.org/themes/global-settings-and-styles/settings/custom/ and https://developer.wordpress.org/block-editor/how-to-guides/themes/theme-json/):
- camelCased keys → kebab-case (`lineHeight` → `line-height`).
- Numeric segments hyphenate digit-by-digit (`abc123` → `abc-1-2-3`).
- Slug-style keys (kebab-case alphanumeric) emit verbatim.

**Underscore behavior:** WP does NOT transform underscores. A theme.json `settings.custom.dailyos.snake_case_key` would emit `--wp--custom--dailyos--snake_case_key`. Spot-checked `wp/dailyos/theme/theme.json` — palette slugs and custom-token keys are all kebab-case + alphanumeric (no underscores). The regex character class `[A-Za-z0-9_-]+` would correctly capture underscores if any chrome module references them; the alias-discipline gate (AC #30 step 3) would then assert the underscore-bearing alias has a matching theme.json source, surfacing the issue.

Test cases the V1.2 regex handles correctly:
- `var(--color-spice-turmeric)` → captured.
- `var(--color-spice-turmeric-10)` → captured (digit suffix).
- `var(--color-desk-charcoal-4)` → captured.
- `var(--alert-red, #dc2626)` → captured (with fallback).
- `var(--folio-height, 56px)` → captured.
- `var(--snake_case_var)` → captured (hypothetical underscore).
- `var(--space-2xl)` → captured (alphanumeric).

Test case the regex would NOT handle (intentional exclusion): nested `var(var(--x), var(--y))` — but WP doesn't emit nested var-fallbacks in custom-prop declarations, and chrome modules don't author them. **Not a concern.**

**Observational note (not a fix request):** The V1.2 classification step strips fallback values before alias-resolution; an alias like `var(--alert-red, #dc2626)` already exists in canonical chrome.js post-Patch 9a, so the regex captures it correctly and the classifier routes it as "alias-required" with the hex fallback preserved at the consumer site. This is the right shape — fallbacks are a consumer-site concern, not a token-emission concern.

**Verdict: regex sufficient. APPROVE.**

---

## Verdict

**APPROVE.**

V1.2 resolves all 2 must-fix + 5 advisory items from cycle-2 cleanly with no regressions. All 4 cycle-3 stress-area probes return alignment with WP block-themes practice and skill-reference conventions:

1. External-canonical asset sourcing is consistent with standard WP build-pipeline patterns.
2. The "no WP-only module-CSS overlays" invariant correctly forecloses drift paths.
3. Pre-lift upgrade sequencing matches monorepo + L2-then-PR conventions.
4. AC #30 regex correctly handles all WP-emitted token shapes including digit-suffixed, fallback-bearing, and (hypothetical) underscore variants.

K-in clean (independent rescan matches V1.2 §3). No path-α triggers. No L6 escalation conditions. No BLOCKED-with-cited-path.

From this reviewer's lens, the packet exits L0. Convergence to unanimous APPROVE depends on the remaining 4 cycle-3 reviewers (codex challenge, ce-architecture-strategist, codex consult, ce-design-lens-reviewer).

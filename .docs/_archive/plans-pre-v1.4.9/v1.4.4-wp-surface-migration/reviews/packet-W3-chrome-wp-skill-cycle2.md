# L0 Cycle-2 Review — Packet W3 Chrome (WordPress Block-Themes Lens)

**Reviewer:** wp-block-themes skill lens (`/Users/jamesgiroux/.claude/skills/wp-block-themes/`) + upstream WP docs.
**Packet under review:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` (V1.1).
**Date:** 2026-05-19.
**Verdict:** **CONDITIONAL APPROVE** — 2 must-fix conditions, 3 advisory notes. No BLOCKED-with-cited-path findings; no path-α triggers; no critical rewrite.

---

## K-in result

Greped `/Users/jamesgiroux/Documents/dailyos-repo/docs/solutions/` + `.docs/decisions/` for WP/block-theme/enqueue/template-part/chrome.js/Gutenberg prior art.

- `docs/solutions/` — zero hits on chrome lift, runtime DOM-injection, template-part DB override, or asset-hook scoping. One cross-reference (already cited by packet §3): `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` (W4 substrate-type discovery; informs §10 W4-coupling matrix). One additional cross-reference: `docs/solutions/tooling-decisions/phpcs-warning-severity-zero-prevents-warning-only-ci-fails-2026-05-19.md` (WP PHPCS posture; relevant to `functions.php` lint, already cited).
- `.docs/decisions/` — ADRs 0129 (composable surfaces / WP Studio), 0130 (surface-independent composition), 0077 (magazine layout), 0073 (editorial design), 0076 (brand identity) are the relevant WP-touching set. All 5 are consumed verbatim by the packet (§3 table). 0111 (surface-independent ability invocation) is correctly flagged orthogonal.

**K-in verdict: clean.** No documented prior solution reinvented. No BLOCKED-with-cited-path.

---

## Validation against upstream WP docs (10 assertions)

### 1. §5.1 + AC #30 — `settings.custom.dailyos.tokens` emission rule. **CONFIRMED.**

Packet claim: nested keys emit `--wp--custom--<level1>--<level2>--<name>` with camelCase → kebab-case.

Upstream (https://developer.wordpress.org/themes/global-settings-and-styles/settings/custom/): "The generated CSS custom property will follow this pattern: `--wp--custom--{key}--{value}`." + "WordPress will automatically hyphenate camel-cased names." Example: `lineHeight` → `line-height`. Nesting: each level joined by `--`.

Upstream (https://developer.wordpress.org/block-editor/how-to-guides/themes/theme-json/): "`camelCased` keys are transformed into its `kebab-case` form, as to follow the CSS property naming schema."

Packet's `--wp--custom--dailyos--tokens--folio-height` emission is correctly specified. AC #30's discipline gate (grep `wp/dailyos/theme/theme.json` for source names) is the correct enforcement primitive.

**Minor note:** Packet should also warn that numeric keys are hyphenated digit-by-digit (`abc123` → `abc-1-2-3`). If any of the chrome modules' `var(--*)` references contain numerics (e.g., `--space-32`), the matching `theme.json` key + alias source must follow the digit-by-digit transformation. → see Advisory A1.

### 2. §5.5 hook-scoping table. **CONFIRMED with one sharpening.**

Packet's table maps `wp_enqueue_scripts` (frontend), `enqueue_block_assets` (frontend + editor iframe), `enqueue_block_editor_assets` (editor UI / not content iframe), `customize_preview_init`, `after_setup_theme`.

Upstream (https://developer.wordpress.org/block-editor/how-to-guides/enqueueing-assets-in-the-editor/): "As of WordPress 6.3, all assets added through the [`enqueue_block_assets`] PHP action will also be enqueued in the iframed Editor." + "`enqueue_block_editor_assets` … Whenever you need to enqueue assets for the Editor itself (i.e. not the user-generated content)." Pre-6.3 backward-compat note: pre-6.3 `enqueue_block_assets` doesn't enter the iframe.

**Sharpening — must-fix M1:** Packet must declare a minimum WP version floor (likely `>= 6.3`) somewhere in §5.5 or in `style.css` theme header (`Requires at least: 6.3`). Without it, the AC #26 claim "alias proof works in editor iframe" is conditional on WP version. The 6.3 floor is consistent with v1.4.3 W3 magazine-theme PR #315; just needs to be made explicit.

### 3. §5.6 + AC #20 — template-part block markup requirement. **CONFIRMED.**

Packet claim: lift target is `<!-- wp:group {"className":"dailyos-header-noop"} --><div class="wp-block-group dailyos-header-noop"></div><!-- /wp:group -->`, NOT raw `<div>`.

Upstream (https://developer.wordpress.org/themes/templates/template-parts/): "Template parts in block themes contain block markup and nothing else."

Packet's V1.1 sharpening (CH6 fold) is correct. Raw HTML would fail Site Editor parse and likely render as a freeform/HTML block on save. The `wp:group` + matching closing comment + matching outer `<div class="wp-block-group ...">` is the standard valid serialization pattern.

### 4. §5.6 + AC #29 + §13 — Site Editor DB override mechanic. **CONFIRMED.**

Packet claim: saving a template part in Site Editor writes a `wp_template_part` post that overrides theme file forever; reset via `wp template-part delete` or SQL `DELETE FROM wp_posts WHERE post_type='wp_template_part'`.

Upstream (https://developer.wordpress.org/themes/templates/template-parts/): "if you save the parts from this screen, they will be stored in the database and will overrule any templates in your theme."

Upstream (https://developer.wordpress.org/block-editor/explanations/architecture/full-site-editing-templates/): "When a user edits a template (or template-part), the initial theme template file is kept as is but a forked version of the template is saved to the `wp_template` custom post type (or `wp_template_part` for template parts)." + "The rendering/fetching of templates only need to consider the custom post type templates."

Mechanic accurate. §13 reset path acceptable.

**Sharpening — advisory A2:** The packet's §13 SQL example filters by `post_author IN (SELECT … WHERE user_login = <theme-author-user>)` — that's wrong; template-part DB rows are not author-scoped to the theme author. They're scoped via the `tax_input` of `wp_theme` taxonomy (the term slug matches the theme slug). Replace with `AND ID IN (SELECT object_id FROM wp_term_relationships WHERE term_taxonomy_id = (SELECT term_taxonomy_id FROM wp_term_taxonomy tt JOIN wp_terms t ON tt.term_id=t.term_id WHERE tt.taxonomy='wp_theme' AND t.slug='dailyos'))` — OR just rely on the WP-CLI path. (Recommendation: drop the SQL fallback or replace it with the simpler WP-CLI invocation; the SQL example as written will not reliably target the right rows.)

### 5. AC #23 — stock-theme negative gate. **CONFIRMED; verification approach below.**

Packet claim: under TwentyTwentyFive, no chrome assets load. Mechanism: assets live in `wp/dailyos/theme/`; chrome enqueue is hooked from `wp/dailyos/theme/functions.php`, which is only loaded when the active theme IS `dailyos`.

This is correct WP behavior — `functions.php` of a non-active theme is never included. The negative gate is structural, not conditional.

**Realistic verification approach (suggested, AC #23 already approximates this):**
1. PHPUnit integration test using `switch_theme('twentytwentyfive')` then `do_action('wp_enqueue_scripts')` and inspect `wp_styles()->queue` / `wp_scripts()->queue` for absence of `dailyos-chrome`, `dailyos-folio`, `dailyos-tokens`, `dailyos-aliases`, `dailyos-nav`, `dailyos-magazine`, `dailyos-atmosphere`, `dailyos-pill`, `dailyos-fonts` handles.
2. Alternatively: render a frontend page via `WP_UnitTestCase` request and grep output for `wp/dailyos/theme/assets/chrome/` substring — assert zero matches.
3. WP-CLI/Playwright variant for L4: `wp theme activate twentytwentyfive && curl localhost/?p=N | grep -c 'wp/dailyos/theme/assets/chrome/'` should return 0.

Recommend codifying option (1) as the AC #23 verification method — fastest, deterministic, runs in PHPUnit.

### 6. AC #24 — `! is_customize_preview()` guard. **CORRECT in pattern, but packet rationale needs refinement.**

Upstream (https://developer.wordpress.org/reference/functions/is_customize_preview/): the function "Whether the site is being previewed in the Customizer." Returns true only inside the Customizer preview iframe.

The guard pattern (`if ( is_admin() || is_customize_preview() ) return;`) is syntactically valid WP. The packet's stated rationale — "Customizer preview has its own preview chrome that would conflict" — is *plausible* but not the canonical WP rationale.

**Caveat:** Upstream-fetched commentary warns that `! is_customize_preview()` is not a blanket "Customizer-safe" guard for asset enqueue — many WP core features explicitly load assets *inside* the preview to support live editing. For chrome.js specifically (which DOM-injects), the guard is defensible because the chrome would interfere with Customizer's own preview UI. **Acceptable as written**, but document the narrower justification in §5.5: "skip in Customizer preview because chrome.js DOM-mutates `document.body` in ways incompatible with Customizer's preview frame management" rather than the looser "its own preview chrome" framing.

→ Advisory A3.

### 7. §5.5 functions.php bounded-behavior list. **CONFIRMED; one addition recommended.**

Packet's prohibition list covers: DB writes, runtime invocation, claim/trust/provenance branching, HTTP to runtime, block/REST/CPT registration, top-level execution, closing `?>`.

This aligns with WP block-theme conventions for keeping the theme presentation-only.

**Recommended addition — must-fix M2:** explicitly prohibit `register_sidebar()` / dynamic-sidebar / widget registration. Block themes by convention do not register classic widget sidebars — that's classic-theme territory. Allowing it would mix paradigms and break Site Editor expectations. Also recommend explicitly prohibiting `add_theme_support('custom-header')` / `('custom-background')` (legacy Customizer surfaces — irrelevant under FSE) and `register_nav_menus()` (use `wp_navigation` post type instead, which the Tier-6 deferral already implies).

### 8. §5.1 enqueue dependency chain. **CONFIRMED with caveat.**

Packet claim: `dailyos-fonts → dailyos-tokens → dailyos-aliases → chrome modules → chrome.js`. Dependency graph via `wp_enqueue_style`'s `$deps`.

Upstream (https://developer.wordpress.org/reference/functions/wp_enqueue_style/): `$deps` is "An array of registered stylesheet handles this stylesheet depends on." WP's `WP_Dependencies` machinery handles topological ordering, and `WP_Styles` (extending `WP_Dependencies`) emits `<link>` tags in dependency-resolved order. In practice, dependencies ARE emitted before dependents in the document — this is well-established WP behavior across many years and many themes, even if the upstream doc page doesn't quote-pin it.

**Caveat — advisory A4:** Stylesheet **cascade** is dependent on `<link>` order in the HTML. WP's dependency resolver IS order-preserving for declared deps. However, **CSS specificity, not declaration order, ultimately decides which rule wins** if two rules collide. The alias layer relies on `:root { --color-spice-turmeric: var(--wp--preset--...); }` redefining the variable; that works because variable assignment uses last-declaration-wins at the same specificity. AC #30's discipline gate is sufficient. No code change needed; just acknowledge in §5.1 that the contract is "declarations resolve last-wins at `:root`" not "module CSS overrides preset CSS."

### 9. §5.4 chrome.js Patch 9 idempotency guard. **ACCEPTABLE; `wp_body_open` is NOT a better alternative for this use case.**

Question raised: is `body.querySelector('.FolioBar_folio, ...')` the right idiom, or would the WP `wp_body_open` hook prevent re-injection more reliably?

Upstream (https://developer.wordpress.org/reference/hooks/wp_body_open/): "Triggered after the opening body tag." Fires once per server-side page render via `wp_body_open()` template call.

**Analysis:** `wp_body_open` is a *server-side* hook that fires once per HTML render. It does not protect against client-side re-execution of `chrome.js` (which is what Patch 9 guards). The scenarios the packet guards against (preview reloads, partial refresh, future iframe asset routing) are CLIENT-SIDE phenomena — `wp_body_open` is structurally incapable of preventing them. Patch 9's DOM-state-based idempotency guard is the correct WP-side pattern.

**However:** the packet could *additionally* use `wp_body_open` to emit a server-rendered marker like `<div id="dailyos-chrome-mount" data-dailyos-chrome-status="pending"></div>` and have chrome.js check `getElementById('dailyos-chrome-mount').dataset.dailyosChromeStatus !== 'mounted'`. This would make the idempotency contract explicit at the markup level instead of relying on selector existence. **Not required, but recommended for L5 drift hardening.** Patch 9 as specified is sufficient for L1.

→ Advisory A5 (optional hardening).

### 10. §5.7 footer-as-blocks vs §5.6 header-as-no-op asymmetry. **DEFENSIBLE; explicitly codified in §10 invariant.**

Asymmetry:
- Header `parts/header.html` → no-op group block; chrome.js injects FolioBar at runtime (NOT Site-Editor-authorable).
- Footer `parts/footer.html` → Gutenberg-blocks footer (Site-Editor-authorable).

Per WP block-theme best practice, *both* template parts should be Site-Editor-authorable, and the asymmetry IS a divergence from convention. **However**, the packet's §1 + §10 codification ("Chrome runtime-injection scope" invariant + 4-module allowlist) explicitly names the trade and bounds it. The justification is grounded: chrome must achieve Tauri-WP parity via shared `chrome.js`; footer has no Tauri equivalent and IS authorable content.

The asymmetry is **defensible because it's named, bounded, and gated** — not because it's WP-best-practice (it isn't). The packet handles this correctly by:
1. Naming the trade in §1.
2. Codifying the 4-module allowlist in §10.
3. Requiring a wave-plan amendment + new L0 packet for any expansion.
4. Quantifying the block-ification cost (§8 + CH5 fold) as the exit path.

**No change required.** Note that the FSE convention "every part should be Site-Editor-authorable" is being explicitly traded for chrome parity — this trade is now part of the project's documented invariants. Approve.

---

## Conditions for cycle-3 fold

**Must-fix (block APPROVE until folded):**

- **M1.** Declare WP version floor (`Requires at least: 6.3` in `style.css` theme header + explicit mention in §5.5 hook-scoping table) so AC #26's editor-iframe alias proof has an unambiguous version contract. (per https://developer.wordpress.org/block-editor/how-to-guides/enqueueing-assets-in-the-editor/ — 6.3 is the version where `enqueue_block_assets` enters the editor iframe.)
- **M2.** Expand §5.5 functions.php prohibition list to explicitly forbid `register_sidebar()` / classic widget sidebars, `register_nav_menus()` (use `wp_navigation` per FSE convention), `add_theme_support('custom-header')`, `add_theme_support('custom-background')`. These would mix classic-theme paradigms into a block theme.

**Advisory (record-only; no block):**

- **A1.** Note in §5.1 / AC #30 that numeric segments in `theme.json` `settings.custom` keys hyphenate digit-by-digit (`abc123` → `abc-1-2-3`); audit chrome modules' `var(--space-32)` / `var(--space-64)` style references against this rule when generating the alias source list.
- **A2.** Replace §13's SQL fallback example. The current author-filter is wrong; template-part rows are theme-taxonomy-scoped via `wp_theme` tax + `wp_term_relationships`, not `post_author`. Recommend dropping the SQL block entirely and recommending WP-CLI exclusively, OR fix the query to filter via `wp_term_relationships → wp_theme:dailyos`. Per https://developer.wordpress.org/block-editor/explanations/architecture/full-site-editing-templates/ template-part scoping.
- **A3.** §5.5 — sharpen the AC #24 rationale: "chrome.js DOM-mutates `document.body` in ways incompatible with Customizer's preview frame management" rather than the looser "Customizer has its own preview chrome." Documents the narrower, defensible justification per https://developer.wordpress.org/reference/functions/is_customize_preview/.
- **A4.** §5.1 — note that the alias contract is "last-declaration-wins for `:root` custom property assignment", not "module CSS overrides preset CSS." Pre-empts confusion if a future CSS reviewer challenges the cascade model. Per https://developer.wordpress.org/reference/functions/wp_enqueue_style/ + CSS cascade semantics.
- **A5.** (Optional L5 hardening) — consider adding `wp_body_open` server-side mount-marker (`<div id="dailyos-chrome-mount" data-dailyos-chrome-status="pending">`) for chrome.js to flip to `mounted`, making Patch 9's idempotency contract markup-explicit instead of selector-implicit. Defer to v1.4.5 or later; not L1 blocker.

---

## Verdict

**CONDITIONAL APPROVE.**

All 10 upstream-claim validations confirmed (with 2 must-fix sharpenings + 5 advisory notes). The packet's WP-block-theme claims hold up against upstream documentation. K-in is clean. The header-vs-footer asymmetry is named, bounded, and gated by a §10 invariant — defensible.

Cycle-2 must-fixes (M1, M2) fold into V1.2 in cycle-3. No critical rewrite. No L6 escalation trigger. No path-α threshold reached. No BLOCKED-with-cited-path.

This reviewer joins the cycle-1 panel's CONDITIONAL APPROVE pattern. Re-dispatch the panel against V1.2 once M1+M2 are folded; if remaining 3 cycle-1 reviewers (codex challenge, ce-architecture-strategist, codex consult, ce-design-lens-reviewer) likewise resolve, packet converges to unanimous APPROVE and exits L0.

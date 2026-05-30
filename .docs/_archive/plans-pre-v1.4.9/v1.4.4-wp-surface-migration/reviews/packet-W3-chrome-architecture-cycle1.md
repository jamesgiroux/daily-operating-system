# L0 Architecture Review — Packet W3 Chrome Lane (Pulled Forward) — Cycle 1

**Reviewer:** `compound-engineering:ce-architecture-strategist`
**Mode:** Pattern compliance + design integrity (architect-reviewer per L0 matrix)
**Date:** 2026-05-19
**Packet under review:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` V1.0

## Verdict: **CONDITIONAL APPROVE**

Three conditions, none blocking the lane's strategic shape. Detail below.

---

## 1. K-in audit (substrate-grep, before scoring)

Per CLAUDE.md "Knowledge store discovery" + engineering-ladder.md L0 K-in obligation. Reviewer K-in independent of packet's §3 self-report.

### `docs/solutions/` — searched

Greps: `chrome`, `wp.theme`, `block.theme`, `gutenberg`, `template.part`, `runtime.inject`, `DOM.mutation`, `enqueue`, `theme.json`, `magazine`, `ollie`, `woff2`, `font.face`, `folio`, `nav.island`, `atmosphere`. **Zero hits in chrome lift territory.** Three unrelated entries (k-in grep methodology, phpcs warning severity, pnpm/symlink) — none touch chrome/theme/runtime-injection.

**Verdict: net new K territory. No documented prior solution reinvented.** Confirms packet §3.

### `.docs/decisions/` — searched

`theme | primary surface | magazine | gutenberg | block theme | template part` → 15 ADRs touch adjacent concerns. The 5 the packet names (0129, 0130, 0077, 0073, 0076) are correctly the load-bearing ones. ADR-0083 (product vocabulary), 0084 (surface jobs-to-be-done), 0090 (user entity page architecture), 0111 (surface-independent ability invocation), 0118 (AI harness) are touched but not consumed — packet correctly treats them as orthogonal.

**No reinvented substrate.** Packet does not duplicate any documented decision; it consumes them.

---

## 2. ADR compliance

| ADR | Anchor | Packet treatment | Compliance |
|---|---|---|---|
| **0129** — Composable surfaces; WP Studio as primary surface | §4 (WP path Abilities API + MCP Adapter), §6 (block-shaped composition is "the right shape"), §7 (Tauri reorients to runtime-host) | Packet's chrome lift instantiates §2 ("custom block library + magazine theme + custom post types"). Theme shipping is in-charter. Runtime-injection model for chrome layer doesn't violate §6 because §6 scopes "block-shaped composition" to *user-authored content*, not chrome-shell. Packet §1's note ("end-user authoring of chrome via Site Editor is the v1.4.x non-goal") is correctly bounded. | **COMPLIANT** |
| **0130** — Surface-independent composition contract | §1 (substrate owns composition), §4 (renderers, not authors), §5 (authorship boundary) | Chrome is not composition. Chrome is surface shell (folio + nav + atmosphere + page-layout container). Compositions render *inside* `MagazinePageLayout`'s content slot. Chrome carries zero `Block`, zero `Section`, zero `ProvenanceRef`. ADR-0130 is intact and untouched. | **COMPLIANT** (orthogonal) |
| **0077** — Magazine layout editorial redesign | §"Chrome (Shared, Fixed)" — folio bar / floating nav island / atmosphere / asterisk watermark; §"Page Container"; §"Section Patterns" | Packet lifts the exact chrome ADR-0077 §Design System specifies. Folio dimensions, nav island position, atmosphere model, page max-width 1100px, watermark sizing — all match. The lift IS ADR-0077 manifesting on WP. | **COMPLIANT — direct instantiation** |
| **0073** — Editorial design language | §1 (Newsreader / DM Sans / JetBrains Mono); §3 (breathing room); §4 (cards only featured) | Fonts package lifts the canonical four-family stack (adds Montserrat 800 for brand mark per ADR-0076). Spacing tokens consumed from `design-tokens.css` lift. `font-display: swap` at AC #9 is the right choice for FOIT-avoidance under editorial-warmth aesthetic. | **COMPLIANT** |
| **0076** — Brand identity (paper / desk / spice / garden) | §2 (color families with material names); §3 (DOS lineage as brand asset) | `design-tokens.css` lift carries the named palette intact. AtmosphereLayer tint variants (turmeric, terracotta, larkspur, olive, eucalyptus) are page-context spice/garden assignments per ADR-0077 §Atmosphere. Pill `tone` variants (sage / turmeric / terracotta / larkspur / olive / eucalyptus / neutral) are paint tokens, NOT trust-band semantic tokens — packet §10 calls this out explicitly. | **COMPLIANT** |

**ADRs reviewed for relevance, not consumed (orthogonal):** 0083 (product vocabulary — chrome strings should still pass discipline; not a packet ask but flag to L4), 0084 (surface jobs-to-be-done — applies to body content, not chrome), 0090 (user entity page architecture — v1.4.4 W2 concern), 0007 (dashboard is the product — strategic predecessor, no direct consumption). Packet §3's orthogonality list is correct.

---

## 3. Architectural trade — "chrome is runtime-injected, not block-tree-rendered"

**Packet location:** §1 strategic framing, §5.6 template-part no-op, §10 new invariant.

### The trade explicitly stated

`parts/header.html` becomes a 1-line no-op `<div>`. `chrome.js` (lifted from canonical) reads `body.dataset.*` populated by PHP `wp_add_inline_script('dailyos-chrome', 'window.dailyosChrome = ...', 'before')` and DOM-injects FolioBar + FloatingNavIsland + AtmosphereLayer client-side, before first paint.

This **deviates** from the project-wide "many small Gutenberg blocks" strategy (memory `project_wp_block_custom_vs_core_strategy`: "each Tauri component or pattern gets a Gutenberg block equivalent").

### Is the trade sound?

**Yes, for chrome specifically.** Justifications hold:

1. **Tauri-WP chrome parity is real and load-bearing.** Identical `chrome.js` + identical module CSS on both surfaces means the canonical `.docs/design/reference/_shared/` source is single-source-of-truth across surfaces. Block-ifying chrome would fork chrome rendering logic (React component on Tauri, render.php + edit.js on WP) and introduce parity drift surface. The sync-script + 8-patch model in §5.4 preserves parity at theme-asset granularity.

2. **Chrome is not user-authorable content.** ADR-0129 §6's block-shape thesis applies to *what the AI produces and the user composes* — entity blocks, claim blocks, briefing blocks, agentic blocks. Folio bar / nav island / atmosphere are surface shell; users don't compose them per-page. The block-ification gain (Site-Editor authorability) is genuinely a non-goal for chrome.

3. **The deferral is bounded and reversible.** Packet §8 explicitly names "Chrome-as-Gutenberg-blocks" as v1.4.5+ scope. The runtime-inject path doesn't foreclose the block path — once a working substrate-on-chrome reference exists, the translation has a target.

### Does it set a problematic precedent?

**Bounded risk, not blocking.** The risk is "future contributors see chrome.js DOM-injection and think it's an acceptable pattern for substrate-rendering surfaces." Mitigations packet has:

- §10 new invariant explicitly names this as a **deliberate Tauri-WP parity choice**, not a general escape valve
- §6 "Substrate consumed (no rewrites)" lists `chrome.js` consumption as one-way-sync, one source of truth
- Plugin-owned trust/provenance rendering is untouched (architecture invariant line 76 of v1.4.3-waves.md)

Mitigations packet **lacks** (Condition 1, below).

**Architecturally:** chrome ≠ composition. Treating them differently is correct. The invariant boundary needs sharper enforcement language.

---

## 4. File ownership matrix — boundary check

§5.1–§5.7 file ownership against architecture invariants:

| Boundary | Packet position | Compliance |
|---|---|---|
| Theme owns layout / color / typography / motion tokens | `wp/dailyos/theme/assets/chrome/styles/*` — chrome modules + design-tokens.css + token-aliases.css | **COMPLIANT** — chrome modules contain zero trust/provenance selectors (verified: only `Pill.module.css` carries tone variants, and packet §10 correctly classifies them as paint not semantic) |
| Plugin owns trust / provenance / essential styling | `wp/dailyos/assets/dailyos-baseline-tokens.css` (91-line shim) — **unchanged** | **COMPLIANT** — packet §6 explicitly lists baseline-tokens.css as consumed-no-rewrites |
| Plugin block registration via `glob('blocks/*/block.json')` | `wp/dailyos/includes/class-dailyos-plugin.php` — **unchanged**; chrome adds no blocks | **COMPLIANT** |
| Stock TwentyTwentyFive fallback supported | "Under stock theme, chrome doesn't render but plugin's trust/provenance does. Intended degradation curve preserved." (§5.1) | **COMPLIANT** — degradation path is named; needs verification gate (Condition 2, below) |
| Canonical Tauri-side source `.docs/design/reference/_shared/` | Sync direction one-way OUT only | **COMPLIANT** — `.synced-from` artifact + sync-chrome.sh enforces |
| `wp/dailyos/theme/theme.json` auto-generated from `src/styles/design-tokens.css` | "remains canonical for WP preset palette + custom block tokens" (§5.1) | **COMPLIANT** — chrome aliases reference theme.json-emitted custom properties, don't override them |

### Pill duplication (§5.2 AC #6) — architectural flag

`Pill.module.css` (theme chrome) uses `.Pill_pill` / `.Pill_pillDot` (verified at source). `wp/dailyos/blocks/pill/style.css` uses `.dailyos-pill` / `.dailyos-pill__dot` (verified at target). **No selector overlap; no CSS-cascade conflict.**

However, the same conceptual primitive exists in two universes (chrome's Pill ≈ the block universe's Pill). Packet correctly flags this as "Architecture-reconciliation flag for future work" (§5.2) and defers to v1.4.5+. This is a **maintenance project item, not a packet blocker.** Filing this in Codebase Maintenance is appropriate.

### `parts/footer.html` rewrite (§5.7) — soft architectural question

Footer becomes substrate-mode (CPT archive links + tagline) as Gutenberg block markup, NOT chrome.js inject. This is the **correct** split: footer is content-shaped (links to archives that will multiply as v1.4.4 W2 CPTs register), header is chrome-shaped (single-shell shape across all pages). The packet does the right thing here, intuitively — but the *rule* isn't named anywhere.

**Suggested invariant addition** (not blocking): chrome ≡ "page-invariant shell elements" (folio + nav + atmosphere). Footer ≡ "content-shaped multi-link landing" lives as blocks. Edge cases (e.g., breadcrumbs) should reference the rule. (Condition 3, below.)

---

## 5. v1.4.3-waves.md line 76 reconciliation

**Line 76:** "DailyOS magazine theme owns no trust/provenance styling. Plugin owns the essential trust/provenance CSS; theme can override but is not a security/runtime dependency. Stock WP theme is a supported fallback."

**Packet §10 new invariant:** "Chrome is runtime-injected, not block-tree-rendered, as a deliberate Tauri-WP parity choice."

### Do they sit cleanly together?

**Yes.** They address orthogonal layers:

- Line 76 governs the **substrate-rendering surface** (trust/provenance is plugin-owned)
- Packet §10 governs the **shell surface** (chrome is theme-owned, runtime-injected)

The two coexist because:

1. Chrome modules in §5.2 contain zero trust/provenance selectors (verified by grep against `.docs/design/reference/_shared/styles/{FolioBar,FloatingNavIsland,AtmosphereLayer,MagazinePageLayout,Pill}.module.css` — only `ConfidenceScoreChip`, `ClaimRow`, `TrustBandBadge`, etc. carry trust styling, and **none of those are lifted**)
2. `dailyos-baseline-tokens.css` (plugin) remains the trust/provenance CSS authority; chrome doesn't touch it (§6)
3. Under stock TwentyTwentyFive: chrome doesn't render (no theme = no chrome.js enqueue), plugin's trust/provenance does render (block registration is plugin-owned). Degradation curve preserved.

**One enforceable gap:** §5.1 claims the degradation works but doesn't specify a verification gate. v1.4.3 W3 invariant line 76 includes "Suite S test: switch to TwentyTwentyFive; account-overview block still renders trust band + provenance label legibly." Packet §10 needs the equivalent **negative-confirmation** test: under TwentyTwentyFive, chrome.js does NOT load (no orphaned `<script>` tags from plugin paths), confirming chrome IS theme-bound. (Condition 2, below.)

---

## 6. Risk analysis

| Risk | Severity | Mitigation in packet | Residual |
|---|---|---|---|
| Future contributors generalize runtime-DOM-injection as an acceptable WP pattern | Medium | §10 invariant names it as deliberate parity choice | Needs strengthening — Condition 1 |
| Chrome doesn't render in Site Editor iframe | Low (named as non-goal) | §5.5 AC #19: editor styles load tokens/fonts for future block previews | Acceptable |
| Sync drift between canonical `_shared/` and theme `assets/chrome/` | Low | `.synced-from` artifact + one-way sync-chrome.sh + AC #13 re-sync idempotency | Acceptable — sync drift is detectable via `git rev-parse` comparison |
| Pill primitive lives in two universes | Low (no selector overlap) | Flagged for v1.4.5+ reconciliation | Acceptable — file as maintenance |
| Plugin enqueue precedence vs theme chrome enqueue | Low | §5.1 explicit chain: baseline (plugin, priority 9) → tokens → aliases → chrome modules | Acceptable |
| stock-theme fallback regression | Medium | §5.1 names degradation path | Needs verification gate — Condition 2 |
| `chrome_config()` CPT stubs for unregistered post types | Low | §5.5 graceful default + `post_type_exists()` guards | Acceptable |
| Font license compliance | Low | AC #10 LICENSES.md audit | Acceptable |
| Brand-mark Montserrat 800 weight as 5th font family | Low | ADR-0076 names it as the brand-mark font; lift is intentional | Acceptable |
| §5.7 footer-as-blocks-but-chrome-as-runtime split rule not codified | Low | Packet does the right thing intuitively | Codification recommended — Condition 3 |

---

## 7. Recommendations

**The three conditions are all docs/gates, not implementation rewrites. None block the lane's strategic shape; all can fold into cycle-1 or cycle-2 of L0 hardening without renumbering the wave.**

### Condition 1 — Sharpen the §10 invariant scope language

Current §10 reads "Chrome is runtime-injected, not block-tree-rendered, as a deliberate Tauri-WP parity choice." Strengthen to enumerate **what counts as chrome** vs. what doesn't, and what mandates a wave-plan amendment for future runtime-injection patterns:

> **Chrome is runtime-injected, not block-tree-rendered, as a deliberate Tauri-WP parity choice.** "Chrome" scope: page-invariant shell elements (FolioBar, FloatingNavIsland, AtmosphereLayer, MagazinePageLayout container). Content-shaped or per-page-variable surfaces (footers with CPT archive links, breadcrumbs, entity-page bodies) ship as Gutenberg blocks. Future runtime-injection patterns outside chrome scope require an L0 packet documenting the parity-vs-authorability trade resolution and a wave-plan amendment.

### Condition 2 — Add a stock-theme negative-confirmation gate

Add an AC to §5.5 (functions.php) or new §5.8:

> **AC #23** — Under TwentyTwentyFive (or any non-DailyOS-Magazine theme), `chrome.js` does NOT load. Verification: switch theme, inspect page, assert zero `<script>` tags reference `wp/dailyos/theme/assets/chrome/`. Confirms chrome is theme-bound, not plugin-bound; pairs with v1.4.3 W3 line-76 Suite S test that confirms plugin trust/provenance still renders.

### Condition 3 — Codify the chrome-vs-blocks decision rule

Add a paragraph to §1 or §10 that names the rule the packet implicitly follows:

> **Chrome vs. block decision rule.** A surface element ships as runtime-injected chrome iff (a) it's page-invariant across all CPTs and routes, (b) Tauri-WP parity is load-bearing, (c) end-user per-page authoring is a non-goal. Footer (CPT archive links varying as CPTs register), entity-page bodies (claim-shaped, composition-bearing), and breadcrumb composition all fail the test and ship as blocks. This rule is what makes the §10 invariant non-leaky.

### Non-blocking suggestions (cycle-2 or maintenance, not L0 conditions)

- **Pill duplication** (chrome `.Pill_*` vs plugin `.dailyos-pill__*`) — file as Codebase Maintenance ticket against project id `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` per `project_l2_path_alpha_to_maintenance_project` policy. Reconciliation lane in v1.4.5+ as packet §8 already names.
- **Product vocabulary discipline (ADR-0083)** — chrome strings ("Today", "Accounts", "DailyOS" brand label, "Memory + judgment for the alone part of knowledge work" tagline) should pass ADR-0083 review. Flag for L4 hands-on; not an L0 blocker.
- **`MePage.module.css`** appeared in the canonical chrome module dir grep but is NOT in the packet's §5.2 lift list. Confirm intentional (MePage is a content surface, not chrome) — if so, no action; if oversight, add to lift or document the exclusion.

---

## 8. Architecture summary

The packet is **strategically sound and tactically tight**. The lift is well-scoped (5 tiers in dep order, with tier 6/7 explicitly deferred), the trade-off is named not hidden, ADR consumption is correct and complete, file ownership respects the established theme/plugin split, and the K-in audit confirms no reinvention of documented substrate.

The three conditions sharpen the invariant boundary so future contributors don't generalize the runtime-inject path beyond chrome, and add a negative-confirmation gate that mirrors v1.4.3 W3's stock-theme fallback discipline. They are doc/AC additions, not implementation rewrites.

**Verdict: CONDITIONAL APPROVE.** Cycle-2 folds the three conditions; cycle-2 then unanimous-APPROVE-able from this reviewer.

---

## File paths referenced

- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` — packet under review
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/decisions/0073-editorial-design-language.md`
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/decisions/0076-brand-identity.md`
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/decisions/0077-magazine-layout-editorial-redesign.md`
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md`
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/decisions/0130-surface-independent-composition-contract.md`
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.3-waves.md` (line 76 invariant — verified in context)
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/design/reference/_shared/styles/{FolioBar,FloatingNavIsland,AtmosphereLayer,MagazinePageLayout,Pill}.module.css` — chrome modules canonical source
- `/Users/jamesgiroux/Documents/dailyos-repo/wp/dailyos/blocks/pill/` — Pill block plugin universe (verified separate from chrome Pill)
- `/Users/jamesgiroux/Documents/dailyos-repo/wp/dailyos/theme/` — lift target

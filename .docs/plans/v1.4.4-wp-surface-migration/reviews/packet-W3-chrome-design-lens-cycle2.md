# Design-Lens Review — L0 Packet W3 Chrome Lane (Pulled Forward)
**Reviewer:** `compound-engineering:ce-design-lens-reviewer`
**Cycle:** 2
**Date:** 2026-05-19
**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` (V1.1)
**Verdict:** APPROVE — all 6 cycle-1 gaps resolved. One observation (confidence 50, FYI-only, no AC required).

---

## K-in record (cycle-2 re-confirm)

**`docs/solutions/` grep:** Same files present as cycle-1. No new entries covering chrome lift, FolioBar, FloatingNavIsland, AtmosphereLayer, token-alias, or WP enqueue patterns. `docs/solutions/tooling-decisions/phpcs-warning-severity-zero-prevents-warning-only-ci-fails-2026-05-19.md` (lines 13-15, 32, 49-52) remains the only adjacent hit — WP PHPCS CI posture; pre-applied per §5.5. No reinvented documented substrate.

**`.docs/decisions/` grep:** ADR-0077 tint table re-confirmed against V1.1 §5.5 forward-stub table (see Gap A below). ADR-0073/0076/0077/0129/0130 consumed. No new ADRs hit that cycle-1 missed. ADR-0077 line 64 ("Entity accent borders: larkspur (people/1:1)") confirms `dailyos_person → larkspur` assignment. ADR-0077 lines 52-56 confirm briefing/meeting/account → turmeric; actions → terracotta; weekly → larkspur. No canonical reference for a `project` tint exists in ADR-0077 — TBD-at-W2-L0 call is correct.

**Verdict: K-in clean. No reinvented substrate.**

---

## Gap resolution verdicts

### Gap A — Tint forward-stub table

**Resolved.**

V1.1 §5.5 `chrome_config()` code block explicitly declares:

```php
$stub_tints = [
    'dailyos_briefing' => 'turmeric',  // per ADR-0077
    'dailyos_meeting'  => 'turmeric',  // per ADR-0077
    'dailyos_person'   => 'larkspur',  // per ADR-0077 entity accent for people
    'dailyos_project'  => 'turmeric',  // TBD at v1.4.4 W2 L0 — no canonical ADR-0077 tint
];
```

All four mappings are present, ADR-cited, and the project stub-to-turmeric pending W2 decision is correctly flagged. Confirmed against ADR-0077: briefing/meeting = turmeric (lines 52-54), person = larkspur (line 64 entity accent pattern). Correct.

**`post_type_exists` guard behavior at W2 land:** When W2 registers any of the 4 CPTs, `post_type_exists($cpt)` becomes true and the stub branch fires immediately — no packet amendment required. The tint values in the table become the live tints on first CPT post render. The `dailyos_project → turmeric` stub is the default until W2 L0 resolves the canonical project tint and updates the table. This is a one-line table edit in `chrome_config()`, not a conditional branch change — the guard pattern is clean for the W2 handoff.

**One minor gap in the stub tint table for `dailyos_project`:** The code comment says "TBD at v1.4.4 W2 L0" but the table's commentary in the changelog (§2) says it stubs to `turmeric`. The code value and the comment are consistent, but nowhere does the packet note where the W2 L0 decision should be recorded (e.g., in the AC list, a new AC, or explicitly in the W2 L0 packet scope). This is a confidence-50 FYI — the implementer will not be blocked since the guard pattern is self-contained and turmeric is the stated interim value. No AC required. Noted below as Observation 1.

---

### Gap B — Customizer preview guard

**Resolved.**

V1.1 AC #24 (§7, §5.5): "Chrome assets do NOT enqueue inside Customizer preview iframe (`! is_customize_preview()` check at top of `enqueue_assets()`). Verification: integration test invoking Customizer preview returns no chrome assets."

The enqueue chain in §5.5 shows the guard at line 1:

```php
if ( is_admin() || is_customize_preview() ) return;
```

Both `is_admin()` (covers block editor context) and `is_customize_preview()` (covers Customizer preview) are guarded. AC #24 is static-grep-verifiable (the check is present or absent) AND integration-test-verifiable. Fully resolved.

---

### Gap C — Scroll-spy deferral

**Resolved.**

V1.1 §5.4 explicit: "Canonical `chrome.js` does NOT implement `IntersectionObserver` scroll-spy for FloatingNavIsland chapter local-pill. The `.docs/design/patterns/FloatingNavIsland.md` spec at lines 53-55 promises it. This packet does NOT implement scroll-spy. Deferral: v1.4.5+ lane, OR a canonical-chrome.js patch on the Tauri side (re-synced by `sync-chrome.sh`)."

V1.1 §8 deferrals table also carries: "Scroll-spy in FloatingNavIsland chapter local-pill (V1.1, D-C deferral). Canonical `chrome.js` does NOT implement `IntersectionObserver`; `FloatingNavIsland.md` spec at lines 53-55 promises it. → v1.4.5+ lane OR canonical-chrome.js patch on Tauri side + re-sync."

The deferral is explicit in two places. An L4 tester who observes chapter active state not updating on scroll has a clear reference. Resolved.

---

### Gap D — Refresh button inline style

**Resolved.**

V1.1 Patch 9b (§5.4) replaces the inline `style=` with `class: F('folioRefreshButton')` and specifies the rule body in `FolioBar.module.css`:

```css
.FolioBar_folioRefreshButton {
  transition: color var(--transition-normal), border-color var(--transition-normal);
  cursor: pointer;
}
.FolioBar_folioRefreshButton:hover {
  color: var(--color-text-primary);
  border-color: var(--color-rule-heavy);
}
.FolioBar_folioRefreshButton:focus-visible {
  outline: 2px solid var(--color-spice-turmeric);
  outline-offset: 2px;
}
```

**Token verification (confirmed against canonical `design-tokens.css`):**
- `--transition-normal: 0.15s ease` — present, line 269.
- `--color-text-primary: var(--color-desk-charcoal)` — present, line 53. Resolves to the primary text color; correct for a button that changes text color on hover.
- `--color-rule-heavy: rgba(30, 37, 48, 0.12)` — present, line 60. Correct for border-color on a secondary UI element.
- `--color-spice-turmeric: #c9a227` — present, line 36. Solid color, appropriate for focus-visible outline (sufficient contrast against frosted glass background, consistent with the product's primary accent usage).

All three rules (hover, focus-visible, transition) are present. Pseudo-class targeting is now possible. The inline-style-can't-express-hover bug is fixed. The `cursor: pointer` addition is correct for a `<button>` in a context where the browser's UA reset may suppress it inside the frosted bar.

AC #4 correctly notes FolioBar.module.css carries the V1.1 addition (~290 LOC final). The canonical upstream source should be patched per Patch 9b's note ("canonical source should be patched upstream to match") — that's a future upstream promotion, not a W3 blocker.

**Resolved.**

---

### Gap E — folio-home-href resolution

**Resolved.**

V1.1 AC #25: "`folio-home-href` resolves as `get_permalink( get_option( 'page_on_front' ) ) ?: home_url( '/' )`. Verification: PHPUnit test with/without `page_on_front` configured asserts correct resolution."

The `chrome_config()` code block in §5.5 shows this as the first line of the function:

```php
$folio_home_href = get_permalink( get_option( 'page_on_front' ) ) ?: home_url( '/' );
```

`get_option('page_on_front')` returns the page ID of the configured static front page, or `0`/`false` if not configured. `get_permalink(0)` returns `false`, so the fallback `?: home_url('/')` fires correctly. PHPUnit test branch covers both cases. The `'#'` fallback from the pre-V1.1 packet is eliminated. Resolved.

---

### Gap F — Local pill hidden at W3

**Resolved.**

V1.1 §5.5 explicit: "`chrome_config()` for `dailyos_account` singular emits NO `chapters` key. `FloatingNavIsland` renders global-pill-only — local pill stays hidden via `navIslandLocalHidden` class. Local pill activates when v1.4.4 W2 entity surfaces start providing `chapters` inventory per `.docs/design/patterns/FloatingNavIsland.md` line 53-55."

AC #20 wording is corrected from "before first paint" to "before first user interaction" and confirms the behavior. An L4 tester seeing global-pill-only on account pages now has an explicit reference that this is the intended W3 state. Resolved.

---

## Additional cycle-2 scope

### New gaps introduced by V1.1

No new design-blocking gaps introduced. V1.1 adds 10 ACs (23-30 + 2 in §5.4 and §5.6) and 2 new invariants. All additions are narrowing (scope-locking) rather than scope-expanding. No new interactive element is introduced without a state specified.

The one structural addition with a mild concern is the DOM idempotency guard (Patch 9a, AC #27): the guard checks for class `.FolioBar_folio` as the sentinel. If the class name ever changes in a future canonical sync (which is unlikely since the class contract invariant now exists in §10), the guard becomes a no-op and double-injection re-emerges silently. This is a confidence-50 observation only — the §10 invariant "Downstream targets class contract, not DOM insertion order" locks the class names as a contract, so the risk is mitigated by the invariant itself.

### Patch 9 class rule — token appropriateness

Confirmed above under Gap D. All four tokens are in the canonical design-tokens.css. Semantic fit:

- `--transition-normal` (0.15s ease): appropriate for a button state change. The canonical chrome.js used `150ms` inline; `--transition-normal` is `0.15s ease` — equivalent duration, adds easing curve (improvement over the raw inline value which had no easing function).
- `--color-text-primary`: correct for active-state text color on a secondary action button within the frosted bar.
- `--color-rule-heavy`: a 12%-opacity near-black used for dividers. Using it for border-color on hover is tonally correct — slightly strengthens the border without introducing a new color. Consistent with how other secondary-rule surfaces behave in FolioBar.
- `--color-spice-turmeric` for focus-visible outline: the primary accent. This is the canonical focus-ring color across FolioBar interactive elements (consistent with how `.FolioBar_folioBreadcrumbButton:focus-visible` and related rules use it per the pattern spec). Correct.

No token-semantic issues. The rule body is production-appropriate.

### Stub tint table — `post_type_exists` guard and W2 land

The guard `post_type_exists( $cpt ) && ( is_singular( $cpt ) || is_post_type_archive( $cpt ) )` is a clean passive stub. When W2 registers a CPT, the branch activates automatically on the next request — no `chrome_config()` structural change required. The W2 L0 packet will need to update the tint value for `dailyos_project` (from turmeric to whatever is decided canonically), but that is a one-line edit well within W2 scope. There is no risk of the stub "going live incorrectly" — the `post_type_exists` guard guarantees silence until the CPT is registered, and the turmeric default for project is an acceptable interim visual state.

One observation: AC #17 says "each of 4 stub CPTs (only fires when `post_type_exists` true)" as a PHPUnit verification, but if the CPTs don't exist in the test environment, this branch cannot be tested without a fixture that manually registers the CPT. The packet doesn't specify whether the PHPUnit test fixture includes a `register_post_type('dailyos_project', ...)` stub for coverage. This is a confidence-50 FYI — a competent implementer will add the fixture; the AC wording is sufficient to imply it. Not a blocking gap.

---

## Observations (confidence 50 — FYI only, no AC required)

**Observation 1 — `dailyos_project` tint resolution ownership.** The W2 L0 packet inherits the obligation to resolve the project tint and update `chrome_config()`'s stub table. The current packet notes "TBD at v1.4.4 W2 L0" in the code comment but not in the AC list or in a formal handoff note to W2 scope. The W2 L0 author should know to check this comment. No blocking gap — the interim turmeric value is visually safe — but a one-line note in §8 deferrals ("W2 L0 must resolve `dailyos_project` tint and update `chrome_config()` stub table") would eliminate any chance of the resolution being forgotten in the W2 scope definition.

**Observation 2 — PHPUnit stub CPT fixture coverage.** AC #17's PHPUnit branch test for the 4 stub CPTs requires a fixture that registers those CPTs in the test environment. Not spelled out. A competent implementer will add this, but making it explicit ("Verification: PHPUnit test with fixture that calls `register_post_type('dailyos_project', [])` before asserting the stub branch fires") would make the AC unambiguous at L1.

---

## Dimensional rating (dimensions with relevant coverage)

**Interaction state coverage: 9/10.** All cycle-1 gaps resolved. Every interactive element in the chrome (FolioBar action buttons, FloatingNavIsland pills, breadcrumb buttons) now has class-based hover/focus-visible rules specified or inherited from verbatim-lifted module CSS. Scroll-spy deferred explicitly. The 1-point gap: AC #27's idempotency guard uses a class-name sentinel that is implicitly locked by the §10 invariant but not verified by a CI gate — a static grep asserting the sentinel class exists in `FolioBar.module.css` would close it.

**User flow completeness: 8/10.** Local-pill-hidden-at-W3 is now explicit. Customizer guard is specified. folio-home-href resolution is specified with both branches. The remaining 2 points: `dailyos_project` tint resolution path to W2 is implicit (Observation 1); PHPUnit stub CPT fixture is implicit (Observation 2).

**Unresolved design decisions: 9/10.** All cycle-1 "TBD" markers are resolved or explicitly deferred with a named future lane. The one remaining open item (`dailyos_project` tint) is correctly labeled TBD-at-W2 with an ADR-citation obligation. A 10 would have a Linear ticket filed to carry the W2 obligation.

---

## Verdict

**APPROVE.**

All 6 cycle-1 gaps (A through F) are resolved in V1.1. Token values in Patch 9b are confirmed correct against the canonical `design-tokens.css`. The `post_type_exists` guard pattern is clean for the W2 handoff. No new design-blocking gaps introduced by V1.1. Two confidence-50 observations logged; neither requires an AC before L1 begins.

This packet is clear for L1 implementation.

# Design-Lens Review — L0 Packet W3 Chrome Lane (Pulled Forward)
**Reviewer:** `compound-engineering:ce-design-lens-reviewer`
**Cycle:** 3
**Date:** 2026-05-19
**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` (V1.2)
**Verdict:** APPROVE — both cycle-2 advisory observations resolved. No new design-blocking gaps. No cycle-2 regression.

---

## K-in record (cycle-3 re-confirm)

**`docs/solutions/` grep:** Same 14 files as V1.2 §3 rescan. One new file present at scan time:
`docs/solutions/tooling-decisions/pre-push-hook-duration-vs-ssh-idle-timeout-2026-05-19.md` (in-flight per git status — not yet committed). No content relevant to chrome lift, FolioBar, focus-visible, token parity, or WP enqueue patterns. No documented prior solution reinvented.

**`.docs/decisions/` grep:** ADR-0073 and ADR-0076 re-read for focus-visible + outline-color guidance below. ADR-0077 unchanged. No new ADRs since cycle-2. K-in clean.

---

## Cycle-2 advisory resolution

### Advisory 1 — `dailyos_project` tint resolution ownership

**Resolved.**

V1.2 §8 deferrals table adds an explicit row:

> "V1.2 NEW: `dailyos_project` tint resolution (design-lens advisory 1) — TBD-at-L1-kickoff (filed against v1.4.4 W2 L0 author) — v1.4.4 W2 L0 must resolve via ADR-0077 amendment OR new ADR before CPT registration"

The row is in the formal deferrals table with a named recipient (W2 L0 author), a resolution mechanism (ADR-0077 amendment or new ADR), and a pre-condition gate (must resolve before CPT registration). The cycle-2 concern was that the obligation lived only in a code comment where a W2 L0 author might miss it. It now lives in the packet's structured §8 deferrals table — the canonical handoff surface. Ticket-ID placeholder follows the same "TBD-at-L1-kickoff" convention used consistently across §8. **Resolved.**

### Advisory 2 — PHPUnit stub CPT fixture coverage

**Resolved.**

V1.2 §5.5 AC #17 now reads (paraphrased): "Branch test for each of 4 stub CPTs requires `register_post_type()` fixture in test setUp() — without it `is_singular()` and `is_post_type_archive()` return false for unregistered CPTs. Fixture: `register_post_type('dailyos_briefing', ['public' => true, 'rewrite' => ['slug' => 'briefings']])`, etc."

The AC is now specific enough to implement without guessing. The cycle-2 concern was that a competent implementer would have to infer the fixture requirement; V1.2 makes it explicit with an example. **Resolved.**

---

## Focus areas (cycle-3 dispatch)

### 1. §5.4.1 canonical pre-lift upgrade — CSS parity against folio-refresh-button.tsx:38-52

Cross-checked V1.2 §5.4.1 `.FolioBar_folioRefreshButton` rule against the React component at `src/components/ui/folio-refresh-button.tsx:38-52`.

React component inline styles (lines 38-51):
- `fontFamily: "var(--font-mono)"` → CSS: `font-family: var(--font-mono)` — present
- `fontSize: 11` → CSS: `font-size: 11px` — present
- `fontWeight: 600` → CSS: `font-weight: 600` — present
- `letterSpacing: "0.06em"` → CSS: `letter-spacing: 0.06em` — present
- `textTransform: "uppercase"` → CSS: `text-transform: uppercase` — present
- `color: "var(--color-text-tertiary)"` → CSS: `color: var(--color-text-tertiary)` — present
- `background: "none"` → CSS: `background: none` — present
- `border: "1px solid var(--color-rule-heavy)"` → CSS: `border: 1px solid var(--color-rule-heavy)` — present
- `borderRadius: 4` → CSS: `border-radius: var(--radius-editorial-sm)` — **token substitution, not literal 4px**

  `--radius-editorial-sm` resolves to `4px` per `design-tokens.css` line 249. Semantically equivalent and strictly better — if `--radius-editorial-sm` ever changes, the canonical CSS tracks it automatically; the React component's hardcoded `4` would not. This is a net improvement, not a parity gap.

- `padding: "2px 10px"` → CSS: `padding: 2px 10px` — present
- `cursor: loading ? "default" : "pointer"` → CSS: `cursor: pointer` (idle); disabled rule adds `cursor: default` — coherent; covered separately below.
- `opacity: loading ? 0.6 : 1` → CSS: disabled rule `opacity: 0.6`; idle is browser default `1` — coherent; covered separately below.
- `transition: "color 150ms, border-color 150ms, opacity 150ms"` → CSS: `transition: color var(--transition-normal), border-color var(--transition-normal), opacity var(--transition-normal)` — present. `--transition-normal: 0.15s ease` per design-tokens.css line 269. Duration is identical (150ms = 0.15s); CSS adds an easing curve (improvement over bare `150ms`).

React `onMouseEnter` hover behavior (lines 53-57):
- `color: "var(--color-text-secondary)"` → CSS hover: `color: var(--color-text-secondary)` — present
- `borderColor: "var(--color-text-tertiary)"` → CSS hover: `border-color: var(--color-text-tertiary)` — present

`onMouseLeave` restores to idle values — handled automatically by CSS cascade; no `:mouseleave` state needed in CSS.

Additionally the CSS rule carries `-webkit-app-region: no-drag` which has no React equivalent (Tauri-specific drag region opt-out). This is a chrome.js rendering context detail, not a parity gap.

**CSS parity verdict: complete.** All 12 properties from the React inline style are represented. Two improvements over literal parity: token substitution for border-radius, token substitution for transition timing. No missing properties. No semantic drift.

---

### 2. `:focus-visible` — 2px turmeric outline-offset 2px appropriateness per ADR-0073

The rule:
```css
.FolioBar_folioRefreshButton:focus-visible {
  outline: 2px solid var(--color-spice-turmeric);
  outline-offset: 2px;
}
```

**ADR-0073 check:** ADR-0073 establishes turmeric/gold (`#c9a227`) as "Active/now state, customer meetings, primary accent" and confirms it is used on "4px accent bars, priority numbers, active sidebar items." The ADR names gold as the canonical accent signal. Using it for focus-visible is consistent with the ADR's accent-color semantic — focus-visible is the keyboard-active state, directly analogous to other active/now states the ADR assigns to gold.

ADR-0073 does not prescribe a specific focus ring width or offset. The 2px/2px combination is the CSS specification's suggested baseline for `outline`-based focus indicators and is the pattern used in the existing `FolioBar.module.css` rules for other interactive elements (confirmed by cycle-2 review citing `.FolioBar_folioBreadcrumbButton:focus-visible` as a consistent reference). There is no ADR-0073 or ADR-0076 deviation.

**ADR-0073 verdict: appropriate.** Turmeric is the correct accent color for this context. 2px outline / 2px offset is consistent with the existing FolioBar interactive-element pattern. The current React component has no `:focus-visible` rule at all (inline styles cannot express pseudo-class selectors) — this is an accessibility improvement with no design-language conflict.

---

### 3. Disabled state behavior — coherence

**V1.2 CSS rule:**
```css
.FolioBar_folioRefreshButton:disabled,
.FolioBar_folioRefreshButton[aria-busy="true"] {
  cursor: default;
  opacity: 0.6;
}
```

**React component (lines 36, 49-50):**
- `disabled={loading}` — sets the HTML `disabled` attribute when `loading` is true
- `cursor: loading ? "default" : "pointer"` — cursor matches disabled state
- `opacity: loading ? 0.6 : 1` — opacity matches disabled state

The React component does NOT set `aria-busy`. The CSS rule's `[aria-busy="true"]` selector therefore never fires from the current React render path — it has no effect on the Tauri production surface.

**Is this a problem?** No, for two reasons:

1. The `[aria-busy="true"]` selector is the chrome.js surface's equivalent of the React component's `disabled` attribute. When chrome.js renders the refresh button in the WP context, it does not have access to React's `disabled` prop — it manages state via DOM attributes. The `aria-busy` selector gives the WP-side a CSS hook for the loading state without requiring a DOM `disabled` attribute (which would block click events in ways that may not match the chrome.js orchestration intent). The Tauri-side and WP-side disabled representations differ by surface constraint, not by design intent.

2. The visual output is identical: both paths produce `cursor: default; opacity: 0.6` on the loading state. The user experience is coherent across surfaces.

**One observation (confidence 50, FYI):** The React component does not set `aria-busy="true"` during loading — it only sets `disabled`. This means keyboard users navigating the Tauri surface get the correct disabled-button semantics (browser suppresses focus and click), but screen readers receive no `aria-busy` signal during the in-progress state. This is a pre-existing accessibility gap in the React component, not introduced by V1.2. The canonical CSS rule correctly positions the WP surface to express both `disabled` and `aria-busy` states if chrome.js ever emits either. Not a packet blocker — the gap exists in `folio-refresh-button.tsx` and belongs to a maintenance ticket, not this L0.

**Disabled state coherence verdict: coherent.** React uses `disabled`, chrome.js uses `[aria-busy="true"]` (or `disabled` if applicable in WP context). Visual output is identical on both surfaces. V1.2 correctly covers both DOM-attribute patterns in the CSS rule.

---

### 4. Regression check against cycle-2 APPROVE

Reviewing V1.2 changes against cycle-2 APPROVE baseline:

- **Patch 9b removed from `patch-chrome-js.py`** — the CSS override WP-overlay is gone. Cycle-2's APPROVE cited no concern with the Patch 9b approach (Gap D rated Resolved). V1.2 replaces the WP-overlay with upstream-to-canonical — strictly better from a sync-model perspective. Not a regression.

- **§5.4.1 canonical pre-lift upgrade added** — new scope that wasn't in V1.1. This is the mechanism that makes the WP-overlay unnecessary. The AC #31 verification (separate PR lands before sync) ensures sequencing is enforced. Not a regression.

- **AC #30 regex fix** — tightened from `var\(--[a-z-]+\)` to `var\(--[A-Za-z0-9_-]+(?:,[^)]+)?\)`. This closes a real gap the cycle-2 challenge surfaced. Makes the AC more correct, not less. Not a regression.

- **§5.5 prohibition list expanded** — `register_sidebar()`, `register_nav_menus()`, classic theme-support calls added to the "MUST NOT" list. Narrowing, not expanding scope. Not a regression.

- **§13 reset path corrected** — `wp template-part delete` replaced with validated `wp post list | xargs` WP-CLI path. Operational correctness improvement. Not a regression.

**No cycle-2 regression found.** All V1.2 changes are either fold-of-challenge-blocker or fold-of-advisory-observation. The dimensional ratings from cycle-2 (Interaction state coverage 9/10, User flow completeness 8/10, Unresolved design decisions 9/10) hold or improve with the two advisories now resolved.

---

## Dimensional rating (cycle-3 delta only)

**Unresolved design decisions: 10/10 (was 9/10).** The one open item — `dailyos_project` tint — is now in the formal §8 deferrals table with a named recipient, resolution mechanism, and pre-condition gate. Nothing is TBD-without-an-owner. A 10 means every interaction is specific enough to implement without asking "how should this work?" — satisfied.

**User flow completeness: 9/10 (was 8/10).** PHPUnit fixture coverage is now explicit in AC #17. The `dailyos_project` tint handoff path to W2 is now in §8. The 1-point gap: the `aria-busy` accessibility gap noted above is pre-existing in the React component and out of scope for this packet — not counted against the packet's user flow completeness.

---

## Verdict

**APPROVE.**

Both cycle-2 advisory observations are resolved in V1.2. §5.4.1 CSS parity is complete against `folio-refresh-button.tsx:38-52` with two token-substitution improvements (border-radius, transition). The `:focus-visible` 2px turmeric outline is consistent with ADR-0073 accent semantics and the existing FolioBar interactive-element pattern. Disabled state behavior is coherent across React and chrome.js surfaces. No cycle-2 regression. The `[aria-busy="true"]` selector gap in the React component is pre-existing and belongs to maintenance, not this packet.

This packet is clear for L1 implementation.

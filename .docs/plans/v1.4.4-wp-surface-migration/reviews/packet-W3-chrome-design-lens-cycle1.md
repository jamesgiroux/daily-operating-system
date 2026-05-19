# Design-Lens Review — L0 Packet W3 Chrome Lane (Pulled Forward)
**Reviewer:** `compound-engineering:ce-design-lens-reviewer`
**Cycle:** 1
**Date:** 2026-05-19
**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md`
**Verdict:** CONDITIONAL APPROVE — 4 gaps must land in AC before L1 begins. None are architectural blockers; all are implementation details the packet leaves ambiguous that will cause implementer divergence or L4 failure.

---

## K-in record

**`docs/solutions/` grep:** Zero hits on `chrome`, `foliobar`, `navisland`, `atmosphere`, `token-alias`, `design-system`, `magazine`. Confirms packet's own K-in finding — this is net-new WP chrome territory. No reinvented documented substrate.

**`.docs/decisions/` hits consumed by this review:**
- ADR-0077 (lines 47-56) — canonical tint-to-surface mapping. Used in §1 gap below.
- ADR-0073 — editorial token set. No gaps found against packet scope.
- ADR-0076 — palette. No gaps found.
- ADR-0129, ADR-0130 — surface/composition contract. No conflicts with chrome runtime-inject model.
- ADR-0008 — profile-aware navigation. Entity-mode `account` default in `buildNav()` consistent with current CS-first customer-zero configuration.
- ADR-0054 — list page signal-first pattern. Not consumed by chrome layer; no conflict.

No reinvented documented substrate found.

---

## §1 — Information Architecture: `chrome_config()` surface-to-chrome mapping

**Rating: Interaction state coverage — 6/10 — it's a 6 because the tint assignments for 4 of 5 substrate surfaces are declared as stubs that all emit `turmeric`, overriding the canonical ADR-0077 surface tint map. A 10 would name the correct tint per surface so the stub values are correct defaults, not placeholders that will need a second pass.**

**Gap — confidence 75.**

ADR-0077 defines the canonical tint-to-surface mapping:
- Daily Briefing: `turmeric` (primary, plus `larkspur` secondary on atmosphere)
- Account Detail: `turmeric`
- Meeting Intelligence: `turmeric`
- Actions: `terracotta`
- Weekly Forecast: `larkspur`

The packet's `chrome_config()` re-targeting (§5.5) assigns all CPT stubs `tint: turmeric` pending W2 CPT registrations. `turmeric` is correct for `dailyos_account` (the only live CPT today). But the packet provides no tint mapping table for the stubs, so when W2 lands `dailyos_briefing` / `dailyos_project` / `dailyos_person` / `dailyos_meeting`, the implementer has no in-packet reference for what tint each should receive. ADR-0077 does not enumerate `project` or `person` tints (it predates those surfaces as distinct CPTs); the implementer will need to extrapolate or guess.

**Specific gaps:**
- `dailyos_briefing` → tint not specified in packet. ADR-0077 says briefing = `turmeric`. Probably `turmeric`. Not stated.
- `dailyos_project` → no canonical reference. ADR-0077 doesn't name a project tint. No spec.
- `dailyos_person` → ADR-0077 entity accent borders use `larkspur` for people/1:1. Likely `larkspur` for person surfaces. Not stated.
- `dailyos_meeting` → ADR-0077 meeting intel = `turmeric`. Probably `turmeric`. Not stated.

The implementer for W2 CPT registrations will look at this packet for the tint values. The packet defers to "stubs that emit turmeric" with no forward table. W2 implementers will diverge.

**Required fix:** Add a forward-stub tint table in §5.5 (even if marked "TBD at W2"):
```
dailyos_briefing  → turmeric  (per ADR-0077)
dailyos_project   → TBD — no canonical reference; W2 L0 must decide
dailyos_person    → larkspur  (per ADR-0077 entity accent pattern for people surfaces)
dailyos_meeting   → turmeric  (per ADR-0077 meeting intel page)
```
Without this, W2 implementers will either guess or default all to `turmeric`, producing visual ambiguity across surfaces on a chrome layer that is supposed to carry contextual tint semantics.

---

## §2 — Interaction State Coverage

**Rating: 6/10 — it's a 6 because the AC list enumerates static/structural checks well but omits 4 runtime interaction states that are load-bearing for L4 hands-on. A 10 would have every state specified with the expected behavior.**

### Gap 2a — Customizer preview / editor iframe chrome.js guard — confidence 100.

The packet states at §5.5 AC #19: "chrome itself does NOT render in editor iframe — runtime injection requires frontend DOM." This is correct. But there is no AC that enforces a guard in `functions.php` to prevent `chrome.js` from enqueueing inside `is_customize_preview()` or inside the block editor iframe (`is_admin() && is_block_editor()`).

Evidence from Studio source `functions.php` (grep of enqueue chain): `add_action('wp_enqueue_scripts', ...)` is the only enqueue hook present. `wp_enqueue_scripts` does not fire inside the Site Editor canvas iframe (which uses `block_editor_settings`) or inside Customizer preview (which does fire `wp_enqueue_scripts` but with `is_customize_preview()` === true).

The risk: if chrome.js fires inside a Customizer preview, the injected FolioBar + NavIsland will overlay the Customizer's own preview chrome, producing a broken preview UX at L4 dogfood. The canonical mockup-substrate chrome.js has a `data-chrome="off"` guard but no Customizer detection.

**Required fix:** Add AC: `functions.php` wraps the `chrome.js` enqueue (and `wp_add_inline_script`) in a `! is_customize_preview()` guard. Static-grep verifiable.

### Gap 2b — Scroll-spy chapter tracking in FloatingNavIsland — confidence 75.

The canonical `chrome.js` (line 305–314, 354–367) parses `body.dataset.chapters` and renders the local pill with hash-anchor `<a href="#chapter-id">` links. It does NOT wire scroll-spy — there is no `IntersectionObserver` in the canonical `chrome.js`. The FloatingNavIsland pattern spec (`FloatingNavIsland.md` lines 53–55) says: "Click scrolls to the section (smooth-scroll); active chapter gets surface-tint highlight via scroll-spy."

Scroll-spy is NOT in the canonical `chrome.js` being lifted. In the Tauri React component (`FloatingNavIsland.tsx`), the `activeChapterId` prop + `onChapterClick` callback drive this. The canonical mockup-substrate chrome.js simply sets `activeChapter` from `body.dataset.activeChapter` at render time — a static assignment, not runtime scroll tracking.

On the WP surface: for any `dailyos_account` (or future CPT) that declares `chapters`, the local pill will render correctly but chapter active state will never update as the user scrolls. The first chapter is always highlighted. This is a known Tauri-WP gap that the packet does not acknowledge.

The packet's AC #20 ("chrome.js DOM-mutates body to inject FolioBar before first paint") doesn't cover this. AC #18 covers `window.dailyosChrome` JSON shape including `folio-crumbs` but not chapter scroll behavior.

**Required fix:** One of: (a) Add explicit AC acknowledging that scroll-spy is not implemented in this lift, with a deferral note; or (b) scope a minimal IntersectionObserver into the WP patch script (Patch 9). The gap being unacknowledged is what creates implementer ambiguity — an implementer doing L1 who finds scroll-spy missing will not know whether to implement it or defer.

**Confidence 75:** A competent implementer might infer "mockup-substrate doesn't scroll-spy, so WP doesn't either" from context. But the pattern spec says it should, and there's no explicit deferral in the packet.

### Gap 2c — AtmosphereLayer tint transition on navigation — confidence 50 (FYI).

The canonical `AtmosphereLayer.module.css` includes `atmosphere-breathe` keyframe (per §5.1 design-tokens.css description — "atmosphere-breathe keyframe"). The tint CSS class is set by `chrome.js` at inject time from `body.dataset.tint` (chrome.js lines 382–384). On the WP surface, navigation between CPT pages is full-page reload (not SPA routing), so tint transitions across surfaces are always full-page refresh. This is fine for the current scope and is not a gap — just noting that the breathing animation + tint context are page-load semantics, not transition semantics, on WP. No AC needed.

### Gap 2d — FolioBar action set hover/focus states — confidence 75.

The canonical `chrome.js` (line 202) injects the `refresh` button with an inline `style=` attribute containing hover-state colors (`color`, `border-color` via `transition: color 150ms, border-color 150ms`). This is inline CSS on a DOM-injected element. On the WP surface:

1. Hover state: the inline `style` attribute has no `:hover` pseudo-class — inline styles can't target pseudo-classes. The hover effect will be absent on the refresh button in the WP render. The Tauri React component (`FolioBar.tsx`) achieves this via CSS Module class; the mockup-substrate chrome.js cheats with inline `style` for the mockup use case.
2. Focus-visible: the packet's architecture review should catch this, but the design-lens flag is: after lift, keyboard focus on folio action buttons will produce no visible indicator unless `FolioBar.module.css` has a `:focus-visible` rule that applies to the class-named button. If the module CSS targets the class selector and the lifted CSS is verbatim from canonical, this should be covered — but only for buttons that have the module class applied. The `refresh` button is appended via `actWrap.append(...)` without a module class name (`F(...)`) — it has only inline `style`.

**Required AC:** Add verification that the refresh button rendered via `chrome.js` has a CSS Module class (not just inline style) so hover and focus-visible states work correctly in the WP DOM context. This is a direct-lift wrongness (see §4 AI slop / wrong-shape WP markup).

---

## §3 — User Flow Completeness

**Rating: 7/10 — it's a 7 because the packet's dogfood flow (front page → account archive → account singular) is minimally covered by AC #17/#20/#22, but 3 edge cases that will surface at L4 are not addressed. A 10 would have a flow description covering happy path + 2-3 edge cases.**

### Gap 3a — `folio-home-href` resolution for the brand mark home link — confidence 100.

Patch 8 in `patch-chrome-js.py` resolves `folioHomeHref` from `body.dataset.folioHomeHref || '#'`. The packet's `chrome_config()` must emit `folio-home-href` as a key in `window.dailyosChrome`. The packet's AC #18 lists the keys chrome.js consumes: `active-page`, `tint`, `folio-label`, `folio-crumbs`, `folio-date`, `nav-home-{id,label,href}`, `folio-home-href`, `nav-items-json`. `folio-home-href` is listed — good.

But: the expected value for `folio-home-href` on the substrate-rendering WP theme is not defined. On the marketing Studio theme, it's `home_url('/')`. On the substrate theme, the "home" destination should be... what? The front page? A Briefing CPT archive (which may not exist in v1.4.4 W3)? The WP admin dashboard?

The packet says "stubs that emit turmeric + label from post_type_object->labels->name" for unmapped CPTs but never specifies the home-href resolution rule for the substrate context. If the front page is a static page (WP `page_on_front`), `home_url('/')` is correct. But if the front page is not configured, it resolves to the blog index — wrong for a substrate-rendering theme that may have no blog posts.

**Required fix:** Add explicit AC: `chrome_config()` emits `folio-home-href` as `home_url('/')` (or the configured front-page URL via `get_permalink(get_option('page_on_front'))`). Document the rule so L1 implementer doesn't guess.

### Gap 3b — Breadcrumb truncation behavior — confidence 50 (FYI).

The canonical `chrome.js` renders each breadcrumb crumb as a `<button>` (non-last) or `<span>` (last). On a long account title (`folio-crumbs: Accounts / Very Long Account Name Inc.`), there is no truncation rule in `FolioBar.module.css` (inferred from pattern spec; not directly checked). The WP surface can produce long account names from `get_the_title()`. At L4 dogfood on a narrow viewport or long title, the folio-left region may overflow into the center or right regions.

This is a deferrable cosmetic concern, not a blocking gap. Noting it so L4 tester is primed. No AC required at L0.

### Gap 3c — Chapter-based local pill on `dailyos_account` — confidence 75.

The `FloatingNavIsland.md` pattern spec says AccountDetail provides chapters per the active view (Health / Context / Work). The packet's `chrome_config()` for `singular(dailyos_account)` emits no `chapters` key in `window.dailyosChrome` (§5.5 shows only `tint`, `folio-label`, `folio-crumbs`, `nav-active`). Without a `chapters` value, `body.dataset.chapters` will be empty and the local pill will render hidden (`navIslandLocalHidden`). This is correct behavior for W3 — the account detail blocks that define chapters are W2 work.

However, the packet doesn't explicitly state this: "chapters will be empty in W3; the local pill will be hidden; W2 account detail blocks will populate chapters via a future `chrome_config()` or a separate enqueue." This omission means the L4 tester at W3 will see a global-pill-only nav on account pages and won't know if that's intended.

**Required fix (light):** Add a note in §5.5 or AC #20: "Local pill is intentionally hidden at W3 (no chapters declared). W2 account detail blocks are expected to populate `chapters` via a future `chrome_config()` extension or a block-registered script."

---

## §4 — AI Slop Risk in the Direct Lift

**No generic-SaaS-pattern slop.** The chrome modules being lifted are the canonical DailyOS design language — editorially differentiated by design. The risk here is wrong-shape WP markup, not aesthetic genericness.

**One concrete wrong-shape issue — confidence 100:**

In `chrome.js` `buildFolio()`, the `refresh` action button is rendered with an inline `style` attribute (canonical line 202):

```js
actWrap.append(el('button', {
  type: 'button',
  title: refreshTitle,
  style: "font-family:var(--font-mono); font-size:11px; ...; transition: color 150ms, border-color 150ms;",
}, 'Refresh'));
```

Inline `style` cannot express `:hover` pseudo-class states. In the Tauri/mockup substrate context this is a mockup convenience — reference HTML doesn't need real hover states. But in the WP surface context, this is the production rendering path. The refresh button will lack hover and focus-visible styling, and `transition:` on an inline style is a no-op for pseudo-class changes.

The Pill.module.css verbatim lift is clean (class-based, no inline style). The FolioBar.module.css covers `.FolioBar_folioBreadcrumbButton` hover state (inferred from pattern spec). The gap is specifically the `refresh` action button's inline-style-only definition.

The 8 WP patches in `patch-chrome-js.py` do not address this. Patch 8 covers `folioHomeHref`; no patch targets the refresh button's inline style to add a module class.

**Required fix:** Add Patch 9 (or extend Patch 4/8 scope) to replace the refresh button's inline style string with a CSS Module class (`F('folioRefreshButton')`) that is defined in `FolioBar.module.css`. Alternatively, add an AC that audits all `style=` attributes in the patched `chrome.js` and asserts each either has a corresponding module-class `:hover`/`:focus-visible` rule or is explicitly documented as mockup-substrate-only behavior that is acceptable in the WP production path.

---

## §5 — Dimensional Rating

**Information density: 8/10.** FolioBar composition (left/center/right regions), breadcrumb parsing, readiness stats, action set are all specified at the token level via ADR-0073/0076/0077. The center `folio-date` slot usage is called out in `chrome_config()` (the Studio source uses it for editorial dates; the substrate theme uses it for `post_type_object` labels). Minor: no guidance on what the center slot emits for `dailyos_account` singular — the Studio source emits "A WORKING SYSTEM" etc. The substrate version's center content is unspecified. Not a blocking gap but implementers will diverge.

**Interaction quality: 5/10** (see §2 gaps — hover, scroll-spy, Customizer guard are all unspecified). A 10 would enumerate every interactive element's states.

**Visual hierarchy: 9/10.** Tint-as-surface-context, folio-bar three-region layout, atmosphere gradient + watermark, local vs global pill — all clearly mapped to token names. Gap is the tint table for 4 non-account CPTs (§1).

**Motion/transition behavior: 7/10.** `atmosphere-breathe` keyframe is noted. `--transition-fast` used for hover/active in FloatingNavIsland. FolioBar action button `transition` is inline-style-only (§4). No guidance on whether AtmosphereLayer should animate on first load vs. delay. The canonical CSS likely handles this via animation-delay but it's not mentioned. Not blocking.

**Editorial voice: 9/10.** `folio-date` center text is editorial copy per ADR-0077 pattern ("MEMORY + JUDGEMENT", "A WORKING SYSTEM", etc.). Packet states the front page gets `folio-date: $(date 'F j, Y')` which is a PHP date format, not editorial copy. The Studio source uses hardcoded editorial strings here. The substrate source using a date format is intentional (substrate is data-driven, not marketing-copy-driven), but this is a deliberate departure that should be noted as such. Not a gap.

---

## Summary of required AC additions

| # | Gap | Location | Confidence |
|---|-----|----------|-----------|
| **A** | Forward tint table for 4 non-account CPT stubs | §5.5 | 75 |
| **B** | `! is_customize_preview()` guard on chrome.js enqueue | New AC in §5.5 | 100 |
| **C** | Scroll-spy deferral acknowledged (or Patch 9 added) | New AC in §5.4 or §8 | 75 |
| **D** | Refresh button inline style → module class (Patch 9) | §5.4 + AC in §5.2 | 100 |
| **E** | `folio-home-href` resolution rule stated explicitly | AC #18 extension | 100 |
| **F** | Local pill hidden-at-W3 behavior noted explicitly | §5.5 or AC #20 | 75 |

Gaps B, D, E are load-bearing for L4 hands-on (Customizer will be broken; home link will be `#` without the rule; refresh button hover will be absent). The others are implementer-divergence risks for W2 handoff.

**Verdict: CONDITIONAL APPROVE.** Address gaps B, D, E as new ACs before L1. Gaps A, C, F can land as notes/deferrals in the packet prose. No architectural rewrite required.

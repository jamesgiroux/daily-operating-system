# L0 Architecture Review — Packet W3 Chrome Lane (Pulled Forward) — Cycle 2

**Reviewer:** `compound-engineering:ce-architecture-strategist`
**Mode:** Pattern compliance + design integrity (architect-reviewer per L0 matrix)
**Date:** 2026-05-19
**Packet under review:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` V1.1
**Cycle-1 verdict reviewed:** `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md`

## Verdict: **APPROVE**

All 3 cycle-1 conditions resolved cleanly. V1.1 fold introduces no new architectural debt; the §10 W4-coupling matrix, the class-contract invariant, and Patch 9 design all hold under stress-test. K-in citations correct.

---

## 1. Cycle-1 condition resolution

### Condition 1 — Sharpen §10 invariant scope. **RESOLVED.**

- Packet `L0-packet-W3-chrome-lane-pulled-forward.md:446` ("Chrome runtime-injection scope") enumerates the exact 4 modules (`FolioBar`, `FloatingNavIsland`, `AtmosphereLayer`, `MagazinePageLayout`) and explicitly excludes footer, breadcrumbs, entity bodies, claim cards, trust/provenance UI, action affordances, per-page authorable regions.
- Wave-plan amendment requirement for future patterns outside the set is stated verbatim: "Future runtime-injection patterns outside this 4-module set REQUIRE a wave-plan amendment + new L0 packet documenting the parity-vs-authorability trade."
- §1 strategic framing (lines 34-49) carries the same list with consistent wording — no drift between §1 and §10.
- Matches the strengthening I proposed in cycle-1 §7 Condition 1 verbatim in scope (the language is tighter than my suggestion).

### Condition 2 — Stock-theme negative-confirmation gate. **RESOLVED.**

- AC #23 at `L0-packet-W3-chrome-lane-pulled-forward.md:356` mirrors v1.4.3 W3 Suite S precedent: under TwentyTwentyFive/any non-DailyOS theme, both `<script>` AND `<link>` assertions cover zero chrome-asset references in output.
- AC #23 covers both stylesheets and scripts (cycle-1 ask was script-only; V1.1 strengthens to both, correctly). Pairs with line-76 plugin-side test that confirms trust/provenance still renders.
- Verification method is PHPUnit/integration, not L4-hands-on-only — automatable.

### Condition 3 — Codify chrome-vs-blocks decision rule. **RESOLVED.**

- §1 lines 34-49 enumerate the 4 runtime-injected modules and the explicit "stays Gutenberg" list.
- §10 invariant "Chrome runtime-injection scope" carries the rule into the invariant set (binding, not just framing).
- Decision rule rationale (lines 43-47) addresses the 3 criteria I named in cycle-1 §7 Condition 3 (page-invariant, Tauri-WP parity load-bearing, end-user per-page authoring non-goal).
- §5.7 footer-as-blocks-not-chrome-inject is cross-referenced from §10 invariant explicitly (line 446: "Footer ... STAYS as Gutenberg blocks").

---

## 2. Cycle-2 additional scope — stress test

### §10 W4-coupling matrix (C1 fold) — DISJOINT WRITE SETS CONFIRMED

- W4 paths per `L0-packet-W3-chrome-lane-pulled-forward.md:451`: `src-tauri/src/services/surface_nonce.rs`, `src-tauri/src/bridges/surface_client.rs`, `src-tauri/src/commands/surface_runtime.rs`, `wp/dailyos/includes/class-dailyos-plugin.php`, `wp/dailyos/includes/transport/class-dailyos-runtime-client.php`, `wp/dailyos/blocks/account-overview/*`.
- Chrome paths per `L0-packet-W3-chrome-lane-pulled-forward.md:452`: `wp/dailyos/theme/**` only.
- **Verified disjoint.** No path overlap. W4 touches `wp/dailyos/includes/` + `wp/dailyos/blocks/account-overview/` + `src-tauri/`; chrome touches `wp/dailyos/theme/` exclusively. The "forbidden" clause at line 453 is the enforcement (no W4 PRs into theme/, no chrome PRs into runtime/plugin/blocks except non-regression tests).
- **No hidden coupling surface** — `wp/dailyos/theme/theme.json` (auto-generated from `src/styles/design-tokens.css`) is listed in §6 as substrate-consumed-not-edited; if v1.4.3 W4 needs to mutate theme.json, the disjoint rule breaks. Greped `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:14-20` — revised W4 explicitly extends `surface_nonce.rs` (no theme.json touch). Disjointness holds.
- The cross-reference to the K-in solution file at line 451 strengthens the assertion — W4 V1.0 was reinventing; V1.1 shrinks away from new files entirely.

### §10 "Downstream targets class contract, not DOM order" — COHERENT WITH chrome.js MODEL

- Verified canonical `chrome.js:465-488` `inject()` function: prepends `buildAtmosphere`, prepends `buildFolio`, appends `buildNav`. DOM insertion order is `[Folio, Atmosphere, ...body content..., Nav]` (prepends apply in reverse).
- All 3 chrome elements carry class contracts (`.FolioBar_folio`, `.AtmosphereLayer_atmosphere`, `.FloatingNavIsland_navIslandContainer` — verified at `chrome.js:134, 373, 377` and `styles/FloatingNavIsland.module.css:13`).
- Position-fixed semantics in FolioBar + FloatingNavIsland CSS modules mean DOM order is **deliberately decoupled** from rendered position. Stacking context relies on z-index tokens, not sibling order.
- **The invariant is sound and load-bearing.** Tests that target sibling positions or `body.children[0]` would lock in the runtime-inject implementation; tests targeting class selectors translate cleanly to a future block-ified rewrite. The §8 cost analysis (700-1100 LOC at 2x if downstream targets DOM order) is the right pressure to codify the rule now.

### Patch 9 design — SOUND; NO CANONICAL-VS-OVERLAY MAINTENANCE DEBT

**Patch 9a (DOM idempotency guard):**
- Proposed guard `if (body.querySelector('.FolioBar_folio, .FloatingNavIsland_navIslandContainer, .AtmosphereLayer_atmosphere')) return;` queries all 3 injection-target selectors. Verified each selector is actually emitted by canonical `chrome.js` (FolioBar buildFolio root has `.FolioBar_folio` class; FloatingNavIsland container at `chrome.js:373, 377` uses `.FloatingNavIsland_navIslandContainer`; AtmosphereLayer root at `styles/AtmosphereLayer.module.css` uses `.AtmosphereLayer_atmosphere`).
- Guard is correctly scoped: returns BEFORE `body.dataset.chrome = 'on'`, preventing the side-effect dataset write from firing twice. Good.
- WP-specific concern (iframe reloads, preview refresh) — keeping the guard in the WP overlay patch (option a) is correct: Tauri doesn't have iframe reloads, so the canonical chrome.js shouldn't carry WP-specific defensive code. Decision rationale at line 246 is correct.

**Patch 9b (refresh-button class replacement):**
- Verified canonical `chrome.js:195-204`: refresh button currently uses inline `style="...; transition: color 150ms, border-color 150ms;"` and cannot express `:hover` / `:focus-visible`. Replacement with `class: F('folioRefreshButton')` is the right fix.
- New `.FolioBar_folioRefreshButton` rule at `L0-packet-W3-chrome-lane-pulled-forward.md:230-242` consumes proper design tokens (`--transition-normal`, `--color-text-primary`, `--color-rule-heavy`, `--color-spice-turmeric`) — no hard-coded values.
- **Canonical-vs-WP-overlay maintenance debt — assessed and bounded.** The trade documented at line 246 is honest: 9b could go upstream (improves Tauri too) but is held at overlay to minimize Tauri regression surface. The packet names v1.4.5+ as the promotion path. This is the same pattern as patches 1-8 (all overlay-only for WP-specific reasons), so the maintenance shape isn't novel — it's the established pattern for the lift. AC #4 carves out FolioBar.module.css from byte-equivalence specifically because of Patch 9b's extra rule, which is the right call.
- One residual: if canonical chrome.js gets edited in future and Patch 9b stops applying cleanly, `patch-chrome-js.py` exits code 2 with named diagnostic per §5.4. Failure mode is loud, not silent. Acceptable.

### V1.1 fold introduces no NEW architectural issues

- AC #28 Pill dual-primitive allowlist gate is a structural CI gate, not a one-off check — addresses the maintenance flag from cycle-1 §4. Filed to Codebase Maintenance project per cycle-1 §7 non-blocking suggestion. Verified `wp/dailyos/blocks/pill/` exists and is plugin-owned; chrome `.Pill_*` selectors verified non-overlapping.
- AC #26 computed-style proof tightens the alias-discipline gate (was: enqueue-handle inspection; now: actual `getComputedStyle` resolution at both frontend AND editor iframe). Strictly stronger.
- AC #29 Site Editor DB override safety + §13 reset path appendix close a real footgun cycle-1 didn't surface — that's net-positive for the packet.
- AC #30 alias discipline gate makes the load-order assertion machine-checkable. Was a soft architectural concern; now a CI gate.

---

## 3. K-in audit (reviewer-independent re-grep)

`docs/solutions/workflow-issues/`:
- `k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` — verified lines 14-20 contain the W4 reinvention finding referenced in §3 K-in record + §10 W4-coupling matrix. Citation correct.
- `node-modules-tracked-symlink-enotdir-pnpm-install-2026-05-19.md` — unrelated.
- `substrate-only-landing-needs-l0-amendment-2026-05-18.md` — unrelated (landing pages).
- `worktree-setup-needs-pnpm-install-2026-05-19.md` — unrelated.

`.docs/decisions/` — re-confirmed cycle-1 finding: 5 ADRs consumed (0073, 0076, 0077, 0129, 0130); no conflicts; no reinvention.

**K-in clean. No BLOCKED-with-cited-path.**

---

## 4. File paths referenced

- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:34-49` — §1 strategic framing + scope enumeration
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:230-246` — Patch 9 detail + canonical-vs-overlay decision
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:356` — AC #23 stock-theme negative gate
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:444-453` — §10 invariants (3 new V1.1)
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/design/reference/_shared/chrome.js:195-204` — canonical refresh-button inline-style site (Patch 9b target)
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/design/reference/_shared/chrome.js:465-491` — `inject()` function + DOMContentLoaded bootstrap (Patch 9a target)
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/design/reference/_shared/styles/FloatingNavIsland.module.css:13` — `.FloatingNavIsland_navIslandContainer` selector (idempotency-guard target)
- `/Users/jamesgiroux/Documents/dailyos-repo/docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:14-20` — W4 reinvention finding (§10 W4-coupling matrix cite)
- `/Users/jamesgiroux/Documents/dailyos-repo/wp/dailyos/blocks/pill/` — plugin-universe Pill block (verified separate from chrome `.Pill_*`)
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md` — cycle-1 verdict (3 conditions)

---

## 5. Architecture summary

V1.1 folds the 3 cycle-1 conditions cleanly, adds 3 net-positive ACs (#26, #28, #29) that strengthen the substrate boundary beyond what cycle-1 asked for, codifies 3 new invariants in §10 (runtime-injection scope, class-contract-not-DOM-order, W4-coupling matrix), and surfaces no new architectural debt. Patch 9 design is sound; the overlay-vs-canonical decision is the established pattern for the lift and the failure mode (patch anchor lost) is loud. K-in citations verified accurate.

**Verdict: APPROVE.** Cycle-2 closes from this reviewer.
